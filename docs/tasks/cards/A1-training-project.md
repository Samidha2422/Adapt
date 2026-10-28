# A1: Training project and dataset inventory

## 0. Your role in one paragraph
You are teammate A, responsible for text, slang, evaluation, and training. This task creates the Python training project and provenance inventory without downloading sensitive data or training a model.

## 1. Project context
The Harassment Risk Engine is a Rust HTTP service that scores messages, images, audio, and call events using content and directional conversation history. Demo applications, dashboards, and bots are its clients.
IDs are pseudonymized with a server HMAC salt before storage.
Media is processed transiently; retain scores, masked transcripts, OCR text, and hashes rather than uploads.
Analyze only consented material and keep human moderators responsible for decisions.
Every feature is Built, Simulated, or Roadmap; synthetic data, benign proxies, and demo overrides must be disclosed, and any simulated contribution sets simulated=true.

## 2. Repo map and your boundary
Branch: a-text/training-project. Edit training/ only. Request shared dependencies or types through docs/REQUESTS.md. Do not edit any Lead, B, or C crate, root Cargo.toml, or generated OpenAPI.

## 3. Prerequisites already done
Read ENGINE_PROMPT.md sections 6, 9, 14, 16, and 17 and all docs/research/*.md. No Rust signature is required by this task. Rust DTO dependencies are intentionally absent: training outputs serialized artifacts consumed later through the model registry.

## 4. Exact deliverables
Create training/pyproject.toml, training/README.md, training/src/hre_training/__init__.py, training/src/hre_training/inventory.py, training/tests/test_inventory.py, and training/datasets.json. The inventory command must emit dataset ID, source URL, license verification state, intended split, language, and whether synthetic. Do not list a dataset as licensed until its original license is verified.

## 5. Step-by-step instructions
1. Read AGENTS.md and docs/DATA.md before editing.
2. Create an installable Python project using uv and the standard library for the initial inventory command; no ML dependency is needed yet.
3. Record Jigsaw, HateXplain, Davidson, TextDetox, RAVDESS, and CREMA-D as candidates. Verify existence and licenses against original sources; record unknown explicitly.
4. Make downloads opt-in and require a destination under gitignored data/raw/.
5. Describe generator A as training only. Generator B belongs in Rust evaluation and must use different structures and parameter ranges.
6. Reserve fresh manually written English, Hindi, Hinglish, and one Marathi/Konkani-labelled subset; do not invent performance results.
7. Document friends' PC prerequisites and CPU fallback. If TRAINING_GUIDE.md is absent, state that clearly instead of making up its content.

## 6. Behavior rules and edge cases
The inventory must work offline. Missing credentials or unaccepted dataset terms must produce a clear skip, never a silent substitute. Do not gather explicit media or actual harassment logs. Do not import training data into held-out evaluation. Never claim that synthetic performance estimates real prevalence.

## 7. Tests you must write
test_inventory_has_provenance asserts every row has source and license state. test_inventory_does_not_download patches network access to fail and checks offline listing still succeeds. test_train_and_eval_split_names_differ prevents accidental reuse of the evaluation partition name.

## 8. Commands
Ask Lead to add a logged train-inventory recipe once the command exists. Use that recipe only. With bash: `just train-inventory > target/logs/a1.log 2>&1; tail -n 40 target/logs/a1.log; grep -n "error" -A6 target/logs/a1.log | head -60`. On Windows use the PowerShell logged runner documented in RUNBOOK.md. Read the .exit file; successful file creation does not prove the command passed.

## 9. Definition of done
- Offline inventory and its tests pass through logged recipes.
- README states source terms, split policy, safe-data rules, and unresolved prerequisites.
- Only training/ is changed and no model, dataset, token, or secret is committed.

## 10. Do NOT
Edit other owners' crates; add unapproved dependencies; auto-download datasets; store raw IDs or retained media; commit models/data; report guessed metrics.

## 11. How to report back
Report at most ten lines: branch, changed paths, exact command, exit status, tests passed, source/license checks, artifacts, incomplete work, next dependency, and questions. Commit verified work with `feat(training): add offline dataset inventory`.

## 12. If blocked
Write training/BLOCKED.md with the exact missing prerequisite and stop dependent work. Keep independent inventory documentation progressing.
