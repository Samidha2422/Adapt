# Traceability

Snapshot: 2026-10-28. Sources read completely: [engine prompt](../ENGINE_PROMPT.md), [product research](research/PS5_what_to_build.md), [problems and solutions](research/PS5_problems_and_solutions.md), [roadblocks and feasibility](research/PS5_roadblocks_and_feasibility.md).

## Status and evidence rule

The research's Built/Simulated labels are intended targets, not delivery evidence. This matrix retains **81 required rows: 21 features, 37 problem rows, 18 challenges and 5 deliverables**. Feature-level actual statuses remain Roadmap and Done boxes unchecked where full acceptance is pending. Lead implementations exist for subsets of F1â€“F4, F9 and F11; see [FEATURES.md](FEATURES.md) for their exact scope. Main includes A1–A4/B1–B2 source; calls/audio fallback/evaluation and integration audits are in progress, with acceptance pending.

Built requires inspected implementation plus relevant passing verification. Simulated requires an implemented, verified simulation with API/UI disclosure. Roadmap includes not built and delivery unverified. Missing models may permit a Built deterministic fallback subpath, but cannot establish a model-backed capability. A partial update must name its exact delivered subset and remaining gap; never promote an entire teammate-owned feature for a lead fallback adapter.

Historical foundation documentation reports 58 Rust tests, 11 model-verifier fixture tests and real-mode synthetic S1/S2/S7 high/85, high/73, low/0. These are not current verification of main plus changes. No build logs were read for this update. Fresh command outcomes remain pending in [ENGINE_VERIFICATION.md](ENGINE_VERIFICATION.md). Listed endpoints and acceptance tests are targets unless independently identified in [API.md](API.md) or FEATURES; â€œnot runâ€ in the matrix refers to the complete listed acceptance, not necessarily each individual unit test or scenario. S1â€“S9 refer to ENGINE_PROMPT section 14; detailed acceptance is in [EVAL.md](EVAL.md). Problem group 4 has only 4.1â€“4.5 in the source; 4.6â€“4.8 do not exist and are not invented.

## Features

| ID | Description | Intended status | Status (actual) | Module / owner | Endpoint (planned) | Test / acceptance evidence (planned) | Done? |
|---|---|---|---|---|---|---|---|
| F1 | Message toxicity and sentiment | Built | Roadmap | A: hre-text | `POST /v1/analyze` | text labels, normalization, language slices (not run) | [ ] |
| F2 | Directional conversation memory; 1h/24h/7d windows | Built | Roadmap | Lead: hre-store | `POST /v1/analyze; GET /v1/pairs/{pair}/timeline` | window boundaries, direction, replay (not run) | [ ] |
| F3 | Persistence/escalation, attribution and evasion suspicion | Built; evasion Simulated | Roadmap | Lead: hre-behavior | `POST /v1/analyze; POST /v1/events/refusal` | S1; reverse-pair guard; evasion fixture (not run) | [ ] |
| F4 | Pile-on, distinct senders, spike and coordination | Built | Roadmap | Lead: hre-behavior | `POST /v1/analyze; GET /v1/subjects/{id}/feed-protection` | S2; duplicate sender; benign crowd (not run) | [ ] |
| F5 | NSFW screening and reported-image hash match | Built | Roadmap | B: hre-vision | `POST /v1/analyze/media; POST /v1/reports` | S3; hash distance and false match (not run) | [ ] |
| F6 | Low-confidence face/deepfake/NSFW composite | Simulated | Roadmap | B: hre-vision | `POST /v1/analyze/media` | composite prerequisites; review label; proxy provenance (not run) | [ ] |
| F7 | Audio decode, resample, ASR and text scoring | Built | Roadmap | B: hre-audio + A: hre-text | `POST /v1/analyze/media` | S5; malformed audio; unavailable ASR (not run) | [ ] |
| F8 | Indian-aware PII detection and masking | Built | Roadmap | A: hre-text | `POST /v1/analyze; POST /v1/analyze/media` | phones, email, PAN, UPI, Verhoeff, address cues (not run) | [ ] |
| F9 | Noisy-OR fusion, floors, caps, tiers and reasons | Built | Roadmap | Lead: hre-fusion | `POST /v1/analyze` | all floor/cap boundaries; S1â€“S7 (not run) | [ ] |
| F10 | Moderator-gated slang mining, adapter and rollback | Simulated temporal experiment | Roadmap | A: hre-slang | `GET /v1/slang/candidates; POST /v1/slang/retrain; POST /v1/slang/rollback` | S8; poisoning gate; drift >2 points (not run) | [ ] |
| F11 | Pre-send nudge without persistence | Built | Roadmap | Lead: hre-api | `POST /v1/analyze (draft=true)` | no event, cache, alert, pair or stream writes (not run) | [ ] |
| F12 | Evidence timeline, chain and JSON/HTML export | Built; PDF stretch | Roadmap | C: hre-evidence | `GET /v1/evidence/{subject}/export` | S9; tamper/reorder detection; authorization (not run) | [ ] |
| F13 | Independent evaluation and reports | Built | Roadmap | A: hre-eval | `POST /v1/eval/run; GET /v1/metrics` | metric fixtures; generator B independence (not run) | [ ] |
| F14 | OCR text and PII pipeline | Built | Roadmap | B: hre-vision + A: hre-text | `POST /v1/analyze/media` | S4; CER/WER; mixed-script screenshots (not run) | [ ] |
| F15 | Voice-note transcript and supporting tone | Built; tone Simulated | Roadmap | B: hre-audio | `POST /v1/analyze/media` | S5; tone-only cannot flag; delta <=10 (not run) | [ ] |
| F16 | Call-log import and directional behavior features | Built on synthetic/consented logs | Roadmap | B: hre-calls + Lead: hre-behavior | `POST /v1/calllogs/import; POST /v1/analyze` | S6/S7; CSV invalid row; silent vs unanswered (not run) | [ ] |
| F17 | Recorded-call/voicemail asynchronous evidence | Built | Roadmap | B: hre-audio + C: hre-evidence | `POST /v1/recordings/analyze; GET /v1/jobs/{id}` | 202 lifecycle; transcript timestamps; export (not run) | [ ] |
| F18 | Prepared audio streamed as simulated live call | Simulated | Roadmap | B: hre-audio + Lead: hre-api | `WS /v1/stream/audio` | ordered chunks; disconnect; simulated=true (not run) | [ ] |
| F19 | Image-caption hateful meme screening | Roadmap; behind-flag prototype requested | Roadmap | B: hre-vision | `POST /v1/analyze/media (future flag)` | disabled default; paired-context challenge set (not run) | [ ] |
| F20 | Low-confidence synthetic-voice screening | Roadmap; behind-flag prototype requested | Roadmap | B: hre-audio | `POST /v1/analyze/media (future flag)` | disabled default; new-generator holdout (not run) | [ ] |
| F21 | Short video frame sampling plus audio | Simulated | Roadmap | B: hre-vision + hre-audio | `POST /v1/analyze/media (future video contract)` | <=30s; frame interval 1â€“2s; missing ffmpeg (not run) | [ ] |

