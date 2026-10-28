# Cross-owner requests

### API text/vision patch ready for Luna (unverified)

API changes are in `crates/hre-api/{src/engine.rs,src/lib.rs,tests/scenarios.rs,tests/media.rs,README.md}`. Actual-byte multipart `/v1/analyze/image` uses fields `request` and `image`; JSON captions only reach text analysis. Removed all production image fixture construction and caption-driven score overrides. Configured asset overrides load only in demo_mode. Text/image/OCR run in spawn_blocking. OCR metadata is explicitly simulated; no media bodies enter API output or persistence; simulation and unsolicited-media provenance reach stored history. Reports reject missing/non-upload events and blank/oversized reasons; SHA-256 comes from upload bytes, not UUID or caption. Approximate matching is disabled pending decoded-pixel hashing; the interim lookup uses a SHA-derived key plus full SHA equality and needs the requested exact store query. Eight new tests cover multipart, privacy, drafts, reports, byte-bound retries, MIME/size/auth, text/PII and OpenAPI; two old caption-fixture scenario tests now assert honest caption-only behavior. Parent/Luna: run logged `format`, `check`, `openapi`, then send only the prescribed summary. This API session ran source inspection and quiet `git diff --check`, not builds/tests or raw log reading; no commit/staging performed. C1/C2/C8/dashboard untouched.

### API call CSV persistence policy (supersedes batch-boundary request)

User explicitly authorized whole-file CSV validation followed by independent per-event engine analysis/persistence, with per-row outcomes and documented partial commits. No batch transaction is required or requested. Delivered `/v1/calllogs/import` accepts JSON platform/conversation_id/csv/draft, validates through hre-calls in spawn_blocking before writes, then returns imported/cached/draft/failed per row. Failed rows do not roll back successful rows or stop later rows. Parser HMAC scope is preserved without double hashing; source simulation enters persisted events and existing behavior/fusion. Six focused tests in `crates/hre-api/tests/call_import.rs` cover malformed-last-row no effects, privacy/simulation/HMAC scope, retries, partial conflicts/continuation, drafts, duplicate/header validation and OpenAPI. Dependency `hre-calls` is present in `crates/hre-api/Cargo.toml`; engine.rs has no doctest snippets to suppress. The reported unresolved hre_calls errors require rerunning against the completed dependency graph. Audio uploads are explicitly rejected; hre-audio is a decoder boundary, not integrated ASR. API patch is ready for parent/Luna logged format/check/openapi; this session ran no noisy checks or raw log reads.

### B4 immediate API handoff to Dirac / Lead / Luna (2026-10-28, verification pending)

Workspace membership was added by the parent. Both library source files are now
present. B3 actual source is `crates/hre-audio/src/lib.rs`; no new external deps.

```rust
pub fn decode_wav(input: &[u8], mime: &str) -> Result<DecodedAudio, AudioError>;
impl DecodedAudio {
    pub fn samples(&self) -> &[f32];
    pub fn metadata(&self) -> AudioMetadata;
}
pub fn unavailable_asr(audio: &DecodedAudio, simulated: bool) -> AudioReport;
pub fn analyze_wav(input: &[u8], mime: &str, simulated: bool)
    -> Result<AudioReport, AudioError>;
```

AudioReport has public Serialize fields metadata, decode_status, status, backend,
transcript, score, simulated, degraded. ASR status=Roadmap/backend=Disabled;
transcript=None/score=None always, degraded includes audio.asr_unavailable and
audio.tone_unavailable. Successful decode alone is Built (Simulated for synthetic
input). Decode errors remain errors. No ASR, tone inference, language prediction,
resampling, model artifacts, subprocesses, files, jobs, streaming or evidence
storage. No text hook is called without actual ASR output.

Accepted audio: RIFF/WAVE little-endian PCM16 mono/stereo, native sample rates
8000,16000,22050,24000,32000,44100,48000. Stereo averaged to normalized mono.
Strict MIME allowlist audio/wav,audio/x-wav,audio/wave (case insensitive, no
parameters), independent magic/container/size/alignment checks. Encoded cap
25 MiB, duration cap 300 seconds, max 1,024 chunks; both caps apply (high-rate
stereo may hit the byte cap before 300s). Decoded mono storage is at most 50 MiB,
plus caller-owned encoded input and allocation overhead. Unknown chunks validated and
discarded. No RF64/RIFX/float/compressed/extensible WAV, duplicate fmt/data,
partial/empty frames, truncation, missing padding, or trailing bytes. DecodedAudio
does not Serialize and Debug prints metadata only. Caller owns/drop-cleans input
and decoded samples; this is not guaranteed memory zeroization. Native-rate PCM
must not be passed to a future 16kHz ASR backend without verified resampling.

