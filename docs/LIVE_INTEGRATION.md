# Run the engine, dashboard, and Android demo together

Models run on the engine host. The Android app talks to the `aegis-demo` chat server, which calls the engine. The dashboard reads the same engine's stored alerts and runtime health. No model files or engine moderator credentials belong in the APK.

## Engine and dashboard

From the Adapt repository root, set these variables in the terminal that starts the engine:

```powershell
$env:HRE_SALT = '<32-or-more-random-characters>'
$env:HRE_API_KEYS = '<client-key>'
$env:HRE_MODERATOR_TOKENS = '<moderator-token>'
$env:HRE_MODELS_DIR = 'models'
$env:HRE_DEMO_MODE = 'false'
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 run
```

The runner does not automatically load `.env` files. Keep the engine bound to loopback when the chat server runs on the same computer. Open `http://127.0.0.1:8080/dashboard` and sign in with the moderator token. Browser sessions expire after eight hours or server restart; sign-out revokes the session. API tools can keep using the moderator bearer token.

`GET /v1/health/models` requires a client key or moderator token. `hash_verified` means artifact integrity only. `backend: onnx` means the supported runtime loaded the model, checked its tensors and completed a warm-up prediction. Proxy artifacts and unsupported modules retain their degradation reasons.

The fine-tuned text model requires its matching `tokenizer.json` and exact eleven-label order. Behavior models require ordered metadata and baselines in their manifest entries or the transferred `features.json` companion. Missing, incompatible or unverified models select heuristics. Do not replace expected hashes to silence a mismatch without confirming the transferred file's provenance.

The call vector follows the companion's eight-feature order: pair outgoing counts for one hour/day, unanswered count, night count, post-refusal count, answered duration in seconds, distinct callers to the target, and reciprocal calls divided by total calls for the pair. Future and expired events are excluded. Confirm these definitions against the trainer's feature extraction before reporting model accuracy; feature names and tensor shapes alone cannot establish training parity. Call inference supplements the existing deterministic risk rules.

ONNX Runtime must be compatible with the pinned `ort` dependency. Set `ORT_DYLIB_PATH` to a trusted runtime DLL if automatic discovery does not select the appropriate installation. The system Windows DLL may have a different API version. Local model weights, backups, datasets and secrets stay out of Git.

## Chat server and Android

Follow `aegis-demo/docs/RUNBOOK.md` for the chat server's engine URL and key, Android endpoint, and live repository mode. In the chat server terminal set `AEGIS_ENGINE_URL=http://127.0.0.1:8080`, `AEGIS_ENGINE_API_KEY` to an entry from `HRE_API_KEYS`, and `PORT=8081`. The engine URL is a host-side address; the Android URL points to the chat server, not directly to the engine.

The Android live default is `http://10.0.2.2:8081/` for the emulator. For a physical device, use the computer's LAN address for the chat server, or the logged `app-reverse` recipe and `http://127.0.0.1:8081/` over USB. Settings saves the server URL and provides a connection check showing backend/degradation information. Explicit Replay mode remains available for offline demonstrations.

## Verification

All commands use the logged runner and repository delegation protocol:

```powershell
just check
just models-verify
just integration-smoke
```

The opt-in smoke test loads the transferred trained models, checks actual ONNX health, analyzes benign and threatening drafts, and verifies that drafts created no alerts. It is excluded from ordinary tests so contributors without model artifacts can still work. Logs and numeric exit codes are under `target/logs/`.

The dashboard's overview, queue and alert detail use engine data. Historical evaluation reports are not live accuracy measurements, so missing current metrics remain unavailable. Moderator review must not report success without persistence; an unsupported review returns 501. These boundaries are separate from model integration.

## Verification snapshot (2026-10-29)

`check` passed formatting, clippy and 220 Rust tests, with zero failures and six ignored tests. Ordinary API tests use an explicit model-free directory so manually transferred artifacts cannot change their baseline behavior.

`integration-smoke` passed using the transferred artifacts and this machine's ONNX Runtime. It confirmed verified ONNX readiness for text, message behavior and call behavior; two real text draft requests returned non-simulated ONNX results and left the alert store empty. This verifies runtime integration, not prediction accuracy or training parity.

Full Python graph-contract verification is blocked by an incomplete `onnx` installation; the local dependency-install recipe failed because this environment denied access to PyPI. Android verification requires an installed JDK 17 or 21. See each repository's logged checks for the latest code-check outcome.

The chat server's check also stops before compilation: crates.io is inaccessible, and its offline cache lacks `serde_yaml`. No JDK 17/21 was found in the checked installation locations (JRE 11, JDK 25 and JDK 27 are not selected by the Android runner). Restore dependency access/cache and set `AEGIS_JAVA_HOME` to a JDK 17/21, then run `server-check`, `app-test`, `app-build`, and the live `integration-check` from the demo repository. The live check needs both services running and the matching engine key. Device operation and the full two-service path have not been verified here.
