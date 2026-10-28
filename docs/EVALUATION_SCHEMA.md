# Engine evaluation schema v1

`just eval` (or `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 eval`) runs the API-based full evaluator. It writes `reports/metrics.json` and `reports/METRICS.md` under the repository root, using the full evaluator's report schema rather than schema v1 below.

The rest of this document describes the engine-only evaluator selected with `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 eval --independent`. The script sets the working directory to the repository root before invoking the evaluator. Its default output directory is `reports`, so the two files are written under the repository root; `--output PATH` selects a directory, with relative paths resolved from that working directory and absolute paths used directly. Direct executable invocations resolve relative output and config paths from the caller's working directory. The two evaluators use the same default filenames and overwrite one another's reports if run in the same output directory. The engine-only evaluator needs no server, credentials, model files, training data, media assets or C8 client. Parent/Luna owns recipe execution and log review.

Arguments after `--independent`: `--seed 20261028 --variants 4 --output reports`. Variants must be 1..64. A seed shifts phase lengths and schedules deterministically; it is not a training split or a statistical independence certificate. Default input is 68 conversations across 17 scenario classes and 18 fresh authored text cases. Generator B uses phase schedules, irregular gaps and interleaved replies rather than a training generator or training fixtures. No files under `training/` are read. It uses a public synthetic-only HMAC key, retains no raw identity or body in reports, opens no database and publishes no alerts.

Config is read from `config/behavior_rules.toml` and `config/fusion.toml` relative to the working directory; absent files use crate defaults, unreadable/invalid files fail. Exact resolved configurations are embedded. Behavior night windows use the serialized behavior config (UTC by default); this CLI does not load API `engine.toml` timezone overrides. Text always uses the real deterministic `hre_text::analyze` backend, behavior uses `hre_behavior::analyze`, fusion uses `hre_fusion::fuse`. It does not activate ONNX from artifact hashes or substitute proxy scores. The adapter matches the current API's text/behavior fusion fields; coercion-only/doxxing floors are not invented. Results measure these functions, not HTTP, database, action delivery, or deployed API equivalence.

Top-level fields:

| Field | Meaning |
|---|---|
| `schema_version` | Integer `1` |
| `simulated` | Always `true`: every input is synthetic/authored |
| `seed`, `variants`, `generator` | Reproduction parameters and Generator B version |
| `behavior_config`, `fusion_config` | Exact evaluated config values |
| `modules` | text, behavior, fusion; OCR/image/audio/call have null metrics and explicit reasons |
| `ablations` | text_only, text_behavior on the same conversations; plus_calls/full null when unavailable |
| `slices` | Heavy benign messages/calls, behavior on calls, polite-stalker system/keyword, text keyword baseline, language slices |
| `detection` | Positive/detected/missed conversation counts, detection rate, detected-only TTD p50/p95 |
| `latency` | Sample count and nearest-rank p50/p95/p99 milliseconds with scope |
| `conversations` | Case IDs/classes/labels, contact count, maximum scores and nullable first review event; no bodies or subject IDs |
| `text_cases` | Case IDs, language tags, authored boolean labels, actual heuristic and keyword scores; no bodies |
| `ocr_cer`, `ocr_wer`, `asr_wer`, `cohens_kappa`, `slang_temporal_split` | Null until real evaluation inputs/backends exist; `unavailable_reasons` explains each |
| `limitations` | Machine-readable methodology and scope statements |

Each measurement has `status` (`built` or `roadmap`), `backend` (`heuristic` or `disabled`), `simulated: true`, `unit`, `threshold`, `metrics`, `pr_curve`, `reason`. Built describes the executed heuristic, not model inference or generalization. Roadmap/Disabled describes unavailable evaluation here, not a claim that every module is absent repository-wide. No fabricated or proxy modality scores are emitted.

Metrics contain `n`, `confusion: {tp,fp,tn,fn}`, `precision`, `recall`, `f1`, `fpr`. Positive predictions use inclusive `score >= threshold`. Precision = TP/(TP+FP), recall = TP/(TP+FN), F1 = 2TP/(2TP+FP+FN), FPR = FP/(FP+TN). Zero denominators serialize as JSON `null`, including FPR for all-positive slices. PR points contain threshold/precision/recall at every distinct observed score plus 0 and 1.000001 (no predictions). These are empirical PR curves, not interpolated AP/AUC. JSON contains full curves; Markdown contains tables and limits.

Text is scored per fresh authored text case at 0.5. Conversation behavior is the maximum actual behavior-or-pile-on score across contact events at a documented fixed operating threshold of 0.31; this is not a calibrated probability. Conversation fusion uses maximum actual severity/100 with the configured review threshold. Confusion counts each conversation once and includes both directions, so it does not measure offender attribution. Positive labels express authored scenario intent, not engine-generated verdicts. Deliberately benign ambiguous/idiom cases can reveal false positives. Language slices (English, Hindi, Hinglish, Marathi-like) are tiny illustrations; Marathi-like text has not been validated by a native annotator.

Detection rate is positive conversations with any event at or above review divided by all positive conversations. `first_review_event` is one-based count of all contacts (including replies, excluding refusal markers), measured from conversation start, not from a labeled onset. Misses remain `null` and count as misses, never zero. TTD percentiles include detected positive conversations only; counts expose survivor bias. Heavy benign slice FPR is conversation-level (any false flag), separately for messages and answered calls. It is not event-level FPR.

The keyword baseline is a frozen case-insensitive substring list: kill, hurt, idiot, worthless, nudes, leak. It has no history or adaptation. Its polite-stalker comparison reports actual misses against the same conversations; no claimed relative gain is hard-coded.

The supported ablation removes behavior/pile-on signals and all behavior-derived floors/caps/context to produce text_only, then adds them for text_behavior. Call logs are already input to behavior; no independent call classifier is added. Therefore plus_calls and full multimodal metrics are null with reasons. Cyberflasher has no scored case because no real media backend/corpus is connected. No C8, deepfake or synthetic-voice scores are produced. OCR CER/WER and ASR WER stay null; metadata extraction or hash checks cannot stand in for transcription/classifier accuracy.

Latency measures serial wall time for text, history append, behavior and combined fusion per contact. It excludes generation, report IO, text-only ablation and HTTP/storage. No warm-up or benchmark claim is made; build profile and host affect it. It must not be presented as production p95.

Agreement utility: pass `kappa LABEL_A.json LABEL_B.json` through the eval runner. Each file is a nonempty JSON boolean array in exactly the same item order; lengths must match. The utility prints `{n, cohens_kappa, reason}`. Kappa uses observed versus marginal chance agreement; constant identical labels yield null with reason. It does not claim generated labels are independent annotator agreement, and does not mutate the main report. Slang temporal metrics remain null until a real adaptation engine and independently labeled temporal split are connected.

Verification requested from parent/Luna: logged `format`, `check`, then both evaluator paths. Inspect their `.exit` and `.log` files; return at most 15 lines, no raw logs. Model-free tests cover confusion/null denominators, tied PR thresholds, kappa validation, deterministic chronological HMAC fixtures, config/variant validation, recomputable report metrics, null unavailable modalities, report privacy and independent executable output from a directory without models/training/config.

The logged runner invokes the full evaluator by default; pass `--independent` to select the schema documented here.
