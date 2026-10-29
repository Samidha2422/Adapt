//! Safe ONNX Runtime loader, execution, and backends for behavior and text models.

use crate::{InferenceBackend, ModelError};
use hre_audio::{AudioBackend, AudioError, AudioReport, AudioTranscription, HeuristicAudioBackend};
use hre_core::{Backend, Reason, Signal, Status};
use hre_text::{
    cues::detect_cues,
    normalizer::{normalize, MAX_TEXT_LENGTH_BYTES},
    pii::mask_pii,
    TextBackend, TextError, TextReport,
};
use hre_vision::{
    validate_image, HeuristicImageBackend, HeuristicOcrBackend, ImageAnalysis, ImageBackend,
    OcrBackend, OcrReport, VisionError,
};
use ndarray::{Array2, Array4};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::{Tensor, TensorElementType, ValueType};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

static ORT_ENV: OnceLock<Result<ort::environment::Environment, String>> = OnceLock::new();

fn validate_tensor(
    dtype: &ValueType,
    expected_type: TensorElementType,
    expected_shape: &[i64],
) -> bool {
    matches!(dtype, ValueType::Tensor { ty, shape, dimension_symbols }
        if *ty == expected_type && &shape[..] == expected_shape
            && dimension_symbols.iter().all(String::is_empty))
}

