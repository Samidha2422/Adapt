//! Adapt Moderator Dashboard (`hre-dashboard`).
//! Provides Material 3 Expressive (M3E) server-rendered moderator UI,
//! real-time SSE live queue triage, incident investigation, and evaluation metrics.

pub mod assets;
pub mod backend;
pub mod svg;
pub mod views;

use axum::{
    extract::{Form, Path},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Extension, Router,
};
use backend::DashboardState;
use hre_core::{ReviewDecision, ReviewRequest};
use serde::Deserialize;
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
        .route("/dashboard/queue/items", get(queue_items))
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

async fn overview_view(
    state: Option<Extension<DashboardState>>,
) -> Result<Html<String>, StatusCode> {
    let state = state.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let health = state.0.health().await?;
    let alerts = state.0.alerts().await?;
    let content = views::overview::render_overview(&health, &alerts);
    Ok(Html(views::base::render_page(
        "Overview", "overview", &content,
    )))
}

async fn queue_view(state: Option<Extension<DashboardState>>) -> Result<Html<String>, StatusCode> {
    let state = state.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let alerts = state.0.alerts().await?;
    let content = views::queue::render_queue_page(&alerts);
    Ok(Html(views::base::render_page(
        "Live Queue",
        "queue",
        &content,
    )))
}

async fn queue_items(state: Option<Extension<DashboardState>>) -> Result<Html<String>, StatusCode> {
    let state = state.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let alerts = state.0.alerts().await?;
    Ok(Html(views::queue::render_queue_items(&alerts)))
}

async fn alert_detail_view(
    Path(id): Path<String>,
    state: Option<Extension<DashboardState>>,
) -> Result<Html<String>, StatusCode> {
    let parsed_uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return Err(StatusCode::NOT_FOUND),
    };

    let state = state.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let alert = state
        .0
        .alerts()
        .await?
        .into_iter()
        .find(|alert| alert.alert_id == Some(parsed_uuid))
        .ok_or(StatusCode::NOT_FOUND)?;

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
    state: Option<Extension<DashboardState>>,
    Form(form): Form<ReviewForm>,
) -> Result<Response, StatusCode> {
    let parsed_uuid = match Uuid::parse_str(&id) {
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
        note: form.note,
    };

    let state = state.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    state.0.review(parsed_uuid, review_req).await?;
    let confirmation_html = r#"<div class="card" role="status">Decision recorded.</div>"#;

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

async fn metrics_view(
    state: Option<Extension<DashboardState>>,
) -> Result<Html<String>, StatusCode> {
    let state = state.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let metrics = state.0.metrics().await?;
    let content = views::metrics::render_metrics(metrics.as_ref());
    Ok(Html(views::base::render_page(
        "Evaluation Metrics",
        "metrics",
        &content,
    )))
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
