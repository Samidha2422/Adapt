# C8: CLI and nine scenario fixtures

## 0. Your role in one paragraph
You are teammate C. Build the CLI and synthetic scenario inputs that drive the engine via its HTTP contract. P1 depends on L1; end-to-end assertions additionally depend on L6 and the relevant feature implementations.

## 1. Project context
The Harassment Risk Engine is a Rust HTTP service that scores messages, images, audio, and call events using content and directional conversation history. Demo applications, dashboards, and bots are its clients.
IDs are pseudonymized with a server HMAC salt before storage.
Media is processed transiently; retain scores, masked transcripts, OCR text, and hashes rather than uploads.
Analyze only consented material and keep human moderators responsible for decisions.
Every feature is Built, Simulated, or Roadmap; synthetic data, benign proxies, and demo overrides must be disclosed, and any simulated contribution sets simulated=true.

## 2. Repo map and your boundary
Branch c-surfaces/cli-scenarios. Edit crates/hre-cli/ only; ask Lead to place or share final fixtures with the demo repo and to wire just recipes and integration tests. Do not edit Lead crates, workspace root, data/ or other owners' files.

## 3. Prerequisites already done
From `crates/hre-core/src/types.rs`:
```rust
wire_enum!(EventKind { Message, Image, Audio, CallEvent });
wire_enum!(ChannelType { Dm, Group, Comment, Livestream, Voice, Call });
wire_enum!(CallDirection { Incoming, Outgoing });
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    #[serde(rename = "type")]
    pub kind: ChannelType,
    pub conversation_id: String,
    #[serde(default)] pub group_id: Option<String>,
    #[serde(default)] pub post_id: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(default, deny_unknown_fields)]
pub struct EventContext {
    pub is_contact: bool,
    pub recipient_blocked_sender: bool,
    pub recipient_muted_sender: bool,
    pub stop_requested: bool,
    pub sender_account_age_days: Option<u32>,
    pub recipient_is_minor: Option<bool>,
    pub reply_to_event_id: Option<Uuid>,
    pub mentions: Vec<String>,
    pub language_hint: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Message { pub text: String }
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Media {
    pub mime: String,
    #[serde(default)] pub caption: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Call {
    pub direction: CallDirection,
    pub duration_s: u32,
    pub answered: bool,
    #[serde(default)] pub voicemail: bool,
}
/// Client envelope. Raw identities must be pseudonymized before storage.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeRequest {
    pub event_id: Uuid,
    pub kind: EventKind,
    pub occurred_at: DateTime<Utc>,
    #[serde(default)] pub draft: bool,
    pub sender_id: String,
    pub recipient_id: String,
    pub platform: String,
    pub channel: Channel,
    #[serde(default)] pub context: EventContext,
    #[serde(default)] pub message: Option<Message>,
    #[serde(default)] pub media: Option<Media>,
    #[serde(default)] pub call: Option<Call>,
}
```
Existing validation signature: `pub fn validate(&self) -> Result<(), CoreError>`. It requires one matching payload; image/audio require media metadata, not a message.
From `crates/hre-core/src/types.rs`:
```rust
wire_enum!(Tier { Low, Review, High, Urgent });
wire_enum!(Action { Log, WarnSender, BlurMedia, Throttle, Escalate });
wire_enum!(SecondaryAction { AlertModerator, CollapseReplies, RateLimit, NeedsHumanReview, ShowTranscript });
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Reason {
    pub code: String,
    pub text: String,
    pub weight: f64,
    pub evidence: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct Signals {
    pub text: Signal, pub ocr: Signal, pub image: Signal, pub audio: Signal,
    pub call: Signal, pub behavior: Signal, pub pile_on: Signal, pub pii: Signal,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Nudge { pub text: String, pub rewrite_hint: String }
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AnalyzeResponse {
    pub event_id: Uuid,
    #[schema(minimum = 0, maximum = 100)] pub severity: u8,
    pub tier: Tier,
    pub reasons: Vec<Reason>,
    pub signals: Signals,
    pub recommended_action: Action,
    pub secondary_actions: Vec<SecondaryAction>,
    pub alert_id: Option<Uuid>,
    pub latency_ms: u64,
    pub model_versions: BTreeMap<String, String>,
    pub simulated: bool,
    pub degraded: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)] pub nudge: Option<Nudge>,
}
```
From `crates/hre-core/src/types.rs` (the wire_enum macro derives Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema and uses snake_case serialization):
```rust
wire_enum!(Status { Built, Simulated, Roadmap });
wire_enum!(Backend { Onnx, Heuristic, Disabled, Ggml });
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Signal {
    #[schema(minimum = 0, maximum = 1)] pub score: f64,
    pub label: String,
    pub status: Status,
    pub backend: Backend,
    #[schema(minimum = 0, maximum = 1)] pub confidence: f64,
    pub details: BTreeMap<String, Value>,
}
```
Here `Value` is `serde_json::Value`, and `BTreeMap` is from std.
The prompt's expected fixture path is `data/test/scenarios/*.json`; that is outside this card's ownership, so deliver source fixtures inside the CLI and request Lead to arrange the shared final path.

