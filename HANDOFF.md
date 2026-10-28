# Harassment Risk Engine handoff

## Objective and source of truth

Complete the Rust multimodal harassment risk engine described in [ENGINE_PROMPT.md](ENGINE_PROMPT.md). Read that file and all three files in [docs/research](docs/research/) before implementation. The prompt's intended Built labels are requirements, not evidence of completed work. Respect [AGENTS.md](AGENTS.md), the owner boundaries, logged `just` recipes and Luna verification protocol. The user has shared [MODEL_CONTRACT.md](docs/MODEL_CONTRACT.md) with the model-training teammate; coordinate before changing that interface.

The target is an HTTP/WebSocket engine with directional history, explainable severity/actions, safe media handling, model-free fallbacks, model-backed upgrades, moderator surfaces, connectors, evidence, and honest evaluation. Completion requires nine real demo scenarios through the API, `just eval` reports, model-free operation, mock mode, and all 81 feature/problem/challenge/deliverable rows traceable to implementation or honest limitations.

## Current state

Branch `lead/engine-foundation` at foundation commit `3e10113` contains six lead crates: `hre-core`, `hre-store`, `hre-ml`, `hre-behavior`, `hre-fusion`, and `hre-api`. It provides validated DTOs and generated OpenAPI, mock responses, API authentication, HMAC pseudonyms, SQLite history, idempotency, atomic assessment/alert/protection persistence, refusal events, behavior rules, pile-on, victim-reply guard, occlusion attribution, fusion and draft nudges. Media routes and most non-lead crates are not complete. Alerts survive restart; SSE is process-local. Cross-instance scoring is not serialized.

Last reported verification: `just format`, `just check`, `just build`, `just openapi`, and real `just demo-seed` passed; 58 Rust tests, zero warnings. Synthetic real-server scenarios 1, 2, 7 yielded high/85, high/73, low/0. `just models-verify-test` passed 11 tests with no skips. This verifies fixture inspection, not any real model artifact or inference. The model registry accepts shared manifest entries and checks hashes; it does not run ONNX. The model loader experiment was removed because missing/incompatible dynamic runtime must safely fall back. No real model-backed backend is currently active.

Read [docs/FEATURES.md](docs/FEATURES.md) for the 47-element behavior feature order and golden vector; [docs/API.md](docs/API.md), [docs/openapi.json](docs/openapi.json), [docs/LIMITATIONS.md](docs/LIMITATIONS.md), [docs/TRACEABILITY.md](docs/TRACEABILITY.md), [docs/REQUESTS.md](docs/REQUESTS.md), and [docs/TASKS.md](docs/TASKS.md) for contracts, actual limitations, and ownership. Set `HRE_SALT`, `HRE_API_KEYS`, and `HRE_MODERATOR_TOKENS`; see [RUNBOOK.md](RUNBOOK.md). `just demo-seed` requires a running server and key; currently the real path tests only scenarios 1, 2, and 7. Mock mode is simulated.

## Features still required

Partial: F1 minimal two-phrase threat fallback only; F2 pair history; F3 rule-based stalking and occlusion without trained model/evasion; F4 sender-count pile-on without spike/coordination; F9 rules fusion without full modality acceptance; F11 draft nudge; F16 individual call-event behavior without CSV import/full call features. The feature implementations are not equivalent to full original acceptance.

Not implemented: F5 image/NSFW/hash matching; F6 deepfake/face composite; F7 audio/ASR; F8 PII; F10 slang; F12 evidence; F13 evaluation; F14 OCR; F15 voice-note/tone; F17 recording/voicemail jobs; F18 simulated streaming; F19 meme; F20 cloned voice; F21 video. F19/F20 are roadmap; F18/F21 and proxy demos must be visibly simulated. F5/F6/F7/F14/F15/F17 also require real safe media integration, not captions masquerading as OCR or ASR. All gaps and research problems are tracked in LIMITATIONS/TRACEABILITY.

## Implementation order

1. Coordinate teammate branches/cards under [docs/tasks/cards](docs/tasks/cards/). A owns text/slang/eval/training; B owns vision/audio/calls; C owns evidence/dashboard/connectors/CLI. Do not silently edit another owner's crate; route requests through REQUESTS.md. The human can authorize changed ownership if teammates are unavailable.
2. A: build normalization, multilingual classifier interface and deterministic fallback, PII masking, model export and parity tests, synthetic training generator A and independent Rust evaluation generator B, slang moderator gate, and real metrics. Match the published model contract and the 47-feature golden vector. Freeze labels, versions and licenses; report synthetic limitations and leakage safeguards.
3. B: validate and sniff size-limited media, hash-only reported image matching and NSFW, OCR→text/PII, consented audio→Whisper→text with supporting tone, call CSV/features and recording jobs. Implement missing-model degradation, no raw media retention by default, and simulation provenance. Coordinate ONNX and GGML tooling without modifying the shared contract unilaterally.
4. C: implement moderator dashboard, durable evidence timeline and hash-chain export, connectors for owned/consented spaces, CLI and nine fixtures. Ensure mock responses never masquerade as real inference and simulation is visible.
5. Lead: add a verified ONNX runtime behind a feature with safe missing-library handling, input/output/feature-order/label/hash checks, runtime readiness separate from successful inference, and an actual heuristic fallback. Integrate A/B/C, model reload, rate limits, response-size limits, streaming and remaining required endpoints. Resolve cross-instance read/score/write races and non-durable SSE; audit privacy, access control and retention.
6. Complete real scenarios 3–6 and 8–9, assert all nine through the API, then implement `just eval` and report precision, recall, F1, FPR, confusion matrix, detection rate, time-to-detection, baselines/ablations, OCR/ASR accuracy, kappa and latency without invented metrics. Train A and evaluate B on structurally independent synthetic data plus fresh examples.
7. Run logged format/lint/tests/build/OpenAPI/demo/eval/bench; update traceability and limitations from evidence. Use conventional commits and report API contract changes. Do not commit models, datasets, secrets or unverified metrics.

## Human input and limitations

The model trainer must supply licensed, contract-compatible artifacts and tokenizer/feature metadata plus parity evidence; training requires access to the teammates' machines or compute. Real platform connectors need accounts, tokens, configured permissions and informed participants. Legal/recording consent and deployment decisions need human review. A coding agent can implement and test the remaining source with suitable tools and model artifacts, but cannot conjure accounts, training hardware, verified licenses, consent, or teammates' unpublished branches. Current performance budgets and security hardening are not verified.

Original prompt and research documents are included as project inputs for the next agent; they describe requirements, not current capabilities.
