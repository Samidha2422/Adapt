use crate::features::CallFeatures;
use hre_core::{Action, Backend, Reason, SecondaryAction, Signal, Status, Tier};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallRiskReport {
    pub severity: u8,
    pub tier: Tier,
    pub signal: Signal,
    pub reasons: Vec<Reason>,
    pub recommended_action: Action,
    pub secondary_actions: Vec<SecondaryAction>,
}

/// Evaluates harassment risk from directional call features.
pub fn evaluate_call_risk(features: &CallFeatures, after_refusal_count: usize) -> CallRiskReport {
    let mut score = 0.0f64;
    let mut reasons = Vec::new();

    // Volume & Persistence risk
    if features.call_count_24h >= 20 {
        let volume_score = ((features.call_count_24h as f64 - 10.0) / 30.0).clamp(0.0, 1.0);
        score += volume_score * 0.40;
    }

    // Overnight calling risk
    if features.night_call_ratio_24h >= 0.50 && features.call_count_24h >= 5 {
        score += features.night_call_ratio_24h * 0.35;
        let mut evidence = BTreeMap::new();
        evidence.insert(
            "night_calls_ratio".into(),
            serde_json::json!(features.night_call_ratio_24h),
        );
        evidence.insert(
            "call_count_24h".into(),
            serde_json::json!(features.call_count_24h),
        );
        reasons.push(Reason {
            code: "NIGHT_CALLS".into(),
            text: format!(
                "{} calls overnight during recipient rest hours",
                features.call_count_24h
            ),
            weight: 0.35,
            evidence,
        });
    }

    // Unanswered / Silent calls harassment
    if features.unanswered_ratio_24h >= 0.70 && features.call_count_24h >= 5 {
        score += features.unanswered_ratio_24h * 0.25;
        let mut evidence = BTreeMap::new();
        evidence.insert(
            "unanswered_ratio".into(),
            serde_json::json!(features.unanswered_ratio_24h),
        );
        reasons.push(Reason {
            code: "UNANSWERED_CALLS".into(),
            text: format!(
                "{:.0}% of incoming calls went unanswered",
                features.unanswered_ratio_24h * 100.0
            ),
            weight: 0.25,
            evidence,
        });
    }

    if features.silent_calls_24h >= 3 {
        score += 0.20;
        let mut evidence = BTreeMap::new();
        evidence.insert(
            "silent_calls".into(),
            serde_json::json!(features.silent_calls_24h),
        );
        reasons.push(Reason {
            code: "SILENT_CALLS".into(),
            text: format!(
                "{} silent calls (duration < 5s) received",
                features.silent_calls_24h
            ),
            weight: 0.20,
            evidence,
        });
    }

    // Scenario 6 pattern: >= 30 calls overnight, mostly unanswered
    let overnight_harassment = features.call_count_24h >= 25
        && features.night_call_ratio_24h >= 0.50
        && features.unanswered_ratio_24h >= 0.60;
    if overnight_harassment {
        score = score.max(0.65);
        let mut evidence = BTreeMap::new();
        evidence.insert(
            "call_count".into(),
            serde_json::json!(features.call_count_24h),
        );
        reasons.push(Reason {
            code: "REPEATED_CALLS".into(),
            text: format!(
                "Persistent overnight calling pattern: {} calls",
                features.call_count_24h
            ),
            weight: 0.65,
            evidence,
        });
    }

    // Contact after block/refusal floors
    if after_refusal_count >= 3 {
        score = score.max(0.75);
        let mut evidence = BTreeMap::new();
        evidence.insert(
            "after_refusal_count".into(),
            serde_json::json!(after_refusal_count),
        );
        reasons.push(Reason {
            code: "REPEATED_AFTER_REFUSAL".into(),
            text: "Repeated calls continued after block or stop request".into(),
            weight: 0.75,
            evidence,
        });
    } else if after_refusal_count >= 1 {
        score = score.max(0.61);
        let mut evidence = BTreeMap::new();
        evidence.insert(
            "after_refusal_count".into(),
            serde_json::json!(after_refusal_count),
        );
        reasons.push(Reason {
            code: "CONTACT_AFTER_REFUSAL".into(),
            text: "Call received after contact refusal or block".into(),
            weight: 0.61,
            evidence,
        });
    }

    // Protective discount for long answered calls (Benign couple Scenario 7)
    if features.call_count_24h > 0
        && features.unanswered_ratio_24h <= 0.10
        && features.avg_duration_s_24h >= 300.0
        && after_refusal_count == 0
    {
        score = (score * 0.2).min(0.15);
        let mut evidence = BTreeMap::new();
        evidence.insert(
            "avg_duration_s".into(),
            serde_json::json!(features.avg_duration_s_24h),
        );
        reasons.push(Reason {
            code: "HEALTHY_CALL_RECIPROCITY".into(),
            text: "High call answer rate and long duration indicate consensual contact".into(),
            weight: 0.05,
            evidence,
        });
    }

    let severity = (score.clamp(0.0, 1.0) * 100.0).round() as u8;
    let tier = match severity {
        0..=30 => Tier::Low,
        31..=60 => Tier::Review,
        61..=85 => Tier::High,
        _ => Tier::Urgent,
    };

    let recommended_action = match tier {
        Tier::Low => Action::Log,
        Tier::Review => Action::WarnSender,
        Tier::High => Action::Throttle,
        Tier::Urgent => Action::Escalate,
    };

    let mut secondary_actions = Vec::new();
    if matches!(tier, Tier::High | Tier::Urgent) {
        secondary_actions.push(SecondaryAction::AlertModerator);
    }

    let mut details = BTreeMap::new();
    details.insert(
        "call_count_24h".into(),
        serde_json::json!(features.call_count_24h),
    );
    details.insert(
        "night_call_ratio_24h".into(),
        serde_json::json!(features.night_call_ratio_24h),
    );
    details.insert(
        "unanswered_ratio_24h".into(),
        serde_json::json!(features.unanswered_ratio_24h),
    );

    let signal = Signal {
        score: f64::from(severity) / 100.0,
        label: if severity >= 31 {
            "call_harassment".into()
        } else {
            "call_benign".into()
        },
        status: Status::Built,
        backend: Backend::Heuristic,
        confidence: 0.90,
        details,
    };

    CallRiskReport {
        severity,
        tier,
        signal,
        reasons,
        recommended_action,
        secondary_actions,
    }
}
