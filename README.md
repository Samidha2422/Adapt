# Harassment Risk Engine

Rust HTTP engine for explainable message and call behavior scoring. See [API](docs/API.md), [runbook](RUNBOOK.md), and [demo](DEMO_SCRIPT.md).

Set `HRE_SALT` (32+ random bytes), `HRE_API_KEYS`, and `HRE_MODERATOR_TOKENS` using [.env.example](.env.example). Environment variables must be exported in the launching shell; `.env` is not automatically loaded.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 bootstrap
./target/tools/bin/just.exe mock
```

Run `just run` for SQLite-backed scoring, `just demo-seed` against a running server, and `just openapi` to regenerate the contract. Without installed just, every recipe is available through `scripts/just.ps1 <recipe>`.

Luna reports `format`, `check`, `build`, `openapi` and `demo-seed` passed after the latest source changes: 58 tests and zero warnings. Real-mode synthetic scenarios 1, 2 and 7 returned high/85, high/73 and low/0 respectively. ONNX verification fixture tests passed with zero skips; [verification records](docs/MODELS.md#artifact-admission) do not establish model artifact validity or inference. ONNX inference is unavailable. Models are optional; unavailable modules are degraded. Mock fixtures are simulated and never persist. The engine prioritizes human review; it cannot access private chats or tap phone calls.
