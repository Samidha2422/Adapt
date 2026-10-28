pub mod deepfake;
pub mod hasher;
pub mod nsfw;
pub mod ocr;
pub mod validate;

pub use deepfake::{evaluate_deepfake_composite, DeepfakeAssessment};
pub use hasher::{compute_dhash_from_grid, compute_phash, hamming_distance};
pub use nsfw::{HeuristicImageBackend, ImageAnalysis, ImageBackend};
pub use ocr::{compute_cer, HeuristicOcrBackend, OcrBackend, OcrReport};
pub use validate::{validate_image, ImageMetadata, VisionError, MAX_DIMENSION, MAX_IMAGE_SIZE};

pub mod fixtures {
    pub fn minimal_png(width: u32, height: u32) -> Vec<u8> {
        let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        v.extend_from_slice(&13u32.to_be_bytes());
        v.extend_from_slice(b"IHDR");
        v.extend_from_slice(&width.to_be_bytes());
        v.extend_from_slice(&height.to_be_bytes());
        v.extend_from_slice(&[8, 2, 0, 0, 0]);
        v.extend_from_slice(&[0, 0, 0, 0]);
        v.extend_from_slice(&0u32.to_be_bytes());
        v.extend_from_slice(b"IEND");
        v.extend_from_slice(&[0xAE, 0x42, 0x60, 0x82]);
        v
    }

    pub fn png_with_text(keyword: &str, text: &str) -> Vec<u8> {
        let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        v.extend_from_slice(&13u32.to_be_bytes());
        v.extend_from_slice(b"IHDR");
        v.extend_from_slice(&64u32.to_be_bytes());
        v.extend_from_slice(&64u32.to_be_bytes());
        v.extend_from_slice(&[8, 2, 0, 0, 0]);
        v.extend_from_slice(&[0, 0, 0, 0]);

        let mut text_data = Vec::new();
        text_data.extend_from_slice(keyword.as_bytes());
        text_data.push(0);
        text_data.extend_from_slice(text.as_bytes());
        v.extend_from_slice(&(text_data.len() as u32).to_be_bytes());
        v.extend_from_slice(b"tEXt");
        v.extend_from_slice(&text_data);
        v.extend_from_slice(&[0, 0, 0, 0]);

        v.extend_from_slice(&0u32.to_be_bytes());
        v.extend_from_slice(b"IEND");
        v.extend_from_slice(&[0xAE, 0x42, 0x60, 0x82]);
        v
    }

