use crate::record::EvidenceRecord;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

pub const GENESIS_PREV_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub struct CertificateDraft {
    pub act_reference: String,
    pub part_a_statement: String,
    pub part_b_advisory: String,
    pub emergency_helplines: Vec<String>,
}

impl Default for CertificateDraft {
    fn default() -> Self {
        Self {
            act_reference: "Bharatiya Sakshya Adhiniyam 2023, Section 63(4)".into(),
            part_a_statement: "Electronic record produced by computer output. SHA-256 hash chain and HMAC manifest guarantee integrity and tamper-evidence.".into(),
            part_b_advisory: "draft aid, not legal advice; Part B must come from an expert; verify with counsel. Never call this export court-certified.".into(),
            emergency_helplines: vec![
                "cybercrime.gov.in (anonymous reporting for cybercrimes against women and children)".into(),
                "National Cyber Crime Helpline: 1930".into(),
                "Women Helpline: 181".into(),
                "National Emergency Number: 112".into(),
                "StopNCII.org (non-consensual intimate image removal aid)".into(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EvidenceExport {
    pub export_id: Uuid,
    pub subject_pseudonym: String,
    pub generated_at: String,
    pub record_count: usize,
    pub records: Vec<EvidenceRecord>,
    pub manifest_hash: String,
    pub hmac_signature: String,
    pub certificate_draft: CertificateDraft,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EvidenceError {
    #[error("tampered record hash at index {index}: calculated {calculated}, expected {expected}")]
    RecordHashMismatch {
        index: usize,
        calculated: String,
        expected: String,
    },
    #[error("broken hash chain link at index {index}: prev_hash {prev_hash} does not match predecessor {predecessor_hash}")]
    BrokenChainLink {
        index: usize,
        prev_hash: String,
        predecessor_hash: String,
    },
    #[error("manifest hash mismatch: calculated {calculated}, expected {expected}")]
    ManifestMismatch {
        calculated: String,
        expected: String,
    },
    #[error("tampered export signature: HMAC verification failed")]
    InvalidSignature,
    #[error("HMAC key error: {0}")]
    KeyError(String),
}

/// Builds an immutable, hash-chained evidence export signed with HMAC-SHA256.
pub fn build_timeline(
    subject_pseudonym: &str,
    mut records: Vec<EvidenceRecord>,
    secret_key: &[u8],
) -> Result<EvidenceExport, EvidenceError> {
    let mut prev = GENESIS_PREV_HASH.to_string();

    for r in records.iter_mut() {
        r.prev_hash = prev.clone();
        r.canonical_hash = EvidenceRecord::compute_canonical_hash(
            &r.event_id,
            &r.timestamp_utc,
            &r.kind,
            &r.sender_pseudonym,
            &r.recipient_pseudonym,
            &r.summary,
            r.severity,
            &r.prev_hash,
        );
        prev = r.canonical_hash.clone();
    }

    // Compute manifest hash: sha256 of all canonical hashes concatenated
    let mut manifest_hasher = Sha256::new();
    for r in &records {
        manifest_hasher.update(r.canonical_hash.as_bytes());
    }
    let manifest_hash = format!("{:x}", manifest_hasher.finalize());

    // Compute HMAC signature over the manifest hash
    let mut mac = HmacSha256::new_from_slice(secret_key)
        .map_err(|e| EvidenceError::KeyError(e.to_string()))?;
    mac.update(manifest_hash.as_bytes());
    let hmac_signature = format!("{:x}", mac.finalize().into_bytes());

    Ok(EvidenceExport {
        export_id: Uuid::new_v4(),
        subject_pseudonym: subject_pseudonym.to_string(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        record_count: records.len(),
        records,
        manifest_hash,
        hmac_signature,
        certificate_draft: CertificateDraft::default(),
    })
}

/// Verifies cryptographic integrity of the entire timeline, hash chain, and HMAC signature.
pub fn verify_timeline(export: &EvidenceExport, secret_key: &[u8]) -> Result<(), EvidenceError> {
    let mut expected_prev = GENESIS_PREV_HASH.to_string();

    for (i, r) in export.records.iter().enumerate() {
        if r.prev_hash != expected_prev {
            return Err(EvidenceError::BrokenChainLink {
                index: i,
                prev_hash: r.prev_hash.clone(),
                predecessor_hash: expected_prev,
            });
        }

        let computed = EvidenceRecord::compute_canonical_hash(
            &r.event_id,
            &r.timestamp_utc,
            &r.kind,
            &r.sender_pseudonym,
            &r.recipient_pseudonym,
            &r.summary,
            r.severity,
            &r.prev_hash,
        );
        if computed != r.canonical_hash {
            return Err(EvidenceError::RecordHashMismatch {
                index: i,
                calculated: computed,
                expected: r.canonical_hash.clone(),
            });
        }
        expected_prev = r.canonical_hash.clone();
    }

    // Check manifest hash
    let mut manifest_hasher = Sha256::new();
    for r in &export.records {
        manifest_hasher.update(r.canonical_hash.as_bytes());
    }
    let computed_manifest = format!("{:x}", manifest_hasher.finalize());
    if computed_manifest != export.manifest_hash {
        return Err(EvidenceError::ManifestMismatch {
            calculated: computed_manifest,
            expected: export.manifest_hash.clone(),
        });
    }

    // Check HMAC signature
    let mut mac = HmacSha256::new_from_slice(secret_key)
        .map_err(|e| EvidenceError::KeyError(e.to_string()))?;
    mac.update(export.manifest_hash.as_bytes());
    let computed_sig = format!("{:x}", mac.finalize().into_bytes());
    if computed_sig != export.hmac_signature {
        return Err(EvidenceError::InvalidSignature);
    }

    Ok(())
}
