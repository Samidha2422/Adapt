# Lead implementation and feature status

Snapshot: 2026-10-28. The lead workspace contains core, SQLite store, model registry, behavior, fusion and API crates. Luna reports `format`, `check`, `build`, `openapi` and `demo-seed` passed after the latest source changes, with 58 tests and zero warnings. These results do not establish full feature acceptance or model inference.

The real JSON path combines directional history and deterministic behavior and fusion rules. It includes refusal markers, pile-on detection, a victim-reply guard, draft isolation and a minimal two-phrase threat fallback. These are limited lead implementations, not the A-owned text pipeline or trained models. Call-event metadata can enter the behavior rules; call-log import and the B-owned call pipeline are unavailable. SQLite atomically stores an event, assessment, optional alert and feed protection; exact retries return the stored assessment, while conflicting requests are rejected. Alerts and feed protection survive restart; SSE broadcasts are process-local. Drafts do not persist or publish alerts.

Mock fixtures are simulated. Luna reports real-mode synthetic demo scenarios 1, 2 and 7 returned high/85, high/73 and low/0 respectively. The remaining real scenarios require teammate components. The ONNX verification fixture tests passed with zero skips per Luna; ONNX inference is unavailable. See [verification records](MODELS.md#artifact-admission).

The complete 81-row feature, problem, challenge and deliverable matrix remains in [TRACEABILITY.md](TRACEABILITY.md). Its feature-level Roadmap statuses reflect outstanding acceptance and dependencies; they do not deny the partial lead implementations described here. Multimodal pipelines, slang, evaluation, evidence, dashboard, connectors and CLI are not built.

## Message behavior feature vector v1

The model input is `Features::vector()`, not map iteration. All 47 values are f64 before export conversion. Windows are inclusive and end at event time: 1h, 24h and 7d. Empty aggregate values are zero. Future events are excluded. Counts are counts, ratios are fractions, and gap values are seconds.

For each window, in the order 1h, 24h, 7d, append these eleven features with the respective suffix: `msg_count`, `reply_ratio`, `night_ratio`, `tox_mean`, `tox_max`, `threat_max`, `sexual_max`, `sent_mean`, `pii_mentions`, `location_cue_count`, `unsolicited_media_count`. Reply ratio is reverse messages divided by all messages in both directions. Night ratio is the fraction of forward messages in the configured recipient-local night. Means and maxima use forward messages.

Then append, in order: `unreplied_streak`, `initiator_ratio`, `median_gap_s`, `burst_max_10m`, `after_refusal_count`, `since_refusal_hours`, `active_days_streak`, `tox_slope_last20`, `sent_slope_last20`, `repetition_score`, `sender_account_age_days`, `is_contact`, `distinct_channels`, `evasion_similarity`. Missing refusal time and account age are -1. Initiator ratio is a single-session proxy; repetition is exact content-hash equality; evasion similarity is unavailable and currently zero. Night uses a fixed UTC offset, without DST inference. See `hre_behavior::extract_features` for boundary rules and `hre_behavior::feature_order` for the authoritative names.

Golden fixture: at 180000 ms UTC, events at 0, 60000, 120000, 180000 ms have directions A→B, A→B, B→A, A→B and toxicity 0.1, 0.2, 0, 0.3. Their hashes are equal; other metadata defaults. With night 22:00–06:00 the expected vector is:

```json
[3,0.25,1,0.2,0.3,0,0,0,0,0,0,3,0.25,1,0.2,0.3,0,0,0,0,0,0,3,0.25,1,0.2,0.3,0,0,0,0,0,0,1,1,90,3,0,-1,1,0.1,0,1,-1,0,1,0]
```

`golden_vector_v1_matches_documented_fixture` checks the Rust vector within 1e-4. Independent training-side parity remains required before enabling a model. The call feature contract is proposed in [MODEL_CONTRACT.md](MODEL_CONTRACT.md) and requires B's agreement.
