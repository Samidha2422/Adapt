# Repository instructions

1. Remove redundant slogans, explanatory text, and minor labels. If removal makes an interface incomprehensible, redesign its layout.
2. Every modification must improve user experience, developer experience, and AI agent usage experience without breaking existing functionality.

Read ENGINE_PROMPT.md and docs/research before implementation. Read the installed rust-best-practices skill before Rust changes.

Run noisy commands only through `just <recipe>` or `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 <recipe>`. All output goes to `target/logs/<recipe>.log`; numeric result goes to `.exit`. Delegate execution and log reading to Luna. Parent receives a summary of at most 15 lines, never raw logs. `bootstrap` installs just only under target/tools. Never run unlogged cargo, downloads, training, or benchmarks.

| Owner | Files |
|---|---|
| Lead | root workspace/runner, hre-core, hre-api, integration |
| Lead sidecar Aristotle | hre-store, hre-ml, hre-behavior, hre-fusion |
| A | hre-text, hre-slang, hre-eval, training |
| B | hre-vision, hre-audio, hre-calls |
| C | hre-evidence, hre-dashboard, hre-connectors, hre-cli |

Never edit another owner's crates. Request changes in docs/REQUESTS.md. Shared integration signatures live in docs/LEAD_HANDOFF.md and actual public Rust source. Parent owns verification and commits; do not stage prompt/research inputs with implementation changes.

Use typed DTOs, exact dependency versions, thiserror in libraries and anyhow in binaries. Prefer borrowed inputs; avoid unnecessary abstraction. No unwrap/expect/panic outside tests or startup. Use tracing without payloads, credentials, or raw IDs. Offload blocking inference/file work with spawn_blocking. Add meaningful tests for validation, privacy, scoring and persistence boundaries. Verify before claiming success.

Feature status is Built, Simulated or Roadmap (snake_case on the wire). All mock results and any contributing proxy/override set simulated=true. Unavailable modules report Roadmap/Disabled. Never claim model inference from hash verification alone. HMAC identities before persistence; retain no raw message/media bodies. Drafts persist nothing and publish no alerts.
