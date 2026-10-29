#![allow(clippy::unwrap_used, clippy::expect_used, unused_imports)]
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use hre_core::{Backend, EngineConfig, Status};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Instant;
use tower::ServiceExt;

fn find_models_dir() -> PathBuf {
    for candidate in [
        PathBuf::from("models"),
        PathBuf::from("../../models"),
        PathBuf::from("../models"),
    ] {
        if candidate.join("manifest.json").exists() {
            return candidate.canonicalize().unwrap_or(candidate);
        }
    }
    PathBuf::from("models")
}

fn valid_test_png() -> Vec<u8> {
    vec![
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 2, 8, 2,
        0, 0, 0, 253, 212, 154, 115, 0, 0, 0, 16, 73, 68, 65, 84, 120, 156, 99, 252, 207, 0, 2, 76,
        96, 146, 1, 0, 13, 29, 1, 3, 130, 201, 113, 255, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96,
        130,
    ]
}

fn config() -> EngineConfig {
    let models = find_models_dir();
    EngineConfig {
        salt: "01234567890123456789012345678901".into(),
        api_keys: vec!["key".into()],
        moderator_tokens: vec!["mod".into()],
        database_url: "sqlite::memory:".into(),
        models_dir: models.to_string_lossy().to_string(),
        demo_mode: false,
        ..Default::default()
    }
}

fn envelope(mime: &str) -> Value {
    json!({
        "event_id": uuid::Uuid::new_v4(),
        "kind": "image",
        "occurred_at": "2026-10-28T12:00:00Z",
        "sender_id": "real-test-sender",
        "recipient_id": "real-test-recipient",
        "platform": "demo-chat",
        "channel": {"type": "dm", "conversation_id": "test-channel"},
        "media": {"mime": mime}
    })
}

async fn upload_image(
    app: &Router,
    event: &Value,
    bytes: &[u8],
    mime: &str,
) -> (StatusCode, Value, u128) {
    let start = Instant::now();
    let mut body = format!(
        "--hre-boundary\r\nContent-Disposition: form-data; name=\"request\"\r\n\r\n{event}\r\n--hre-boundary\r\nContent-Disposition: form-data; name=\"image\"; filename=\"test.png\"\r\nContent-Type: {mime}\r\n\r\n"
    ).into_bytes();
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

    let duration_ms = start.elapsed().as_millis();
    let status = response.status();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json_val = serde_json::from_slice(&body_bytes).unwrap();
    (status, json_val, duration_ms)
}

#[tokio::test]
#[ignore = "requires the genuine NSFW model and image fixtures; run just multimodal-smoke"]
async fn test_real_api_nsfw_classification_negative_and_positive() {
    let models_dir = find_models_dir();
    assert!(
        models_dir.join("nsfw.onnx").is_file(),
        "transfer the genuine NSFW model first"
    );

    let app = hre_api::router(config(), false).await.unwrap();

    // 1. Negative fixture (Normal image)
    let normal_bytes = std::fs::read(models_dir.join("synthetic_test.png")).unwrap();
    let event_norm = envelope("image/png");
    let (status, res_norm, dur_norm) =
        upload_image(&app, &event_norm, &normal_bytes, "image/png").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(res_norm["signals"]["image"]["status"], "built");
    assert_eq!(res_norm["signals"]["image"]["backend"], "onnx");
    assert_eq!(res_norm["signals"]["image"]["label"], "normal");
    assert!(res_norm["signals"]["image"]["score"].as_f64().unwrap() < 0.10);
    assert_eq!(res_norm["simulated"], false);
    assert_eq!(res_norm["recommended_action"], "log");
    println!("NSFW Negative Fixture passed in {}ms", dur_norm);

    // 2. Positive fixture (Explicit image)
    let pos_path = models_dir.join("nsfw_positive_fixture.png");
    if pos_path.exists() {
        let pos_bytes = std::fs::read(&pos_path).unwrap();
        let event_pos = envelope("image/png");
        let (status, res_pos, dur_pos) =
            upload_image(&app, &event_pos, &pos_bytes, "image/png").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(res_pos["signals"]["image"]["status"], "built");
        assert_eq!(res_pos["signals"]["image"]["backend"], "onnx");
        assert_eq!(res_pos["signals"]["image"]["label"], "nsfw");
        let nsfw_score = res_pos["signals"]["image"]["score"].as_f64().unwrap();
        assert!(
            nsfw_score >= 0.50,
            "Expected NSFW score >= 0.50, got {nsfw_score}"
        );
        assert_eq!(res_pos["recommended_action"], "blur_media");
        println!(
            "NSFW Positive Fixture passed with score {} in {}ms",
            nsfw_score, dur_pos
        );
    }
}

