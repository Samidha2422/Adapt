# Model candidates and verification

Snapshot: 2026-10-28. No model inference is verified. Every name below is a candidate; neither a research recommendation nor a public repository alone establishes suitability, license clearance, ONNX compatibility or measured accuracy. Unknown license means unknown, not permissive.

## Candidate inventory

| Task | Candidate named by prompt/research | Existence/license verification at initial baseline | Fallback / delivery gap |
|---|---|---|---|
| Toxicity multilabel | unitary/unbiased-toxic-roberta | Not yet verified; license unknown | Lead deterministic text adapter; A backend pending |
| Multilingual toxicity | unitary/multilingual-toxic-xlm-roberta | Not yet verified; license unknown | Normalization/coverage/label mapping unverified |
| Accurate binary toxicity | textdetox/xlmr-large-toxicity-classifier-v2 | Not yet verified; license unknown; do not inherit sibling license | Optional escalation only; no multilabel inference from binary output |
| Fine-tuned head | hre-text-ft, XLM-R base | Internal artifact not delivered; base/data license unknown | Human/A training pending |
| Sentiment | cardiffnlp/twitter-xlm-roberta-base-sentiment | Not yet verified; license unknown | Do not fabricate sentiment from absent model |
| Sentence embeddings | sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2 | Not yet verified; license unknown | Similarity/slang embedding unavailable |
| Alternative embeddings | intfloat/multilingual-e5-small | Not yet verified; license unknown | Exact preprocessing and revision pending |
| Optional NER | Unspecified multilingual token classifier | No exact artifact; license unknown | A regex plan; not equivalent to NER |
| NSFW default | Marqo/nsfw-image-detection-384 | Not yet verified; license unknown | B pending; benign proxy only simulated |
| NSFW alternative | Freepik/nsfw_image_detector | Not yet verified; license unknown | Same; no vendor accuracy claim |
| Face detection | UltraFace or YuNet | Exact repository/weight revision unspecified; license unknown | B pending |
| Deepfake default | prithivMLmods/deepfake-detector-model-v1 | Not yet verified; license unknown | Simulated target; low confidence |
| Heavy deepfake | HoopitAI/video-deepfake-detection-GenD_DINOv3_L_FF | Not yet verified; license unknown including base terms | Evaluation candidate only |
| OCR | PaddleOCR det/cls/rec, English/Devanagari | Exact artifacts and wrapper unselected; licenses unknown | ocrs/Tesseract are candidates, not verified script coverage |
| Optional OCR | Tesseract eng+hin+mar; ocrs | Engine/data exact revisions unverified; licenses unknown | No claim of Hindi/Marathi/Konkani quality |
| ASR | Whisper base/tiny/small converted to ggml | Official model/conversion terms not yet checked | Missing ASR must be declared; no invented transcript |
| Speech emotion | superb/wav2vec2-base-superb-er | Not yet verified; license unknown | Supporting signal only; <=+10 and cannot flag alone |
| Synthetic voice | MelodyMachine/Deepfake-audio-detection-V2 | Not yet verified; license unknown | Roadmap, disabled by default |
| Meme image+text | CLIP ViT-B/32 ONNX | Exact source/export artifact unspecified; license unknown | Roadmap, disabled by default |
| Message behavior | behavior_msg.onnx (XGBoost export) | Internal artifact not delivered; provenance unknown | Lead rules fallback |
| Call behavior | behavior_call.onnx (XGBoost export) | Internal artifact not delivered; provenance unknown | Lead rules fallback; B call features pending |
| Hash matching | image_hasher (library, not model) | Version/API/license not yet verified | Self-owned test hashes; B implementation pending |

## Artifact admission

For each exact artifact record publisher URL, revision, date checked, license identifier and license-text URL, base-model/data restrictions, files/checksums, labels/order, input/output shapes, tokenizer/preprocessing, export tool versions, parity result and intended backend. A card metadata label is an observation, not a legal review or reproducible pinned artifact. Missing/conflicting terms remain unknown/not cleared.

The registry parses the shared [model contract](MODEL_CONTRACT.md) entries, including declared outputs and optional feature/baseline arrays, and validates metadata and paths. It verifies file hashes when present and reports heuristic fallback and degradation; it does not validate tensor compatibility or run inference. Model health is available at `GET /v1/health/models`. Hot reload and atomic session swaps remain future work.

