# Task board

Snapshot: 2026-10-28. Luna reports `format`, `check`, `build`, `openapi` and `demo-seed` passed after the latest source changes, with 58 tests and zero warnings. Real-mode synthetic scenarios 1, 2 and 7 returned high/85, high/73 and low/0 respectively. ONNX verification fixture tests passed with zero skips; see [verification records](MODELS.md#artifact-admission).

| ID | Task | Owner | Depends on | Phase | Current evidence/state | Done? |
|---|---|---|---|---|---|---|
| L0 | Scaffold/log recipes/config/root docs | Lead | none | P0 | Present; latest build/check reported passed | [ ] |
| L1 | Core DTO/config/IDs/OpenAPI/mock/health | Lead | L0 | P0 | Present; latest OpenAPI and check reported passed; full acceptance pending | [ ] |
| L2 | SQLite migrations/repos/windows | Lead | L1 | P1 | Implemented; atomic event/assessment/alert/protection persistence; full acceptance pending | [ ] |
| L3 | Registry/manifest/runtime/tokenizers/fallback traits | Lead | L1 | P1 | Shared contract entries parsed and hashes checked; verifier tests passed with zero skips; ONNX runtime/tokenizers unavailable | [ ] |
| L4 | Behavior/rules/pile-on/attribution/C6 | Lead | L2 | P1 | Rules implemented; S1/S2/S7 synthetic results reported; full acceptance pending | [ ] |
| L5 | Fusion floors/caps/reasons | Lead | L4 | P1 | Rules implemented; full end-to-end acceptance pending | [ ] |
| L6 | API auth/routes/streams/media/draft/idempotency | Lead | L2/L5 | P1 | JSON path implemented; durable alerts and protection; media unavailable | [ ] |
| L7 | S1/S2/S7 integration and demo seed | Lead | L6/C8 | P1 | Luna reports real synthetic S1 high/85, S2 high/73, S7 low/0; C8 remains pending | [ ] |
| L8 | ONNX behavior + golden vector | Lead | A4/A5 | P2 | Pending | [ ] |
| L9 | Hardening/performance/privacy/final traceability | Lead | all | P4 | Pending; initial documentation underway | [ ] |
| A1 | Training/data project and provenance | A | none | P0 | Human-team pending | [ ] |
| A2 | Text/sentiment/embedding exports and parity | A | A1 | P1 | Human-team pending | [ ] |
| A3 | Text normalization/PII/cues/backends | A | L1/A2 | P1 | Human-team pending | [ ] |
| A4 | Generator A and feature golden vectors | A | L1 | P1 | Human-team pending | [ ] |
| A5 | XGBoost train/export/baseline/manifest | A | A4 | P2 | Human-team pending | [ ] |
| A6 | Slang lexicon/miner/adapter/scheduler | A | A3/L2 | P2 | Human-team pending | [ ] |
| A7 | Eval generator B/metrics/ablations/kappa/reports | A | L6 | P2 | Human-team pending | [ ] |
| A8 | Human-run text fine-tune/export/eval | A + human | A1/A3 | P3 | Human-team pending | [ ] |
| B1 | Image validation/NSFW/hash/reports | B | L1/L3 | P1 | Human-team pending | [ ] |
| B2 | OCR pipeline and CER evaluation | B | B1/A3 | P2 | Human-team pending | [ ] |
| B3 | Audio decode/Whisper/text/tone | B | L3/A3 | P2 | Human-team pending | [ ] |
| B4 | Call CSV/features/recordings/jobs | B | L2 | P2 | Human-team pending | [ ] |
| B5 | Prepared simulated audio streaming | B | B3 | P3 | Human-team pending | [ ] |
| B6 | Face/deepfake/composite | B | B1 | P3 | Human-team pending | [ ] |
| B7 | Flagged meme/voice/video prototypes | B | B1/B3 | P3 | Human-team pending; cut first | [ ] |
| C1 | Dashboard shell/SSE alert queue | C | L1 mock | P1 | Human-team pending | [ ] |
| C2 | Alert review/timeline/pairs | C | C1/L6 | P2 | Human-team pending | [ ] |
| C3 | Slang/metrics/model/upload pages | C | C2 | P2 | Human-team pending | [ ] |
| C4 | Evidence chain/JSON/HTML/certificate draft | C | L2 | P2 | Human-team pending; PDF stretch | [ ] |
| C5 | Discord connector | C | L6 | P3 | Human-team pending | [ ] |
| C6 | Telegram connector | C | L6 | P3 | Human-team pending | [ ] |
| C7 | Instagram comment connector | C | L6 | P3 | Human-team pending; stretch | [ ] |
| C8 | CLI seed/simulate/import/export/scenarios | C | L1 | P1 | Human-team pending | [ ] |

## Phases and acceptance

P0 H0–1.5 targets contract/mock and cards. P1 H1.5–6 targets S1/S2/S7 through the API. P2 H6–14 targets breadth, OCR/calls/slang/eval/evidence/dashboard. P3 H14–20 targets audio/deepfake/connectors/trained models. P4 H20–24 targets hardening, H22 freeze and rehearsal. These are planned relative windows, not elapsed-time claims.

Cut video, synthetic voice, hateful memes, deepfake, simulated live audio and PDF first if time is short; preserve explicit Roadmap/Simulated disclosures. Android Shield is optional post-P3, not a hidden requirement of the walking skeleton.

## Promotion rule

Do not check a task from a scaffold or stub alone. Require concrete implemented paths, acceptance evidence and remaining gaps. Human teams must implement their crates. Cross-owner changes use REQUESTS. Lead adapters may unblock P0/P1 but do not close A/B/C tasks.