#[tokio::test]
#[ignore = "requires genuine deepfake/NSFW models and fixtures; run just multimodal-smoke"]
async fn test_real_api_deepfake_classification_authentic_fake_and_composite() {
    let models_dir = find_models_dir();
    assert!(
        models_dir.join("deepfake.onnx").is_file(),
        "transfer the genuine deepfake model first"
    );

    let app = hre_api::router(config(), false).await.unwrap();

    // 1. Authentic face/image
    let normal_bytes = std::fs::read(models_dir.join("synthetic_test.png")).unwrap();
    let event_norm = envelope("image/png");
    let (status, res_norm, dur_norm) =
        upload_image(&app, &event_norm, &normal_bytes, "image/png").await;

    assert_eq!(status, StatusCode::OK);
    let df_score_norm = res_norm["signals"]["image"]["details"]["deepfake_score"]
        .as_f64()
        .unwrap();
    let df_auth_norm = res_norm["signals"]["image"]["details"]["deepfake_authentic"]
        .as_f64()
        .unwrap();
    assert!(
        df_score_norm < 0.50,
        "Expected authentic deepfake score < 0.50, got {df_score_norm}"
    );
    assert!(
        df_auth_norm > 0.50,
        "Expected authentic prob > 0.50, got {df_auth_norm}"
    );
    println!(
        "Deepfake Authentic test passed (score={:.4}) in {}ms",
        df_score_norm, dur_norm
    );

    // 2. Manipulated/fake face fixture
    let fake_path = models_dir.join("deepfake_positive_fixture.png");
    if fake_path.exists() {
        let fake_bytes = std::fs::read(&fake_path).unwrap();
        let event_fake = envelope("image/png");
        let (status, res_fake, dur_fake) =
            upload_image(&app, &event_fake, &fake_bytes, "image/png").await;

        assert_eq!(status, StatusCode::OK);
        let df_score_fake = res_fake["signals"]["image"]["details"]["deepfake_score"]
            .as_f64()
            .unwrap();
        assert!(
            df_score_fake >= 0.70,
            "Expected manipulated score >= 0.70, got {df_score_fake}"
        );
        println!(
            "Deepfake Manipulated test passed (score={:.4}) in {}ms",
            df_score_fake, dur_fake
        );
    }

    // 3. Composite Synthetic Intimate Media (deepfake >= 0.70 AND nsfw >= 0.50)
    let composite_path = models_dir.join("synthetic_intimate_fixture.png");
    if composite_path.exists() {
        let comp_bytes = std::fs::read(&composite_path).unwrap();
        let event_comp = envelope("image/png");
        let (status, res_comp, dur_comp) =
            upload_image(&app, &event_comp, &comp_bytes, "image/png").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            res_comp["signals"]["image"]["details"]["synthetic_intimate"],
            true
        );
        let reasons = res_comp["reasons"].as_array().unwrap();
        let has_composite_reason = reasons
            .iter()
            .any(|r| r["code"] == "POSSIBLE_SYNTHETIC_INTIMATE_MEDIA");
        assert!(
            has_composite_reason,
            "Expected POSSIBLE_SYNTHETIC_INTIMATE_MEDIA reason"
        );
        assert_eq!(res_comp["tier"], "urgent");
        assert_eq!(res_comp["recommended_action"], "blur_media");
        println!(
            "Deepfake Composite Synthetic Intimate passed in {}ms",
            dur_comp
        );
    }
}

