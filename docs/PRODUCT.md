# Product

Snapshot: 2026-10-28. Delivery is incomplete. [TRACEABILITY.md](TRACEABILITY.md) records intended scope separately from actual evidence; all initial actual statuses remain Roadmap/unverified.

The Harassment Risk Engine is intended to score events from cooperating clients using conversation history and multiple modalities. It should help moderators and recipients recognize persistent unwanted contact even when individual messages appear polite. A score prioritizes review; it does not determine a person's intent or guilt.

## Users and decisions

| User | Needed outcome | Acceptance |
|---|---|---|
| Recipient | Safer defaults and usable evidence without replaying harmful media | Blur/transcript recommendations, masked ordinary output, authorized export |
| Moderator | Reasons, context and reversible decisions | Alert queue, directional history, confirm/reject/escalate, audit trail |
| Sender | Useful pre-send feedback | Draft analysis that leaves no persistent state |
| Client developer | Stable contract and useful failures without model files | Generated OpenAPI, mock mode, deterministic fallbacks, structured errors |
| AI agent | Clear ownership, statuses and reproducible acceptance | Traceable IDs, named gaps, source-backed model metadata, explicit pending requests |

## Input and output

Inputs are message, image, audio and call_event, with event ID, timestamp, sender/recipient and platform/channel context. Block/mute/stop events are explicit inputs; absent platform context must remain unknown. The response contract targets severity 0–100, tier, reasons, per-module signals, action, latency, model versions, simulation and degradation metadata.

| Severity | Tier | Intended response |
|---|---|---|
| 0–30 | low | Log; optional draft nudge |
| 31–60 | review | Moderator queue and soft warning |
| 61–85 | high | Blur/hide or throttle recommendation; moderator alert |
| 86–100 | urgent | Immediate human review and appropriate safety guidance |

Clients enact actions only within their permissions. Returning throttle or blur_media is not proof that an external platform enforced it.

## Scope and release evidence

Lead is implementing core/store/registry/behavior/fusion/API. Human teams own text/slang/eval/training (A), vision/audio/calls (B), and evidence/dashboard/connectors/CLI (C). Lead fallback adapters support the walking skeleton; they do not complete teammate crates.

The intended demo has nine scenarios: polite stalker, pile-on, benign cyberflashing proxy, OCR screenshot, TTS threat, overnight call log, healthy heavy users, invented slang update and evidence export. See [EVAL.md](EVAL.md). All nine require end-to-end acceptance before claiming the full demo works. No such pass is recorded here.

## Non-goals and limits

No arbitrary private-DM access, covert call capture, cross-platform identity linking, reliable grooming detection, guaranteed novel deepfake/voice detection, perfect sarcasm understanding or intent inference from tone. Group exclusion, impersonation, surveillance, account compromise and contact-network tracking remain Roadmap. The engine supports human decisions; it does not replace moderators or provide certified legal evidence.

Do not add slogans or labels to compensate for unclear UI. Preserve necessary simulation/degradation warnings, action meaning and accessible names. Stable reason codes plus plain-language explanations should serve users, developers and agents together.
