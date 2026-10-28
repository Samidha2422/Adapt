# Engine integration handoff

Snapshot: 2026-10-28. This handoff records the interrupted integration so another developer/agent can continue safely.

## State right now

- Local branch: `main` at `9e60ed6a57add96adb4cc989b72e925c7563b75a`, matching `origin/main` at the time checked.
- `origin/lead/engine-integration` was `48b281c58b66d0e897c1c442cfe3894afb15d1f9`.
- A merge of `origin/lead/engine-integration` into local `main` is **in progress** (`MERGE_HEAD` is the lead commit above). Do not start a second merge.
- No unresolved conflict paths were reported by `git diff --name-only --diff-filter=U` at handoff time. This does not mean the combined changes have passed verification.
- Many integration changes are staged, and several files have additional unstaged edits (`MM`/`AM` in `git status`). `Cargo.lock` and `docs/openapi.json` also have unstaged changes. Preserve both staged and unstaged work; inspect the diffs before staging/committing.
- Nothing from this integration was committed or pushed. `origin/main` was not changed by this work.

Start by checking the actual current state, since other contributors or agents may have changed it after this snapshot:

```powershell
git status
git diff --name-only --diff-filter=U
git diff --stat
git diff --cached --stat
git diff
git diff --cached
```

Do not use `git reset --hard`, `git clean`, `git merge --abort`, or blanket `ours`/`theirs` resolution: that can discard the already-integrated work. If you decide to abandon the merge, first preserve the complete worktree and index (for example, make a separate copy/patch or branch) and understand the effect of the chosen recovery action.

## What was integrated

The merge combines the newer `main` implementation with engine integration work from `lead/engine-integration`. Main already had the A1-A4/B1-B2 work. The integration adds/updates:

- API call import and media upload paths, test coverage, API README and OpenAPI output.
- Bounded WAV decoding with explicit unavailable-ASR behavior; call CSV parsing and feature extraction.
- Evaluation support, including an independent engine-only evaluator and CLI coverage.
- Text and vision hardening, docs/traceability/limitations, training inventory/export work, and logged verification recipes.

This source integration does **not** mean the engine is fully complete. Real ONNX inference, real ASR, true pixel OCR/NSFW/deepfake inference, and full end-to-end acceptance remain limited by model artifacts/integration and the checks below. Read `docs/ENGINE_VERIFICATION.md`, `docs/LIMITATIONS.md`, `docs/TASKS.md`, and `docs/TRACEABILITY.md` before making completion claims. `HANDOFF.md` contains older snapshot information; use this file plus fresh `git status` as the guide for the interrupted merge.

### Important behavior decision

The API does not pretend that a caption is an audio recording. A JSON analysis request carrying only an audio caption is rejected as unsupported (HTTP 501), and tests were adjusted to require that behavior and avoid treating the rejected audio as evidence. Real image analysis uses uploaded image bytes; it must not manufacture image inference from a caption. Keep provenance and simulation clearly labeled.

## Known verification state at interruption

These are the latest results reported during the interrupted integration, **not a new run against the final post-interruption worktree**:

- `format`: passed.
- `build`: passed.
- `openapi`: passed (but `docs/openapi.json` has an unstaged working-tree modification at handoff).
- `training-test`: passed.
- `models-verify-test`: passed.
- `check`: failed. The latest reported failure was `crates/hre-eval/tests/cli.rs` around line 57: the default evaluator sends caption-only audio through the API and receives HTTP 501. The appropriate fix is to make default evaluation handle unsupported audio honestly (or omit it with an explicit unavailable metric), not to invent a transcript or accept a fake score. An evaluator agent was asked to address this but was interrupted; inspect the current diff to see whether it made a partial fix.
- Earlier checks in the same run also exposed then-fixed issues: an unused import, a call-import test expecting a disabled backend rather than main's `heuristic` backend, and scenario tests expecting 200 for caption-only audio. Verify those fixes still exist in the current combined diff.
- The prior integration audit recorded demo scenario 3 failing because its caption-only image fixture expects image inference. It was not resolved by this merge. Do not claim all nine scenarios pass.

No verification commands were run after the user asked to stop and create this handoff.

## Suggested continuation

1. Inspect staged and unstaged changes, especially the evaluator CLI test, its runner, audio request construction, API scenario tests, `Cargo.lock`, and generated OpenAPI. Check `git diff --check` as a read-only whitespace check.
2. Fix the evaluator's unsupported-audio handling and its test. Preserve explicit unavailable/null metrics and provenance; never synthesize ASR output to satisfy the test.
3. Reconcile generated OpenAPI with the actual API behavior. Keep the bounded call-import envelope and the real multipart image upload contract consistent with the implementation.
4. Use the repository's logged recipes (the wrapper writes `target/logs/<recipe>.log` and `target/logs/<recipe>.exit`):

   ```powershell
   just format
   just check
   just build
   just openapi
   just training-test
   just models-verify-test
   just demo-seed
   ```

   `just check` runs format check, clippy, and workspace tests. For any failure, inspect its corresponding `target/logs/*.log` and `.exit`; rerun the affected check after fixing it. `demo-seed` has a known scenario-3 issue noted above, so report its outcome precisely rather than claiming success. These are repo recipes; if `just` is unavailable, invoke `scripts/just.ps1 <recipe>` from PowerShell.

5. Review `git status`, `git diff --check`, `git diff`, and `git diff --cached` again. Stage only the intended final files, including generated OpenAPI if it matches the implementation. Ensure no unresolved paths remain.
6. Commit the completed merge with both parents intact, e.g. `git commit -m "merge: integrate engine hardening"`. Before pushing, `git fetch origin` and check whether `origin/main` advanced. If it did, integrate that advancement without force-pushing. Then push with `git push origin main`; never use `--force` for this shared main branch.

## Keep completion claims scoped

Passing Rust checks proves the code and tests pass; it does not prove trained model quality or model availability. Report model-dependent tasks as unavailable until the artifacts, manifest hashes/output shapes, runtime loading, and relevant inference checks are present. Keep scenario-3 and real audio limitations visible until fixed with valid input and evidence.
