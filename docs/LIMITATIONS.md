# Limitations and delivery gaps

Snapshot: 2026-10-28. **The implementation is incomplete.** Luna reports `format`, `check`, `build`, `openapi` and `demo-seed` passed after the latest source changes, with 58 tests and zero warnings. Real-mode synthetic scenarios 1, 2 and 7 returned high/85, high/73 and low/0 respectively. ONNX verification fixture tests passed with zero skips; see [verification records](MODELS.md#artifact-admission).

## Honest feature status

| Feature | Intended status | Actual delivery baseline | Exact gap |
|---|---|---|---|
| F1 Message toxicity and sentiment | Built | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F2 Directional conversation memory; 1h/24h/7d windows | Built | Roadmap / partial | Lead SQLite windows implemented; full acceptance pending |
| F3 Persistence/escalation, attribution and evasion suspicion | Built; evasion Simulated | Roadmap / partial | Lead rules and occlusion implemented; S1 synthetic result reported; evasion and full acceptance pending |
| F4 Pile-on, distinct senders, spike and coordination | Built | Roadmap / partial | Lead distinct-sender threshold implemented; S2 synthetic result reported; coordination and full acceptance pending |
| F5 NSFW screening and reported-image hash match | Built | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F6 Low-confidence face/deepfake/NSFW composite | Simulated | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F7 Audio decode, resample, ASR and text scoring | Built | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F8 Indian-aware PII detection and masking | Built | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F9 Noisy-OR fusion, floors, caps, tiers and reasons | Built | Roadmap / partial | Lead rules implemented; full end-to-end acceptance pending |
| F10 Moderator-gated slang mining, adapter and rollback | Simulated temporal experiment | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F11 Pre-send nudge without persistence | Built | Roadmap / partial | Lead draft path implemented; full acceptance pending |
| F12 Evidence timeline, chain and JSON/HTML export | Built; PDF stretch | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F13 Independent evaluation and reports | Built | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F14 OCR text and PII pipeline | Built | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F15 Voice-note transcript and supporting tone | Built; tone Simulated | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F16 Call-log import and directional behavior features | Built on synthetic/consented logs | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F17 Recorded-call/voicemail asynchronous evidence | Built | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F18 Prepared audio streamed as simulated live call | Simulated | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F19 Image-caption hateful meme screening | Roadmap; behind-flag prototype requested | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F20 Low-confidence synthetic-voice screening | Roadmap; behind-flag prototype requested | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F21 Short video frame sampling plus audio | Simulated | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |

The deterministic lead adapter supplies a minimal threat fallback and rules-based behavior and fusion. It does not complete teammate text, OCR, ASR, media screening, trained inference, evidence, dashboard or evaluation modules. Canned mock outputs and benign-proxy overrides are simulations; they are not model-quality measurements.

## Explicit research Roadmap rows

| Problem | Why it remains out of implementation scope |
|---|---|
| 1.3 Group exclusion | Requires membership changes and context the engine does not currently receive |
| 1.4 Impersonation | Profile similarity and trusted identity evidence are absent |
| 1.7 Indirect harassment | Target attribution and mention graphs are unresolved |
| 2.4 Surveillance | Profile/story-view and location telemetry are not ingested |
| 2.6 Spyware/account compromise | Login/device security is outside message scoring |
| 2.8 Third-party contact | No consented victim-network graph |
| 3.6 Grooming | Reliable age/trajectory data and safe validation unavailable; human safeguarding review only |
| 5.8 Live telephone-call analysis | No OS/operator integration; uploaded recordings and prepared WS simulation are distinct |

F19/F20 remain Roadmap even if a disabled extension is reserved; no behind-flag implementation is verified. F21 is only an intended short-video simulation. API video envelope support must be decided; the input kinds do not currently specify a standalone video kind.

## Unresolved implementation and evidence gaps

- Core DTOs, mock mode, auth, HMAC, persistence, idempotency and draft paths exist in source. SQLite atomically stores event, assessment, optional alert and feed protection; retries return the stored assessment and conflicting requests are rejected. Alerts and protection are durable, while SSE delivery is process-local and not a durable queue.
- The registry accepts shared contract entries and checks hashes; ONNX inference, tokenizer runtime and model reload are unavailable. Passing verifier tests do not verify actual model artifacts or inference.
- Behavior features, victim-reply guard, occlusion, pile-on and fusion rules exist; the three synthetic scenario results above are reported, but full acceptance and model parity remain pending. Full call features need B's implementation.
- A/B/C human crates, data generators, training outputs, the remaining six real API scenarios, UI enforcement, connector permissions and verifiable evidence export are pending.
- No measured precision, recall, F1, FPR, detection rate, time-to-detection, OCR CER, ASR WER, kappa, ablation or latency results are recorded. Reports must not contain invented zeros or successes.
- Exact model/dataset revisions, artifact licenses, conversion compatibility and regional language quality remain candidate-specific unknowns unless official observations are recorded in MODELS.
- Retention periods, upload deletion/backup erasure, raw-ID/PII audit, tenant/subject authorization and signing-key management remain unverified.
- API and feature documents describe the lead subset; teammate cards and other companion documentation may still need reconciliation.

## Limits even after the PoC works

Synthetic A/B behavior data may not generalize. Fresh test text reduces but cannot disprove pretrained leakage. pHash only covers known reported test images and can collide or miss edits; it does not determine consent. NSFW does not establish harassment. Deepfake/voice detection, sarcasm, reclaimed language, code-mix, OCR and transcription are fallible. Sextortion cues are uncertain. Tone cannot establish intent.

No arbitrary external private-chat access, cross-platform identity linking, covert call capture, reliable grooming detection or replacement for human moderators is claimed. A high score is a review priority, not a finding of wrongdoing. Evidence hashes/HMAC are not court certification. Legal/platform assertions require current official sources and deployment-specific review.

## Scope and updates

The matrix retains 81 unchecked required rows, 19 platform rows and 45 roadblock rows. Update actual status only with source and acceptance evidence. See REQUESTS for concrete handoffs.

## Source-check findings — 2026-10-28

Seven exact publisher model cards were accessible and their declared license metadata is recorded in MODELS; none is cleared for activation or locally tested. The requested multilingual Unitary config has a single toxic output, not seven harassment labels, and its card does not establish Hindi/Marathi/Konkani coverage. A must resolve label and language suitability before implementation claims.

Official sources were recorded for staged DPDP Rules commencement, Discord/Telegram message access, Google Play Call Log restrictions, WCD support resources and StopNCII's participating-platform scope. BSA certificate text, 1930/anonymous-reporting details, recording law and unselected platform integrations remain unverified; several official fetches returned empty content or timed out. These are verification gaps, not proof that a model, service or legal provision does not exist.
