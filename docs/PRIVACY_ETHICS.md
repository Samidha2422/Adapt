# Privacy and ethics

Snapshot: 2026-10-28. This document specifies intended safeguards. None is certified implemented or legally sufficient. Legal/platform details in the research are unverified unless a dated official source is recorded below.

## Processing boundary

Use only a cooperating demo client, an authorized owned space with informed participants, or evidence a user lawfully holds and chooses to submit. Do not collect real intimate-abuse imagery, conduct covert recording, scrape private chats, infer cross-platform identity or make a grooming diagnosis.

HMAC-SHA256 pseudonyms with a secret server salt are required before persistence. Domain-separate platform/tenant identifiers so a shared literal ID does not accidentally link users across platforms. Raw IDs must not reach database rows, traces, cache keys or error details. Salt rotation and deletion mapping need an explicit design; pseudonymization is not anonymity.

Default media handling is transient: validate, analyze, retain only minimal derived scores/hashes/masked OCR/transcripts and delete temporary files on success, errors, timeout and cancellation. Explicit-media storage remains hash-only. A generic evidence flag does not authorize unlimited content retention. Any non-explicit evidence exception needs informed purpose, access controls, expiry and deletion. Backups/caches require the same policy.

Draft analysis must leave no durable state or recipient alert. Non-evidence responses and logs mask PII, including OCR/transcripts and example contexts. A user-provided label at export is separate from stored pseudonyms. Hashes, transcripts and pseudonymous histories remain sensitive.

## Human decisions and harms

Reasons describe observations and uncertainty. Avoid calling someone a stalker, criminal, impersonator or minor based on a score. High volume, night activity, dialect, identity terms and vocal emotion are not sufficient on their own. Reciprocity and C6 reverse-pair victim protection need tests; threatening content still requires careful human review.

Tone must never trigger a flag alone; maximum increment is +10. Deepfake/voice outputs are low-confidence; an urgent composite means urgent human review, not proof of an authentic/synthetic intimate image. Slang changes require authenticated moderator confirmation and rollback. Preserve appeal/review decisions and audit provenance without duplicating harmful media.

## Legal and resource verification register

| Topic | Current statement | Gate before deployment/export |
|---|---|---|
| India's DPDP framework | Research references the Act; commencement, rules, exemptions and applicability not yet verified here | Check official Gazette/MeitY for applicable dates and obligations; seek jurisdiction-specific advice |
| Recording consent | No blanket claim of legality or illegality | Analyze only lawful user-held uploads; recording/monitoring requirements depend on jurisdiction and circumstances |
| Electronic evidence | Export is a draft aid, never court-certified | Verify official Bharatiya Sakshya Adhiniyam section 63 and Schedule before certificate wording; expert/counsel review |
| Cybercrime resources | cybercrime.gov.in, 1930, 181, 112 are research candidates | Verify exact official purpose, availability and region; do not conflate financial-fraud help with emergency response |
| StopNCII | Candidate user resource | Verify eligibility and service scope on its official site before recommending |

Unverified resource numbers and legal descriptions must not be shipped as authoritative guidance. Dated verification notes will be appended when checked; source facts do not establish this product's compliance.

## Evidence integrity and remaining work

Target export includes canonical JSON hashes, prev_hash chain, manifest hash and server HMAC. Those detect certain alterations relative to a trusted key; they do not prove origin, truth, chain of custody or legal admissibility. Define canonicalization and key management, then test tampering, deletion, reordering and unauthorized access.

Missing evidence: access and consent flows, retention durations, erasure/backup handling, raw-ID/log audit, upload cleanup across failures, safeguarding escalation and appeal procedures. See SECURITY, DATA and REQUESTS.

## Official-source observations — checked 2026-10-28

These observations update the initial verification register only as stated. They do not certify implementation or establish deployment-specific legal applicability.

| Topic | Verified source fact / boundary | Official source |
|---|---|---|
| DPDP Rules | Final Gazette text is dated 13 November 2025. Rule 1 stages commencement: rules 1, 2, 17–21 on publication, rule 4 after one year, rules 3, 5–16, 22–23 after eighteen months. Do not say every provision is already effective. Act-section commencement, later amendments and this product's obligations still need separate review. | [MeitY-hosted final Gazette, English rule 1 on PDF page 24](https://www.meity.gov.in/static/uploads/2025/11/53450e6e5dc0bfa85ebd78686cadad39.pdf) |
| Discord | MESSAGE_CONTENT is privileged and must be enabled. Current docs describe review when an app has more than 10,000 users who can see it across servers. Do not use the research's under-100-server shortcut as current guidance. | [Discord Gateway access rules](https://docs.discord.com/developers/events/gateway#privileged-intents) |
| Telegram | Bots receive user private messages sent to them. Group visibility depends on admin/privacy mode; privacy-enabled bots receive a restricted subset. This is not access to unrelated users' private conversations. | [Telegram Bots FAQ](https://core.telegram.org/bots/faq#what-messages-will-my-bot-get) |
| Android call logs | Google Play restricts Call Log permissions to permitted uses/approved exceptions. The policy does not establish that all Android call-log access is impossible or that this app qualifies. CSV ingestion avoids needing a mobile permission in this PoC. | [Google Play Call Log policy](https://support.google.com/googleplay/android-developer/answer/10208820) |
| Women Helpline | Official WCD page identifies 181 for women seeking support/information, with emergency referrals to ERSS; it lists a West Bengal availability exception at check time. Do not promise universal availability. | [WCD Women Helpline](https://www.wcd.gov.in/public/women/help) |
| Emergency support | WCD scheme page identifies ERSS 112 for emergency assistance. This does not validate every region/operator condition. | [WCD scheme](https://www.wcd.gov.in/offerings/women--helpline--scheme) |
| StopNCII | Its official description says hashing happens on-device and matches are handled by participating platforms under their policies. It cannot remove images across the entire internet. This engine has no verified StopNCII partnership or hash feed. Eligibility remains unchecked. | [StopNCII process](https://stopncii.org/how-it-works/) |

Failed or incomplete checks: the India Code BSA PDF request timed out; cybercrime.gov.in returned no readable content; the direct 112 site timed out; the MeitY framework landing page returned no readable text. Consequently, exact section 63(4)/Schedule Part A/Part B certificate wording, anonymous-reporting eligibility, 1930 service scope and broader recording-consent rules remain unverified. Keep proposed certificate wording and those claims out of shipped authoritative guidance until official text is read. No conclusion about absence of law/service follows from a failed fetch.

Instagram/Meta flow, personal/business account permissions, WhatsApp business scope, X pricing/access, other platform scopes/quotas, iOS live-call access and Android recording rules were not verified in this pass. They remain unknown, rather than repeated as blanket legal or technical prohibitions. No live-call capture is implemented or promised by this project.