Actual source: `crates/hre-calls/src/lib.rs`; crate manifest is present. Add
`hre-calls = { path = "../hre-calls" }` to the API's dependencies. Parent owns
workspace membership/lock refresh; B leaves shared root files untouched to avoid
concurrent edits. Public signatures now available:

```rust
pub fn import_csv(input: &[u8], pseudonymizer: &hre_core::Pseudonymizer,
    platform: &str) -> Result<Vec<CallRecord>, CallError>;
pub fn extract_features(records: &[CallRecord], caller: &hre_core::SubjectId,
    callee: &hre_core::SubjectId, as_of: chrono::DateTime<chrono::Utc>,
    recipient_offset: chrono::FixedOffset) -> Result<CallFeatures, CallError>;
impl CallFeatures { pub fn model_vector(&self) -> [f64; 8]; }
```

CSV requires exactly these nine columns (any order):
`event_id,occurred_at,caller_id,callee_id,duration_s,answered,voicemail,after_refusal,simulated`.
Caller/callee are actual calling/receiving parties, never device-relative incoming/
outgoing labels. UUIDs must be non-nil and unique within each import. RFC3339 times
in 1970..=2100; booleans exactly true/false; unsigned decimal duration <=86,400s.
IDs/platform are trimmed, nonempty, <=256 UTF-8 bytes, no control characters.
IDs are immediately HMACed using the supplied core Pseudonymizer/platform; outputs
and errors retain no raw CSV/identities. Import is all-or-error and performs no I/O.
Supports BOM, LF/CRLF, quoted commas and doubled quotes, but rejects multiline
fields, blank rows, unknown/duplicate headers. Caps: 2 MiB, 10,000 calls, 4,096
bytes/physical line, 256 decoded bytes/field. Header-only import returns no calls.

Use spawn_blocking around import/extraction. Set upload/body limits before buffering.
For Kolkata pass `FixedOffset::east_opt(19_800)`; supply the recipient's offset for
other locations. Windows are inclusive 1h/24h/7d, ending at explicit as_of; future
calls excluded. `CallFeatures::{one_hour,one_day,seven_days,max_calls_1h,
gap_regularity,distinct_callers_24h,recipient_reply_ratio_24h,new_number_flag,
simulated,status}` are public and Serialize. `CallWindow` exposes call_count,
unanswered_calls, short_answered_calls (<5s answered; never acoustic silence),
night_calls (22..05), after_refusal_calls, voicemail_count, answered_duration_s,
avg_duration_s, unanswered_ratio, night_call_ratio. `new_number_flag` is always
None: no identity linking. Refusal flags are source-supplied, not independently
verified. Model vector follows the eight proposed MODEL_CONTRACT names, raw f64;
no model inference, risk score, thresholds or trained missing-history policy.
Distinct callers requires complete target history; reverse ratio is reverse /
(forward+reverse), zero when empty. Any contributing synthetic record makes the
feature status Simulated and simulated=true. Do not infer known safety from absent
history. API must propagate simulation from imports to persisted assessments.

API integration remains Lead-owned: auth, history merge, refusal-source validation,
cross-import idempotency, caller authorization, drafts (no writes/alerts), storage,
fusion, recordings/jobs and route wiring. Parsing alone does not activate these.
Luna: both lib.rs and tests.rs files are present, ready for logged format/check.
No builds/tests/downloads were run in the B session (no Luna delegation tool).
Parent owns verification, Cargo.lock refresh and commits. Exact existing recipes:
`powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 format`, then
`powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 check`.
Luna must inspect their target/logs/<recipe>.exit/log and summarize <=15 lines.
Call tests cover strict CSV quoting/encoding/header/row validation, atomic errors,
HMAC privacy/platform scoping, 10,000-record boundary, duplicate IDs, invalid public
records, golden vector, inclusive/future windows, night-hour boundaries and
simulation across contributing directions/target history. Audio tests cover PCM
normalization/stereo/native rates, all documented unavailable outcomes, malformed
chunks/format/length/padding, byte/duration/chunk caps and arbitrary-byte plus PCM
roundtrip properties. These tests are written, not yet verified by this session.

