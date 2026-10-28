#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension,
};
use hre_core::{
    Action, AnalyzeResponse, Backend, HealthResponse, ModelHealth, ReviewRequest, Signals, Tier,
};
use hre_dashboard::backend::{
    DashboardBackend, DashboardFuture, DashboardHealth, DashboardMetrics, DashboardState,
};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

struct TestBackend {
    alert: AnalyzeResponse,
    review_status: StatusCode,
    reviews: Arc<Mutex<Vec<(Uuid, ReviewRequest)>>>,
}

impl DashboardBackend for TestBackend {
    fn alerts(&self) -> DashboardFuture<'_, Vec<AnalyzeResponse>> {
        Box::pin(async { Ok(vec![self.alert.clone()]) })
    }

    fn health(&self) -> DashboardFuture<'_, DashboardHealth> {
        Box::pin(async {
            Ok(DashboardHealth {
                engine: HealthResponse {
                    status: "ok".into(),
                    version: "test".into(),
                    mock: false,
                    degraded: vec![],
                },
                models: vec![ModelHealth {
                    name: "text".into(),
                    version: Some("test-artifact".into()),
                    hash_verified: true,
                    backend: Backend::Heuristic,
                    degraded: vec!["text.inference_unavailable".into()],
                }],
            })
        })
    }

    fn metrics(&self) -> DashboardFuture<'_, Option<DashboardMetrics>> {
        Box::pin(async { Ok(None) })
    }

    fn review(&self, id: Uuid, request: ReviewRequest) -> DashboardFuture<'_, ()> {
        Box::pin(async move {
            if !self.review_status.is_success() {
                return Err(self.review_status);
            }
            self.reviews.lock().unwrap().push((id, request));
            Ok(())
        })
    }
}

fn backend(review_status: StatusCode) -> TestBackend {
    let id = Uuid::new_v4();
    TestBackend {
        alert: AnalyzeResponse {
            event_id: Uuid::new_v4(),
            severity: 47,
            tier: Tier::Review,
            reasons: vec![],
            signals: Signals::default(),
            recommended_action: Action::WarnSender,
            secondary_actions: vec![],
            alert_id: Some(id),
            latency_ms: 123,
            model_versions: Default::default(),
            simulated: false,
            degraded: vec![],
            nudge: None,
        },
        review_status,
        reviews: Arc::new(Mutex::new(Vec::new())),
    }
}

async fn body(response: axum::response::Response) -> String {
    String::from_utf8(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap()
}

#[tokio::test]
async fn detail_uses_persisted_assessment_instead_of_fabricating_a_fixture() {
    let backend = backend(StatusCode::OK);
    let id = backend.alert.alert_id.unwrap();
    let app = hre_dashboard::router().layer(Extension(DashboardState(Arc::new(backend))));
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/dashboard/alerts/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let html = body(response).await;
    assert!(html.contains("47/100") && html.contains("123ms"));
    assert!(!html.contains("Live Model Inference"));
}

#[tokio::test]
async fn unknown_valid_alert_id_returns_not_found() {
    let app =
        hre_dashboard::router().layer(Extension(DashboardState(Arc::new(backend(StatusCode::OK)))));
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/dashboard/alerts/{}", Uuid::new_v4()))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn absent_bridge_returns_unavailable_instead_of_empty_or_fabricated_data() {
    for path in [
        "/dashboard",
        "/dashboard/queue",
        "/dashboard/queue/items",
        "/dashboard/metrics",
    ] {
        let response = hre_dashboard::router()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE, "{path}");
    }
}

#[tokio::test]
async fn failed_review_never_acknowledges_persistence() {
    let backend = backend(StatusCode::NOT_IMPLEMENTED);
    let id = backend.alert.alert_id.unwrap();
    let app = hre_dashboard::router().layer(Extension(DashboardState(Arc::new(backend))));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/dashboard/alerts/{id}/review"))
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("decision=confirm&note=test"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    assert!(!body(response).await.contains("Decision recorded"));
}

#[tokio::test]
async fn review_passes_typed_decision_and_note_to_persistence() {
    let backend = backend(StatusCode::OK);
    let id = backend.alert.alert_id.unwrap();
    let reviews = Arc::clone(&backend.reviews);
    let app = hre_dashboard::router().layer(Extension(DashboardState(Arc::new(backend))));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/dashboard/alerts/{id}/review"))
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "decision=reject&note=%3Cscript%3Etest%3C%2Fscript%3E",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(!body(response).await.contains("<script>"));
    let records = reviews.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].0, id);
    assert_eq!(
        serde_json::to_value(&records[0].1).unwrap(),
        serde_json::json!({"decision":"reject", "note":"<script>test</script>"})
    );
}

#[tokio::test]
async fn overview_shows_runtime_heuristics_and_integrity_as_separate_facts() {
    let app =
        hre_dashboard::router().layer(Extension(DashboardState(Arc::new(backend(StatusCode::OK)))));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/dashboard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let html = body(response).await;
    assert!(
        html.contains("Heuristic")
            && html.contains("Verified")
            && html.contains("text.inference_unavailable")
    );
    assert!(!html.contains("1,482") && !html.contains("0.933"));
}

#[test]
fn unavailable_metrics_do_not_show_benchmark_fixtures() {
    let html = hre_dashboard::views::metrics::render_metrics(None);
    assert!(html.contains("No measured evaluation report"));
    assert!(!html.contains("93.8%") && !html.contains("500 Curated"));
}

#[test]
fn measured_metrics_escape_source_and_disclose_synthetic_data() {
    let metrics = DashboardMetrics {
        source: "<script>source</script>".into(),
        samples: 17,
        simulated: true,
        precision: Some(0.75),
        recall: Some(f64::NAN),
        ..Default::default()
    };
    let html = hre_dashboard::views::metrics::render_metrics(Some(&metrics));
    assert!(html.contains("75.0%") && html.contains("17 samples") && html.contains("Unavailable"));
    assert!(html.contains("Simulated evaluation data") && !html.contains("<script>"));
}

#[test]
fn queue_deduplicates_persisted_events_and_escapes_explanations() {
    let mut alert = backend(StatusCode::OK).alert;
    alert.reasons.push(hre_core::Reason {
        code: "test".into(),
        text: "<script>alert(1)</script>".into(),
        weight: 0.5,
        evidence: Default::default(),
    });
    let html = hre_dashboard::views::queue::render_queue_items(&[alert.clone(), alert]);
    assert_eq!(html.matches("data-event-id=").count(), 1);
    assert!(!html.contains("<script>"));
}

#[test]
fn session_shell_contains_logout_and_does_not_read_or_write_moderator_tokens() {
    let html = hre_dashboard::views::base::render_page("Test", "queue", "");
    assert!(html.contains("method=\"post\" action=\"/dashboard/logout\""));
    assert!(
        !html.contains("getItem('adapt_mod_token')") && !html.contains("setItem('adapt_mod_token'")
    );
}
