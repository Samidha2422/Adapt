//! Model manifest verification, honest fallback selection, and ONNX Runtime inference.
pub mod onnx;
pub use onnx::{
    OnnxAudioBackend, OnnxBehaviorModel, OnnxDeepfakeModel, OnnxImageBackend, OnnxNsfwModel,
    OnnxOcrBackend, OnnxOcrDetector, OnnxOcrModel, OnnxTextBackend, OnnxTextModel,
    OnnxWhisperModel, TextBoundingBox, TEXT_MODEL_LABELS,
};

use hre_core::Backend;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Component, Path},
};

/// Manifest metadata does not establish runtime readiness or tensor compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelManifest {
    pub models: Vec<ModelEntry>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelEntry {
    pub id: String,
    pub task: String,
    pub backend: ModelBackend,
    pub path: String,
    pub sha256: String,
    pub version: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub tokenizer: Option<String>,
    pub labels: Vec<String>,
    pub thresholds: BTreeMap<String, f64>,
    pub license: String,
    pub tier: String,
    pub status: ModelStatus,
    pub outputs: Vec<ModelOutput>,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub baseline: Vec<f64>,
}

impl ModelEntry {
    /// Validate the training order and return its benign attribution baseline.
    /// Transferred models may keep this metadata in the companion features.json.
    pub fn validated_features(
        &self,
        root: &Path,
        expected: &[String],
    ) -> Result<Vec<f64>, ModelError> {
        let (features, baseline) = if self.features.is_empty() && self.baseline.is_empty() {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct CompanionFeatures {
                version: String,
                message_features: Vec<String>,
                call_features: Vec<String>,
                baseline_message: Vec<f64>,
                baseline_call: Vec<f64>,
            }
            let path = checked_artifact_path(root, "features.json")?;
            let json = std::fs::read_to_string(path)
                .map_err(|_| ModelError::Invalid("feature metadata unreadable"))?;
            let companion: CompanionFeatures = serde_json::from_str(&json)?;
            if companion.version != "v1" {
                return Err(ModelError::Invalid("unsupported feature schema version"));
            }
            match self.task.as_str() {
                "behavior_messages" => (companion.message_features, companion.baseline_message),
                "behavior_calls" => (companion.call_features, companion.baseline_call),
                _ => return Err(ModelError::Invalid("task has no behavior feature schema")),
            }
        } else {
            (self.features.clone(), self.baseline.clone())
        };
        if features != expected
            || features.is_empty()
            || features.len() != baseline.len()
            || baseline.iter().any(|value| !value.is_finite())
        {
            return Err(ModelError::Invalid("feature order or baseline mismatch"));
        }
        Ok(baseline)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelBackend {
    Onnx,
    Ggml,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelStatus {
    Built,
    Simulated,
    Roadmap,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelOutput {
    pub name: String,
    pub dtype: TensorType,
    pub shape: Vec<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TensorType {
    Float32,
    Int64,
}
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("invalid model manifest: {0}")]
    Invalid(&'static str),
    #[error("manifest JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("inference failed: {0}")]
    Inference(String),
}
impl ModelManifest {
    /// Verify an enabled artifact before loading it. Run in spawn_blocking during reloads.
    /// This checks admission, not runtime readiness; loaders must validate tensors and warm up.
    pub fn validated_entry(&self, root: &Path, task: &str) -> Result<&ModelEntry, ModelError> {
        let entry = self
            .models
            .iter()
            .find(|entry| entry.task == task)
            .ok_or(ModelError::Invalid("model entry missing"))?;
        if !matches!(entry.status, ModelStatus::Built) {
            return Err(ModelError::Invalid(
                "model is not enabled for real inference",
            ));
        }
        let path = checked_artifact_path(root, &entry.path)?;
        if !hash_file(&path)
            .map_err(|_| ModelError::Invalid("model unreadable"))?
            .eq_ignore_ascii_case(&entry.sha256)
        {
            return Err(ModelError::Invalid("model hash mismatch"));
        }
        if is_proxy_artifact(&path, task)? {
            return Err(ModelError::Invalid("proxy model is not trained inference"));
        }
        if let Some(tokenizer) = &entry.tokenizer {
            checked_artifact_path(root, tokenizer)?;
        }
        match task {
            "text_fine_tuned" => {
                if entry.tokenizer.is_none()
                    || !entry
                        .labels
                        .iter()
                        .map(String::as_str)
                        .eq(TEXT_MODEL_LABELS)
                {
                    return Err(ModelError::Invalid(
                        "text label order or tokenizer mismatch",
                    ));
                }
                validate_probability_output(entry, 11)?;
            }
            "behavior_messages" | "behavior_calls" => {
                if !entry
                    .labels
                    .iter()
                    .map(String::as_str)
                    .eq(["non_harassment", "harassment"])
                {
                    return Err(ModelError::Invalid("behavior label order mismatch"));
                }
                validate_probability_output(entry, 2)?;
            }
            _ => {}
        }
        if !matches!(entry.backend, ModelBackend::Onnx) {
            return Err(ModelError::Invalid("ONNX runtime requires ONNX backend"));
        }
        Ok(entry)
    }

    /// Validates manifest declarations, without inspecting model tensor metadata.
    pub fn parse(json: &str) -> Result<Self, ModelError> {
        let manifest: Self = serde_json::from_str(json)?;
        let mut ids = BTreeSet::new();
        let mut tasks = BTreeSet::new();
        for m in &manifest.models {
            let mut outputs = BTreeSet::new();
            if m.id.trim().is_empty()
                || m.task.trim().is_empty()
                || !ids.insert(&m.id)
                || !tasks.insert(&m.task)
                || !safe_relative_path(&m.path)
                || m.tokenizer.as_ref().is_some_and(|p| !safe_relative_path(p))
                || m.sha256.len() != 64
                || !m.sha256.bytes().all(|b| b.is_ascii_hexdigit())
                || m.version.trim().is_empty()
                || m.tier.trim().is_empty()
                || m.license.trim().is_empty()
                || m.labels.iter().any(|s| s.trim().is_empty())
                || m.thresholds
                    .iter()
                    .any(|(k, v)| k.trim().is_empty() || !v.is_finite())
                || m.outputs.iter().any(|o| {
                    o.name.trim().is_empty()
                        || !outputs.insert(&o.name)
                        || o.shape.is_empty()
                        || o.shape.contains(&0)
                })
                || (matches!(m.backend, ModelBackend::Onnx) && m.outputs.is_empty())
                || (matches!(m.backend, ModelBackend::Ggml) && !m.outputs.is_empty())
                || m.features.len() != m.baseline.len()
                || m.baseline.iter().any(|v| !v.is_finite())
                || m.features.iter().any(|s| s.trim().is_empty())
                || m.features
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != m.features.len()
            {
                return Err(ModelError::Invalid(
                    "invalid model metadata, path, output, feature order or baseline",
                ));
            }
        }
        Ok(manifest)
    }
}
fn validate_probability_output(entry: &ModelEntry, labels: u64) -> Result<(), ModelError> {
    if entry.outputs.len() != 1
        || entry.outputs[0].name != "probabilities"
        || !matches!(entry.outputs[0].dtype, TensorType::Float32)
        || entry.outputs[0].shape != [1, labels]
    {
        return Err(ModelError::Invalid("probability output contract mismatch"));
    }
    Ok(())
}

fn checked_artifact_path(root: &Path, relative: &str) -> Result<std::path::PathBuf, ModelError> {
    if !safe_relative_path(relative) {
        return Err(ModelError::Invalid("unsafe artifact path"));
    }
    let root = root
        .canonicalize()
        .map_err(|_| ModelError::Invalid("model root unavailable"))?;
    let path = root
        .join(relative)
        .canonicalize()
        .map_err(|_| ModelError::Invalid("artifact missing"))?;
    if !path.starts_with(root) || !path.is_file() {
        return Err(ModelError::Invalid("unsafe artifact path"));
    }
    Ok(path)
}

/// Conservative admission guard for the shipped neural families, not proof of training.
/// Tree ensembles can legitimately be small and use a separate lower bound.
pub(crate) fn is_proxy_artifact(path: &Path, task: &str) -> Result<bool, ModelError> {
    let minimum = match task {
        "text_fast" | "text_accurate" | "text_fine_tuned" | "sentiment" | "embeddings" => 1_000_000,
        "nsfw" | "deepfake" | "face" | "tone" => 16_384,
        "ocr" => 100_000,
        "behavior_messages" | "behavior_calls" => 1_024,
        _ => return Ok(false),
    };
    let size = path
        .metadata()
        .map_err(|_| ModelError::Invalid("model unreadable"))?
        .len();
    Ok(size < minimum)
}
fn safe_relative_path(s: &str) -> bool {
    !s.is_empty()
        && !s.contains(['\\', ':'])
        && !s.chars().any(char::is_control)
        && s.split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && Path::new(s)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelReport {
    pub name: String,
    pub version: Option<String>,
    pub hash_verified: bool,
    pub backend: Backend,
    pub degraded: Vec<String>,
}
/// Startup-only synchronous verifier; call via spawn_blocking when reloading from async code.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelRegistry {
    pub models: BTreeMap<String, ModelReport>,
    pub degraded: Vec<String>,
}
impl ModelRegistry {
    /// Missing or invalid manifests select deterministic fallbacks rather than panicking.
    pub fn load(root: &Path) -> Self {
        let mut registry = Self::default();
        let manifest = match std::fs::read_to_string(root.join("manifest.json")) {
            Ok(json) => match ModelManifest::parse(&json) {
                Ok(m) => m,
                Err(_) => {
                    registry.degraded.push("registry.manifest_invalid".into());
                    return registry;
                }
            },
            Err(e) => {
                registry.degraded.push(
                    if e.kind() == std::io::ErrorKind::NotFound {
                        "registry.manifest_missing"
                    } else {
                        "registry.manifest_unreadable"
                    }
                    .into(),
                );
                return registry;
            }
        };
        for entry in manifest.models {
            let mut report = ModelReport {
                name: entry.id.clone(),
                version: Some(entry.version),
                hash_verified: false,
                backend: Backend::Heuristic,
                degraded: Vec::new(),
            };
            let path = root.join(&entry.path);
            let inside_root = root
                .canonicalize()
                .ok()
                .zip(path.canonicalize().ok())
                .is_some_and(|(r, p)| p.starts_with(r));
            if !path.exists() {
                report.degraded.push(format!("{}.onnx_missing", entry.task));
            } else if !inside_root {
                report.degraded.push(format!("{}.unsafe_path", entry.task));
            } else {
                match hash_file(&path) {
                    Ok(hash) if hash.eq_ignore_ascii_case(&entry.sha256) => {
                        report.hash_verified = true;
                        // Hash verification is not runnable ONNX inference.
                        report
                            .degraded
                            .push(format!("{}.inference_unavailable", entry.task));
                        if is_proxy_artifact(&path, &entry.task).unwrap_or(true) {
                            report.degraded.push(format!("{}.proxy_model", entry.task));
                        }
                        if !matches!(entry.status, ModelStatus::Built) {
                            report.degraded.push(format!("{}.not_enabled", entry.task));
                        }
                        if entry
                            .tokenizer
                            .as_ref()
                            .is_some_and(|p| checked_artifact_path(root, p).is_err())
                        {
                            report
                                .degraded
                                .push(format!("{}.tokenizer_unavailable", entry.task));
                        }
                    }
                    Ok(_) => report
                        .degraded
                        .push(format!("{}.hash_mismatch", entry.task)),
                    Err(_) => report.degraded.push(format!("{}.unreadable", entry.task)),
                }
            }
            registry.models.insert(entry.task, report);
        }
        registry
    }
    /// Record successful validated loading and warm-up, never hash-only readiness.
    /// The caller must first use validated_entry and the task's runtime loader.
    pub fn mark_runtime_ready(&mut self, task: &str) -> Result<(), ModelError> {
        let report = self
            .models
            .get_mut(task)
            .ok_or(ModelError::Invalid("model entry missing"))?;
        let unavailable = format!("{task}.inference_unavailable");
        if !report.hash_verified || report.degraded.iter().any(|reason| reason != &unavailable) {
            return Err(ModelError::Invalid(
                "artifact is not eligible for runtime readiness",
            ));
        }
        report.backend = Backend::Onnx;
        report.degraded.clear();
        Ok(())
    }
    /// Always reports missing fallback models even when the manifest has no entry for them.
    pub fn report(&self, name: &str) -> ModelReport {
        let task = match name {
            "text" => "text_fast",
            "behavior_msg" => "behavior_messages",
            "behavior_call" => "behavior_calls",
            "image" => "nsfw",
            "audio" => "whisper",
            _ => name,
        };
        self.models
            .get(name)
            .or_else(|| self.models.get(task))
            .or_else(|| self.models.values().find(|report| report.name == name))
            .cloned()
            .map(|mut report| {
                report.name = name.into();
                report
            })
            .unwrap_or_else(|| ModelReport {
                name: name.into(),
                version: None,
                hash_verified: false,
                backend: Backend::Heuristic,
                degraded: vec![format!("{name}.onnx_missing")],
            })
    }
    /// Aggregate manifest and per-module degradation in stable order.
    pub fn degradation(&self) -> Vec<String> {
        let mut all = self.degraded.clone();
        all.extend(self.models.values().flat_map(|m| m.degraded.clone()));
        all.sort();
        all.dedup();
        all
    }
}
fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
/// Numerical backend contract. Inference implementers must validate input order and output range.
pub trait InferenceBackend: Send + Sync {
    fn predict(&self, features: &[f64]) -> Result<f64, ModelError>;
    fn backend(&self) -> Backend;
}
/// Deterministic weighted logistic fallback reusable by numerical modules.
#[derive(Debug, Clone)]
pub struct LogisticFallback {
    pub bias: f64,
    pub weights: Vec<f64>,
}
impl InferenceBackend for LogisticFallback {
    fn predict(&self, features: &[f64]) -> Result<f64, ModelError> {
        if features.len() != self.weights.len()
            || !self.bias.is_finite()
            || features
                .iter()
                .chain(self.weights.iter())
                .any(|x| !x.is_finite())
        {
            return Err(ModelError::Invalid(
                "nonfinite or mismatched feature vector",
            ));
        }
        let z = self.bias
            + features
                .iter()
                .zip(&self.weights)
                .map(|(x, w)| x * w)
                .sum::<f64>();
        if !z.is_finite() {
            return Err(ModelError::Invalid("logistic accumulation overflow"));
        }
        Ok(1.0 / (1.0 + (-z.clamp(-700.0, 700.0)).exp()))
    }
    fn backend(&self) -> Backend {
        Backend::Heuristic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry() -> serde_json::Value {
        serde_json::json!({
            "id": "artifact-v1", "task": "text_fast", "backend": "onnx",
            "path": "test.onnx", "sha256": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "version": "1", "tokenizer": "tokenizer.json", "labels": ["toxic"],
            "thresholds": {"decision": 0.5}, "license": "unverified", "tier": "fast",
            "status": "roadmap", "outputs": [{"name": "probabilities", "dtype": "float32", "shape": [1, 7]}]
        })
    }
    fn parse_entries(entries: Vec<serde_json::Value>) -> Result<ModelManifest, ModelError> {
        ModelManifest::parse(&serde_json::json!({"models": entries}).to_string())
    }
    #[test]
    fn parses_every_published_contract_example() {
        let contract = include_str!("../../../docs/MODEL_CONTRACT.md");
        let mut lines = contract.lines();
        let mut entries = Vec::new();
        while let Some(line) = lines.next() {
            if line.trim() == "```json" {
                entries.push(serde_json::from_str(lines.next().unwrap()).unwrap());
            }
        }
        assert_eq!(entries.len(), 13);
        assert!(parse_entries(entries).is_ok());
    }
    #[test]
    fn contract_entries_and_feature_baseline_are_preserved() {
        let mut model = entry();
        model["features"] = serde_json::json!(["b", "a"]);
        model["baseline"] = serde_json::json!([2.0, 1.0]);
        let parsed = parse_entries(vec![model]).unwrap();
        assert_eq!(parsed.models[0].features, ["b", "a"]);
        assert_eq!(parsed.models[0].baseline, [2.0, 1.0]);
        assert_eq!(parsed.models[0].outputs[0].shape, [1, 7]);
        let mut whisper = entry();
        whisper["backend"] = "ggml".into();
        whisper["outputs"] = serde_json::json!([]);
        whisper["tokenizer"] = serde_json::Value::Null;
        assert!(parse_entries(vec![whisper]).is_ok());
    }
    #[test]
    fn manifest_rejects_invalid_contract_and_unsafe_paths() {
        for (field, value) in [
            ("path", serde_json::json!("../test.onnx")),
            ("path", serde_json::json!("C:/test.onnx")),
            ("tokenizer", serde_json::json!("../tokenizer")),
            ("sha256", serde_json::json!("not-a-hash")),
            ("version", serde_json::json!(" ")),
            ("features", serde_json::json!(["x"])),
            (
                "outputs",
                serde_json::json!([{"name":"p","dtype":"float32","shape":[1,0]}]),
            ),
            (
                "outputs",
                serde_json::json!([{"name":"p","dtype":"float","shape":[1,2]}]),
            ),
            (
                "outputs",
                serde_json::json!([{"name":"p","dtype":"float32","shape":[1,2],"extra":1}]),
            ),
            ("outputs", serde_json::json!([])),
        ] {
            let mut model = entry();
            model[field] = value;
            assert!(
                parse_entries(vec![model]).is_err(),
                "accepted invalid {field}"
            );
        }
        let mut model = entry();
        model.as_object_mut().unwrap().remove("tokenizer");
        assert!(parse_entries(vec![model]).is_err());
        let mut model = entry();
        model["features"] = serde_json::json!(["x", "x"]);
        model["baseline"] = serde_json::json!([0, 0]);
        assert!(parse_entries(vec![model]).is_err());
        let mut other = entry();
        other["id"] = "other".into();
        assert!(parse_entries(vec![entry(), other.clone()]).is_err());
        other["id"] = entry()["id"].clone();
        other["task"] = "other".into();
        assert!(parse_entries(vec![entry(), other]).is_err());
    }
    #[test]
    fn verified_file_still_does_not_claim_active_inference() {
        let dir = std::env::temp_dir().join(format!(
            "hre-ml-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.onnx"), b"abc").unwrap();
        let manifest = serde_json::json!({"models":[entry()]});
        std::fs::write(dir.join("manifest.json"), manifest.to_string()).unwrap();
        let registry = ModelRegistry::load(&dir);
        assert!(registry.report("text").hash_verified);
        assert!(registry.report("artifact-v1").hash_verified);
        assert_eq!(registry.report("text_fast").backend, Backend::Heuristic);
        assert_eq!(
            registry.report("text").degraded,
            [
                "text_fast.inference_unavailable",
                "text_fast.proxy_model",
                "text_fast.not_enabled",
                "text_fast.tokenizer_unavailable"
            ]
        );
        assert_eq!(registry.report("absent").degraded, ["absent.onnx_missing"]);
        std::fs::remove_file(dir.join("test.onnx")).unwrap();
        assert_eq!(
            ModelRegistry::load(&dir).report("text").degraded,
            ["text_fast.onnx_missing"]
        );
        std::fs::remove_file(dir.join("manifest.json")).unwrap();
        assert_eq!(
            ModelRegistry::load(&dir).degraded,
            ["registry.manifest_missing"]
        );
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn validated_loading_checks_digest_labels_and_readiness_separately() {
        let dir = std::env::temp_dir().join(format!("hre-admission-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let bytes = vec![0u8; 2048];
        std::fs::write(dir.join("test.onnx"), &bytes).unwrap();
        let mut value = entry();
        value["task"] = "behavior_messages".into();
        value["status"] = "built".into();
        value["sha256"] = format!("{:x}", Sha256::digest(&bytes)).into();
        value["tokenizer"] = serde_json::Value::Null;
        value["labels"] = serde_json::json!(["non_harassment", "harassment"]);
        value["outputs"][0]["shape"] = serde_json::json!([1, 2]);
        let manifest = parse_entries(vec![value.clone()]).unwrap();
        assert!(manifest.validated_entry(&dir, "behavior_messages").is_ok());
        std::fs::write(
            dir.join("manifest.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();
        let mut registry = ModelRegistry::load(&dir);
        assert_eq!(registry.report("behavior_msg").backend, Backend::Heuristic);
        // Models become active only after the caller records successful runtime validation.
        registry.mark_runtime_ready("behavior_messages").unwrap();
        assert_eq!(registry.report("behavior_msg").backend, Backend::Onnx);
        value["labels"] = serde_json::json!(["harassment", "non_harassment"]);
        assert!(parse_entries(vec![value])
            .unwrap()
            .validated_entry(&dir, "behavior_messages")
            .is_err());
        std::fs::write(dir.join("test.onnx"), b"changed").unwrap();
        assert!(manifest.validated_entry(&dir, "behavior_messages").is_err());
        assert!(ModelRegistry::load(&dir)
            .mark_runtime_ready("behavior_messages")
            .is_err());
        std::fs::remove_file(dir.join("test.onnx")).unwrap();
        std::fs::remove_file(dir.join("manifest.json")).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn companion_schema_requires_exact_order_and_complete_baseline() {
        let dir = std::env::temp_dir().join(format!("hre-features-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut value = entry();
        value["task"] = "behavior_messages".into();
        let model = parse_entries(vec![value]).unwrap().models.remove(0);
        let expected = vec!["first".into(), "second".into()];
        let mut companion = serde_json::json!({
            "version":"v1", "message_features":expected, "call_features":["call"],
            "baseline_message":[0.0, -1.0], "baseline_call":[0.0]
        });
        let path = dir.join("features.json");
        std::fs::write(&path, companion.to_string()).unwrap();
        assert_eq!(
            model.validated_features(&dir, &expected).unwrap(),
            [0.0, -1.0]
        );
        assert!(model
            .validated_features(&dir, &["second".into(), "first".into()])
            .is_err());
        companion["baseline_message"] = serde_json::json!([0.0]);
        std::fs::write(&path, companion.to_string()).unwrap();
        assert!(model.validated_features(&dir, &expected).is_err());
        companion["version"] = "unknown".into();
        std::fs::write(&path, companion.to_string()).unwrap();
        assert!(model.validated_features(&dir, &expected).is_err());
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn tiny_neural_artifact_is_rejected_even_with_matching_digest() {
        let dir = std::env::temp_dir().join(format!("hre-proxy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.onnx"), b"abc").unwrap();
        let mut value = entry();
        value["status"] = "built".into();
        value["tokenizer"] = serde_json::Value::Null;
        let manifest = parse_entries(vec![value]).unwrap();
        assert!(matches!(
            manifest.validated_entry(&dir, "text_fast"),
            Err(ModelError::Invalid("proxy model is not trained inference"))
        ));
        std::fs::write(
            dir.join("manifest.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();
        let mut registry = ModelRegistry::load(&dir);
        assert!(registry.report("text").hash_verified);
        assert!(registry
            .report("text")
            .degraded
            .contains(&"text_fast.proxy_model".into()));
        assert!(registry.mark_runtime_ready("text_fast").is_err());
        std::fs::remove_file(dir.join("test.onnx")).unwrap();
        std::fs::remove_file(dir.join("manifest.json")).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    proptest::proptest! {
        #[test]
        fn manifest_parser_never_panics(s in ".{0,2048}") { let _ = ModelManifest::parse(&s); }
    }
}
