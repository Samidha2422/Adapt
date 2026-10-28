# Security

Snapshot: 2026-10-28. Security controls below are requirements, not an attestation. Implementation/build/network checks remain unresolved.

## Trust boundaries and controls

| Boundary / threat | Required control | Acceptance evidence pending |
|---|---|---|
| Untrusted ingest | Client X-API-Key, constant-time secret comparison, validation and bounded rate | Missing/wrong key, equal-prefix token, rate-limit tests |
| Moderator/admin writes | Separate Bearer role/token checks for review, slang, reload, eval/reset as appropriate | Client key cannot perform privileged writes |
| History/evidence reads | Authorized subject/tenant scope, no object-ID enumeration | Cross-subject/tenant export and alert denial |
| Replay and counters | Atomic event idempotency, conflict semantics and recovery | Duplicate/concurrent events count once; interrupted analysis recovers |
| Media upload | Image <=10MB/audio <=25MB; sniff MIME, validate decode/expansion bounds | Malformed MIME, oversized body, decompression/pixel bomb cases |
| Temp files/processes | Unpredictable safe paths, no traversal, cleanup, bounded subprocess/inference | Timeout/cancel/failure cleanup; ffmpeg argument safety |
| Secrets and PII | Env-managed secrets, masking, no raw-ID persistence | Logs/DB/errors inspection with invented PII fixtures |
| Browser clients | Allowlisted CORS and deliberate credential handling | Unauthorized origin denied; same-origin admin workflow |
| SSE/WS/jobs | Authenticate subscription/job access, scope events, bound buffers/chunks | Slow reader, disconnect, unauthorized job and stream tests |
| Models and manifests | Allowlisted task/schema, local path containment, hashes, label/shape checks | Missing/corrupt/mismatched artifact leaves safe fallback |
| Reload and adapters | Privileged audited atomic replacement; validate frozen precision before promotion | Failed reload preserves old version; rollback tested |
| Evidence | Canonicalization, chain/HMAC verification and protected signing key | Modified/reordered/deleted records and invalid manifest fail |

Server recommendations do not grant connector permission to delete/mute users. Treat metadata such as block state, age and supplied transcripts as claims from a client trust level, not established fact. Demo overrides require demo_mode plus fixed asset hashes and simulated disclosure; they must not activate silently in production.

## Error and degradation policy

Errors should use the prompt's application/problem+json contract, stable reason codes and safe messages. Missing dependencies should yield declared degraded/disabled paths rather than fabricated healthy scores. Distinguish not analyzed from analyzed and benign. Do not reveal filesystem paths, tokens, full content or stack traces to clients.

Reject malformed timestamps, impossible durations, excessive text/chunk counts and unsupported media. Avoid unbounded requests to model runtimes, SQLite and background queues. Authenticate before expensive decoding. No remote URL fetch is needed for the walking skeleton; any future remote-media fetch requires a separate SSRF design.

## Release gates

Parent owns implementation and logged verification. Require auth/authorization, payload bounds, cleanup, masking, HMAC persistence, idempotency/draft and hostile-input checks before declaring safeguards Built. Network isolation is not evidence of application security. No dependency audit, penetration test, policy certification or security build result is available from this sidecar.