### API integration coordination (2026-10-28)

- Sartre (`01a1827d-a5b4-7400-9edb-f0d039283ada`): please publish delivered `hre-calls` CSV parser signatures and readiness here. API owner will integrate `/v1/calllogs/import` once the crate and workspace dependency exist. No ASR or fabricated transcript scoring will be added.
- Vision owner: `compute_phash` samples encoded bytes, not decoded pixels; API disables approximate matching. Metadata extraction is not OCR inference; API labels contributing metadata extraction simulated. Please provide decoded-pixel hashing and honest OCR status upstream.
- Store owner: please add `Store::find_reported_sha256(&str) -> Result<Option<ReportedHashMatch>, StoreError>` for exact matching. Until delivered API checks full SHA after a zero-distance lookup and never matches on the approximate hash alone.
- Parent/Luna: API owner has no agent delegation tool. Please run logged format/check after API handoff, inspect logs, and return at most 15 summary lines. No build/test logs are read in this session.

Snapshot: 2026-10-28. These requests are pending work records for the parent/human teams; no external message or implementation is implied.

| ID | To | Concrete request | Needed for | State |
|---|---|---|---|---|
| REQ01 | Lead | Supply generated core DTO/OpenAPI paths and actual mock routes plus schema verification evidence | Contract freeze; truthful endpoint docs | Pending |
| REQ02 | Lead | Resolve wire Status enum conflict: roadmap versus disabled; represent missing analysis separately from low risk | F1–F21; schema compatibility | Pending |
| REQ03 | Lead | Report actual fallback adapters, reason codes, model-health/reload behavior and supported routes | Partial Built updates without teammate overclaim | Pending |
| REQ04 | Lead/Luna | Supply exact logged check/demo commands, exit outcomes and artifact paths; explain unresolved network/build errors | Completion and status evidence | Pending |
| REQ05 | A | Implement text normalization/language/PII/cues/sentiment, slang gate and eval; provide tests and supported language slices | F1/F8/F10/F13; C3/C14 | Human-team pending |
| REQ06 | A/Lead | Publish docs/FEATURES.md names/order/formulas, Python/Rust golden vectors and benign baseline | F3/F16; model parity within 1e-4 | Pending |
| REQ07 | A/human | Provide licensed dataset/version records, generators A/B, train/validation/test manifests and training guide | DATA/TRAINING; Deliverable5 | Pending |
| REQ08 | B | Implement media validation, NSFW/hash/OCR, audio/ASR, calls/CSV/jobs and explicit unavailable behavior | F5–F7/F14–F18/F21 | Human-team pending |
| REQ09 | B | Select exact model artifacts/revisions and verify licenses including base terms; supply real OCR/ASR metrics | Model activation; regional media claims | Pending; no downloads authorized here |
| REQ10 | C | Implement dashboard/evidence/connectors/CLI with auth, simulation display and real action evidence | F12; Deliverable4; S9 | Human-team pending |
| REQ11 | C/Lead | Define canonical JSON/export signature/key verification and access scope; verify certificate wording | Tamper verification; legal draft aid | Pending |
| REQ12 | Lead | Supply HMAC scoping, retention defaults, temp cleanup, PII masking and draft/idempotency test evidence | C11; security/privacy claims | Pending |
| REQ13 | C/human | Verify selected platform login flow/scopes/quotas and owned-space participant consent before enabling connector | Platform appendix; permissions | Pending |
| REQ14 | Parent | Send a completion update with paths/symbols, commands/outcomes and remaining gaps | Update TRACEABILITY actual status and checkboxes | Pending |
| REQ15 | Parent | Ensure API.md, FEATURES.md, RUNBOOK.md, DEMO_SCRIPT.md, root docs and cards are authored by assigned owner | Prompt deliverables beyond this sidecar | Pending; outside sidecar edit scope |

| REQ16 | Lead sidecar Aristotle | Add efficient latest-refusal lookup and atomic event/assessment persistence, plus durable alert and feed-protection queries; support recovery and idempotent retries across server instances | Refusal history, restart recovery, reliable alert delivery | Pending |
| REQ17 | Lead sidecar Aristotle | Handle contacts at the same timestamp as a refusal consistently with their after_refusal flag, and preserve independent mute/stop state when a block is lifted | Accurate refusal scoring | Pending |

