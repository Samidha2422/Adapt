use crate::lexicon::{SlangLexicon, SlangMatch, SlangTerm};
use crate::miner::CandidateMiner;
use hre_core::Tier;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalExperimentResult {
    pub term: String,
    pub before_match: bool,
    pub before_tier: Tier,
    pub confirmed_meaning: String,
    pub confirmed_severity: f64,
    pub after_match: bool,
    pub after_tier: Tier,
    pub caught: bool,
}

/// Runs a deterministic temporal-split experiment on an invented code word.
pub fn run_temporal_experiment(
    term: &str,
    meaning: &str,
    severity: f64,
    sample_message: &str,
) -> TemporalExperimentResult {
    let lexicon = SlangLexicon::new();
    let miner = CandidateMiner::new();

    // Step 1: Before confirmation - term is not in lexicon
    let before_matches: Vec<SlangMatch> = lexicon.find_matches(sample_message).unwrap_or_default();
    let before_match = !before_matches.is_empty();
    let before_score = 0.05; // benign baseline text score
    let before_tier = if before_score >= 0.31 {
        Tier::Review
    } else {
        Tier::Low
    };

    // Step 2: Message triggers mining into candidate queue
    let _candidate_ids = miner
        .ingest_risky_message(sample_message, "sender_1", 0.65, &lexicon)
        .unwrap_or_default();

    // Step 3: Moderator confirms candidate
    let confirmed_term = SlangTerm {
        term: term.to_string(),
        meaning: meaning.to_string(),
        severity,
        language: "en".into(),
        added_by: "moderator".into(),
        created_at: chrono::Utc::now().timestamp_millis(),
        version: 1,
    };
    let _ = lexicon.insert(confirmed_term);

    // Step 4: After confirmation - subsequent message with invented word is scanned
    let after_matches: Vec<SlangMatch> = lexicon.find_matches(sample_message).unwrap_or_default();
    let after_match = !after_matches.is_empty();
    let slang_boost = after_matches.first().map_or(0.0, |m| m.severity);
    let after_score = before_score + slang_boost;
    let after_tier = if after_score >= 0.31 {
        Tier::Review
    } else {
        Tier::Low
    };

    TemporalExperimentResult {
        term: term.to_string(),
        before_match,
        before_tier,
        confirmed_meaning: meaning.to_string(),
        confirmed_severity: severity,
        after_match,
        after_tier,
        caught: after_match && after_tier == Tier::Review,
    }
}
