# API changelog

## Unreleased — 2026-10-28

Contract freeze is pending/unverified. Parent is proceeding with P0 scaffold/core contract/mock. This sidecar has not inspected a final generated OpenAPI contract or verified any route. The endpoint matrix in TRACEABILITY is proposed scope from ENGINE_PROMPT, not a published implementation guarantee.

Intended v1 envelope includes event_id, kind, occurred_at, draft, sender/recipient, platform/channel/context and modality payload. Intended response includes severity, tier, reasons, signals, recommended_action, secondary_actions, alert_id, latency_ms, model_versions, simulated and degraded. Auth targets client X-API-Key and moderator/admin Bearer tokens; errors target application/problem+json.

Known unresolved compatibility questions:

- R5 Status vocabulary is Built/Simulated/Roadmap while the signal example says built/simulated/disabled. Lead must select and publish a single wire contract with an explicit unavailable/roadmap mapping.
- Scalar/enum and field shapes, optionality, idempotency conflict behavior, auth roles and unknown-signal representation are not frozen by this document.
- Media multipart, 202 jobs, SSE/WS payloads, evidence export, call CSV and slang operations require generated schemas and teammate integration.
- Mock canned scenarios and demo overrides must preserve simulated provenance. Missing model fallback is degraded and must report its actual backend.

No breaking change entry is asserted: there is no verified prior frozen contract to compare. Once frozen, append date, old/new shape, affected routes/client impact, migration, compatibility decision and schema/test evidence for each breaking change. Notify the parent/human; documentation alone is insufficient notification.
