# Latest engine and Android integration

Sources reviewed: Adapt main `428231e` (multiline OCR), aegis-demo main `27d10c6`
(composer, calls and slang changes), and the trainer's supplied integration guide.
The guide describes artifacts on the training machine; it is not evidence that
those artifacts have been transferred to this computer.

## Connection

Android sends requests to the chat server on port 8081. The chat server calls
Adapt on port 8080 with `AEGIS_ENGINE_API_KEY`, matching `HRE_API_KEYS`. Models
remain on the Adapt host. Android receives the engine's actual actions, reasons,
signal details, backend/status, simulation flags and degradation information.

The updated media path forwards image bytes to `/v1/analyze/image` and audio bytes
to `/v1/analyze/audio`. No user upload is replaced with a bundled demo asset.
Synthetic sample uploads explicitly set `context.simulated=true` at the engine.
Blocked uploads stay held; risky audio can be shown as a transcript; missing audio
inference is unscored. Raw upload bytes are discarded after analysis.

For audio, multipart parts are `request` (the existing envelope with kind=audio)
and `audio` (the file). The response is AnalyzeResponse with `recommended_action`,
not `action`. A masked transcript is returned only to the upload client; engine
assessments and alert broadcasts do not retain it. JSON recordings/jobs are the
older fixture interface, not the upload path.

## Artifacts to transfer from the trainer

Transfer the matching manifest and preprocessing assets with the weights. Preserve
the original expected hashes; do not replace hashes merely to make verification pass.

| Capability | Files |
|---|---|
| Text | `text_fine_tuned.onnx`, matching `tokenizer.json`, exact eleven-label metadata |
| NSFW | genuine `nsfw.onnx` (trainer reports approximately 344 MB), 224px RGB, mean/std 0.5 per channel, normal/nsfw order |
| Deepfake | genuine `deepfake.onnx` (approximately 343 MB), authentic/manipulated order and matching preprocessing |
| OCR | `ocr_detector.onnx`, `ocr_recognizer.onnx`, `ocr_vocab.json` in matching token order |
| Whisper | `whisper_encoder.onnx`, `whisper_decoder.onnx`, `whisper_tokenizer.json`, `whisper_preprocessor_config.json` |
| Behavior | `behavior_msg.onnx`, `behavior_call.onnx`, ordered feature/baseline metadata (`features.json`) |

At inspection this computer has the text/behavior artifacts, OCR detector and
vocabulary, and Whisper encoder/decoder/tokenizer/preprocessor files. The NSFW
and deepfake files are still 339 and 343 bytes, and the OCR recognizer is 82 KB.
The current manifest has no Whisper task entry. Transfer the matching trained
artifacts and manifest before claiming genuine media inference. Standalone face, tone, sentiment,
embedding and fast/accurate text files listed as stubs by the trainer do not become
trained models through integration.

Whisper currently uses a Python ONNX worker and requires compatible `numpy`,
`scipy`, `onnxruntime`, `transformers` and tokenizer dependencies in the engine
environment. Short PCM WAV clips are the supported live decoder path. This is an
English Whisper-tiny pipeline; no multilingual ASR accuracy claim is made. Decoding
is limited to the first 30 seconds and 64 generated tokens; chunk longer recordings
before claiming complete transcription.

## Engine first; demo integration follows

At the user's request, this change updates only Adapt. The actual aegis-demo
checkout is unchanged. Draft demo backend and Android patches are saved locally
under `target/integration-patches`; they are unfinished and unverified, and must be
reviewed in the demo workspace before application. They are not part of this commit.

After switching to aegis-demo, connect its media uploads to the new audio route,
preserve signal `details` and `secondary_actions` in Rust/Kotlin DTOs, keep
synthetic sample provenance, and replace the recently added fixed audio success
response. The current fake recording UI must become a real recorder or an honest
audio file picker. Live slang confirmation also currently returns success without
calling Adapt, and its displayed scores are fixed; use real results or direct
moderator actions to the dashboard. CSV imports must inspect per-row failures.
