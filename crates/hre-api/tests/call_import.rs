#![allow(clippy::unwrap_used)]
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use hre_core::{EngineConfig, Pseudonymizer};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

fn config() -> EngineConfig {
    EngineConfig {
        salt: "01234567890123456789012345678901".into(),
        api_keys: vec!["key".into()],
        moderator_tokens: vec!["mod".into()],
        database_url: "sqlite::memory:".into(),
        ..Default::default()
    }
}
fn row(id: uuid::Uuid, duration: u32, simulated: bool) -> String {
    format!("{id},2026-10-28T12:00:00Z,private-caller,private-callee,{duration},false,false,true,{simulated}\n")
}
fn payload(rows: &str, draft: bool) -> Value {
    json!({"platform":"demo-chat","conversation_id":"private-channel","draft":draft,
        "csv":format!("{}\n{rows}", hre_calls::CSV_COLUMNS.join(","))})
}
async fn import(app: &Router, value: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::post("/v1/calllogs/import")
                .header("content-type", "application/json")
                .header("x-api-key", "key")
                .body(Body::from(value.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}
async fn persistent_app() -> (Router, hre_store::Store, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!("hre-calls-{}.sqlite", uuid::Uuid::new_v4()));
    let mut config = config();
    config.database_url = format!("sqlite:{}", path.to_string_lossy().replace('\\', "/"));
    let app = hre_api::router(config.clone(), false).await.unwrap();
    let store = hre_store::Store::connect(&config.database_url)
        .await
        .unwrap();
    (app, store, path)
}
async fn stored_calls(store: &hre_store::Store) -> Vec<hre_store::StoredEvent> {
    let ids = Pseudonymizer::new(config().salt.as_bytes()).unwrap();
    let caller = ids.subject("demo-chat", "private-caller").unwrap();
    let callee = ids.subject("demo-chat", "private-callee").unwrap();
    store
        .pair_window(&caller.0, &callee.0, i64::MIN, i64::MAX)
        .await
        .unwrap()
}

#[tokio::test]
async fn malformed_last_csv_row_has_no_event_assessment_or_alert_side_effects() {
    let (app, store, path) = persistent_app().await;
    let id = uuid::Uuid::new_v4();
    let csv = format!("{}invalid-last-row\n", row(id, 0, true));
    let (status, problem) = import(&app, payload(&csv, false)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(problem["status"], 422);
    assert!(stored_calls(&store).await.is_empty());
    assert!(store.assessment(&id.to_string()).await.unwrap().is_none());
    assert!(store.alerts().await.unwrap().is_empty());
    assert!(!problem.to_string().contains("private-caller"));
    drop(store);
    drop(app);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn imports_use_existing_atomic_assessments_and_preserve_simulation_and_hmac_scope() {
    let (app, store, path) = persistent_app().await;
    let id = uuid::Uuid::new_v4();
    let input = payload(&row(id, 0, true), false);
    let (status, result) = import(&app, input.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["atomic"], false);
    assert_eq!(result["rows"][0]["row"], 2);
    assert_eq!(result["rows"][0]["outcome"], "imported");
    let assessment = &result["rows"][0]["assessment"];
    assert_eq!(assessment["simulated"], true);
    assert_eq!(assessment["signals"]["behavior"]["status"], "simulated");
    assert_eq!(assessment["signals"]["call"]["backend"], "heuristic");
    assert_eq!(assessment["signals"]["call"]["status"], "simulated");
    let retry = import(&app, input).await.1;
    assert_eq!(retry["rows"][0]["outcome"], "cached");
    assert_eq!(retry["rows"][0]["assessment"], *assessment);
    let events = stored_calls(&store).await;
    assert_eq!(events.len(), 1);
    assert!(events[0].simulated);
    assert_eq!(events[0].kind, hre_store::EventKind::Call);
    assert_eq!(store.alerts().await.unwrap().len(), 1);
    let serialized = format!(
        "{result}{}{}",
        serde_json::to_string(&events).unwrap(),
        store.assessment(&id.to_string()).await.unwrap().unwrap()
    );
    for secret in [
        "private-caller",
        "private-callee",
        "private-channel",
        "caller_id,callee_id",
    ] {
        assert!(!serialized.contains(secret));
    }
    drop(store);
    drop(app);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn conflicts_are_explicit_per_row_and_do_not_rollback_or_stop_other_rows() {
    let (app, store, path) = persistent_app().await;
    let existing = uuid::Uuid::new_v4();
    assert_eq!(
        import(&app, payload(&row(existing, 0, false), false))
            .await
            .0,
        StatusCode::OK
    );
    let before = uuid::Uuid::new_v4();
    let after = uuid::Uuid::new_v4();
    let csv = format!(
        "{}{}{}",
        row(before, 0, false),
        row(existing, 60, false),
        row(after, 0, true)
    );
    let (status, result) = import(&app, payload(&csv, false)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["rows"][0]["outcome"], "imported");
    assert_eq!(result["rows"][1]["outcome"], "failed");
    assert_eq!(result["rows"][1]["problem"]["status"], 409);
    assert!(result["rows"][1].get("assessment").is_none());
    assert_eq!(result["rows"][2]["outcome"], "imported");
    assert_eq!(stored_calls(&store).await.len(), 3);
    for id in [before, after] {
        assert!(store.assessment(&id.to_string()).await.unwrap().is_some());
    }
    drop(store);
    drop(app);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn call_import_drafts_never_persist_or_reserve_event_ids() {
    let (app, store, path) = persistent_app().await;
    let id = uuid::Uuid::new_v4();
    let (status, preview) = import(&app, payload(&row(id, 0, true), true)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preview["rows"][0]["outcome"], "draft");
    assert!(preview["rows"][0]["assessment"]["alert_id"].is_null());
    assert!(stored_calls(&store).await.is_empty());
    assert!(store.assessment(&id.to_string()).await.unwrap().is_none());
    assert!(store.alerts().await.unwrap().is_empty());
    let committed = import(&app, payload(&row(id, 60, false), false)).await.1;
    assert_eq!(committed["rows"][0]["outcome"], "imported");
    drop(store);
    drop(app);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn whole_file_validation_rejects_duplicates_and_bad_envelope_before_any_rows_run() {
    let app = hre_api::router(config(), false).await.unwrap();
    let id = uuid::Uuid::new_v4();
    let duplicate = format!("{}{}", row(id, 0, false), row(id, 0, false));
    assert_eq!(
        import(&app, payload(&duplicate, false)).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut bad = payload(&row(id, 0, false), false);
    bad["conversation_id"] = json!("");
    assert_eq!(import(&app, bad).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        import(&app, payload(&row(id, 0, false), false)).await.1["rows"][0]["outcome"],
        "imported"
    );
}

#[test]
fn generated_contract_exposes_csv_envelope_and_partial_outcomes() {
    let schema = serde_json::to_value(hre_api::openapi()).unwrap();
    let endpoint = &schema["paths"]["/v1/calllogs/import"]["post"];
    assert!(endpoint["requestBody"]["content"]["application/json"].is_object());
    assert!(endpoint["description"]
        .as_str()
        .unwrap()
        .contains("not all-or-nothing"));
    assert!(
        schema["components"]["schemas"]["CallImportResponse"]["properties"]["rows"].is_object()
    );
}
