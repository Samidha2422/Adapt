pub mod adapter;
pub mod lexicon;
pub mod miner;
pub mod temporal;

pub use adapter::{AdapterHead, AdapterManager, ADAPTER_FEATURE_DIM};
pub use lexicon::{SlangError, SlangLexicon, SlangMatch, SlangTerm};
pub use miner::{CandidateMiner, SlangAuditEntry, SlangCandidate};
pub use temporal::{run_temporal_experiment, TemporalExperimentResult};

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use hre_core::Tier;

    #[test]
    fn test_lexicon_insert_and_word_boundary_match() {
        let lexicon = SlangLexicon::new();
        let term = SlangTerm {
            term: "vexling".into(),
            meaning: "invented harassing slur".into(),
            severity: 0.45,
            language: "en".into(),
            added_by: "moderator1".into(),
            created_at: 1000,
            version: 1,
        };
        lexicon.insert(term).unwrap();

        // Exact match with punctuation
        let matches = lexicon.find_matches("Stop being a vexling!").unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].term, "vexling");
        assert_eq!(matches[0].severity, 0.45);

        // Substring inside larger word must NOT match
        let no_matches = lexicon.find_matches("This is unvexlinghood").unwrap();
        assert!(no_matches.is_empty());
    }

    #[test]
    fn test_candidate_miner_and_sender_accumulation() {
        let miner = CandidateMiner::new();
        let lexicon = SlangLexicon::new();

        miner
            .ingest_risky_message("You are such a grokling", "sender_1", 0.70, &lexicon)
            .unwrap();
        miner
            .ingest_risky_message("Hey grokling look here", "sender_2", 0.80, &lexicon)
            .unwrap();

        let candidates = miner.list_candidates().unwrap();
        let grokling = candidates.iter().find(|c| c.term == "grokling").unwrap();
        assert_eq!(grokling.sender_count, 2);
        assert_eq!(grokling.score, 0.80);
    }

    #[test]
    fn test_moderator_gate_confirm_and_reject() {
        let miner = CandidateMiner::new();
        let lexicon = SlangLexicon::new();

        miner
            .ingest_risky_message("Don't be a sneetch", "sender_1", 0.65, &lexicon)
            .unwrap();
        let candidates = miner.list_candidates().unwrap();
        let sneetch = candidates.iter().find(|c| c.term == "sneetch").unwrap();

        let confirmed = miner
            .confirm_candidate(
                sneetch.id,
                "discriminatory label",
                0.40,
                "mod_user",
                &lexicon,
            )
            .unwrap();
        assert_eq!(confirmed.term, "sneetch");
        assert_eq!(confirmed.severity, 0.40);

        // Lexicon now immediately matches
        let matches = lexicon.find_matches("such a sneetch").unwrap();
        assert_eq!(matches.len(), 1);

        // Audit log records confirmation
        let audit = miner.audit_log().unwrap();
        assert!(audit
            .iter()
            .any(|a| a.term == "sneetch" && a.action == "confirm"));

        // Reject another candidate
        miner
            .ingest_risky_message("dummy quxword", "sender_1", 0.50, &lexicon)
            .unwrap();
        let qux = miner
            .list_candidates()
            .unwrap()
            .into_iter()
            .find(|c| c.term == "quxword")
            .unwrap();
        let rejected = miner.reject_candidate(qux.id, "mod_user").unwrap();
        assert_eq!(rejected.term, "quxword");

        let audit2 = miner.audit_log().unwrap();
        assert!(audit2
            .iter()
            .any(|a| a.term == "quxword" && a.action == "reject"));
    }

    #[test]
    fn test_adapter_retrain_drift_guard_and_rollback() {
        let adapter = AdapterManager::new();
        let initial_head = adapter.current_head().unwrap();
        assert_eq!(initial_head.version, 1);

        let train_data = vec![
            ([1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], 1.0),
            ([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 0.0),
        ];

        // Validation set with high agreement: precision = 1.0
        let good_val_data = vec![
            ([1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], 1.0),
            ([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 0.0),
        ];

        let new_head = adapter.retrain(&train_data, &good_val_data).unwrap();
        assert_eq!(new_head.version, 2);

        // Rollback restores version 1
        let rolled_back = adapter.rollback().unwrap();
        assert_eq!(rolled_back.version, 1);
        assert_eq!(adapter.current_head().unwrap().version, 1);

        // Bad validation set causing drift (precision drop > 0.02)
        let drifting_val_data = vec![
            ([0.9, 0.9, 0.9, 0.9, 0.9, 0.9, 0.9, 0.9], 0.0), // False positive
        ];
        let err = adapter
            .retrain(&train_data, &drifting_val_data)
            .unwrap_err();
        assert!(matches!(err, SlangError::DriftGuardViolation { .. }));
    }

    #[test]
    fn test_scenario_8_temporal_split_miss_confirm_caught() {
        let result = run_temporal_experiment(
            "vexling",
            "invented harassing code word",
            0.45,
            "You are such a vexling",
        );

        assert_eq!(result.term, "vexling");
        assert!(!result.before_match);
        assert_eq!(result.before_tier, Tier::Low); // Miss before confirmation
        assert!(result.after_match);
        assert_eq!(result.after_tier, Tier::Review); // Caught after confirmation
        assert!(result.caught);
    }
}
