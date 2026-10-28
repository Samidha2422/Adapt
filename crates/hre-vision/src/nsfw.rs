use crate::hasher::compute_phash;
use crate::validate::{validate_image, VisionError};
use hre_core::{Action, Backend, Signal, Status};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAnalysis {
    pub signal: Signal,
    pub p_hash: u64,
    pub sha256: String,
    pub recommended_action: Option<Action>,
    pub simulated: bool,
    pub degraded: Vec<String>,
}

pub trait ImageBackend: Send + Sync {
    fn analyze(&self, bytes: &[u8], declared_mime: &str) -> Result<ImageAnalysis, VisionError>;
}

/// Deterministic screening backend with honest zero-model fallback and demo override support.
#[derive(Debug, Clone, Default)]
pub struct HeuristicImageBackend {
    /// Mapping of lowercase SHA-256 hex strings to synthetic NSFW scores (0.0..=1.0)
    overrides: BTreeMap<String, f64>,
}

impl HeuristicImageBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_override(mut self, sha256: &str, score: f64) -> Self {
        if score.is_finite() {
            self.overrides
                .insert(sha256.to_lowercase(), score.clamp(0.0, 1.0));
        }
        self
    }

    pub fn from_overrides_toml(toml_str: &str) -> Self {
        #[derive(Deserialize)]
        struct OverridesDoc {
            #[serde(default)]
            assets: BTreeMap<String, f64>,
        }
        let doc: Result<OverridesDoc, _> = toml::from_str(toml_str);
        let mut backend = Self::new();
        if let Ok(doc) = doc {
            for (hash, score) in doc.assets {
                backend = backend.with_override(&hash, score);
            }
        }
        backend
    }

    pub fn from_config_file(path: &std::path::Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => Self::from_overrides_toml(&content),
            Err(_) => Self::new(),
        }
    }
}

impl ImageBackend for HeuristicImageBackend {
    fn analyze(&self, bytes: &[u8], declared_mime: &str) -> Result<ImageAnalysis, VisionError> {
        let _meta = validate_image(bytes, declared_mime)?;

        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let sha256 = format!("{:x}", hasher.finalize());

        let p_hash = compute_phash(bytes, declared_mime)?;

        if let Some(&override_score) = self.overrides.get(&sha256) {
            let recommended_action = if override_score >= 0.5 {
                Some(Action::BlurMedia)
            } else {
                None
            };
            let mut details = BTreeMap::new();
            details.insert("sha256".into(), serde_json::json!(sha256));
            details.insert("p_hash".into(), serde_json::json!(p_hash));
            details.insert("override_applied".into(), serde_json::json!(true));
            details.insert("simulated".into(), serde_json::json!(true));
            details.insert("hash_kind".into(), serde_json::json!("encoded_byte_proxy"));

            Ok(ImageAnalysis {
                signal: Signal {
                    score: override_score,
                    label: "simulated_nsfw_proxy".into(),
                    status: Status::Simulated,
                    backend: Backend::Heuristic,
                    confidence: 1.0,
                    details,
                },
                p_hash,
                sha256,
                recommended_action,
                simulated: true,
                degraded: vec!["image.demo_override_simulated".into()],
            })
        } else {
            // Honest zero-model fallback: reports missing ONNX model and 0.0 score
            let mut details = BTreeMap::new();
            details.insert("sha256".into(), serde_json::json!(sha256));
            details.insert("p_hash".into(), serde_json::json!(p_hash));
            details.insert("onnx_missing".into(), serde_json::json!(true));
            details.insert("hash_kind".into(), serde_json::json!("encoded_byte_proxy"));

            Ok(ImageAnalysis {
                signal: Signal {
                    score: 0.0,
                    label: "image_unavailable".into(),
                    status: Status::Roadmap,
                    backend: Backend::Disabled,
                    confidence: 0.0,
                    details,
                },
                p_hash,
                sha256,
                recommended_action: None,
                simulated: false,
                degraded: vec![
                    "image.onnx_missing".into(),
                    "image.inference_unavailable".into(),
                ],
            })
        }
    }
}
