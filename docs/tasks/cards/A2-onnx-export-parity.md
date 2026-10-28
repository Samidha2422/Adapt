# A2: Text, sentiment and embedding exports with parity tests

## 0. Your role in one paragraph
You are teammate A. Export verified text, sentiment and embedding models for later inference and test parity with their source implementations. P1 depends on A1 implementation, which must be verified before starting dependent steps.

## 1. Project context
The Harassment Risk Engine is a Rust HTTP service that scores messages, images, audio, and call events using content and directional conversation history. Demo applications, dashboards, and bots are its clients.
IDs are pseudonymized with a server HMAC salt before storage.
Media is processed transiently; retain scores, masked transcripts, OCR text, and hashes rather than uploads.
Analyze only consented material and keep human moderators responsible for decisions.
Every feature is Built, Simulated, or Roadmap; synthetic data, benign proxies, and demo overrides must be disclosed, and any simulated contribution sets simulated=true.

## 2. Repo map and your boundary
Branch a-text/onnx-export. Edit training/ only. Lead owns registry, manifest integration, models directory policy and shared recipes; request changes. Never edit A3's text crate or commit exported model weights.

## 3. Prerequisites already done
From `crates/hre-core/src/types.rs` (the wire_enum macro derives Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema and uses snake_case serialization):
```rust
wire_enum!(Status { Built, Simulated, Roadmap });
wire_enum!(Backend { Onnx, Heuristic, Disabled, Ggml });
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Signal {
    #[schema(minimum = 0, maximum = 1)] pub score: f64,
    pub label: String,
    pub status: Status,
    pub backend: Backend,
    #[schema(minimum = 0, maximum = 1)] pub confidence: f64,
    pub details: BTreeMap<String, Value>,
}
```
Here `Value` is `serde_json::Value`, and `BTreeMap` is from std.
A1's card exists, but its project, packages and tests have not been established as implemented. `hre-ml` presently verifies model files but has no linked ONNX runtime; a verified hash is not inference.

## 4. Exact deliverables
Add `training/src/hre_training/export_onnx.py`, parity tests and a provenance/usage section in `training/README.md`. Python task: no Rust interface is required. Proposed artifact names, tokenizer inputs, label order, dynamic shapes, tolerances and metadata are **pending Lead agreement**; do not claim compatibility until tested against the actual registry and consumer.

## 5. Step-by-step instructions
1. Verify A1's project and environment; if missing, work on source/license research and block export implementation.
2. Verify upstream model existence, license, label maps and pinned revisions for candidate toxicity, multilingual sentiment and embeddings; do not assume all candidates are usable.
3. Agree exact Python packages and pinned versions with Lead; none are authorized by the current root Cargo.toml. Follow A1 environment conventions and make downloads opt-in.
4. Export to gitignored models directory with associated tokenizer and verified metadata. Keep files out of commits; preserve label ordering from model config.
5. Run representative English, Hindi/Hinglish, empty, long/truncated, Unicode and batch inputs through original and exported model. Compare logits/embeddings and define numerical tolerance per model with Lead.
6. Provide an offline missing-artifact skip with honest status; send model SHA-256, license evidence, revisions, input/output shapes, tolerance and commands to Lead for registry integration.

## 6. Behavior rules and edge cases
Missing files, unavailable dependency, restricted license or failed parity must not yield a success claim. Never fabricate downloads, accuracy metrics, inference availability, or an ONNX backend. No model needed to run offline metadata tests.

## 7. Tests you must write
test_metadata_label_order; test_export_parity_with_fixed_inputs when model exists; test_dynamic_batch_shapes; test_missing_model_skip_is_explicit; test_unverified_license_blocks_export; test_no_model_artifacts_committed.

## 8. Commands
Only use logged recipes. Bash, for a recipe Lead confirms exists:
```sh
just check > target/logs/card-check.log 2>&1; tail -n 40 target/logs/card-check.log; grep -n "error" -A6 target/logs/card-check.log | head -60
```
On Windows use `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 check`; it writes `target/logs/check.log` and `check.exit`. Read the exit file; locally inspect only the last 40 lines and bounded error excerpts. Request missing task-specific logged recipes from Lead; do not pretend an unimplemented recipe tested your work. Never send raw logs in your report.

## 9. Definition of done
- Each exported model has source/license/revision metadata and actual parity evidence, or a documented blocker.
- Artifacts stay uncommitted; Lead receives the agreed integration contract.

## 10. Do NOT
Do not edit other owners' crates, root workspace files, generated OpenAPI, or shared configuration; request Lead changes through docs/REQUESTS.md. Do not add dependencies not listed or approved; do not use unwrap/expect/panic in non-test code, retain raw media by default, or commit models, datasets, secrets, or raw IDs. Do not claim proposed or missing integration is implemented.

## 11. How to report back
Report at most ten lines: branch; changed files; interface decisions and Lead agreements; exact logged commands; exit codes and test results; behavior verified; degradation/simulation; incomplete work; dependency requests; questions. Commit verified work using a conventional commit.

## 12. If blocked
Write `BLOCKED.md` inside your owned task directory with the precise question, missing owner/interface, and what is safe to continue; stop dependent work. Send Lead the blocker.
