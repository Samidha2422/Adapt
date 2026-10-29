# Model contract

Runtime compatibility note (2026-10-29): the original exports below are historical
coordination proposals. The merged ViT image backend consumes 224x224 RGB with
mean/std 0.5 and `[normal, nsfw]` order, rather than the proposed 384px NSFW wrapper.
Whisper now has separate encoder/decoder ONNX graphs and local tokenizer/processor
assets rather than a GGML runtime. Use [DEMO_MODEL_INTEGRATION.md](DEMO_MODEL_INTEGRATION.md)
and actual runtime source for the transferred model bundle. The trainer must supply
matching manifest hashes and processor/label parity; these notes do not validate
artifacts that have not been transferred. The verifier's graph-output checks alone
do not establish input or preprocessing compatibility.

This document includes historical proposed wrapper contracts alongside updated descriptions of transferred models. Proposed examples are not proof of a model's availability, preprocessing, labels, license, or measured thresholds. Consult the runtime source and the model manifest for the currently implemented contract. Exporters must inspect artifacts and processors and demonstrate parity before enabling them. A verified hash alone never means inference is active.

## Manifest and verifier agreement

The manifest is a JSON object with a `models` array. Entries record the artifact ID, task, backend, path, hash, metadata and output tensors. The examples below include historical proposals; use the current manifest and verifier for the supported schema and task-specific checks.

Metadata conventions: `version` and `tier` are nonempty strings; `status` is `built`, `simulated`, or `roadmap` (some examples use `roadmap`). `tokenizer` is a model-root-relative tokenizer artifact path where applicable; `unverified` is a placeholder requiring replacement. `labels` is ordered by output class, or empty when classes do not apply. `thresholds` contains named numeric settings; an empty object means none are specified. `license` is the verified license identifier or reference. `proposed-1`, all-zero SHA-256 values and example paths are illustrative, not evidence of artifacts. Verify licenses and pin processor, vocabulary, feature order and calibration before activation. Paths are relative to the model root and must remain inside it after canonicalization. The listed inputs are part of the task contract; validate them against actual runtime metadata before use.

For ONNX, verify safe path, file digest, successful model parsing, output names, dtypes and shapes against the applicable task contract. The Whisper encoder has explicitly supported symbolic dimensions; other unexpected dynamic dimensions are rejected. Verify input metadata separately and report fallback and degradation. Run output range, finite-value, label-order, preprocessing and numerical parity tests with representative and golden inputs before activation; metadata checks alone do not validate semantics. Reports must distinguish artifact verification, runtime readiness, and actual inference.

The transferred Whisper pipeline uses separate ONNX encoder and decoder graphs; its current contract is described in the Whisper section below.

Common conventions: all image tensors are RGB NCHW float32, scaled from 8-bit to `[0,1]` then normalized as specified; preprocessing occurs outside ONNX unless explicitly stated. Probability outputs are finite and in `[0,1]`; multiclass probabilities sum to approximately one. Thresholds below are proposed operating defaults, not validated claims, and must be calibrated per artifact and deployment. A threshold never overrides fusion policy or human review. A model, data proxy, or override contributing simulated output must set `simulated=true`; unavailable modules are Roadmap/Disabled. Retain no raw message or media bodies; drafts persist nothing and publish no alerts.

## Text fast — `text_fast`

Input `input_ids` int64 `[1,128]`, `attention_mask` int64 `[1,128]`; wrapper output `probabilities` float32 `[1,7]`, independent sigmoid probabilities in label order `toxicity`, `severe_toxicity`, `obscene`, `identity_attack`, `insult`, `threat`, `sexual_explicit`. Use the exact pinned model tokenizer, special tokens, truncate to 128 and pad to 128 with its pad ID; mask real tokens with 1, padding with 0. Adapter maps verified native labels; absent categories cannot be fabricated as predictions. Default per-label decision >=0.5; escalate to accurate tier when toxicity is within `[0.35,0.75]`, inclusive. Quantization must pass parity tests.

```json
{"id":"text_fast","version":"proposed-1","task":"text_fast","backend":"onnx","path":"text_fast.onnx","sha256":"0000000000000000000000000000000000000000000000000000000000000000","tokenizer":"unverified","labels":["toxicity","severe_toxicity","obscene","identity_attack","insult","threat","sexual_explicit"],"thresholds":{"decision":0.5,"escalation_min":0.35,"escalation_max":0.75},"license":"unverified","tier":"fast","status":"roadmap","outputs":[{"name":"probabilities","dtype":"float32","shape":[1,7]}]}
```

## Text accurate — `text_accurate`

