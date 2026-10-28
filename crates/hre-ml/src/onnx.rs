//! Safe ONNX Runtime loader, execution, and backends for behavior and text models.

use crate::{InferenceBackend, ModelError};
use hre_core::{Backend, Reason, Signal, Status};
use hre_text::{
    cues::detect_cues,
    normalizer::{normalize, MAX_TEXT_LENGTH_BYTES},
    pii::mask_pii,
    TextBackend, TextError, TextReport,
};
use hre_vision::{validate_image, HeuristicImageBackend, ImageAnalysis, ImageBackend, VisionError};
use ndarray::{Array2, Array4};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

static ORT_ENV: OnceLock<Result<ort::environment::Environment, String>> = OnceLock::new();

/// The 11 independent multi-label categories produced by the fine-tuned XLM-RoBERTa model.
pub const TEXT_MODEL_LABELS: [&str; 11] = [
    "toxicity",
    "severe_toxicity",
    "obscene",
    "insult",
    "threat",
    "identity_attack",
    "sexual_explicit",
    "coercion_sextortion",
    "stalking_cue",
    "doxxing_cue",
    "sexual_harassment",
];

/// Locate onnxruntime.dll dynamically across environment, venv, target, and system paths.
pub fn find_onnxruntime_dylib() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("ORT_DYLIB_PATH") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    let candidates = [
        ".venv/Lib/site-packages/onnxruntime/capi/onnxruntime.dll",
        "../.venv/Lib/site-packages/onnxruntime/capi/onnxruntime.dll",
        "../../.venv/Lib/site-packages/onnxruntime/capi/onnxruntime.dll",
        "target/tools/onnxruntime.dll",
        "models/onnxruntime.dll",
    ];
    for c in &candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let p = parent.join("onnxruntime.dll");
            if p.exists() {
                return Some(p);
            }
            for ancestor in parent.ancestors().take(5) {
                let p = ancestor.join(".venv/Lib/site-packages/onnxruntime/capi/onnxruntime.dll");
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// Get or initialize the global ONNX Runtime environment.
pub fn get_or_init_ort_env() -> Result<&'static ort::environment::Environment, ModelError> {
    ORT_ENV
        .get_or_init(|| {
            let dylib_path = find_onnxruntime_dylib().ok_or_else(|| {
                "onnxruntime dynamic library (onnxruntime.dll) not found in candidate paths"
                    .to_string()
            })?;
            let builder = ort::init_from(&dylib_path).map_err(|e| {
                format!(
                    "failed to initialize ONNX Runtime from {}: {e}",
                    dylib_path.display()
                )
            })?;
            builder
                .build()
                .map_err(|e| format!("failed to build ONNX Environment: {e}"))
        })
        .as_ref()
        .map_err(|msg| ModelError::Inference(msg.clone()))
}

/// Thread-safe ONNX model wrapper for behavior tabular models.
pub struct OnnxBehaviorModel {
    session: Mutex<Session>,
    expected_features: usize,
}

impl std::fmt::Debug for OnnxBehaviorModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnnxBehaviorModel")
            .field("expected_features", &self.expected_features)
            .finish_non_exhaustive()
    }
}

impl OnnxBehaviorModel {
    /// Load an ONNX behavior model from disk with validation of the input feature count.
    pub fn load(model_path: &Path, expected_features: usize) -> Result<Self, ModelError> {
        let env = get_or_init_ort_env()?;
        let session = Session::builder(env)
            .map_err(|e| ModelError::Inference(format!("session builder error: {e}")))?
            .with_optimization_level(GraphOptimizationLevel::Level1)
            .map_err(|e| ModelError::Inference(format!("optimization level error: {e}")))?
            .commit_from_file(model_path)
            .map_err(|e| {
                ModelError::Inference(format!(
                    "failed to load ONNX model from {}: {e}",
                    model_path.display()
                ))
            })?;
        Ok(Self {
            session: Mutex::new(session),
            expected_features,
        })
    }

