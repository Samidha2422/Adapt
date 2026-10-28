# Training handoff

Snapshot: 2026-10-28. Training is owned by teammate A and the human. No training, export, dataset/model download or GPU job was performed by this sidecar.

The intended implementation guide is training/README.md. The prompt also references a human-provided TRAINING_GUIDE.md; no supplied guide has been verified here. Do not imply that either file or an exported model exists until the owner delivers it. Creating or copying guides outside the owned doc set is outside this assignment.

## Required handoff

1. Check exact base model and dataset licenses before use; record URLs, revision/date, restrictions and attribution in MODELS/DATA.
2. Generate training data with Python generator A; reserve independent Rust generator B plus fresh hand-written edge cases for evaluation. Keep pairs/conversations disjoint.
3. Publish behavior feature names, formulas, units, order and benign baseline in docs/FEATURES.md; deliver golden vectors reproduced by Rust within 1e-4.
4. Train message and call XGBoost candidates and export behavior_msg.onnx and behavior_call.onnx with features.json, label mapping, manifest metadata and checksums.
5. For text fine-tuning, deliver exact base revision/tokenizer, harassment labels, language support and data provenance; binary toxicity must not be represented as seven-label toxicity.
6. Validate framework/ONNX parity with tolerances and held-out quality; calibrate on validation only. Include FPR for benign-heavy cases, C6 victim replies and language slices.
7. Supply immutable candidate artifacts to the human for local placement; lead registry validates before reload. No automatic remote fetch is assumed.

The zero-model fallback is a release requirement and must remain functional independently of training. Synthetic behavior scores do not establish real-world stalking accuracy. Slang adapter training requires moderator-approved contexts, a replay set, a frozen validation precision gate (>2 percentage point drop rejected), version audit and rollback.

## Execution and acceptance

Parent/A own logged just train-* recipes and Luna/log protocol. Exact commands depend on the delivered training project and are not invented here. Required evidence is command/exit, run/config/split IDs, artifact sizes/hashes, parity report and metrics. Build/network availability and runtime compatibility remain unresolved.
