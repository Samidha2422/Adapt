#![allow(clippy::unwrap_used)]
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hre_core::EngineConfig;
use http_body_util::BodyExt;
use tower::ServiceExt;

fn config() -> EngineConfig {
    EngineConfig {
        salt: "a".repeat(32),
        api_keys: vec!["client-key".into()],
        moderator_tokens: vec!["moderator-key".into()],
        ..EngineConfig::default()
    }
}
fn payload(sender: &str) -> String {
    serde_json::json!({"event_id":"927c1186-f53d-49fc-8247-2e3268bf89e4", "kind":"message", "occurred_at":"2025-01-01T00:00:00Z", "sender_id":sender, "recipient_id":"recipient", "platform":"demo", "channel":{"type":"dm","conversation_id":"one"}, "message":{"text":"hello"}}).to_string()
}
async fn request(
    mock: bool,
    key: Option<&str>,
    body: String,
) -> (StatusCode, serde_json::Value, String) {
    let app = hre_api::router(config(), mock).await.unwrap();
    let mut builder = Request::post("/v1/analyze").header("content-type", "application/json");
    if let Some(key) = key {
        builder = builder.header("x-api-key", key);
    }
    let response = app
        .oneshot(builder.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap(), content_type)
}
#[tokio::test]
async fn mock_analyze_checks_auth_and_returns_simulated_fixture() {
    for key in [None, Some("wrong"), Some("moderator-key")] {
        let (status, body, content_type) = request(true, key, payload("mock:pile_on")).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["status"], 401);
        assert_eq!(content_type, "application/problem+json");
    }
    let (status, body, _) = request(true, Some("client-key"), payload("mock:pile_on")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["simulated"], true);
    assert_eq!(body["reasons"][0]["code"], "PILE_ON");
    assert_eq!(body["signals"]["pile_on"]["status"], "simulated");
}
#[tokio::test]
async fn invalid_input_returns_problems() {
    let (status, _, _) = request(true, Some("client-key"), "{".into()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _, _) = request(true, Some("client-key"), payload("")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _, _) = request(true, Some("client-key"), " ".repeat(1_100_000)).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}
#[tokio::test]
async fn health_and_openapi_describe_the_routes() {
    let app = hre_api::router(config(), true).await.unwrap();
    let response = app
        .oneshot(Request::get("/v1/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let health: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(health["mock"], true);
    let spec = serde_json::to_value(hre_api::openapi()).unwrap();
    assert!(spec["paths"]["/v1/analyze"]["post"]["security"].is_array());
    assert_eq!(
        spec["components"]["securitySchemes"]["api_key"]["name"],
        "X-API-Key"
    );
}
