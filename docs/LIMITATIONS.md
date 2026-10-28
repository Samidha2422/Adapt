# Limitations and delivery gaps

Snapshot: 2026-10-28. **The implementation is incomplete.** Main `1f8d420` includes A1–A4/B1–B2; calls/audio fallback/eval, API uploads and audit tests are concurrent changes. Historical foundation reports (58 Rust tests, 11 verifier fixture tests and S1/S2/S7 results) do not verify this snapshot. Current commands and acceptance are pending in [ENGINE_VERIFICATION.md](ENGINE_VERIFICATION.md).

## Honest feature status

| Feature | Intended status | Actual delivery baseline | Exact gap |
|---|---|---|---|
| F1 Message toxicity and sentiment | Built | Roadmap / partial | Main text normalization, heuristic classification and sentiment/cues exist; no active ONNX inference or measured language quality |
| F2 Directional conversation memory; 1h/24h/7d windows | Built | Roadmap / partial | Lead SQLite windows implemented; full acceptance pending |
| F3 Persistence/escalation, attribution and evasion suspicion | Built; evasion Simulated | Roadmap / partial | Lead rules and occlusion implemented; S1 synthetic result reported; evasion and full acceptance pending |
| F4 Pile-on, distinct senders, spike and coordination | Built | Roadmap / partial | Lead distinct-sender threshold implemented; S2 synthetic result reported; coordination and full acceptance pending |
| F5 NSFW screening and reported-image hash match | Built | Roadmap / partial | Main vision interfaces plus working-tree upload path; unavailable real NSFW classifier; encoded-byte hash proxy is not perceptual matching |
| F6 Low-confidence face/deepfake/NSFW composite | Simulated | Roadmap / partial | Interfaces/composite audit in progress; no verified face/deepfake inference, proxy composite acceptance or generalization |
| F7 Audio decode, resample, ASR and text scoring | Built | Roadmap / partial | Working-tree bounded PCM16 WAV decoding and unavailable-ASR report; no genuine ASR, waveform-to-transcript measurement or verified API audio path |
| F8 Indian-aware PII detection and masking | Built | Roadmap / partial | Main regex/validation/masking implementation; concurrent privacy audit; current integration and coverage verification pending |
| F9 Noisy-OR fusion, floors, caps, tiers and reasons | Built | Roadmap / partial | Lead rules implemented; full end-to-end acceptance pending |
| F10 Moderator-gated slang mining, adapter and rollback | Simulated temporal experiment | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F11 Pre-send nudge without persistence | Built | Roadmap / partial | Lead draft path implemented; full acceptance pending |
| F12 Evidence timeline, chain and JSON/HTML export | Built; PDF stretch | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F13 Independent evaluation and reports | Built | Roadmap / partial | Working-tree Generator B/heuristic replay/metric/report source; command run pending; no full media ablation or measured field quality |
| F14 OCR text and PII pipeline | Built | Roadmap / partial | Main metadata/fixture text hook; audit marks it simulated; no pixel OCR or measured CER/WER |
| F15 Voice-note transcript and supporting tone | Built; tone Simulated | Roadmap / partial | Working-tree WAV fallback returns null transcript/score; supplied transcript is not ASR; genuine transcription/tone inference unavailable |
| F16 Call-log import and directional behavior features | Built on synthetic/consented logs | Roadmap / partial | Working-tree CSV validation/HMAC/directional features; HTTP import/persistence/fusion and fresh verification pending |
| F17 Recorded-call/voicemail asynchronous evidence | Built | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F18 Prepared audio streamed as simulated live call | Simulated | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F19 Image-caption hateful meme screening | Roadmap; behind-flag prototype requested | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F20 Low-confidence synthetic-voice screening | Roadmap; behind-flag prototype requested | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |
| F21 Short video frame sampling plus audio | Simulated | Roadmap / unverified | Human-owned implementation and passing integration evidence pending |

The current engine uses teammate text heuristics alongside lead behavior/fusion rules. Main includes training and vision source; it is no longer a lead-only skeleton. Source presence does not complete OCR, ASR, real image inference, evidence, dashboard or evaluation acceptance. Mock outputs, supplied metadata and proxy overrides are simulations, not model-quality measurements.

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
- Behavior features, victim-reply guard, occlusion, pile-on and fusion rules exist; the three synthetic scenario results above are reported, but full acceptance and model parity remain pending. B's call feature source is being added; its API/persistence/fusion integration and acceptance remain pending.
- A1–A4/B1–B2 source has landed; A7/B3/B4 and API upload/audit work are in progress. Trained artifacts, remaining full API scenarios, slang/evidence/jobs/streams, UI enforcement and connectors remain outstanding. C1/C2/C8 dashboard/CLI cards are deferred until engine verification.
- No current evaluator run or verified report is claimed here. The new evaluator measures synthetic heuristic replay only, excluding HTTP/persistence/media inference. Keep unsupported modality metrics, OCR CER/WER, ASR WER, absent independent-label kappa and slang results null with reasons; no invented zeros or successes.
- Exact model/dataset revisions, artifact licenses, conversion compatibility and regional language quality remain candidate-specific unknowns unless official observations are recorded in MODELS.
- Retention periods, upload deletion/backup erasure, raw-ID/PII audit, tenant/subject authorization and signing-key management remain unverified.
- API and feature documents describe the lead subset; teammate cards and other companion documentation may still need reconciliation.

## Limits even after the PoC works

Synthetic A/B behavior data may not generalize. Fresh test text reduces but cannot disprove pretrained leakage. pHash only covers known reported test images and can collide or miss edits; it does not determine consent. NSFW does not establish harassment. Deepfake/voice detection, sarcasm, reclaimed language, code-mix, OCR and transcription are fallible. Sextortion cues are uncertain. Tone cannot establish intent.

No arbitrary external private-chat access, cross-platform identity linking, covert call capture, reliable grooming detection or replacement for human moderators is claimed. A high score is a review priority, not a finding of wrongdoing. Evidence hashes/HMAC are not court certification. Legal/platform assertions require current official sources and deployment-specific review.

## Scope and updates

The matrix retains 81 unchecked required rows, 19 platform rows and 45 roadblock rows. Update actual status only with source and acceptance evidence. See REQUESTS for concrete handoffs.

## Source-check findings â€” 2026-10-28

Seven exact publisher model cards were accessible and their declared license metadata is recorded in MODELS; none is cleared for activation or locally tested. The requested multilingual Unitary config has a single toxic output, not seven harassment labels, and its card does not establish Hindi/Marathi/Konkani coverage. A must resolve label and language suitability before implementation claims.

Official sources were recorded for staged DPDP Rules commencement, Discord/Telegram message access, Google Play Call Log restrictions, WCD support resources and StopNCII's participating-platform scope. BSA certificate text, 1930/anonymous-reporting details, recording law and unselected platform integrations remain unverified; several official fetches returned empty content or timed out. These are verification gaps, not proof that a model, service or legal provision does not exist.

## Upload and model honesty boundaries

The working-tree image route is `POST /v1/analyze/image` with multipart `request` and `image`, distinct from the planned generic media endpoint. It handles supplied bytes; validation/hashing do not prove pixel decoding or learned classification. Caption scoring is separate from simulated OCR metadata/fixture extraction. `compute_phash` is an encoded-byte proxy despite its compatibility name; it does not justify perceptual resize/compression matching or NCII identification. Audio fallback must report ASR unavailable; no genuine ASR is present. Calls metadata cannot prove acoustic silence or identify a switched number. Current privacy/draft/retry/upload audit tests are pending execution.
