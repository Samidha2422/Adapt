# C2: Alert details, review actions, timeline and pairs

## 0. Your role in one paragraph
You are teammate C. Extend the dashboard with alert details, review actions and pair timelines. The board places this in P2 with dependencies C1 and L6; this card is prepared early at the human's request.

## 1. Project context
The Harassment Risk Engine is a Rust HTTP service that scores messages, images, audio, and call events using content and directional conversation history. Demo applications, dashboards, and bots are its clients.
IDs are pseudonymized with a server HMAC salt before storage.
Media is processed transiently; retain scores, masked transcripts, OCR text, and hashes rather than uploads.
Analyze only consented material and keep human moderators responsible for decisions.
Every feature is Built, Simulated, or Roadmap; synthetic data, benign proxies, and demo overrides must be disclosed, and any simulated contribution sets simulated=true.

## 2. Repo map and your boundary
Branch c-surfaces/dashboard-detail. Edit crates/hre-dashboard/ only. Lead owns alert/review/timeline APIs, auth, store, server routing and shared contracts; request missing functionality.

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
From `crates/hre-core/src/types.rs`:
```rust
wire_enum!(ReviewDecision { Confirm, Reject, Escalate });
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReviewRequest { pub decision: ReviewDecision, pub note: Option<String> }
```
Prompt describes GET /v1/alerts/{id}, POST /v1/alerts/{id}/review, and GET /v1/pairs/{pair}/timeline. Their exact response schemas and existence must be verified in actual OpenAPI and Lead handoff. C1 and L6 are not established complete.

## 4. Exact deliverables
Add alert detail, review controls, pair and timeline views and tests in `crates/hre-dashboard/`. **Proposed interface pending Lead agreement**:
```rust
pub fn detail_router() -> axum::Router;
```
Route names, state, server auth/CSRF and timeline models require agreement; do not invent endpoint response structs or pretend the router is already mounted.

## 5. Step-by-step instructions
1. Check C1 and L6 implementation and generated contract; record missing endpoints or fields.
2. Reuse C1's approved dependencies and workspace axum = =0.8.4, serde = =1.0.219 and tokio = =1.45.1; do not add unpinned libraries.
3. Render reason codes and human-readable explanation, signal scores/status/backend and simulation/degraded state from real responses. Show occlusion only when actually supplied; never call it SHAP.
4. Render chronologically ordered timeline with pagination or explicit limits as supported by Lead's API. Avoid exposing raw media/identities and handle masked content.
5. Submit Confirm/Reject/Escalate using the actual ReviewRequest above. Require moderator authorization and Lead-approved CSRF defense; show success only after server confirmation.
6. Provide keyboard accessible controls, loading/empty/error states and retry on network failure.

## 6. Behavior rules and edge cases
Do not fabricate data when endpoints or models are unavailable; show an actionable unavailable state. Reject malformed IDs and unauthorized review attempts. Duplicate submissions must follow the server's actual semantics; do not assert idempotency without a contract. Review is human decision-making, not an automated verdict.

## 7. Tests you must write
test_detail_renders_reason_and_simulated_label; test_unknown_alert_not_found; test_review_serializes_decision_and_optional_note; test_unauthorized_review_is_not_success; test_timeline_empty_and_ordered; test_malformed_payload_safe; test_network_failure_preserves_form.

## 8. Commands
Only use logged recipes. Bash, for a recipe Lead confirms exists:
```sh
just check > target/logs/card-check.log 2>&1; tail -n 40 target/logs/card-check.log; grep -n "error" -A6 target/logs/card-check.log | head -60
```
On Windows use `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 check`; it writes `target/logs/check.log` and `check.exit`. Read the exit file; locally inspect only the last 40 lines and bounded error excerpts. Request missing task-specific logged recipes from Lead; do not pretend an unimplemented recipe tested your work. Never send raw logs in your report.

## 9. Definition of done
- Detail, review and timeline work against verified C1/L6 contracts or are explicitly blocked.
- Moderator authorization and failure states are tested.
- All rendered claims reflect actual response data.

## 10. Do NOT
Do not edit other owners' crates, root workspace files, generated OpenAPI, or shared configuration; request Lead changes through docs/REQUESTS.md. Do not add dependencies not listed or approved; do not use unwrap/expect/panic in non-test code, retain raw media by default, or commit models, datasets, secrets, or raw IDs. Do not claim proposed or missing integration is implemented.

## 11. How to report back
Report at most ten lines: branch; changed files; interface decisions and Lead agreements; exact logged commands; exit codes and test results; behavior verified; degradation/simulation; incomplete work; dependency requests; questions. Commit verified work using a conventional commit.

## 12. If blocked
Write `BLOCKED.md` inside your owned task directory with the precise question, missing owner/interface, and what is safe to continue; stop dependent work. Send Lead the blocker.