## 4. Exact deliverables
Create `crates/hre-cli/Cargo.toml`, `src/main.rs`, fixture files and tests. **Proposed command signatures pending Lead agreement**:
```text
hre-cli seed
hre-cli simulate <scenario>
hre-cli import <path>
hre-cli export <subject>
```
Agree flags, auth, URLs, fixture schema and shared path with Lead; commands must use real documented HTTP routes and return nonzero for errors, not pretend absent endpoints exist.

## 5. Step-by-step instructions
1. Inspect current OpenAPI and actual mock endpoints; list gaps with Lead before implementing dependent commands.
2. Use workspace serde = =1.0.219, serde_json = =1.0.140, chrono = =0.4.41, uuid = =1.16.0, tokio = =1.45.1, anyhow = =1.0.98 where needed. HTTP and argument parser crates are not pinned; request Lead-approved exact versions or use already available facilities.
3. Author nine explicit synthetic fixtures: polite stalker (review then high after block); pile-on (high/feed protection); cyberflashing (high/blur, proxy simulated); OCR screenshot (flag/blur/reason); threatening synthetic TTS (high/urgent); overnight calls (review then high); reciprocal heavy couple (low); confirmed invented slang (miss then caught); evidence export (verifiable hash chain).
4. Separate fixture inputs, expected observations and provenance. Populate matching message/call/media payloads; never embed real abusive media or raw user data.
5. Seed/reset only through authorized routes and use appropriate moderator/admin credentials without logging them. Report unsupported operations as pending.
6. Provide a deterministic fixture reader, useful errors and CLI help. Coordinate with Lead on `just demo-seed`, shared fixture placement, and scenario integration assertions.

## 6. Behavior rules and edge cases
No model files should be needed for mock fixtures, but missing real pipeline support is reported as pending/degraded. Validate malformed or unknown scenario input. Never set success solely because HTTP returned 200; check actual expected tier/action when live functionality exists.

## 7. Tests you must write
test_fixture_ids_unique; test_requests_match_declared_payload; test_nine_scenario_expectations; test_missing_fixture_reports_error; test_mock_unavailable_fails_clearly; test_synthetic_proxy_is_marked; test_credentials_not_logged.

## 8. Commands
Only use logged recipes. Bash, for a recipe Lead confirms exists:
```sh
just check > target/logs/card-check.log 2>&1; tail -n 40 target/logs/card-check.log; grep -n "error" -A6 target/logs/card-check.log | head -60
```
On Windows use `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 check`; it writes `target/logs/check.log` and `check.exit`. Read the exit file; locally inspect only the last 40 lines and bounded error excerpts. Request missing task-specific logged recipes from Lead; do not pretend an unimplemented recipe tested your work. Never send raw logs in your report.

## 9. Definition of done
- Nine fixtures with expectations and honest provenance are available to Lead.
- Working CLI operations are tested; unavailable routes and unmet scenario expectations are clearly listed.
- Shared fixture placement and demo-seed remain Lead integration work.

## 10. Do NOT
Do not edit other owners' crates, root workspace files, generated OpenAPI, or shared configuration; request Lead changes through docs/REQUESTS.md. Do not add dependencies not listed or approved; do not use unwrap/expect/panic in non-test code, retain raw media by default, or commit models, datasets, secrets, or raw IDs. Do not claim proposed or missing integration is implemented.

## 11. How to report back
Report at most ten lines: branch; changed files; interface decisions and Lead agreements; exact logged commands; exit codes and test results; behavior verified; degradation/simulation; incomplete work; dependency requests; questions. Commit verified work using a conventional commit.

## 12. If blocked
Write `BLOCKED.md` inside your owned task directory with the precise question, missing owner/interface, and what is safe to continue; stop dependent work. Send Lead the blocker.
