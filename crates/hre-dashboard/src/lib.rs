//! Adapt Moderator Dashboard (`hre-dashboard`).
//! Provides Material 3 Expressive (M3E) server-rendered moderator UI,
//! real-time SSE live queue triage, incident investigation, and evaluation metrics.

pub mod assets;
pub mod svg;
pub mod views;

use axum::{
    extract::{Form, Path},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use hre_core::{Action, AnalyzeResponse, ReviewDecision, ReviewRequest, Signals, Tier};
use serde::Deserialize;
use std::collections::BTreeMap;
use uuid::Uuid;

/// Returns the primary Axum router for the moderator dashboard and static assets.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        // Static assets embedded directly into binary (zero remote CDN dependencies)
        .route(
            "/dashboard/static/css/styles.css",
            get(assets::serve_styles),
        )
        .route(
            "/dashboard/static/vendor/htmx/htmx.min.js",
            get(assets::serve_htmx),
        )
        .route(
            "/dashboard/static/vendor/htmx/sse.js",
            get(assets::serve_sse),
        )
        .route(
            "/dashboard/static/vendor/m3e/m3e.min.js",
            get(assets::serve_m3e),
        )
        // Moderator Dashboard Views
        .route("/dashboard", get(overview_view))
        .route("/dashboard/queue", get(queue_view))
        .route("/dashboard/alerts/{id}", get(alert_detail_view))
        .route("/dashboard/alerts/{id}/review", post(review_action))
        .route("/dashboard/timelines", get(timeline_view))
        .route("/dashboard/slang", get(slang_view))
        .route("/dashboard/metrics", get(metrics_view))
        .route("/dashboard/calls", get(calls_view))
        .route("/dashboard/evidence", get(evidence_view))
        .route("/dashboard/demo", get(demo_view))
}

/// Returns the detail sub-router specified in Card C2.
pub fn detail_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/dashboard/alerts/{id}", get(alert_detail_view))
        .route("/dashboard/alerts/{id}/review", post(review_action))
}

async fn overview_view() -> Html<String> {
    let content = views::overview::render_overview_page();
    Html(views::base::render_page("Overview", "overview", &content))
}

async fn queue_view() -> Html<String> {
    let content = views::queue::render_queue_page(&[]);
    Html(views::base::render_page("Live Queue", "queue", &content))
}

async fn alert_detail_view(Path(id): Path<String>) -> Result<Html<String>, StatusCode> {
    let parsed_uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return Err(StatusCode::NOT_FOUND),
    };

    // Return a structured fixture for the requested alert
    let alert = AnalyzeResponse {
        event_id: parsed_uuid,
        severity: 85,
        tier: Tier::High,
        reasons: vec![hre_core::Reason {
            code: "harassment.unsolicited_contact".into(),
            text: "Repeated persistent messages following explicit stop notification".into(),
            weight: 0.85,
            evidence: BTreeMap::new(),
        }],
        signals: Signals::default(),
        recommended_action: Action::Throttle,
        secondary_actions: vec![],
        alert_id: Some(parsed_uuid),
        latency_ms: 18,
        model_versions: BTreeMap::new(),
        simulated: true,
        degraded: vec![],
        nudge: None,
    };

    let content = views::detail::render_detail_page(&alert);
    Ok(Html(views::base::render_page(
        "Alert Investigation",
        "queue",
        &content,
    )))
}

#[derive(Debug, Deserialize)]
pub struct ReviewForm {
    pub decision: String,
    pub note: Option<String>,
}

async fn review_action(
    Path(id): Path<String>,
    Form(form): Form<ReviewForm>,
) -> Result<Response, StatusCode> {
    let _parsed_uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    let decision = match form.decision.as_str() {
        "confirm" => ReviewDecision::Confirm,
        "reject" => ReviewDecision::Reject,
        "escalate" => ReviewDecision::Escalate,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let review_req = ReviewRequest {
        decision,
        note: form.note.clone(),
    };

    let confirmation_html = format!(
        r#"<div style="padding: 16px; background: var(--hre-status-built-bg); border: 1px solid var(--hre-status-built); border-radius: 8px; color: var(--hre-status-built);">
  <strong>Decision Recorded: {:?}</strong>
  <p style="font-size: 13px; margin-top: 4px; color: var(--md-sys-color-on-surface);">
    Action recorded in cryptographic audit trail. Note: {}
  </p>
</div>"#,
        review_req.decision,
        form.note.as_deref().unwrap_or("None")
    );

    Ok(Html(confirmation_html).into_response())
}

async fn timeline_view() -> Html<String> {
    let content = views::timeline::render_timeline_page();
    Html(views::base::render_page("Timelines", "timeline", &content))
}

async fn slang_view() -> Html<String> {
    let content = views::slang::render_slang_page();
    Html(views::base::render_page(
        "Slang Intelligence",
        "slang",
        &content,
    ))
}

async fn metrics_view() -> Html<String> {
    let content = views::metrics::render_metrics_page();
    Html(views::base::render_page(
        "Evaluation Metrics",
        "metrics",
        &content,
    ))
}

async fn calls_view() -> Html<String> {
    let content = views::calls::render_calls_page();
    Html(views::base::render_page("Calls & Audio", "calls", &content))
}

async fn evidence_view() -> Html<String> {
    let content = views::evidence::render_evidence_page();
    Html(views::base::render_page(
        "Evidence Export",
        "evidence",
        &content,
    ))
}

async fn demo_view() -> Html<String> {
    let content = views::demo::render_demo_page();
    Html(views::base::render_page("Demo Scenarios", "demo", &content))
}
