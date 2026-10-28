pub mod classifier;
pub mod csv;
pub mod features;

pub use classifier::{evaluate_call_risk, CallRiskReport};
pub use csv::{parse_call_log_csv, CallRecord, CallsError};
pub use features::{extract_call_features, CallFeatureConfig, CallFeatures};

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use hre_core::{Action, CallDirection, Tier};

    #[test]
    fn test_parse_valid_csv() {
        let csv = "timestamp,caller,callee,duration_s,answered,direction\n\
                   1793188800,alice,bob,120,true,incoming\n\
                   1793188900,alice,bob,0,false,incoming";
        let records = parse_call_log_csv(csv).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].caller, "alice");
        assert_eq!(records[0].duration_s, 120);
        assert!(records[0].answered);
        assert!(!records[1].answered);
    }

    #[test]
    fn test_parse_csv_missing_column() {
        let csv = "caller,callee,duration_s\nalice,bob,120";
        assert_eq!(
            parse_call_log_csv(csv),
            Err(CallsError::MissingColumn("timestamp"))
        );
    }

    #[test]
    fn test_scenario_6_overnight_unanswered_calls_classified_high() {
        let config = CallFeatureConfig::default();
        // 40 calls overnight, all unanswered
        let base_ts = 1_793_188_800_000_i64; // arbitrary night time
        let mut history = Vec::new();
        for i in 0..40 {
            history.push(CallRecord {
                timestamp_ms: base_ts + i * 60_000,
                caller: "caller".into(),
                callee: "callee".into(),
                duration_s: 0,
                answered: false,
                direction: CallDirection::Incoming,
                voicemail: false,
            });
        }

        let features =
            extract_call_features(&history, "caller", "callee", base_ts + 40 * 60_000, &config);
        assert_eq!(features.call_count_24h, 40);
        assert_eq!(features.unanswered_ratio_24h, 1.0);

        let report = evaluate_call_risk(&features, 0);
        assert_eq!(report.tier, Tier::High);
        assert_eq!(report.recommended_action, Action::Throttle);
        assert!(report
            .reasons
            .iter()
            .any(|r| r.code == "REPEATED_CALLS" || r.code == "UNANSWERED_CALLS"));
    }

    #[test]
    fn test_calls_after_block_triggers_urgent_or_high() {
        let features = CallFeatures {
            call_count_1h: 1,
            call_count_24h: 3,
            call_count_7d: 3,
            max_calls_1h: 1,
            unanswered_ratio_24h: 0.0,
            silent_calls_24h: 0,
            avg_duration_s_24h: 60.0,
            night_call_ratio_24h: 0.0,
            after_block_calls: 3,
            new_number_flag: false,
            gap_regularity: 0.0,
            voicemail_count_24h: 0,
        };
        let report = evaluate_call_risk(&features, 3);
        assert_eq!(report.tier, Tier::High);
        assert_eq!(report.severity, 75);
        assert!(report
            .reasons
            .iter()
            .any(|r| r.code == "REPEATED_AFTER_REFUSAL"));
    }

    #[test]
    fn test_scenario_7_healthy_couple_calls_stay_low() {
        let features = CallFeatures {
            call_count_1h: 1,
            call_count_24h: 5,
            call_count_7d: 20,
            max_calls_1h: 1,
            unanswered_ratio_24h: 0.0,
            silent_calls_24h: 0,
            avg_duration_s_24h: 900.0, // 15-minute answered calls
            night_call_ratio_24h: 0.0,
            after_block_calls: 0,
            new_number_flag: false,
            gap_regularity: 0.0,
            voicemail_count_24h: 0,
        };
        let report = evaluate_call_risk(&features, 0);
        assert_eq!(report.tier, Tier::Low);
        assert_eq!(report.recommended_action, Action::Log);
    }

    proptest::proptest! {
        #[test]
        fn test_csv_parser_never_panics(s in "\\PC{0,1024}") {
            let _ = parse_call_log_csv(&s);
        }
    }
}