Missing model means an explicitly named fallback and degraded reason, never an ONNX-success claim. No ONNX session or tokenizer runtime is linked. A matching hash does not establish inference or artifact suitability. Luna reports `models-verify-install` and `models-verify-test` passed with zero skipped tests. Their records are [models-verify-install.log](../target/logs/models-verify-install.log), [models-verify-install.exit](../target/logs/models-verify-install.exit), [models-verify-test.log](../target/logs/models-verify-test.log), and [models-verify-test.exit](../target/logs/models-verify-test.exit); both exit markers are 0. This verifies the test suite, not actual model artifacts or inference.

## Runtime and scope caveats

ONNX via ort is the target general runtime; Whisper ggml/native is a stated exception in the prompt. No particular ort version/API/build feature has been independently verified here. Model cards do not prove that proposed ONNX conversions exist. Label counts, thresholds, supported languages, scripts, memory footprint and CPU budgets require exact configs and local evaluation.

Dated official-source observations will be appended below. Until verified and approved for the intended use, keep candidates inactive and allow zero-model startup. Do not auto-download artifacts.

## Official-source observations — checked 2026-10-28

These checks inspected publisher-controlled cards/configs through the web tool, not weights. They supersede the initial inventory's "not yet verified" cells only for the stated facts. Every artifact remains an inactive candidate: revision pinning, full license/base/data review, checksums, export parity, local inference and suitability are pending.

| Candidate | Verified observation | Official publisher source | Remaining uncertainty |
|---|---|---|---|
| unitary/unbiased-toxic-roberta | Repository/card exists; card declares Apache-2.0. Config includes toxicity, severe_toxicity, obscene, identity_attack, insult, threat and sexual_explicit, plus identity labels. Do not treat every output as a toxicity class. | [Card](https://huggingface.co/unitary/unbiased-toxic-roberta/raw/main/README.md), [config](https://huggingface.co/unitary/unbiased-toxic-roberta/raw/main/config.json) | Exact exported head/order and pinned weights still unverified |
| unitary/multilingual-toxic-xlm-roberta | Repository/card exists; card declares Apache-2.0. Published config maps only one output, toxic. Card's seven tested languages do not include Hindi, Marathi or Konkani. | [Card](https://huggingface.co/unitary/multilingual-toxic-xlm-roberta/raw/main/README.md), [config](https://huggingface.co/unitary/multilingual-toxic-xlm-roberta/raw/main/config.json) | Cannot supply seven-label toxicity or established regional coverage from this artifact |
| textdetox/xlmr-large-toxicity-classifier-v2 | Exact v2 card exists, declares openrail++, and describes binary classification with neutral/toxic indices; Hindi is listed by publisher. | [Exact v2 card](https://huggingface.co/textdetox/xlmr-large-toxicity-classifier-v2/raw/main/README.md) | Full license restrictions/base terms, conversion and local quality unreviewed; not a seven-label substitute |
| Marqo/nsfw-image-detection-384 | Exact card exists and declares Apache-2.0; publisher specifies 384px input. | [Card](https://huggingface.co/Marqo/nsfw-image-detection-384/raw/main/README.md) | Vendor test accuracy is not this project's accuracy; artifact/export/base terms pending |
| Freepik/nsfw_image_detector | Exact card exists and declares MIT; publisher describes four output categories. | [Card](https://huggingface.co/Freepik/nsfw_image_detector/raw/main/README.md) | Need exact config mapping and intended NSFW-score aggregation |
| prithivMLmods/deepfake-detector-model-v1 | Exact card exists, declares Apache-2.0 and labels itself experimental; metadata/body differ on base-model naming. | [Card](https://huggingface.co/prithivMLmods/deepfake-detector-model-v1/raw/main/README.md) | Verify exact base revision/architecture and terms; no generalization guarantee |
| MelodyMachine/Deepfake-audio-detection-V2 | Exact card exists and declares Apache-2.0; reported result is tagged with a train split and sparse dataset metadata. | [Card](https://huggingface.co/MelodyMachine/Deepfake-audio-detection-V2/raw/main/README.md) | That result is not held-out evidence for this product; base/data provenance unresolved |
| HoopitAI/video-deepfake-detection-GenD_DINOv3_L_FF | Exact card request was inaccessible through the web tool. | [Attempted publisher card](https://huggingface.co/HoopitAI/video-deepfake-detection-GenD_DINOv3_L_FF/raw/main/README.md) | Existence/access and license remain unknown; do not infer missing or permissive |

The Unitary cards warn of differences between Hugging Face exports and the Detoxify library. Validate the exact chosen artifact rather than transferring library behavior or results to it. [Publisher notice](https://huggingface.co/unitary/unbiased-toxic-roberta/raw/main/README.md).

No replacement model is selected or activated by this sidecar. For absent multilabel/language support, keep deterministic fallback/unavailable behavior and request a verified alternative from A. All other inventory candidates retain unknown license/unverified existence as stated.
