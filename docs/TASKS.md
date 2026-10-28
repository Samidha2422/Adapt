# Task board

Snapshot: 2026-10-28. Main at `1f8d420` includes A1–A4/B1–B2 source, plus the lead foundation. Calls/audio fallback/eval, uploads and audit tests are concurrent changes; current acceptance is pending. Historical foundation results do not verify this snapshot. See [ENGINE_VERIFICATION.md](ENGINE_VERIFICATION.md).

| ID | Task | Owner | Depends on | Phase | Current evidence/state | Done? |
|---|---|---|---|---|---|---|
| L0 | Scaffold/log recipes/config/root docs | Lead | none | P0 | Present; historical build/check report only; fresh verification pending | [ ] |
| L1 | Core DTO/config/IDs/OpenAPI/mock/health | Lead | L0 | P0 | Present; historical OpenAPI/check report only; current schema and acceptance pending | [ ] |
| L2 | SQLite migrations/repos/windows | Lead | L1 | P1 | Implemented; atomic event/assessment/alert/protection persistence; full acceptance pending | [ ] |
| L3 | Registry/manifest/runtime/tokenizers/fallback traits | Lead | L1 | P1 | Shared contract entries parsed and hashes checked; historical verifier fixture tests reported passed; ONNX runtime/tokenizers unavailable | [ ] |
| L4 | Behavior/rules/pile-on/attribution/C6 | Lead | L2 | P1 | Rules implemented; S1/S2/S7 synthetic results reported; full acceptance pending | [ ] |
| L5 | Fusion floors/caps/reasons | Lead | L4 | P1 | Rules implemented; full end-to-end acceptance pending | [ ] |
| L6 | API auth/routes/streams/media/draft/idempotency | Lead | L2/L5 | P1 | JSON path and durable alerts/protection present; image multipart upload added in working tree; audit and current verification pending | [ ] |
| L7 | S1/S2/S7 integration and demo seed | Lead | L6/C8 | P1 | Historical synthetic S1 high/85, S2 high/73, S7 low/0; current run and C8 pending | [ ] |
| L8 | ONNX behavior + golden vector | Lead | A4/A5 | P2 | Pending | [ ] |
| L9 | Hardening/performance/privacy/final traceability | Lead | all | P4 | Privacy/honesty audit and tests in progress; docs reconciled; current verification pending | [ ] |
| A1 | Training/data project and provenance | A | none | P0 | Main: training inventory/provenance scaffolding; concurrent audit corrections; downloads/unification not implemented | [ ] |
| A2 | Text/sentiment/embedding exports and parity | A | A1 | P1 | Main: candidate export contracts; concurrent fail-closed license/export audit; actual export/runtime parity absent | [ ] |
| A3 | Text normalization/PII/cues/backends | A | L1/A2 | P1 | Main: text normalization, heuristic classification, PII/cues; audit changes in progress; model runtime and language quality unverified | [ ] |
| A4 | Generator A and feature golden vectors | A | L1 | P1 | Main: synthetic Generator A and 47-feature golden fixture; one fixture is not broad parity or trained-model quality | [ ] |
| A5 | XGBoost train/export/baseline/manifest | A | A4 | P2 | Human-team pending | [ ] |
| A6 | Slang lexicon/miner/adapter/scheduler | A | A3/L2 | P2 | Human-team pending | [ ] |
| A7 | Eval generator B/metrics/ablations/kappa/reports | A | L6 | P2 | Working tree: Generator B, heuristic replay, metrics/baselines/partial ablations/report writer; execution pending; media metrics unavailable | [ ] |
| A8 | Human-run text fine-tune/export/eval | A + human | A1/A3 | P3 | Human-team pending | [ ] |
| B1 | Image validation/NSFW/hash/reports | B | L1/L3 | P1 | Main: validation/screening/hash interfaces; audit corrects byte proxy and unavailable classifier claims; API uploads in progress | [ ] |
| B2 | OCR pipeline and CER evaluation | B | B1/A3 | P2 | Main: metadata/fixture extraction and text/PII hook; simulated OCR only; pixel OCR and CER evaluation unavailable | [ ] |
| B3 | Audio decode/Whisper/text/tone | B | L3/A3 | P2 | Working tree: bounded PCM16 WAV decode and unavailable-ASR report with null transcript/score; no genuine ASR; integration/verification pending | [ ] |
| B4 | Call CSV/features/recordings/jobs | B | L2 | P2 | Working tree: CSV/HMAC and directional call features; API import, recording jobs and acceptance pending | [ ] |
| B5 | Prepared simulated audio streaming | B | B3 | P3 | Human-team pending | [ ] |
| B6 | Face/deepfake/composite | B | B1 | P3 | Audit of existing deepfake interfaces in progress; no verified real face/deepfake inference or generalization | [ ] |
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

P0 H0â€“1.5 targets contract/mock and cards. P1 H1.5â€“6 targets S1/S2/S7 through the API. P2 H6â€“14 targets breadth, OCR/calls/slang/eval/evidence/dashboard. P3 H14â€“20 targets audio/deepfake/connectors/trained models. P4 H20â€“24 targets hardening, H22 freeze and rehearsal. These are planned relative windows, not elapsed-time claims.

Cut video, synthetic voice, hateful memes, deepfake, simulated live audio and PDF first if time is short; preserve explicit Roadmap/Simulated disclosures. Android Shield is optional post-P3, not a hidden requirement of the walking skeleton.

## Promotion rule

Do not check a task from a scaffold or stub alone. Require concrete implemented paths, acceptance evidence and remaining gaps. Maintain crate ownership; source availability and fresh acceptance evidence are separate. Cross-owner changes use REQUESTS. Lead adapters may unblock P0/P1 but do not close A/B/C tasks.

## Active engine gate before dashboard work

Task cards C1/C2/C8 mean dashboard shell, review/timeline and CLI; research challenges with the same IDs mean behavioral stalking, multimodal fusion and detector generalization. Do not close either set from heuristic source or an engine-only evaluation run. Finish and verify integrated engine changes before dashboard work; keep missing models, metadata OCR simulation and unavailable ASR visible. Parent owns verification/commits; documentation edits do not change the shared model contract.
