# API integration

`POST /v1/analyze` accepts the existing JSON `AnalyzeRequest`. An image caption
is analyzed as text. A caption never creates image bytes or OCR results. Audio
analysis returns 501 until an ASR pipeline is delivered.
The delivered `hre-audio` library validates/decodes supported WAV containers;
it is not integrated ASR, transcription, tone inference or audio risk scoring.
The image upload endpoint rejects audio envelopes with 422.

`POST /v1/analyze/image` accepts authenticated `multipart/form-data` with exactly
two fields, in either order:

- `request`: the JSON `AnalyzeRequest`, with `kind: "image"` and `media.mime`.
- `image`: the actual file bytes, with a matching part Content-Type.

The upload limit is 10 MiB plus 64 KiB for the envelope and multipart overhead.
Filenames are ignored. Processing runs in a blocking worker; no file is written.
Retries bind the event UUID to both the envelope and the actual file SHA-256.
Image validation currently checks signatures and dimensions; the delivered vision
crate does not decode pixels or run a real NSFW/OCR model. Missing models report
Roadmap/Disabled with zero scores. Container metadata extraction is labelled
simulated and feeds the delivered text/PII rules. Media text bodies are excluded
from responses, assessments and alerts. Demo image score overrides load only
when `demo_mode=true`, and remain simulated.

`POST /v1/reports` requires a previously persisted upload, rejects unknown events
with 404 and other event kinds with 422, and never writes a fabricated hash.
Reasons must contain 1..1024 bytes; the store receives a fixed report label rather
than the caller's free text. Matching requires identical SHA-256; the delivered
encoded-byte hash is not used for perceptual similarity. Local reported test
hash floors remain simulated. Drafts never persist or become reportable.

The served OpenAPI is generated from route annotations. Parent/Luna owns the
logged `format`, `check` and `openapi` recipes and the committed contract export.
Regression coverage is in `tests/media.rs` and `tests/scenarios.rs`.

`POST /v1/calllogs/import` accepts JSON with `platform`, `conversation_id`, `csv`
and optional `draft` (default false). The decoded UTF-8 CSV limit is 2 MiB; the
JSON body limit is 4 MiB plus 64 KiB. CSV columns are exactly
`event_id,occurred_at,caller_id,callee_id,duration_s,answered,voicemail,after_refusal,simulated`.
The delivered `hre-calls` parser validates the entire file before any row runs.
Malformed CSV returns 422 without writes or alerts. Identities are HMACed once
under the supplied platform and join existing message/call history.

After validation, rows run in file order through the existing engine and its
atomic per-event persistence. The import is **not all-or-nothing**: a conflict or
other row failure does not roll back prior successful rows or stop later rows.
The 200 response contains `atomic:false` and `rows`, each with the CSV line,
event UUID, outcome (`imported`, `cached`, `draft`, `failed`) and either an
assessment or problem. Retries with unchanged row data/platform/conversation
return the cached assessment; changed data under the same UUID produces a row
409. Drafts persist nothing and publish no alerts. Cancelling a request can leave
completed rows committed; retry the same CSV to recover their cached outcomes.

CSV `after_refusal` is source-supplied metadata, combined with stored refusal
state. `simulated` propagates into stored history and assessments. Scores come
from existing behavior/fusion rules; the call model remains Roadmap/Disabled.
Voicemail metadata is included in retry identity but has no separate persisted
feature in the current store schema. CSV imports are unavailable in mock mode
(501). Focused coverage is in `tests/call_import.rs`.