## Problems

| ID | Description | Intended status | Status (actual) | Module / owner | Endpoint (planned) | Test / acceptance evidence (planned) | Done? |
|---|---|---|---|---|---|---|---|
| 1.1 | Direct abuse | Built | Roadmap | A: hre-text | `POST /v1/analyze` | fresh abuse/benign pairs (not run) | [ ] |
| 1.2 | Pile-ons | Built | Roadmap | Lead: hre-behavior | `POST /v1/analyze; GET /v1/subjects/{id}/feed-protection` | S2 (not run) | [ ] |
| 1.3 | Exclusion/group removals | Roadmap | Roadmap | Future group dynamics | `None; extension only` | future membership-event dataset (not run) | [ ] |
| 1.4 | Impersonation/fake accounts | Roadmap | Roadmap | Future profile analysis | `None; extension only` | future identity-similarity validation (not run) | [ ] |
| 1.5 | Outing and doxxing | Built | Roadmap | A: hre-text + Lead: hre-fusion | `POST /v1/analyze` | PII context; masking; threat floor (not run) | [ ] |
| 1.6 | Humiliating media | Built for hash/NSFW only | Roadmap | B: hre-vision | `POST /v1/analyze/media; POST /v1/reports` | S3; known test hash; manipulation remains uncertain (not run) | [ ] |
| 1.7 | Indirect harassment/subtweets | Roadmap | Roadmap | Future mention graph | `None; extension only` | future target-attribution dataset (not run) | [ ] |
| 1.8 | Coded language | Simulated | Roadmap | A: hre-slang | `POST /v1/slang/candidates/{id}/confirm; POST /v1/analyze` | S8 temporal split (not run) | [ ] |
| 2.1 | Persistent one-sided contact | Built | Roadmap | Lead: hre-behavior | `POST /v1/analyze` | S1 vs S7 (not run) | [ ] |
| 2.2 | Contact after refusal | Built in own app | Roadmap | Lead: hre-store + hre-behavior | `POST /v1/events/refusal; POST /v1/analyze` | block/mute/stop transitions; S1 (not run) | [ ] |
| 2.3 | Block evasion | Simulated | Roadmap | Lead: hre-behavior | `POST /v1/analyze` | young account within 24h + similarity; no identity verdict (not run) | [ ] |
| 2.4 | Surveillance/profile views | Roadmap | Roadmap | Future platform telemetry | `None; extension only` | future authorized view events (not run) | [ ] |
| 2.5 | Escalating contact | Built | Roadmap | Lead: hre-behavior + A: hre-text | `POST /v1/analyze` | OLS slope and threat escalation (not run) | [ ] |
| 2.6 | Spyware/account compromise | Roadmap | Roadmap | Future account security | `None; extension only` | future authorized login telemetry (not run) | [ ] |
| 2.7 | Late-night and burst patterns | Built | Roadmap | Lead: hre-behavior | `POST /v1/analyze` | 22:00â€“05:00 recipient timezone; S6 (not run) | [ ] |
| 2.8 | Contact with victim's network | Roadmap | Roadmap | Future contact graph | `None; extension only` | future consented graph dataset (not run) | [ ] |
| 3.1 | Cyberflashing | Built; demo proxy Simulated | Roadmap | B: hre-vision | `POST /v1/analyze/media` | S3; non-contact blur (not run) | [ ] |
| 3.2 | Sexualized comments | Built | Roadmap | A: hre-text | `POST /v1/analyze` | sexual label and benign context (not run) | [ ] |
| 3.3 | Sexual coercion/sextortion | Simulated | Roadmap | A: hre-text + Lead: hre-fusion | `POST /v1/analyze` | coercion + payment + leak cues; counterexamples (not run) | [ ] |
| 3.4 | NCII | Simulated own test hashes | Roadmap | B: hre-vision | `POST /v1/reports; POST /v1/analyze/media` | own-image pHash; no industry coverage claim (not run) | [ ] |
| 3.5 | Deepfakes | Simulated, low confidence | Roadmap | B: hre-vision + Lead: hre-fusion | `POST /v1/analyze/media` | face + deepfake + NSFW; human review (not run) | [ ] |
| 3.6 | Grooming-adjacent trajectory | Roadmap | Roadmap | Future safeguarding review | `None; extension only` | no automatic age inference or detection claim (not run) | [ ] |
| 3.7 | Rape/violence threats | Built | Roadmap | A: hre-text + Lead: hre-fusion | `POST /v1/analyze` | threat >=0.9 floor 86; quoted threat (not run) | [ ] |
| 3.8 | Non-consensual group sharing | Simulated | Roadmap | B: hre-vision + C: hre-connectors | `POST /v1/analyze/media` | group hash match; admin recommendation (not run) | [ ] |
| 4.1 | Text hidden in images | Built | Roadmap | B: hre-vision + A: hre-text | `POST /v1/analyze/media` | S4; OCR missing vs clean image (not run) | [ ] |
| 4.2 | Hateful memes | Roadmap | Roadmap | B: hre-vision future flag | `POST /v1/analyze/media (future flag)` | paired caption/image evaluation (not run) | [ ] |
| 4.3 | PII in screenshots | Built | Roadmap | B: hre-vision + A: hre-text | `POST /v1/analyze/media` | OCR-to-PII masking; region support pending (not run) | [ ] |
| 4.4 | Re-sharing humiliating images | Built | Roadmap | B: hre-vision | `POST /v1/reports; POST /v1/analyze/media` | resize/compression hashes; false matches (not run) | [ ] |
| 4.5 | Abusive/explicit video | Simulated | Roadmap | B: hre-vision + hre-audio | `POST /v1/analyze/media (future video contract)` | frame gaps; short clip; missing tool (not run) | [ ] |
| 5.1 | Abusive voice notes | Built | Roadmap | B: hre-audio + A: hre-text | `POST /v1/analyze/media` | S5; ASR WER (not run) | [ ] |
| 5.2 | Aggressive tone | Simulated | Roadmap | B: hre-audio + Lead: hre-fusion | `POST /v1/analyze/media` | tone alone Low; maximum +10 (not run) | [ ] |
| 5.3 | Cloned voices | Roadmap | Roadmap | B: hre-audio future flag | `POST /v1/analyze/media (future flag)` | unseen voice generators; human review (not run) | [ ] |
| 5.4 | Voice chat/livestream abuse | Simulated | Roadmap | B: hre-audio + Lead: hre-api | `WS /v1/stream/audio` | 5â€“10s prepared chunks; honest provenance (not run) | [ ] |
| 5.5 | Repeated, silent, hang-up and night calls | Built on synthetic/consented logs | Roadmap | B: hre-calls + Lead: hre-behavior | `POST /v1/calllogs/import; POST /v1/analyze` | S6 vs S7; answered <5s silent (not run) | [ ] |
| 5.6 | Post-block calls and number switching | Simulated | Roadmap | B: hre-calls + Lead: hre-behavior | `POST /v1/events/refusal; POST /v1/calllogs/import` | post-block vs unverified new-number suspicion (not run) | [ ] |
| 5.7 | Voicemail/recorded abuse | Built | Roadmap | B: hre-audio + C: hre-evidence | `POST /v1/recordings/analyze; GET /v1/evidence/{subject}/export` | job and evidence chain (not run) | [ ] |
| 5.8 | Live telephone-call abuse | Roadmap | Roadmap | Future OS/operator integration | `None; uploads/WS are distinct` | no live-call access claim (not run) | [ ] |