    /// Predict harassment probability for a float feature vector.
    pub fn predict(&self, features: &[f32]) -> Result<f32, ModelError> {
        if features.len() != self.expected_features {
            return Err(ModelError::Invalid("feature vector length mismatch"));
        }
        if features.iter().any(|v| !v.is_finite()) {
            return Err(ModelError::Invalid(
                "non-finite feature value in inference input",
            ));
        }
        let array = Array2::from_shape_vec((1, self.expected_features), features.to_vec())
            .map_err(|e| ModelError::Inference(format!("failed to create input array: {e}")))?;
        let tensor = Tensor::from_array(array)
            .map_err(|e| ModelError::Inference(format!("failed to create input tensor: {e}")))?;

        let mut session = self
            .session
            .lock()
            .map_err(|_| ModelError::Inference("ONNX session mutex poisoned".into()))?;
        let outputs = session
            .run(ort::inputs!["features" => tensor])
            .map_err(|e| ModelError::Inference(format!("inference run error: {e}")))?;

        let probs = outputs["probabilities"]
            .try_extract_tensor::<f32>()
            .map_err(|e| ModelError::Inference(format!("failed to extract probabilities: {e}")))?;
        let slice = probs.1;

        if slice.len() < 2 {
            return Err(ModelError::Inference(format!(
                "expected at least 2 outputs, got {}",
                slice.len()
            )));
        }
        let prob_harassment = slice[1];
        if !prob_harassment.is_finite() || !(0.0..=1.0).contains(&prob_harassment) {
            return Err(ModelError::Inference(format!(
                "output probability out of range: {prob_harassment}"
            )));
        }
        Ok(prob_harassment)
    }

    /// Return expected feature count for this model.
    pub fn expected_features(&self) -> usize {
        self.expected_features
    }
}

impl InferenceBackend for OnnxBehaviorModel {
    fn predict(&self, features: &[f64]) -> Result<f64, ModelError> {
        let f32_features: Vec<f32> = features.iter().map(|&x| x as f32).collect();
        let prob = self.predict(&f32_features)?;
        Ok(f64::from(prob))
    }

    fn backend(&self) -> Backend {
        Backend::Onnx
    }
}

/// Thread-safe ONNX model wrapper for fine-tuned XLM-RoBERTa text classification.
pub struct OnnxTextModel {
    session: Mutex<Session>,
    tokenizer: tokenizers::Tokenizer,
}

impl std::fmt::Debug for OnnxTextModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnnxTextModel").finish_non_exhaustive()
    }
}

impl OnnxTextModel {
    /// Load an ONNX text model and SentencePiece tokenizer from disk.
    pub fn load(model_path: &Path, tokenizer_path: &Path) -> Result<Self, ModelError> {
        let env = get_or_init_ort_env()?;
        let session = Session::builder(env)
            .map_err(|e| ModelError::Inference(format!("session builder error: {e}")))?
            .with_optimization_level(GraphOptimizationLevel::Level1)
            .map_err(|e| ModelError::Inference(format!("optimization level error: {e}")))?
            .commit_from_file(model_path)
            .map_err(|e| {
                ModelError::Inference(format!(
                    "failed to load ONNX text model from {}: {e}",
                    model_path.display()
                ))
            })?;
        let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path).map_err(|e| {
            ModelError::Inference(format!(
                "failed to load tokenizer from {}: {e}",
                tokenizer_path.display()
            ))
        })?;
        Ok(Self {
            session: Mutex::new(session),
            tokenizer,
        })
    }

    /// Predict the 11 independent sigmoid output probabilities for the input text.
    pub fn predict(&self, text: &str) -> Result<[f32; 11], ModelError> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| ModelError::Inference(format!("tokenization failed: {e}")))?;

        let mut ids_vec: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();
        let mut mask_vec: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&x| x as i64)
            .collect();

        // Enforce exact length of 128 (pad with pad_token_id = 1 if shorter, truncate if longer).
        if ids_vec.len() < 128 {
            ids_vec.resize(128, 1);
            mask_vec.resize(128, 0);
        } else if ids_vec.len() > 128 {
            ids_vec.truncate(128);
            mask_vec.truncate(128);
        }

        let input_ids = Array2::from_shape_vec((1, 128), ids_vec)
            .map_err(|e| ModelError::Inference(format!("failed to create input_ids array: {e}")))?;
        let attention_mask = Array2::from_shape_vec((1, 128), mask_vec).map_err(|e| {
            ModelError::Inference(format!("failed to create attention_mask array: {e}"))
        })?;

        let tensor_ids = Tensor::from_array(input_ids).map_err(|e| {
            ModelError::Inference(format!("failed to create input_ids tensor: {e}"))
        })?;
        let tensor_mask = Tensor::from_array(attention_mask).map_err(|e| {
            ModelError::Inference(format!("failed to create attention_mask tensor: {e}"))
        })?;

        let mut session = self
            .session
            .lock()
            .map_err(|_| ModelError::Inference("ONNX text session mutex poisoned".into()))?;

        let outputs = session
            .run(ort::inputs![
                "input_ids" => tensor_ids,
                "attention_mask" => tensor_mask
            ])
            .map_err(|e| ModelError::Inference(format!("text inference run error: {e}")))?;

        let probs = outputs["probabilities"]
            .try_extract_tensor::<f32>()
            .map_err(|e| {
                ModelError::Inference(format!("failed to extract text probabilities: {e}"))
            })?;
        let slice = probs.1;

        if slice.len() != 11 {
            return Err(ModelError::Inference(format!(
                "expected 11 output probabilities, got {}",
                slice.len()
            )));
        }

        let mut result = [0.0f32; 11];
        for i in 0..11 {
            let p = slice[i];
            if !p.is_finite() || !(0.0..=1.0).contains(&p) {
                return Err(ModelError::Inference(format!(
                    "output probability out of range at index {i}: {p}"
                )));
            }
            result[i] = p;
        }
        Ok(result)
    }
}

