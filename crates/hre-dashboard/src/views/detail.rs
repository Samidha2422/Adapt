//! Alert Detail & Investigation View (C2 deliverable).
//! Displays 8 modality breakdown, attribution bars, reasons, and review actions.

use super::queue::html_escape;
use hre_core::{Action, AnalyzeResponse, Backend, Signal, Status, Tier};

/// Renders a single modality signal meter card.
fn render_signal_card(name: &str, signal: &Signal) -> String {
    let score_pct = (signal.score * 100.0).clamp(0.0, 100.0);
    let conf_pct = (signal.confidence * 100.0).clamp(0.0, 100.0);

    let (status_str, status_class) = match signal.status {
        Status::Built => ("built", "badge-built"),
        Status::Simulated => ("simulated", "badge-simulated"),
        Status::Roadmap => ("roadmap", "badge-roadmap"),
    };

    let backend_str = match signal.backend {
        Backend::Onnx => "onnx",
        Backend::Heuristic => "heuristic",
        Backend::Disabled => "disabled",
        Backend::Ggml => "ggml",
    };

    let details_json = serde_json::to_string_pretty(&signal.details).unwrap_or_default();

    format!(
        r##"<div class="signal-card">
  <div class="signal-header">
    <span class="signal-name">{}</span>
    <div style="display: flex; gap: 6px;">
      <span class="badge {}">{}</span>
      <span class="badge badge-roadmap" style="font-size: 10px;">{}</span>
    </div>
  </div>

  <div style="display: flex; justify-content: space-between; align-items: baseline; margin-top: 8px;">
    <span style="font-size: 20px; font-weight: 700; color: var(--md-sys-color-primary);" class="table-numbers">{:.2}</span>
    <span style="font-size: 11px; color: var(--md-sys-color-on-surface-variant);">Conf: {:.0}%</span>
  </div>

  <div class="progress-track" title="Risk Score">
    <div class="progress-fill" style="width: {:.1}%;"></div>
  </div>

  <div style="margin-top: 8px; font-size: 11px; color: var(--md-sys-color-on-surface-variant); display: flex; justify-content: space-between;">
    <span>Label: <strong>{}</strong></span>
  </div>

  <details style="margin-top: 8px; font-size: 11px;">
    <summary style="cursor: pointer; color: var(--md-sys-color-primary);">Details JSON</summary>
    <pre style="margin-top: 4px; padding: 6px; background: var(--md-sys-color-surface-container-highest); border-radius: 4px; font-size: 10px; overflow-x: auto;">{}</pre>
  </details>
</div>"##,
        name,
        status_class,
        status_str,
        backend_str,
        signal.score,
        conf_pct,
        score_pct,
        html_escape(if signal.label.is_empty() {
            "none"
        } else {
            &signal.label
        }),
        html_escape(&details_json)
    )
}