fn validate_session(
    session: &Session,
    inputs: &[(&str, TensorElementType, &[i64])],
    labels: i64,
) -> Result<(), ModelError> {
    if session.inputs().len() != inputs.len()
        || inputs.iter().any(|(name, ty, shape)| {
            !session
                .inputs()
                .iter()
                .any(|input| input.name() == *name && validate_tensor(input.dtype(), *ty, shape))
        })
        || session.outputs().len() != 1
        || session.outputs()[0].name() != "probabilities"
        || !validate_tensor(
            session.outputs()[0].dtype(),
            TensorElementType::Float32,
            &[1, labels],
        )
    {
        return Err(ModelError::Invalid(
            "ONNX tensor metadata does not match runtime contract",
        ));
    }
    Ok(())
}

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
        if p.is_file() {
            return Some(p);
        }
        return None;
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
        if p.is_file() {
            return p.canonicalize().ok();
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let p = parent.join("onnxruntime.dll");
            if p.is_file() {
                return Some(p);
            }
            for ancestor in parent.ancestors().take(5) {
                let p = ancestor.join(".venv/Lib/site-packages/onnxruntime/capi/onnxruntime.dll");
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    if let Some(system_root) = std::env::var_os("SystemRoot") {
        let p = PathBuf::from(system_root).join("System32/onnxruntime.dll");
        if p.is_file() {
            return Some(p);
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
        if !matches!(expected_features, 8 | 47) {
            return Err(ModelError::Invalid("unsupported behavior feature count"));
        }
        if crate::is_proxy_artifact(model_path, "behavior_messages")? {
            return Err(ModelError::Invalid("proxy behavior model"));
        }
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
        validate_session(
            &session,
            &[(
                "features",
                TensorElementType::Float32,
                &[1, expected_features as i64],
            )],
            2,
        )?;
        let model = Self {
            session: Mutex::new(session),
            expected_features,
        };
        model.predict(&vec![0.0; expected_features])?;
        Ok(model)
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

        let probs = outputs
            .get("probabilities")
            .ok_or(ModelError::Invalid("probabilities output missing"))?
            .try_extract_tensor::<f32>()
            .map_err(|e| ModelError::Inference(format!("failed to extract probabilities: {e}")))?;
        let slice = probs.1;

        if slice.len() != 2 || probs.0[..] != [1, 2] {
            return Err(ModelError::Inference(format!(
                "expected [1,2] probabilities, got {} values",
                slice.len()
            )));
        }
        if slice
            .iter()
            .any(|p| !p.is_finite() || !(0.0..=1.0).contains(p))
            || (slice.iter().sum::<f32>() - 1.0).abs() > 0.001
        {
            return Err(ModelError::Invalid(
                "invalid binary probability distribution",
            ));
        }
        Ok(slice[1])
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
        if crate::is_proxy_artifact(model_path, "text_fine_tuned")? {
            return Err(ModelError::Invalid("proxy text model"));
        }
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
        validate_session(
            &session,
            &[
                ("input_ids", TensorElementType::Int64, &[1, 128]),
                ("attention_mask", TensorElementType::Int64, &[1, 128]),
            ],
            11,
        )?;
        let mut tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path).map_err(|e| {
            ModelError::Inference(format!(
                "failed to load tokenizer from {}: {e}",
                tokenizer_path.display()
            ))
        })?;
        if tokenizer.token_to_id("<pad>") != Some(1)
            || tokenizer.token_to_id("<s>") != Some(0)
            || tokenizer.token_to_id("</s>") != Some(2)
        {
            return Err(ModelError::Invalid("incompatible XLM-R special tokens"));
        }
        tokenizer.with_padding(None);
        tokenizer
            .with_truncation(Some(tokenizers::TruncationParams {
                max_length: 128,
                ..Default::default()
            }))
            .map_err(|e| ModelError::Inference(format!("tokenizer truncation failed: {e}")))?;
        let model = Self {
            session: Mutex::new(session),
            tokenizer,
        };
        model.predict("Hello world")?;
        Ok(model)
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

        // Tokenizer truncation preserves the terminal special token before fixed padding.
        if ids_vec.len() < 128 {
            ids_vec.resize(128, 1);
            mask_vec.resize(128, 0);
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

        let probs = outputs
            .get("probabilities")
            .ok_or(ModelError::Invalid("probabilities output missing"))?
            .try_extract_tensor::<f32>()
            .map_err(|e| {
                ModelError::Inference(format!("failed to extract text probabilities: {e}"))
            })?;
        let slice = probs.1;

        if slice.len() != 11 || probs.0[..] != [1, 11] {
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
        let manifest = crate::ModelManifest::parse(
            &std::fs::read_to_string(dir.join("manifest.json"))
                .map_err(|_| ModelError::Invalid("manifest unavailable"))?,
        )?;
        let entry = manifest.validated_entry(dir, "text_fine_tuned")?;
        let model_path = dir.join(&entry.path);
        let tokenizer_path = dir.join(
            entry
                .tokenizer
                .as_deref()
                .ok_or(ModelError::Invalid("text tokenizer missing"))?,
        );
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
        let scores = match self.model.predict(&normalized) {
            Ok(scores) => scores,
            Err(_) => {
                let mut report = hre_text::HeuristicTextBackend.analyze(raw_text)?;
                report
                    .degraded
                    .push("text_fine_tuned.inference_failed".into());
                return Ok(report);
            }
        };

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

/// Thread-safe ONNX model wrapper for Deepfake image classification (dima806/deepfake_vs_real_image_detection).
pub struct OnnxDeepfakeModel {
    session: Mutex<Session>,
}

impl std::fmt::Debug for OnnxDeepfakeModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnnxDeepfakeModel").finish_non_exhaustive()
    }
}

impl OnnxDeepfakeModel {
    pub fn load(model_path: &Path) -> Result<Self, ModelError> {
        let env = get_or_init_ort_env()?;
        let session = Session::builder(env)
            .map_err(|e| ModelError::Inference(format!("session builder error: {e}")))?
            .with_optimization_level(GraphOptimizationLevel::Level1)
            .map_err(|e| ModelError::Inference(format!("optimization level error: {e}")))?
            .commit_from_file(model_path)
            .map_err(|e| {
                ModelError::Inference(format!(
                    "failed to load ONNX Deepfake model from {}: {e}",
                    model_path.display()
                ))
            })?;
        Ok(Self {
            session: Mutex::new(session),
        })
    }

    /// Predict [authentic, manipulated] probabilities for input tensor [1, 3, 224, 224].
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
            .map_err(|_| ModelError::Inference("ONNX deepfake session mutex poisoned".into()))?;

        let outputs = session
            .run(ort::inputs!["pixel_values" => input_tensor])
            .map_err(|e| ModelError::Inference(format!("deepfake inference run error: {e}")))?;

        let probs = outputs["probabilities"]
            .try_extract_tensor::<f32>()
            .map_err(|e| {
                ModelError::Inference(format!("failed to extract deepfake probabilities: {e}"))
            })?;
        let slice = probs.1;
        if slice.len() != 2 {
            return Err(ModelError::Inference(format!(
                "expected 2 output probabilities, got {}",
                slice.len()
            )));
        }
        let authentic = slice[0];
        let manipulated = slice[1];
        if !authentic.is_finite()
            || !manipulated.is_finite()
            || !(0.0..=1.0).contains(&authentic)
            || !(0.0..=1.0).contains(&manipulated)
        {
            return Err(ModelError::Inference(format!(
                "output probability out of range: authentic={authentic}, manipulated={manipulated}"
            )));
        }
        Ok([authentic, manipulated])
    }

    /// Convenience helper that pre-processes image bytes and predicts [authentic, manipulated].
    pub fn predict_image(
        &self,
        bytes: &[u8],
        declared_mime: &str,
    ) -> Result<[f32; 2], VisionError> {
        let tensor = preprocess_image(bytes, declared_mime)?;
        self.predict(tensor)
            .map_err(|e| VisionError::Ocr(e.to_string()))
    }
}

/// Bounding box representation for detected text regions in original image coordinates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextBoundingBox {
    pub x_min: u32,
    pub y_min: u32,
    pub x_max: u32,
    pub y_max: u32,
    pub score: f32,
}

impl TextBoundingBox {
    pub fn new(x_min: u32, y_min: u32, x_max: u32, y_max: u32, score: f32) -> Self {
        Self {
            x_min,
            y_min,
            x_max,
            y_max,
            score,
        }
    }

    pub fn width(&self) -> u32 {
        self.x_max.saturating_sub(self.x_min)
    }

    pub fn height(&self) -> u32 {
        self.y_max.saturating_sub(self.y_min)
    }

    pub fn area(&self) -> u32 {
        self.width() * self.height()
    }

    pub fn iou(&self, other: &Self) -> f32 {
        let x1 = self.x_min.max(other.x_min);
        let y1 = self.y_min.max(other.y_min);
        let x2 = self.x_max.min(other.x_max);
        let y2 = self.y_max.min(other.y_max);

        if x2 <= x1 || y2 <= y1 {
            return 0.0;
        }

        let inter_area = ((x2 - x1) * (y2 - y1)) as f32;
        let union_area = (self.area() + other.area()) as f32 - inter_area;
        if union_area <= 0.0 {
            0.0
        } else {
            inter_area / union_area
        }
    }
}

/// Sort bounding boxes into natural reading order:
/// Groups boxes into horizontal lines, sorts lines top-to-bottom,
/// and sorts boxes within each line left-to-right.
pub fn sort_reading_order(boxes: &mut [TextBoundingBox]) {
    if boxes.len() <= 1 {
        return;
    }

    // 1. Initial sort by y_min
    boxes.sort_by_key(|b| b.y_min);

    // 2. Line clustering
    let mut lines: Vec<Vec<TextBoundingBox>> = Vec::new();
    for b in boxes.iter() {
        let b_yc = (b.y_min + b.y_max) as f32 / 2.0;
        let b_h = b.height() as f32;
        let mut placed = false;

        for line in lines.iter_mut() {
            let line_avg_yc = line
                .iter()
                .map(|item| (item.y_min + item.y_max) as f32 / 2.0)
                .sum::<f32>()
                / line.len() as f32;
            let line_avg_h =
                line.iter().map(|item| item.height() as f32).sum::<f32>() / line.len() as f32;
            let threshold = 0.5 * b_h.min(line_avg_h).max(4.0);

            if (b_yc - line_avg_yc).abs() < threshold {
                line.push(b.clone());
                placed = true;
                break;
            }
        }

        if !placed {
            lines.push(vec![b.clone()]);
        }
    }

    // 3. Sort each line left-to-right, and lines top-to-bottom
    lines.sort_by(|l1, l2| {
        let y1 = l1.iter().map(|b| b.y_min).min().unwrap_or(0);
        let y2 = l2.iter().map(|b| b.y_min).min().unwrap_or(0);
        y1.cmp(&y2)
    });

    let mut result = Vec::with_capacity(boxes.len());
    for line in lines.iter_mut() {
        line.sort_by_key(|b| b.x_min);
        result.append(line);
    }

    boxes.clone_from_slice(&result);
}

/// Preprocess image for PP-OCRv4 text detection (DBNet):
/// Limits maximum side to 960, scales height and width to multiples of 32,
/// and applies ImageNet normalization (mean [0.485, 0.456, 0.406], std [0.229, 0.224, 0.225]).
pub fn preprocess_ocr_detector(
    img: &image::DynamicImage,
) -> Result<(Array4<f32>, (u32, u32)), VisionError> {
    let (orig_w, orig_h) = (img.width(), img.height());
    if orig_w < 16 || orig_h < 16 {
        return Err(VisionError::InvalidDimensions);
    }

    let max_side = orig_w.max(orig_h);
    let ratio = if max_side > 960 {
        960.0 / max_side as f32
    } else {
        1.0
    };

    let scaled_w = (((orig_w as f32 * ratio / 32.0).round() as u32) * 32).clamp(32, 960);
    let scaled_h = (((orig_h as f32 * ratio / 32.0).round() as u32) * 32).clamp(32, 960);

    let resized = img
        .resize_exact(scaled_w, scaled_h, image::imageops::FilterType::Triangle)
        .to_rgb8();

    let mean = [0.485f32, 0.456f32, 0.406f32];
    let std = [0.229f32, 0.224f32, 0.225f32];

    let mut tensor = Array4::<f32>::zeros((1, 3, scaled_h as usize, scaled_w as usize));
    for y in 0..scaled_h {
        for x in 0..scaled_w {
            let p = resized.get_pixel(x, y);
            for c in 0..3 {
                let v = p[c] as f32 / 255.0;
                tensor[[0, c, y as usize, x as usize]] = (v - mean[c]) / std[c];
            }
        }
    }

    Ok((tensor, (scaled_w, scaled_h)))
}

/// Postprocess DBNet probability map:
/// Binarizes with `thresh`, performs connected component flood fill,
/// filters by `min_size` and `box_thresh`, unclips by `unclip_ratio`,
/// projects to original coordinates, applies NMS, and sorts into reading order.
#[allow(clippy::too_many_arguments)]
pub fn postprocess_dbnet(
    probs: &[f32],
    scaled_w: u32,
    scaled_h: u32,
    orig_w: u32,
    orig_h: u32,
    thresh: f32,
    box_thresh: f32,
    min_size: u32,
    unclip_ratio: f32,
) -> Vec<TextBoundingBox> {
    let w = scaled_w as usize;
    let h = scaled_h as usize;
    if probs.len() != w * h || w == 0 || h == 0 {
        return Vec::new();
    }

    let mut visited = vec![false; w * h];
    let mut raw_boxes = Vec::new();

    let neighbors: [(isize, isize); 8] = [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ];

    let mut queue = std::collections::VecDeque::new();

    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            if visited[idx] || probs[idx] <= thresh {
                continue;
            }

            visited[idx] = true;
            queue.push_back((x, y));

            let mut min_x = x;
            let mut max_x = x;
            let mut min_y = y;
            let mut max_y = y;
            let mut sum_score = 0.0f32;
            let mut pixel_count = 0usize;

            while let Some((cx, cy)) = queue.pop_front() {
                let p_idx = cy * w + cx;
                sum_score += probs[p_idx];
                pixel_count += 1;

                min_x = min_x.min(cx);
                max_x = max_x.max(cx);
                min_y = min_y.min(cy);
                max_y = max_y.max(cy);

                for (dx, dy) in neighbors {
                    let nx = cx as isize + dx;
                    let ny = cy as isize + dy;
                    if nx >= 0 && nx < w as isize && ny >= 0 && ny < h as isize {
                        let n_idx = ny as usize * w + nx as usize;
                        if !visited[n_idx] && probs[n_idx] > thresh {
                            visited[n_idx] = true;
                            queue.push_back((nx as usize, ny as usize));
                        }
                    }
                }
            }

            if pixel_count < 8 {
                continue;
            }

            let comp_w = (max_x - min_x + 1) as u32;
            let comp_h = (max_y - min_y + 1) as u32;
            if comp_w < min_size || comp_h < min_size {
                continue;
            }

            let mean_score = sum_score / pixel_count as f32;
            if mean_score < box_thresh {
                continue;
            }

            // Unclip expansion
            let comp_w_f = comp_w as f32;
            let comp_h_f = comp_h as f32;
            let dist = (comp_w_f * comp_h_f * unclip_ratio) / (2.0 * (comp_w_f + comp_h_f));

            let exp_x1 = (min_x as f32 - dist).max(0.0);
            let exp_x2 = (max_x as f32 + dist).min((w - 1) as f32);
            let exp_y1 = (min_y as f32 - dist).max(0.0);
            let exp_y2 = (max_y as f32 + dist).min((h - 1) as f32);

            let sx = orig_w as f32 / scaled_w as f32;
            let sy = orig_h as f32 / scaled_h as f32;

            let orig_x1 = ((exp_x1 * sx).round() as u32).min(orig_w.saturating_sub(1));
            let orig_x2 = ((exp_x2 * sx).round() as u32).min(orig_w);
            let orig_y1 = ((exp_y1 * sy).round() as u32).min(orig_h.saturating_sub(1));
            let orig_y2 = ((exp_y2 * sy).round() as u32).min(orig_h);

            if orig_x2 > orig_x1 + 3 && orig_y2 > orig_y1 + 3 {
                raw_boxes.push(TextBoundingBox::new(
                    orig_x1, orig_y1, orig_x2, orig_y2, mean_score,
                ));
            }
        }
    }

    raw_boxes.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut filtered_boxes: Vec<TextBoundingBox> = Vec::new();

    for candidate in raw_boxes {
        let mut keep = true;
        for kept in &filtered_boxes {
            if candidate.iou(kept) > 0.4 {
                keep = false;
                break;
            }
        }
        if keep {
            filtered_boxes.push(candidate);
        }
    }

    sort_reading_order(&mut filtered_boxes);
    filtered_boxes
}