/// Pluggable ONNX text inference backend implementing `hre_text::TextBackend`.
#[derive(Debug, Clone)]
pub struct OnnxTextBackend {
    model: Arc<OnnxTextModel>,
    thresholds: BTreeMap<String, f64>,
}

impl OnnxTextBackend {
    /// Create a new ONNX text backend from a shared model and calibrated thresholds.
    pub fn new(model: Arc<OnnxTextModel>, thresholds: BTreeMap<String, f64>) -> Self {
        Self { model, thresholds }
    }

    /// Load the model and tokenizer from directory and create the backend.
    pub fn load_from_dir(
        dir: &Path,
        thresholds: BTreeMap<String, f64>,
    ) -> Result<Self, ModelError> {
        let model_path = dir.join("text_fine_tuned.onnx");
        let tokenizer_path = dir.join("tokenizer.json");
        let model = Arc::new(OnnxTextModel::load(&model_path, &tokenizer_path)?);
        Ok(Self::new(model, thresholds))
    }

    /// Helper to get a threshold with fallback.
    fn threshold(&self, name: &str, default: f64) -> f64 {
        self.thresholds.get(name).copied().unwrap_or(default)
    }
}

impl TextBackend for OnnxTextBackend {
    fn analyze(&self, raw_text: &str) -> Result<TextReport, TextError> {
        let byte_len = raw_text.len();
        if byte_len > MAX_TEXT_LENGTH_BYTES {
            return Err(TextError::TextTooLong {
                len: byte_len,
                max: MAX_TEXT_LENGTH_BYTES,
            });
        }
        if raw_text.trim().is_empty() {
            return Err(TextError::EmptyText);
        }

        // 1. Normalize
        let normalized = normalize(raw_text);
        let lower = normalized.to_ascii_lowercase();

        // 2. PII Detection & Masking
        let (masked_text, pii_matches) = mask_pii(raw_text);

        // 3. Cue Detection
        let cues = detect_cues(&lower);

        // 4. ONNX Model Inference
        let scores = self
            .model
            .predict(&normalized)
            .map_err(|e| TextError::Classification(e.to_string()))?;

        let mut toxicity = f64::from(scores[0]);
        let severe_toxicity = f64::from(scores[1]);
        let obscene = f64::from(scores[2]);
        let mut insult = f64::from(scores[3]);
        let mut threat = f64::from(scores[4]);
        let identity_attack = f64::from(scores[5]);
        let sexual_explicit = f64::from(scores[6]);
        let mut coercion = f64::from(scores[7]);
        let _stalking_cue = f64::from(scores[8]);
        let _doxxing_cue = f64::from(scores[9]);
        let _sexual_harassment = f64::from(scores[10]);

        let mut reasons = Vec::new();

        // Quoted abuse discount
        if cues.is_quoted && toxicity > 0.3 {
            toxicity *= 0.35;
            threat *= 0.35;
            coercion *= 0.35;
            insult *= 0.35;
            reasons.push(Reason {
                code: "QUOTED_ABUSE_DISCOUNT".into(),
                text: "Content is quoted speech or reported context, discounted hostility score"
                    .into(),
                weight: 0.2,
                evidence: BTreeMap::new(),
            });
        }

        // Calibrated thresholds
        let threat_th = self.threshold("threat", 0.45);
        let coercion_th = self.threshold("coercion_sextortion", 0.60);

        if threat >= threat_th || !cues.threat_cues.is_empty() {
            reasons.push(Reason {
                code: "DIRECT_THREAT_DETECTED".into(),
                text: if !cues.threat_cues.is_empty() {
                    format!("Direct threat pattern detected: {:?}", cues.threat_cues)
                } else {
                    format!("High neural threat probability: {threat:.2}")
                },
                weight: threat.max(0.8),
                evidence: BTreeMap::from([("threat_prob".into(), serde_json::json!(threat))]),
            });
        }

        if coercion >= coercion_th || !cues.coercion_cues.is_empty() {
            reasons.push(Reason {
                code: "COERCION_SEXTORTION_CUE".into(),
                text: if !cues.coercion_cues.is_empty() {
                    format!("Coercion or extortion pattern: {:?}", cues.coercion_cues)
                } else {
                    format!("High neural coercion probability: {coercion:.2}")
                },
                weight: coercion.max(0.8),
                evidence: BTreeMap::from([("coercion_prob".into(), serde_json::json!(coercion))]),
            });
        }

        if !cues.location_cues.is_empty() {
            reasons.push(Reason {
                code: "LOCATION_STALKING_CUE".into(),
                text: format!("Physical presence / location cue: {:?}", cues.location_cues),
                weight: 0.75,
                evidence: BTreeMap::new(),
            });
        }

        // PII signal and Doxxing detection
        let has_pii = !pii_matches.is_empty();
        let is_doxxing = has_pii && (toxicity >= 0.325 || threat >= 0.45 || coercion >= 0.60);
        let pii_score = if is_doxxing {
            reasons.push(Reason {
                code: "DOXXING_WITH_HOSTILITY".into(),
                text:
                    "Personal identifiable information posted in a hostile or threatening context"
                        .into(),
                weight: 0.85,
                evidence: BTreeMap::new(),
            });
            0.90
        } else if has_pii {
            reasons.push(Reason {
                code: "PII_DETECTED".into(),
                text: format!("Detected {} personal information items", pii_matches.len()),
                weight: 0.45,
                evidence: BTreeMap::new(),
            });
            0.60
        } else {
            0.0
        };

        // All 11 label values in details
        let mut details = BTreeMap::new();
        for (i, &name) in TEXT_MODEL_LABELS.iter().enumerate() {
            details.insert(name.into(), serde_json::json!(f64::from(scores[i])));
        }

        let primary_signal = Signal {
            score: toxicity,
            label: "text_xlmr_fine_tuned".into(),
            status: Status::Built,
            backend: Backend::Onnx,
            confidence: if toxicity > 0.325 { 0.85 } else { 0.65 },
            details,
        };

        let pii_signal = Signal {
            score: pii_score,
            label: "pii_heuristic".into(),
            status: Status::Built,
            backend: Backend::Heuristic,
            confidence: if has_pii { 0.90 } else { 0.50 },
            details: BTreeMap::from([
                ("match_count".into(), serde_json::json!(pii_matches.len())),
                ("is_doxxing".into(), serde_json::json!(is_doxxing)),
            ]),
        };

        Ok(TextReport {
            primary_signal,
            pii_signal,
            masked_text,
            pii_matches,
            toxicity,
            severe_toxicity,
            obscene,
            identity_attack,
            insult,
            threat,
            sexual_explicit,
            coercion,
            location_cues: cues.location_cues.len(),
            reasons,
            degraded: Vec::new(),
        })
    }
}