No request permits editing a different owner's crate. Continue lead walking-skeleton work through lead adapters while human implementations remain pending. No accounts, tokens or recordings are requested from the user by this document.

| REQ18 | Lead sidecar Aristotle | Preserve older reverse-direction refusal markers in the reverse harassment guard; its current 24-hour filter drops them before feature extraction | Correct reply-to-harasser guard after old refusal | Implemented; awaiting Luna verification and API integration |

### REQ18 sidecar handoff (unverified)

- `hre_behavior::analyze` retains past refusal markers of any age for reverse feature extraction while retaining the 24-hour contact window and excluding the pending reply. Added regression coverage for old blocks, unblocks, independent mute state, future markers and old contacts. Luna must run the logged checks; API must supply markers in both directions as described in REQ16/REQ17.

### REQ16/REQ17 sidecar handoff (unverified)

- `Store::latest_refusal_markers(sender, recipient, to_ms)` returns at most one latest marker per block, unblock, mute and stop_request, using a channel/time index. Resolve block against unblock by `(timestamp_ms, event_id)`; mute and stop_request remain independent. Load markers for both pair directions, including markers older than seven days, and include them with the window passed to behavior. API's existing `refusal_active` must be replaced accordingly; do not let unblock clear mute or stop.
- `Store::persist_assessment(&event, &assessment, alert, protect_feed)` commits the event, masked assessment, optional masked alert and feed protection together. On an existing identical event it returns the original assessment and does not repeat effects; compare the request hash before returning that response. A pre-existing event without an assessment can be completed. Existing legacy `insert_event`/`save_assessment` do not have these guarantees. Do not call for drafts. `Store::alerts()` and `Store::feed_protected(recipient)` read durable state; the API must propagate storage errors and pseudonymize recipient IDs.
- Persisted alert records are queryable, not a delivery acknowledgement or durable SSE outbox. Instances can score different events from stale snapshots before either transaction commits; serialize the read/score/write cycle across instances or design an optimistic retry before claiming cross-instance ordering. Retry publication requires an explicit delivery protocol. Protection currently only activates; deactivation and review lifecycle are not implemented. Assessment/alert JSON must be masked by the API before storage.
- Behavior uses the contact's `after_refusal` flag at the same timestamp as the active refusal and retains independent mute/stop state after unblock. Older refusal markers must be supplied explicitly. Tests were added but require Luna execution; REQ16 and REQ17 remain pending API integration and verification.


| REQ19 | Lead/Luna | Evaluate and pin optional `ort = =2.0.0-rc.10` (default features disabled, `load-dynamic`), provision a compatible ONNX Runtime 1.22 library, and add logged offline-capable feature-on/feature-off build and test recipes runnable by Luna; coordinate behavior consumer injection separately | Verified optional behavior ONNX backend without implicit runtime downloads | Pending |

### REQ19 ML assessment

- `ort` is absent from workspace dependencies, Cargo.lock and the local Cargo source cache. rc.10 documents Rust 1.81 and ONNX Runtime 1.22; its default features include binary downloads, while `load-dynamic` supports an externally provisioned runtime. This is a candidate, not an existing pin or verified local build.
- Supply contract-compatible message model and exporter evidence: frozen 47-feature order matching `hre_behavior::feature_order()`, a golden feature vector, labels, license, preprocessing, probability/range and numerical parity evidence. For calls, obtain owner agreement on the proposed 8-feature order and missing-value policy plus golden vectors before activation.
- Integrate runtime selection with the behavior owner: current behavior analysis constructs the heuristic directly. Preserve deterministic fallback on missing/invalid artifacts or runtime, verify canonical path and SHA-256, parse the model, validate exact fixed input/output metadata and manifest labels, and distinguish artifact verification, runtime readiness and actual inference. Hash verification alone must not report ONNX active.
- This session has no available Luna delegation tool; no builds, tests, downloads or runtime checks were performed. Existing `hre-ml` tests cover missing model and a hash-matching malformed artifact remaining heuristic; they do not prove runtime parsing or feature-enabled behavior.

| REQ20 | Lead | Integrate `crates/hre-text` into workspace and engine API | Full workspace integration of text analysis; replace explicit-threat shortcut in API engine | Implemented and integrated on lead/engine-foundation |