/// Preprocess a dynamic image line crop for PP-OCRv4 text recognition:
/// Resizes preserving aspect ratio to height 48 (clamped to max width 320),
/// right-pads to 320 with white/background (1.0 in normalized space),
/// and normalizes to [-1.0, 1.0] in NCHW [1, 3, 48, 320].
pub fn preprocess_image_line(
    dynamic_img: &image::DynamicImage,
) -> Result<Array4<f32>, VisionError> {
    let (orig_w, orig_h) = (dynamic_img.width(), dynamic_img.height());
    if orig_w == 0 || orig_h == 0 {
        return Err(VisionError::InvalidDimensions);
    }
    let target_h = 48u32;
    let ratio = orig_w as f32 / orig_h as f32;
    let scaled_w = ((target_h as f32 * ratio).round() as u32).clamp(1, 320);
    let resized = dynamic_img
        .resize_exact(scaled_w, target_h, image::imageops::FilterType::Triangle)
        .to_rgb8();

    let mut tensor = Array4::<f32>::zeros((1, 3, 48, 320));
    tensor.fill(1.0); // right-side white padding
    for y in 0..48 {
        for x in 0..scaled_w {
            let p = resized.get_pixel(x, y);
            tensor[[0, 0, y as usize, x as usize]] = (p[0] as f32 / 255.0 - 0.5) / 0.5;
            tensor[[0, 1, y as usize, x as usize]] = (p[1] as f32 / 255.0 - 0.5) / 0.5;
            tensor[[0, 2, y as usize, x as usize]] = (p[2] as f32 / 255.0 - 0.5) / 0.5;
        }
    }
    Ok(tensor)
}

pub fn preprocess_ocr_line(bytes: &[u8], declared_mime: &str) -> Result<Array4<f32>, VisionError> {
    let _meta = validate_image(bytes, declared_mime)?;
    let dynamic_img = image::load_from_memory(bytes).map_err(|_| VisionError::TruncatedImage)?;
    preprocess_image_line(&dynamic_img)
}

/// Thread-safe ONNX model wrapper for PP-OCRv4 text detection (DBNet).
pub struct OnnxOcrDetector {
    session: Mutex<Session>,
}

impl std::fmt::Debug for OnnxOcrDetector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnnxOcrDetector").finish_non_exhaustive()
    }
}

impl OnnxOcrDetector {
    pub fn load(model_path: &Path) -> Result<Self, ModelError> {
        let env = get_or_init_ort_env()?;
        let session = Session::builder(env)
            .map_err(|e| ModelError::Inference(format!("session builder error: {e}")))?
            .with_optimization_level(GraphOptimizationLevel::Level1)
            .map_err(|e| ModelError::Inference(format!("optimization level error: {e}")))?
            .commit_from_file(model_path)
            .map_err(|e| {
                ModelError::Inference(format!(
                    "failed to load ONNX OCR detector model from {}: {e}",
                    model_path.display()
                ))
            })?;
        Ok(Self {
            session: Mutex::new(session),
        })
    }

    pub fn detect(&self, img: &image::DynamicImage) -> Result<Vec<TextBoundingBox>, ModelError> {
        let (tensor, (scaled_w, scaled_h)) = preprocess_ocr_detector(img)
            .map_err(|e| ModelError::Inference(format!("OCR detector preprocess error: {e}")))?;
        let (orig_w, orig_h) = (img.width(), img.height());

        let input_tensor = Tensor::from_array(tensor)
            .map_err(|e| ModelError::Inference(format!("failed to create detector tensor: {e}")))?;

        let mut session = self.session.lock().map_err(|_| {
            ModelError::Inference("ONNX OCR detector session mutex poisoned".into())
        })?;

        let outputs = session
            .run(ort::inputs!["x" => input_tensor])
            .map_err(|e| ModelError::Inference(format!("OCR detector inference run error: {e}")))?;

        let probs = outputs["sigmoid_0.tmp_0"]
            .try_extract_tensor::<f32>()
            .map_err(|e| {
                ModelError::Inference(format!("failed to extract sigmoid_0.tmp_0: {e}"))
            })?;
        let slice = probs.1;
        let boxes = postprocess_dbnet(slice, scaled_w, scaled_h, orig_w, orig_h, 0.3, 0.5, 3, 1.6);
        Ok(boxes)
    }
}

/// Thread-safe ONNX model wrapper for PP-OCRv4 text line recognition.
pub struct OnnxOcrModel {
    session: Mutex<Session>,
    vocab: Vec<String>,
}

impl std::fmt::Debug for OnnxOcrModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnnxOcrModel")
            .field("vocab_size", &self.vocab.len())
            .finish_non_exhaustive()
    }
}

impl OnnxOcrModel {
    pub fn load(model_path: &Path, vocab_path: &Path) -> Result<Self, ModelError> {
        let env = get_or_init_ort_env()?;
        let session = Session::builder(env)
            .map_err(|e| ModelError::Inference(format!("session builder error: {e}")))?
            .with_optimization_level(GraphOptimizationLevel::Level1)
            .map_err(|e| ModelError::Inference(format!("optimization level error: {e}")))?
            .commit_from_file(model_path)
            .map_err(|e| {
                ModelError::Inference(format!(
                    "failed to load ONNX OCR model from {}: {e}",
                    model_path.display()
                ))
            })?;
        let vocab_str = std::fs::read_to_string(vocab_path).map_err(|e| {
            ModelError::Inference(format!(
                "failed to read OCR vocab from {}: {e}",
                vocab_path.display()
            ))
        })?;
        #[derive(serde::Deserialize)]
        struct VocabFile {
            #[serde(default)]
            tokens: Vec<String>,
        }
        let vocab: Vec<String> = if let Ok(vf) = serde_json::from_str::<VocabFile>(&vocab_str) {
            if !vf.tokens.is_empty() {
                vf.tokens
            } else {
                serde_json::from_str(&vocab_str).map_err(|e| {
                    ModelError::Inference(format!("failed to parse OCR vocab JSON: {e}"))
                })?
            }
        } else {
            serde_json::from_str(&vocab_str).map_err(|e| {
                ModelError::Inference(format!("failed to parse OCR vocab JSON: {e}"))
            })?
        };
        Ok(Self {
            session: Mutex::new(session),
            vocab,
        })
    }