/// Preprocess raw image bytes for the ViT NSFW model:
/// 1. Validates image MIME and dimensions using hre_vision::validate_image.
/// 2. Decodes image into RGB format.
/// 3. Resizes to 224x224 using bilinear interpolation (Triangle filter).
/// 4. Normalizes to [-1.0, 1.0] with mean=[0.5, 0.5, 0.5], std=[0.5, 0.5, 0.5] in NCHW order.
pub fn preprocess_image(bytes: &[u8], declared_mime: &str) -> Result<Array4<f32>, VisionError> {
    let _meta = validate_image(bytes, declared_mime)?;
    let dynamic_img = image::load_from_memory(bytes).map_err(|_| VisionError::TruncatedImage)?;
    let rgb = dynamic_img
        .resize_exact(224, 224, image::imageops::FilterType::Triangle)
        .to_rgb8();

    let mut tensor = Array4::<f32>::zeros((1, 3, 224, 224));
    for y in 0..224 {
        for x in 0..224 {
            let p = rgb.get_pixel(x as u32, y as u32);
            // ViT Normalization: (pixel / 255.0 - 0.5) / 0.5
            tensor[[0, 0, y, x]] = (p[0] as f32 / 255.0 - 0.5) / 0.5;
            tensor[[0, 1, y, x]] = (p[1] as f32 / 255.0 - 0.5) / 0.5;
            tensor[[0, 2, y, x]] = (p[2] as f32 / 255.0 - 0.5) / 0.5;
        }
    }
    Ok(tensor)
}