Inputs `input_ids`, `attention_mask`, both int64 `[1,256]`; output `probabilities` float32 `[1,2]`, labels `non_toxic`, `toxic` in that order. Tokenize with the pinned model tokenizer and pad/truncate to 256. Wrapper maps the native binary output to these probabilities; it does not create the seven fast-tier categories. Proposed toxic decision >=0.5; run for fast-tier ambiguity or explicitly selected evaluation, subject to timeout and degradation.

```json
{"id":"text_accurate","version":"proposed-1","task":"text_accurate","backend":"onnx","path":"text_accurate.onnx","sha256":"0000000000000000000000000000000000000000000000000000000000000000","tokenizer":"unverified","labels":["non_toxic","toxic"],"thresholds":{"decision":0.5},"license":"unverified","tier":"accurate","status":"roadmap","outputs":[{"name":"probabilities","dtype":"float32","shape":[1,2]}]}
```

## Fine-tuned text — `text_fine_tuned`

Inputs `input_ids`, `attention_mask`, int64 `[1,128]`; output `probabilities` float32 `[1,11]`, independent sigmoid labels `toxicity`, `severe_toxicity`, `obscene`, `insult`, `threat`, `identity_attack`, `sexual_explicit`, `coercion_sextortion`, `stalking_cue`, `doxxing_cue`, `sexual_harassment`. Use the pinned training tokenizer, special tokens and pad/truncate to 128; training and export freeze exactly these 11 labels and their order. Default per-label decision >=0.325 pending held-out calibration.

```json
{"id":"text_fine_tuned","version":"proposed-1","task":"text_fine_tuned","backend":"onnx","path":"text_fine_tuned.onnx","sha256":"0000000000000000000000000000000000000000000000000000000000000000","tokenizer":"unverified","labels":["toxicity","severe_toxicity","obscene","insult","threat","identity_attack","sexual_explicit","coercion_sextortion","stalking_cue","doxxing_cue","sexual_harassment"],"thresholds":{"decision":0.325},"license":"unverified","tier":"fine_tuned","status":"roadmap","outputs":[{"name":"probabilities","dtype":"float32","shape":[1,11]}]}
```

## Sentiment — `sentiment`

Inputs `input_ids`, `attention_mask`, int64 `[1,128]`; output `probabilities` float32 `[1,3]`, labels `negative`, `neutral`, `positive`. Use its pinned tokenizer, special tokens and pad/truncate to 128; wrapper maps verified labels and applies softmax if needed. Select argmax only if its probability >=0.5; otherwise report uncertain. Sentiment is supporting context, not a harassment verdict.

```json
{"id":"sentiment","version":"proposed-1","task":"sentiment","backend":"onnx","path":"sentiment.onnx","sha256":"0000000000000000000000000000000000000000000000000000000000000000","tokenizer":"unverified","labels":["negative","neutral","positive"],"thresholds":{"minimum_confidence":0.5},"license":"unverified","tier":"default","status":"roadmap","outputs":[{"name":"probabilities","dtype":"float32","shape":[1,3]}]}
```

## Embeddings — `embeddings`

Inputs `input_ids`, `attention_mask`, int64 `[1,128]`; output `embedding` float32 `[1,384]`. No class labels. Use the exact pinned tokenizer, prefix policy and pad/truncate to 128; wrapper applies the model's verified pooling and L2 normalization. Reject zero-norm or nonfinite embeddings. Proposed cosine candidate threshold >=0.8, subject to calibration and the distinct-sender and moderator gates; similarity is not an automatic lexicon update. A model with another embedding dimension requires a revised contract.

```json
{"id":"embeddings","version":"proposed-1","task":"embeddings","backend":"onnx","path":"embeddings.onnx","sha256":"0000000000000000000000000000000000000000000000000000000000000000","tokenizer":"unverified","labels":[],"thresholds":{"cosine_candidate":0.8},"license":"unverified","tier":"default","status":"roadmap","outputs":[{"name":"embedding","dtype":"float32","shape":[1,384]}]}
```

## NSFW — `nsfw`

Input `pixel_values` float32 `[1,3,224,224]`; output `probabilities` float32 `[1,2]`, labels `normal`, `nsfw` in that order. Decode to RGB, resize to 224x224 with bilinear interpolation, scale to `[0,1]`, then normalize with mean and standard deviation `[0.5,0.5,0.5]`. Verify parity with the exact exported processor. Decision threshold >=0.5; contextual fusion determines action. Benign proxies are simulated.

```json
{"id":"nsfw","version":"transferred-1","task":"nsfw","backend":"onnx","path":"nsfw.onnx","sha256":"bdaccd5a615761cc67f60465bc52deaa8b5298c1ba8ce43ed14d1e7a1967cd67","tokenizer":null,"labels":["normal","nsfw"],"thresholds":{"decision":0.5},"license":"Apache-2.0","tier":"default","status":"built","outputs":[{"name":"probabilities","dtype":"float32","shape":[1,2]}]}
```