    /// Run OCR model on preprocessed tensor [1, 3, 48, 320] and decode with CTC greedy search.
    pub fn predict(&self, tensor: Array4<f32>) -> Result<(String, f64), ModelError> {
        if tensor.shape() != [1, 3, 48, 320] {
            return Err(ModelError::Invalid(
                "expected input tensor shape [1, 3, 48, 320]",
            ));
        }
        let input_tensor = Tensor::from_array(tensor)
            .map_err(|e| ModelError::Inference(format!("failed to create OCR tensor: {e}")))?;

        let mut session = self
            .session
            .lock()
            .map_err(|_| ModelError::Inference("ONNX OCR session mutex poisoned".into()))?;

        let outputs = session
            .run(ort::inputs!["pixel_values" => input_tensor])
            .map_err(|e| ModelError::Inference(format!("OCR inference run error: {e}")))?;

        let probs = outputs["token_probabilities"]
            .try_extract_tensor::<f32>()
            .map_err(|e| {
                ModelError::Inference(format!("failed to extract token_probabilities: {e}"))
            })?;
        let slice = probs.1; // [1 * 80 * 256]
        if slice.len() != 80 * 256 {
            return Err(ModelError::Inference(format!(
                "expected 20480 output values, got {}",
                slice.len()
            )));
        }

        let (text, conf) = decode_ctc(slice, 80, &self.vocab);
        Ok((text, conf))
    }

    pub fn recognize(
        &self,
        bytes: &[u8],
        declared_mime: &str,
    ) -> Result<(String, f64), VisionError> {
        let tensor = preprocess_ocr_line(bytes, declared_mime)?;
        self.predict(tensor)
            .map_err(|e| VisionError::Ocr(e.to_string()))
    }
}

fn decode_ctc(probs: &[f32], timesteps: usize, vocab: &[String]) -> (String, f64) {
    let num_classes = vocab.len();
    let mut decoded_tokens: Vec<usize> = Vec::new();
    let mut token_confs: Vec<f32> = Vec::new();
    let mut prev_token = 0usize;

    for t in 0..timesteps {
        let step_slice = &probs[t * num_classes..(t + 1) * num_classes];
        let mut best_class = 0;
        let mut best_prob = step_slice[0];
        for (c, &prob) in step_slice.iter().enumerate().skip(1) {
            if prob > best_prob {
                best_prob = prob;
                best_class = c;
            }
        }
        if best_class != prev_token {
            if best_class != 0 && best_class < vocab.len() {
                decoded_tokens.push(best_class);
                token_confs.push(best_prob);
            }
            prev_token = best_class;
        }
    }

    let text: String = decoded_tokens
        .iter()
        .filter_map(|&idx| vocab.get(idx))
        .cloned()
        .collect();
    let avg_conf = if token_confs.is_empty() {
        0.0
    } else {
        f64::from(token_confs.iter().sum::<f32>() / token_confs.len() as f32)
    };
    (text, avg_conf)
}

/// Pluggable ONNX OCR backend implementing `hre_vision::OcrBackend`.
/// Supports full document multi-line detection (PP-OCRv4 DBNet),
/// region cropping, line recognition (PP-OCRv4 rec), and CTC decoding.
#[derive(Debug, Clone)]
pub struct OnnxOcrBackend {
    detector: Option<Arc<OnnxOcrDetector>>,
    model: Option<Arc<OnnxOcrModel>>,
    fallback: HeuristicOcrBackend,
}

impl OnnxOcrBackend {
    pub fn new(model: Option<Arc<OnnxOcrModel>>, fallback: HeuristicOcrBackend) -> Self {
        Self {
            detector: None,
            model,
            fallback,
        }
    }

    pub fn with_detector(mut self, detector: Option<Arc<OnnxOcrDetector>>) -> Self {
        self.detector = detector;
        self
    }

    pub fn fallback_only(fallback: HeuristicOcrBackend) -> Self {
        Self {
            detector: None,
            model: None,
            fallback,
        }
    }