/// Thread-safe ONNX model wrapper for NSFW image classification (Falconsai/nsfw_image_detection).
pub struct OnnxNsfwModel {
    session: Mutex<Session>,
}

impl std::fmt::Debug for OnnxNsfwModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnnxNsfwModel").finish_non_exhaustive()
    }
}

impl OnnxNsfwModel {
    /// Load an ONNX NSFW model from disk.
    pub fn load(model_path: &Path) -> Result<Self, ModelError> {
        let env = get_or_init_ort_env()?;
        let session = Session::builder(env)
            .map_err(|e| ModelError::Inference(format!("session builder error: {e}")))?
            .with_optimization_level(GraphOptimizationLevel::Level1)
            .map_err(|e| ModelError::Inference(format!("optimization level error: {e}")))?
            .commit_from_file(model_path)
            .map_err(|e| {
                ModelError::Inference(format!(
                    "failed to load ONNX NSFW model from {}: {e}",
                    model_path.display()
                ))
            })?;
        Ok(Self {
            session: Mutex::new(session),
        })
    }

    /// Predict the [normal, nsfw] probabilities for the input image tensor [1, 3, 224, 224].
    pub fn predict(&self, tensor: Array4<f32>) -> Result<[f32; 2], ModelError> {
        if tensor.shape() != [1, 3, 224, 224] {
            return Err(ModelError::Invalid(
                "expected input tensor shape [1, 3, 224, 224]",
            ));
        }
        if tensor.iter().any(|v| !v.is_finite()) {
            return Err(ModelError::Invalid("non-finite value in image tensor"));
        }
        let input_tensor = Tensor::from_array(tensor)
            .map_err(|e| ModelError::Inference(format!("failed to create image tensor: {e}")))?;

        let mut session = self
            .session
            .lock()
            .map_err(|_| ModelError::Inference("ONNX nsfw session mutex poisoned".into()))?;

        let outputs = session
            .run(ort::inputs!["pixel_values" => input_tensor])
            .map_err(|e| ModelError::Inference(format!("image inference run error: {e}")))?;

        let probs = outputs["probabilities"]
            .try_extract_tensor::<f32>()
            .map_err(|e| {
                ModelError::Inference(format!("failed to extract image probabilities: {e}"))
            })?;
        let slice = probs.1;

        if slice.len() != 2 {
            return Err(ModelError::Inference(format!(
                "expected 2 output probabilities, got {}",
                slice.len()
            )));
        }

        let normal = slice[0];
        let nsfw = slice[1];
        if !normal.is_finite()
            || !nsfw.is_finite()
            || !(0.0..=1.0).contains(&normal)
            || !(0.0..=1.0).contains(&nsfw)
        {
            return Err(ModelError::Inference(format!(
                "output probability out of range: normal={normal}, nsfw={nsfw}"
            )));
        }
        Ok([normal, nsfw])
    }
}

/// Pluggable ONNX image inference backend implementing `hre_vision::ImageBackend`.
#[derive(Debug, Clone)]
pub struct OnnxImageBackend {
    model: Arc<OnnxNsfwModel>,
    threshold: f64,
    fallback: HeuristicImageBackend,
}

impl OnnxImageBackend {
    /// Create a new ONNX image backend.
    pub fn new(model: Arc<OnnxNsfwModel>, threshold: f64, fallback: HeuristicImageBackend) -> Self {
        Self {
            model,
            threshold,
            fallback,
        }
    }

    /// Load the model from file and wrap with fallback.
    pub fn load(
        model_path: &Path,
        threshold: f64,
        fallback: HeuristicImageBackend,
    ) -> Result<Self, ModelError> {
        let model = Arc::new(OnnxNsfwModel::load(model_path)?);
        Ok(Self::new(model, threshold, fallback))
    }
}

