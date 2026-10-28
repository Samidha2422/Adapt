# Decisions

Snapshot: 2026-10-28. Decisions describe scope and tradeoffs; current evidence is recorded in FEATURES and TRACEABILITY.

| ID | Decision | Rationale and consequence |
|---|---|---|
| D01 | Maintain intended and actual status separately | Research Built means target scope; describe partial lead implementations without promoting unfinished feature acceptance. |
| D02 | Preserve all 81 required rows plus research appendices | Nothing is silently dropped. Source group 4 stops at 4.5; do not invent 4.6–4.8. |
| D03 | Preserve human crate ownership | Lead implements its six areas; A/B/C crates stay pending until human teams deliver. |
| D04 | Put temporary walking-skeleton adapters in lead code | Model-free end-to-end work can proceed without editing or claiming teammate modules. |
| D05 | No model downloads or automatic activation | Candidates require exact existence/license/artifact checks; unknown remains unknown. |
| D06 | Report occlusion attribution precisely | Replacing a feature with baseline is occlusion, not exact SHAP or causal evidence. |
| D07 | Keep simulation provenance separate from backend | Heuristic is a backend; fabricated signals, synthetic demo assets and canned mock responses need explicit simulation disclosure. |
| D08 | Treat missing media analysis as unavailable | No silent zero-risk result or invented transcript/OCR from absent dependencies. |
| D09 | Keep legal/platform facts source-scoped and dated | Research has unverified claims. Official sources only; legal applicability and deployment compliance remain unresolved. |
| D10 | Keep documentation updates within the requested file scope | Code and the shared model contract remain owned by their respective agents. |
| D11 | Record ONNX/ggml inconsistency explicitly | ONNX is the general target; native Whisper ggml is an exception, pending implementation decision. |
| D12 | Do not retain explicit media through a generic evidence flag | Hash-only explicit-media rule must not be bypassed; other evidence exceptions need consent/expiry. |
| D13 | Use built/simulated/roadmap for wire status | Disabled is a backend for unavailable signals, not a fourth status; see core DTOs. |
| D14 | Do not equate recommendations with enforcement | External connector actions require real permissions, implementation and tests. |
| D15 | Distinguish prior checks from current verification | Earlier format/lint/tests and OpenAPI have successful exit markers; a fresh check after recent edits is pending. Luna reports verifier tests passed with zero skips; see [verification records](MODELS.md#artifact-admission). |

## Update rule

Record a changed decision with date, exact affected contract/feature IDs and evidence, preserving prior rationale. Breaking contract changes also require CHANGELOG_API and a parent/human notification. For unresolved implementation choices see REQUESTS.

## Dated source corrections — 2026-10-28

D16: Treat unitary/multilingual-toxic-xlm-roberta as a candidate single-output toxicity head; exact publisher config contradicts a seven-label assumption. Its published tested-language list does not establish Indian regional coverage. Do not invent missing labels. See MODELS official-source register.

D17: Use final Gazette commencement text for DPDP Rules, not draft explanatory notes or a generic statement that all rules are in force. Keep Act-section applicability and certificate-law verification open. See PRIVACY_ETHICS sources.

D18: Do not select a replacement model solely to fill a documentation gap. A/human must choose and verify exact alternatives; lead fallback/unavailable behavior remains the walking-skeleton path.