    pub fn load(
        model_path: &Path,
        vocab_path: &Path,
        fallback: HeuristicOcrBackend,
    ) -> Result<Self, ModelError> {
        let detector = if let Some(parent) = model_path.parent() {
            let det_path = parent.join("ocr_detector.onnx");
            if det_path.exists() {
                match OnnxOcrDetector::load(&det_path) {
                    Ok(d) => Some(Arc::new(d)),
                    Err(e) => {
                        tracing::warn!(
                            "Failed to load ONNX OCR detector ({e}); proceeding with line recognizer only"
                        );
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };
        let model = Arc::new(OnnxOcrModel::load(model_path, vocab_path)?);
        Ok(Self {
            detector,
            model: Some(model),
            fallback,
        })
    }

    pub fn load_pipeline(
        detector_path: &Path,
        model_path: &Path,
        vocab_path: &Path,
        fallback: HeuristicOcrBackend,
    ) -> Result<Self, ModelError> {
        let detector = if detector_path.exists() {
            Some(Arc::new(OnnxOcrDetector::load(detector_path)?))
        } else {
            None
        };
        let model = Arc::new(OnnxOcrModel::load(model_path, vocab_path)?);
        Ok(Self {
            detector,
            model: Some(model),
            fallback,
        })
    }

    pub fn process_image(
        &self,
        bytes: &[u8],
        declared_mime: &str,
    ) -> Result<OcrReport, VisionError> {
        let _meta = validate_image(bytes, declared_mime)?;

        // 1. Try detector + recognizer pipeline if both are available
        if let (Some(detector), Some(model)) = (&self.detector, &self.model) {
            if let Ok(dynamic_img) = image::load_from_memory(bytes) {
                if let Ok(boxes) = detector.detect(&dynamic_img) {
                    if !boxes.is_empty() {
                        let mut recognized_lines: Vec<(String, f64, TextBoundingBox)> = Vec::new();
                        for b in &boxes {
                            let crop_w = b.width().min(dynamic_img.width().saturating_sub(b.x_min));
                            let crop_h =
                                b.height().min(dynamic_img.height().saturating_sub(b.y_min));
                            if crop_w < 4 || crop_h < 4 {
                                continue;
                            }
                            let crop = dynamic_img.crop_imm(b.x_min, b.y_min, crop_w, crop_h);
                            if let Ok(tensor) = preprocess_image_line(&crop) {
                                if let Ok((line_text, line_conf)) = model.predict(tensor) {
                                    let trimmed = line_text.trim();
                                    if !trimmed.is_empty() {
                                        recognized_lines.push((
                                            trimmed.to_string(),
                                            line_conf,
                                            b.clone(),
                                        ));
                                    }
                                }
                            }
                        }

                        if !recognized_lines.is_empty() {
                            let raw_text = recognized_lines
                                .iter()
                                .map(|(t, _, _)| t.as_str())
                                .collect::<Vec<_>>()
                                .join("\n");
                            let avg_conf = recognized_lines.iter().map(|(_, c, _)| *c).sum::<f64>()
                                / recognized_lines.len() as f64;

                            let report = hre_text::analyze(&raw_text)
                                .map_err(|err| VisionError::Ocr(err.to_string()))?;
                            let masked_text = report.masked_text.clone();
                            let mut details = BTreeMap::new();
                            details.insert(
                                "ocr_text_masked".into(),
                                serde_json::json!(masked_text.clone()),
                            );
                            details.insert("confidence".into(), serde_json::json!(avg_conf));
                            details
                                .insert("detector_model".into(), serde_json::json!("PP-OCRv4_det"));
                            details.insert(
                                "recognizer_model".into(),
                                serde_json::json!("PP-OCRv4_rec"),
                            );
                            details.insert(
                                "box_count".into(),
                                serde_json::json!(recognized_lines.len()),
                            );
                            let boxes_json: Vec<serde_json::Value> = recognized_lines
                                .iter()
                                .map(|(txt, conf, b)| {
                                    serde_json::json!({
                                        "text": txt,
                                        "confidence": conf,
                                        "x_min": b.x_min,
                                        "y_min": b.y_min,
                                        "x_max": b.x_max,
                                        "y_max": b.y_max,
                                    })
                                })
                                .collect();
                            details.insert("detected_boxes".into(), serde_json::json!(boxes_json));

                            let score = report
                                .primary_signal
                                .score
                                .max(report.toxicity)
                                .max(report.threat)
                                .max(report.sexual_explicit);
                            let recommended_action = if score >= 0.5 {
                                Some(hre_core::Action::BlurMedia)
                            } else {
                                None
                            };

                            return Ok(OcrReport {
                                raw_text,
                                masked_text,
                                text_report: Some(report),
                                signal: Signal {
                                    score,
                                    label: if score >= 0.5 {
                                        "ocr_abuse".into()
                                    } else {
                                        "ocr_text".into()
                                    },
                                    status: Status::Built,
                                    backend: Backend::Onnx,
                                    confidence: avg_conf,
                                    details,
                                },
                                recommended_action,
                                simulated: false,
                                degraded: Vec::new(),
                            });
                        }
                    } else {
                        // Detector ran successfully and found no text regions
                        // Check if it might be a single-line tight crop where detector didn't find margins
                        let (w, h) = (dynamic_img.width(), dynamic_img.height());
                        let is_tight_line = h <= 64 || (w as f32 / h as f32) >= 2.5;
                        if is_tight_line {
                            if let Ok((raw_text, conf)) = model.recognize(bytes, declared_mime) {
                                if !raw_text.trim().is_empty() && conf >= 0.5 {
                                    let report = hre_text::analyze(&raw_text)
                                        .map_err(|err| VisionError::Ocr(err.to_string()))?;
                                    let masked_text = report.masked_text.clone();
                                    let mut details = BTreeMap::new();
                                    details.insert(
                                        "ocr_text_masked".into(),
                                        serde_json::json!(masked_text.clone()),
                                    );
                                    details.insert("confidence".into(), serde_json::json!(conf));
                                    details.insert(
                                        "recognizer_model".into(),
                                        serde_json::json!("PP-OCRv4_rec"),
                                    );
                                    details.insert("box_count".into(), serde_json::json!(1));

                                    let score = report
                                        .primary_signal
                                        .score
                                        .max(report.toxicity)
                                        .max(report.threat)
                                        .max(report.sexual_explicit);
                                    let recommended_action = if score >= 0.5 {
                                        Some(hre_core::Action::BlurMedia)
                                    } else {
                                        None
                                    };

                                    return Ok(OcrReport {
                                        raw_text,
                                        masked_text,
                                        text_report: Some(report),
                                        signal: Signal {
                                            score,
                                            label: if score >= 0.5 {
                                                "ocr_abuse".into()
                                            } else {
                                                "ocr_text".into()
                                            },
                                            status: Status::Built,
                                            backend: Backend::Onnx,
                                            confidence: conf,
                                            details,
                                        },
                                        recommended_action,
                                        simulated: false,
                                        degraded: Vec::new(),
                                    });
                                }
                            }
                        }

                        // Genuine image with no text
                        let mut details = BTreeMap::new();
                        details.insert("ocr_text_masked".into(), serde_json::json!(""));
                        details.insert("confidence".into(), serde_json::json!(1.0));
                        details.insert("detector_model".into(), serde_json::json!("PP-OCRv4_det"));
                        details.insert("box_count".into(), serde_json::json!(0));

                        return Ok(OcrReport {
                            raw_text: String::new(),
                            masked_text: String::new(),
                            text_report: None,
                            signal: Signal {
                                score: 0.0,
                                label: "no_text_detected".into(),
                                status: Status::Built,
                                backend: Backend::Onnx,
                                confidence: 1.0,
                                details,
                            },
                            recommended_action: None,
                            simulated: false,
                            degraded: Vec::new(),
                        });
                    }
                }
            }
        }

        // 2. Recognizer-only path (if detector is disabled/missing)
        if let Some(model) = &self.model {
            if let Ok((raw_text, conf)) = model.recognize(bytes, declared_mime) {
                if !raw_text.trim().is_empty() && conf >= 0.5 {
                    let report = hre_text::analyze(&raw_text)
                        .map_err(|err| VisionError::Ocr(err.to_string()))?;
                    let masked_text = report.masked_text.clone();
                    let mut details = BTreeMap::new();
                    details.insert(
                        "ocr_text_masked".into(),
                        serde_json::json!(masked_text.clone()),
                    );
                    details.insert("confidence".into(), serde_json::json!(conf));
                    details.insert("recognizer_model".into(), serde_json::json!("PP-OCRv4_rec"));

                    let score = report
                        .primary_signal
                        .score
                        .max(report.toxicity)
                        .max(report.threat)
                        .max(report.sexual_explicit);
                    let recommended_action = if score >= 0.5 {
                        Some(hre_core::Action::BlurMedia)
                    } else {
                        None
                    };

                    return Ok(OcrReport {
                        raw_text,
                        masked_text,
                        text_report: Some(report),
                        signal: Signal {
                            score,
                            label: if score >= 0.5 {
                                "ocr_abuse".into()
                            } else {
                                "ocr_text".into()
                            },
                            status: Status::Built,
                            backend: Backend::Onnx,
                            confidence: conf,
                            details,
                        },
                        recommended_action,
                        simulated: false,
                        degraded: Vec::new(),
                    });
                }
            }
        }

        // 3. Fall back to heuristic backend (extracts container metadata / fixtures)
        self.fallback.process_image(bytes, declared_mime)
    }
}

impl OcrBackend for OnnxOcrBackend {
    fn recognize(&self, image_bytes: &[u8], declared_mime: &str) -> Result<String, VisionError> {
        if let Ok(report) = self.process_image(image_bytes, declared_mime) {
            if !report.raw_text.is_empty() {
                return Ok(report.raw_text);
            }
        }
        self.fallback.recognize(image_bytes, declared_mime)
    }
}

/// Thread-safe Whisper speech recognition model wrapping the Python ONNX transcriber.
pub struct OnnxWhisperModel {
    models_dir: PathBuf,
}

impl std::fmt::Debug for OnnxWhisperModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnnxWhisperModel")
            .field("models_dir", &self.models_dir)
            .finish_non_exhaustive()
    }
}

impl OnnxWhisperModel {
    pub fn load(models_dir: &Path) -> Result<Self, ModelError> {
        let models_dir = models_dir
            .canonicalize()
            .map_err(|_| ModelError::Invalid("missing Whisper model directory"))?;
        for name in [
            "whisper_encoder.onnx",
            "whisper_decoder.onnx",
            "whisper_tokenizer.json",
            "whisper_preprocessor_config.json",
        ] {
            if !models_dir.join(name).is_file() {
                return Err(ModelError::Invalid("missing local Whisper asset"));
            }
        }
        Ok(Self { models_dir })
    }

    /// Transcribes PCM WAV bytes using the local Whisper worker.
    pub fn transcribe(&self, bytes: &[u8]) -> Result<AudioTranscription, AudioError> {
        if bytes.is_empty() {
            return Err(AudioError::EmptyInput);
        }
        use std::io::Write;
        use std::process::{Command, Stdio};
        use std::time::{Duration, Instant};

        let meta = hre_audio::validate_audio(bytes, "")?;
        if meta.mime != "audio/wav" {
            return Err(AudioError::UnsupportedFormat);
        }
        let current_dir = std::env::current_dir()
            .map_err(|_| AudioError::Transcription("worker directory unavailable".into()))?;
        let root = current_dir
            .ancestors()
            .chain(self.models_dir.ancestors())
            .find(|dir| dir.join("scripts/test_audio_transcription.py").is_file())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."));
        let root = root
            .canonicalize()
            .map_err(|_| AudioError::Transcription("worker directory unavailable".into()))?;
        let py_cmd = [
            root.join(".venv/Scripts/python.exe"),
            root.join(".venv/bin/python"),
        ]
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from("python"));