impl ImageBackend for OnnxImageBackend {
    fn analyze(&self, bytes: &[u8], declared_mime: &str) -> Result<ImageAnalysis, VisionError> {
        // If demo override is configured and matched in fallback, honor simulated verdict immediately.
        let fallback_analysis = self.fallback.analyze(bytes, declared_mime)?;
        if fallback_analysis.simulated {
            return Ok(fallback_analysis);
        }

        let tensor = preprocess_image(bytes, declared_mime)?;
        let probs = match self.model.predict(tensor) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(
                    "ONNX image inference failed ({e}); falling back to heuristic backend"
                );
                return Ok(fallback_analysis);
            }
        };

        let normal_score = f64::from(probs[0]);
        let nsfw_score = f64::from(probs[1]);
        let recommended_action = if nsfw_score >= self.threshold {
            Some(hre_core::Action::BlurMedia)
        } else {
            None
        };

        let mut details = BTreeMap::new();
        details.insert("normal".into(), serde_json::json!(normal_score));
        details.insert("nsfw".into(), serde_json::json!(nsfw_score));
        details.insert(
            "sha256".into(),
            serde_json::json!(&fallback_analysis.sha256),
        );
        details.insert("p_hash".into(), serde_json::json!(fallback_analysis.p_hash));
        details.insert(
            "model_id".into(),
            serde_json::json!("Falconsai/nsfw_image_detection"),
        );

        let label = if nsfw_score >= self.threshold {
            "nsfw".to_string()
        } else {
            "normal".to_string()
        };

        let confidence = if nsfw_score >= self.threshold {
            nsfw_score
        } else {
            normal_score
        };

        Ok(ImageAnalysis {
            signal: Signal {
                score: nsfw_score,
                label,
                status: Status::Built,
                backend: Backend::Onnx,
                confidence,
                details,
            },
            p_hash: fallback_analysis.p_hash,
            sha256: fallback_analysis.sha256,
            recommended_action,
            simulated: false,
            degraded: Vec::new(),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::excessive_precision)]
mod tests {
    use super::*;
    use sha2::Digest;

    const GOLDEN_MSG_FEATURES: [f32; 47] = [
        3.0, 0.25, 1.0, 0.2, 0.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // 1h
        3.0, 0.25, 1.0, 0.2, 0.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // 24h
        3.0, 0.25, 1.0, 0.2, 0.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // 7d
        1.0, 1.0, 90.0, 3.0, 0.0, -1.0, 1.0, 0.1, 0.0, 1.0, -1.0, 0.0, 1.0, 0.0,
    ];

    const PYTHON_GOLDEN_MSG_PROB: f32 = 0.002_782_583_2;
    const PYTHON_ZERO_MSG_PROB: f32 = 0.001_337_289_8;

    // Golden values from Python for text phrases
    const PYTHON_BENIGN_PROBS: [f32; 11] = [
        0.165136, 0.097729, 0.070734, 0.105954, 0.141483, 0.111232, 0.076977, 0.150679, 0.153005,
        0.154070, 0.100611,
    ];

    const PYTHON_THREAT_PROBS: [f32; 11] = [
        0.807850, 0.414337, 0.211157, 0.432461, 0.671688, 0.254576, 0.134756, 0.397271, 0.368611,
        0.224740, 0.364127,
    ];

    fn find_models_dir() -> Option<PathBuf> {
        let candidates = [
            PathBuf::from("models"),
            PathBuf::from("../../models"),
            PathBuf::from("../../../models"),
        ];
        candidates.into_iter().find(|c| {
            c.join("behavior_msg.onnx").exists()
                && c.join("text_fine_tuned.onnx").exists()
                && c.join("nsfw.onnx").exists()
        })
    }

    const PYTHON_SYNTHETIC_NORMAL: f32 = 0.99960452;
    const PYTHON_SYNTHETIC_NSFW: f32 = 0.00039555;

    fn valid_test_png() -> Vec<u8> {
        vec![
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 2,
            8, 2, 0, 0, 0, 253, 212, 154, 115, 0, 0, 0, 16, 73, 68, 65, 84, 120, 156, 99, 252, 207,
            0, 2, 76, 96, 146, 1, 0, 13, 29, 1, 3, 130, 201, 113, 255, 0, 0, 0, 0, 73, 69, 78, 68,
            174, 66, 96, 130,
        ]
    }

    #[test]
    fn test_preprocess_image_shape_and_range() {
        let png = valid_test_png();
        let tensor = preprocess_image(&png, "image/png").expect("preprocess valid png");
        assert_eq!(tensor.shape(), [1, 3, 224, 224]);
        for &val in tensor.iter() {
            assert!(
                (-1.01..=1.01).contains(&val),
                "preprocessed value out of range: {val}"
            );
        }
    }

