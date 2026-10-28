use crate::validate::{validate_image, VisionError};
use hre_core::{Action, Backend, Signal, Status};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrReport {
    pub raw_text: String,
    pub masked_text: String,
    pub text_report: Option<hre_text::TextReport>,
    pub signal: Signal,
    pub recommended_action: Option<Action>,
    pub simulated: bool,
    pub degraded: Vec<String>,
}

pub trait OcrBackend: Send + Sync {
    fn recognize(&self, image_bytes: &[u8], declared_mime: &str) -> Result<String, VisionError>;
}

/// Fallback and synthetic OCR extractor.
///
/// Extracts supplied container metadata (PNG tEXt/iTXt, JPEG COM) or fixtures.
/// It does not recognize pixels; contributing results are always simulated.
#[derive(Debug, Clone, Default)]
pub struct HeuristicOcrBackend {
    fixture_texts: BTreeMap<String, String>,
}

impl HeuristicOcrBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_fixture(mut self, sha256_or_id: &str, text: &str) -> Self {
        self.fixture_texts
            .insert(sha256_or_id.to_string(), text.to_string());
        self
    }

    /// Attempts to extract textual metadata embedded in PNG or JPEG containers.
    pub fn extract_embedded_text(bytes: &[u8]) -> Option<String> {
        if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
            extract_png_text(bytes)
        } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            extract_jpeg_com(bytes)
        } else {
            None
        }
    }

    /// Full OCR screening pipeline: extract text, mask PII, forward to hre-text,
    /// and generate an OCR signal and blur recommendation if abusive.
    pub fn process_image(
        &self,
        bytes: &[u8],
        declared_mime: &str,
    ) -> Result<OcrReport, VisionError> {
        let _meta = validate_image(bytes, declared_mime)?;

        let raw_text = self.recognize(bytes, declared_mime)?;
        if raw_text.trim().is_empty() {
            let mut details = BTreeMap::new();
            details.insert("ocr_text_masked".into(), serde_json::json!(""));
            return Ok(OcrReport {
                raw_text: String::new(),
                masked_text: String::new(),
                text_report: None,
                signal: Signal {
                    score: 0.0,
                    label: "ocr_unavailable".into(),
                    status: Status::Roadmap,
                    backend: Backend::Disabled,
                    confidence: 0.0,
                    details,
                },
                recommended_action: None,
                simulated: false,
                degraded: vec!["ocr.onnx_missing".into(), "ocr.heuristic_fallback".into()],
            });
        }

        // Mask PII before passing downstream or displaying to moderators
        let mut report =
            hre_text::analyze(&raw_text).map_err(|err| VisionError::Ocr(err.to_string()))?;
        let masked_text = report.masked_text.clone();
        // Fixture and container metadata are supplied text, never pixel recognition.
        for signal in [&mut report.primary_signal, &mut report.pii_signal] {
            signal.status = Status::Simulated;
            signal
                .details
                .insert("simulated".into(), serde_json::json!(true));
        }
        let text_report = Some(report);

        let mut score = 0.0;
        let mut reasons = Vec::new();
        if let Some(ref rep) = text_report {
            score = rep.toxicity.max(rep.threat).max(rep.sexual_explicit);
            reasons = rep.reasons.clone();
        }

        let recommended_action = if score >= 0.5 {
            Some(Action::BlurMedia)
        } else {
            None
        };

        let mut details = BTreeMap::new();
        details.insert(
            "ocr_text_masked".into(),
            serde_json::json!(masked_text.clone()),
        );
        details.insert("reasons_count".into(), serde_json::json!(reasons.len()));
        details.insert("simulated".into(), serde_json::json!(true));
        details.insert(
            "source".into(),
            serde_json::json!("fixture_or_container_metadata"),
        );

        Ok(OcrReport {
            // Preserve the DTO field without returning unmasked identifiers.
            raw_text: masked_text.clone(),
            masked_text,
            text_report,
            signal: Signal {
                score,
                label: "simulated_ocr_text".into(),
                status: Status::Simulated,
                backend: Backend::Heuristic,
                confidence: 0.0,
                details,
            },
            recommended_action,
            simulated: true,
            degraded: vec![
                "ocr.onnx_missing".into(),
                "ocr.metadata_or_fixture_simulated".into(),
            ],
        })
    }
}

