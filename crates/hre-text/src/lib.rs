//! Multimodal Harassment Risk Engine: Text Normalization, PII, Cues & Classification.

pub mod classifier;
pub mod cues;
pub mod normalizer;
pub mod pii;

pub use classifier::{HeuristicTextBackend, TextBackend, TextReport};
pub use normalizer::normalize;
pub use pii::{detect_pii, mask_pii, PiiKind, PiiMatch};

#[derive(Debug, thiserror::Error)]
pub enum TextError {
    #[error("text length {len} exceeds maximum allowed {max} bytes")]
    TextTooLong { len: usize, max: usize },
    #[error("empty text input")]
    EmptyText,
    #[error("classification failed: {0}")]
    Classification(String),
}

/// Convenience function to analyze text using the default deterministic heuristic backend.
pub fn analyze(text: &str) -> Result<TextReport, TextError> {
    HeuristicTextBackend.analyze(text)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_normalization_unicode_and_spacing() {
        let raw = "  Hеllо \u{200B}   wоrld!  ";
        let norm = normalize(raw);
        assert_eq!(norm, "Hello world!");
    }

    #[test]
    fn test_missing_model_degraded() {
        let report = analyze("Hello world").unwrap();
        assert!(report.degraded.iter().any(|d| d == "text.onnx_missing"));
        assert!(report
            .degraded
            .iter()
            .any(|d| d == "text.heuristic_fallback"));
        assert_eq!(report.primary_signal.backend, hre_core::Backend::Heuristic);
        assert_eq!(report.primary_signal.status, hre_core::Status::Built);
    }

    #[test]
    fn test_indian_phone_and_email_masked() {
        let text = "Contact me at +91 9876543210 or user@example.com";
        let report = analyze(text).unwrap();
        assert_eq!(report.masked_text, "Contact me at [PHONE] or [EMAIL]");
        assert_eq!(report.pii_matches.len(), 2);
    }

    #[test]
    fn test_aadhaar_requires_verhoeff() {
        let valid_digits = [2, 3, 6, 6, 1, 8, 3, 0, 8, 8, 9];
        let check = pii::generate_verhoeff(&valid_digits);
        let valid_aadhaar = format!("2366 1830 889{check}");
        let invalid_aadhaar = "2366 1830 8890";

        let report_valid = analyze(&format!("Aadhaar is {valid_aadhaar}")).unwrap();
        assert!(report_valid
            .pii_matches
            .iter()
            .any(|m| m.kind == PiiKind::Aadhaar));

        let report_invalid = analyze(&format!("Number is {invalid_aadhaar}")).unwrap();
        assert!(!report_invalid
            .pii_matches
            .iter()
            .any(|m| m.kind == PiiKind::Aadhaar));
    }

    #[test]
    fn test_pin_needs_address_context() {
        let no_ctx = "Random number 560001";
        let report_no = analyze(no_ctx).unwrap();
        assert!(!report_no.pii_matches.iter().any(|m| m.kind == PiiKind::Pin));

        let with_ctx = "Near MG Road postal pincode 560001";
        let report_yes = analyze(with_ctx).unwrap();
        assert!(report_yes
            .pii_matches
            .iter()
            .any(|m| m.kind == PiiKind::Pin));
    }

    #[test]
    fn test_quoted_abuse_not_auto_verdict() {
        let direct = "i will kill you";
        let quoted = "> i will kill you";

        let report_direct = analyze(direct).unwrap();
        let report_quoted = analyze(quoted).unwrap();

        assert!(report_direct.threat > 0.9);
        assert!(report_quoted.threat < 0.5);
        assert!(report_quoted
            .reasons
            .iter()
            .any(|r| r.code == "QUOTED_ABUSE_DISCOUNT"));
    }

    #[test]
    fn test_bounded_invalid_text() {
        assert!(matches!(analyze("   "), Err(TextError::EmptyText)));
        assert!(matches!(analyze(""), Err(TextError::EmptyText)));

        let huge = "a".repeat(normalizer::MAX_TEXT_LENGTH_BYTES + 1);
        assert!(matches!(analyze(&huge), Err(TextError::TextTooLong { .. })));
    }

    proptest! {
        #[test]
        fn property_normalizer_never_panics(s in ".*") {
            let _ = normalize(&s);
        }
    }
}
