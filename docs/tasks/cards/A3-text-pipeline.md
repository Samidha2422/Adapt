# A3: Text normalization, classification, PII and cue detection

## 0. Your role in one paragraph
You are teammate A. Build text analysis with deterministic model-free behavior, preserving the difference between heuristic and actual ONNX inference. P1 depends on L1 and A2; A2 is not yet established.

## 1. Project context
The Harassment Risk Engine is a Rust HTTP service that scores messages, images, audio, and call events using content and directional conversation history. Demo applications, dashboards, and bots are its clients.
IDs are pseudonymized with a server HMAC salt before storage.
Media is processed transiently; retain scores, masked transcripts, OCR text, and hashes rather than uploads.
Analyze only consented material and keep human moderators responsible for decisions.
Every feature is Built, Simulated, or Roadmap; synthetic data, benign proxies, and demo overrides must be disclosed, and any simulated contribution sets simulated=true.

## 2. Repo map and your boundary
Branch a-text/text-pipeline. Edit crates/hre-text/ only. Lead owns core, model registry, API, shared workspace and configuration. Request required interfaces and dependencies rather than editing their crates.

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
`hre-ml::InferenceBackend` currently accepts numeric feature slices, not text or tokenized inputs:
```rust
pub trait InferenceBackend: Send + Sync {
    fn predict(&self, features: &[f64]) -> Result<f64, ModelError>;
    fn backend(&self) -> Backend;
}
```
A2 export contract and tokenizer interface are pending; verified model hash does not imply active inference.

## 4. Exact deliverables
Create `crates/hre-text/Cargo.toml`, `src/lib.rs`, normalizer, PII and cue modules and tests. **Proposed interface pending Lead agreement**:
```rust
pub trait TextBackend: Send + Sync {
    fn analyze(&self, text: &str) -> Result<hre_core::Signal, TextError>;
}
pub fn normalize(text: &str) -> String;
```
Agree typed multi-label results, degradation propagation, language detection/tokenizer and PII outputs with Lead; do not treat this sketch as an existing contract.

## 5. Step-by-step instructions
1. Read prompt sections 6, 8 and 11; inspect A2's actual exported metadata and Lead's interfaces.
2. Use existing serde = =1.0.219, serde_json = =1.0.140, thiserror = =2.0.12 as needed. Regex, language identification and tokenizer dependencies have no approved pins: request exact versions before adding.
3. Normalize Unicode/homoglyph/spacing/repeated characters conservatively; preserve enough context to identify quoted abuse and avoid treating every quote as intent.
4. Implement deterministic text fallback with explicit heuristic backend and missing-model degradation; no external model is required for baseline.
5. Detect and mask email, Indian phone patterns, Aadhaar-like numbers only with Verhoeff validation, PAN, UPI, and PIN near address cues; distinguish matched PII from toxic context.
6. Add clear, bounded threat/coercion/location cues; leave sextortion as simulated pattern signal, not a verdict.
7. Wire tokenizer/ONNX only after A2 and Lead agree on real runtime, labels and inputs.

## 6. Behavior rules and edge cases
Empty or oversized text must produce an explicit bounded error consistent with AnalyzeRequest validation (max 32768 bytes). Missing model chooses deterministic fallback and reports text.onnx_missing. Text scores stay finite in 0..1; don't claim ONNX when unavailable. PII outputs must be masked and no sensitive text logged.

## 7. Tests you must write
test_normalization_unicode_and_spacing; test_missing_model_degraded; test_indian_phone_and_email_masked; test_aadhaar_requires_verhoeff; test_pin_needs_address_context; test_quoted_abuse_not_auto_verdict; test_bounded_invalid_text; property_normalizer_never_panics after approved proptest.

## 8. Commands
Only use logged recipes. Bash, for a recipe Lead confirms exists:
```sh
just check > target/logs/card-check.log 2>&1; tail -n 40 target/logs/card-check.log; grep -n "error" -A6 target/logs/card-check.log | head -60
```
On Windows use `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 check`; it writes `target/logs/check.log` and `check.exit`. Read the exit file; locally inspect only the last 40 lines and bounded error excerpts. Request missing task-specific logged recipes from Lead; do not pretend an unimplemented recipe tested your work. Never send raw logs in your report.

## 9. Definition of done
- Model-free analysis and PII masking are verified.
- A2/Lead runtime integration is either actually tested or explicitly pending.
- No raw sensitive content in logs.

## 10. Do NOT
Do not edit other owners' crates, root workspace files, generated OpenAPI, or shared configuration; request Lead changes through docs/REQUESTS.md. Do not add dependencies not listed or approved; do not use unwrap/expect/panic in non-test code, retain raw media by default, or commit models, datasets, secrets, or raw IDs. Do not claim proposed or missing integration is implemented.

## 11. How to report back
Report at most ten lines: branch; changed files; interface decisions and Lead agreements; exact logged commands; exit codes and test results; behavior verified; degradation/simulation; incomplete work; dependency requests; questions. Commit verified work using a conventional commit.

## 12. If blocked
Write `BLOCKED.md` inside your owned task directory with the precise question, missing owner/interface, and what is safe to continue; stop dependent work. Send Lead the blocker.
