# B1: Image validation, screening fallback and reported hashes

## 0. Your role in one paragraph
You are teammate B. Implement the image module with safe validation, honest NSFW availability, and reported-image hash matching. P1 depends on L1 and L3; the current registry verifies hashes but does not implement ONNX inference.

## 1. Project context
The Harassment Risk Engine is a Rust HTTP service that scores messages, images, audio, and call events using content and directional conversation history. Demo applications, dashboards, and bots are its clients.
IDs are pseudonymized with a server HMAC salt before storage.
Media is processed transiently; retain scores, masked transcripts, OCR text, and hashes rather than uploads.
Analyze only consented material and keep human moderators responsible for decisions.
Every feature is Built, Simulated, or Roadmap; synthetic data, benign proxies, and demo overrides must be disclosed, and any simulated contribution sets simulated=true.

## 2. Repo map and your boundary
Branch b-multimodal/vision. Edit crates/hre-vision/ only. Lead owns core, store, ML, API, shared config and workspace membership; request their adapters, hash persistence and report endpoint rather than modifying them.

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
`crates/hre-ml/src/lib.rs` currently defines:
```rust
pub trait InferenceBackend: Send + Sync {
    fn predict(&self, features: &[f64]) -> Result<f64, ModelError>;
    fn backend(&self) -> Backend;
}
impl ModelRegistry {
    pub fn load(root: &Path) -> Self;
    pub fn report(&self, name: &str) -> ModelReport;
    pub fn degradation(&self) -> Vec<String>;
}
```
These are real signatures; `load` verifies a manifest but reports inference unavailable even for a verified model. Its numeric trait is not an image inference interface. `ReportRequest` from core is:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportRequest { pub event_id: Uuid, pub reason: String }
```

## 4. Exact deliverables
Create `crates/hre-vision/Cargo.toml`, `crates/hre-vision/src/lib.rs`, validation, image backend and hash matching modules with adjacent tests. The following are **proposed interfaces pending Lead agreement**, not existing APIs:
```rust
pub trait ImageBackend: Send + Sync {
    fn analyze(&self, bytes: &[u8]) -> Result<hre_core::Signal, VisionError>;
}
pub fn validate_image(bytes: &[u8], declared_mime: &str) -> Result<(), VisionError>;
pub fn hamming_distance(left: u64, right: u64) -> u32;
```
Agree image-hash representation, configurable distance threshold, model preprocessing, degradation propagation, and report persistence with Lead before integration. Actual report storage and HTTP routes belong to Lead.

## 5. Step-by-step instructions
1. Read prompt sections 5.2, 6, 8 and 15. Inspect current ML and store APIs; ask Lead for approved boundaries.
2. Use existing workspace `hre-core`, `serde = =1.0.219`, `thiserror = =2.0.12`, `sha2 = =0.10.9` where needed. Image decoding, MIME sniffing and pHash dependencies and versions are not yet pinned: request Lead approval and exact pins before adding; do not guess.
3. Enforce <=10 MB, detect actual format, reject mismatched MIME, invalid/truncated image and unreasonable dimensions before allocation.
4. Implement deterministic honest fallback and degradation. A non-model fallback cannot assert it has detected explicit imagery merely from arbitrary bytes or a caption.
5. Compute pHash only from successfully decoded images, compare with configured Hamming threshold, and expose only hashes/scores to Lead's store adapter. Treat reported-hash NCII matching as simulated and human-reviewed.
6. Request Lead integration for reported hashes and demo override handling. Synthetic override must propagate status Simulated and response simulated=true.
7. Keep CPU-heavy decoding/inference off async runtime threads; integration must use spawn_blocking.

## 6. Behavior rules and edge cases
Missing/unloadable model yields deterministic heuristic or unavailable result and a degradation marker, never a fabricated ONNX claim. Invalid input gets a bounded error, no crash. Configure hash threshold; do not guess. Unsolicited explicit media from non-contact can lead to severity floor 61 and blur only through Lead fusion; hash match floor 86 remains simulated.

## 7. Tests you must write
test_rejects_mime_mismatch; test_rejects_truncated_and_oversize; test_missing_model_is_degraded; test_hamming_same_hash_zero_and_threshold_boundary; test_report_match_is_simulated; test_no_pixels_are_persisted; test_decode_limits_prevent_oversized_dimensions.

## 8. Commands
Only use logged recipes. Bash, for a recipe Lead confirms exists:
```sh
just check > target/logs/card-check.log 2>&1; tail -n 40 target/logs/card-check.log; grep -n "error" -A6 target/logs/card-check.log | head -60
```
On Windows use `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 check`; it writes `target/logs/check.log` and `check.exit`. Read the exit file; locally inspect only the last 40 lines and bounded error excerpts. Request missing task-specific logged recipes from Lead; do not pretend an unimplemented recipe tested your work. Never send raw logs in your report.

## 9. Definition of done
- Safe image path and deterministic fallback are verified with benign fixtures.
- Hash API and Lead integration requests are documented; no persistence or real model claim is fabricated.
- Simulator labels and degradation are preserved.

## 10. Do NOT
Do not edit other owners' crates, root workspace files, generated OpenAPI, or shared configuration; request Lead changes through docs/REQUESTS.md. Do not add dependencies not listed or approved; do not use unwrap/expect/panic in non-test code, retain raw media by default, or commit models, datasets, secrets, or raw IDs. Do not claim proposed or missing integration is implemented.

## 11. How to report back
Report at most ten lines: branch; changed files; interface decisions and Lead agreements; exact logged commands; exit codes and test results; behavior verified; degradation/simulation; incomplete work; dependency requests; questions. Commit verified work using a conventional commit.

## 12. If blocked
Write `BLOCKED.md` inside your owned task directory with the precise question, missing owner/interface, and what is safe to continue; stop dependent work. Send Lead the blocker.
