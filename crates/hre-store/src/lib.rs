//! SQLite event memory. Ingest must pseudonymize identifiers before calling this crate.
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use std::str::FromStr;

/// Storage failures never include event payloads or identifiers.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database operation failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("invalid stored record: {0}")]
    Json(#[from] serde_json::Error),
    #[error("event ID already exists with different content")]
    IdempotencyConflict,
    #[error("invalid event: {0}")]
    Invalid(&'static str),
}

/// Refusal records use the refused sender → refusing recipient direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Message,
    Image,
    Audio,
    Call,
    Refusal,
}

/// Scores and metadata only: no raw message, image, audio, or raw user IDs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StoredEvent {
    pub event_id: String,
    pub sender: String,
    pub recipient: String,
    pub timestamp_ms: i64,
    pub kind: EventKind,
    pub channel: String,
    pub toxicity: f64,
    pub threat: f64,
    pub sexual: f64,
    pub sentiment: f64,
    pub pii_mentions: u32,
    pub location_cues: u32,
    pub unsolicited_media: bool,
    pub content_hash: Option<String>,
    pub sender_account_age_days: Option<f64>,
    pub is_contact: bool,
    pub recipient_initiated: bool,
    pub after_refusal: bool,
    pub answered: bool,
    pub duration_s: f64,
    pub simulated: bool,
}
impl Default for StoredEvent {
    fn default() -> Self {
        Self {
            event_id: String::new(),
            sender: String::new(),
            recipient: String::new(),
            timestamp_ms: 0,
            kind: EventKind::Message,
            channel: String::new(),
            toxicity: 0.0,
            threat: 0.0,
            sexual: 0.0,
            sentiment: 0.0,
            pii_mentions: 0,
            location_cues: 0,
            unsolicited_media: false,
            content_hash: None,
            sender_account_age_days: None,
            is_contact: false,
            recipient_initiated: false,
            after_refusal: false,
            answered: false,
            duration_s: 0.0,
            simulated: false,
        }
    }
}
/// Accept only hex HMAC-SHA256 identifiers (pseudonymization itself belongs upstream).
pub fn is_pseudonym(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
impl StoredEvent {
    /// Validate finite scores and pseudonymous IDs before persistence.
    pub fn validate(&self) -> Result<(), StoreError> {
        if !is_pseudonym(&self.sender) || !is_pseudonym(&self.recipient) {
            return Err(StoreError::Invalid(
                "sender and recipient must be HMAC hex pseudonyms",
            ));
        }
        if self.event_id.is_empty() || self.event_id.len() > 256 || self.channel.len() > 256 {
            return Err(StoreError::Invalid("invalid event ID or channel length"));
        }
        if [self.toxicity, self.threat, self.sexual]
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || !self.sentiment.is_finite()
            || !(-1.0..=1.0).contains(&self.sentiment)
            || !self.duration_s.is_finite()
            || self.duration_s < 0.0
            || self
                .sender_account_age_days
                .is_some_and(|v| !v.is_finite() || v < 0.0)
        {
            return Err(StoreError::Invalid("nonfinite or out-of-range score"));
        }
        if self.content_hash.as_ref().is_some_and(|v| !is_pseudonym(v)) {
            return Err(StoreError::Invalid("content hash must be SHA256 hex"));
        }
        Ok(())
    }
}

/// Cloneable asynchronous repository. Connections use SQLite foreign keys and a busy timeout.
#[derive(Clone, Debug)]
pub struct Store {
    pool: SqlitePool,
}
impl Store {
    /// Connect and run the bundled idempotent schema migration without compile-time SQL macros.
    pub async fn connect(url: &str) -> Result<Self, StoreError> {
        let options = sqlx::sqlite::SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .foreign_keys(true)
            .busy_timeout(std::time::Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        sqlx::raw_sql(include_str!("../migrations/0001_events.sql"))
            .execute(&pool)
            .await?;
        Ok(Self { pool })
    }
    /// Atomic insertion; exact retries return false, conflicting retries return an error.
    pub async fn insert_event(&self, event: &StoredEvent) -> Result<bool, StoreError> {
        event.validate()?;
        let payload = serde_json::to_string(event)?;
        let mut tx = self.pool.begin().await?;
        let inserted = sqlx::query("INSERT INTO events(event_id,sender,recipient,timestamp_ms,kind,payload) VALUES(?,?,?,?,?,?) ON CONFLICT(event_id) DO NOTHING")
            .bind(&event.event_id).bind(&event.sender).bind(&event.recipient).bind(event.timestamp_ms)
            .bind(serde_json::to_string(&event.kind)?).bind(&payload).execute(&mut *tx).await?.rows_affected() == 1;
        if !inserted {
            let old: String = sqlx::query_scalar("SELECT payload FROM events WHERE event_id=?")
                .bind(&event.event_id)
                .fetch_one(&mut *tx)
                .await?;
            if old != payload {
                return Err(StoreError::IdempotencyConflict);
            }
        }
        tx.commit().await?;
        Ok(inserted)
    }
    /// Latest marker per refusal channel, including unblock; caller resolves active state.
    pub async fn latest_refusal_markers(
        &self,
        sender: &str,
        recipient: &str,
        to_ms: i64,
    ) -> Result<Vec<StoredEvent>, StoreError> {
        let mut markers = Vec::new();
        for channel in ["block", "unblock", "mute", "stop_request"] {
            let payload: Option<String> = sqlx::query_scalar("SELECT payload FROM events WHERE sender=? AND recipient=? AND kind=? AND json_extract(payload, '$.channel')=? AND timestamp_ms<=? ORDER BY timestamp_ms DESC,event_id DESC LIMIT 1")
                .bind(sender).bind(recipient).bind("\"refusal\"").bind(channel).bind(to_ms)
                .fetch_optional(&self.pool).await?;
            if let Some(payload) = payload {
                markers.push(serde_json::from_str(&payload)?);
            }
        }
        Ok(markers)
    }

    /// Atomically persist an event, masked assessment, optional masked alert and protection.
    /// Exact event retries return the original assessment, without overwriting side effects.
    /// The caller must compare its request hash with the returned assessment on retry.
    pub async fn persist_assessment(
        &self,
        event: &StoredEvent,
        assessment: &serde_json::Value,
        alert: Option<&serde_json::Value>,
        protect_feed: bool,
    ) -> Result<(bool, serde_json::Value), StoreError> {
        event.validate()?;
        let payload = serde_json::to_string(event)?;
        let assessment_payload = serde_json::to_string(assessment)?;
        let mut tx = self.pool.begin().await?;
        let inserted = sqlx::query("INSERT INTO events(event_id,sender,recipient,timestamp_ms,kind,payload) VALUES(?,?,?,?,?,?) ON CONFLICT(event_id) DO NOTHING")
            .bind(&event.event_id).bind(&event.sender).bind(&event.recipient).bind(event.timestamp_ms)
            .bind(serde_json::to_string(&event.kind)?).bind(&payload).execute(&mut *tx).await?.rows_affected() == 1;
        if !inserted {
            let old: String = sqlx::query_scalar("SELECT payload FROM events WHERE event_id=?")
                .bind(&event.event_id)
                .fetch_one(&mut *tx)
                .await?;
            if old != payload {
                return Err(StoreError::IdempotencyConflict);
            }
            let existing: Option<String> =
                sqlx::query_scalar("SELECT payload FROM assessments WHERE event_id=?")
                    .bind(&event.event_id)
                    .fetch_optional(&mut *tx)
                    .await?;
            if let Some(existing) = existing {
                tx.commit().await?;
                return Ok((false, serde_json::from_str(&existing)?));
            }
        }
        sqlx::query("INSERT INTO assessments(event_id,payload) VALUES(?,?)")
            .bind(&event.event_id)
            .bind(&assessment_payload)
            .execute(&mut *tx)
            .await?;
        if let Some(alert) = alert {
            sqlx::query("INSERT INTO alerts(event_id,payload) VALUES(?,?)")
                .bind(&event.event_id)
                .bind(serde_json::to_string(alert)?)
                .execute(&mut *tx)
                .await?;
        }
        if protect_feed {
            sqlx::query("INSERT INTO feed_protection(recipient,protected) VALUES(?,1) ON CONFLICT(recipient) DO UPDATE SET protected=1")
                .bind(&event.recipient).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok((inserted, assessment.clone()))
    }

    /// Durable masked alerts in event order. Publishing and acknowledgement belong to the caller.
    pub async fn alerts(&self) -> Result<Vec<serde_json::Value>, StoreError> {
        let rows: Vec<String> = sqlx::query_scalar("SELECT alerts.payload FROM alerts JOIN events USING(event_id) ORDER BY events.timestamp_ms,events.event_id")
            .fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|row| Ok(serde_json::from_str(&row)?))
            .collect()
    }

    /// Whether feed protection has been durably activated for a pseudonymous recipient.
    pub async fn feed_protected(&self, recipient: &str) -> Result<bool, StoreError> {
        if !is_pseudonym(recipient) {
            return Err(StoreError::Invalid(
                "recipient must be an HMAC hex pseudonym",
            ));
        }
        let value: Option<i64> =
            sqlx::query_scalar("SELECT protected FROM feed_protection WHERE recipient=?")
                .bind(recipient)
                .fetch_optional(&self.pool)
                .await?;
        Ok(value == Some(1))
    }
    /// Inclusive directional pair window, stable ordering even for equal timestamps.
    pub async fn pair_window(
        &self,
        sender: &str,
        recipient: &str,
        from_ms: i64,
        to_ms: i64,
    ) -> Result<Vec<StoredEvent>, StoreError> {
        let rows = sqlx::query("SELECT payload FROM events WHERE sender=? AND recipient=? AND timestamp_ms BETWEEN ? AND ? ORDER BY timestamp_ms,event_id")
            .bind(sender).bind(recipient).bind(from_ms).bind(to_ms).fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|r| Ok(serde_json::from_str(r.try_get::<&str, _>("payload")?)?))
            .collect()
    }
    /// Inclusive target window across all senders for pile-on analysis.
    pub async fn target_window(
        &self,
        recipient: &str,
        from_ms: i64,
        to_ms: i64,
    ) -> Result<Vec<StoredEvent>, StoreError> {
        let rows = sqlx::query("SELECT payload FROM events WHERE recipient=? AND timestamp_ms BETWEEN ? AND ? ORDER BY timestamp_ms,event_id")
            .bind(recipient).bind(from_ms).bind(to_ms).fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|r| Ok(serde_json::from_str(r.try_get::<&str, _>("payload")?)?))
            .collect()
    }
    /// Persist a PII-masked API assessment; the API owns masking and DTO validation.
    pub async fn save_assessment(
        &self,
        event_id: &str,
        assessment: &serde_json::Value,
    ) -> Result<(), StoreError> {
        sqlx::query("INSERT INTO assessments(event_id,payload) VALUES(?,?) ON CONFLICT(event_id) DO UPDATE SET payload=excluded.payload")
            .bind(event_id).bind(serde_json::to_string(assessment)?).execute(&self.pool).await?;
        Ok(())
    }
    /// Fetch a cached assessment for API retry handling.
    pub async fn assessment(
        &self,
        event_id: &str,
    ) -> Result<Option<serde_json::Value>, StoreError> {
        let value: Option<String> =
            sqlx::query_scalar("SELECT payload FROM assessments WHERE event_id=?")
                .bind(event_id)
                .fetch_optional(&self.pool)
                .await?;
        value
            .map(|s| serde_json::from_str(&s).map_err(StoreError::from))
            .transpose()
    }

    /// Insert a reported perceptual hash and SHA-256 for abuse detection (hash-only storage).
    pub async fn insert_reported_hash(
        &self,
        p_hash: u64,
        sha256: &str,
        label: &str,
        reported_at: i64,
    ) -> Result<bool, StoreError> {
        let inserted = sqlx::query(
            "INSERT INTO reported_hashes(p_hash, sha256, label, reported_at) VALUES(?,?,?,?) ON CONFLICT(sha256) DO NOTHING",
        )
        .bind(p_hash as i64)
        .bind(sha256)
        .bind(label)
        .bind(reported_at)
        .execute(&self.pool)
        .await?
        .rows_affected()
            == 1;
        Ok(inserted)
    }

    /// Query reported hashes and find the closest match within max_hamming_distance.
    pub async fn find_reported_hash(
        &self,
        p_hash: u64,
        max_distance: u32,
    ) -> Result<Option<ReportedHashMatch>, StoreError> {
        let rows = sqlx::query("SELECT p_hash, sha256, label FROM reported_hashes")
            .fetch_all(&self.pool)
            .await?;
        let mut best: Option<ReportedHashMatch> = None;
        for row in rows {
            let row_phash = row.try_get::<i64, _>("p_hash")? as u64;
            let dist = (p_hash ^ row_phash).count_ones();
            if dist <= max_distance {
                let candidate = ReportedHashMatch {
                    p_hash: row_phash,
                    sha256: row.try_get::<&str, _>("sha256")?.to_string(),
                    label: row.try_get::<&str, _>("label")?.to_string(),
                    distance: dist,
                };
                if best
                    .as_ref()
                    .is_none_or(|b| candidate.distance < b.distance)
                {
                    best = Some(candidate);
                }
            }
        }
        Ok(best)
    }
}

