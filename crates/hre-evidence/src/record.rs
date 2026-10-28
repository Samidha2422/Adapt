use hre_core::Reason;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EvidenceRecord {
    pub record_id: Uuid,
    pub event_id: String,
    pub timestamp_utc: String,
    pub timestamp_local: String,
    pub kind: String,
    pub sender_pseudonym: String,
    pub recipient_pseudonym: String,
    pub platform: String,
    pub summary: String,
    pub severity: u8,
    pub tier: String,
    pub reasons: Vec<Reason>,
    pub prev_hash: String,
    pub canonical_hash: String,
}

impl EvidenceRecord {
    /// Computes the canonical SHA-256 hash of a record's deterministic content (excluding self canonical_hash).
    #[allow(clippy::too_many_arguments)]
    pub fn compute_canonical_hash(
        event_id: &str,
        timestamp_utc: &str,
        kind: &str,
        sender_pseudonym: &str,
        recipient_pseudonym: &str,
        summary: &str,
        severity: u8,
        prev_hash: &str,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(event_id.as_bytes());
        hasher.update(b"|");
        hasher.update(timestamp_utc.as_bytes());
        hasher.update(b"|");
        hasher.update(kind.as_bytes());
        hasher.update(b"|");
        hasher.update(sender_pseudonym.as_bytes());
        hasher.update(b"|");
        hasher.update(recipient_pseudonym.as_bytes());
        hasher.update(b"|");
        hasher.update(summary.as_bytes());
        hasher.update(b"|");
        hasher.update([severity]);
        hasher.update(b"|");
        hasher.update(prev_hash.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}
