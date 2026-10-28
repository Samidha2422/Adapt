#![allow(clippy::unwrap_used)]
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use hre_core::EngineConfig;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

fn config() -> EngineConfig {
    EngineConfig {
        models_dir: "target/model-free-api-test-artifacts".into(),
        salt: "01234567890123456789012345678901".into(),
        api_keys: vec!["key".into()],
        moderator_tokens: vec!["mod".into()],
        database_url: "sqlite::memory:".into(),
        ..Default::default()
    }
}
fn envelope() -> Value {
    json!({"event_id":uuid::Uuid::new_v4(),"kind":"image","occurred_at":"2026-10-28T12:00:00Z",
        "sender_id":"private-sender","recipient_id":"private-recipient","platform":"demo-chat",
        "channel":{"type":"dm","conversation_id":"private-channel"},"media":{"mime":"image/png"}})
}
async fn upload(app: &Router, envelope: &Value, bytes: &[u8], mime: &str) -> (StatusCode, Value) {
    let mut body = format!("--hre-boundary\r\nContent-Disposition: form-data; name=\"request\"\r\n\r\n{envelope}\r\n--hre-boundary\r\nContent-Disposition: form-data; name=\"image\"; filename=\"../../ignored.png\"\r\nContent-Type: {mime}\r\n\r\n").into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\r\n--hre-boundary--\r\n");
    let response = app
        .clone()
        .oneshot(
            Request::post("/v1/analyze/image")
                .header("x-api-key", "key")
                .header("content-type", "multipart/form-data; boundary=hre-boundary")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap())
}
async fn report(app: &Router, event: &Value, reason: &str) -> StatusCode {
    app.clone()
        .oneshot(
            Request::post("/v1/reports")
                .header("x-api-key", "key")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"event_id":event["event_id"],"reason":reason}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn actual_image_bytes_are_hashed_and_caption_never_becomes_ocr() {
    use sha2::Digest;
    let app = hre_api::router(config(), false).await.unwrap();
    let mut event = envelope();
    event["media"]["caption"] = json!("I will kill you");
    let bytes = hre_vision::minimal_png(10, 10);
    let (status, result) = upload(&app, &event, &bytes, "image/png").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        result["signals"]["image"]["details"]["sha256"],
        format!("{:x}", sha2::Sha256::digest(&bytes))
    );
    assert_eq!(result["signals"]["ocr"]["score"], 0.0);
    assert_eq!(result["signals"]["image"]["score"], 0.0);
    assert_eq!(result["signals"]["image"]["status"], "roadmap");
    assert_eq!(result["signals"]["text"]["backend"], "heuristic");
    assert_eq!(result["tier"], "urgent");
    assert_eq!(result["simulated"], false);
}

