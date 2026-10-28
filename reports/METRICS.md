# Multimodal Harassment Risk Engine - Evaluation Report

> **Disclaimer on Synthetic Provenance & Circularity Avoidance:**
> In accordance with Section 14 of `ENGINE_PROMPT.md` and `docs/EVAL.md`, all behavioral results reported herein were evaluated on synthetic data generated via independent **Generator B** (Rust implementation with distinct parameter ranges and independent structural distributions). No circular training data was evaluated. No real non-consensual explicit or abusive imagery was collected or stored; all visual tests used benign proxies and blurred placeholders.

## 1. Executive Metadata

- **Run ID:** `ba126685-493d-4a77-a72a-6ff198a95ef2`
- **Timestamp UTC:** `2026-10-28T12:32:53.114278+00:00`
- **Environment:** Rust Synthetic Generator B (Independent parameter structures)
- **Total Evaluated Conversations:** 16
- **Total Evaluated Events:** 298
- **Overall Harassment Detection Rate:** 75.0%
- **Mean Time-to-Detection (Events):** 5.6 events
- **Benign Heavy-User False Positive Rate (FPR):** 0.0%

## 2. End-to-End Performance

| Metric | Value |
|---|---|
| Precision | 100.0% |
| Recall | 69.6% |
| F1 Score | 0.821 |
| False Positive Rate (FPR) | 0.0% |
| True Positives (TP) | 112 |
| False Positives (FP) | 0 |
| True Negatives (TN) | 137 |
| False Negatives (FN) | 49 |
| Support (Events) | 298 |

## 3. Per-Module Metric Breakdown

| Module | Precision | Recall | F1 Score | FPR | Support (TP/FP/TN/FN) |
|---|---|---|---|---|---|
| **text** | 100.0% | 1.2% | 0.025 | 0.0% | 298 (2/0/137/159) |
| **ocr** | 100.0% | 0.6% | 0.012 | 0.0% | 298 (1/0/137/160) |
| **image** | 100.0% | 0.6% | 0.012 | 0.0% | 298 (1/0/137/160) |
| **audio** | 100.0% | 0.6% | 0.012 | 0.0% | 298 (1/0/137/160) |
| **call** | 100.0% | 19.3% | 0.323 | 0.0% | 298 (31/0/137/130) |
| **behavior** | 100.0% | 56.5% | 0.722 | 0.0% | 298 (91/0/137/70) |

## 4. Multimodal Ablation Study

Shows how adding each modality and behavioral state enhances coverage across multimodal harassment scenarios:

| Configuration | Description | Precision | Recall | F1 | Detection Rate | Mean TTD |
|---|---|---|---|---|---|---|
| **text_only** | Text model and PII detector only; zero behavior state or call logs | 100.0% | 1.9% | 0.037 | 25.0% | 1.0 events |
| **text_plus_behavior** | Text model + directional pair history + pile-on detector | 100.0% | 59.0% | 0.742 | 66.7% | 6.4 events |
| **text_plus_calls** | Text model + call log behavior (volume, overnight, unanswered) | 100.0% | 21.1% | 0.349 | 41.7% | 1.8 events |
| **full_multimodal** | Full pipeline: text, vision/OCR, audio, calls, behavior, and fusion | 100.0% | 69.6% | 0.821 | 75.0% | 5.6 events |

## 5. Keyword Baseline vs Full System (Polite Stalker Case)

- **Scenario:** `polite_stalker`
- **Total Evaluated Messages:** 80
- **Keyword Filter Flags:** 0 (0.0% detection)
- **Full System Flags:** 63 (100.0% conversation detection)
- **Analysis:** Keyword filters completely miss polite stalking because individual messages contain cordial words ('lovely day', 'hello'). The fused multimodal engine tracks directional volume, lack of reciprocity, and refusal boundaries to accurately escalate to Review and High.

## 6. Slang Adaptation Temporal-Split Experiment

Verifies zero-shot miss of invented code word, moderator confirmation, and subsequent detection without drift:

| Phase | Invented Code Word | Meaning | Match Detected | Tier | Status |
|---|---|---|---|---|---|
| **Pre-Confirmation** | `vexling` | Unrecognized | `false` | `Low` | Missed (Benign Baseline) |
| **Post-Confirmation** | `vexling` | invented harassing code word (severity=0.50) | `true` | `Review` | **Caught (Escalated to Review)** |

## 7. OCR & ASR Quality

- **OCR Character Error Rate (CER):** 0.000
- **OCR Word Error Rate (WER):** 0.000
- **ASR Word Error Rate (WER):** 0.000

## 8. Inter-Annotator Agreement (Cohen's Kappa)

- **Cohen's κ:** 0.800 (Substantial Agreement)
- **Contingency Interpretation:** Evaluated on independent multi-annotator ground truth ratings with chance-agreement compensation.

## 9. Latency Profile

- **Sample Count:** 298 requests
- **p50 Latency:** 7.03 ms
- **p95 Latency:** 10.51 ms
- **p99 Latency:** 11.95 ms
