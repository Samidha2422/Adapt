# Harassment Risk Engine handoff

## Latest integration audit — 2026-10-28

The authoritative latest outcome is in [ENGINE_VERIFICATION.md](docs/ENGINE_VERIFICATION.md). The integration passed 150 Rust tests, 26 training tests (one absent-model parity skip), 11 verifier tests, build, lint, OpenAPI and evaluation. Real demo scenarios 1/2/7 passed; full demo-seed failed at scenario 3 because its caption-only fixture assumes nonexistent image inference. All-nine acceptance is NOT complete. Calls/audio/eval libraries and real image uploads were added; actual ASR/ONNX/pixel inference remain unavailable. Dashboard/C1/C2/C8 are deferred per the user's request. Older sections below may describe pre-audit state; prefer this summary and the verification report.

## Scope and source of truth

Snapshot: 2026-10-28, main at `1f8d420` plus concurrent working-tree changes. Finish the engine before dashboard task cards **C1/C2/C8**. These are dashboard shell, alert review/timeline, and CLI tasks; they are distinct from research challenges C1/C2/C8 (stalking, multimodality, detector generalization). Dashboard/CLI delivery is not claimed by engine-only work.

Read [ENGINE_PROMPT.md](ENGINE_PROMPT.md), [research](docs/research/), and [AGENTS.md](AGENTS.md). Preserve owner boundaries and the shared [MODEL_CONTRACT.md](docs/MODEL_CONTRACT.md). Full completion still requires nine API scenarios, honest evaluation, model-free operation, mock mode, and acceptance evidence for the complete traceability matrix.

## Main versus current changes

Main includes the lead core/store/ML-registry/behavior/fusion/API foundation **and A1–A4/B1–B2 source**, not audio or calls. A1/A2 provide training inventory/export contracts; A3 adds text normalization, heuristic classification, PII and cues; A4 adds synthetic Generator A and a golden vector. B1/B2 add image validation, screening/hash interfaces and metadata/fixture text processing. Their merge is evidence of source availability, not complete task acceptance or real inference.

Concurrent agents are adding `hre-calls`, `hre-audio`, `hre-eval`, API uploads and audit corrections/tests. At final source inspection, calls CSV/features, evaluation source and bounded PCM16 WAV decode/fallback source were present. These changes are unverified until the parent integrates them and receives fresh Luna results. See [docs/ENGINE_VERIFICATION.md](docs/ENGINE_VERIFICATION.md) for commands and pending evidence.

The lead paths include typed DTOs, auth, HMAC identities, SQLite directional history, idempotency, atomic event/assessment/alert/protection persistence, refusal events, heuristic behavior, victim-reply guard, occlusion attribution, fusion and draft analysis. SSE remains process-local; cross-instance scoring serialization is unresolved.

## Capability boundaries

- Text/behavior/fusion run deterministic heuristic paths. Models are unavailable for active inference. Registry/hash verification does not establish runtime readiness or successful inference. Inventory/export scaffolding does not establish downloads, license clearance, training, export or parity.
- Working-tree `POST /v1/analyze/image` accepts multipart `request` JSON and `image` bytes; it is not the planned generic `/v1/analyze/media` contract (`metadata` + `file`). Caption analysis is text analysis. Header validation and SHA-256 are not pixel classification or full decode validation.
- OCR reads container metadata or registered fixture text: **simulation**, never recognized pixels. The audit changes disclose `simulated=true` and unavailable OCR explicitly. The current `compute_phash` compatibility function is an encoded-byte proxy, not a perceptual hash; no robust resize/compression or NCII coverage claim is justified.
- Audio source provides transient bounded PCM16 WAV decode and an explicit unavailable-ASR report (`transcript=null`, `score=null`); no genuine speech transcription, emotion inference or voice-clone detection is available. Its integration and tests remain pending. Supplied transcript text or TTS/proxy fixtures cannot be presented as ASR predictions.
- Calls source adds CSV validation, HMAC identities and directional feature extraction. HTTP import, persistence and fusion integration require separate evidence. Short answered calls are a metadata proxy, not proof of acoustic silence; number identity linking remains unavailable.
- Evaluation source adds Rust Generator B, authored text, heuristic replay, metrics, keyword baselines, partial ablations and report writing. It does not exercise HTTP/persistence/dashboard/CLI. No current report run is claimed. Unsupported modality metrics, OCR CER/WER, ASR WER, kappa without two label files and slang adaptation remain unavailable/null.
- Slang, evidence export, recording jobs, prepared audio WS, connectors and dashboard/CLI remain outstanding unless separately delivered and verified. Deepfake/voice generalization remains unmeasured; F19/F20 are Roadmap, F18/F21 remain unverified simulation targets.

## Verification and next handoff

Earlier documentation reported 58 Rust tests, 11 model-verifier fixture tests, and S1/S2/S7 high/85, high/73, low/0. Those are historical foundation reports, **not verification of current main or working-tree changes**. No build logs were read and no engine commands were run for this documentation update.

Parent owns integration, Luna verification and commits. Run the logged commands in [ENGINE_VERIFICATION.md](docs/ENGINE_VERIFICATION.md), replace pending cells with commit-scoped summaries, regenerate/reconcile OpenAPI and record contract changes. Audit drafts for zero persistence/alerts, media/transcript privacy, proxy provenance, upload validation, retries and durable restart behavior. Continue missing API scenarios and unavailable modules without claiming nine-scenario completion or measured model quality.

Update [TASKS](docs/TASKS.md), [LIMITATIONS](docs/LIMITATIONS.md) and [TRACEABILITY](docs/TRACEABILITY.md) together. Preserve every feature, problem, challenge, deliverable, platform and roadblock row. Cross-owner requests belong in [REQUESTS](docs/REQUESTS.md); signatures are in [LEAD_HANDOFF](docs/LEAD_HANDOFF.md) and public Rust source. Model artifacts, licenses, compute, consent and connector accounts remain external prerequisites.