        struct TempWav(PathBuf);
        impl Drop for TempWav {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let wav_path = std::env::temp_dir().join(format!(
            "hre_whisper_{}_{}.wav",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&wav_path)
            .map_err(|_| AudioError::Transcription("worker input creation failed".into()))?;
        let wav = TempWav(wav_path);
        let written = file.write_all(bytes);
        drop(file);
        written.map_err(|_| AudioError::Transcription("worker input write failed".into()))?;

        let mut child = Command::new(py_cmd)
            .current_dir(&root)
            .arg(root.join("scripts/test_audio_transcription.py"))
            .arg("--worker")
            .arg(&self.models_dir)
            .arg(&wav.0)
            .env("HF_HUB_OFFLINE", "1")
            .env("TRANSFORMERS_OFFLINE", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| AudioError::Transcription("worker invocation failed".into()))?;
        let started = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if started.elapsed() < Duration::from_secs(120) => {
                    std::thread::sleep(Duration::from_millis(25));
                }
                result => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(AudioError::Transcription(if result.is_err() {
                        "worker wait failed".into()
                    } else {
                        "worker timed out".into()
                    }));
                }
            }
        }
        let out = child
            .wait_with_output()
            .map_err(|_| AudioError::Transcription("worker output unavailable".into()))?;
        if !out.status.success() {
            return Err(AudioError::Transcription("Whisper inference failed".into()));
        }

        let stdout = String::from_utf8_lossy(&out.stdout);
        #[derive(serde::Deserialize)]
        struct WorkerResult {
            transcript: String,
            #[serde(default = "default_lang")]
            language: String,
            duration_s: f64,
        }
        fn default_lang() -> String {
            "en".into()
        }

        let res: WorkerResult = serde_json::from_str(stdout.trim())
            .map_err(|_| AudioError::Transcription("invalid worker response".into()))?;
        if !res.duration_s.is_finite() || !(0.0..=300.0).contains(&res.duration_s) {
            return Err(AudioError::Transcription("invalid worker duration".into()));
        }
        Ok(AudioTranscription {
            transcript: res.transcript,
            language: res.language,
            duration_s: res.duration_s,
            tone_score: 0.0,
            simulated: false,
        })
    }
}

/// Pluggable ONNX Whisper audio transcription backend implementing `hre_audio::AudioBackend`.
#[derive(Debug, Clone)]
pub struct OnnxAudioBackend {
    model: Option<Arc<OnnxWhisperModel>>,
    fallback: HeuristicAudioBackend,
}

impl OnnxAudioBackend {
    pub fn new(model: Option<Arc<OnnxWhisperModel>>, fallback: HeuristicAudioBackend) -> Self {
        Self { model, fallback }
    }

    pub fn fallback_only(fallback: HeuristicAudioBackend) -> Self {
        Self {
            model: None,
            fallback,
        }
    }

    pub fn load(models_dir: &Path, fallback: HeuristicAudioBackend) -> Result<Self, ModelError> {
        let model = Arc::new(OnnxWhisperModel::load(models_dir)?);
        Ok(Self::new(Some(model), fallback))
    }

    pub fn process_audio(
        &self,
        bytes: &[u8],
        declared_mime: &str,
    ) -> Result<AudioReport, AudioError> {
        let _meta = hre_audio::validate_audio(bytes, declared_mime)?;

        let mut failure = "whisper.onnx_missing";
        if let Some(model) = &self.model {
            failure = "whisper.inference_failed";
            if let Ok(transcription) = model.transcribe(bytes) {
                let raw_transcript = transcription.transcript;
                let text_report = if raw_transcript.trim().is_empty() {
                    None
                } else {
                    hre_text::analyze(&raw_transcript).ok()
                };
                let masked_transcript = text_report
                    .as_ref()
                    .map(|r| r.masked_text.clone())
                    .unwrap_or_else(|| hre_text::mask_pii(&raw_transcript).0);

                let score = text_report
                    .as_ref()
                    .map(|r| r.primary_signal.score)
                    .unwrap_or(0.0);

                let mut details = BTreeMap::new();
                details.insert(
                    "transcript_masked".into(),
                    serde_json::json!(masked_transcript.clone()),
                );
                details.insert("model_id".into(), serde_json::json!("openai/whisper-tiny"));
                details.insert(
                    "duration_s".into(),
                    serde_json::json!(transcription.duration_s),
                );
                details.insert(
                    "processed_duration_s".into(),
                    serde_json::json!(transcription.duration_s.min(30.0)),
                );
                let truncated = transcription.duration_s > 30.0;
                details.insert("truncated".into(), serde_json::json!(truncated));
                let mut degraded = vec!["tone.disabled".into()];
                if truncated {
                    degraded.push("audio.first_window_only".into());
                }

                let signal = Signal {
                    score,
                    label: if score >= 0.31 {
                        "audio_harassment".into()
                    } else {
                        "audio_benign".into()
                    },
                    status: Status::Built,
                    backend: Backend::Onnx,
                    confidence: 0.95,
                    details,
                };

                let tone_signal = Signal {
                    score: 0.0,
                    label: "tone_unavailable".into(),
                    status: Status::Roadmap,
                    backend: Backend::Disabled,
                    confidence: 0.0,
                    details: BTreeMap::new(),
                };

                return Ok(AudioReport {
                    raw_transcript,
                    masked_transcript,
                    language: transcription.language,
                    duration_s: transcription.duration_s,
                    text_report,
                    signal,
                    tone_signal,
                    reasons: Vec::new(),
                    recommended_action: if score >= 0.5 {
                        hre_core::Action::WarnSender
                    } else {
                        hre_core::Action::Log
                    },
                    secondary_actions: if score >= 0.31 {
                        vec![hre_core::SecondaryAction::ShowTranscript]
                    } else {
                        Vec::new()
                    },
                    simulated: false,
                    degraded,
                });
            }
        }

        let mut report = self.fallback.process_audio(bytes, declared_mime)?;
        report
            .degraded
            .retain(|code| !matches!(code.as_str(), "audio.onnx_missing" | "whisper.ggml_missing"));
        report.degraded.push(failure.into());
        report.degraded.push("tone.disabled".into());
        report
            .reasons
            .retain(|reason| reason.code != "AGGRESSIVE_TONE");
        report.tone_signal = Signal {
            score: 0.0,
            label: "tone_unavailable".into(),
            status: Status::Roadmap,
            backend: Backend::Disabled,
            confidence: 0.0,
            details: BTreeMap::new(),
        };
        if !report.simulated {
            report.signal.status = Status::Roadmap;
            report.signal.backend = Backend::Disabled;
        }
        Ok(report)
    }
}

impl AudioBackend for OnnxAudioBackend {
    fn transcribe(
        &self,
        bytes: &[u8],
        declared_mime: &str,
    ) -> Result<AudioTranscription, AudioError> {
        hre_audio::validate_audio(bytes, declared_mime)?;
        if let Some(model) = &self.model {
            if let Ok(t) = model.transcribe(bytes) {
                return Ok(t);
            }
        }
        let mut transcription = self.fallback.transcribe(bytes, declared_mime)?;
        transcription.tone_score = 0.0;
        Ok(transcription)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::excessive_precision)]
mod tests {
    use super::*;
    use sha2::Digest;