/// A matched reported hash result with Hamming distance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportedHashMatch {
    pub p_hash: u64,
    pub sha256: String,
    pub label: String,
    pub distance: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(id: &str, ts: i64) -> StoredEvent {
        StoredEvent {
            event_id: id.into(),
            sender: "a".repeat(64),
            recipient: "b".repeat(64),
            timestamp_ms: ts,
            ..Default::default()
        }
    }
    #[tokio::test]
    async fn retries_conflict_and_windows_are_directional_and_inclusive() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        let a = event("a", 100);
        assert!(store.insert_event(&a).await.unwrap());
        assert!(!store.insert_event(&a).await.unwrap());
        let mut conflict = a.clone();
        conflict.toxicity = 0.5;
        assert!(matches!(
            store.insert_event(&conflict).await,
            Err(StoreError::IdempotencyConflict)
        ));
        store.insert_event(&event("future", 101)).await.unwrap();
        assert_eq!(
            store
                .pair_window(&a.sender, &a.recipient, 100, 100)
                .await
                .unwrap(),
            vec![a.clone()]
        );
        assert!(store
            .pair_window(&a.recipient, &a.sender, 0, 200)
            .await
            .unwrap()
            .is_empty());
        store
            .save_assessment("a", &serde_json::json!({"severity": 12}))
            .await
            .unwrap();
        assert_eq!(
            store.assessment("a").await.unwrap().unwrap()["severity"],
            12
        );
    }
    #[tokio::test]
    async fn raw_identifiers_are_rejected() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        let mut e = event("e", 0);
        e.sender = "raw-user".into();
        assert!(matches!(
            store.insert_event(&e).await,
            Err(StoreError::Invalid(_))
        ));
    }
    #[tokio::test]
    async fn assessment_alert_and_protection_are_atomic_and_retry_safe() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        let e = event("atomic", 100);
        let assessment = serde_json::json!({"request_hash": "hash", "response": {"severity": 75}});
        let alert = serde_json::json!({"severity": 75});
        sqlx::query("CREATE TRIGGER reject_alert BEFORE INSERT ON alerts BEGIN SELECT RAISE(ABORT, 'test'); END")
            .execute(&store.pool).await.unwrap();
        assert!(store
            .persist_assessment(&e, &assessment, Some(&alert), true)
            .await
            .is_err());
        assert!(store
            .pair_window(&e.sender, &e.recipient, 0, 100)
            .await
            .unwrap()
            .is_empty());
        assert!(store.assessment(&e.event_id).await.unwrap().is_none());
        assert!(!store.feed_protected(&e.recipient).await.unwrap());
        sqlx::query("DROP TRIGGER reject_alert")
            .execute(&store.pool)
            .await
            .unwrap();
        assert!(
            store
                .persist_assessment(&e, &assessment, Some(&alert), true)
                .await
                .unwrap()
                .0
        );
        let retry = store
            .persist_assessment(&e, &serde_json::json!({"changed": true}), None, false)
            .await
            .unwrap();
        assert!(!retry.0);
        assert_eq!(retry.1, assessment);
        assert_eq!(store.alerts().await.unwrap(), vec![alert]);
        assert!(store.feed_protected(&e.recipient).await.unwrap());
        let mut conflict = e.clone();
        conflict.toxicity = 0.5;
        assert!(matches!(
            store
                .persist_assessment(&conflict, &assessment, None, false)
                .await,
            Err(StoreError::IdempotencyConflict)
        ));
    }
    #[tokio::test]
    async fn latest_refusal_markers_are_directional_and_bounded_by_time() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        for (id, channel, ts) in [
            ("a", "block", 1),
            ("b", "block", 2),
            ("c", "mute", 2),
            ("d", "unblock", 3),
        ] {
            let mut e = event(id, ts);
            e.kind = EventKind::Refusal;
            e.channel = channel.into();
            store.insert_event(&e).await.unwrap();
        }
        let sender = "a".repeat(64);
        let recipient = "b".repeat(64);
        let markers = store
            .latest_refusal_markers(&sender, &recipient, 2)
            .await
            .unwrap();
        assert_eq!(
            markers
                .iter()
                .map(|e| e.event_id.as_str())
                .collect::<Vec<_>>(),
            vec!["b", "c"]
        );
        assert!(store
            .latest_refusal_markers(&recipient, &sender, 5)
            .await
            .unwrap()
            .is_empty());
    }
    #[tokio::test]
    async fn test_reported_hashes_insert_and_find_within_distance() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        let p_hash = 0b1010101010101010u64;
        let sha = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert!(store
            .insert_reported_hash(p_hash, sha, "ncii_test", 1000)
            .await
            .unwrap());
        // Duplicate sha256 insert should return false (idempotent)
        assert!(!store
            .insert_reported_hash(p_hash, sha, "ncii_test", 1000)
            .await
            .unwrap());

        // Exact match (distance 0)
        let exact = store.find_reported_hash(p_hash, 0).await.unwrap();
        assert!(exact.is_some());
        assert_eq!(exact.unwrap().distance, 0);

        // Near match (distance 2)
        let near_hash = p_hash ^ 0b11;
        let found = store.find_reported_hash(near_hash, 2).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().distance, 2);

        // Outside threshold (distance 2 with threshold 1)
        let outside = store.find_reported_hash(near_hash, 1).await.unwrap();
        assert!(outside.is_none());
    }
    proptest::proptest! {
        #[test]
        fn pseudonym_parser_never_accepts_wrong_length(s in ".{0,128}") {
            if is_pseudonym(&s) { proptest::prop_assert_eq!(s.len(), 64); }
        }
    }
}
