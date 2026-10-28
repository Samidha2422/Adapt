use hre_core::{Backend, Reason, Signal, Status};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepfakeAssessment {
    pub face_detected: bool,
    pub deepfake_score: f64,
    pub nsfw_score: f64,
    pub synthetic_intimate: bool,
    pub low_confidence: bool,
    pub signal: Signal,
    pub reason: Option<Reason>,
}

/// Composite rule for synthetic intimate media detection (F6 / PS5 row 3.5).
///
/// If a face is detected, deepfake score is high (>= 0.70), and NSFW score is high (>= 0.50):
/// Flags `POSSIBLE_SYNTHETIC_INTIMATE_MEDIA`, sets tier floor to urgent (86) in fusion,
/// labels low confidence (0.40), and routes to human review.
pub fn evaluate_deepfake_composite(
    face_detected: bool,
    deepfake_score: f64,
    nsfw_score: f64,
) -> DeepfakeAssessment {
    let finite_score = |score: f64| {
        if score.is_finite() {
            score.clamp(0.0, 1.0)
        } else {
            0.0
        }
    };
    let deepfake = finite_score(deepfake_score);
    let nsfw = finite_score(nsfw_score);

    let synthetic_intimate = face_detected && deepfake >= 0.70 && nsfw >= 0.50;

    let mut details = BTreeMap::new();
    details.insert("simulated".into(), serde_json::json!(true));
    details.insert("face_detected".into(), serde_json::json!(face_detected));
    details.insert("deepfake_score".into(), serde_json::json!(deepfake));
    details.insert("nsfw_score".into(), serde_json::json!(nsfw));
    details.insert(
        "composite_rule_fired".into(),
        serde_json::json!(synthetic_intimate),
    );

    if synthetic_intimate {
        let mut evidence = BTreeMap::new();
        evidence.insert("deepfake_score".into(), serde_json::json!(deepfake));
        evidence.insert("nsfw_score".into(), serde_json::json!(nsfw));
        evidence.insert("low_confidence".into(), serde_json::json!(true));

        DeepfakeAssessment {
            face_detected,
            deepfake_score: deepfake,
            nsfw_score: nsfw,
            synthetic_intimate: true,
            low_confidence: true,
            signal: Signal {
                score: deepfake.max(nsfw),
                label: "possible_synthetic_intimate".into(),
                status: Status::Simulated,
                backend: Backend::Heuristic,
                confidence: 0.40,
                details,
            },
            reason: Some(Reason {
                code: "POSSIBLE_SYNTHETIC_INTIMATE_MEDIA".into(),
                text: "Possible synthetic intimate media; low confidence, human review required"
                    .into(),
                weight: 0.86,
                evidence,
            }),
        }
    } else {
        DeepfakeAssessment {
            face_detected,
            deepfake_score: deepfake,
            nsfw_score: nsfw,
            synthetic_intimate: false,
            low_confidence: false,
            signal: Signal {
                score: 0.0,
                label: "simulated_composite_no_match".into(),
                status: Status::Simulated,
                backend: Backend::Heuristic,
                confidence: 0.0,
                details,
            },
            reason: None,
        }
    }
}
