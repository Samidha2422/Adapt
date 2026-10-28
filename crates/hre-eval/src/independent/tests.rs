use super::*;

#[test]
fn generator_is_repeatable_pseudonymous_and_chronological() -> Result<(), EvalError> {
    let a = generator_b::generate(71, 1)?;
    let b = generator_b::generate(71, 1)?;
    assert_eq!(a.len(), 17);
    for (left, right) in a.iter().zip(&b) {
        let left_events: Vec<_> = left.events.iter().map(|e| &e.stored).collect();
        let right_events: Vec<_> = right.events.iter().map(|e| &e.stored).collect();
        assert_eq!(
            serde_json::to_value(&left_events)?,
            serde_json::to_value(&right_events)?
        );
        assert!(left_events
            .windows(2)
            .all(|w| w[0].timestamp_ms < w[1].timestamp_ms));
        for event in left_events {
            event
                .validate()
                .map_err(|e| std::io::Error::other(e.to_string()))?;
        }
    }
    let different = generator_b::generate(72, 1)?;
    assert_ne!(a[1].events.len(), different[1].events.len());
    Ok(())
}

#[test]
fn report_keeps_misses_null_and_metrics_recomputable_without_bodies() -> Result<(), EvalError> {
    let report = evaluate(
        71,
        1,
        hre_behavior::BehaviorConfig::default(),
        FusionConfig::default(),
    )?;
    let rows: Vec<_> = report
        .conversations
        .iter()
        .map(|r| (r.positive, f64::from(r.max_severity) / 100.0))
        .collect();
    let recomputed = metrics::Metrics::calculate(&rows, 0.31);
    assert_eq!(
        report.modules["fusion"]
            .metrics
            .as_ref()
            .map(|m| m.confusion),
        Some(recomputed.confusion)
    );
    assert_eq!(
        report.detection.detected_conversations,
        report
            .conversations
            .iter()
            .filter(|r| r.positive && r.first_review_event.is_some())
            .count()
    );
    assert!(report
        .conversations
        .iter()
        .all(|r| r.first_review_event.is_none() == (r.max_severity < 31)));
    assert!(report
        .conversations
        .iter()
        .filter_map(|r| r.first_review_event.map(|n| (r, n)))
        .all(|(r, n)| n >= 1 && n <= r.contact_events));
    for name in ["ocr", "image", "audio", "call"] {
        assert!(report.modules[name].metrics.is_none());
        assert!(report.modules[name].reason.is_some());
        assert_eq!(report.modules[name].backend, Backend::Disabled);
    }
    assert!(report.ablations["plus_calls"].metrics.is_none());
    assert!(report.ablations["full"].metrics.is_none());
    assert!(report.slices["polite_stalker_keyword"]
        .metrics
        .as_ref()
        .is_some_and(|m| m.confusion.fn_ == 1));
    assert!(report.slices["heavy_benign_calls"]
        .metrics
        .as_ref()
        .is_some_and(|m| m.n == 1 && m.recall.is_none()));
    let json = serde_json::to_string(&report)?;
    assert!(!json.contains("Hope your afternoon"));
    assert!(!json.contains("private pictures tonight"));
    assert!(!json.contains("masked_text"));
    assert!(!json.contains("sender\""));
    assert!(json.contains("\"fn\":"));
    assert!(json.contains("\"simulated\":true"));
    Ok(())
}

#[test]
fn rejects_invalid_limits_and_config_instead_of_reporting_fallback_as_requested_config() {
    assert!(matches!(
        evaluate(
            0,
            0,
            hre_behavior::BehaviorConfig::default(),
            FusionConfig::default()
        ),
        Err(EvalError::InvalidVariants)
    ));
    assert!(matches!(
        evaluate(
            0,
            65,
            hre_behavior::BehaviorConfig::default(),
            FusionConfig::default()
        ),
        Err(EvalError::InvalidVariants)
    ));
    let behavior = hre_behavior::BehaviorConfig {
        unreplied_review_count: 0,
        ..Default::default()
    };
    assert!(matches!(
        evaluate(0, 1, behavior, FusionConfig::default()),
        Err(EvalError::Behavior(_))
    ));
}
