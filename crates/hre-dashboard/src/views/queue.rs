//! Live Alert Queue View (C1 deliverable) with real-time SSE, deduplication, and filtering.

use hre_core::{Action, AnalyzeResponse, Tier};

/// Renders a single alert row/card for the live queue.
pub fn render_alert_item(alert: &AnalyzeResponse) -> String {
    let (tier_str, tier_badge_class) = match alert.tier {
        Tier::Urgent => ("urgent", "badge-urgent"),
        Tier::High => ("high", "badge-high"),
        Tier::Review => ("review", "badge-review"),
        Tier::Low => ("low", "badge-low"),
    };

    let simulated_chip = if alert.simulated {
        r#"<span class="badge badge-simulated" title="Simulated: Mock or demo override active">Simulated</span>"#
    } else {
        r#"<span class="badge badge-built" title="Production AI / Validated Model">Built</span>"#
    };

    let reasons_summary = if alert.reasons.is_empty() {
        "No explicit harassment reasons detected.".to_string()
    } else {
        alert
            .reasons
            .iter()
            .map(|r| format!("{}: {}", r.code, r.text))
            .collect::<Vec<_>>()
            .join(" | ")
    };

    let alert_id_display = alert
        .alert_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| alert.event_id.to_string());
    let short_id = if alert_id_display.len() >= 8 {
        &alert_id_display[..8]
    } else {
        &alert_id_display
    };

    let detail_link = format!("/dashboard/alerts/{}", alert_id_display);
    let action_str = match alert.recommended_action {
        Action::Log => "log",
        Action::WarnSender => "warn_sender",
        Action::BlurMedia => "blur_media",
        Action::Throttle => "throttle",
        Action::Escalate => "escalate",
    };

    format!(
        r##"<div class="alert-row alert-row-enter" data-event-id="{}" id="alert-item-{}">
  <div>
    <span class="table-numbers" style="font-weight: 700; font-size: 18px; color: var(--md-sys-color-primary);">{}</span>
    <div style="font-size: 11px; color: var(--md-sys-color-on-surface-variant);">/100</div>
  </div>

  <div>
    <span class="badge {}">{}</span>
    <div style="margin-top: 4px;">{}</div>
  </div>

  <div>
    <span class="badge" style="background: var(--md-sys-color-surface-variant); color: var(--md-sys-color-on-surface-variant);">{}</span>
    <div style="font-size: 11px; margin-top: 4px;" class="table-numbers">{}ms</div>
  </div>

  <div>
    <div style="font-weight: 600; font-size: 14px; margin-bottom: 2px;">
      <a href="{}" style="color: var(--md-sys-color-primary); text-decoration: none;">Alert #{}</a>
    </div>
    <div style="font-size: 12px; color: var(--md-sys-color-on-surface-variant); line-height: 1.4;">
      {}
    </div>
  </div>

  <div style="font-size: 12px; color: var(--md-sys-color-on-surface-variant);">
    Action: <strong>{}</strong>
  </div>

  <div style="text-align: right;">
    <a href="{}" class="btn btn-secondary" style="font-size: 12px; padding: 6px 12px;">Investigate</a>
  </div>
</div>"##,
        alert.event_id,
        alert.event_id,
        alert.severity,
        tier_badge_class,
        tier_str,
        simulated_chip,
        action_str,
        alert.latency_ms,
        detail_link,
        short_id,
        html_escape(&reasons_summary),
        action_str,
        detail_link
    )
}

