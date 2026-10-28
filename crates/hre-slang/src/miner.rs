use crate::lexicon::{SlangError, SlangLexicon, SlangTerm};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, RwLock},
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub struct SlangCandidate {
    pub id: Uuid,
    pub term: String,
    pub context_masked: String,
    pub sender_count: usize,
    pub score: f64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlangAuditEntry {
    pub candidate_id: Option<Uuid>,
    pub term: String,
    pub action: String, // "confirm", "reject", "retrain", "rollback"
    pub actor: String,
    pub timestamp: i64,
    pub details: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct CandidateMiner {
    candidates: Arc<RwLock<BTreeMap<Uuid, SlangCandidate>>>,
    sender_tracker: Arc<RwLock<BTreeMap<String, BTreeSet<String>>>>, // term -> set of senders
    audit_log: Arc<RwLock<Vec<SlangAuditEntry>>>,
}

impl CandidateMiner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingests a message from a risky conversation to mine potential slang tokens.
    pub fn ingest_risky_message(
        &self,
        text: &str,
        sender_id: &str,
        risk_score: f64,
        lexicon: &SlangLexicon,
    ) -> Result<Vec<Uuid>, SlangError> {
        let (masked_text, _) = hre_text::mask_pii(text);
        let normalized = hre_text::normalize(text);

        let mut discovered = Vec::new();
        let tokens: Vec<&str> = normalized
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|w| w.len() >= 3 && w.len() <= 30 && w.chars().all(|c| c.is_alphabetic()))
            .collect();

        let mut candidates = self
            .candidates
            .write()
            .map_err(|_| SlangError::LockError("candidates lock poisoned".into()))?;
        let mut senders = self
            .sender_tracker
            .write()
            .map_err(|_| SlangError::LockError("senders lock poisoned".into()))?;

        for token in tokens {
            let lower = token.to_lowercase();
            // Skip common benign English/Hinglish stop words or existing lexicon terms
            if is_common_stopword(&lower) || lexicon.get(&lower)?.is_some() {
                continue;
            }

            let entry = senders.entry(lower.clone()).or_default();
            entry.insert(sender_id.to_string());
            let distinct_count = entry.len();

            // Check if candidate already exists in queue
            if let Some(existing) = candidates.values_mut().find(|c| c.term == lower) {
                existing.sender_count = distinct_count;
                existing.score = existing.score.max(risk_score);
            } else {
                let id = Uuid::new_v4();
                candidates.insert(
                    id,
                    SlangCandidate {
                        id,
                        term: lower,
                        context_masked: masked_text.clone(),
                        sender_count: distinct_count,
                        score: risk_score,
                        created_at: chrono::Utc::now().timestamp_millis(),
                    },
                );
                discovered.push(id);
            }
        }

        Ok(discovered)
    }

    pub fn list_candidates(&self) -> Result<Vec<SlangCandidate>, SlangError> {
        let candidates = self
            .candidates
            .read()
            .map_err(|_| SlangError::LockError("candidates lock poisoned".into()))?;
        let mut list: Vec<SlangCandidate> = candidates.values().cloned().collect();
        // Sort by sender count and risk score descending
        list.sort_by(|a, b| {
            b.sender_count.cmp(&a.sender_count).then(
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
        });
        Ok(list)
    }

    /// Moderator confirms a candidate term into the active lexicon with explicit meaning & severity.
    pub fn confirm_candidate(
        &self,
        candidate_id: Uuid,
        meaning: &str,
        severity: f64,
        moderator: &str,
        lexicon: &SlangLexicon,
    ) -> Result<SlangTerm, SlangError> {
        let candidate = {
            let mut candidates = self
                .candidates
                .write()
                .map_err(|_| SlangError::LockError("candidates lock poisoned".into()))?;
            candidates
                .remove(&candidate_id)
                .ok_or_else(|| SlangError::CandidateNotFound(candidate_id.to_string()))?
        };

        let term = SlangTerm {
            term: candidate.term.clone(),
            meaning: meaning.to_string(),
            severity: severity.clamp(0.1, 0.60), // Slang alone never exceeds review tier (0.60)
            language: "en".into(),
            added_by: moderator.to_string(),
            created_at: chrono::Utc::now().timestamp_millis(),
            version: 1,
        };

        lexicon.insert(term.clone())?;

        let mut audit = self
            .audit_log
            .write()
            .map_err(|_| SlangError::LockError("audit lock poisoned".into()))?;
        audit.push(SlangAuditEntry {
            candidate_id: Some(candidate_id),
            term: candidate.term,
            action: "confirm".into(),
            actor: moderator.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            details: BTreeMap::from([
                ("meaning".into(), meaning.to_string()),
                ("severity".into(), format!("{severity:.2}")),
            ]),
        });

        Ok(term)
    }

    /// Moderator rejects a candidate term.
    pub fn reject_candidate(
        &self,
        candidate_id: Uuid,
        moderator: &str,
    ) -> Result<SlangCandidate, SlangError> {
        let candidate = {
            let mut candidates = self
                .candidates
                .write()
                .map_err(|_| SlangError::LockError("candidates lock poisoned".into()))?;
            candidates
                .remove(&candidate_id)
                .ok_or_else(|| SlangError::CandidateNotFound(candidate_id.to_string()))?
        };

        let mut audit = self
            .audit_log
            .write()
            .map_err(|_| SlangError::LockError("audit lock poisoned".into()))?;
        audit.push(SlangAuditEntry {
            candidate_id: Some(candidate_id),
            term: candidate.term.clone(),
            action: "reject".into(),
            actor: moderator.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            details: BTreeMap::new(),
        });

        Ok(candidate)
    }

    pub fn audit_log(&self) -> Result<Vec<SlangAuditEntry>, SlangError> {
        let audit = self
            .audit_log
            .read()
            .map_err(|_| SlangError::LockError("audit lock poisoned".into()))?;
        Ok(audit.clone())
    }
}

fn is_common_stopword(word: &str) -> bool {
    matches!(
        word,
        "the"
            | "and"
            | "you"
            | "that"
            | "was"
            | "for"
            | "are"
            | "with"
            | "his"
            | "they"
            | "this"
            | "have"
            | "from"
            | "one"
            | "had"
            | "word"
            | "but"
            | "not"
            | "what"
            | "all"
            | "were"
            | "when"
            | "your"
            | "can"
            | "said"
            | "there"
            | "use"
            | "each"
            | "which"
            | "she"
            | "how"
            | "their"
            | "will"
            | "other"
            | "about"
            | "many"
            | "then"
            | "them"
            | "these"
            | "some"
            | "her"
            | "would"
            | "make"
            | "like"
            | "him"
            | "into"
            | "time"
            | "has"
            | "look"
            | "two"
            | "more"
            | "write"
            | "see"
            | "number"
            | "way"
            | "could"
            | "people"
            | "than"
            | "first"
            | "water"
            | "been"
            | "call"
            | "who"
            | "oil"
            | "its"
            | "now"
            | "find"
            | "hello"
            | "good"
            | "morning"
            | "afternoon"
            | "evening"
            | "night"
            | "today"
            | "tomorrow"
            | "bhai"
            | "yaar"
            | "kya"
            | "hai"
            | "mein"
            | "nahi"
            | "theek"
    )
}