/// Renders the complete Alert Detail view.
pub fn render_detail_page(alert: &AnalyzeResponse) -> String {
    let (tier_str, tier_badge_class) = match alert.tier {
        Tier::Urgent => ("urgent", "badge-urgent"),
        Tier::High => ("high", "badge-high"),
        Tier::Review => ("review", "badge-review"),
        Tier::Low => ("low", "badge-low"),
    };

    let simulated_badge = if alert.simulated {
        r#"<span class="badge badge-simulated" style="font-size: 13px;">Simulated Pipeline Active</span>"#
    } else {
        r#"<span class="badge badge-built" style="font-size: 13px;">Engine result</span>"#
    };

    let degraded_notice = if !alert.degraded.is_empty() {
        format!(
            r#"<div style="margin-top: 12px; padding: 10px 14px; background: var(--hre-tier-high-bg); color: var(--hre-tier-high); border-radius: 8px; font-size: 13px;">
  <strong>Degraded Backends:</strong> {}
</div>"#,
            html_escape(&alert.degraded.join(", "))
        )
    } else {
        String::new()
    };

    // Reasons HTML
    let mut reasons_html = String::new();
    for reason in &alert.reasons {
        let weight_pct = (reason.weight * 100.0).clamp(0.0, 100.0);
        let evidence_str = serde_json::to_string(&reason.evidence).unwrap_or_default();
        reasons_html.push_str(&format!(
            r##"<div style="padding: 12px; border-bottom: 1px solid var(--md-sys-color-outline-variant);">
  <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px;">
    <strong style="color: var(--md-sys-color-primary); font-size: 14px;">{}</strong>
    <span class="table-numbers" style="font-size: 12px; font-weight: 600; color: var(--md-sys-color-on-surface-variant);">Weight: {:.0}%</span>
  </div>
  <div style="font-size: 13px; color: var(--md-sys-color-on-surface); margin-bottom: 6px;">{}</div>
  <div class="progress-track" style="margin-bottom: 6px;">
    <div class="progress-fill" style="width: {:.1}%; background-color: var(--md-sys-color-tertiary);"></div>
  </div>
  <div style="font-size: 11px; color: var(--md-sys-color-outline); font-family: var(--md-sys-typescale-mono);">{}</div>
</div>"##,
            html_escape(&reason.code),
            weight_pct,
            html_escape(&reason.text),
            weight_pct,
            html_escape(&evidence_str)
        ));
    }

    // Partition active vs inactive modality signals to cut cognitive noise
    let sigs = &alert.signals;
    let all_signals = [
        ("Text NLP", &sigs.text),
        ("Vision OCR", &sigs.ocr),
        ("Image NSFW", &sigs.image),
        ("Audio Tone/ASR", &sigs.audio),
        ("Call Patterns", &sigs.call),
        ("Behavioral Trajectory", &sigs.behavior),
        ("Social Pile-On", &sigs.pile_on),
        ("PII Leakage", &sigs.pii),
    ];

    let mut active_cards = String::new();
    let mut inactive_badges = String::new();

    for (name, sig) in all_signals {
        if sig.score > 0.0 {
            active_cards.push_str(&render_signal_card(name, sig));
        } else {
            let backend_name = match sig.backend {
                Backend::Disabled => "Disabled",
                Backend::Heuristic => "Heuristic",
                Backend::Onnx => "ONNX",
                Backend::Ggml => "GGML",
            };
            let (status, class) = match sig.status {
                Status::Built => ("built", "badge-built"),
                Status::Simulated => ("simulated", "badge-simulated"),
                Status::Roadmap => ("roadmap", "badge-roadmap"),
            };
            inactive_badges.push_str(&format!(
                r#"<span class="badge {class}" style="font-size: 11px;">{name} ({backend_name}, {status})</span>"#,
            ));
        }
    }

    if active_cards.is_empty() {
        active_cards.push_str(&render_signal_card("Text NLP", &sigs.text));
        active_cards.push_str(&render_signal_card("Behavioral Trajectory", &sigs.behavior));
    }

    let inactive_tray = if !inactive_badges.is_empty() {
        format!(
            r##"<div style="margin-top: 14px; padding: 10px 14px; background: var(--md-sys-color-surface-container); border-radius: 8px; font-size: 12px; color: var(--md-sys-color-on-surface-variant); display: flex; flex-wrap: wrap; gap: 8px; align-items: center;">
  <span style="font-weight: 600; font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em;">Zero-score modalities:</span>
  {}
</div>"##,
            inactive_badges
        )
    } else {
        String::new()
    };

    let alert_id_str = alert
        .alert_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| alert.event_id.to_string());

    let action_str = match alert.recommended_action {
        Action::Log => "log",
        Action::WarnSender => "warn_sender",
        Action::BlurMedia => "blur_media",
        Action::Throttle => "throttle",
        Action::Escalate => "escalate",
    };

    format!(
        r##"<div class="page-header">
  <div>
    <div style="display: flex; align-items: center; gap: 10px; margin-bottom: 6px;">
      <a href="/dashboard/queue" class="btn btn-secondary" style="font-size: 12px; padding: 4px 10px;">&larr; Back to Queue</a>
      <span class="badge {}">{}</span>
      {}
    </div>
    <h1 class="page-title">Alert Investigation: {}</h1>
    <p class="page-subtitle">Event ID: <code>{}</code> | Latency: <strong>{}ms</strong></p>
  </div>

  <div style="text-align: right;">
    <div style="font-size: 12px; color: var(--md-sys-color-on-surface-variant);">Overall Severity Score</div>
    <div style="font-size: 36px; font-weight: 800; color: var(--md-sys-color-primary);" class="table-numbers">{}/100</div>
  </div>
</div>

{}

<!-- Grid layout: Left Signals & Attribution, Right Moderation Actions -->
<div style="display: grid; grid-template-columns: 2fr 1fr; gap: 24px; align-items: start;">
  <div>
    <!-- Modality Signals Grid -->
    <div class="card">
      <h2 class="card-title">8-Modality Risk Signals</h2>
      <div class="signal-grid">
        {}
      </div>
      {}
    </div>

    <!-- Attribution & Reasons -->
    <div class="card">
      <h2 class="card-title">Attribution & Reason Breakdown</h2>
      <div>
        {}
      </div>
    </div>

    <!-- Privacy-Preserving Content Shield -->
    <div class="card">
      <h2 class="card-title">Media & Content Shield</h2>
      <p>Raw message and media content is not retained. Review the scores and reasons above.</p>
    </div>
  </div>

  <!-- Right Sidebar: Reversible Moderation Actions -->
  <div>
    <div class="card" style="border: 2px solid var(--md-sys-color-primary-container);">
      <h2 class="card-title" style="color: var(--md-sys-color-primary);">Moderator Adjudication</h2>

      <form id="review-form" hx-post="/dashboard/alerts/{}/review" hx-swap="outerHTML">
        <div class="form-group">
          <label class="form-label">Engine Recommended Action</label>
          <div style="font-weight: 700; font-size: 15px; color: var(--md-sys-color-primary);">{}</div>
        </div>

        <div class="form-group">
          <label class="form-label" for="decision-select">Decision</label>
          <select id="decision-select" name="decision" class="form-control" style="font-weight: 600;">
            <option value="confirm">Confirm Violation (Escalate/Action)</option>
            <option value="reject">Reject (False Positive / Benign)</option>
            <option value="escalate">Escalate to Senior Safety Team</option>
          </select>
        </div>

        <div class="form-group">
          <label class="form-label" for="mod-note">Investigation Note (Optional)</label>
          <textarea id="mod-note" name="note" class="form-control" rows="3" placeholder="Contextual rationale..."></textarea>
        </div>

        <button type="submit" class="btn btn-primary" style="width: 100%;">
          Submit Decision
        </button>
      </form>
    </div>

    <!-- Quick Navigation -->
    <div class="card">
      <h3 style="font-size: 14px; font-weight: 600; margin-bottom: 10px;">Investigation Tools</h3>
      <div style="display: flex; flex-direction: column; gap: 8px;">
        <a href="/dashboard/timelines" class="btn btn-secondary" style="font-size: 12px;">View Pair Timeline</a>
        <a href="/dashboard/evidence" class="btn btn-secondary" style="font-size: 12px;">Generate Evidence Report</a>
      </div>
    </div>
  </div>
</div>

"##,
        tier_badge_class,
        tier_str,
        simulated_badge,
        alert_id_str,
        alert.event_id,
        alert.latency_ms,
        alert.severity,
        degraded_notice,
        active_cards,
        inactive_tray,
        reasons_html,
        alert_id_str,
        action_str
    )
}
