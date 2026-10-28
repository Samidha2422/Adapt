# Engine verification report

## Final audited result — 2026-10-28

This section supersedes the earlier pending ledger below. Audited main at `1f8d420`, then fixed integration on `lead/engine-integration`. Luna reports: format/check/build/OpenAPI/eval PASS; 150 Rust tests passed (0 failed/ignored); 26 training tests passed with 1 real-model parity skip because `models/unbiased-toxic-roberta.onnx` is absent; 11 model-verifier tests passed with no skips.

Real demo-seed FAILS: on a fresh server/DB scenarios 1, 2 and 7 pass, then scenario 3 expects high but receives low/2. The old seed submits only an image caption. The removed caption-to-synthetic-image shortcut was not genuine detection. Scenario 4 has the same obsolete assumption and was not reached. All nine real scenarios do NOT pass. Logs and exit markers are in target/logs/.

Delivered: actual-byte multipart image ingestion; explicit simulated metadata OCR; honest unavailable NSFW and encoded-byte hash semantics; PII panic/masking and quote-discount fixes; call CSV validation/import with per-row outcomes and partial-commit semantics; bounded PCM16 WAV decoding with disabled ASR; independent Generator B evaluation writing reports/metrics.json and reports/METRICS.md; fail-closed license and export gates. MODEL_CONTRACT was preserved.

Remaining: real ONNX artifacts/runtime and parity, pixel OCR/NSFW, perceptual hashes, Whisper/recording jobs, slang, complete modality evaluation and all-nine acceptance. Synthetic heuristic metrics do not establish real-world accuracy. Dashboard/C1/C2/C8 were deliberately left out. See training/AUDIT.md and docs/LIMITATIONS.md. The full original engine is NOT complete.

Snapshot: 2026-10-28. **Pending execution and parent review.** Source inspection covers main `1f8d420` plus concurrent changes; the exact tested revision/diff must be recorded after integration. This is report scaffolding, not a passing verification report.

## Execution and evidence ownership

Parent delegates noisy commands and log reading to Luna. This documentation task ran no builds, tests, downloads, training or benchmarks and read no build logs. Each recipe writes `target/logs/<recipe>.log` and a numeric `.exit`; the parent receives at most 15 summary lines, never raw logs. Execute from the repository root using `just <recipe>` or `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 <recipe>`.

Historical foundation reports (58 Rust tests, 11 verifier fixture tests, S1/S2/S7 results) are not current evidence. Re-run after concurrent integration. No verified metrics, test counts or durations are supplied here.

## Command ledger

| Exact command for Luna | Purpose | Current result | Evidence to record |
|---|---|---|---|
| `just format` | Apply workspace formatting before checks | Pending | Revision/diff scope and Luna summary |
| `just check` | fmt check, clippy with denied warnings, workspace tests | Pending | Numeric exits, summary test/warning counts; child fmt/lint/test results |
| `just build` | Build integrated workspace | Pending | Numeric exit and concise summary |
| `just training-test` | Offline training inventory/export/generator checks | Pending | Summary; distinguishes scaffolding from executed training/export |
| `just models-verify-test` | Model-verifier fixture tests | Pending | Summary; does not verify real models or inference |
| `just openapi` | Regenerate schema from integrated routes | Pending | Numeric exit plus parent schema diff review |
| `just run` | Start authenticated real-mode API for demo checks | Pending | Readiness and env configuration names only; stop server afterward |
| `just demo-seed` | Exercise available real-server scenarios | Pending | Scenario IDs, assertions and omissions; never imply all nine |
| `just eval` | Write `reports/metrics.json` and `reports/METRICS.md` using heuristic Generator B replay | Pending | Seed/variants/configs, case counts, synthetic scope, null reasons |
| `just bench` | Available benchmark coverage | Pending | Actual harness/coverage, profile, machine and percentiles or unavailable reason |

`just run` needs `HRE_SALT`, `HRE_API_KEYS` and `HRE_MODERATOR_TOKENS` configured without exposing values. Commands above are existing recipes at inspection, not proof their dependencies are complete. Do not download models as an implicit prerequisite. `just models-verify` is conditional on supplied admitted artifacts; none is claimed here. Full benchmark coverage and `/v1/metrics/latency` remain separate requirements.

## Acceptance ledger

| Boundary | Source/evidence target | Current result |
|---|---|---|
| Text/PII heuristics | `crates/hre-text/src/`; authored benign/abuse and privacy tests | Pending current checks; no ONNX quality claim |
| Uploads | `crates/hre-api/tests/media.rs`; actual image bytes, MIME/body limits, caption separation, draft/retry/report behavior | Tests in progress; pending |
| OCR/hash honesty | `crates/hre-vision/src/ocr.rs`, `hasher.rs`, API propagation | Metadata/fixtures simulated; byte proxy not perceptual hashing; pending checks |
| Calls | `crates/hre-calls/src/lib.rs`; CSV validation, HMAC, direction/windows, reciprocity, simulation propagation | Source present; integration/verification pending |
| Audio | `crates/hre-audio/`; decode/fallback validation and ASR-unavailable disclosure | Bounded PCM16 WAV decode/fallback source present; tests/integration pending; no genuine ASR |
| Draft/privacy/persistence | API integration tests; no raw IDs/bodies, no draft writes/alerts, restart/idempotency | Audit in progress; pending |
| Evaluation | `crates/hre-eval/src/`; Generator B, confusion/PR metrics, keyword baseline, benign-heavy slices, report writer | Source present; command/report validation pending |
| Research C1/C2/C8 | Baseline comparison; modality provenance; uncertain detector review/unavailability | Partial heuristics only; no generalization or full multimodal acceptance |
| Task cards C1/C2/C8 | Dashboard shell, review/timeline, CLI fixtures | Outside this pre-dashboard engine verification; outstanding |
| Nine scenarios | ENGINE_PROMPT section 14 and API fixtures | Full run pending; mock/proxy runs explicitly simulated |

## Report fields to fill after execution

- Tested commit and working-tree diff scope; execution date; Luna summary reference.
- Each command's numeric outcome, meaningful assertions, warnings and remaining failures.
- Evaluator seed, variants, backend/config versions, synthetic disclosure and case denominators.
- Precision/recall/F1/FPR, confusion/PR data, detection and missed-conversation counts, TTD definition and exclusions, latency scope/profile; leave unmeasured values null.
- Separate unavailable OCR CER/WER, ASR WER, standalone image/audio/call classifier metrics, full ablation, annotator kappa and slang temporal split. Never substitute zero or metadata/proxy accuracy.
- Remaining scenarios, API routes, privacy/security boundaries and platform/model prerequisites. Link final artifacts after inspection; file presence alone is not acceptance.
