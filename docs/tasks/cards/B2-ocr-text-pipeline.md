# B2: OCR backends, text and PII handoff, CER evaluation

## 0. Your role in one paragraph
You are teammate B. Extract text from screenshots, hand it to A's text and PII pipeline, and evaluate OCR on rendered synthetic images. The board assigns this P2, depending on B1 and A3; this card is prepared early at the human's request.

## 1. Project context
The Harassment Risk Engine is a Rust HTTP service that scores messages, images, audio, and call events using content and directional conversation history. Demo applications, dashboards, and bots are its clients.
IDs are pseudonymized with a server HMAC salt before storage.
Media is processed transiently; retain scores, masked transcripts, OCR text, and hashes rather than uploads.
Analyze only consented material and keep human moderators responsible for decisions.
Every feature is Built, Simulated, or Roadmap; synthetic data, benign proxies, and demo overrides must be disclosed, and any simulated contribution sets simulated=true.

## 2. Repo map and your boundary
Branch b-multimodal/ocr. Edit crates/hre-vision/ only. Do not edit A's hre-text, Lead's API/ML/core or root workspace. Coordinate pipeline and evaluation interfaces with their owners.

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
```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Media {
    pub mime: String,
    #[serde(default)] pub caption: Option<String>,
}
```
This actual core metadata is not OCR output. B1 and A3 are not established as complete; no OcrBackend or text handoff has been verified.

## 4. Exact deliverables
Add OCR modules and tests to `crates/hre-vision/`, plus generated synthetic rendered-text fixtures or instructions within that crate. **Proposed interface pending Lead and A agreement**:
```rust
pub trait OcrBackend: Send + Sync {
    fn recognize(&self, image_bytes: &[u8]) -> Result<String, OcrError>;
}
```
Agree masking, languages, confidence, status/degraded propagation and text handoff before implementation integration. Evaluation must compute character error rate from labeled synthetic references without pretending missing OCR runs.

## 5. Step-by-step instructions
1. Verify B1 image validation and A3's real text/PII contracts have landed; request any missing signatures.
2. Use existing workspace dependencies only; OCR/rendering crates, OCR model files and versions require Lead approval and documented licenses. Candidate backends in prompt are unverified, not pre-approved dependencies.
3. Define validated image input, OCR output and error handling with Lead and A. Invoke actual OCR before passing extracted text to A3 for masking and scoring.
4. Use generated rendered text in English and, if supported by verified backend, Hindi/Devanagari. Label language support and synthetic provenance. Caption is not OCR.
5. Compute CER as edit distance divided by reference character count; define empty-reference behavior explicitly. Record per-language counts and backend/versions, not invented results.
6. If model/backend missing, return unavailable/degraded, no fabricated OCR text; keep image screening independent.

## 6. Behavior rules and edge cases
Reject malformed/oversized images through B1. OCR output must be masked for non-evidence consumers. Missing OCR models are degraded, not successful recognition. Avoid retaining raw pixels by default; do not claim Devanagari support for Latin-only fallback.

## 7. Tests you must write
test_missing_ocr_degraded; test_rendered_latin_extracts_known_text when backend available; test_malformed_image; test_caption_is_not_ocr; test_cer_exact_zero_and_one_edit; test_empty_reference_policy; test_pii_masked_before_display; test_unsupported_language_reported.

## 8. Commands
Only use logged recipes. Bash, for a recipe Lead confirms exists:
```sh
just check > target/logs/card-check.log 2>&1; tail -n 40 target/logs/card-check.log; grep -n "error" -A6 target/logs/card-check.log | head -60
```
On Windows use `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 check`; it writes `target/logs/check.log` and `check.exit`. Read the exit file; locally inspect only the last 40 lines and bounded error excerpts. Request missing task-specific logged recipes from Lead; do not pretend an unimplemented recipe tested your work. Never send raw logs in your report.

## 9. Definition of done
- Actual OCR and A3 handoff are tested after dependencies land, or clearly blocked.
- CER reports sample count, backend, language and synthetic provenance.
- Missing OCR remains honest and safe.

## 10. Do NOT
Do not edit other owners' crates, root workspace files, generated OpenAPI, or shared configuration; request Lead changes through docs/REQUESTS.md. Do not add dependencies not listed or approved; do not use unwrap/expect/panic in non-test code, retain raw media by default, or commit models, datasets, secrets, or raw IDs. Do not claim proposed or missing integration is implemented.

## 11. How to report back
Report at most ten lines: branch; changed files; interface decisions and Lead agreements; exact logged commands; exit codes and test results; behavior verified; degradation/simulation; incomplete work; dependency requests; questions. Commit verified work using a conventional commit.

## 12. If blocked
Write `BLOCKED.md` inside your owned task directory with the precise question, missing owner/interface, and what is safe to continue; stop dependent work. Send Lead the blocker.
