# Teammate handoff

Start with AGENTS.md, docs/API.md, and docs/openapi.json. The OpenAPI artifact is generated from hre-core types and hre-api route annotations; ask Lead before changing the contract.

| Owner | Branch prefix | Boundary |
|---|---|---|
| Lead | lead/ | hre-core, hre-store, hre-ml, hre-behavior, hre-fusion, hre-api, integration |
| A | a-text/ | hre-text, hre-slang, hre-eval, training/ |
| B | b-multimodal/ | hre-vision, hre-audio, hre-calls |
| C | c-surfaces/ | hre-evidence, hre-dashboard, hre-connectors, hre-cli |

Cards in cards/ define exact prerequisites, interfaces, edge cases, tests, and completion checks. A pending prerequisite means the dependent task cannot be claimed as complete. Do not implement another owner's crate to work around it; write a request in docs/REQUESTS.md.

Run commands through the logged recipes in justfile. The PowerShell bootstrap runner is scripts/just.ps1. Logs belong in target/logs/ and must not be committed. Read exit files before reporting success. Gemini agents may read the final 40 lines and bounded error excerpts locally; send Lead a short result instead of raw logs.

Use Built for a working verified implementation, Simulated for synthetic/proxy demonstrations, and Roadmap for unavailable functionality. Never substitute a caption for actual OCR or user-supplied text for actual ASR without the simulation label. Missing model files must select deterministic fallbacks and identify degraded modules in the response.
