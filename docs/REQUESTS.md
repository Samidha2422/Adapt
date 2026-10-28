# Cross-owner requests

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
