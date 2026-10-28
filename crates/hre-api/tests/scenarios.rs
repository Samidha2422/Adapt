#![allow(clippy::unwrap_used)]
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use hre_core::{Action, AnalyzeResponse, EngineConfig, Pseudonymizer, SecondaryAction, Tier};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn app() -> Router {
    hre_api::router(
        EngineConfig {
            salt: "01234567890123456789012345678901".into(),
            api_keys: vec!["key".into()],
            moderator_tokens: vec!["mod".into()],
            database_url: "sqlite::memory:".into(),
            ..Default::default()
        },
        false,
    )
    .await
    .unwrap()
}
fn event(sender: &str, recipient: &str, second: i64) -> Value {
    json!({"event_id":uuid::Uuid::new_v4(),"kind":"message","occurred_at":chrono::DateTime::from_timestamp(1793188800+second,0).unwrap(),"sender_id":sender,"recipient_id":recipient,"platform":"demo-chat","channel":{"type":"dm","conversation_id":"scenario"},"message":{"text":"Hello, how is your day?"}})
}
async fn analyze(app: &Router, event: Value) -> AnalyzeResponse {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/analyze")
                .header("content-type", "application/json")
                .header("x-api-key", "key")
                .body(Body::from(event.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
async fn post(app: &Router, path: &str, input: &Value) -> StatusCode {
    app.clone()
        .oneshot(
            Request::post(path)
                .header("content-type", "application/json")
                .header("x-api-key", "key")
                .body(Body::from(input.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}
async fn alerts(app: &Router) -> Vec<AnalyzeResponse> {
    let response = app
        .clone()
        .oneshot(
            Request::get("/v1/alerts")
                .header("authorization", "Bearer mod")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
#[tokio::test]
async fn scenario_1_polite_stalker_rises_to_review_then_high_after_block() {
    let app = app().await;
    let mut review = false;
    for i in 0..80 {
        if i == 70 {
            let refusal = json!({"event_id":uuid::Uuid::new_v4(),"sender_id":"polite-sender","recipient_id":"target","platform":"demo-chat","kind":"block","occurred_at":chrono::DateTime::from_timestamp(1793188800+699,0).unwrap()});
            assert_eq!(
                post(&app, "/v1/events/refusal", &refusal).await,
                StatusCode::NO_CONTENT
            );
        }
        let input = event("polite-sender", "target", i * 10);
        let response = analyze(&app, input).await;
        review |= response.tier == Tier::Review;
        assert!(!response.simulated);
        if i == 79 {
            assert_eq!(response.tier, Tier::High);
            assert!(response.reasons.iter().any(|r| r.code.contains("REFUSAL")));
        }
    }
    assert!(review);
}
#[tokio::test]
async fn scenario_2_ten_senders_enable_feed_protection() {
    let app = app().await;
    for i in 0..10 {
        let response = analyze(&app, event(&format!("account-{i}"), "target", i)).await;
        if i == 9 {
            assert_eq!(response.tier, Tier::High);
            assert!(response
                .secondary_actions
                .contains(&SecondaryAction::CollapseReplies));
            assert!(response.signals.pile_on.score >= 0.6);
        }
    }
    let id = Pseudonymizer::new(b"01234567890123456789012345678901")
        .unwrap()
        .subject("demo-chat", "target")
        .unwrap()
        .0;
    let response = app
        .clone()
        .oneshot(
            Request::get(format!("/v1/subjects/{id}/feed-protection"))
                .header("x-api-key", "key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["enabled"], true);
}
#[tokio::test]
async fn scenario_7_healthy_messages_and_long_answered_calls_stay_low() {
    let app = app().await;
    for i in 0..100 {
        let (sender, recipient) = if i % 2 == 0 {
            ("alice", "bob")
        } else {
            ("bob", "alice")
        };
        let mut input = event(sender, recipient, i);
        input["context"] = json!({"is_contact":true});
        if i >= 90 {
            input.as_object_mut().unwrap().remove("message");
            input["kind"] = json!("call_event");
            input["call"] = json!({"direction":"outgoing","duration_s":900,"answered":true});
        }
        assert_eq!(analyze(&app, input).await.tier, Tier::Low, "event {i}");
    }
}
#[tokio::test]
async fn risky_drafts_do_not_contaminate_subsequent_behavior() {
    let app = app().await;
    for i in 0..80 {
        let mut input = event("alice", "bob", i);
        input["draft"] = json!(true);
        analyze(&app, input).await;
    }
    let result = analyze(&app, event("alice", "bob", 81)).await;
    assert_eq!(result.tier, Tier::Low);
    assert!(result.alert_id.is_none());
}

#[tokio::test]
async fn retries_and_drafts_do_not_duplicate_or_publish_alerts() {
    let app = app().await;
    let mut input = event("sender", "target", 1);
    input["message"]["text"] = json!("I will kill you");
    let first = analyze(&app, input.clone()).await;
    assert!(first.alert_id.is_some());
    let retry = analyze(&app, input.clone()).await;
    assert_eq!(first.alert_id, retry.alert_id);
    assert_eq!(alerts(&app).await.len(), 1);
    input["message"]["text"] = json!("different content");
    assert_eq!(
        post(&app, "/v1/analyze", &input).await,
        StatusCode::CONFLICT
    );
    input["message"]["text"] = json!("I will kill you");
    input["draft"] = json!(true);
    let draft = analyze(&app, input).await;
    assert!(draft.alert_id.is_none());
    assert!(draft.nudge.is_some());
    assert_eq!(alerts(&app).await.len(), 1);
}

#[tokio::test]
async fn old_block_remains_active_until_unblock() {
    let app = app().await;
    let mut refusal = json!({"event_id":uuid::Uuid::new_v4(),"sender_id":"sender","recipient_id":"target","platform":"demo-chat","kind":"block","occurred_at":chrono::DateTime::from_timestamp(1793188800-8*86400,0).unwrap()});
    assert_eq!(
        post(&app, "/v1/events/refusal", &refusal).await,
        StatusCode::NO_CONTENT
    );
    let blocked = analyze(&app, event("sender", "target", 1)).await;
    assert!(blocked.reasons.iter().any(|r| r.code.contains("REFUSAL")));
    refusal["event_id"] = json!(uuid::Uuid::new_v4());
    refusal["kind"] = json!("unblock");
    refusal["occurred_at"] = json!(chrono::DateTime::from_timestamp(1793188802, 0).unwrap());
    assert_eq!(
        post(&app, "/v1/events/refusal", &refusal).await,
        StatusCode::NO_CONTENT
    );
    let unblocked = analyze(&app, event("sender", "target", 3)).await;
    assert!(!unblocked.reasons.iter().any(|r| r.code.contains("REFUSAL")));
}

#[tokio::test]
async fn draft_reusing_event_id_keeps_persisted_history() {
    let app = app().await;
    let input = event("sender", "target", 1);
    analyze(&app, input.clone()).await;
    let mut draft = input;
    draft["draft"] = json!(true);
    draft["occurred_at"] = json!(chrono::DateTime::from_timestamp(1793188802, 0).unwrap());
    let reused = analyze(&app, draft.clone()).await;
    draft["event_id"] = json!(uuid::Uuid::new_v4());
    let fresh = analyze(&app, draft).await;
    assert_eq!(reused.signals.behavior.score, fresh.signals.behavior.score);
}

#[tokio::test]
async fn refusal_retries_conflict_on_changed_content() {
    let app = app().await;
    let mut refusal = json!({"event_id":uuid::Uuid::new_v4(),"sender_id":"sender","recipient_id":"target","platform":"demo-chat","kind":"block","occurred_at":chrono::DateTime::from_timestamp(1793188800,0).unwrap()});
    assert_eq!(
        post(&app, "/v1/events/refusal", &refusal).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        post(&app, "/v1/events/refusal", &refusal).await,
        StatusCode::NO_CONTENT
    );
    refusal["kind"] = json!("unblock");
    assert_eq!(
        post(&app, "/v1/events/refusal", &refusal).await,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn unblock_does_not_clear_older_mute_or_stop_request() {
    for kind in ["mute", "stop_request"] {
        let app = app().await;
        let mut refusal = json!({"event_id":uuid::Uuid::new_v4(),"sender_id":"sender","recipient_id":"target","platform":"demo-chat","kind":kind,"occurred_at":chrono::DateTime::from_timestamp(1793188800-9*86400,0).unwrap()});
        assert_eq!(
            post(&app, "/v1/events/refusal", &refusal).await,
            StatusCode::NO_CONTENT
        );
        refusal["event_id"] = json!(uuid::Uuid::new_v4());
        refusal["kind"] = json!("block");
        refusal["occurred_at"] = json!(chrono::DateTime::from_timestamp(1793188798, 0).unwrap());
        assert_eq!(
            post(&app, "/v1/events/refusal", &refusal).await,
            StatusCode::NO_CONTENT
        );
        refusal["event_id"] = json!(uuid::Uuid::new_v4());
        refusal["kind"] = json!("unblock");
        refusal["occurred_at"] = json!(chrono::DateTime::from_timestamp(1793188799, 0).unwrap());
        assert_eq!(
            post(&app, "/v1/events/refusal", &refusal).await,
            StatusCode::NO_CONTENT
        );
        let result = analyze(&app, event("sender", "target", 1)).await;
        assert!(
            result.reasons.iter().any(|r| r.code.contains("REFUSAL")),
            "{kind}"
        );
    }
}

#[tokio::test]
async fn contact_at_block_timestamp_counts_as_after_refusal() {
    let app = app().await;
    let refusal = json!({"event_id":uuid::Uuid::new_v4(),"sender_id":"sender","recipient_id":"target","platform":"demo-chat","kind":"block","occurred_at":chrono::DateTime::from_timestamp(1793188800,0).unwrap()});
    assert_eq!(
        post(&app, "/v1/events/refusal", &refusal).await,
        StatusCode::NO_CONTENT
    );
    let result = analyze(&app, event("sender", "target", 0)).await;
    assert!(result.reasons.iter().any(|r| r.code.contains("REFUSAL")));
}

#[tokio::test]
async fn alerts_protection_and_retries_survive_router_restart() {
    let path = std::env::temp_dir().join(format!("hre-restart-{}.sqlite", uuid::Uuid::new_v4()));
    let url = format!("sqlite:{}", path.to_string_lossy().replace('\\', "/"));
    let config = EngineConfig {
        salt: "01234567890123456789012345678901".into(),
        api_keys: vec!["key".into()],
        moderator_tokens: vec!["mod".into()],
        database_url: url,
        ..Default::default()
    };
    let app = hre_api::router(config.clone(), false).await.unwrap();
    for i in 0..9 {
        analyze(&app, event(&format!("account-{i}"), "target", i)).await;
    }
    let last = event("account-9", "target", 9);
    analyze(&app, last.clone()).await;
    let before = alerts(&app).await;
    assert!(!before.is_empty());
    drop(app);
    let restarted = hre_api::router(config, false).await.unwrap();
    let after = alerts(&restarted).await;
    assert_eq!(before.len(), after.len());
    assert_eq!(
        before.last().unwrap().alert_id,
        after.last().unwrap().alert_id
    );
    let retry = analyze(&restarted, last).await;
    assert_eq!(retry.alert_id, after.last().unwrap().alert_id);
    assert_eq!(alerts(&restarted).await.len(), before.len());
    let id = Pseudonymizer::new(b"01234567890123456789012345678901")
        .unwrap()
        .subject("demo-chat", "target")
        .unwrap()
        .0;
    let response = restarted
        .clone()
        .oneshot(
            Request::get(format!("/v1/subjects/{id}/feed-protection"))
                .header("x-api-key", "key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["enabled"], true);
    drop(restarted);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn persistence_uses_pseudonyms_and_never_stores_drafts_or_message_text() {
    let path = std::env::temp_dir().join(format!("hre-privacy-{}.sqlite", uuid::Uuid::new_v4()));
    let url = format!("sqlite:{}", path.to_string_lossy().replace('\\', "/"));
    let app = hre_api::router(
        EngineConfig {
            salt: "01234567890123456789012345678901".into(),
            api_keys: vec!["key".into()],
            moderator_tokens: vec!["mod".into()],
            database_url: url.clone(),
            ..Default::default()
        },
        false,
    )
    .await
    .unwrap();
    let mut input = event("private-sender", "private-recipient", 1);
    input["message"]["text"] = json!("private message body");
    let id = input["event_id"].as_str().unwrap().to_owned();
    analyze(&app, input.clone()).await;
    input["event_id"] = json!(uuid::Uuid::new_v4());
    input["draft"] = json!(true);
    analyze(&app, input).await;
    let ids = Pseudonymizer::new(b"01234567890123456789012345678901").unwrap();
    let sender = ids.subject("demo-chat", "private-sender").unwrap().0;
    let recipient = ids.subject("demo-chat", "private-recipient").unwrap().0;
    let store = hre_store::Store::connect(&url).await.unwrap();
    let events = store
        .pair_window(&sender, &recipient, i64::MIN, i64::MAX)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    let persisted = format!(
        "{}{}",
        serde_json::to_string(&events).unwrap(),
        store.assessment(&id).await.unwrap().unwrap()
    );
    for secret in [
        "private-sender",
        "private-recipient",
        "private message body",
        "scenario",
    ] {
        assert!(!persisted.contains(secret));
    }
    drop(store);
    drop(app);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn scenario_3_cyberflashing_proxy_is_high_and_blurred() {
    let app = app().await;
    let input = json!({
        "event_id": uuid::Uuid::new_v4(),
        "kind": "image",
        "occurred_at": chrono::DateTime::from_timestamp(1793188800, 0).unwrap(),
        "sender_id": "stranger",
        "recipient_id": "target",
        "platform": "demo-chat",
        "channel": {"type": "dm", "conversation_id": "scenario-3"},
        "context": {"is_contact": false},
        "media": {
            "mime": "image/png",
            "caption": "cyberflashing"
        }
    });
    let response = analyze(&app, input).await;
    assert_eq!(response.tier, Tier::High);
    assert_eq!(response.recommended_action, Action::BlurMedia);
    assert!(response.simulated);
    assert!(
        response
            .reasons
            .iter()
            .any(|r| r.code.contains("UNSOLICITED_EXPLICIT_MEDIA")
                || r.code.contains("REPORTED_HASH"))
    );
}

#[tokio::test]
async fn scenario_4_screenshot_meme_ocr_extracts_and_blurs() {
    let app = app().await;
    let input = json!({
        "event_id": uuid::Uuid::new_v4(),
        "kind": "image",
        "occurred_at": chrono::DateTime::from_timestamp(1793188800, 0).unwrap(),
        "sender_id": "meme-bully",
        "recipient_id": "target",
        "platform": "demo-chat",
        "channel": {"type": "dm", "conversation_id": "scenario-4"},
        "context": {"is_contact": false},
        "media": {
            "mime": "image/png",
            "caption": "I will kill you and your family"
        }
    });
    let response = analyze(&app, input).await;
    assert!(response.tier == Tier::High || response.tier == Tier::Urgent);
    assert_eq!(response.recommended_action, Action::BlurMedia);
    assert!(response
        .reasons
        .iter()
        .any(|r| r.code == "OCR_ABUSE" || r.code.contains("THREAT")));
    assert!(response.signals.ocr.score >= 0.5);
}

#[tokio::test]
async fn scenario_5_voice_note_threatening_transcribed_shows_transcript() {
    let app = app().await;
    let input = json!({
        "event_id": uuid::Uuid::new_v4(),
        "kind": "audio",
        "occurred_at": chrono::DateTime::from_timestamp(1793188800, 0).unwrap(),
        "sender_id": "harasser",
        "recipient_id": "target",
        "platform": "demo-chat",
        "channel": {"type": "dm", "conversation_id": "scenario-5"},
        "context": {"is_contact": false},
        "media": {
            "mime": "audio/mpeg",
            "caption": "threat:I will find you and kill you"
        }
    });
    let response = analyze(&app, input).await;
    assert!(response.tier == Tier::High || response.tier == Tier::Urgent);
    assert!(response.signals.audio.score >= 0.8);
    assert!(response
        .secondary_actions
        .contains(&SecondaryAction::ShowTranscript));
    assert!(response.reasons.iter().any(|r| r.code.contains("THREAT")));
}

#[tokio::test]
async fn scenario_6_overnight_unanswered_calls_classified_high() {
    let app = app().await;
    let base_ts = 1793214000i64;
    for i in 0..40 {
        if i == 30 {
            let refusal = json!({
                "event_id": uuid::Uuid::new_v4(),
                "sender_id": "persistent-caller",
                "recipient_id": "target",
                "platform": "phone",
                "kind": "block",
                "occurred_at": chrono::DateTime::from_timestamp(base_ts + 29 * 60 + 30, 0).unwrap()
            });
            assert_eq!(
                post(&app, "/v1/events/refusal", &refusal).await,
                StatusCode::NO_CONTENT
            );
        }
        let input = json!({
            "event_id": uuid::Uuid::new_v4(),
            "kind": "call_event",
            "occurred_at": chrono::DateTime::from_timestamp(base_ts + i * 60, 0).unwrap(),
            "sender_id": "persistent-caller",
            "recipient_id": "target",
            "platform": "phone",
            "channel": {"type": "call", "conversation_id": "phone-call"},
            "context": {"is_contact": false, "recipient_blocked_sender": i >= 30},
            "call": {
                "direction": "incoming",
                "duration_s": 0,
                "answered": false,
                "voicemail": false
            }
        });
        let response = analyze(&app, input).await;
        if i >= 30 {
            assert!(response.tier == Tier::High || response.tier == Tier::Urgent);
            assert!(response.signals.call.score >= 0.6);
            assert!(response
                .reasons
                .iter()
                .any(|r| r.code.contains("REPEATED_CALLS")
                    || r.code.contains("NIGHT")
                    || r.code.contains("UNANSWERED")
                    || r.code.contains("REFUSAL")));
        }
    }
}

#[tokio::test]
async fn call_log_csv_import_and_recording_job() {
    let app = app().await;
    let csv = "timestamp,caller,callee,duration_s,answered,direction\n\
               1793188800,caller1,callee1,120,true,incoming\n\
               1793188860,caller1,callee1,0,false,incoming";
    let import_req = json!({ "csv_data": csv });
    let res = app
        .clone()
        .oneshot(
            Request::post("/v1/calllogs/import")
                .header("content-type", "application/json")
                .header("x-api-key", "key")
                .body(Body::from(import_req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&res.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["imported"], 2);

    // Test recording job
    let rec_req = json!({
        "mime": "audio/mpeg",
        "caption": "I will find you and kill you"
    });
    let res2 = app
        .clone()
        .oneshot(
            Request::post("/v1/recordings/analyze")
                .header("content-type", "application/json")
                .header("x-api-key", "key")
                .body(Body::from(rec_req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res2.status(), StatusCode::ACCEPTED);
    let body2: Value =
        serde_json::from_slice(&res2.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let job_id = body2["job_id"].as_str().unwrap();

    let res3 = app
        .clone()
        .oneshot(
            Request::get(format!("/v1/jobs/{job_id}"))
                .header("x-api-key", "key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res3.status(), StatusCode::OK);
    let body3: Value =
        serde_json::from_slice(&res3.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body3["status"], "completed");
}

#[tokio::test]
async fn scenario_8_slang_update_miss_confirm_caught() {
    let app = app().await;

    // 1. Before confirmation, message with invented slang 'vexling' is a miss (Tier::Low)
    let mut input = event("bully", "target", 1);
    input["message"]["text"] = json!("You are such a vexling");
    let res_before = analyze(&app, input.clone()).await;
    assert_eq!(res_before.tier, Tier::Low);
    assert!(!res_before.reasons.iter().any(|r| r.code == "SLANG_MATCH"));

    // 2. Moderator confirms the invented term 'vexling'
    let conf_req = json!({
        "term": "vexling",
        "meaning": "invented harassing slur",
        "severity": 0.45
    });
    let res_conf = app
        .clone()
        .oneshot(
            Request::post("/v1/slang/terms")
                .header("content-type", "application/json")
                .header("authorization", "Bearer mod")
                .body(Body::from(conf_req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_conf.status(), StatusCode::OK);

    // 3. Next message with 'vexling' is immediately caught (Tier::Review)
    let mut input_after = event("bully", "target", 2);
    input_after["message"]["text"] = json!("Stop being a vexling");
    let res_after = analyze(&app, input_after).await;
    assert_eq!(res_after.tier, Tier::Review);
    assert!(res_after.reasons.iter().any(|r| r.code == "SLANG_MATCH"));
}

#[tokio::test]
async fn scenario_9_evidence_export_multimodal_timeline_hash_chained_and_verifiable() {
    let app = app().await;

    // Seed events across multimodal channels: message, image, audio, call
    let mut msg_ev = event("harasser", "victim", 1);
    msg_ev["message"]["text"] = json!("I know where you live");
    analyze(&app, msg_ev).await;

    let img_ev = json!({
        "event_id": uuid::Uuid::new_v4(),
        "kind": "image",
        "occurred_at": chrono::DateTime::from_timestamp(1793188800 + 10, 0).unwrap(),
        "sender_id": "harasser",
        "recipient_id": "victim",
        "platform": "demo-chat",
        "channel": {"type": "dm", "conversation_id": "scenario-9"},
        "context": {"is_contact": false},
        "media": {
            "mime": "image/png",
            "caption": "cyberflashing"
        }
    });
    analyze(&app, img_ev).await;

    let aud_ev = json!({
        "event_id": uuid::Uuid::new_v4(),
        "kind": "audio",
        "occurred_at": chrono::DateTime::from_timestamp(1793188800 + 20, 0).unwrap(),
        "sender_id": "harasser",
        "recipient_id": "victim",
        "platform": "demo-chat",
        "channel": {"type": "dm", "conversation_id": "scenario-9"},
        "context": {"is_contact": false},
        "media": {
            "mime": "audio/mpeg",
            "caption": "threat:I will find you"
        }
    });
    analyze(&app, aud_ev).await;

    let call_ev = json!({
        "event_id": uuid::Uuid::new_v4(),
        "kind": "call_event",
        "occurred_at": chrono::DateTime::from_timestamp(1793188800 + 30, 0).unwrap(),
        "sender_id": "harasser",
        "recipient_id": "victim",
        "platform": "demo-chat",
        "channel": {"type": "call", "conversation_id": "scenario-9"},
        "context": {"is_contact": false},
        "call": {
            "direction": "incoming",
            "duration_s": 0,
            "answered": false,
            "voicemail": false
        }
    });
    analyze(&app, call_ev).await;

    // Export evidence
    let export_req = json!({ "subject_id": "victim" });
    let res_exp = app
        .clone()
        .oneshot(
            Request::post("/v1/evidence/export")
                .header("content-type", "application/json")
                .header("x-api-key", "key")
                .body(Body::from(export_req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_exp.status(), StatusCode::OK);
    let export_body: Value =
        serde_json::from_slice(&res_exp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let record_count = export_body["record_count"].as_u64().unwrap();
    assert_eq!(record_count, 4);
    assert!(!export_body["manifest_hash"].as_str().unwrap().is_empty());
    assert!(!export_body["hmac_signature"].as_str().unwrap().is_empty());

    // Verify evidence cryptographically
    let res_verify = app
        .clone()
        .oneshot(
            Request::post("/v1/evidence/verify")
                .header("content-type", "application/json")
                .header("x-api-key", "key")
                .body(Body::from(export_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_verify.status(), StatusCode::OK);
    let verify_body: Value =
        serde_json::from_slice(&res_verify.into_body().collect().await.unwrap().to_bytes())
            .unwrap();
    assert_eq!(verify_body["valid"], true);

    // Get printable HTML report with BSA 2023 Sec 63(4) certificate draft
    let res_html = app
        .clone()
        .oneshot(
            Request::get("/v1/evidence/victim/html")
                .header("x-api-key", "key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_html.status(), StatusCode::OK);
    let html_bytes = res_html.into_body().collect().await.unwrap().to_bytes();
    let html_str = String::from_utf8(html_bytes.to_vec()).unwrap();
    assert!(html_str.contains("Bharatiya Sakshya Adhiniyam 2023, Section 63(4)"));
    assert!(html_str.contains("cybercrime.gov.in"));
    assert!(html_str.contains("1930"));
}