## Deepfake — `deepfake`

Input `pixel_values` float32 `[1,3,224,224]`; output `probabilities` float32 `[1,2]`, labels `authentic`, `manipulated`. The current implementation resizes the whole image to 224x224 and normalizes RGB with mean/std 0.5; it does not run a real face detector. Verify parity with the native processor. A manipulated probability of 0.70 combined with NSFW probability of 0.50 triggers the composite rule. This is a low-confidence supporting signal; missing model yields unavailable, not authentic.

```json
{"id":"deepfake","version":"transferred-1","task":"deepfake","backend":"onnx","path":"deepfake.onnx","sha256":"dfa99abbee32433d8b67fbd2f9be91323f8b6e5c8e984ef70064025b65b09ac7","tokenizer":null,"labels":["authentic","manipulated"],"thresholds":{"decision":0.5},"license":"Apache-2.0","tier":"default","status":"built","outputs":[{"name":"probabilities","dtype":"float32","shape":[1,2]}]}
```

## Face detection — `face`

Input `pixel_values` float32 `[1,3,320,320]`; outputs `boxes` float32 `[1,100,4]`, `scores` float32 `[1,100]`, `valid_count` int64 `[1]`. One class `face`; boxes are normalized `[x_min,y_min,x_max,y_max]` in the letterboxed input, scores are confidences, and only the first `valid_count` rows are valid. RGB letterbox to 320x320 with black padding, scale to `[0,1]`, no mean subtraction. Wrapper performs decoding and nonmaximum suppression, sorts by descending score, truncates to 100 and zero-pads. Proposed score >=0.5 and IoU NMS threshold 0.4; reverse letterbox for original coordinates, clamp and reject invalid boxes. Actual detector preprocessing and decoding require explicit adaptation and parity checks.

```json
{"id":"face","version":"proposed-1","task":"face","backend":"onnx","path":"face.onnx","sha256":"0000000000000000000000000000000000000000000000000000000000000000","tokenizer":null,"labels":["face"],"thresholds":{"score":0.5,"nms_iou":0.4},"license":"unverified","tier":"default","status":"roadmap","outputs":[{"name":"boxes","dtype":"float32","shape":[1,100,4]},{"name":"scores","dtype":"float32","shape":[1,100]},{"name":"valid_count","dtype":"int64","shape":[1]}]}
```

## OCR — `ocr`

Proposed **recognition wrapper for one already detected text-line crop**, not an end-to-end OCR detector. Input `pixel_values` float32 `[1,3,48,320]`; output `token_probabilities` float32 `[1,80,256]` over a versioned 256-entry vocabulary, with index 0 reserved for CTC blank. Labels are token IDs in that pinned vocabulary, not a claimed upstream alphabet; its concrete Unicode mapping and language coverage must be supplied and validated by the exporter before activation. Resize line crop preserving aspect ratio into 320x48 with right-side black padding, RGB scale to `[0,1]`, normalize with mean/std `[0.5,0.5,0.5]`. Wrapper maps native outputs to 80 time steps and the exact vocabulary, including blank; incompatible native recognizers require a revised contract. CTC greedy decode: argmax each step, collapse consecutive repeats, remove blank; confidence is the mean selected nonblank token probability. Proposed acceptance >=0.5; report uncertain/unsupported script rather than invent text. Detection, reading order, and Hindi/Marathi/Devanagari coverage require independently validated components and are not guaranteed by this example.

```json
{"id":"ocr","version":"proposed-1","task":"ocr","backend":"onnx","path":"ocr_recognizer.onnx","sha256":"0000000000000000000000000000000000000000000000000000000000000000","tokenizer":null,"labels":[],"thresholds":{"minimum_confidence":0.5},"license":"unverified","tier":"default","status":"roadmap","outputs":[{"name":"token_probabilities","dtype":"float32","shape":[1,80,256]}]}
```

## Whisper — `whisper`

The transferred ONNX Whisper encoder takes `input_features` float32 `[batch,80,3000]` and returns `last_hidden_state` float32 `[batch,1500,384]` for that input. Its graph declares the first two output dimensions as `batch_size` and `encoder_sequence_length / 2`; the verifier accepts those exact symbols for this task and represents them as `1` and `1500` in the manifest. Runtime warm-up must confirm the concrete tensor. The companion decoder takes `input_ids` int64 and `encoder_hidden_states` float32, returning `logits` float32 `[batch,sequence,51865]` plus cache tensors. The decoder, tokenizer and processor must match the pinned asset hashes. The worker accepts PCM WAV, resamples to mono 16 kHz, uses English decoding and processes at most the first 30 seconds. No transcript confidence threshold is assumed; validate speech recognition separately.

