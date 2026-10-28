# Evaluation

Snapshot: 2026-10-28. Evaluation implementation, fixtures, reports and runtime checks are unverified. There are no measured results in this document. A synthetic walking-skeleton demonstration cannot establish real-world accuracy.

## Metric definitions

At the agreed positive threshold (default tier >=review), report TP/FP/TN/FN and support. Precision = TP/(TP+FP); recall = TP/(TP+FN); F1 = 2PR/(P+R); FPR = FP/(FP+TN). Undefined denominators produce null with an explanation, not a fabricated zero or perfect score. Include confusion matrices, class prevalence and precision-recall curves from continuous scores.

Detection rate = harassment conversations with at least one >=review event / all labeled harassment conversations. Time-to-detection = number of events from conversation start through its first >=review result; additionally report elapsed event time. Undetected conversations are censored/missed, not zero-time successes. Report detected-only summaries alongside miss counts and total support.

Per-module metrics cover text, OCR, image, audio, calls and behavior, plus fused end-to-end results. OCR CER/WER and ASR WER use edit distance / reference characters or words, with normalization stated. Report OCR script/font/resolution slices and ASR accent/noise/code-mix slices; they are not classifier accuracy. Treat empty reference cases explicitly.

Report latency p50/p95/p99 with hardware, concurrency, warm/cold state, batch/input size and active backend. Measure CPU targets from ARCHITECTURE; a heuristic-only latency cannot stand in for model inference latency.

## Required comparisons

Use independent generator B and hand-written edge cases. Compare keyword baseline and fused system on polite stalking; ablate text-only → +behavior → +calls → full. Report benign-heavy message and call FPR separately, quoted abuse, sarcasm/reclaimed language, victim hostile replies, missing context and new-account ambiguity. Hindi and at least a Marathi/Konkani sample need explicit support and results; no evidence yet establishes regional coverage.

For slang, withhold invented terms temporally. Report before/after confirmation on disjoint contexts, false positives, frozen-set precision and rollback behavior. Reject an adapter precision drop >2 percentage points. Compute Cohen kappa for independent two-annotator labels with contingency counts and undefined-case handling.

## Nine demo acceptance cases

| Scenario | Required observation | Evidence still needed |
|---|---|---|
| S1 Polite stalker | 80 friendly messages; review then high after block; keyword baseline misses | API sequence and refusal state assertion |
| S2 Pile-on | Ten senders/one minute; pile_on and high; feed protection | Distinct-sender/spike assertions and client state |
| S3 Cyberflashing proxy | High and blur_media; simulated signal and response | Benign asset hash override, no real explicit asset |
| S4 OCR screenshot | Extracted abusive text drives flag/blur and masked reason | Real OCR result plus CER; text injected by fixture is simulated |
| S5 TTS voice threat | Transcript and high/urgent; synthetic provenance | ASR transcript/WER and text result, not fabricated transcript |
| S6 Overnight calls | Forty calls, unanswered/post-block; review→high | Imported log counts and directional refusal history |
| S7 Healthy heavy users | Reciprocal messages and long answered calls stay low | Message/call false-positive regression |
| S8 Slang update | Miss→moderator confirm→caught | Authenticated decision, version and before/after test |
| S9 Evidence export | Multimodal timeline, valid chain, verifiable manifest | JSON/HTML output, tamper/reorder test and access check |

## Test and reporting protocol

Planned acceptance paths are tests/scenarios.rs and data/test/scenarios/*.json; their existence/passing status is unverified. The parent should use logged just demo-seed and just eval recipes through Luna. Intended reports are reports/metrics.json and reports/METRICS.md; no reports are asserted here. Include run ID/date, source revision, model/config hashes, backend/degraded flags, split provenance, synthetic/real designation and all unmeasured cells as null/not run.

Separate rule correctness tests, simulated-demo acceptance and model-quality evaluation. Add model-free startup, missing/corrupt model, fallback selection, floor/cap boundaries, tone-only guard, C6 reverse-pair guard, idempotency, draft no-write, privacy/auth tests and behavior golden vectors within 1e-4. Proposed test names in TRACEABILITY are acceptance plans, not existing passing tests.

Build/network remain unresolved. No sidecar build, test, benchmark, model download, training or commit was run. A future status update must provide exact command, outcome and log/report evidence before marking Built.
