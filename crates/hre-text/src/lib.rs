//! Text normalization, multi-label toxicity, Indian-aware PII detection, and cue analysis.
//!
//! Provides:
//! - Conservative Unicode, homoglyph, spacing, and repetition normalizer (`normalize`)
//! - Indian-aware PII detection and masking (`detect_and_mask_pii`): email, phone, Aadhaar (Verhoeff), PAN, UPI, PIN
//! - Behavioral cues (`analyze_cues`): threat, coercion, location, sextortion pattern, quoted abuse
//! - Deterministic model-free text classification (`HeuristicTextBackend`, `TextBackend`)
//! - Multi-label scoring matching Detoxify categories

pub mod backend;
pub mod cues;
pub mod error;
pub mod normalize;
pub mod pii;

pub use backend::{HeuristicTextBackend, MultiLabelScores, TextAnalysisResult, TextBackend};
pub use cues::{analyze_cues, find_quoted_spans, CueMatches};
pub use error::TextError;
pub use normalize::{fold_leetspeak, normalize, validate_text, MAX_TEXT_BYTES};
pub use pii::{
    compute_verhoeff_check_digit, detect_and_mask_pii, validate_verhoeff, PiiDetectionResult,
    PiiItem, PiiKind,
};

/// Convenience function to analyze text using the default deterministic heuristic backend.
pub fn analyze(text: &str) -> Result<hre_core::Signal, TextError> {
    let backend = HeuristicTextBackend::new();
    backend.analyze(text)
}

/// Convenience function to analyze text and obtain full multi-label, PII, and cue breakdowns.
pub fn analyze_detailed(text: &str) -> Result<TextAnalysisResult, TextError> {
    let backend = HeuristicTextBackend::new();
    backend.analyze_detailed(text)
}