```json
{"id":"whisper","version":"onnx-tiny-1","task":"whisper","backend":"onnx","path":"whisper_encoder.onnx","sha256":"6642befb640f950d4a8cbbd17834d59e7e75f575b81ccf213e06b050623ab1dd","tokenizer":"whisper_tokenizer.json","labels":[],"thresholds":{},"license":"Apache-2.0","tier":"default","status":"built","outputs":[{"name":"last_hidden_state","dtype":"float32","shape":[1,1500,384]}]}
```

## Tone — `tone`

Inputs `audio` float32 `[1,160000]`, `attention_mask` int64 `[1,160000]`; output `probabilities` float32 `[1,4]`, labels `neutral`, `happy`, `sad`, `angry`. Decode mono, resample to 16 kHz, normalize PCM to `[-1,1]`, truncate or zero-pad to 10 seconds; mask valid samples with 1 and padding with 0. Wrapper handles model-specific normalization and maps native labels, verified by parity. Argmax only if >=0.5, otherwise uncertain. Tone is supporting context, adds at most the configured fusion allowance and never flags alone.

```json
{"id":"tone","version":"proposed-1","task":"tone","backend":"onnx","path":"tone.onnx","sha256":"0000000000000000000000000000000000000000000000000000000000000000","tokenizer":null,"labels":["neutral","happy","sad","angry"],"thresholds":{"minimum_confidence":0.5},"license":"unverified","tier":"default","status":"roadmap","outputs":[{"name":"probabilities","dtype":"float32","shape":[1,4]}]}
```

## Behavior messages — `behavior_messages`

Input `features` float32 `[1,47]`; output `probabilities` float32 `[1,2]`, labels `non_harassment`, `harassment`. Source is the exact current `hre_behavior::feature_order()` window-major ordering: for each `1h`, `24h`, `7d`, append `msg_count`, `reply_ratio`, `night_ratio`, `tox_mean`, `tox_max`, `threat_max`, `sexual_max`, `sent_mean`, `pii_mentions`, `location_cue_count`, `unsolicited_media_count`, each suffixed with its window; then append `unreplied_streak`, `initiator_ratio`, `median_gap_s`, `burst_max_10m`, `after_refusal_count`, `since_refusal_hours`, `active_days_streak`, `tox_slope_last20`, `sent_slope_last20`, `repetition_score`, `sender_account_age_days`, `is_contact`, `distinct_channels`, `evasion_similarity`. Use the extractor's units and sentinel values; do not normalize unless the identical transformation is embedded in the wrapper and training. Validate finite values, exact order and a golden vector. Proposed score threshold >=0.5; rules and fusion retain their independent protections and floors. Models trained on synthetic data must disclose that limitation.

```json
{"id":"behavior_messages","version":"proposed-1","task":"behavior_messages","backend":"onnx","path":"behavior_msg.onnx","sha256":"0000000000000000000000000000000000000000000000000000000000000000","tokenizer":null,"labels":["non_harassment","harassment"],"thresholds":{"decision":0.5},"license":"unverified","tier":"default","status":"roadmap","outputs":[{"name":"probabilities","dtype":"float32","shape":[1,2]}]}
```

## Behavior calls — `behavior_calls`

Input `features` float32 `[1,8]`; output `probabilities` float32 `[1,2]`, labels `non_harassment`, `harassment`. Proposed ordered features: `outgoing_calls_1h`, `outgoing_calls_24h`, `unanswered_calls_24h`, `night_calls_24h`, `after_refusal_calls_24h`, `answered_duration_s_24h`, `distinct_callers_24h`, `recipient_reply_ratio_24h`. Counts are nonnegative, duration is seconds, ratio is `[0,1]`; windows end at event time, exclude future events, and include endpoints. Missing data must be explicitly represented by an agreed trained missing-value policy, never silently converted into a known zero. No call feature schema or trained model is established in `docs/FEATURES.md`; this list is a proposed export boundary requiring owner agreement and golden vectors before use. Apply preprocessing identically in training and inference. Proposed threshold >=0.5; contextual rules and fusion remain independent. Synthetic training must be disclosed.

```json
{"id":"behavior_calls","version":"proposed-1","task":"behavior_calls","backend":"onnx","path":"behavior_call.onnx","sha256":"0000000000000000000000000000000000000000000000000000000000000000","tokenizer":null,"labels":["non_harassment","harassment"],"thresholds":{"decision":0.5},"license":"unverified","tier":"default","status":"roadmap","outputs":[{"name":"probabilities","dtype":"float32","shape":[1,2]}]}
```
