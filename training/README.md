# Harassment Risk Engine – Training & Synthetic Generator A

## Role and Scope

Generator A creates reproducible synthetic message and call event histories for training machine learning models (such as XGBoost behavioral classifiers) and providing baseline calibration.

> [!IMPORTANT]
> **Training Only**: Generator A fixtures are strictly reserved for model training. Independent evaluation is performed by Generator B (in Rust / `hre-eval`) using structurally distinct parameter ranges and distributions to avoid data leakage and circular evaluation.

## Feature Extraction & Golden Vector Parity

Feature extraction in `hre_training.features` reproduces the exact 47-feature vector contract defined in `docs/FEATURES.md` and implemented in `hre-behavior`:

- **Window-level features** (11 features × 3 windows: 1h, 24h, 7d = 33 features):
  `msg_count`, `reply_ratio`, `night_ratio`, `tox_mean`, `tox_max`, `threat_max`, `sexual_max`, `sent_mean`, `pii_mentions`, `location_cue_count`, `unsolicited_media_count`.
- **Global features** (14 features):
  `unreplied_streak`, `initiator_ratio`, `median_gap_s`, `burst_max_10m`, `after_refusal_count`, `since_refusal_hours`, `active_days_streak`, `tox_slope_last20`, `sent_slope_last20`, `repetition_score`, `sender_account_age_days`, `is_contact`, `distinct_channels`, `evasion_similarity`.

The canonical golden vector fixture is exported to `training/fixtures/golden_vector.json` and verified in `test_generator_a.py`.

## Scenario Classes

Generator A models 14 scenario classes:
1. `normal`: low volume, balanced replies, friendly tone
2. `heavy_benign`: couple/friends, high volume (>40 messages, answered calls), reply ratio > 0.4, remains Low tier
3. `polite_stalker`: relentless unreplied messages, continues after block
4. `escalating_stalker`: transitions from polite to guilt-tripping to threats
5. `post_block_contacter`: contacts immediately following a block refusal
6. `pile_on`: 10+ distinct senders targeting one recipient in under a minute
7. `bully`: repeated insults and obscene comments
8. `cyberflasher`: unsolicited explicit media from a stranger
9. `sextortion`: coercion + leak/payment demand cues
10. `silent_caller`: answered calls with duration < 5s (hang-ups)
11. `night_caller`: repeated calls during recipient night window (22:00–06:00)
12. `post_block_caller`: call attempts after a block
13. `new_number_caller`: calls from new accounts (<2 days old)
14. `noisy_ambiguous`: borderline toxicity and quoted phrases

## Commands

Run unit and parity tests:
```bash
python -c "import sys; sys.path.insert(0, 'training/src'); import unittest; unittest.main(module=None, argv=['', 'discover', '-s', 'training/tests', '-p', 'test_*.py'])"
```

Generate synthetic fixtures:
```bash
python training/generator_a.py --seed 42 --output training/fixtures/synthetic_samples.json
```
