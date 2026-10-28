//! Runtime overview using the host engine's persisted alerts and model health.

use super::queue::html_escape;
use crate::{backend::DashboardHealth, svg};
use hre_core::{AnalyzeResponse, Backend, Tier};

/// Compatibility renderer without an injected engine.
pub fn render_overview_page() -> String {
    r#"<h1 class="page-title">Operations Overview</h1><div class="card">Engine unavailable.</div>"#
        .into()
}

pub fn render_overview(health: &DashboardHealth, alerts: &[AnalyzeResponse]) -> String {
    let mut counts = [0_u32; 4];
    for alert in alerts {
        let index = match alert.tier {
            Tier::Low => 0,
            Tier::Review => 1,
            Tier::High => 2,
            Tier::Urgent => 3,
        };
        counts[index] += 1;
    }
    let mut cards = String::new();
    for (label, value) in [
        ("Stored alerts", alerts.len().to_string()),
        ("Urgent alerts", counts[3].to_string()),
        ("High / Review", (counts[1] + counts[2]).to_string()),
        ("Engine", html_escape(&health.engine.status)),
    ] {
        cards.push_str(&format!(
            r#"<div class="card" style="margin-bottom:0"><div>{label}</div><div class="table-numbers" style="font-size:28px;font-weight:700">{value}</div></div>"#
        ));
    }
    let mut rows = String::new();
    for model in &health.models {
        let (backend, status, class) = match model.backend {
            Backend::Onnx => ("ONNX", "Built", "badge-built"),
            Backend::Ggml => ("GGML", "Built", "badge-built"),
            Backend::Heuristic => ("Heuristic", "Built", "badge-built"),
            Backend::Disabled => ("Disabled", "Roadmap", "badge-roadmap"),
        };
        // Mock engine results cannot demonstrate a runnable trained backend.
        let (status, class) = if health.engine.mock && model.backend != Backend::Disabled {
            ("Simulated", "badge-simulated")
        } else {
            (status, class)
        };
        rows.push_str(&format!(
            r#"<tr><td><strong>{}</strong></td><td>{backend}</td><td><span class="badge {class}">{status}</span></td><td>{}</td><td>{}</td><td>{}</td></tr>"#,
            html_escape(&model.name),
            html_escape(model.version.as_deref().unwrap_or("Unavailable")),
            if model.hash_verified { "Verified" } else { "Unverified" },
            html_escape(&model.degraded.join(", ")),
        ));
    }
    if rows.is_empty() {
        rows.push_str(r#"<tr><td colspan="6">Model availability unavailable.</td></tr>"#);
    }
    let simulated = health.engine.mock || alerts.iter().any(|alert| alert.simulated);
    let disclosure = if simulated {
        r#"<span class="badge badge-simulated">Contains simulated results</span>"#
    } else {
        ""
    };
    format!(
        r#"<div class="page-header"><h1 class="page-title">Operations Overview</h1><div><a href="/dashboard/queue" class="btn btn-primary">Live Queue &rarr;</a> {disclosure}</div></div>
<div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:16px;margin-bottom:24px">{cards}</div>
<div class="card"><h2 class="card-title">Stored Alert Tiers</h2>{}</div>
<div class="card"><h2 class="card-title">Model Availability</h2>
<p>Active backends reported by the engine. Artifact integrity does not establish inference.</p>
<div class="table-container"><table class="table"><thead><tr><th>Module</th><th>Backend</th><th>Status</th><th>Version</th><th>Artifact integrity</th><th>Degraded</th></tr></thead><tbody>{rows}</tbody></table></div>
<p>{}</p></div>
<a href="/dashboard/metrics" class="btn btn-secondary">Evaluation Metrics &rarr;</a>"#,
        svg::render_tier_distribution_svg(counts[0], counts[1], counts[2], counts[3]),
        html_escape(&health.engine.degraded.join(", ")),
    )
}
