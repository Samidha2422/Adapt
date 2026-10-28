# A1–A4 / B1–B2 audit handoff

Scope: only `crates/hre-text`, `crates/hre-vision`, and `training`. API, workspace runner, and shared model contract were not edited by this audit. Other working-tree changes belong to concurrent owners. No builds, downloads, training, benchmarks, or test execution were run here: parent Luna owns logged checks. This session exposes no agent messaging tool.

## Native candidates versus deployment adapters

`MODEL_SPECS` records unverified native candidate expectations. It must not be emitted as a deployment manifest. `docs/MODEL_CONTRACT.md` specifies proposed deployment wrappers, not actual upstream signatures. Neither set proves artifact availability, upstream labels, license clearance, or inference readiness.

| Candidate spec | Proposed deployment wrapper | Required work (absent) |
|---|---|---|
| `toxicity_multilabel`, dynamic batch/sequence, max 512, `logits`, seven proposed labels | `text_fast`, int64 `[1,128]` inputs, float32 `probabilities` `[1,7]` | Pin tokenizer/revision and native label order; pad/truncate to 128; verify independent sigmoid and mapping; export fixed dimensions and prove runtime parity. Renaming the task/output alone is insufficient. |
| One-output multilingual toxicity | No seven-category equivalent | Cannot fabricate six absent categories. A different deployment target needs coordinated agreement. |
| Binary toxicity large, max 512, `neutral`/`toxic` logits | `text_accurate`, `[1,256]`, `non_toxic`/`toxic` probabilities | Verify native activation/labels; explicitly map neutral to non_toxic and pin processing; prove parity. |
| Sentiment max 256, dynamic logits | `sentiment`, `[1,128]`, three probabilities | Pin tokenizer and native class order; verify softmax and static adapter. |
| Dynamic `sentence_embedding` | `embeddings`, `[1,128]` inputs, `embedding` `[1,384]` | Verify pooling, dimension, L2 normalization, finite/nonzero outputs and exact output name. |

All `license_cleared` flags are false. Publisher URLs and previously asserted license names are discovery pointers only; no review evidence is committed. Export fails closed before checking weights. Tests mock admission only to exercise missing/existing-weight behavior. Existing weights raise `NotImplementedError`; a file's existence never proves export or numerical parity. The artifact parity test explicitly skips even when an artifact exists because its runtime/reference harness is absent.

## Concrete missing delivery

| Card | Present | Absent or limited |
|---|---|---|
| A1 | Offline candidate inventory and schema tests | Actual dataset download/unification, reviewed licenses, revision/checksum provenance, delivered regional held-out corpus. Synthetic numeric histories do not establish language coverage. |
| A2 | Candidate native specs and finite numeric comparator | Actual ONNX exports, deployment adapters, reference/runtime parity, pinned weights/tokenizers. |
| A3 | Deterministic text rules, normalization, PII/cues | Language identification, tokenizer/ONNX classification, sentiment/embedding inference. Rule heuristics remain Built/Heuristic with explicit degradation. |
| A4 | Generator A with fourteen scenario classes and one 47-value golden fixture | Broad independent cross-language parity, trained behavior model; generated expected tiers are labels, not measured predictions. Installed `hre-generator-a` entry references a missing package `main`; use the source wrapper until repaired. |
| B1 | Header/MIME/dimension screening, SHA-256, synthetic override scores | Pixel decoding/validation, real NSFW inference, perceptual hash from decoded pixels, known-image match database. `compute_phash` is an encoded-byte proxy retained for signature compatibility, not NCII detection. |
| B2 | Supplied fixture/container-text scoring and CER utility | Real OCR recognition, text detection/line crops, language support, rendered-pixel CER evaluation. Metadata/fixture results are Simulated; no supplied text is Roadmap/Disabled. |

Audio/calls B3–B5 were absent from the delivered tracked scope at audit start. Concurrent untracked `hre-audio`/`hre-calls` directories appeared during the audit; these are outside ownership and were neither changed nor validated here. Do not describe them as delivered or working based on directory existence.

## Correctness/security changes

- PII: short-number bounds and Unicode context slicing; identifier masking preserves repeated letters while retaining Unicode canonicalization; sentence-final email punctuation. Text-report match offsets refer to canonical PII input before repetition folding; standalone `mask_pii` offsets refer to its input. Reporting-prefix quotes no longer discount direct threat/coercion cues outside the quote or elsewhere in the message.
- NSFW: unavailable inference is Roadmap/Disabled (`image_unavailable`), never benign. Nonfinite overrides are ignored, finite overrides clamped consistently for code/config, and contributing overrides are simulated.
- OCR: supplied text and nested contributing signals are simulated. The legacy `raw_text` field returns masked text; serialized reports contain no detected raw identifiers. Overlength text errors propagate rather than becoming a zero score. iTXt parsing respects compression/header fields; PNG length arithmetic is checked. CER retains two rows rather than a quadratic matrix.
- Deepfake: caller-supplied composite scores remain simulated even when the rule does not fire; nonfinite values become zero.
- Training: unverified licenses fail closed; unimplemented export never reports success; parity rejects empty/nonfinite values and invalid tolerances; native specs are copied to avoid caller mutation; incomplete dynamic-axis metadata fails validation. Dataset paths are rooted to the repository rather than process cwd; malformed provenance gets typed errors.

## Parent Luna verification

Run logged recipes `training-test`, `format`, then `check` through `just` or `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 <recipe>`. Inspect `.exit` and logs locally; return at most 15 lines, no raw logs. Formatting may touch other owners' files, so parent coordinates it. All added regressions remain unexecuted in this session.

Pay particular attention to API integration: it must propagate OCR/image simulated status and unavailable backends to aggregate results. This audit deliberately did not edit the API or shared DTOs. The public OCR DTO retains its fields, but `raw_text` is privacy-masked and unavailable/simulated statuses change intentionally. No staging or commits were performed.

Changed paths: `crates/hre-text/src/{classifier,cues,lib,normalizer,pii}.rs`; `crates/hre-vision/src/{deepfake,hasher,lib,nsfw,ocr}.rs`; `training/{README.md,AUDIT.md,datasets.json}`; `training/src/hre_training/{export_onnx,inventory}.py`; `training/tests/{test_export_onnx,test_inventory}.py`.
