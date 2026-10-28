use hre_core::*;
use std::collections::BTreeMap;

/// Nine deterministic client fixtures; no persistence, inference or real evidence export.
pub fn analyze(request: &AnalyzeRequest) -> AnalyzeResponse {
    let tag = request.sender_id.strip_prefix("mock:").unwrap_or("normal");
    let (score, code, text, action) = if tag.starts_with("polite_stalker") {
        if request.context.recipient_blocked_sender {
            (
                75,
                "CONTACT_AFTER_BLOCK",
                "Simulated repeated contact after block",
                Action::Throttle,
            )
        } else {
            (
                45,
                "PERSISTENCE",
                "Simulated one-sided persistence",
                Action::WarnSender,
            )
        }
    } else if tag.starts_with("pile_on") {
        (
            75,
            "PILE_ON",
            "Simulated ten accounts targeting one person",
            Action::Throttle,
        )
    } else if tag.starts_with("cyberflasher") || tag.starts_with("cyberflashing") {
        (
            70,
            "UNSOLICITED_MEDIA",
            "Benign proxy for unsolicited explicit media",
            Action::BlurMedia,
        )
    } else if tag.starts_with("screenshot") {
        (
            70,
            "OCR_ABUSE",
            "Simulated abusive screenshot text",
            Action::BlurMedia,
        )
    } else if tag.starts_with("voice_note") {
        (
            90,
            "THREAT",
            "Simulated threatening voice-note transcript",
            Action::Escalate,
        )
    } else if tag.starts_with("call_log") {
        (
            75,
            "REPEATED_CALLS",
            "Simulated overnight unanswered calls",
            Action::Throttle,
        )
    } else if tag.starts_with("slang") {
        (
            45,
            "SLANG_MATCH",
            "Simulated moderator-confirmed invented term",
            Action::WarnSender,
        )
    } else if tag.starts_with("evidence") {
        (
            45,
            "EVIDENCE_PREVIEW",
            "Simulated timeline preview; no export generated",
            Action::WarnSender,
        )
    } else {
        (
            8,
            "HEALTHY_CONTACT",
            "Simulated reciprocal conversation",
            Action::Log,
        )
    };
    let mut signal = Signal {
        score: f64::from(score) / 100.0,
        label: code.into(),
        status: Status::Simulated,
        backend: Backend::Heuristic,
        confidence: 1.0,
        details: BTreeMap::new(),
    };
    signal
        .details
        .insert("fixture".into(), serde_json::json!(true));
    let mut signals = Signals::default();
    match code {
        "PILE_ON" => signals.pile_on = signal,
        "UNSOLICITED_MEDIA" => signals.image = signal,
        "OCR_ABUSE" => {
            signal.details.insert(
                "ocr_text".into(),
                serde_json::json!("[synthetic abusive text]"),
            );
            signals.ocr = signal;
        }
        "THREAT" => {
            signal.details.insert(
                "transcript".into(),
                serde_json::json!("[synthetic threat transcript]"),
            );
            signals.audio = signal;
        }
        "REPEATED_CALLS" => signals.call = signal,
        "SLANG_MATCH" => signals.text = signal,
        _ => signals.behavior = signal,
    }
    AnalyzeResponse {
        event_id: request.event_id,
        severity: score,
        tier: tier(score),
        reasons: vec![Reason {
            code: code.into(),
            text: text.into(),
            weight: f64::from(score) / 100.0,
            evidence: BTreeMap::new(),
        }],
        signals,
        recommended_action: action,
        secondary_actions: if code == "PILE_ON" {
            vec![
                SecondaryAction::AlertModerator,
                SecondaryAction::CollapseReplies,
                SecondaryAction::RateLimit,
            ]
        } else if score >= 31 {
            vec![SecondaryAction::AlertModerator]
        } else {
            vec![]
        },
        alert_id: None,
        latency_ms: 0,
        model_versions: BTreeMap::new(),
        simulated: true,
        degraded: vec!["mock.no_inference".into(), "mock.no_persistence".into()],
        nudge: nudge(request.draft, score),
    }
}
pub(crate) fn tier(score: u8) -> Tier {
    match score {
        0..=30 => Tier::Low,
        31..=60 => Tier::Review,
        61..=85 => Tier::High,
        _ => Tier::Urgent,
    }
}
pub(crate) fn nudge(draft: bool, score: u8) -> Option<Nudge> {
    (draft && score >= 31).then(|| Nudge {
        text: "Pause before sending.".into(),
        rewrite_hint: "Respect the recipient's boundaries and wait for a reply.".into(),
    })
}
