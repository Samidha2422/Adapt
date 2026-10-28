# C1: Dashboard shell and live alert queue

## 0. Your role in one paragraph
You are teammate C. Build a minimal moderator dashboard shell and live queue against the Lead's L1 mock, with honest handling of missing or unavailable feed endpoints. This is P1.

## 1. Project context
The Harassment Risk Engine is a Rust HTTP service that scores messages, images, audio, and call events using content and directional conversation history. Demo applications, dashboards, and bots are its clients.
IDs are pseudonymized with a server HMAC salt before storage.
Media is processed transiently; retain scores, masked transcripts, OCR text, and hashes rather than uploads.
Analyze only consented material and keep human moderators responsible for decisions.
Every feature is Built, Simulated, or Roadmap; synthetic data, benign proxies, and demo overrides must be disclosed, and any simulated contribution sets simulated=true.

## 2. Repo map and your boundary
Branch c-surfaces/dashboard-shell. Edit crates/hre-dashboard/ only. Lead owns hre-api, authentication, SSE, server wiring, workspace and API contract; request integration instead of editing them.

## 3. Prerequisites already done
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
`crates/hre-api/src/main.rs` currently calls `hre_api::router(config, mock).await?`. Prompt specifies GET /v1/alerts and GET /v1/stream/alerts; their availability and payloads must be checked in the actual OpenAPI/runtime, not assumed. Mock server existence does not establish that every route or feed is implemented.

## 4. Exact deliverables
Create `crates/hre-dashboard/Cargo.toml`, `src/lib.rs`, templates/assets and tests under that crate. **Proposed interface pending Lead agreement**:
```rust
pub fn router() -> axum::Router;
```
The router's state/auth integration, SSE event envelope and mock fixtures require Lead agreement; do not pretend this signature exists or is final. Queue should show identifiable alert, tier, reasons and simulation indicator when those fields are available, with empty/loading/disconnected/error states.

## 5. Step-by-step instructions
1. Inspect docs/openapi.json and actual mock routes; identify which queue/feed contracts really exist.
2. Use workspace axum = =0.8.4, tokio = =1.45.1, serde = =1.0.219, serde_json = =1.0.140, tower = =0.5.2 as needed. Prompt suggests askama/htmx but no versions are pinned: request approval before adding; use a simple server-rendered shell and local assets if feasible.
3. Build semantic keyboard-accessible HTML with a readable queue, status, and clear empty/disconnected states. Avoid explanatory slogans or redundant labels.
4. Consume actual mock or agreed fixture contract; distinguish initial load from live updates, deduplicate reconnect deliveries by stable ID, and reconnect without duplicate rows.
5. Protect moderator surfaces via Lead's auth contract. Never expose or log moderator tokens in HTML, URLs or browser storage. Ask Lead for secure same-origin feed authentication.
6. Vendor all used assets; no CDN. Surface simulated results visibly. Ask Lead to mount the router; do not touch server wiring.

## 6. Behavior rules and edge cases
Missing feed or expired authorization must show a clear unavailable state without fake alerts. Reject malformed feed items without breaking the page; preserve existing valid items and reconnect safely. Model absence appears as degraded where the supplied response says so; no invented scoring or tier thresholds.

## 7. Tests you must write
test_queue_empty_state; test_valid_alert_renders_tier_and_reason; test_simulated_badge; test_malformed_event_does_not_break_queue; test_reconnect_deduplicates_id; test_auth_failure_shows_unavailable; test_no_remote_assets.

## 8. Commands
Only use logged recipes. Bash, for a recipe Lead confirms exists:
```sh
just check > target/logs/card-check.log 2>&1; tail -n 40 target/logs/card-check.log; grep -n "error" -A6 target/logs/card-check.log | head -60
```
On Windows use `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 check`; it writes `target/logs/check.log` and `check.exit`. Read the exit file; locally inspect only the last 40 lines and bounded error excerpts. Request missing task-specific logged recipes from Lead; do not pretend an unimplemented recipe tested your work. Never send raw logs in your report.

## 9. Definition of done
- Shell and queue are usable and accessible with verified mock fixtures.
- Feed/auth integration is documented and any missing endpoint remains an explicit blocker.
- No unapproved shared files or dependencies changed.

## 10. Do NOT
Do not edit other owners' crates, root workspace files, generated OpenAPI, or shared configuration; request Lead changes through docs/REQUESTS.md. Do not add dependencies not listed or approved; do not use unwrap/expect/panic in non-test code, retain raw media by default, or commit models, datasets, secrets, or raw IDs. Do not claim proposed or missing integration is implemented.

## 11. How to report back
Report at most ten lines: branch; changed files; interface decisions and Lead agreements; exact logged commands; exit codes and test results; behavior verified; degradation/simulation; incomplete work; dependency requests; questions. Commit verified work using a conventional commit.

## 12. If blocked
Write `BLOCKED.md` inside your owned task directory with the precise question, missing owner/interface, and what is safe to continue; stop dependent work. Send Lead the blocker.
