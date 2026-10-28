# Training and candidate inventory

Generator A creates deterministic, synthetic numeric message/call histories for training only. It contains no message bodies and demonstrates no language coverage. Independent evaluation requires Generator B and fresh held-out examples. `fixtures/golden_vector.json` checks one feature vector, not broad Rust/Python parity or model accuracy.

`datasets.json` is a candidate inventory, not proof that a dataset was acquired, licensed, or unified. External licenses are unverified; `claimed_license` records prior assertions only. Source and license URLs are discovery pointers, not reviewed evidence. ParaDetox's entry does not establish Russian or Hindi coverage; Jigsaw Toxic Comment and Civil Comments must have separate provenance. The regional evaluation entry reserves a future benchmark.

Downloads require opt-in, an admitted record with license review, and a destination within repository `data/raw`. Download/unification is not implemented; no corpus is fetched. Validation runs offline and checks schema, not legal clearance. Do not commit weights or private data.

`src/hre_training/export_onnx.py` contains **unverified native candidate specs**, not deployment manifests. All candidate licenses fail closed. A license name or model-card URL alone does not authorize export. Export execution and reference/runtime parity are absent: missing weights raise an error after license admission; existing files never produce an `exported` claim. Numeric comparison rejects empty/nonfinite inputs but does not run inference.

See [AUDIT.md](AUDIT.md) for the native-to-deployment mismatch, absent deliverables, and parent/Luna verification handoff. The shared `docs/MODEL_CONTRACT.md` remains authoritative and unchanged.

Parent Luna runs checks through logged repository recipes: `training-test`, `format`, `check`. No raw builds, downloads, training, or benchmarks. Logs and numeric results belong in `target/logs/<recipe>.log` and `.exit`; return a summary of at most 15 lines.
