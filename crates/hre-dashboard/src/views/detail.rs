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
        if signal.label.is_empty() {
            "none"
        } else {
            &signal.label
        },
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
        r#"<span class="badge badge-built" style="font-size: 13px;">Live Model Inference</span>"#
    };

    let degraded_notice = if !alert.degraded.is_empty() {
        format!(
            r#"<div style="margin-top: 12px; padding: 10px 14px; background: var(--hre-tier-high-bg); color: var(--hre-tier-high); border-radius: 8px; font-size: 13px;">
  <strong>Degraded Backends:</strong> {}
</div>"#,
            alert.degraded.join(", ")
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

    // 8 Modality Signal Cards
    let sigs = &alert.signals;
    let signals_html = format!(
        "{}{}{}{}{}{}{}{}",
        render_signal_card("Text NLP", &sigs.text),
        render_signal_card("Vision OCR", &sigs.ocr),
        render_signal_card("Image NSFW", &sigs.image),
        render_signal_card("Audio Tone/ASR", &sigs.audio),
        render_signal_card("Call Patterns", &sigs.call),
        render_signal_card("Behavioral Trajectory", &sigs.behavior),
        render_signal_card("Social Pile-On", &sigs.pile_on),
        render_signal_card("PII Leakage", &sigs.pii),
    );

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

<!-- Grid layout: Left 8-Signals & Attribution, Right Moderation Actions -->
<div style="display: grid; grid-template-columns: 2fr 1fr; gap: 24px; align-items: start;">
  <div>
    <!-- Modality Signals Grid -->
    <div class="card">
      <h2 class="card-title">8-Modality Risk Signals</h2>
      <div class="signal-grid">
        {}
      </div>
    </div>

    <!-- Attribution & Reasons -->
    <div class="card">
      <h2 class="card-title">Attribution & Reason Breakdown</h2>
      <div>
        {}
      </div>
    </div>

    <!-- Privacy-Preserving Media Preview -->
    <div class="card">
      <h2 class="card-title">Media & Content Shield</h2>
      <p style="font-size: 13px; color: var(--md-sys-color-on-surface-variant); margin-bottom: 12px;">
        To protect moderator wellbeing, explicit or harassment content is automatically blurred. Unblur only if necessary for adjudication.
      </p>
      <div class="media-preview-container">
        <div id="media-content" class="media-blur" style="background: #2b2b36; color: #fff; padding: 24px; text-align: center; font-size: 13px;">
          [Protected Harassment Payload — Click Below to Unblur]
        </div>
      </div>
      <button type="button" class="btn btn-secondary" style="margin-top: 10px;" onclick="toggleMediaBlur()">
        Toggle Blur Shield
      </button>
    </div>
  </div>

  <!-- Right Sidebar: Reversible Moderation Actions -->
  <div>
    <div class="card" style="border: 2px solid var(--md-sys-color-primary-container);">
      <h2 class="card-title" style="color: var(--md-sys-color-primary);">Human Moderation Action</h2>
      <p style="font-size: 13px; color: var(--md-sys-color-on-surface-variant); margin-bottom: 16px;">
        AI flags harassment risks and provides reasons; human moderators retain sole authority over user actions.
      </p>

      <form id="review-form" hx-post="/dashboard/alerts/{}/review" hx-swap="outerHTML">
        <div class="form-group">
          <label class="form-label">Recommended Engine Action</label>
          <div style="font-weight: 700; font-size: 15px; color: var(--md-sys-color-primary);">{}</div>
        </div>

        <div class="form-group">
          <label class="form-label" for="decision-select">Your Adjudication Decision</label>
          <select id="decision-select" name="decision" class="form-control" style="font-weight: 600;">
            <option value="confirm">Confirm Violation (Escalate/Action)</option>
            <option value="reject">Reject (False Positive / Benign)</option>
            <option value="escalate">Escalate to Senior Safety Team</option>
          </select>
        </div>

        <div class="form-group">
          <label class="form-label" for="mod-note">Moderator Investigation Note (Optional)</label>
          <textarea id="mod-note" name="note" class="form-control" rows="3" placeholder="Explain the rationale or contextual nuance..."></textarea>
        </div>

        <button type="submit" class="btn btn-primary" style="width: 100%;">
          Submit Reversible Decision
        </button>
      </form>
    </div>

    <!-- Quick Navigation -->
    <div class="card">
      <h3 style="font-size: 15px; margin-bottom: 12px;">Investigation Tools</h3>
      <div style="display: flex; flex-direction: column; gap: 8px;">
        <a href="/dashboard/timelines" class="btn btn-secondary" style="font-size: 13px;">View Pair Timeline</a>
        <a href="/dashboard/evidence" class="btn btn-secondary" style="font-size: 13px;">Generate Evidence Report</a>
      </div>
    </div>
  </div>
</div>

<script>
  function toggleMediaBlur() {{
    const el = document.getElementById('media-content');
    if (el) {{
      el.classList.toggle('media-unblurred');
    }}
  }}
</script>"##,
        tier_badge_class,
        tier_str,
        simulated_badge,
        alert_id_str,
        alert.event_id,
        alert.latency_ms,
        alert.severity,
        degraded_notice,
        signals_html,
        reasons_html,
        alert_id_str,
        action_str
    )
}