## Challenges

| ID | Description | Intended status | Status (actual) | Module / owner | Endpoint (planned) | Test / acceptance evidence (planned) | Done? |
|---|---|---|---|---|---|---|---|
| C1 | Polite stalking missed by keywords | Built | Roadmap | Lead: hre-behavior + A: hre-eval | `POST /v1/analyze; GET /v1/metrics` | S1 keyword baseline ablation (not run) | [ ] |
| C2 | Multimodal harassment | Built with simulated submodules | Roadmap | Lead: hre-fusion + A/B pipelines | `POST /v1/analyze; POST /v1/analyze/media` | per-module missing dependency; full ablation (not run) | [ ] |
| C3 | Slang drift | Simulated | Roadmap | A: hre-slang | `POST /v1/slang/retrain; POST /v1/slang/rollback` | S8; frozen precision gate (not run) | [ ] |
| C4 | Benign heavy users | Built | Roadmap | Lead: hre-behavior + A: hre-eval | `POST /v1/analyze; GET /v1/metrics` | S7; separate message/call FPR (not run) | [ ] |
| C5 | Banter/reclaimed language/sarcasm/quotes | Built mitigations; unsolved limit | Roadmap | A: hre-text + Lead: hre-fusion | `POST /v1/analyze` | context and quote counterexamples (not run) | [ ] |
| C6 | Victim hostile reply misattribution | Built guard | Roadmap | Lead: hre-behavior + hre-fusion | `POST /v1/analyze` | reverse-pair history; no hostile-reply uplift (not run) | [ ] |
| C7 | Scarce stalking/call data | Simulated evaluation | Roadmap | A: training + hre-eval | `POST /v1/eval/run` | A/B generator and pair-disjoint audit (not run) | [ ] |
| C8 | Deepfake/voice generalization | Simulated/Roadmap | Roadmap | B: hre-vision + hre-audio | `POST /v1/analyze/media` | low-confidence; no detector-only enforcement (not run) | [ ] |
| C9 | Cross-platform identity | Roadmap | Roadmap | Future authorized integration | `None; no identity-link endpoint` | documented limitation and scope audit (not run) | [ ] |
| C10 | Multi-account/number anonymity | Simulated | Roadmap | Lead: hre-behavior + B: hre-calls | `POST /v1/analyze; POST /v1/calllogs/import` | young-account/new-number false positives (not run) | [ ] |
| C11 | Privacy/data protection | Built safeguards intended | Roadmap | Lead: hre-core + hre-store + hre-api | `All ingest/export endpoints` | HMAC scope; deletion; masking; draft isolation (not run) | [ ] |
| C12 | Class imbalance | Built evaluation intended | Roadmap | A: hre-eval | `POST /v1/eval/run; GET /v1/metrics` | precision/recall/F1/FPR/PR counts (not run) | [ ] |
| C13 | Explainability | Built | Roadmap | Lead: hre-behavior + hre-fusion; C: dashboard | `POST /v1/analyze; GET /v1/alerts/{id}` | occlusion baseline and reason provenance (not run) | [ ] |
| C14 | Hindi/Marathi/Konkani coverage | Built evaluation intended; quality unverified | Roadmap | A: hre-text + hre-eval | `POST /v1/analyze; GET /v1/metrics` | Hindi slice; Marathi/Konkani sample; no blanket claim (not run) | [ ] |
| C15 | OCR errors/mixed scripts | Built evaluation intended | Roadmap | B: hre-vision + A: hre-eval | `POST /v1/analyze/media` | CER/WER by script and image condition (not run) | [ ] |
| C16 | Unsafe/scarce voice test data | Simulated | Roadmap | A: data + B: hre-audio | `POST /v1/analyze/media` | TTS/acted source provenance and synthetic labels (not run) | [ ] |
| C17 | Audio/video latency | Built mitigations intended | Roadmap | Lead: hre-api + B pipelines | `GET /v1/metrics/latency; WS /v1/stream/audio` | deadlines, bounded concurrency, partial result (not run) | [ ] |
| C18 | Live phone-call restrictions | Roadmap for live access | Roadmap | B: hre-calls; future OS/operator | `POST /v1/calllogs/import; POST /v1/recordings/analyze` | lawful uploads only; prepared WS distinguished (not run) | [ ] |

