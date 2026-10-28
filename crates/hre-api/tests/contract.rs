#![allow(clippy::unwrap_used)]
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use hre_core::{EngineConfig, Tier};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

fn config() -> EngineConfig {
    EngineConfig {
        salt: "01234567890123456789012345678901".into(),
        api_keys: vec!["test-key".into()],
        moderator_tokens: vec!["test-moderator".into()],
        database_url: "sqlite::memory:".into(),
        ..EngineConfig::default()
    }
}
fn event(sender: &str) -> Value {
    json!({"event_id":uuid::Uuid::new_v4(),"kind":"message","occurred_at":"2026-10-28T12:00:00Z","sender_id":sender,"recipient_id":"target","platform":"demo-chat","channel":{"type":"dm","conversation_id":"test"},"message":{"text":"Hello"}})
}
async fn post(app: Router, body: &Value, key: Option<&str>) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/v1/analyze")
        .header("content-type", "application/json");
    if let Some(key) = key {
        builder = builder.header("x-api-key", key);
    }
    let response = app
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}
#[tokio::test]
async fn auth_and_invalid_envelope_return_problem_json() {
    let app = hre_api::router(config(), true).await.unwrap();
    let (status, problem) = post(app.clone(), &event("alice"), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(problem["status"], 401);
    let mut invalid = event("alice");
    invalid["call"] = json!({"direction":"outgoing","duration_s":0,"answered":false});
    assert_eq!(
        post(app, &invalid, Some("test-key")).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}
#[tokio::test]
async fn all_nine_mock_fixtures_deserialize_into_the_contract() {
    let app = hre_api::router(config(), true).await.unwrap();
    for (tag, tier) in [
        ("polite_stalker", Tier::Review),
        ("pile_on", Tier::High),
        ("cyberflasher", Tier::High),
        ("screenshot", Tier::High),
        ("voice_note", Tier::Urgent),
        ("call_log", Tier::High),
        ("heavy_benign", Tier::Low),
        ("slang", Tier::Review),
        ("evidence", Tier::Review),
    ] {
        let (status, value) = post(
            app.clone(),
            &event(&format!("mock:{tag}")),
            Some("test-key"),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{tag}: {value}");
        let response: hre_core::AnalyzeResponse = serde_json::from_value(value).unwrap();
        assert_eq!(response.tier, tier, "{tag}");
        assert!(response.simulated);
        assert!(response.alert_id.is_none());
    }
}
#[tokio::test]
async fn real_retries_are_cached_and_conflicting_payloads_rejected() {
    let app = hre_api::router(config(), false).await.unwrap();
    let mut request = event("alice");
    let (status, first) = post(app.clone(), &request, Some("test-key")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(post(app.clone(), &request, Some("test-key")).await.1, first);
    request["message"]["text"] = json!("Changed");
    assert_eq!(
        post(app, &request, Some("test-key")).await.0,
        StatusCode::CONFLICT
    );
}
#[tokio::test]
async fn draft_never_consumes_idempotency_key_or_creates_alert() {
    let app = hre_api::router(config(), false).await.unwrap();
    let mut request = event("alice");
    request["draft"] = json!(true);
    let (_, draft) = post(app.clone(), &request, Some("test-key")).await;
    assert!(draft["alert_id"].is_null());
    request["draft"] = json!(false);
    request["message"]["text"] = json!("A different final message");
    assert_eq!(
        post(app, &request, Some("test-key")).await.0,
        StatusCode::OK
    );
}
#[test]
fn generated_contract_contains_payloads_auth_and_signal_statuses() {
    let value = serde_json::to_value(hre_api::openapi()).unwrap();
    assert!(value["paths"]["/v1/analyze"]["post"].is_object());
    assert_eq!(
        value["components"]["securitySchemes"]["api_key"]["name"],
        "X-API-Key"
    );
    assert!(value["components"]["schemas"]["AnalyzeResponse"]["properties"]["signals"].is_object());
}