#[tokio::test]
#[ignore = "requires genuine OCR models, vocabulary and fixtures; run just multimodal-smoke"]
async fn test_real_api_ocr_pixel_recognition_single_and_multiline() {
    let models_dir = find_models_dir();
    for name in [
        "ocr_detector.onnx",
        "ocr_recognizer.onnx",
        "ocr_vocab.json",
        "_test_fixtures/ocr_single_line.png",
        "_test_fixtures/ocr_multiline.png",
    ] {
        assert!(
            models_dir.join(name).is_file(),
            "missing OCR artifact: {name}"
        );
    }

    let app = hre_api::router(config(), false).await.unwrap();

    // 1. Single-line cropped text banner: "STOP CALLING ME"
    let single_path = models_dir.join("_test_fixtures/ocr_single_line.png");
    if single_path.exists() {
        let single_bytes = std::fs::read(&single_path).unwrap();
        let event_single = envelope("image/png");
        let (status, res_single, dur_single) =
            upload_image(&app, &event_single, &single_bytes, "image/png").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(res_single["signals"]["ocr"]["backend"], "onnx");
        assert_eq!(res_single["signals"]["ocr"]["status"], "built");
        assert_eq!(res_single["simulated"], false);
        let ocr_conf = res_single["signals"]["ocr"]["confidence"].as_f64().unwrap();
        assert!(
            ocr_conf >= 0.50,
            "Expected OCR confidence >= 0.50, got {ocr_conf}"
        );
        println!(
            "OCR Single-line Recognition passed (conf={:.3}) in {}ms",
            ocr_conf, dur_single
        );
    }

    // 2. Multi-line document: tests full detection + cropping + recognition pipeline
    let multi_path = models_dir.join("_test_fixtures/ocr_multiline.png");
    if multi_path.exists() {
        let multi_bytes = std::fs::read(&multi_path).unwrap();
        let event_multi = envelope("image/png");
        let (status, res_multi, dur_multi) =
            upload_image(&app, &event_multi, &multi_bytes, "image/png").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(res_multi["signals"]["ocr"]["status"], "built");
        assert_eq!(res_multi["signals"]["ocr"]["backend"], "onnx");
        assert_eq!(res_multi["simulated"], false);
        let box_count = res_multi["signals"]["ocr"]["details"]["box_count"]
            .as_u64()
            .unwrap_or(0);
        assert!(
            box_count >= 2,
            "Expected at least 2 detected boxes, got {box_count}"
        );
        let ocr_score = res_multi["signals"]["ocr"]["score"].as_f64().unwrap_or(0.0);
        assert!(
            ocr_score >= 0.0,
            "Expected non-negative risk score, got {ocr_score}"
        );
        let conf = res_multi["signals"]["ocr"]["confidence"]
            .as_f64()
            .unwrap_or(0.0);
        assert!(
            conf >= 0.50,
            "Expected OCR recognition confidence >= 0.50, got {conf}"
        );
        println!(
            "OCR Multi-line Document passed (boxes={}, score={:.3}) in {}ms",
            box_count, ocr_score, dur_multi
        );
    }

    // 2b. Longer multi-line document with warning and harassment cues
    let long_path = models_dir.join("_test_fixtures/ocr_long_multiline.png");
    if long_path.exists() {
        let long_bytes = std::fs::read(&long_path).unwrap();
        let event_long = envelope("image/png");
        let (status, res_long, dur_long) =
            upload_image(&app, &event_long, &long_bytes, "image/png").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(res_long["signals"]["ocr"]["status"], "built");
        assert_eq!(res_long["signals"]["ocr"]["backend"], "onnx");
        let box_count = res_long["signals"]["ocr"]["details"]["box_count"]
            .as_u64()
            .unwrap_or(0);
        assert!(
            box_count >= 3,
            "Expected at least 3 detected boxes in 5-line page, got {box_count}"
        );
        println!(
            "OCR Long Multi-line Document passed (boxes={}) in {}ms",
            box_count, dur_long
        );
    }

    // 2c. Image with no visible text (smooth/gradient canvas)
    let no_text_path = models_dir.join("_test_fixtures/ocr_no_text.png");
    if no_text_path.exists() {
        let no_text_bytes = std::fs::read(&no_text_path).unwrap();
        let event_no_text = envelope("image/png");
        let (status, res_no_text, dur_no_text) =
            upload_image(&app, &event_no_text, &no_text_bytes, "image/png").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(res_no_text["signals"]["ocr"]["status"], "built");
        assert_eq!(res_no_text["signals"]["ocr"]["backend"], "onnx");
        assert_eq!(res_no_text["signals"]["ocr"]["score"], 0.0);
        let box_count = res_no_text["signals"]["ocr"]["details"]["box_count"]
            .as_u64()
            .unwrap_or(1);
        assert_eq!(box_count, 0, "Expected 0 detected boxes in no-text image");
        println!("OCR No-text image passed in {}ms", dur_no_text);
    }

    // 2d. Document with varied line widths
    let varied_path = models_dir.join("_test_fixtures/ocr_varied_widths.png");
    if varied_path.exists() {
        let varied_bytes = std::fs::read(&varied_path).unwrap();
        let event_varied = envelope("image/png");
        let (status, res_varied, dur_varied) =
            upload_image(&app, &event_varied, &varied_bytes, "image/png").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(res_varied["signals"]["ocr"]["status"], "built");
        assert_eq!(res_varied["signals"]["ocr"]["backend"], "onnx");
        let box_count = res_varied["signals"]["ocr"]["details"]["box_count"]
            .as_u64()
            .unwrap_or(0);
        assert!(
            box_count >= 2,
            "Expected at least 2 boxes in varied widths document, got {box_count}"
        );
        println!(
            "OCR Varied Widths passed (boxes={}) in {}ms",
            box_count, dur_varied
        );
    }

    // 3. Blank / non-text tiny image: tests graceful fallback without panic
    let blank_bytes = valid_test_png();
    let event_blank = envelope("image/png");
    let (status, res_blank, dur_blank) =
        upload_image(&app, &event_blank, &blank_bytes, "image/png").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(res_blank["signals"]["ocr"]["score"], 0.0);
    assert_eq!(res_blank["signals"]["ocr"]["status"], "roadmap");
    println!("OCR Blank image fallback test passed in {}ms", dur_blank);
}

