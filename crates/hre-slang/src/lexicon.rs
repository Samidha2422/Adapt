use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub struct SlangTerm {
    pub term: String,
    pub meaning: String,
    pub severity: f64,
    pub language: String,
    pub added_by: String,
    pub created_at: i64,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlangMatch {
    pub term: String,
    pub meaning: String,
    pub severity: f64,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SlangLexicon {
    terms: Arc<RwLock<BTreeMap<String, SlangTerm>>>,
}

impl SlangLexicon {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, term: SlangTerm) -> Result<(), SlangError> {
        let key = term.term.trim().to_lowercase();
        if key.is_empty() {
            return Err(SlangError::InvalidTerm("term cannot be empty".into()));
        }
        let mut map = self
            .terms
            .write()
            .map_err(|_| SlangError::LockError("lexicon lock poisoned".into()))?;
        map.insert(key, term);
        Ok(())
    }

    pub fn remove(&self, term: &str) -> Result<Option<SlangTerm>, SlangError> {
        let key = term.trim().to_lowercase();
        let mut map = self
            .terms
            .write()
            .map_err(|_| SlangError::LockError("lexicon lock poisoned".into()))?;
        Ok(map.remove(&key))
    }

    pub fn get(&self, term: &str) -> Result<Option<SlangTerm>, SlangError> {
        let key = term.trim().to_lowercase();
        let map = self
            .terms
            .read()
            .map_err(|_| SlangError::LockError("lexicon lock poisoned".into()))?;
        Ok(map.get(&key).cloned())
    }

    pub fn list(&self) -> Result<Vec<SlangTerm>, SlangError> {
        let map = self
            .terms
            .read()
            .map_err(|_| SlangError::LockError("lexicon lock poisoned".into()))?;
        Ok(map.values().cloned().collect())
    }

    /// Finds confirmed slang terms in input text using word boundary matching.
    pub fn find_matches(&self, text: &str) -> Result<Vec<SlangMatch>, SlangError> {
        let map = self
            .terms
            .read()
            .map_err(|_| SlangError::LockError("lexicon lock poisoned".into()))?;

        if map.is_empty() {
            return Ok(Vec::new());
        }

        let lower = text.to_lowercase();
        let mut matches = Vec::new();

        for (term_key, slang_term) in map.iter() {
            let mut search_start = 0;
            while let Some(rel_pos) = lower[search_start..].find(term_key) {
                let start = search_start + rel_pos;
                let end = start + term_key.len();
                search_start = end;

                // Check left word boundary
                let left_ok = if start == 0 {
                    true
                } else {
                    let prev_char = lower[..start].chars().next_back().unwrap_or(' ');
                    !prev_char.is_alphanumeric()
                };

                // Check right word boundary
                let right_ok = if end >= lower.len() {
                    true
                } else {
                    let next_char = lower[end..].chars().next().unwrap_or(' ');
                    !next_char.is_alphanumeric()
                };

                if left_ok && right_ok {
                    matches.push(SlangMatch {
                        term: slang_term.term.clone(),
                        meaning: slang_term.meaning.clone(),
                        severity: slang_term.severity,
                        start,
                        end,
                    });
                }
            }
        }

        matches.sort_by_key(|m| m.start);
        Ok(matches)
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SlangError {
    #[error("invalid slang term: {0}")]
    InvalidTerm(String),
    #[error("lock error: {0}")]
    LockError(String),
    #[error("candidate not found: {0}")]
    CandidateNotFound(String),
    #[error("drift guard violation: validation precision dropped by {drop:.4} (max allowed 0.02)")]
    DriftGuardViolation { drop: f64 },
    #[error("no previous adapter version to rollback to")]
    NoPreviousVersion,
}
