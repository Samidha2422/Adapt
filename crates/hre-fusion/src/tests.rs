use super::*;
use hre_core::Signal;
fn active(score: f64) -> Signal {
    Signal {
        score,
        backend: Backend::Heuristic,
        status: Status::Built,
        ..Default::default()
    }
}
#[test]
fn noisy_or_matches_hand_calculation_and_missing_modules_degrade() {
    let mut input = FusionInput::default();
    input.signals.text = active(0.5);
    input.signals.behavior = active(0.4);
    let r = fuse(&input, &FusionConfig::default());
    assert_eq!(r.severity, 60);
    assert!(r.degraded.contains(&"ocr.unavailable".into()));
}
#[test]
fn every_safety_floor_and_action_is_enforced() {
    let cases = [
        (
            FusionInput {
                threat: 0.9,
                ..Default::default()
            },
            86,
            "THREAT_FLOOR",
        ),
        (
            FusionInput {
                sextortion_pattern: true,
                ..Default::default()
            },
            86,
            "SEXTORTION_PATTERN",
        ),
        (
            FusionInput {
                reported_hash_match: true,
                ..Default::default()
            },
            86,
            "REPORTED_HASH_MATCH",
        ),
        (
            FusionInput {
                synthetic_intimate: true,
                ..Default::default()
            },
            86,
            "POSSIBLE_SYNTHETIC_INTIMATE_MEDIA",
        ),
        (
            FusionInput {
                unsolicited_explicit: true,
                ..Default::default()
            },
            61,
            "UNSOLICITED_EXPLICIT_MEDIA",
        ),
        (
            FusionInput {
                after_refusal_count: 1,
                ..Default::default()
            },
            61,
            "CONTACT_AFTER_REFUSAL",
        ),
        (
            FusionInput {
                after_refusal_count: 3,
                ..Default::default()
            },
            75,
            "REPEATED_AFTER_REFUSAL",
        ),
        (
            FusionInput {
                doxxing_with_toxicity: true,
                ..Default::default()
            },
            70,
            "DOXXING_WITH_TOXICITY",
        ),
        (
            FusionInput {
                address_with_threat: true,
                ..Default::default()
            },
            86,
            "ADDRESS_WITH_THREAT",
        ),
        (
            FusionInput {
                feed_protection: true,
                ..Default::default()
            },
            61,
            "PILE_ON_FLOOR",
        ),
    ];
    for (input, expected, code) in cases {
        let r = fuse(&input, &FusionConfig::default());
        assert_eq!(r.severity, expected, "{code}");
        assert!(r.reasons.iter().any(|reason| reason.code == code));
        if input.unsolicited_explicit {
            assert_eq!(r.recommended_action, Action::BlurMedia);
        }
        if input.feed_protection {
            assert!(r
                .secondary_actions
                .contains(&SecondaryAction::CollapseReplies));
        }
    }
}
#[test]
fn contextual_caps_never_discount_explicit_safety_floors() {
    let mut input = FusionInput {
        healthy_reciprocity: true,
        ..Default::default()
    };
    input.signals.text = active(1.0);
    assert_eq!(fuse(&input, &FusionConfig::default()).severity, 55);
    input.reported_hash_match = true;
    assert_eq!(fuse(&input, &FusionConfig::default()).tier, Tier::Urgent);
    input.reported_hash_match = false;
    input.threat = 0.81;
    assert_eq!(fuse(&input, &FusionConfig::default()).severity, 80);
}
#[test]
fn tone_never_flags_alone_or_pushes_low_to_review() {
    let mut input = FusionInput {
        tone: 1.0,
        ..Default::default()
    };
    assert_eq!(fuse(&input, &FusionConfig::default()).severity, 0);
    input.signals.text = active(0.35);
    assert_eq!(fuse(&input, &FusionConfig::default()).severity, 30);
}
#[test]
fn supporting_tone_cannot_bypass_healthy_reciprocity_cap() {
    let mut input = FusionInput {
        healthy_reciprocity: true,
        tone: 1.0,
        ..Default::default()
    };
    input.signals.text = active(1.0);
    assert_eq!(fuse(&input, &FusionConfig::default()).severity, 55);
}
#[test]
fn c6_suppresses_hostile_reply_without_disabling_media_safety() {
    let mut input = FusionInput {
        reply_to_harasser: true,
        threat: 0.95,
        ..Default::default()
    };
    input.signals.text = active(0.95);
    input.signals.behavior = active(0.95);
    assert_eq!(fuse(&input, &FusionConfig::default()).tier, Tier::Low);
    input.reported_hash_match = true;
    assert_eq!(fuse(&input, &FusionConfig::default()).tier, Tier::Urgent);
}
#[test]
fn discounts_and_simulation_are_reported() {
    let mut input = FusionInput {
        quoted_abuse: true,
        banter: true,
        ..Default::default()
    };
    input.signals.text = active(1.0);
    input.signals.text.status = Status::Simulated;
    let r = fuse(&input, &FusionConfig::default());
    assert_eq!(r.severity, 28);
    assert!(r.simulated);
    assert!(r.reasons.iter().any(|r| r.code == "BANTER_DISCOUNT"));
}
#[test]
fn configs_are_strict_and_calibration_is_monotone() {
    assert!(FusionConfig::parse(include_str!("../../../config/fusion.toml")).is_ok());
    assert!(FusionConfig::parse("[weights]\ntext=2.0").is_err());
    let mut c = FusionConfig {
        calibration: vec![[0.0, 0.0], [50.0, 40.0], [100.0, 100.0]],
        ..Default::default()
    };
    assert!(c.validate().is_ok());
    assert_eq!(calibrate(25.0, &c.calibration), 20.0);
    c.calibration[1][1] = 101.0;
    assert!(c.validate().is_err());
}
proptest::proptest! {
    #[test]
    fn arbitrary_signals_always_produce_bounded_severity(v in proptest::num::f64::ANY) {
        let mut input=FusionInput::default();input.signals.text=active(v);proptest::prop_assert!(fuse(&input,&FusionConfig::default()).severity<=100);
    }
    #[test]
    fn config_parser_never_panics(s in ".{0,2048}") {let _=FusionConfig::parse(&s);}
}