/// Helper function to escape HTML safely without external dependencies.
pub fn html_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Renders the complete Live Alert Queue page shell.
pub fn render_queue_page(initial_alerts: &[AnalyzeResponse]) -> String {
    let mut alerts_html = String::new();
    if initial_alerts.is_empty() {
        alerts_html.push_str(
            r#"<div id="queue-empty-state" style="padding: 48px 24px; text-align: center; color: var(--md-sys-color-on-surface-variant);">
  <svg style="width: 48px; height: 48px; fill: var(--md-sys-color-outline-variant); margin-bottom: 12px;" viewBox="0 0 24 24">
    <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z"/>
  </svg>
  <div style="font-size: 16px; font-weight: 500;">No active alerts in queue</div>
  <div style="font-size: 13px; margin-top: 4px;">Incoming alerts from the live SSE feed will appear here in real time.</div>
</div>"#,
        );
    } else {
        for alert in initial_alerts {
            alerts_html.push_str(&render_alert_item(alert));
        }
    }

    format!(
        r##"<div class="page-header">
  <div>
    <h1 class="page-title">Live Alert Triage Queue</h1>
    <p class="page-subtitle">Real-time multimodal harassment incident stream with automated deduplication and explainable severity scores.</p>
  </div>

  <div style="display: flex; gap: 12px; align-items: center;">
    <button type="button" class="btn btn-secondary" onclick="filterQueue('all')">All</button>
    <button type="button" class="btn btn-secondary" onclick="filterQueue('urgent')"><span class="badge badge-urgent" style="padding: 2px 6px; font-size: 11px;">Urgent</span></button>
    <button type="button" class="btn btn-secondary" onclick="filterQueue('high')"><span class="badge badge-high" style="padding: 2px 6px; font-size: 11px;">High</span></button>
    <button type="button" class="btn btn-secondary" onclick="filterQueue('review')"><span class="badge badge-review" style="padding: 2px 6px; font-size: 11px;">Review</span></button>
  </div>
</div>

<!-- SSE Connection Container with HTMX -->
<div class="card" style="padding: 0; overflow: hidden;">
  <div style="padding: 14px 20px; background: var(--md-sys-color-surface-container); border-bottom: 1px solid var(--md-sys-color-outline-variant); display: flex; justify-content: space-between; align-items: center;">
    <div style="display: flex; align-items: center; gap: 12px;">
      <span style="font-weight: 600; font-size: 14px;">Incoming Incident Stream</span>
      <span class="badge badge-built" id="sse-badge">SSE Connected</span>
    </div>
    <div style="font-size: 12px; color: var(--md-sys-color-on-surface-variant);">
      Listening on <code>/v1/stream/alerts</code>
    </div>
  </div>

  <!-- Real-time feed element -->
  <div id="alerts-feed"
       hx-ext="sse"
       sse-connect="/v1/stream/alerts"
       sse-swap="alert"
       hx-swap="afterbegin"
       style="min-height: 200px;">
    {}
  </div>
</div>

<script>
  // Client-side deduplication and safe reconnect handling
  const seenEventIds = new Set();
  document.querySelectorAll('[data-event-id]').forEach(el => {{
    seenEventIds.add(el.getAttribute('data-event-id'));
  }});

  document.body.addEventListener('htmx:sseBeforeMessage', function(e) {{
    try {{
      const data = JSON.parse(e.detail.data);
      if (data && data.event_id) {{
        if (seenEventIds.has(data.event_id)) {{
          e.preventDefault(); // Prevent duplicate insertion
          return;
        }}
        seenEventIds.add(data.event_id);
        const emptyState = document.getElementById('queue-empty-state');
        if (emptyState) emptyState.remove();
      }}
    }} catch (err) {{
      // Malformed event handling: do not break the page
      console.warn('Received unparseable SSE message', err);
    }}
  }});

  document.body.addEventListener('htmx:sseError', function() {{
    const badge = document.getElementById('sse-badge');
    if (badge) {{
      badge.className = 'badge badge-roadmap';
      badge.textContent = 'Disconnected / Retrying';
    }}
  }});

  function filterQueue(tier) {{
    document.querySelectorAll('.alert-row').forEach(row => {{
      if (tier === 'all') {{
        row.style.display = 'grid';
      }} else {{
        const text = row.innerText.toLowerCase();
        row.style.display = text.includes(tier) ? 'grid' : 'none';
      }}
    }});
  }}
</script>"##,
        alerts_html
    )
}
