# Harassment Risk Engine — Training, Datasets & Model Exports

This directory hosts the Python training project, dataset provenance inventory, ONNX export specifications, and Synthetic Generator A for the Harassment Risk Engine.

> [!IMPORTANT]
> **Zero Weights in Git**: Binary model weights (`*.onnx`, `*.bin`, `*.safetensors`, `*.pt`) are strictly gitignored and must never be committed to the repository.
> **Split Independence**: Generator A fixtures are strictly reserved for training. Independent evaluation is performed by Generator B (in Rust / `hre-eval`) using structurally distinct distributions and parameter ranges to prevent circular leakage.

---

## 1. Dataset Provenance Inventory (Card A1)

Dataset records are maintained in [`datasets.json`](datasets.json) and managed via [`inventory.py`](src/hre_training/inventory.py).

### Candidate Datasets
| Dataset ID | Candidate / Origin | License State | Split | Description |
|---|---|---|---|---|
| `jigsaw_toxic_comment` | Jigsaw / Civil Comments | `CC0-1.0` (Verified) | `train` | Multi-label toxicity benchmark (threat, insult, identity attack, etc.) |
| `hatexplain` | HateXplain Benchmark | `MIT` (Verified) | `train` | Hate speech and offensive language with target group annotations |
| `davidson_hate_speech` | Davidson et al. | `MIT` (Verified) | `train` | Distinguishes offensive language from targeted hate speech |
| `textdetox_paradetox` | TextDetox / ParaDetox | `Apache-2.0` (Verified) | `train` | Parallel toxic/neutral sentence pairs across English and Hindi |
| `ravdess_audio_emotion` | RAVDESS Audio/Visual | `CC-BY-NC-4.0` (Restricted) | `train` | Acted emotional speech (calm, happy, angry, sad); research only |
| `crema_d_speech` | CREMA-D Actors Dataset | `ODbL-1.0` (Verified) | `train` | Multi-actor voice recordings annotated for emotional intent |
| `synthetic_generator_a` | Internal Generator A | `Proprietary Internal` | `train` | Deterministic synthetic message/call behavior histories |
| `held_out_regional_eval` | Hand-Authored Regional | `Proprietary Internal` | `eval_held_out` | Fresh hand-written English, Hindi, Hinglish, Marathi, and Konkani |

### Safe Download & Offline Rules
- **Offline First**: All inventory listing, validation, and metadata queries operate completely offline with no network dependencies.
- **Opt-In Required**: Downloads are strictly opt-in via `--opt-in` and restricted to `data/raw/`. Arbitrary download locations are rejected.
- **Non-Commercial Gate**: Restricted licenses (e.g. `CC-BY-NC-4.0`) require explicit authorization and cannot be deployed commercially.

---

## 2. ONNX Model Export & Parity (Card A2)

Export specifications, label orders, and parity contracts are managed via [`export_onnx.py`](src/hre_training/export_onnx.py).

### Candidate Models & Export Contracts
| Model ID | Task | License | Cleared | Dynamic Shapes | Labels |
|---|---|---|---|---|---|
| `unitary/unbiased-toxic-roberta` | Toxicity Multi-label | `Apache-2.0` | **Yes** | `[batch_size, seq_len]` | 7 labels (toxicity, severe_toxicity, obscene, identity_attack, insult, threat, sexual_explicit) |
| `unitary/multilingual-toxic-xlm-roberta` | Multilingual Toxicity | `Apache-2.0` | **Yes** | `[batch_size, seq_len]` | `["toxic"]` (Single output) |
| `textdetox/xlmr-large-toxicity-classifier-v2` | Binary Toxicity (Large) | `openrail++` | **Yes** | `[batch_size, seq_len]` | `["neutral", "toxic"]` |
| `cardiffnlp/twitter-xlm-roberta-base-sentiment` | Sentiment | `unknown` | **Blocked** | `[batch_size, seq_len]` | `["negative", "neutral", "positive"]` |
| `sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2` | Sentence Embeddings | `Apache-2.0` | **Yes** | `[batch_size, seq_len]` | Dim 384 embedding |

### License Clearance Gate & Tolerances
- **License Gate**: Any candidate model with an `unknown` or unreviewed license (e.g. Cardiff sentiment) is blocked from export until official terms are vetted.
- **Parity Tolerance**: Exported ONNX logits must match upstream PyTorch reference outputs within `1e-4` absolute tolerance.
- **Honest Fallback**: When model weights are absent locally, the engine explicitly falls back to deterministic heuristic classification (`Backend::Heuristic`, `degraded: ["text.onnx_missing"]`) rather than claiming artificial model inference.

---

## 3. Synthetic Generator A (Card A4)

Generator A deterministically synthesizes conversation histories for training behavioral classifiers (such as XGBoost) without touching private user logs.

### Feature Extraction (47 Features)
Implements exact parity with `docs/FEATURES.md` and `hre-behavior::extract_features`:
- **Windowed Features** (11 features × 3 windows: 1h, 24h, 7d = 33 features):
  `msg_count`, `reply_ratio`, `night_ratio`, `tox_mean`, `tox_max`, `threat_max`, `sexual_max`, `sent_mean`, `pii_mentions`, `location_cue_count`, `unsolicited_media_count`.
- **Global Features** (14 features):
  `unreplied_streak`, `initiator_ratio`, `median_gap_s`, `burst_max_10m`, `after_refusal_count`, `since_refusal_hours`, `active_days_streak`, `tox_slope_last20`, `sent_slope_last20`, `repetition_score`, `sender_account_age_days`, `is_contact`, `distinct_channels`, `evasion_similarity`.

The canonical golden vector is stored in `fixtures/golden_vector.json`.

---

## 4. Commands & Verification

### Run All Unit & Parity Tests
```bash
python -c "import sys; sys.path.insert(0, 'training/src'); import unittest; unittest.main(module=None, argv=['', 'discover', '-s', 'training/tests', '-p', 'test_*.py'])"
```

### Dataset Provenance Inventory CLI
```bash
# Verify inventory schema and split boundaries
python training/inventory.py --verify

# Print formatted inventory table
python training/inventory.py
```

### ONNX Model Export Specifications CLI
```bash
# Display model candidates and contract status
python training/export_onnx.py

# Verify license clearance
python training/export_onnx.py --verify-license unitary/unbiased-toxic-roberta
```

### Generate Synthetic Behavior Data
```bash
python training/generator_a.py --seed 42 --output training/fixtures/synthetic_samples.json
```