    #[test]
    fn test_preprocess_image_rejects_mime_mismatch_and_truncated() {
        let png = hre_vision::fixtures::minimal_png(10, 10);
        assert!(matches!(
            preprocess_image(&png, "image/jpeg"),
            Err(VisionError::MimeMismatch { .. })
        ));

        let truncated = vec![0x89, b'P', b'N'];
        assert_eq!(
            preprocess_image(&truncated, "image/png"),
            Err(VisionError::TruncatedImage)
        );
    }

    #[test]
    fn test_python_rust_parity_nsfw_synthetic_image() {
        let models_dir = match find_models_dir() {
            Some(d) => d,
            None => return,
        };
        let model_path = models_dir.join("nsfw.onnx");
        if !model_path.exists() {
            return;
        }
        let test_img_path = models_dir.join("synthetic_test.png");
        if !test_img_path.exists() {
            return;
        }
        let img_bytes = std::fs::read(&test_img_path).expect("read test image");
        let tensor = preprocess_image(&img_bytes, "image/png").expect("preprocess synthetic test");
        let model = OnnxNsfwModel::load(&model_path).expect("load nsfw model");
        let probs = model.predict(tensor).expect("predict nsfw");

        let diff_normal = (probs[0] - PYTHON_SYNTHETIC_NORMAL).abs();
        let diff_nsfw = (probs[1] - PYTHON_SYNTHETIC_NSFW).abs();

        assert!(
            diff_normal < 0.01,
            "normal parity failed: rust={}, python={}, diff={}",
            probs[0],
            PYTHON_SYNTHETIC_NORMAL,
            diff_normal
        );
        assert!(
            diff_nsfw < 0.01,
            "nsfw parity failed: rust={}, python={}, diff={}",
            probs[1],
            PYTHON_SYNTHETIC_NSFW,
            diff_nsfw
        );
    }

    #[test]
    fn test_onnx_image_backend_normal_and_action() {
        let models_dir = match find_models_dir() {
            Some(d) => d,
            None => return,
        };
        let model_path = models_dir.join("nsfw.onnx");
        if !model_path.exists() {
            return;
        }
        let fallback = HeuristicImageBackend::new();
        let backend =
            OnnxImageBackend::load(&model_path, 0.50, fallback).expect("load onnx image backend");

        let png = valid_test_png();
        let analysis = backend.analyze(&png, "image/png").expect("analyze png");

        assert_eq!(analysis.signal.backend, Backend::Onnx);
        assert_eq!(analysis.signal.status, Status::Built);
        assert_eq!(analysis.signal.label, "normal");
        assert!(!analysis.simulated);
        assert!(analysis.signal.score < 0.10);
        assert_eq!(analysis.recommended_action, None);
        assert!(analysis.degraded.is_empty());
    }

    #[test]
    fn test_onnx_image_backend_demo_override_is_simulated() {
        let models_dir = match find_models_dir() {
            Some(d) => d,
            None => return,
        };
        let model_path = models_dir.join("nsfw.onnx");
        if !model_path.exists() {
            return;
        }
        let png = valid_test_png();
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, &png);
        let sha256 = format!("{:x}", sha2::Digest::finalize(hasher));

        let fallback = HeuristicImageBackend::new().with_override(&sha256, 0.95);
        let backend =
            OnnxImageBackend::load(&model_path, 0.50, fallback).expect("load onnx image backend");

