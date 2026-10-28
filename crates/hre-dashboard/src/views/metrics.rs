//! Measured evaluation results supplied by the host API.

use super::queue::html_escape;
use crate::{backend::DashboardMetrics, svg};

pub fn render_metrics_page() -> String {
    render_metrics(None)
}

fn number(value: Option<f64>) -> String {
    value
        .filter(|v| v.is_finite() && *v >= 0.0)
        .map_or_else(|| "Unavailable".into(), |v| format!("{v:.3}"))
}

fn ratio(value: Option<f64>) -> String {
    value
        .filter(|v| v.is_finite() && (0.0..=1.0).contains(v))
        .map_or_else(|| "Unavailable".into(), |v| format!("{:.1}%", v * 100.0))
}

pub fn render_metrics(metrics: Option<&DashboardMetrics>) -> String {
    let header = r#"<div class="page-header"><h1 class="page-title">Model Metrics &amp; Evaluation</h1></div>"#;
    let Some(metrics) = metrics else {
        return format!(
            r#"{header}<div class="card"><span class="badge badge-roadmap">Unavailable</span><p>No measured evaluation report is available.</p></div>"#
        );
    };
    let mut cards = String::new();
    for (label, value) in [
        ("Precision", ratio(metrics.precision)),
        ("Recall", ratio(metrics.recall)),
        ("F1", ratio(metrics.f1)),
        ("Detection rate", ratio(metrics.detection_rate)),
        (
            "Time to detection (events)",
            number(metrics.time_to_detection_events),
        ),
        ("Latency P50 (ms)", number(metrics.latency_p50_ms)),
        ("Latency P95 (ms)", number(metrics.latency_p95_ms)),
        ("Latency P99 (ms)", number(metrics.latency_p99_ms)),
    ] {
        cards.push_str(&format!(
            r#"<div class="card" style="margin-bottom:0"><div>{label}</div><div class="table-numbers" style="font-size:24px;font-weight:700">{value}</div></div>"#
        ));
    }
    let matrix = metrics.confusion.map_or_else(
        || r#"<div class="card">Confusion matrix unavailable.</div>"#.into(),
        |[tp, fp, fn_count, tn]| {
            format!(
                r#"<div class="card"><h2 class="card-title">Confusion Matrix</h2>{}</div>"#,
                svg::render_confusion_matrix_svg(tp, fp, fn_count, tn)
            )
        },
    );
    let disclosure = if metrics.simulated {
        r#"<span class="badge badge-simulated">Simulated evaluation data</span>"#
    } else {
        ""
    };
    format!(
        r#"{header}<div class="card">{} &middot; {} samples {disclosure}</div><div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:16px;margin-bottom:24px">{cards}</div>{matrix}"#,
        html_escape(&metrics.source),
        metrics.samples
    )
}
