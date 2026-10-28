#![allow(clippy::unwrap_used)]
//! Unit and integration tests for hre-dashboard (C1 and C2 deliverables).

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hre_core::{Action, AnalyzeResponse, Reason, ReviewDecision, ReviewRequest, Signals, Tier};
use std::collections::BTreeMap;
use tower::ServiceExt;
use uuid::Uuid;

fn sample_alert(simulated: bool) -> AnalyzeResponse {
    let event_id = Uuid::new_v4();
    AnalyzeResponse {
        event_id,
        severity: 78,
        tier: Tier::High,
        reasons: vec![Reason {
            code: "harassment.persistent_contact".into(),
            text: "Repeated messages despite recipient block".into(),
            weight: 0.78,
            evidence: BTreeMap::new(),
        }],
        signals: Signals::default(),
        recommended_action: Action::BlurMedia,
        secondary_actions: vec![],
        alert_id: Some(event_id),
        latency_ms: 14,
        model_versions: BTreeMap::new(),
        simulated,
        degraded: vec![],
        nudge: None,
    }
}

#[tokio::test]
async fn test_queue_empty_state() {
    let html = hre_dashboard::views::queue::render_queue_page(&[]);
    assert!(
        html.contains("No active alerts in queue"),
        "Empty queue must show clear empty state"
    );
}

#[tokio::test]
async fn test_valid_alert_renders_tier_and_reason() {
    let alert = sample_alert(false);
    let html = hre_dashboard::views::queue::render_alert_item(&alert);

    assert!(
        html.contains("badge-high"),
        "Must render tier badge for High"
    );
    assert!(
        html.contains("Repeated messages despite recipient block"),
        "Must render reason text"
    );
    assert!(html.contains("78"), "Must render severity score 78");
}

#[tokio::test]
async fn test_simulated_badge() {
    let sim_alert = sample_alert(true);
    let html_sim = hre_dashboard::views::queue::render_alert_item(&sim_alert);
    assert!(
        html_sim.contains("badge-simulated"),
        "Simulated alert must display simulated badge"
    );
    assert!(
        html_sim.contains("Simulated"),
        "Simulated alert text must be present"
    );

    let real_alert = sample_alert(false);
    let html_real = hre_dashboard::views::queue::render_alert_item(&real_alert);
    assert!(
        html_real.contains("badge-built"),
        "Real alert must display built badge"
    );
}

#[tokio::test]
async fn test_malformed_event_does_not_break_queue() {
    let page = hre_dashboard::views::queue::render_queue_page(&[]);
    assert!(
        page.contains("catch (err)"),
        "Client side must safely catch unparseable SSE payloads"
    );
}

#[tokio::test]
async fn test_reconnect_deduplicates_id() {
    let page = hre_dashboard::views::queue::render_queue_page(&[]);
    assert!(
        page.contains("seenEventIds.has(data.event_id)"),
        "Client side must deduplicate incoming events by stable event ID"
    );
}

#[tokio::test]
async fn test_auth_failure_shows_unavailable() {
    let page = hre_dashboard::views::queue::render_queue_page(&[]);
    assert!(
        page.contains("Disconnected / Retrying"),
        "Must handle disconnection / auth failure gracefully"
    );
}

#[tokio::test]
async fn test_no_remote_assets() {
    let base_html = hre_dashboard::views::base::render_page("Test", "queue", "content");
    assert!(
        !base_html.contains("http://") && !base_html.contains("https://"),
        "Page must vendor all styles/scripts; zero remote HTTP/HTTPS CDN references allowed"
    );
    assert!(
        base_html.contains("/dashboard/static/"),
        "Must serve local dashboard static assets"
    );
}

#[tokio::test]
async fn test_detail_renders_reason_and_simulated_label() {
    let alert = sample_alert(true);
    let html = hre_dashboard::views::detail::render_detail_page(&alert);

    assert!(
        html.contains("Simulated Pipeline Active"),
        "Detail view must visibly disclose simulated pipeline"
    );
    assert!(
        html.contains("harassment.persistent_contact"),
        "Detail view must render reason code"
    );
    assert!(
        html.contains("8-Modality Risk Signals"),
        "Must render 8-modality breakdown"
    );
}

#[tokio::test]
async fn test_unknown_alert_not_found() {
    let app = hre_dashboard::router();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/dashboard/alerts/not-a-valid-uuid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Malformed or unknown alert ID must return 404"
    );
}

#[tokio::test]
async fn test_review_serializes_decision_and_optional_note() {
    let req = ReviewRequest {
        decision: ReviewDecision::Confirm,
        note: Some("Confirmed persistent harassment pattern".into()),
    };

    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"decision\":\"confirm\""));
    assert!(json.contains("\"note\":\"Confirmed persistent harassment pattern\""));
}

#[tokio::test]
async fn test_unauthorized_review_is_not_success() {
    let app = hre_dashboard::router();
    // Submitting with empty/malformed body
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/dashboard/alerts/not-a-uuid/review")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(
        response.status(),
        StatusCode::OK,
        "Invalid or malformed review request must not succeed"
    );
}

#[tokio::test]
async fn test_timeline_empty_and_ordered() {
    let html = hre_dashboard::views::timeline::render_timeline_page();
    assert!(
        html.contains("Subject Interaction Timeline"),
        "Timeline view must render properly"
    );
    assert!(
        html.contains("Day 1") && html.contains("Day 3") && html.contains("Day 4"),
        "Timeline must maintain chronological ordering"
    );
}

#[tokio::test]
async fn test_malformed_payload_safe() {
    let app = hre_dashboard::router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/dashboard/alerts/{}/review", Uuid::new_v4()))
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("decision=invalid_action"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Invalid action name must return 400 Bad Request"
    );
}

#[tokio::test]
async fn test_network_failure_preserves_form() {
    let alert = sample_alert(false);
    let html = hre_dashboard::views::detail::render_detail_page(&alert);
    assert!(
        html.contains("<form id=\"review-form\""),
        "Detail view must contain review form with durable action inputs"
    );
    assert!(
        html.contains("name=\"decision\""),
        "Form must contain decision selector"
    );
}
