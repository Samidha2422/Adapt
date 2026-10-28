# A4: Synthetic generator A and behavior golden vectors

## 0. Your role in one paragraph
You are teammate A. Build reproducible synthetic training conversations and call histories and make the feature contract testable across Python and Rust. This is P1; L1 exists, but the shared feature definitions and golden vectors have not been agreed yet.

## 1. Project context
The Harassment Risk Engine is a Rust HTTP service that scores messages, images, audio, and call events using content and directional conversation history. Demo applications, dashboards, and bots are its clients.
IDs are pseudonymized with a server HMAC salt before storage.
Media is processed transiently; retain scores, masked transcripts, OCR text, and hashes rather than uploads.
Analyze only consented material and keep human moderators responsible for decisions.
Every feature is Built, Simulated, or Roadmap; synthetic data, benign proxies, and demo overrides must be disclosed, and any simulated contribution sets simulated=true.

## 2. Repo map and your boundary
Branch a-text/synthetic-generator. Edit training/ only. FEATURES.md is shared and currently owned by the Lead sidecar; deliver a proposed patch or vectors to Lead through a request rather than editing it without agreement. Do not edit hre-core, hre-store, hre-ml, hre-behavior, hre-fusion, hre-api, or B/C crates.

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
`docs/FEATURES.md` currently only states that store times are UTC milliseconds, windows are inclusive, ordered timestamp then event ID, exclude future events, and IDs stored must be HMAC pseudonyms. It offers proposed store/behavior/fusion entrypoints, not a frozen feature vector. No golden-vector definitions were present when this card was written. A1 card exists; A1 implementation is not established.

## 4. Exact deliverables
Create `training/src/hre_training/generator_a.py`, `training/tests/test_generator_a.py`, and versioned synthetic fixtures under `training/`; propose a feature-spec patch and golden vectors to Lead. Python task: no Rust public API is required. Proposed serialized event mapping is **pending Lead agreement**: align to the actual AnalyzeRequest above, using synthetic identities and call/message variants; do not silently invent fields. Agree ordered feature names, formulas, empty-window defaults, units, time zone, boundary inclusion, and vector schema with Lead before freezing values.

## 5. Step-by-step instructions
1. Read ENGINE_PROMPT sections 9.1–9.3 and 14; inspect A1's current training project before reusing it.
2. Use Python standard library only for generation and tests unless Lead approves another dependency; no unverified package versions or downloads.
3. Seed a local RNG explicitly. Generate normal, heavy_benign, polite_stalker, escalating_stalker, post_block_contacter, pile_on, bully, cyberflasher, sextortion, silent_caller, night_caller, post_block_caller, new_number_caller and noisy/ambiguous examples. Use invented content and IDs.
4. Produce message and call event timestamps, direction, replies, refusal chronology and explicit synthetic labels. Cover reciprocity and after-block behavior; never infer cross-platform identity.
5. Propose feature formulas covering message and call lists in section 9, including 1h/24h/7d, nighttime timezone, ratios with zero denominator, missing data, and ordered output; have Lead resolve ambiguity.
6. Hand-calculate small fixtures with expected vectors; include exact timestamps and feature order. Arrange an independent Rust consumer to compare within 1e-4 once the Lead contract lands.
7. Keep evaluation generator B independent in parameter ranges and structure; do not use generator A fixtures as final held-out evaluation.

## 6. Behavior rules and edge cases
Invalid configuration, invalid times or negative sizes must produce explicit errors. Same seed/config must reproduce outputs; do not label synthetic samples as real. Missing models must not be required. Treat window boundaries and night rules as unresolved until Lead agrees; do not invent thresholds.

## 7. Tests you must write
test_same_seed_same_events; test_seed_changes_sample; test_chronology_and_matching_payloads; test_healthy_reciprocity_and_refusal_sequence; test_no_real_identifiers; test_golden_vector_order_and_values after Lead freezes formulas; test_generator_a_training_only demonstrates no evaluation overlap.

## 8. Commands
Only use logged recipes. Bash, for a recipe Lead confirms exists:
```sh
just check > target/logs/card-check.log 2>&1; tail -n 40 target/logs/card-check.log; grep -n "error" -A6 target/logs/card-check.log | head -60
```
On Windows use `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 check`; it writes `target/logs/check.log` and `check.exit`. Read the exit file; locally inspect only the last 40 lines and bounded error excerpts. Request missing task-specific logged recipes from Lead; do not pretend an unimplemented recipe tested your work. Never send raw logs in your report.

## 9. Definition of done
- Seeded message and call datasets and tests exist and identify synthetic provenance.
- Proposed feature spec and golden vectors are sent to Lead; cross-language comparison is passing only after integration.
- Explicitly report any unresolved Lead agreement.

## 10. Do NOT
Do not edit other owners' crates, root workspace files, generated OpenAPI, or shared configuration; request Lead changes through docs/REQUESTS.md. Do not add dependencies not listed or approved; do not use unwrap/expect/panic in non-test code, retain raw media by default, or commit models, datasets, secrets, or raw IDs. Do not claim proposed or missing integration is implemented.

## 11. How to report back
Report at most ten lines: branch; changed files; interface decisions and Lead agreements; exact logged commands; exit codes and test results; behavior verified; degradation/simulation; incomplete work; dependency requests; questions. Commit verified work using a conventional commit.

## 12. If blocked
Write `BLOCKED.md` inside your owned task directory with the precise question, missing owner/interface, and what is safe to continue; stop dependent work. Send Lead the blocker.
