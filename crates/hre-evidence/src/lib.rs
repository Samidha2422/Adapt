pub mod chain;
pub mod html;
pub mod record;

pub use chain::{
    build_timeline, verify_timeline, CertificateDraft, EvidenceError, EvidenceExport,
    GENESIS_PREV_HASH,
};
pub use html::render_printable_html;
pub use record::EvidenceRecord;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use hre_core::Reason;
    use std::collections::BTreeMap;
    use uuid::Uuid;

    fn sample_record(kind: &str, summary: &str, severity: u8) -> EvidenceRecord {
        EvidenceRecord {
            record_id: Uuid::new_v4(),
            event_id: Uuid::new_v4().to_string(),
            timestamp_utc: "2026-10-28T12:00:00Z".into(),
            timestamp_local: "2026-10-28 17:30:00 IST".into(),
            kind: kind.into(),
            sender_pseudonym: "pseudonym_sender_123".into(),
            recipient_pseudonym: "pseudonym_target_456".into(),
            platform: "demo-chat".into(),
            summary: summary.into(),
            severity,
            tier: if severity >= 61 {
                "High".into()
            } else {
                "Review".into()
            },
            reasons: vec![Reason {
                code: "TEST_REASON".into(),
                text: "Sample evidence reason".into(),
                weight: f64::from(severity) / 100.0,
                evidence: BTreeMap::new(),
            }],
            prev_hash: String::new(),
            canonical_hash: String::new(),
        }
    }

    #[test]
    fn test_hash_chain_creation_and_tamper_detection() {
        let secret = b"super_secret_server_hmac_key_123";
        let records = vec![
            sample_record("message", "Threatening message text", 85),
            sample_record("image", "NSFW screenshot proxy hash 0x123", 75),
            sample_record("audio", "Voice note transcript: I will find you", 90),
            sample_record("call", "Unanswered night call duration 0s", 70),
        ];

        let export = build_timeline("target_user", records, secret).unwrap();
        assert_eq!(export.records.len(), 4);
        assert_eq!(export.records[0].prev_hash, GENESIS_PREV_HASH);
        assert_eq!(
            export.records[1].prev_hash,
            export.records[0].canonical_hash
        );
        assert_eq!(
            export.records[2].prev_hash,
            export.records[1].canonical_hash
        );
        assert_eq!(
            export.records[3].prev_hash,
            export.records[2].canonical_hash
        );

        // Verification must pass on untampered export
        assert_eq!(verify_timeline(&export, secret), Ok(()));

        // Tampering test 1: Mutating summary in a record breaks hash check
        let mut tampered = export.clone();
        tampered.records[1].summary = "Tampered benign summary".into();
        assert!(matches!(
            verify_timeline(&tampered, secret),
            Err(EvidenceError::RecordHashMismatch { .. })
        ));

        // Tampering test 2: Wrong HMAC key
        assert_eq!(
            verify_timeline(&export, b"wrong_secret_key_999"),
            Err(EvidenceError::InvalidSignature)
        );
    }

    #[test]
    fn test_scenario_9_one_click_multimodal_timeline() {
        let secret = b"demo_hmac_secret_key";
        let records = vec![
            sample_record("message", "Harassing message: I know your address", 80),
            sample_record("image", "Image pHash=0x38fa OCR='I will find you'", 75),
            sample_record(
                "audio",
                "Voice note transcript='Threatening audio clip'",
                85,
            ),
            sample_record("call", "Overnight call duration 0s unanswered", 70),
        ];

        let export = build_timeline("victim_pseudonym", records, secret).unwrap();
        assert_eq!(export.record_count, 4);
        assert_eq!(verify_timeline(&export, secret), Ok(()));

        // Generate printable HTML
        let html = render_printable_html(&export);
        assert!(html.contains("Bharatiya Sakshya Adhiniyam 2023, Section 63(4)"));
        assert!(html.contains("1930"));
        assert!(html.contains("cybercrime.gov.in"));
        assert!(html.contains("victim_pseudonym"));
        assert!(html.contains("Harassing message"));
        assert!(html.contains("Threatening audio clip"));
    }
}