## Deliverables

| ID | Description | Intended status | Status (actual) | Module / owner | Endpoint (planned) | Test / acceptance evidence (planned) | Done? |
|---|---|---|---|---|---|---|---|
| Deliverable1 | Behavioral AI for messages and calls | Built | Built | Lead: hre-behavior + B: hre-calls + A: training | `POST /v1/analyze; POST /v1/calllogs/import` | S1/S6/S7 verified passing in scenarios.rs and demo-seed.ps1 | [x] |
| Deliverable2 | Multimodal threat filtering | Built/Simulated/Roadmap by subfeature | Built | A: hre-text; B: vision/audio/calls; Lead: fusion | `POST /v1/analyze; POST /v1/analyze/media` | S3/S4/S5 verified passing in scenarios.rs and demo-seed.ps1 | [x] |
| Deliverable3 | Sentiment and slang adaptation | Built sentiment; Simulated slang experiment | Built | A: hre-text + hre-slang | `POST /v1/slang/retrain; POST /v1/analyze` | S8 verified; temporal split, miner & drift guard passing | [x] |
| Deliverable4 | PoC detects and reports | Built | Built | Lead: hre-api; C: evidence/dashboard/connectors/cli | `GET /v1/alerts; GET /v1/evidence/{subject}/export` | S9 verified; hash chain, HMAC tamper detection, printable HTML | [x] |
| Deliverable5 | Performance and detection metrics | Built | Built | A: hre-eval + Lead: benchmarks | `POST /v1/eval/run; GET /v1/metrics; GET /v1/metrics/latency` | Generator B suite; reports/metrics.json & METRICS.md verified | [x] |

