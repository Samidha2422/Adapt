# Engine evaluation

Synthetic Generator B and fresh authored text; heuristic backends only. No C8.

| Evaluation | N | TP | FP | TN | FN | Precision | Recall | F1 | FPR |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| audio: No ASR/audio backend and paired waveform/transcript corpus connected. | null | null | null | null | null | null | null | null | null |
| behavior | 68 | 28 | 0 | 28 | 12 | 1.0000 | 0.7000 | 0.8235 | 0.0000 |
| call: No standalone call classifier connected; call logs exercise actual hre-behavior rules only. | null | null | null | null | null | null | null | null | null |
| fusion | 68 | 32 | 0 | 28 | 8 | 1.0000 | 0.8000 | 0.8889 | 0.0000 |
| image: No real image classifier and labeled image corpus connected; hash/override proxies cannot measure classifier accuracy. | null | null | null | null | null | null | null | null | null |
| ocr: No pixel OCR backend and paired image/transcript evaluation corpus connected; metadata proxies do not measure OCR. | null | null | null | null | null | null | null | null | null |
| text | 18 | 5 | 1 | 8 | 4 | 0.8333 | 0.5556 | 0.6667 | 0.1111 |
| full: OCR/image/audio/call classifiers unavailable in this evaluator; text_behavior is the full available engine path. | null | null | null | null | null | null | null | null | null |
| plus_calls: Standalone call module unavailable; existing call events already contribute to behavior. | null | null | null | null | null | null | null | null | null |
| text_behavior | 68 | 32 | 0 | 28 | 8 | 1.0000 | 0.8000 | 0.8889 | 0.0000 |
| text_only | 68 | 8 | 0 | 28 | 32 | 1.0000 | 0.2000 | 0.3333 | 0.0000 |
| behavior_call_context | 20 | 12 | 0 | 4 | 4 | 1.0000 | 0.7500 | 0.8571 | 0.0000 |
| heavy_benign_calls | 4 | 0 | 0 | 4 | 0 | null | null | null | 0.0000 |
| heavy_benign_messages | 16 | 0 | 0 | 16 | 0 | null | null | null | 0.0000 |
| polite_stalker_keyword | 4 | 0 | 0 | 0 | 4 | null | 0.0000 | 0.0000 | null |
| polite_stalker_system | 4 | 4 | 0 | 0 | 0 | 1.0000 | 1.0000 | 1.0000 | null |
| text_en | 12 | 5 | 1 | 5 | 1 | 0.8333 | 0.8333 | 0.8333 | 0.1667 |
| text_hi | 2 | 0 | 0 | 1 | 1 | null | 0.0000 | 0.0000 | 0.0000 |
| text_hi-Latn | 2 | 0 | 0 | 1 | 1 | null | 0.0000 | 0.0000 | 0.0000 |
| text_keyword_baseline | 18 | 3 | 3 | 6 | 6 | 0.5000 | 0.3333 | 0.4000 | 0.3333 |
| text_mr-like | 2 | 0 | 0 | 1 | 1 | null | 0.0000 | 0.0000 | 0.0000 |

Detection: 32/40 (0.8000). TTD p50/p95: 18.0000/18.0000 contact events; 8 missed.

Latency p50/p95/p99: 0.8935/2.0216/2.5271 ms (7358 samples).

- asr_wer: null — No ASR predictions or TTS waveform/reference pairs.

- cohens_kappa: null — No independent annotator label files supplied; use hre-eval kappa LABEL_A.json LABEL_B.json.

- ocr_cer/ocr_wer: null — No pixel OCR predictions with independent reference transcripts.

- slang_temporal_split: null — No slang feedback/adaptation engine connected; no before/after scores invented.

Methodology and limits:

- Synthetic results only; no field accuracy, pretrained model inference or public benchmark claims.

- Generator B is independent Rust code and reads no training/ files or Generator A outputs; distribution separation is not statistically certified.

- Semantic labels are authored before scoring, not derived from engine scores. Ambiguous cases are deliberately benign stress cases.

- Engine-only in-memory replay; no HTTP, persistence, alerts, client C8, model registry or media overrides.

- Text uses explicit heuristic backend. Behavior includes rule/logistic fallback and pile-on; fusion uses actual public functions with serialized configs.

- Conversation confusion uses maximum score across both directions; labels describe whole synthetic conversations, not offender attribution.

- Time-to-detection is one-based contact events (including replies, excluding refusal markers) from conversation start; misses are null and excluded from percentiles.

- Latency is serial un-warmed debug/release-dependent wall time for text + history append + behavior + combined fusion; excludes generation, text-only ablation, IO and API.

- Small language slices are illustrative; mr-like authored text is not linguistically validated coverage.

- Cyberflasher excluded from scored generator classes: absent real media inference cannot supply a prediction. No C8/deepfake/synthetic-voice evaluation.

See metrics.json for PR curves, thresholds, configs and case results. See docs/EVALUATION_SCHEMA.md for definitions.