### REQ20 A3 text pipeline handoff (verified)

- Card A3 `crates/hre-text` implemented and integrated into workspace `Cargo.toml`, `crates/hre-api`, and `crates/hre-vision`:
  - Normalization: homoglyph mapping, whitespace normalization, repeated characters compression, zero-width space filtering.
  - PII Detection & Masking: Indian phone (+91 / 10-digit 6-9), email, Aadhaar (Verhoeff checksum validated), PAN, UPI, and Indian PIN codes (near address cues).
  - Cues: threat, coercion, location stalking, quoted-abuse hostility discounting.
  - Classification: `HeuristicTextBackend`, `TextBackend` trait, `TextReport`, `analyze` entrypoint.
  - All unit, integration, and proptest property tests verified passing with zero warnings.

| REQ21 | Lead | Review Python synthetic generator A and 47-feature extraction parity fixtures | Parity verification between Python generator A and Rust `hre-behavior` | Implemented on lead/engine-foundation; all 7 Python parity tests passing |

### REQ21 A4 Synthetic Generator A handoff (verified)

- Implemented `training/src/hre_training/generator_a.py`, `features.py`, CLI `training/generator_a.py`, and test suite `training/tests/test_generator_a.py`:
  - 14 scenario classes generated deterministically with seeded RNG (normal, heavy_benign, polite_stalker, escalating_stalker, post_block_contacter, pile_on, bully, cyberflasher, sextortion, silent_caller, night_caller, post_block_caller, new_number_caller, noisy_ambiguous).
  - 47-feature vector extraction exactly matches `docs/FEATURES.md` and `hre-behavior::extract_features` within 1e-4 tolerance.
  - Golden vector fixture exported to `training/fixtures/golden_vector.json` and 14 scenario samples exported to `training/fixtures/synthetic_samples.json`.
  - All 7 tests passing in 0.014s (`test_same_seed_same_events`, `test_seed_changes_sample`, `test_chronology_and_matching_payloads`, `test_healthy_reciprocity_and_refusal_sequence`, `test_no_real_identifiers`, `test_golden_vector_order_and_values`, `test_generator_a_training_only`).

| REQ22 | Lead | Review dataset provenance inventory and license admission rules in `training/datasets.json` | Provenance tracking, opt-in download gate, and train/eval split independence | Implemented on main; 6 unit tests passing |

### REQ22 A1 Dataset provenance inventory handoff (verified)

- Implemented `training/datasets.json`, `training/src/hre_training/inventory.py`, and `training/tests/test_inventory.py`:
  - Cataloged candidates: Jigsaw (CC0-1.0), HateXplain (MIT), Davidson (MIT), TextDetox (Apache-2.0), RAVDESS (CC-BY-NC-4.0 restricted), CREMA-D (ODbL-1.0), Synthetic Generator A (train split), and Hand-Authored Regional Benchmark (eval_held_out split).
  - Enforced offline-first operation with patch-isolated network access tests.
  - Enforced strict opt-in download gates restricted to `data/raw/` to protect against unauthorized media downloads.
  - Verified training and evaluation split partitions are strictly disjoint.
  - 6 passing unit tests in 0.007s.

| REQ23 | Lead | Review ONNX model export contracts, label ordering, and parity verification in `training/src/hre_training/export_onnx.py` | Model registry integration contracts, dynamic batch axes, and parity bounds | Implemented on main; 7 unit tests passing |

### REQ23 A2 ONNX export contracts and parity handoff (verified)

- Implemented `training/src/hre_training/export_onnx.py` and `training/tests/test_export_onnx.py`:
  - Defined verified tensor contracts for multi-label toxicity (7 Detoxify labels in canonical order), multilingual toxicity (toxic binary), large binary toxicity (neutral/toxic), sentiment (negative/neutral/positive), and sentence embeddings (dim 384).
  - Enforced dynamic batch axes (`[batch_size, sequence_length]`) on inputs and dynamic `batch_size` on logits.
  - Built license gate: unverified or unknown licenses (e.g. Cardiff sentiment) strictly block export.
  - Built explicit missing-artifact fallback: missing local model weights produce honest degradation skips without fabricating artificial ONNX inference.
  - Enforced 1e-4 absolute numerical parity bound.
  - Verified zero binary model weights (`*.onnx`, `*.bin`, `*.safetensors`, `*.pt`) committed to git.
