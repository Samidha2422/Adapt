# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Rust Axum + HTMX + SSE + Material 3 Expressive (@m3e/web) + Server-rendered SVG charts. Zero CDN dependencies (all vendored locally).

## Users

Primary users are Trust & Safety moderators, incident investigators, and compliance officers triaging live harassment reports under time pressure. Secondary users are demo evaluators and developers auditing model performance and system health.

## Product Purpose

The Adapt Harassment Risk Engine provides fast, explainable, multimodal risk scoring for digital interactions across text, image, audio, OCR, and call events. The Moderator Dashboard is the operational control center that allows human moderators to inspect high-risk incidents, review multimodal evidence without exposure to raw abusive pixels or unmasked identifiers, take reversible moderation decisions (Confirm, Reject, Escalate), manage slang lexicons, and monitor system health.

## Positioning

Unlike opaque black-box moderation filters or simple keyword matchers, Adapt uses multi-signal fusion (8 distinct signal modules) with contextual behavioral history (directional interactions, polite stalking, pile-ons). It operates with strict privacy (pseudonymized HMAC salts, zero raw media persistence), radical transparency (every signal and mock is explicitly labeled Built, Simulated, or Roadmap), and keeps human moderators firmly in authority.

## Operating Context

Moderators operate in high-density triage shifts. Interfaces must enable instant visual scannability, predictable keyboard navigation, rapid decision-making, and zero eye fatigue. The dashboard operates both in live operations (listening to SSE alert streams) and investigative review (inspecting conversation timelines and cryptographic evidence chains).

## Capabilities and Constraints

- **Live Alert Queue**: Real-time SSE alert streaming (`GET /v1/stream/alerts`) with automatic reconnect, client-side deduplication, and filter controls (Tier, Severity, Platform).
- **Incident Investigation**: Detailed view of all 8 modalities (text, OCR, image, audio, call, behavior, pile_on, PII), confidence scores, reason explanations, attribution bars, and reversible moderation actions (`POST /v1/alerts/{id}/review`).
- **Conversational Timelines**: Chronological interaction graphs for subject pairs (`/v1/pairs/{id}/timeline`), message rate spikes, and block-crossing tracking.
- **Slang Engine Operations**: Reviewing emerging slang candidates, approving/rejecting terms, triggering retraining or model rollback.
- **Evaluation & Auditing**: Visualizing system metrics (`reports/metrics.json`), precision-recall curves, confusion matrices, latency percentiles, and keyword baseline ablation.
- **Calls & Voicemail**: Bulk CSV call log import, audio analysis submission, and asynchronous job monitoring (`/v1/jobs/{id}`).
- **Cryptographic Evidence**: Tamper-evident incident timeline exports with hash chains (`sha256`), HMAC signatures, and Bharatiya Sakshya Adhiniyam (BSA) 2023 Section 63(4) draft certificates.
- **Demo Control Center**: Direct execution of all 9 standardized test scenarios with visual tier assertions and live feedback.
- **Offline / Zero-CDN Delivery**: All CSS, JavaScript (`@m3e/web`, `htmx`), fonts, and icons are vendored locally; no remote network calls at runtime.

## Brand Commitments

- Material 3 Expressive (M3E) design system.
- Professional, restrained, high-clarity interface.
- No slogans, buzzwords, or redundant labels.
- Truthful disclosure: simulated data and degraded models are visibly tagged; never claim real model inference from hashes or mock fallbacks alone.

## Evidence on Hand

- Standardized OpenAPI 3.1 specification (`docs/openapi.json`).
- 9 verified demo scenario fixtures (`data/test/scenarios/*.json`).
- Evaluator metrics schema and benchmarks (`crates/hre-eval`).
- Canonical JSON and HMAC-signed evidence export contract (`crates/hre-evidence`).

## Product Principles

1. **Scannability and Density**: Prioritize dense, legible information hierarchy over decorative whitespace. Moderators need data at a glance.
2. **Radical Honesty**: Clearly distinguish Built, Simulated, and Roadmap features. Never disguise mock responses as production AI.
3. **Privacy First**: Retain no raw media bodies or unmasked identities. Display masked text and cryptographic hashes.
4. **Human Authority**: AI calculates risk signals and suggests actions; human moderators make final decisions.

## Accessibility & Inclusion

- WCAG 2.1 AA compliant contrast across light and dark themes.
- Comprehensive keyboard navigation and focus indicators.
- Live regions (`aria-live="polite"`) for real-time alert updates.
- Full respect for `prefers-reduced-motion` settings.