        let analysis = backend
            .analyze(&png, "image/png")
            .expect("analyze png with override");
        assert_eq!(analysis.signal.status, Status::Simulated);
        assert!(analysis.simulated);
        assert_eq!(
            analysis.recommended_action,
            Some(hre_core::Action::BlurMedia)
        );
        assert!(analysis
            .degraded
            .iter()
            .any(|d| d == "image.demo_override_simulated"));
    }

    #[test]
    fn test_fallback_corrupt_nsfw_model() {
        let dir = std::env::temp_dir().join(format!("hre-corrupt-nsfw-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let corrupt_path = dir.join("corrupt_nsfw.onnx");
        std::fs::write(&corrupt_path, b"not an onnx file").unwrap();

        let load_res = OnnxNsfwModel::load(&corrupt_path);
        assert!(
            load_res.is_err(),
            "corrupt nsfw model loading should fail gracefully"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_python_rust_parity_golden_message_vector() {
        let models_dir = match find_models_dir() {
            Some(d) => d,
            None => return,
        };
        let model_path = models_dir.join("behavior_msg.onnx");
        let model = OnnxBehaviorModel::load(&model_path, 47).expect("load message model");
        let prob = model
            .predict(&GOLDEN_MSG_FEATURES)
            .expect("predict golden message");
        let diff = (prob - PYTHON_GOLDEN_MSG_PROB).abs();
        assert!(
            diff < 1e-4,
            "golden message parity failed: rust={prob}, python={PYTHON_GOLDEN_MSG_PROB}, diff={diff}"
        );
    }

    #[test]
    fn test_python_rust_parity_zero_message_vector() {
        let models_dir = match find_models_dir() {
            Some(d) => d,
            None => return,
        };
        let model_path = models_dir.join("behavior_msg.onnx");
        let model = OnnxBehaviorModel::load(&model_path, 47).expect("load message model");
        let zeros = [0.0f32; 47];
        let prob = model.predict(&zeros).expect("predict zero message");
        let diff = (prob - PYTHON_ZERO_MSG_PROB).abs();
        assert!(
            diff < 1e-4,
            "zero message parity failed: rust={prob}, python={PYTHON_ZERO_MSG_PROB}, diff={diff}"
        );
    }

    #[test]
    fn test_python_rust_parity_text_model_benign() {
        let models_dir = match find_models_dir() {
            Some(d) => d,
            None => return,
        };
        let model_path = models_dir.join("text_fine_tuned.onnx");
        let tokenizer_path = models_dir.join("tokenizer.json");
        let model = OnnxTextModel::load(&model_path, &tokenizer_path).expect("load text model");

        let phrase = "Hello world! Hope you have a wonderful day.";
        let probs = model.predict(phrase).expect("predict benign phrase");

        for i in 0..11 {
            let diff = (probs[i] - PYTHON_BENIGN_PROBS[i]).abs();
            assert!(
                diff < 0.01,
                "label {} parity failed: rust={}, python={}, diff={}",
                TEXT_MODEL_LABELS[i],
                probs[i],
                PYTHON_BENIGN_PROBS[i],
                diff
            );
        }
    }

    #[test]
    fn test_python_rust_parity_text_model_threat() {
        let models_dir = match find_models_dir() {
            Some(d) => d,
            None => return,
        };
        let model_path = models_dir.join("text_fine_tuned.onnx");
        let tokenizer_path = models_dir.join("tokenizer.json");
        let model = OnnxTextModel::load(&model_path, &tokenizer_path).expect("load text model");

        let phrase = "I will find you and kill you, you worthless piece of trash!";
        let probs = model.predict(phrase).expect("predict threat phrase");

        for i in 0..11 {
            let diff = (probs[i] - PYTHON_THREAT_PROBS[i]).abs();
            assert!(
                diff < 0.01,
                "label {} parity failed: rust={}, python={}, diff={}",
                TEXT_MODEL_LABELS[i],
                probs[i],
                PYTHON_THREAT_PROBS[i],
                diff
            );
        }
    }

    #[test]
    fn test_onnx_text_backend_threat_report() {
        let models_dir = match find_models_dir() {
            Some(d) => d,
            None => return,
        };
        let mut thresholds = BTreeMap::new();
        thresholds.insert("decision".into(), 0.325);
        thresholds.insert("threat".into(), 0.45);
        let backend = OnnxTextBackend::load_from_dir(&models_dir, thresholds)
            .expect("load onnx text backend");

        let report = backend
            .analyze("I will find you and kill you, you worthless piece of trash!")
            .expect("analyze threat");

        assert_eq!(report.primary_signal.backend, Backend::Onnx);
        assert_eq!(report.primary_signal.status, Status::Built);
        assert!(report.toxicity > 0.70);
        assert!(report.threat > 0.60);
        assert!(report
            .reasons
            .iter()
            .any(|r| r.code == "DIRECT_THREAT_DETECTED"));
        assert!(report.degraded.is_empty());
    }

    #[test]
    fn test_fallback_corrupt_model() {
        let dir = std::env::temp_dir().join(format!("hre-corrupt-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let corrupt_path = dir.join("corrupt.onnx");
        std::fs::write(&corrupt_path, b"not an onnx file").unwrap();

        let load_res = OnnxBehaviorModel::load(&corrupt_path, 47);
        assert!(
            load_res.is_err(),
            "corrupt model loading should fail gracefully"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
