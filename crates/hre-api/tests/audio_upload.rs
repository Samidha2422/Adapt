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
    json!({"event_id":uuid::Uuid::new_v4(),"kind":"audio","occurred_at":"2026-10-29T00:00:00Z",
        "sender_id":"private-sender","recipient_id":"private-recipient","platform":"demo-chat",
        "channel":{"type":"dm","conversation_id":"private-channel"},"media":{"mime":"audio/mpeg"}})
}

async fn upload(app: &Router, event: &Value, bytes: &[u8], mime: &str) -> (StatusCode, Value) {
    let mut body = format!("--audio-boundary\r\nContent-Disposition: form-data; name=\"request\"\r\n\r\n{event}\r\n--audio-boundary\r\nContent-Disposition: form-data; name=\"audio\"; filename=\"ignored.mp3\"\r\nContent-Type: {mime}\r\n\r\n").into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\r\n--audio-boundary--\r\n");
    let response = app
        .clone()
        .oneshot(
            Request::post("/v1/analyze/audio")
                .header("x-api-key", "key")
                .header(
                    "content-type",
                    "multipart/form-data; boundary=audio-boundary",
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn uploaded_audio_reaches_scoring_but_transcript_is_never_persisted() {
    let path = std::env::temp_dir().join(format!("hre-audio-{}.sqlite", uuid::Uuid::new_v4()));
    let mut cfg = config();
    cfg.database_url = format!("sqlite:{}", path.to_string_lossy().replace('\\', "/"));
    let app = hre_api::router(cfg.clone(), false).await.unwrap();
    let event = envelope();
    let bytes =
        hre_audio::minimal_mp3_with_comment("I will kill you private-audio-body +919876543210");
    let (status, response) = upload(&app, &event, &bytes, "audio/mpeg").await;
    assert_eq!(status, StatusCode::OK, "{response}");
    assert_eq!(response["tier"], "urgent");
    assert_eq!(response["simulated"], true);
    assert!(response["secondary_actions"]
        .as_array()
        .unwrap()
        .contains(&json!("show_transcript")));
    assert!(response["signals"]["audio"]["details"]["transcript"]
        .as_str()
        .unwrap()
        .contains("private-audio-body"));
    assert!(!response.to_string().contains("9876543210"));
    let alerts = app
        .clone()
        .oneshot(
            Request::get("/v1/alerts")
                .header("authorization", "Bearer mod")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let alert_bytes = alerts.into_body().collect().await.unwrap().to_bytes();
    let alerts = String::from_utf8_lossy(&alert_bytes);
    assert!(!alerts.contains("private-audio-body"));
    assert!(!alerts.contains("9876543210"));
    let store = hre_store::Store::connect(&cfg.database_url).await.unwrap();
    let saved = store
        .assessment(event["event_id"].as_str().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert!(!saved.to_string().contains("private-audio-body"));
    assert!(!saved.to_string().contains("9876543210"));
    assert!(saved["response"]["signals"]["audio"]["details"]
        .get("transcript")
        .is_none());
    let retry = upload(&app, &event, &bytes, "audio/mpeg").await;
    assert_eq!(retry.0, StatusCode::OK);
    assert_eq!(retry.1, response);
    drop(store);
    drop(app);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn audio_idempotency_includes_uploaded_bytes_and_drafts_create_no_alerts() {
    let app = hre_api::router(config(), false).await.unwrap();
    let mut event = envelope();
    let first = hre_audio::minimal_mp3_with_comment("Hello");
    assert_eq!(
        upload(&app, &event, &first, "audio/mpeg").await.0,
        StatusCode::OK
    );
    assert_eq!(
        upload(&app, &event, &first, "audio/mpeg").await.0,
        StatusCode::OK
    );
    assert_eq!(
        upload(
            &app,
            &event,
            &hre_audio::minimal_mp3_with_comment("Different"),
            "audio/mpeg"
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    event["event_id"] = json!(uuid::Uuid::new_v4());
    event["draft"] = json!(true);
    let (_, result) = upload(
        &app,
        &event,
        &hre_audio::minimal_mp3_with_comment("I will kill you"),
        "audio/mpeg",
    )
    .await;
    assert!(result["alert_id"].is_null());
    let alerts = app
        .oneshot(
            Request::get("/v1/alerts")
                .header("authorization", "Bearer mod")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = alerts.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), json!([]));
}

#[tokio::test]
async fn invalid_audio_or_mime_is_rejected_before_consuming_event_id() {
    let app = hre_api::router(config(), false).await.unwrap();
    let event = envelope();
    assert_eq!(
        upload(&app, &event, b"not-audio", "audio/mpeg").await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        upload(&app, &event, &hre_audio::minimal_wav(1), "audio/mpeg")
            .await
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        upload(
            &app,
            &event,
            &hre_audio::minimal_mp3_with_comment("hello"),
            "audio/mpeg"
        )
        .await
        .0,
        StatusCode::OK
    );
}

#[test]
fn contract_includes_audio_multipart() {
    let schema = serde_json::to_value(hre_api::openapi()).unwrap();
    assert!(
        schema["paths"]["/v1/analyze/audio"]["post"]["requestBody"]["content"]
            ["multipart/form-data"]
            .is_object()
    );
}

#[tokio::test]
async fn synthetic_source_is_labelled_without_inventing_a_transcript() {
    let app = hre_api::router(config(), false).await.unwrap();
    let mut event = envelope();
    event["media"]["mime"] = json!("audio/wav");
    event["context"] = json!({"simulated":true});
    let (status, result) = upload(&app, &event, &hre_audio::minimal_wav(1), "audio/wav").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["simulated"], true);
    assert_eq!(result["signals"]["audio"]["backend"], "disabled");
    assert_eq!(result["signals"]["audio"]["status"], "roadmap");
    assert_eq!(result["signals"]["audio"]["details"]["transcript"], "");
}
