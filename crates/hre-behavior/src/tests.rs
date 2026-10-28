use super::*;

struct FixedBackend(f64);
impl InferenceBackend for FixedBackend {
    fn predict(&self, _: &[f64]) -> Result<f64, hre_ml::ModelError> {
        Ok(self.0)
    }
    fn backend(&self) -> Backend {
        Backend::Onnx
    }
}

#[test]
fn invalid_model_probabilities_fall_back_with_honest_diagnostics() {
    let events = [event(0, "a", "b", 0)];
    let config = BehaviorConfig::default();
    let expected = analyze(&events, "a", "b", 0, &config).behavior.score;
    for value in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        let report =
            analyze_with_backend(&events, "a", "b", 0, &config, Some(&FixedBackend(value)));
        assert_eq!(report.behavior.score, expected);
        assert_eq!(report.behavior.backend, Backend::Heuristic);
        assert!(report
            .degraded
            .iter()
            .any(|d| d == "behavior_msg.inference_failed"));
        assert!(!report
            .degraded
            .iter()
            .any(|d| d == "behavior_msg.onnx_missing"));
    }
}

#[test]
fn trained_model_does_not_claim_logistic_occlusion_explanations() {
    let events = [event(0, "a", "b", 0)];
    let report = analyze_with_backend(
        &events,
        "a",
        "b",
        0,
        &BehaviorConfig::default(),
        Some(&FixedBackend(0.8)),
    );
    assert_eq!(report.behavior.backend, Backend::Onnx);
    assert_eq!(report.behavior.score, 0.8);
    assert!(!report
        .reasons
        .iter()
        .any(|r| r.code == "BEHAVIOR_OCCLUSION"));
    assert!(report
        .degraded
        .iter()
        .any(|d| d == "behavior.model_attribution_unavailable"));
}
fn event(i: usize, sender: &str, recipient: &str, ts: i64) -> StoredEvent {
    StoredEvent {
        event_id: format!("{i:05}"),
        sender: sender.into(),
        recipient: recipient.into(),
        timestamp_ms: ts,
        ..Default::default()
    }
}
#[test]
fn golden_vector_v1_matches_documented_fixture() {
    let mut events = vec![
        event(0, "a", "b", 0),
        event(1, "a", "b", 60_000),
        event(2, "b", "a", 120_000),
        event(3, "a", "b", 180_000),
    ];
    for (e, tox) in events.iter_mut().zip([0.1, 0.2, 0.0, 0.3]) {
        e.toxicity = tox;
        e.content_hash = Some("a".repeat(64));
    }
    let actual = extract_features(&events, "a", "b", 180_000, &BehaviorConfig::default()).vector();
    let mut expected = Vec::new();
    for _ in 0..3 {
        expected.extend([3.0, 0.25, 1.0, 0.2, 0.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    }
    expected.extend([
        1.0, 1.0, 90.0, 3.0, 0.0, -1.0, 1.0, 0.1, 0.0, 1.0, -1.0, 0.0, 1.0, 0.0,
    ]);
    assert_eq!(actual.len(), expected.len());
    for (index, (a, b)) in actual.iter().zip(expected).enumerate() {
        assert!((a - b).abs() < 1e-4, "feature {index}: {a} != {b}");
    }
}
#[test]
fn polite_stalker_rises_and_refusal_is_directional() {
    let c = BehaviorConfig::default();
    let mut events: Vec<_> = (0..20)
        .map(|i| event(i, "a", "b", i as i64 * 1000))
        .collect();
    assert!(analyze(&events, "a", "b", 20_000, &c).behavior.score >= 0.4);
    let mut refusal = event(20, "a", "b", 20_000);
    refusal.kind = EventKind::Refusal;
    events.push(refusal);
    events.push(event(21, "a", "b", 21_000));
    let report = analyze(&events, "a", "b", 21_000, &c);
    assert_eq!(report.after_refusal_count, 1);
    assert!(!report.reply_to_harasser);
}
#[test]
fn victim_reply_does_not_erase_reverse_unreplied_history() {
    let mut events: Vec<_> = (0..20)
        .map(|i| event(i, "a", "b", i as i64 * 1000))
        .collect();
    let mut reply = event(20, "b", "a", 20_000);
    reply.toxicity = 0.95;
    events.push(reply);
    let r = analyze(&events, "b", "a", 20_000, &BehaviorConfig::default());
    assert!(r.reply_to_harasser);
    assert_eq!(r.behavior.score, 0.0);
}
#[test]
fn reverse_guard_preserves_old_refusals_and_respects_unblocks() {
    let now = 8 * DAY;
    let mut marker = event(0, "a", "b", 0);
    marker.kind = EventKind::Refusal;
    marker.channel = "block".into();
    let contact = event(2, "a", "b", now - 1000);
    let reply = event(3, "b", "a", now);
    let config = BehaviorConfig::default();
    let report = analyze(
        &[marker.clone(), contact.clone(), reply.clone()],
        "b",
        "a",
        now,
        &config,
    );
    assert!(report.reply_to_harasser);
    assert_eq!(report.behavior.score, 0.0);

    let mut unblock = event(1, "a", "b", DAY);
    unblock.kind = EventKind::Refusal;
    unblock.channel = "unblock".into();
    let report = analyze(
        &[
            marker.clone(),
            unblock.clone(),
            contact.clone(),
            reply.clone(),
        ],
        "b",
        "a",
        now,
        &config,
    );
    assert!(!report.reply_to_harasser);

    marker.channel = "mute".into();
    let report = analyze(&[marker, unblock, contact, reply], "b", "a", now, &config);
    assert!(report.reply_to_harasser);
}
#[test]
fn reverse_guard_ignores_future_refusals_and_contacts_outside_24_hours() {
    let now = 8 * DAY;
    let mut marker = event(0, "a", "b", now + 1);
    marker.kind = EventKind::Refusal;
    marker.channel = "block".into();
    let contact = event(1, "a", "b", now - 1000);
    let reply = event(2, "b", "a", now);
    let config = BehaviorConfig::default();
    assert!(
        !analyze(
            &[marker.clone(), contact, reply.clone()],
            "b",
            "a",
            now,
            &config
        )
        .reply_to_harasser
    );

    marker.timestamp_ms = 0;
    let old_contact = event(3, "a", "b", now - DAY - 1);
    assert!(!analyze(&[marker, old_contact, reply], "b", "a", now, &config).reply_to_harasser);
}
#[test]
fn healthy_heavy_pair_stays_low() {
    let mut events = Vec::new();
    for i in 0..160 {
        let (s, r) = if i % 2 == 0 { ("b", "a") } else { ("a", "b") };
        events.push(event(i, s, r, i as i64 * 1000));
    }
    let r = analyze(&events, "a", "b", 160_000, &BehaviorConfig::default());
    assert!(r.healthy_reciprocity);
    assert!(r.behavior.score <= 0.1);
    assert!(!r.reply_to_harasser);
}
#[test]
fn long_answered_contact_calls_are_protective_but_refusal_is_not_discounted() {
    let mut events: Vec<_> = (0..80)
        .map(|i| {
            let mut e = event(i, "a", "b", i as i64 * 1000);
            e.kind = EventKind::Call;
            e.answered = true;
            e.duration_s = 120.0;
            e.is_contact = true;
            e
        })
        .collect();
    let c = BehaviorConfig::default();
    assert!(analyze(&events, "a", "b", 80_000, &c).behavior.score <= 0.1);
    events.last_mut().unwrap().after_refusal = true;
    let r = analyze(&events, "a", "b", 80_000, &c);
    assert!(!r.healthy_reciprocity);
    assert!(r.behavior.score >= 0.75);
}
#[test]
fn ten_accounts_per_minute_triggers_but_retries_and_old_events_do_not() {
    let c = BehaviorConfig::default();
    let mut events: Vec<_> = (0..9)
        .map(|i| event(i, &format!("sender{i}"), "target", 60_000))
        .collect();
    events.push(events[0].clone());
    events.push(event(100, "old", "target", -1));
    assert!(!analyze(&events, "sender0", "target", 60_000, &c).feed_protection);
    events.push(event(9, "sender9", "target", 60_000));
    let r = analyze(&events, "sender0", "target", 60_000, &c);
    assert!(r.feed_protection);
    assert_eq!(r.pile_on.details["distinct_senders"], 10);
}
#[test]
fn windows_exclude_future_and_old_events_and_night_uses_local_offset() {
    let c = BehaviorConfig {
        recipient_utc_offset_minutes: 330,
        ..Default::default()
    };
    let mut events = vec![
        event(0, "a", "b", 0),
        event(1, "a", "b", 3_600_000),
        event(2, "a", "b", 3_600_001),
        event(3, "a", "b", -7 * DAY - 1),
    ];
    events.push(events[0].clone());
    let f = extract_features(&events, "a", "b", 3_600_000, &c);
    assert_eq!(f.get("msg_count_1h"), 2.0);
    assert_eq!(f.get("night_ratio_1h"), 0.5);
}
#[test]
fn configuration_defaults_and_invalid_values() {
    assert!(BehaviorConfig::parse(include_str!("../../../config/behavior_rules.toml")).is_ok());
    assert!(BehaviorConfig::parse("pile_on_window_seconds = -1").is_err());
    assert!(BehaviorConfig::parse("night_start_hour = 24").is_err());
    assert!(BehaviorConfig::parse("[logistic_weights]\nunknown = 1.0").is_err());
}
#[test]
fn unblock_clears_previous_refusal_but_later_context_can_refuse_again() {
    let mut block = event(0, "a", "b", 0);
    block.kind = EventKind::Refusal;
    block.channel = "block".into();
    let mut attempt = event(1, "a", "b", 1000);
    attempt.after_refusal = true;
    let mut unblock = event(2, "a", "b", 2000);
    unblock.kind = EventKind::Refusal;
    unblock.channel = "unblock".into();
    let mut events = vec![block, attempt, unblock, event(3, "a", "b", 3000)];
    let c = BehaviorConfig::default();
    let f = extract_features(&events, "a", "b", 3000, &c);
    assert_eq!(f.get("after_refusal_count"), 0.0);
    assert_eq!(f.get("since_refusal_hours"), -1.0);
    let mut again = event(4, "a", "b", 4000);
    again.after_refusal = true;
    events.push(again);
    assert_eq!(
        extract_features(&events, "a", "b", 4000, &c).get("after_refusal_count"),
        1.0
    );
}
#[test]
fn equal_timestamp_contacts_use_their_after_refusal_flag() {
    let mut marker = event(10, "a", "b", 1000);
    marker.kind = EventKind::Refusal;
    marker.channel = "block".into();
    let mut after = event(1, "a", "b", 1000);
    after.after_refusal = true;
    let before = event(20, "a", "b", 1000);
    let f = extract_features(
        &[marker, after, before],
        "a",
        "b",
        1000,
        &BehaviorConfig::default(),
    );
    assert_eq!(f.get("after_refusal_count"), 1.0);
}
#[test]
fn unblock_preserves_mute_and_stop_requests() {
    for channel in ["mute", "stop_request"] {
        let mut refusal = event(0, "a", "b", 1000);
        refusal.kind = EventKind::Refusal;
        refusal.channel = channel.into();
        let mut unblock = event(1, "a", "b", 2000);
        unblock.kind = EventKind::Refusal;
        unblock.channel = "unblock".into();
        let f = extract_features(
            &[refusal, unblock, event(2, "a", "b", 3000)],
            "a",
            "b",
            3000,
            &BehaviorConfig::default(),
        );
        assert_eq!(f.get("after_refusal_count"), 1.0);
        assert!(f.get("since_refusal_hours") >= 0.0);
    }
}
#[test]
fn old_refusal_marker_applies_to_recent_contacts() {
    let mut refusal = event(0, "a", "b", 0);
    refusal.kind = EventKind::Refusal;
    refusal.channel = "mute".into();
    let now = 8 * DAY;
    let f = extract_features(
        &[refusal, event(1, "a", "b", now)],
        "a",
        "b",
        now,
        &BehaviorConfig::default(),
    );
    assert_eq!(f.get("after_refusal_count"), 1.0);
    assert_eq!(f.get("since_refusal_hours"), 192.0);
}
proptest::proptest! {
    #[test]
    fn config_parser_never_panics(s in ".{0,1024}") {let _=BehaviorConfig::parse(&s);}
    #[test]
    fn permutation_does_not_change_features(n in 0usize..100) {
        let mut events:Vec<_>=(0..n).map(|i|event(i,"a","b",i as i64*1000)).collect();
        let before=extract_features(&events,"a","b",100_000,&BehaviorConfig::default()).vector();events.reverse();
        proptest::prop_assert_eq!(before,extract_features(&events,"a","b",100_000,&BehaviorConfig::default()).vector());
    }
}