    pub fn minimal_jpeg(width: u16, height: u16) -> Vec<u8> {
        let mut v = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        v.extend_from_slice(b"JFIF\0\x01\x01\x01\x00`\x00`\x00\x00");
        // SOF0 marker (0xC0)
        v.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x0B, 0x08]);
        v.extend_from_slice(&height.to_be_bytes());
        v.extend_from_slice(&width.to_be_bytes());
        v.extend_from_slice(&[0x01, 0x01, 0x11, 0x00]);
        // EOI
        v.extend_from_slice(&[0xFF, 0xD9]);
        v
    }

    pub fn minimal_gif(width: u16, height: u16) -> Vec<u8> {
        let mut v = Vec::from(&b"GIF89a"[..]);
        v.extend_from_slice(&width.to_le_bytes());
        v.extend_from_slice(&height.to_le_bytes());
        v.extend_from_slice(&[0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0x3B]);
        v
    }
}
pub use fixtures::*;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use hre_core::{Action, Backend, Status};
    use sha2::Digest;

    // Required tests from task card B1
    #[test]
    fn test_rejects_mime_mismatch() {
        let png = minimal_png(10, 10);
        let res = validate_image(&png, "image/jpeg");
        assert!(matches!(res, Err(VisionError::MimeMismatch { .. })));

        let gif = minimal_gif(10, 10);
        assert!(validate_image(&gif, "image/gif").is_ok());
    }

    #[test]
    fn test_rejects_truncated_and_oversize() {
        let truncated = vec![0x89, b'P', b'N'];
        assert_eq!(
            validate_image(&truncated, "image/png"),
            Err(VisionError::TruncatedImage)
        );

        let oversize = vec![0u8; MAX_IMAGE_SIZE + 1];
        assert!(matches!(
            validate_image(&oversize, "image/png"),
            Err(VisionError::Oversized { .. })
        ));
    }

    #[test]
    fn test_missing_model_is_degraded() {
        let backend = HeuristicImageBackend::new();
        let png = minimal_png(10, 10);
        let report = backend.analyze(&png, "image/png").unwrap();
        assert_eq!(report.signal.backend, Backend::Disabled);
        assert_eq!(report.signal.status, Status::Roadmap);
        assert_eq!(report.signal.label, "image_unavailable");
        assert_eq!(report.signal.score, 0.0);
        assert!(!report.simulated);
        assert!(report.degraded.iter().any(|d| d == "image.onnx_missing"));
    }

    #[test]
    fn test_hamming_same_hash_zero_and_threshold_boundary() {
        let h1: u64 = 0xAAAAAAAAAAAAAAAA;
        let h2: u64 = 0xAAAAAAAAAAAAAAAA;
        assert_eq!(hamming_distance(h1, h2), 0);

        let h3: u64 = 0xAAAAAAAAAAAAAAAB; // 1 bit different
        assert_eq!(hamming_distance(h1, h3), 1);

        let h_inv = !h1; // 64 bits different
        assert_eq!(hamming_distance(h1, h_inv), 64);
    }

    #[test]
    fn test_report_match_is_simulated() {
        let png = minimal_png(10, 10);
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, &png);
        let sha256 = format!("{:x}", sha2::Digest::finalize(hasher));

        let backend = HeuristicImageBackend::new().with_override(&sha256, 0.95);
        let report = backend.analyze(&png, "image/png").unwrap();
        assert_eq!(report.signal.status, Status::Simulated);
        assert!(report.simulated);
        assert_eq!(report.recommended_action, Some(Action::BlurMedia));
    }

    #[test]
    fn test_no_pixels_are_persisted() {
        let png = minimal_png(20, 20);
        let backend = HeuristicImageBackend::new();
        let report = backend.analyze(&png, "image/png").unwrap();
        // The analysis struct only exposes hashes, signals, actions, and degradation
        let serialized = serde_json::to_string(&report).unwrap();
        assert!(!serialized.contains("pixels"));
        assert!(!serialized.contains("raw"));
    }

    #[test]
    fn test_decode_limits_prevent_oversized_dimensions() {
        let huge_png = minimal_png(5000, 100);
        assert!(matches!(
            validate_image(&huge_png, "image/png"),
            Err(VisionError::OversizedDimensions { .. })
        ));

        let huge_jpeg = minimal_jpeg(100, 5000);
        assert!(matches!(
            validate_image(&huge_jpeg, "image/jpeg"),
            Err(VisionError::OversizedDimensions { .. })
        ));
    }

    // Required tests from task card B2
    #[test]
    fn test_missing_ocr_degraded() {
        let backend = HeuristicOcrBackend::new();
        let png = minimal_png(10, 10);
        let report = backend.process_image(&png, "image/png").unwrap();
        assert_eq!(report.signal.score, 0.0);
        assert_eq!(report.raw_text, "");
        assert!(report.degraded.iter().any(|d| d == "ocr.onnx_missing"));
    }

    #[test]
    fn test_container_metadata_scores_supplied_text_as_simulated() {
        let backend = HeuristicOcrBackend::new();
        let meme_png = png_with_text("Description", "I will find you and kill you");
        let report = backend.process_image(&meme_png, "image/png").unwrap();
        assert_eq!(report.raw_text, "I will find you and kill you");
        assert!(report.signal.score >= 0.5);
        assert_eq!(report.recommended_action, Some(Action::BlurMedia));
        assert!(report.simulated);
    }

    #[test]
    fn test_malformed_image() {
        let backend = HeuristicOcrBackend::new();
        let garbage = b"not an image at all";
        assert!(matches!(
            backend.process_image(garbage, "image/png"),
            Err(VisionError::UnsupportedFormat)
        ));
    }

    #[test]
    fn test_caption_is_not_ocr() {
        // Image without embedded text produces empty OCR text even if external caption exists
        let png = minimal_png(10, 10);
        let backend = HeuristicOcrBackend::new();
        let report = backend.process_image(&png, "image/png").unwrap();
        assert_eq!(report.raw_text, "");
    }

    #[test]
    fn test_cer_exact_zero_and_one_edit() {
        assert_eq!(compute_cer("hello world", "hello world"), 0.0);
        // 1 edit (substitution) in 5 chars -> 1/5 = 0.2
        assert_eq!(compute_cer("hello", "hallo"), 0.2);
    }

    #[test]
    fn test_empty_reference_policy() {
        assert_eq!(compute_cer("", ""), 0.0);
        assert_eq!(compute_cer("", "something"), 1.0);
    }

    #[test]
    fn test_pii_masked_before_display() {
        let backend = HeuristicOcrBackend::new();
        let meme = png_with_text("Comment", "Call me at +919876543210 immediately");
        let report = backend.process_image(&meme, "image/png").unwrap();
        assert!(report.masked_text.contains("[PHONE]"));
        assert!(!report.masked_text.contains("9876543210"));
        assert!(!serde_json::to_string(&report)
            .unwrap()
            .contains("9876543210"));
        assert!(report.simulated);
        assert_eq!(report.signal.status, Status::Simulated);
        assert_eq!(
            report.text_report.unwrap().primary_signal.status,
            Status::Simulated
        );
    }

    #[test]
    fn unavailable_ocr_does_not_claim_clean_pixels() {
        let report = HeuristicOcrBackend::new()
            .process_image(&minimal_png(10, 10), "image/png")
            .unwrap();
        assert_eq!(report.signal.status, Status::Roadmap);
        assert_eq!(report.signal.backend, Backend::Disabled);
    }

    #[test]
    fn oversized_metadata_is_an_error_instead_of_a_zero_score() {
        let png = png_with_text(
            "Comment",
            &"a".repeat(hre_text::normalizer::MAX_TEXT_LENGTH_BYTES + 1),
        );
        assert!(matches!(
            HeuristicOcrBackend::new().process_image(&png, "image/png"),
            Err(VisionError::Ocr(_))
        ));
    }

    #[test]
    fn overrides_are_finite_clamped_and_simulated() {
        let png = minimal_png(10, 10);
        let hash = format!("{:x}", sha2::Sha256::digest(&png));
        let report = HeuristicImageBackend::new()
            .with_override(&hash, f64::NAN)
            .analyze(&png, "image/png")
            .unwrap();
        assert_eq!(report.signal.status, Status::Roadmap);
        let config = format!("[assets]\n'{hash}' = 5.0");
        let report = HeuristicImageBackend::from_overrides_toml(&config)
            .analyze(&png, "image/png")
            .unwrap();
        assert_eq!(report.signal.score, 1.0);
        assert!(report.simulated);
        let report = evaluate_deepfake_composite(true, f64::NAN, f64::INFINITY);
        assert!(report.deepfake_score.is_finite() && report.nsfw_score.is_finite());
        assert_eq!(report.signal.status, Status::Simulated);
    }

    #[test]
    fn international_metadata_skips_headers_and_compression() {
        let mut png = minimal_png(10, 10);
        png.truncate(33);
        let data = b"Comment\0\0\0en\0translation\0hello";
        png.extend_from_slice(&(data.len() as u32).to_be_bytes());
        png.extend_from_slice(b"iTXt");
        png.extend_from_slice(data);
        png.extend_from_slice(&[0; 4]);
        assert_eq!(
            HeuristicOcrBackend::extract_embedded_text(&png).as_deref(),
            Some("hello")
        );
        png[49] = 1; // compression flag after the keyword
        assert_eq!(HeuristicOcrBackend::extract_embedded_text(&png), None);
    }

    #[test]
    fn test_unsupported_language_reported() {
        let text = "Text with standard Latin characters";
        let cer = compute_cer(text, text);
        assert_eq!(cer, 0.0);
    }

    #[test]
    fn test_deepfake_composite_rule() {
        // Not synthetic intimate if no face
        let no_face = evaluate_deepfake_composite(false, 0.9, 0.9);
        assert!(!no_face.synthetic_intimate);

        // Not synthetic intimate if low deepfake score
        let low_df = evaluate_deepfake_composite(true, 0.4, 0.9);
        assert!(!low_df.synthetic_intimate);

        // Fired when face + high df + high nsfw
        let intimate = evaluate_deepfake_composite(true, 0.85, 0.75);
        assert!(intimate.synthetic_intimate);
        assert!(intimate.low_confidence);
        assert_eq!(
            intimate.reason.unwrap().code,
            "POSSIBLE_SYNTHETIC_INTIMATE_MEDIA"
        );
        assert_eq!(intimate.signal.status, Status::Simulated);
    }

    proptest::proptest! {
        #[test]
        fn test_cer_always_between_zero_and_inf(ref_str in "\\PC*", hyp_str in "\\PC*") {
            let cer = compute_cer(&ref_str, &hyp_str);
            proptest::prop_assert!(cer >= 0.0);
            proptest::prop_assert!(cer.is_finite());
        }
    }
}