impl OcrBackend for HeuristicOcrBackend {
    fn recognize(&self, image_bytes: &[u8], declared_mime: &str) -> Result<String, VisionError> {
        let _meta = validate_image(image_bytes, declared_mime)?;

        // 1. Check fixture map by SHA-256
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, image_bytes);
        let hash = format!("{:x}", sha2::Digest::finalize(hasher));
        if let Some(text) = self.fixture_texts.get(&hash) {
            return Ok(text.clone());
        }

        // 2. Extract embedded text from image container (e.g. meme screenshot metadata)
        if let Some(text) = Self::extract_embedded_text(image_bytes) {
            return Ok(text);
        }

        // 3. Honest fallback: no fabricated OCR text
        Ok(String::new())
    }
}

fn extract_png_text(bytes: &[u8]) -> Option<String> {
    let mut pos = 8; // skip PNG signature
    while pos + 8 <= bytes.len() {
        let chunk_len =
            u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
                as usize;
        let chunk_type = &bytes[pos + 4..pos + 8];
        let data_start = pos + 8;
        let data_end = data_start.checked_add(chunk_len)?;
        if data_end.checked_add(4)? > bytes.len() {
            break;
        }

        if chunk_type == b"tEXt" {
            let data = &bytes[data_start..data_end];
            if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                if let Ok(text) = std::str::from_utf8(&data[null_pos + 1..]) {
                    if !text.trim().is_empty() {
                        return Some(text.trim().to_string());
                    }
                }
            }
        } else if chunk_type == b"iTXt" {
            let data = &bytes[data_start..data_end];
            // keyword NUL, compression flag/method, language NUL, translation NUL, text.
            if let Some(keyword_end) = data.iter().position(|&b| b == 0) {
                let rest = data.get(keyword_end + 1..)?;
                if !rest.starts_with(&[0, 0]) {
                    pos = data_end + 4;
                    continue; // Compressed or unsupported metadata is not plain text.
                }
                let fields: Vec<&[u8]> = rest[2..].splitn(3, |&b| b == 0).collect();
                if let Some(Ok(text)) = fields.get(2).map(|text| std::str::from_utf8(text)) {
                    if !text.trim().is_empty() {
                        return Some(text.trim().to_string());
                    }
                }
            }
        }

        pos = data_end + 4; // skip chunk data + 4 bytes CRC
    }
    None
}

fn extract_jpeg_com(bytes: &[u8]) -> Option<String> {
    let mut pos = 2;
    while pos + 4 <= bytes.len() {
        if bytes[pos] != 0xFF {
            pos += 1;
            continue;
        }
        while pos < bytes.len() && bytes[pos] == 0xFF {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }
        let marker = bytes[pos];
        pos += 1;
        if marker == 0xD9 || marker == 0xDA {
            break;
        }
        if pos + 2 > bytes.len() {
            break;
        }
        let len = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]) as usize;
        if len < 2 || pos + len > bytes.len() {
            break;
        }
        if marker == 0xFE {
            // COM (comment) marker
            let comment_bytes = &bytes[pos + 2..pos + len];
            if let Ok(s) = std::str::from_utf8(comment_bytes) {
                if !s.trim().is_empty() {
                    return Some(s.trim().to_string());
                }
            }
        }
        pos += len;
    }
    None
}

/// Computes Character Error Rate (CER) = Levenshtein distance / reference character count.
///
/// Empty reference policy:
/// - If both reference and hypothesis are empty: 0.0
/// - If reference is empty and hypothesis is non-empty: 1.0
pub fn compute_cer(reference: &str, hypothesis: &str) -> f64 {
    let ref_chars: Vec<char> = reference.chars().collect();
    let hyp_chars: Vec<char> = hypothesis.chars().collect();

    if ref_chars.is_empty() {
        return if hyp_chars.is_empty() { 0.0 } else { 1.0 };
    }

    let n = ref_chars.len();
    let m = hyp_chars.len();
    // Keep two rows: quadratic memory can exhaust the process on long OCR text.
    let mut previous: Vec<usize> = (0..=m).collect();
    let mut current = vec![0usize; m + 1];

    for i in 1..=n {
        current[0] = i;
        for j in 1..=m {
            let cost = if ref_chars[i - 1] == hyp_chars[j - 1] {
                0
            } else {
                1
            };
            current[j] = (previous[j] + 1)
                .min(current[j - 1] + 1)
                .min(previous[j - 1] + cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[m] as f64 / n as f64
}