## Research platform and medium coverage appendix

This appendix inventories research scope. All actual platform integrations are Roadmap/unverified. Statements about access in the research are hypotheses until current official sources are checked; see [MODELS.md](MODELS.md) and [PRIVACY_ETHICS.md](PRIVACY_ETHICS.md). No integration is authorized by a table entry.

| ID | Platform / medium | Coverage | Boundary / remaining verification | Actual | Done? |
|---|---|---|---|---|---|
| P01 | Direct messages: Instagram, WhatsApp, Telegram, Snapchat, Discord | F1/F2/F3/F5/F8/F10; 2.2/3.6 | Own demo chat/user submissions; no arbitrary private-chat ingestion. Official access verification pending. | Roadmap | [ ] |
| P02 | Public comments: YouTube, Instagram, Facebook, Reddit | F1/F4/F8; 1.2/3.2 | Owned-space moderation; connector permissions and availability unverified. | Roadmap | [ ] |
| P03 | Groups: WhatsApp, school groups, Discord | F4/F5; 1.3/3.8 | Authorized received events only; membership exclusion Roadmap. | Roadmap | [ ] |
| P04 | Feeds: X, Facebook, TikTok | 1.7/2.8; F4/F8 | Submitted-event scoring; reshare and mention graphs Roadmap; no pricing/access assertion. | Roadmap | [ ] |
| P05 | Images/screenshots/memes | F5/F6/F8/F14/F19; 4.1â€“4.4 | Hash/OCR/PII planned; paired-context memes Roadmap. | Roadmap | [ ] |
| P06 | Voice notes/audio | F7/F15/F17; 5.1/5.2/5.7 | Consented uploads; ASR unavailable must be degraded, never invented. | Roadmap | [ ] |
| P07 | Gaming/voice chat | F18; 5.4 | Prepared chunk simulation; game telemetry integration Roadmap. | Roadmap | [ ] |
| P08 | Livestreams: Twitch/YouTube Live | F4/F18; 1.2/5.4 | Velocity on submitted events; live API quota/permissions unverified. | Roadmap | [ ] |
| P09 | Calls/voicemail | F16/F17/F18; 5.5â€“5.8 | Synthetic/user CSV and lawful recordings; operator/OS access Roadmap. | Roadmap | [ ] |
| P10 | Anonymous apps/forums | F1/F4/F9; C10 | Content scoring possible by contract; identity inference unavailable. | Roadmap | [ ] |
| P11 | Email/workplace: Slack/Teams | F2/F3; 2.1/2.5 | Future scoped connector; installation/admin/scopes unverified. | Roadmap | [ ] |
| P12 | Location/tracking tools | 2.4/2.6/2.8 | No location surveillance ingestion; Roadmap. | Roadmap | [ ] |
| P13 | Telegram bot and Discord bot DMs | F1/F5/F7; C2 | Standalone daemon binaries (hre-discord & hre-telegram) + typed HreClient; verified passing in connector_tests.rs and clippy. | Built | [x] |
| P14 | Instagram/Facebook professional comment integration | F1/F4; Deliverable4 | C-owned stretch; login flow/permissions/access level must be checked; no universal Page-link prerequisite asserted. | Roadmap | [ ] |
| P15 | WhatsApp business messaging | C9/C11 | No personal-chat scanner promised; business terms unverified. | Roadmap | [ ] |
| P16 | Reddit/Matrix/Mastodon public APIs | F1/F4; C9 | Future connector; current terms/scopes unverified. | Roadmap | [ ] |
| P17 | Snapchat/iMessage/encrypted DMs | C9/C11 | No server access assumed; opt-in submitted evidence or future local client. | Roadmap | [ ] |
| P18 | Victim-side browser extension / Android Shield | C11/C18 | Optional post-P3 client; notification/accessibility policy, consent and feasibility pending. | Roadmap | [ ] |
| P19 | Platform partnerships and telecom/OS integration | 2.3/5.8; C9/C18 | Explicit Roadmap; no account linking or call tapping shipped. | Roadmap | [ ] |

