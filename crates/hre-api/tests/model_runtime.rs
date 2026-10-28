//! Opt-in local-artifact smoke test: `just integration-smoke`.
#![allow(clippy::unwrap_used)]
use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
#[ignore = "requires transferred trained models and ONNX Runtime"]
async fn trained_models_produce_live_draft_results_without_alerts() {
    let config = hre_core::EngineConfig {
        salt: "local-smoke-salt-never-used-in-production".into(),
        api_keys: vec!["smoke-client".into()],
        moderator_tokens: vec!["smoke-moderator".into()],
        database_url: "sqlite::memory:".into(),
        models_dir: format!("{}/../../models", env!("CARGO_MANIFEST_DIR")),
        ..Default::default()
    };
    let app = hre_api::router(config, false).await.unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::get("/v1/health/models")
                .header("x-api-key", "smoke-client")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let health: Vec<hre_core::ModelHealth> = serde_json::from_slice(&bytes).unwrap();
    for name in ["text", "behavior_msg", "behavior_call"] {
        let model = health.iter().find(|model| model.name == name).unwrap();
        assert!(model.hash_verified, "{name}: {:?}", model.degraded);
        assert_eq!(
            model.backend,
            hre_core::Backend::Onnx,
            "{name}: {:?}",
            model.degraded
        );
    }
    for text in ["Hello, how was your day?", "I will kill you tonight."] {
        let payload = serde_json::json!({"event_id":uuid::Uuid::new_v4(),"kind":"message","occurred_at":chrono::Utc::now(),"sender_id":"smoke-sender","recipient_id":"smoke-recipient","platform":"demo-chat","draft":true,"channel":{"type":"dm","conversation_id":"smoke"},"message":{"text":text}});
        let response = app
            .clone()
            .oneshot(
                Request::post("/v1/analyze")
                    .header("x-api-key", "smoke-client")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let report: hre_core::AnalyzeResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(report.signals.text.backend, hre_core::Backend::Onnx);
        assert!(!report.simulated);
        assert!(report.alert_id.is_none());
        assert!(report.signals.text.score.is_finite());
    }
    let response = app
        .oneshot(
            Request::get("/v1/alerts")
                .header("authorization", "Bearer smoke-moderator")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let alerts: Vec<hre_core::AnalyzeResponse> = serde_json::from_slice(&bytes).unwrap();
    assert!(alerts.is_empty());
}