#[tokio::test]
#[ignore = "requires genuine Whisper assets, Python dependencies and fixtures; run just multimodal-smoke"]
async fn test_real_api_whisper_audio_speech_silence_and_invalid() {
    let models_dir = find_models_dir();
    for name in [
        "whisper_encoder.onnx",
        "whisper_decoder.onnx",
        "whisper_tokenizer.json",
        "whisper_preprocessor_config.json",
        "_test_fixtures/speech_stop_calling_me.wav",
        "_test_fixtures/silence.wav",
    ] {
        assert!(
            models_dir.join(name).is_file(),
            "missing Whisper artifact: {name}"
        );
    }

    let app = hre_api::router(config(), false).await.unwrap();

    // 1. Test API endpoint /v1/recordings/analyze with caption and check completed job
    let rec_req = json!({
        "mime": "audio/mpeg",
        "caption": "Leave me alone or I will report you."
    });
    let submit_res = app
        .clone()
        .oneshot(
            Request::post("/v1/recordings/analyze")
                .header("x-api-key", "key")
                .header("content-type", "application/json")
                .body(Body::from(rec_req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(submit_res.status(), StatusCode::ACCEPTED);
    let submit_body = submit_res.into_body().collect().await.unwrap().to_bytes();
    let job: Value = serde_json::from_slice(&submit_body).unwrap();
    let job_id = job["job_id"].as_str().unwrap();

    let job_res = app
        .clone()
        .oneshot(
            Request::get(format!("/v1/jobs/{job_id}"))
                .header("x-api-key", "key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(job_res.status(), StatusCode::OK);
    let job_body = job_res.into_body().collect().await.unwrap().to_bytes();
    let job_data: Value = serde_json::from_slice(&job_body).unwrap();
    assert_eq!(job_data["status"], "completed");
    println!(
        "API /v1/recordings/analyze and /v1/jobs/{{id}} completed: transcript=\"{}\"",
        job_data["result"]["raw_transcript"]
    );

    // 2. Direct real audio waveform testing through OnnxAudioBackend
    let fallback = hre_audio::HeuristicAudioBackend::new();
    let audio_backend = hre_ml::OnnxAudioBackend::load(&models_dir, fallback)
        .expect("Failed to load OnnxAudioBackend");

    // Real Speech Audio ("Stop calling me.")
    let speech_path = models_dir.join("_test_fixtures/speech_stop_calling_me.wav");
    if speech_path.exists() {
        let speech_bytes = std::fs::read(&speech_path).unwrap();
        let start = Instant::now();
        let report = audio_backend
            .process_audio(&speech_bytes, "audio/wav")
            .expect("audio transcription failed");
        let dur = start.elapsed().as_millis();

        assert_eq!(report.signal.backend, Backend::Onnx);
        assert_eq!(report.signal.status, Status::Built);
        assert!(!report.simulated);
        assert!(
            report
                .raw_transcript
                .to_lowercase()
                .contains("stop calling me"),
            "Expected transcript to contain 'stop calling me', got: '{}'",
            report.raw_transcript
        );
        println!(
            "Whisper Real Speech passed: \"{}\" in {}ms",
            report.raw_transcript, dur
        );
    }

    // Pure Silence WAV
    let silence_path = models_dir.join("_test_fixtures/silence.wav");
    if silence_path.exists() {
        let silence_bytes = std::fs::read(&silence_path).unwrap();
        let report = audio_backend
            .process_audio(&silence_bytes, "audio/wav")
            .expect("silence processing failed");

        assert_eq!(report.raw_transcript, "");
        assert_eq!(report.signal.score, 0.0);
        assert_eq!(report.signal.label, "audio_none");
        println!("Whisper Pure Silence passed (raw_transcript is empty)");
    }

    // Corrupt Audio Input
    let corrupt_bytes = b"NOT_A_VALID_WAV_HEADER_OR_DATA";
    let result = audio_backend.process_audio(corrupt_bytes, "audio/wav");
    assert!(result.is_err(), "Expected error on corrupt audio input");
    println!("Whisper Corrupt Audio rejection passed: {:?}", result.err());
}