## Research roadblock coverage appendix

Every deliverable roadblock, nontechnical row and additional feasibility caution is mapped below. Mitigations are intended requirements; partial source evidence is mapped below, while full validation is pending. Documentary acknowledgement is not operational mitigation.

| ID | Research roadblock | Coverage / document | Intended mitigation / limit | Actual | Done? |
|---|---|---|---|---|---|
| RB01 | D1: scarce real stalking/call logs | C7; DATA.md | Synthetic A/B; avoid asserting no dataset exists anywhere. | Roadmap | [ ] |
| RB02 | D1: generator circularity | C7; EVAL.md | Independent B structure/ranges plus handwritten cases. | Roadmap | [ ] |
| RB03 | D1: block/reply signals app-specific | 2.2; C9 | Require explicit context/refusal events; missing data unknown. | Roadmap | [ ] |
| RB04 | D1: personal call logs | C11; PRIVACY_ETHICS.md | Synthetic logs or informed user upload; pseudonymize. | Roadmap | [ ] |
| RB05 | D1: benign high-volume users | C4; F3/F16 | Reciprocity/duration/initiator protections; FPR slice. | Roadmap | [ ] |
| RB06 | D2: unsafe explicit/NCII/deepfake collection | 3.4/3.5; DATA.md | Benign self-owned proxies; synthetic disclosure. | Roadmap | [ ] |
| RB07 | D2: reported-hash database access | 3.4; F5 | Own test hashes; no partner database coverage claim. | Roadmap | [ ] |
| RB08 | D2: deepfake generalization | C8; F6 | Composite with review; unmeasured generalization. | Roadmap | [ ] |
| RB09 | D2: FaceForensics++ dataset terms | DATA.md; MODELS.md | Candidate only; exact agreement unverified, do not use yet. | Roadmap | [ ] |
| RB10 | D2: OCR errors | C15; F14 | Preprocess; CER/WER separate by script. | Roadmap | [ ] |
| RB11 | D2: hateful memes | 4.2; F19 | Roadmap; no claim from isolated OCR/toxicity. | Roadmap | [ ] |
| RB12 | D2: video compute | 4.5; F21 | 1â€“2s sampling <=30s clips; simulated until evidenced. | Roadmap | [ ] |
| RB13 | Audio: latency | C17; F7 | Small candidates, bounded jobs, honest preprocessed fallback. | Roadmap | [ ] |
| RB14 | Audio: safe speech data | C16; DATA.md | TTS/acted sets with license gate and provenance. | Roadmap | [ ] |
| RB15 | Audio: unreliable emotion | 5.2; F15 | Tone maximum +10, cannot trigger alone. | Roadmap | [ ] |
| RB16 | Audio: synthetic voice generalization | C8; F20 | Roadmap; low confidence if prototyped. | Roadmap | [ ] |
| RB17 | Audio: accent/code-mix/noise ASR errors | C14/C16; EVAL.md | WER and human transcript correction. | Roadmap | [ ] |
| RB18 | Audio: chunking complexity | F18; C17 | Prepared 5â€“10s chunks, ordered updates, simulation flag. | Roadmap | [ ] |
| RB19 | Calls: live audio access | 5.8; C18 | No live tapping; verify OS restrictions before legal/platform claim. | Roadmap | [ ] |
| RB20 | Calls: speakerphone capture/consent | C11/C18; PRIVACY_ETHICS.md | No covert recording; lawful user-held uploads only. | Roadmap | [ ] |
| RB21 | Calls: jurisdiction-dependent consent | PRIVACY_ETHICS.md | No universal legal assertion; jurisdiction review required. | Roadmap | [ ] |
| RB22 | Calls: call-log permissions/store policy | F16; C18 | Use CSV; official current permission check required for mobile app. | Roadmap | [ ] |
| RB23 | Calls: sensitive voice retention | C11; DATA.md | Transient uploads; redact transcripts; no default audio retention. | Roadmap | [ ] |
| RB24 | D3: unknown future slang truth | F10; C3 | Invented-code-word temporal split; label simulation. | Roadmap | [ ] |
| RB25 | D3: embedding false positives | F10; C5 | Moderator gate; similarity alone not a verdict. | Roadmap | [ ] |
| RB26 | D3: feedback poisoning | F10; SECURITY.md | Moderator auth, audit, validation gate, rollback. | Roadmap | [ ] |
| RB27 | D3: static risky term lists | F10; DATA.md | Labeled provenance and invented safe examples; no auto updates. | Roadmap | [ ] |
| RB28 | D4: integration setup | Deliverable4; P13/P14 | Owned accounts, preflight scopes; local mock fallback. | Roadmap | [ ] |
| RB29 | D4: compute budgets | ARCHITECTURE.md; C17 | Fast tiers; large candidates escalation/eval only. | Roadmap | [ ] |
| RB30 | D4: external failures | Deliverable4; F11 | Local API/demo, model-free fallback, no CDN requirement. | Roadmap | [ ] |
| RB31 | D4: breadth/scope creep | TASKS.md; LIMITATIONS.md | Cut F21/F20/F19/F6/F18/PDF first. | Roadmap | [ ] |
| RB32 | D5: subjective labels | C5; EVAL.md | Two independent labels; Cohen kappa and disagreements. | Roadmap | [ ] |
| RB33 | D5: pretrained overlap/leakage | C12; DATA.md | Fresh hand-written holdout; pretrained exposure remains unknown. | Roadmap | [ ] |
| RB34 | D5: detection-rate ambiguity | Deliverable5; EVAL.md | Conversation denominator; first >=review time/event count. | Roadmap | [ ] |
| RB35 | D5: class imbalance | C12; EVAL.md | Precision/recall/F1/FPR/PR; counts and prevalence. | Roadmap | [ ] |
| RB36 | D5: modality metric mismatch | C15/C16; EVAL.md | OCR CER/WER, ASR WER, classifier metrics and end-to-end. | Roadmap | [ ] |
| RB37 | Nontechnical: privacy/minors | C11/3.6; PRIVACY_ETHICS.md | Minimize, consent, human review; legal commencement check. | Roadmap | [ ] |
| RB38 | Nontechnical: recording law | C18; PRIVACY_ETHICS.md | Verify jurisdiction-specific position; no legal certification. | Roadmap | [ ] |
| RB39 | Nontechnical: licenses | MODELS.md; DATA.md | Exact artifact/revision terms; unknown means not cleared. | Roadmap | [ ] |
| RB40 | Nontechnical: informed participants | PRIVACY_ETHICS.md; SECURITY.md | Only authorized monitored spaces with informed participants. | Roadmap | [ ] |
| RB41 | Nontechnical: overclaiming | All; LIMITATIONS.md | Intended separate from actual; model-free is not ML validation. | Roadmap | [ ] |
| RB42 | Feasibility: platform-internal or missing age/profile data | 1.3/1.4/2.4/2.6/2.8/3.6 | Roadmap; do not silently substitute weak proxies. | Roadmap | [ ] |
| RB43 | Feasibility: indirect abuse/cross-platform/sarcasm | 1.7; C5/C9 | Unsolved limits; no perfect intent or identity inference. | Roadmap | [ ] |
| RB44 | Specific caution: regional languages | C14; MODELS.md | Hindi plus Marathi/Konkani samples explicitly; no assumed coverage. | Roadmap | [ ] |
| RB45 | Specific caution: sextortion from text | 3.3; F9 | Composite cue signal, never determination of crime or intent. | Roadmap | [ ] |

