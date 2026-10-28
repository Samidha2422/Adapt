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
