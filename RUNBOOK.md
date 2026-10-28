# Runbook

Use PowerShell from the repository root. Export the variables in `.env.example`; no dotenv loader is installed. Never commit credentials. SQLite defaults to `target/hre.db`. Copy `config/engine.toml` to select a separate database for demos, then use `scripts/just.ps1 run --config <path>`.

`scripts/just.ps1 bootstrap` installs pinned just 1.40.0 under target/tools only. Add target/tools/bin to the current shell PATH or invoke its executable explicitly. Every just recipe forwards to the same PowerShell runner. Runner failure is a nonzero process exit code and numeric `.exit`; a running job has `RUNNING` in `.exit`.

Luna runs `format`, `check`, `build`, `openapi`; inspect only the relevant log and summarize results. `check` runs fmt, lint and test as logged child recipes. `lock` creates Cargo.lock; keep it in the verified implementation commit. Stop run/mock with Ctrl+C; the server drains gracefully.

Start `mock` or `run` in a dedicated terminal, then `demo-seed` in another. Logs are never sent to API clients. `HRE_ALLOWED_ORIGINS` is a comma-separated origin list, empty by default. Public health/schema routes need no token. Moderator alerts/SSE require bearer authorization; use fetch streaming for browser clients because EventSource cannot supply that header.

Missing models select deterministic rules and explicit degradation. No ONNX/Whisper/OCR inference is claimed until its backend is linked. No ffmpeg/network/model files are required for the P0 behavior path. Media uploads, model reload and reset remain unimplemented. In real mode, SQLite stores assessments, alerts and feed-protection state; alert SSE broadcasts are process-local. After a reconnect or a `resync` event, reload `/v1/alerts` with moderator authorization. Database retention and deletion policies remain unverified.

Luna reports the latest `format`, `check`, `build`, `openapi` and `demo-seed` passed, with 58 tests and zero warnings. The real synthetic scenario results were 1 high/85, 2 high/73 and 7 low/0; ONNX verification fixture tests passed with zero skips.

Platform connectors are separate clients. Before enabling them, re-check Discord Message Content intents, Telegram privacy mode, Meta professional-account permissions, and applicable call-recording policies. Use only owned spaces with informed participants. Upload only recordings the user lawfully holds. This runbook makes no claims about current platform/legal eligibility.
