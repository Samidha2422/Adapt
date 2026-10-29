# API changelog

## 2026-10-29: Uploaded audio and synthetic input provenance

- Add authenticated `POST /v1/analyze/audio`: multipart `request` (the existing
  AnalyzeRequest with `kind=audio`) and `audio` (up to 25 MiB). The declared
  media MIME must match the file part and detected format. Returns AnalyzeResponse.
- Audio uses the configured transcription backend and the same text, behavior,
  fusion, idempotency and alert pipeline as committed messages. Missing inference
  stays degraded, with an unavailable audio signal reported as Roadmap/Disabled.
- `signals.audio.details.transcript` contains the PII masked transcript in the
  upload response only. Assessments, alert storage and SSE omit the transcript.
  Retrying the same UUID with different audio bytes returns 409; drafts persist nothing.
- Add optional `context.simulated` (default false). Synthetic client samples retain
  this provenance in committed events and alerts without changing their risk scores.
- Existing image multipart and message contracts remain compatible. Clients need
  the updated engine before using the new audio route or synthetic context field.
- `/v1/recordings/analyze` remains the legacy JSON fixture/job interface. It does
  not accept uploaded audio; clients must use the new multipart audio route.

## Unreleased — 2026-10-28

Contract freeze is pending/unverified. Parent is proceeding with P0 scaffold/core contract/mock. This sidecar has not inspected a final generated OpenAPI contract or verified any route. The endpoint matrix in TRACEABILITY is proposed scope from ENGINE_PROMPT, not a published implementation guarantee.

Intended v1 envelope includes event_id, kind, occurred_at, draft, sender/recipient, platform/channel/context and modality payload. Intended response includes severity, tier, reasons, signals, recommended_action, secondary_actions, alert_id, latency_ms, model_versions, simulated and degraded. Auth targets client X-API-Key and moderator/admin Bearer tokens; errors target application/problem+json.

Known unresolved compatibility questions:

- R5 Status vocabulary is Built/Simulated/Roadmap while the signal example says built/simulated/disabled. Lead must select and publish a single wire contract with an explicit unavailable/roadmap mapping.
- Scalar/enum and field shapes, optionality, idempotency conflict behavior, auth roles and unknown-signal representation are not frozen by this document.
- Media multipart, 202 jobs, SSE/WS payloads, evidence export, call CSV and slang operations require generated schemas and teammate integration.
- Mock canned scenarios and demo overrides must preserve simulated provenance. Missing model fallback is degraded and must report its actual backend.

No breaking change entry is asserted: there is no verified prior frozen contract to compare. Once frozen, append date, old/new shape, affected routes/client impact, migration, compatibility decision and schema/test evidence for each breaking change. Notify the parent/human; documentation alone is insufficient notification.
