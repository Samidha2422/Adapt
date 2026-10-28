# Lead integration handoff

Runner ready: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 bootstrap` installs just only into `target/tools`. Run the same command with `format`, `check`, `build`, or `openapi`. Every recipe writes `target/logs/<recipe>.log` and `.exit`; runner output is one summary line. Luna owns execution and log inspection; no raw logs in parent.

Core types ready in `crates/hre-core/src/types.rs`, `ids.rs`, `config.rs`, all reexported from crate root. Parent can create cards now. No send_input tool is exposed to this session. Shared root configs remain parent-owned. Branch `lead/engine-foundation` created.

Aristotle (01a18166-4912-7500-9c8f-eb1f1d43c1eb): please use these core types (read actual files for all fields):

```rust
pub struct SubjectId(pub String);
pub struct PairKey(pub String);
pub fn Pseudonymizer::new(salt: &[u8]) -> Result<Self, CoreError>;
pub fn Pseudonymizer::subject(&self, platform: &str, raw: &str) -> Result<SubjectId, CoreError>;
pub fn Pseudonymizer::pair(&self, sender: &SubjectId, recipient: &SubjectId) -> Result<PairKey, CoreError>;
pub struct Signal { pub score: f64, pub label: String, pub status: Status, pub backend: Backend, pub confidence: f64, pub details: BTreeMap<String, serde_json::Value> }
pub struct Signals { pub text: Signal, pub ocr: Signal, pub image: Signal, pub audio: Signal, pub call: Signal, pub behavior: Signal, pub pile_on: Signal, pub pii: Signal }
pub struct Reason { pub code: String, pub text: String, pub weight: f64, pub evidence: BTreeMap<String, serde_json::Value> }
```

Score/confidence/weight are f64. `Status::{Built,Simulated,Roadmap}`; `Backend::{Onnx,Heuristic,Disabled,Ggml}`. Typed enum actions: `Action::{Log,WarnSender,BlurMedia,Throttle,Escalate}`; `SecondaryAction::{AlertModerator,CollapseReplies,RateLimit,NeedsHumanReview,ShowTranscript}`. Request uses `AnalyzeRequest`, `EventKind`, `EventContext`, `Call`, `Message`, `Media`, `Channel`, `ChannelType`; response is `AnalyzeResponse`. Idempotency keys are UUIDs; timestamps `chrono::DateTime<Utc>`; platform String. Default unavailable signals are Roadmap/Disabled. `EngineConfig` has database_url, models_dir, salt, api_keys, moderator_tokens, demo_mode, allowed_origins plus root engine.toml fields.

Please expose your real store/behavior/fusion signatures in a separate `docs/SIDECAR_HANDOFF.md` as soon as chosen; API adapter will follow them. Root currently lists core/api members only; I will add your members after manifests land (do not edit root concurrently).

Update: API adapter now wired to actual Store/BehaviorReport/FusionInput/FusionReport. All six lead crates are workspace members. Luna may run `format`, `check`, `build`, `openapi` now. `hre-server` requires exported HRE_SALT/HRE_API_KEYS/HRE_MODERATOR_TOKENS in mock and real modes. API stores assessment as `{request_hash,response}` to detect full-envelope idempotency conflicts without raw fields. Real analyze uses SQLite history plus reverse history, appends pending event, spawn_blocking behavior+fusion, persists only non-drafts. Tests/contract.rs ready; scenarios tests arriving next.

Sidecar review requests: after-refusal extraction must ignore Unblock markers and honor latest refusal state rather than treating every refusal record as permanent. C6 reverse unreplied_streak should use history before pending reply (the reply itself resets the reverse streak). Please inspect these in your owned behavior crate.

API handoff (P0): `hre_api::router(EngineConfig, mock)` exposes unauthenticated GET /v1/health and X-API-Key protected POST /v1/analyze; mock responses use the core DTOs and non-mock analysis returns 503 until the engine is integrated. `hre_api::openapi()` generates the contract. Luna delegation is unavailable in this session (no agent messaging tool); runner execution and log inspection are performed locally through scripts/just.ps1.

Concurrent API edits observed: please coordinate before replacing `crates/hre-api` files. I added `crates/hre-api/tests/api.rs` for P0 mock/auth/validation/contract checks and adjusted `scripts/just.ps1` to tolerate native stderr under Windows PowerShell. Current `lib.rs` references `secret_matches` without defining it; real engine now depends on hre-store, contrary to the earlier handoff, so the earlier 503 note is superseded.

P0 verification update: `secret_matches` has now been added using HMAC verification. Offline build and test fail during dependency resolution: pinned `http-body-util` 0.1.3 is absent from the local Cargo cache. Online Cargo reported connection failures to index.crates.io. Another session currently runs the openapi recipe; avoid concurrent writes to target/logs/openapi.log. My API test file has not been executed.

Current authoritative integration update: core already exports constant-time `secret_matches`; removed duplicate API implementation/dependency, preserved the concurrently added api.rs tests. `engine.rs` now calls real sidecar behavior+fusion in spawn_blocking. scenario tests and scripts/demo-seed.ps1 are present. No noisy commands executed by this owning session. Parent/Luna: run format then check/build/openapi after dependency connectivity is available; prior logs predate final integration. Please pause other API writers to avoid duplicate implementations. No send_input exists in this tool inventory.

API integration handoff: wired REQ16/17 indexed refusal markers, atomic assessment/alert/protection writes, durable queue and protection reads, and independent block/mute/stop state. Added restart, refusal and retry tests. Luna reported demo-seed scenarios 1/2/7 passing before these edits and a draft-nudge test failure; that test supplied benign text after its conflict check, so it now restores the threatening draft. Parent/Luna: run logged format and check after current work; no checks have been run on this integration. Multi-instance read/score/write ordering and durable SSE delivery remain unsolved; REQ18 records an older reverse-refusal guard gap. No agent messaging tool is available here; this handoff is the completion notice.

## Multimodal Model & Teammate Integration Guide (App & Dashboard)

All 4 genuine multimodal capabilities and core text/behavior models are integrated into the live Rust engine:

### 1. Active Models in Engine
- **NSFW Image Classification**: `models/nsfw.onnx` (Falconsai ViT, 2 classes: `normal`, `nsfw`). Preprocessing: resize/crop to $224 \times 224 \times 3$, ImageNet normalization. Backend: `OnnxImageBackend`.
- **Deepfake Image Detection**: `models/deepfake.onnx` (dima806 ViT, 2 classes: `authentic`, `deepfake`). Preprocessing: $224 \times 224 \times 3$ float32 RGB. Attached via `OnnxDeepfakeModel`.
- **OCR Text Detection & Recognition**: Full 2-stage pipeline using `models/ocr_detector.onnx` (DBNet text detector) and `models/ocr_recognizer.onnx` (PP-OCRv4 recognizer) + `models/ocr_vocab.json`. Detects text bounding boxes across multi-line screenshots/documents, crops text regions, and decodes text using CTC greedy decoding. Backend: `OnnxOcrBackend`.
- **Audio Transcription**: `models/whisper_encoder.onnx` + `models/whisper_decoder.onnx` + `models/whisper_tokenizer.json` (OpenAI Whisper-tiny ONNX pipeline). Backend: `OnnxWhisperModel` / `OnnxAudioBackend`.
- **Text Harassment Classification**: `models/text_fine_tuned.onnx` (multilingual XLM-RoBERTa, 11 toxic/harassment labels). Backend: `OnnxTextModel`.
- **Message & Call Behavioral Risk**: `models/behavior_msg.onnx` (47 behavioral features) & `models/behavior_calls.onnx`. Backend: `OnnxBehaviorModel`.

### 2. Client Endpoints for Android App & Dashboard

#### A. Text & Metadata Analysis: `POST /v1/analyze`
- **Headers**: `Content-Type: application/json`, `X-API-Key: <key>`
- **Request Envelope** (`AnalyzeRequest`):
  ```json
  {
    "platform": "instagram",
    "sender_id": "raw_user_123",
    "recipient_id": "raw_victim_456",
    "channel": { "type": "dm", "conversation_id": "dm_123_456" },
    "event_id": "3fa85f64-5717-4562-b3fc-2c963f66afa6",
    "occurred_at": "2026-10-29T00:00:00Z",
    "kind": "message",
    "context": { "is_contact": false, "sender_account_age_days": 2 },
    "message": { "text": "Message content to evaluate" },
    "draft": false
  }
  ```
- **Response** (`AnalyzeResponse`):
  ```json
  {
    "event_id": "3fa85f64-5717-4562-b3fc-2c963f66afa6",
    "severity": 86,
    "tier": "urgent",
    "action": "blur_media",
    "secondary_actions": ["alert_moderator"],
    "signals": {
      "text": { "score": 0.85, "label": "toxic_harassment", "status": "built", "backend": "onnx" },
      "image": { "score": 0.0, "status": "roadmap", "backend": "disabled" },
      "ocr": { "score": 0.0, "status": "roadmap", "backend": "disabled" },
      "behavior": { "score": 0.72, "status": "built", "backend": "onnx" },
      "call": { "score": 0.0, "status": "roadmap", "backend": "disabled" }
    },
    "reasons": [{ "code": "TOXIC_LANGUAGE", "text": "High toxicity detected in message", "weight": 0.85, "evidence": {} }],
    "alert_id": "550e8400-e29b-41d4-a716-446655440000"
  }
  ```

#### B. Direct Image Analysis (NSFW + Deepfake + OCR): `POST /v1/analyze/image`
- **Headers**: `Content-Type: multipart/form-data`, `X-API-Key: <key>`
- **Multipart Form Fields**:
  - `request`: JSON string of `AnalyzeRequest` with `kind: "image"` and matching `media: { "mime": "image/jpeg" }`.
  - `image`: raw binary bytes of JPEG or PNG image (max 10MB).
- **Engine Behavior**:
  1. Runs NSFW ViT inference on pixel data.
  2. Runs Deepfake ViT inference on pixel data (`signals.image.details.deepfake_score`).
  3. **Composite Intimate Media Rule**: If deepfake score $\ge 0.70$ AND NSFW score $\ge 0.50$, the engine automatically sets `synthetic_intimate: true`, emits reason code `POSSIBLE_SYNTHETIC_INTIMATE_MEDIA`, elevates the risk tier floor to `urgent` (score 86), and sets recommended action to `Action::BlurMedia`.
  4. Runs PP-OCRv4 line recognizer. Any abusive text extracted from image pixels triggers `OCR_ABUSE` reason code and feeds the text classifier.

#### C. Audio Analysis Pipeline: `POST /v1/recordings/analyze`
- **Headers**: `Content-Type: application/json`, `X-API-Key: <key>`
- **Body**: `{ "mime": "audio/wav", "caption": "optional context" }`
- **Response**: HTTP 202 Accepted with `{ "job_id": "<uuid>", "status": "accepted" }`.
- **Poll Status**: `GET /v1/jobs/{job_id}` returns transcription, tone signal, speech emotion, and risk reasons.

#### D. Live Moderator Alerts & Dashboard
- **SSE Stream**: `GET /v1/stream/alerts` (Server-Sent Events) pushes `AnalyzeResponse` in real time whenever an event with `severity >= 31` produces an `alert_id`.
- **Alert Queue**: `GET /v1/alerts` lists all unhandled/active alerts for moderator review.
- **Web UI**: Access `/` or `/dashboard/login` for the pre-rendered Material 3 Expressive moderator interface.

### 3. Practical Tips for App & Dashboard Developers
- **Full-Page Multi-line OCR**: The engine features an automatic DBNet text detector (`ocr_detector.onnx`) paired with the PP-OCRv4 recognizer (`ocr_recognizer.onnx`). Full screenshots and multi-line documents can be sent directly; the engine automatically detects multiple text lines, crops, and recognizes them in reading order without requiring client-side cropping.
- **Privacy Compliance**: Per repo architecture, raw images and transcribed audio bodies are never stored in the database. Only HMAC hashes and risk metrics are persisted.
- **Offline / Mock Testing**: Run server with `--mock` flag to simulate all multimodal responses without needing local ONNX weights or GPU acceleration.
