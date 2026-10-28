# Data

Snapshot: 2026-10-28. No datasets were downloaded, inspected or cleared by this sidecar. Dataset names in the research are candidates. Model and dataset licenses must be checked separately; a library license does not clear its training data or weights.

## Data inventory and admission gate

| Data | Candidate / origin | Permitted plan | Current gap |
|---|---|---|---|
| Toxicity text | Jigsaw, HateXplain, Davidson, TextDetox | Approved licensed subset plus fresh authored examples | Exact dataset/version, terms, attribution and exposure unknown |
| Message/call behavior | Synthetic generator A (Python), B (Rust) | Disjoint generators and seeds, noisy and benign-heavy classes | Generators/feature parity not verified |
| OCR screenshots | Self-rendered benign screenshots | Ground-truth strings, script and image-condition labels | Fixtures and script coverage pending |
| Image screening/hash | Self-owned benign images and placeholders | Hash mechanism and simulated scores with visible provenance | No real NCII coverage or partner hash access |
| Deepfake/meme benchmarks | FaceForensics++, Hateful Memes candidates | No use until exact source terms checked | Dataset access/license unverified |
| Audio | TTS from approved text; RAVDESS/CREMA-D candidates | TTS/acted provenance, synthetic-test label, transcript ground truth | Voice/text/dataset terms and recordings unverified |
| User evidence | Explicitly submitted lawful material | Transient processing and authorized minimal export | Retention/deletion/access controls unverified |

Admission record must include source URL, retrieval/check date, immutable revision/checksum, license text URL, restrictions, intended use, consent basis, languages, transformations and split assignment. Unknown terms mean not cleared. Do not infer redistribution permission from public accessibility.

## Storage target

| Entity | Intended contents | Must exclude by default |
|---|---|---|
| Event | Pseudonymous IDs, timestamp, channel/context, masked text or derived scores | Raw identity, arbitrary payload dumps |
| Pair/window state | Directional counters, reciprocity/refusal/time features | Cross-platform inferred identity |
| Media result | Scores, masked OCR/transcript, pHash, SHA-256 | Retained pixels/audio; unmasked ordinary output |
| Alert | Score/tier, reasons, review state, provenance | Unnecessary sensitive content |
| Feedback/lexicon | Moderator decision, term/version, masked context, audit ID | Auto-promoted unreviewed terms |
| Model/eval run | Exact artifact/config/split/hash, backend, timings | Secrets, media dumps or invented metrics |

pHash supports approximate matching, not tamper integrity; SHA-256 supports byte integrity, not proof of authenticity. Transcripts, OCR and pseudonymous history are still sensitive. Evidence retention must be explicit, access-controlled and time-bounded; explicit-media default remains hash-only. The prompt's generic evidence flag must not silently enable retention of explicit pixels.

## Splits and labels

Generator A trains; generator B evaluates with different structure/ranges. Keep sender/recipient pairs and conversations disjoint across splits. Track seeds, generator versions, time ranges and class prevalence. Include normal, heavy_benign, polite/escalating/post-block stalker, pile_on, bully, cyberflasher proxy, sextortion simulation, silent/night/post-block/new-number caller and ambiguous cases.

Hold out fresh human-written text; public benchmarks may overlap pretrained data, so remaining exposure is unknown. Fit thresholds/calibration on validation only. Never tune on generator B's final test labels or use demo overrides to report classifier accuracy.

Label event-level content and conversation-level harassment separately; include source, intended tier, whether simulated, refusal/reply context, language and uncertainty. Two annotators independently label a small held-out set; preserve disagreement and compute kappa rather than rewriting labels to agree with model output.

## Retention and audit gaps

No retention duration, deletion job, backup purge policy, durable schema or audit result is verified. Parent must supply concrete configuration and tests. Candidate phone/ID examples must be invented fixtures, not real identifiers. Do not commit datasets, recordings, private logs or model weights. See PRIVACY_ETHICS and SECURITY.