    #[test]
    fn tensor_contract_rejects_dynamic_shapes_wrong_types_and_symbols() {
        let tensor = |ty, shape: &[i64], symbol: &str| ValueType::Tensor {
            ty,
            shape: ort::value::Shape::new(shape.iter().copied()),
            dimension_symbols: ort::value::SymbolicDimensions::new([String::new(), symbol.into()]),
        };
        assert!(validate_tensor(
            &tensor(TensorElementType::Int64, &[1, 128], ""),
            TensorElementType::Int64,
            &[1, 128]
        ));
        assert!(!validate_tensor(
            &tensor(TensorElementType::Int64, &[1, -1], ""),
            TensorElementType::Int64,
            &[1, 128]
        ));
        assert!(!validate_tensor(
            &tensor(TensorElementType::Float32, &[1, 128], ""),
            TensorElementType::Int64,
            &[1, 128]
        ));
        assert!(!validate_tensor(
            &tensor(TensorElementType::Int64, &[1, 128], "sequence"),
            TensorElementType::Int64,
            &[1, 128]
        ));
    }

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
        candidates.into_iter().find(|c| c.is_dir())
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
    #[ignore = "requires genuine trained NSFW model, matching parity fixture and ONNX Runtime"]
    fn test_python_rust_parity_nsfw_synthetic_image() {
        let models_dir = find_models_dir().expect("required models directory is missing");
        let model_path = models_dir.join("nsfw.onnx");
        assert!(model_path.is_file(), "required trained model is missing");
        let test_img_path = models_dir.join("synthetic_test.png");
        assert!(test_img_path.is_file(), "required parity image is missing");
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
    #[ignore = "requires genuine trained NSFW model and ONNX Runtime"]
    fn test_onnx_image_backend_normal_and_action() {
        let models_dir = find_models_dir().expect("required models directory is missing");
        let model_path = models_dir.join("nsfw.onnx");
        assert!(model_path.is_file(), "required trained model is missing");
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
    #[ignore = "requires genuine trained NSFW model and ONNX Runtime"]
    fn test_onnx_image_backend_demo_override_is_simulated() {
        let models_dir = find_models_dir().expect("required models directory is missing");
        let model_path = models_dir.join("nsfw.onnx");
        assert!(model_path.is_file(), "required trained model is missing");
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
    #[ignore = "requires transferred model and compatible ONNX Runtime; run via delegated logged recipe"]
    fn test_python_rust_parity_golden_message_vector() {
        let models_dir = find_models_dir().expect("required models directory is missing");
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
    #[ignore = "requires transferred model and compatible ONNX Runtime; run via delegated logged recipe"]
    fn test_python_rust_parity_zero_message_vector() {
        let models_dir = find_models_dir().expect("required models directory is missing");
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
    #[ignore = "golden probabilities require revalidation for the manually replaced checkpoint"]
    fn test_python_rust_parity_text_model_benign() {
        let models_dir = find_models_dir().expect("required models directory is missing");
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
    #[ignore = "golden probabilities require revalidation for the manually replaced checkpoint"]
    fn test_python_rust_parity_text_model_threat() {
        let models_dir = find_models_dir().expect("required models directory is missing");
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
    #[ignore = "requires 1 GB transferred model and compatible ONNX Runtime; run via delegated logged recipe"]
    fn test_onnx_text_backend_threat_report() {
        let models_dir = find_models_dir().expect("required models directory is missing");
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

    const PYTHON_SYNTHETIC_DEEPFAKE_AUTH: f32 = 0.811277;
    const PYTHON_SYNTHETIC_DEEPFAKE_MANIP: f32 = 0.188723;

    #[test]
    #[ignore = "requires genuine trained deepfake model, matching parity fixture and ONNX Runtime"]
    fn test_python_rust_parity_deepfake_synthetic_image() {
        let models_dir = find_models_dir().expect("required models directory is missing");
        let model_path = models_dir.join("deepfake.onnx");
        assert!(model_path.is_file(), "required trained model is missing");
        let test_img_path = models_dir.join("synthetic_test.png");
        assert!(test_img_path.is_file(), "required parity image is missing");
        let img_bytes = std::fs::read(&test_img_path).expect("read test image");
        let model = OnnxDeepfakeModel::load(&model_path).expect("load deepfake model");
        let probs = model
            .predict_image(&img_bytes, "image/png")
            .expect("predict deepfake");

        let diff_auth = (probs[0] - PYTHON_SYNTHETIC_DEEPFAKE_AUTH).abs();
        let diff_manip = (probs[1] - PYTHON_SYNTHETIC_DEEPFAKE_MANIP).abs();

        assert!(
            diff_auth < 0.01,
            "deepfake authentic parity failed: rust={}, python={}, diff={}",
            probs[0],
            PYTHON_SYNTHETIC_DEEPFAKE_AUTH,
            diff_auth
        );
        assert!(
            diff_manip < 0.01,
            "deepfake manipulated parity failed: rust={}, python={}, diff={}",
            probs[1],
            PYTHON_SYNTHETIC_DEEPFAKE_MANIP,
            diff_manip
        );
    }

    #[test]
    #[ignore = "requires genuine trained OCR recognizer, vocabulary and ONNX Runtime"]
    fn test_onnx_ocr_backend_pixel_recognition() {
        let models_dir = find_models_dir().expect("required models directory is missing");
        let model_path = models_dir.join("ocr_recognizer.onnx");
        let vocab_path = models_dir.join("ocr_vocab.json");
        assert!(model_path.is_file(), "required OCR recognizer is missing");
        assert!(vocab_path.is_file(), "required OCR vocabulary is missing");
        let fallback = HeuristicOcrBackend::new();
        let backend = OnnxOcrBackend::load(&model_path, &vocab_path, fallback)
            .expect("load onnx ocr backend");

        let png = valid_test_png();
        let report = backend
            .process_image(&png, "image/png")
            .expect("process image ocr");
        assert_eq!(report.signal.status, Status::Roadmap);
    }

    #[test]
    #[ignore = "requires genuine trained Whisper encoder/decoder, local tokenizer/preprocessor and Python ONNX Runtime"]
    fn test_onnx_whisper_silence_transcription() {
        let models_dir = find_models_dir().expect("required models directory is missing");
        for name in [
            "whisper_encoder.onnx",
            "whisper_decoder.onnx",
            "whisper_tokenizer.json",
            "whisper_preprocessor_config.json",
        ] {
            assert!(
                models_dir.join(name).is_file(),
                "required Whisper asset missing: {name}"
            );
        }
        let fallback = HeuristicAudioBackend::new();
        let backend =
            OnnxAudioBackend::load(&models_dir, fallback).expect("load genuine Whisper backend");
        for duration_s in [1, 31] {
            let wav = hre_audio::fixtures::minimal_wav(duration_s);
            let report = backend
                .process_audio(&wav, "audio/wav")
                .expect("process audio silence");
            assert_eq!(report.raw_transcript, "");
            assert_eq!(report.signal.score, 0.0);
            assert_eq!(report.signal.status, Status::Built);
            assert_eq!(report.signal.backend, Backend::Onnx);
            assert_eq!(report.tone_signal.status, Status::Roadmap);
            assert_eq!(report.tone_signal.backend, Backend::Disabled);
            assert_eq!(
                report.signal.details["duration_s"],
                serde_json::json!(f64::from(duration_s))
            );
            assert_eq!(
                report.signal.details["processed_duration_s"],
                serde_json::json!(f64::from(duration_s.min(30)))
            );
            assert_eq!(
                report.signal.details["truncated"],
                serde_json::json!(duration_s > 30)
            );
            assert_eq!(
                report
                    .degraded
                    .iter()
                    .any(|code| code == "audio.first_window_only"),
                duration_s > 30,
            );
            assert!(!report
                .degraded
                .iter()
                .any(|code| code.starts_with("whisper.")));
        }
    }

    #[test]
    fn missing_whisper_reports_disabled_and_missing_reason() {
        let backend = OnnxAudioBackend::fallback_only(HeuristicAudioBackend::new());
        let report = backend
            .process_audio(&hre_audio::fixtures::minimal_wav(1), "audio/wav")
            .unwrap();
        assert!(report
            .degraded
            .iter()
            .any(|code| code == "whisper.onnx_missing"));
        assert_eq!(report.signal.status, Status::Roadmap);
        assert_eq!(report.signal.backend, Backend::Disabled);
        assert_eq!(report.tone_signal.status, Status::Roadmap);
    }

    #[test]
    fn unsupported_whisper_format_reports_failure_and_disables_fixture_tone() {
        let backend = OnnxAudioBackend::new(
            Some(Arc::new(OnnxWhisperModel {
                models_dir: PathBuf::new(),
            })),
            HeuristicAudioBackend::new(),
        );
        let report = backend
            .process_audio(
                &hre_audio::fixtures::minimal_mp3_with_comment("hello"),
                "audio/mpeg",
            )
            .unwrap();
        assert!(report
            .degraded
            .iter()
            .any(|code| code == "whisper.inference_failed"));
        assert!(!report
            .degraded
            .iter()
            .any(|code| code == "whisper.onnx_missing"));
        assert!(report.simulated);
        assert_eq!(report.tone_signal.score, 0.0);
        assert_eq!(report.tone_signal.status, Status::Roadmap);
        assert_eq!(report.tone_signal.backend, Backend::Disabled);
    }

    #[test]
    fn test_fallback_corrupt_deepfake_and_ocr_models() {
        let dir = std::env::temp_dir().join(format!("hre-corrupt-df-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let corrupt_path = dir.join("corrupt.onnx");
        std::fs::write(&corrupt_path, b"not an onnx file").unwrap();

        assert!(OnnxDeepfakeModel::load(&corrupt_path).is_err());
        assert!(OnnxOcrModel::load(&corrupt_path, &corrupt_path).is_err());
        assert!(OnnxOcrDetector::load(&corrupt_path).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[ignore = "requires genuine trained OCR detector, multiline fixture and ONNX Runtime"]
    fn test_onnx_ocr_detector_multiline_boxes() {
        let models_dir = find_models_dir().expect("required models directory is missing");
        let det_path = models_dir.join("ocr_detector.onnx");
        assert!(det_path.is_file(), "required OCR detector is missing");
        let fixture_path = models_dir.join("_test_fixtures").join("ocr_multiline.png");
        assert!(
            fixture_path.is_file(),
            "required OCR image fixture is missing"
        );
        let detector = OnnxOcrDetector::load(&det_path).expect("load ocr detector");
        let img = image::open(&fixture_path).expect("open multiline image");
        let boxes = detector.detect(&img).expect("detect multiline text boxes");

        assert!(
            boxes.len() >= 2,
            "expected at least 2 text lines detected, got {}",
            boxes.len()
        );
        for b in &boxes {
            assert!(b.width() > 10, "box width should be substantial: {:?}", b);
            assert!(b.height() > 5, "box height should be substantial: {:?}", b);
            assert!(b.score >= 0.5, "box score should meet threshold: {:?}", b);
        }
        // Verify reading order: y coordinates increase
        for i in 1..boxes.len() {
            assert!(
                boxes[i].y_min >= boxes[i - 1].y_min || boxes[i].x_min >= boxes[i - 1].x_min,
                "boxes should be sorted in reading order"
            );
        }
    }

    #[test]
    #[ignore = "requires genuine trained OCR detector/recognizer, vocabulary, multiline fixture and ONNX Runtime"]
    fn test_onnx_ocr_backend_multiline_end_to_end() {
        let models_dir = find_models_dir().expect("required models directory is missing");
        let rec_path = models_dir.join("ocr_recognizer.onnx");
        let vocab_path = models_dir.join("ocr_vocab.json");
        let det_path = models_dir.join("ocr_detector.onnx");
        assert!(rec_path.is_file(), "required OCR recognizer is missing");
        assert!(vocab_path.is_file(), "required OCR vocabulary is missing");
        assert!(det_path.is_file(), "required OCR detector is missing");
        let fallback = HeuristicOcrBackend::new();
        let backend = OnnxOcrBackend::load_pipeline(&det_path, &rec_path, &vocab_path, fallback)
            .expect("load onnx ocr pipeline");

        let fixture_path = models_dir.join("_test_fixtures").join("ocr_multiline.png");
        assert!(
            fixture_path.is_file(),
            "required OCR image fixture is missing"
        );
        let bytes = std::fs::read(&fixture_path).expect("read fixture");
        let report = backend
            .process_image(&bytes, "image/png")
            .expect("process multiline ocr");

        assert_eq!(report.signal.status, Status::Built);
        assert_eq!(report.signal.backend, Backend::Onnx);
        assert!(!report.simulated);
        assert!(!report.raw_text.is_empty());
        assert!(
            report.raw_text.contains("HARASS")
                || report.raw_text.contains("INCIDENT")
                || report.raw_text.contains("REPORT")
                || report.raw_text.contains("ALONE"),
            "recognized text did not match expected keywords: {}",
            report.raw_text
        );
        assert!(report.signal.confidence > 0.6);
        assert!(report.signal.score >= 0.0);
    }

    #[test]
    #[ignore = "requires genuine trained OCR detector/recognizer, vocabulary, single-line fixture and ONNX Runtime"]
    fn test_onnx_ocr_backend_single_line_compatibility() {
        let models_dir = find_models_dir().expect("required models directory is missing");
        let rec_path = models_dir.join("ocr_recognizer.onnx");
        let vocab_path = models_dir.join("ocr_vocab.json");
        let det_path = models_dir.join("ocr_detector.onnx");
        assert!(rec_path.is_file(), "required OCR recognizer is missing");
        assert!(vocab_path.is_file(), "required OCR vocabulary is missing");
        assert!(det_path.is_file(), "required OCR detector is missing");
        let fallback = HeuristicOcrBackend::new();
        let backend = OnnxOcrBackend::load_pipeline(&det_path, &rec_path, &vocab_path, fallback)
            .expect("load onnx ocr pipeline");

        let fixture_path = models_dir
            .join("_test_fixtures")
            .join("ocr_single_line.png");
        assert!(
            fixture_path.is_file(),
            "required OCR image fixture is missing"
        );
        let bytes = std::fs::read(&fixture_path).expect("read single-line fixture");
        let report = backend
            .process_image(&bytes, "image/png")
            .expect("process single-line ocr");

        assert_eq!(report.signal.status, Status::Built);
        assert_eq!(report.signal.backend, Backend::Onnx);
        assert!(!report.simulated);
        assert!(
            report.raw_text.contains("STOP") || report.raw_text.contains("CALLING"),
            "single line text mismatch: {}",
            report.raw_text
        );
    }

    #[test]
    #[ignore = "requires genuine trained OCR detector/recognizer, vocabulary, no-text fixture and ONNX Runtime"]
    fn test_onnx_ocr_backend_no_text_image() {
        let models_dir = find_models_dir().expect("required models directory is missing");
        let rec_path = models_dir.join("ocr_recognizer.onnx");
        let vocab_path = models_dir.join("ocr_vocab.json");
        let det_path = models_dir.join("ocr_detector.onnx");
        assert!(rec_path.is_file(), "required OCR recognizer is missing");
        assert!(vocab_path.is_file(), "required OCR vocabulary is missing");
        assert!(det_path.is_file(), "required OCR detector is missing");
        let fallback = HeuristicOcrBackend::new();
        let backend = OnnxOcrBackend::load_pipeline(&det_path, &rec_path, &vocab_path, fallback)
            .expect("load onnx ocr pipeline");

        let fixture_path = models_dir.join("_test_fixtures").join("ocr_no_text.png");
        assert!(
            fixture_path.is_file(),
            "required OCR image fixture is missing"
        );
        let bytes = std::fs::read(&fixture_path).expect("read no-text fixture");
        let report = backend
            .process_image(&bytes, "image/png")
            .expect("process no-text image");

        assert_eq!(report.signal.status, Status::Built);
        assert_eq!(report.signal.backend, Backend::Onnx);
        assert_eq!(report.raw_text, "");
        assert_eq!(report.signal.score, 0.0);
    }
}