#[tokio::test]
async fn multipart_retry_is_cached_and_changed_bytes_conflict() {
    let app = hre_api::router(config(), false).await.unwrap();
    let event = envelope();
    let bytes = hre_vision::minimal_png(10, 10);
    let first = upload(&app, &event, &bytes, "image/png").await;
    assert_eq!(first.0, StatusCode::OK);
    assert_eq!(upload(&app, &event, &bytes, "image/png").await, first);
    assert_eq!(
        upload(&app, &event, &hre_vision::minimal_png(20, 20), "image/png")
            .await
            .0,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn image_validation_errors_are_problems_and_do_not_consume_event_ids() {
    let app = hre_api::router(config(), false).await.unwrap();
    let mut event = envelope();
    let bytes = hre_vision::minimal_png(10, 10);
    event["kind"] = json!("audio");
    assert_eq!(
        upload(&app, &event, &bytes, "image/png").await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    event["kind"] = json!("image");
    for (bytes, mime) in [
        (b"garbage".as_slice(), "image/png"),
        (bytes.as_slice(), "image/jpeg"),
    ] {
        let (status, problem) = upload(&app, &event, bytes, mime).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(problem["status"], 422);
    }
    event["media"]["mime"] = json!("image/jpeg");
    assert_eq!(
        upload(&app, &event, &bytes, "image/jpeg").await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    event["media"]["mime"] = json!("image/png");
    assert_eq!(
        upload(&app, &event, &bytes, "image/png").await.0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn multipart_auth_and_body_limits_apply_before_analysis() {
    let app = hre_api::router(config(), false).await.unwrap();
    let unauthorized = app
        .clone()
        .oneshot(
            Request::post("/v1/analyze/image")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
    let malformed = app
        .clone()
        .oneshot(
            Request::post("/v1/analyze/image")
                .header("x-api-key", "key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    // The image route has its own limit, larger than the JSON route's one MiB.
    let mut bytes = hre_vision::minimal_png(10, 10);
    bytes.resize(1_100_000, 0);
    assert_eq!(
        upload(&app, &envelope(), &bytes, "image/png").await.0,
        StatusCode::OK
    );
    bytes.resize(hre_vision::MAX_IMAGE_SIZE + 1, 0);
    let (status, problem) = upload(&app, &envelope(), &bytes, "image/png").await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(problem["status"], 413);
}

#[tokio::test]
async fn message_text_and_pii_reach_fusion_without_exposing_bodies() {
    let app = hre_api::router(config(), false).await.unwrap();
    let mut event = envelope();
    event.as_object_mut().unwrap().remove("media");
    event["kind"] = json!("message");
    event["message"] = json!({"text":"I will kill you private-message-body +919876543210"});
    let response = app
        .clone()
        .oneshot(
            Request::post("/v1/analyze")
                .header("x-api-key", "key")
                .header("content-type", "application/json")
                .body(Body::from(event.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let result: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(result["tier"], "urgent");
    assert_eq!(result["signals"]["pii"]["details"]["is_doxxing"], true);
    assert!(!String::from_utf8_lossy(&bytes).contains("9876543210"));
    assert!(!String::from_utf8_lossy(&bytes).contains("private-message-body"));
    assert_eq!(
        report(&app, &event, "reported").await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn report_requires_persisted_image_and_only_matches_identical_bytes() {
    let app = hre_api::router(config(), false).await.unwrap();
    let mut event = envelope();
    let bytes = hre_vision::minimal_png(10, 10);
    assert_eq!(
        report(&app, &event, "reported").await,
        StatusCode::NOT_FOUND
    );
    event["draft"] = json!(true);
    let (status, draft) = upload(&app, &event, &bytes, "image/png").await;
    assert_eq!(status, StatusCode::OK);
    assert!(draft["alert_id"].is_null());
    assert_eq!(
        report(&app, &event, "reported").await,
        StatusCode::NOT_FOUND
    );
    event["draft"] = json!(false);
    assert_eq!(
        upload(&app, &event, &bytes, "image/png").await.0,
        StatusCode::OK
    );
    assert_eq!(
        report(&app, &event, " ").await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        report(&app, &event, "reported").await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        report(&app, &event, "reported").await,
        StatusCode::NO_CONTENT
    );
    event["event_id"] = json!(uuid::Uuid::new_v4());
    let (status, matched) = upload(&app, &event, &bytes, "image/png").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(matched["tier"], "urgent");
    assert_eq!(matched["simulated"], true);
    assert_eq!(matched["signals"]["image"]["score"], 0.0);
    assert!(matched["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["code"].as_str().unwrap().contains("REPORTED_HASH")));
    event["event_id"] = json!(uuid::Uuid::new_v4());
    let (_, different) = upload(&app, &event, &hre_vision::minimal_png(20, 20), "image/png").await;
    assert!(!different["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["code"].as_str().unwrap().contains("REPORTED_HASH")));
}

#[tokio::test]
async fn metadata_fallback_is_simulated_and_keeps_media_bodies_out_of_storage() {
    let path = std::env::temp_dir().join(format!("hre-media-{}.sqlite", uuid::Uuid::new_v4()));
    let mut config = config();
    config.database_url = format!("sqlite:{}", path.to_string_lossy().replace('\\', "/"));
    let app = hre_api::router(config.clone(), false).await.unwrap();
    let mut event = envelope();
    event["media"]["caption"] = json!("private-caption-body");
    let bytes = hre_vision::png_with_text(
        "Description",
        "I will kill you private-media-body +919876543210",
    );
    let (status, result) = upload(&app, &event, &bytes, "image/png").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["simulated"], true);
    assert_eq!(result["signals"]["ocr"]["status"], "simulated");
    assert_eq!(result["signals"]["pii"]["status"], "simulated");
    assert_eq!(result["recommended_action"], "blur_media");
    let store = hre_store::Store::connect(&config.database_url)
        .await
        .unwrap();
    let persisted = store
        .assessment(event["event_id"].as_str().unwrap())
        .await
        .unwrap()
        .unwrap();
    let records = format!(
        "{result}{persisted}{}",
        serde_json::to_string(&store.alerts().await.unwrap()).unwrap()
    );
    for secret in [
        "private-media-body",
        "private-caption-body",
        "9876543210",
        "private-sender",
        "private-recipient",
    ] {
        assert!(!records.contains(secret), "leaked {secret}");
    }
    drop(store);
    drop(app);
    let _ = std::fs::remove_file(path);
}

#[test]
fn generated_contract_describes_actual_multipart_upload() {
    let schema = serde_json::to_value(hre_api::openapi()).unwrap();
    assert!(
        schema["paths"]["/v1/analyze/image"]["post"]["requestBody"]["content"]
            ["multipart/form-data"]
            .is_object()
    );
}