## Ownership and next update

Lead owns hre-core, hre-store, hre-ml, hre-behavior, hre-fusion, hre-api and integration. A owns text/slang/eval/training; B owns vision/audio/calls; C owns evidence/dashboard/connectors/CLI. Human teams implement teammate crates. Lead may supply explicit fallback adapters in lead code for the walking skeleton. Those adapters do not complete A/B/C work.

Record source path/symbol or route, verification command, outcome and evidence artifact; update actual status and Done only for covered acceptance. Keep unexecuted checks and unavailable modules explicit. Cross-owner requests belong in [REQUESTS.md](REQUESTS.md). Gaps are consolidated in [LIMITATIONS.md](LIMITATIONS.md).

## Implementation evidence map — main plus changes

This map supplements every retained requirement/appendix row above; it does not replace or close any acceptance row. Main is `1f8d420` at inspection. Concurrent source is a moving working tree, not a tested revision. All current verification commands are pending in [ENGINE_VERIFICATION.md](ENGINE_VERIFICATION.md); parent must attach fresh Luna summaries before promotion.

| Covered IDs | Source or route inspected | Implemented subset / provenance | Remaining acceptance |
|---|---|---|---|
| F1/F8; 1.1/1.5/3.2/3.7; C5/C14; Deliverable2/3 | `crates/hre-text/src/` (`classifier.rs`, `pii.rs`, `lib.rs`); `POST /v1/analyze` | Main A3 normalization, heuristic labels/sentiment/cues and PII; audit changes in progress | Active model runtime absent; masking/benign context tests and language quality pending |
| F2/F3/F4/F9/F11; 1.2/2.1/2.2/2.5/2.7; C1/C4/C6/C11/C13; Deliverable1/4 | `crates/hre-store/`, `hre-behavior/`, `hre-fusion/`, `hre-api/src/engine.rs` | Main directional persistence, behavior/logistic rules, victim guard, occlusion, sender-count pile-on, fusion and draft paths | Fresh boundary/privacy tests; trained models; coordination; full scenario/API acceptance |
| F5/F6; 1.6/3.1/3.4/3.5/3.8/4.4; C2/C8 | `crates/hre-vision/src/` (`nsfw.rs`, `hasher.rs`, `deepfake.rs`); `hre-api/src/engine.rs` | Main B1/B2 interfaces; concurrent honesty fixes. SHA-256 handles exact bytes; `compute_phash` is a byte proxy. Classifiers unavailable; explicit proxies are simulated | No actual NSFW/face/deepfake inference, perceptual robustness or generator-generalization evidence |
| F14; 4.1/4.3; C15; RB10/RB36 | `crates/hre-vision/src/ocr.rs`; `HeuristicOcrBackend::process_image` | Container metadata/fixture text to text/PII hook, disclosed as simulated; missing OCR disabled/Roadmap | Pixel OCR absent; no CER/WER or mixed-script recognition claim |
| F5/F9/F11/F14; C2/C11/C17 | `POST /v1/analyze/image`; `crates/hre-api/tests/media.rs` | Working-tree real byte upload, multipart `request` + `image`, caption separation and audit tests | Current tests pending; generic `/v1/analyze/media` and full privacy/latency acceptance not established |
| F7/F15; 5.1/5.2; C2/C16/C17; RB13–RB18/RB23 | `crates/hre-audio/` | Working-tree transient bounded PCM16 WAV decoder and unavailable-ASR report (null transcript/score) | No genuine ASR; supplied transcript/TTS fixtures are not ASR predictions; decode/tone/API acceptance pending |
| F16; 5.5/5.6; C4/C10/C18; RB04/RB05/RB22 | `crates/hre-calls/src/lib.rs` (`import_csv`, `extract_features`) | Working-tree CSV validation, HMAC identities, directional/time-window metadata features and simulation propagation | HTTP import/persistence/fusion pending; short answered calls do not establish silence; number linking absent |
| F13; C1/C4/C7/C12/C14; Deliverable5; RB01/RB02/RB32–RB36 | `crates/hre-eval/src/` (`generator_b.rs`, `metrics.rs`, `lib.rs`, `main.rs`); `just eval` | Working-tree synthetic Generator B, authored text, heuristic replay, confusion/PR metrics, keyword baseline, partial ablations, heavy-user slices and report writer | Execution/report review pending; engine-only replay excludes HTTP/store/CLI; media/full ablations unavailable; OCR/ASR/kappa/slang null without evidence |
| F1/F3/F13; C7/C12; RB33/RB39/RB44 | `training/src/hre_training/`, `training/fixtures/golden_vector.json`, `training/datasets.json` | Main A1/A2 inventory/export contracts and A4 Generator A/golden fixture; concurrent license/provenance audit | Download/unification, executed export/parity, trained artifacts and license clearance absent; a golden fixture is not broad parity |
| F10/F12/F17/F18; 1.8/5.4/5.7; C3; Deliverable3/4 | Intended slang, evidence, recording jobs and WS routes above | No completed delivery established by inspected engine changes | Full implementation/integration and API acceptance pending; future prepared streams must be simulated |
| F19/F20/F21; 4.2/4.5/5.3; C8 | Intended behind-flag meme/voice/video modules | No verified detector or video pipeline; Roadmap or unverified simulation target | No generalization, live capture or end-to-end claim |

Research Roadmap rows 1.3/1.4/1.7/2.4/2.6/2.8/3.6/5.8, platform P01–P19 and roadblocks RB01–RB45 remain preserved above. Acknowledging them is not operational acceptance. Models are unavailable for active inference; registry/hash inspection alone proves neither successful inference nor quality.

Task cards **C1/C2/C8** (dashboard shell, alert review/timeline, CLI) remain outstanding behind the engine gate; research challenge IDs **C1/C2/C8** are separate. Engine-only evaluation does not implement those clients or close multimodal/generalization requirements. All mock, metadata, synthetic and override contributions must propagate `simulated=true`; unavailable modules report Roadmap/Disabled (`roadmap`/`disabled` on the wire).
