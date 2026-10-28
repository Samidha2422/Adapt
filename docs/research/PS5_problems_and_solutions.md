# Hacksploit 2026 – PS 5: Problems & Solutions

**Detection and Prevention of Personal & Social Harassment**

This document lists every problem covered in our discussion (text, image, voice, and phone-call harassment) and pairs each with a proposed solution for our PoC. Each row carries a feasibility tag.

### Status legend

| Tag | Meaning |
|-----|---------|
| **Built** | Implemented and evaluated in the PoC |
| **Simulated** | Demonstrated with synthetic data, benign proxies, or a mock event |
| **Roadmap** | Described as a future direction, not implemented |

---

## Part 1: Harassment problems and their solutions

### A. Cyberbullying

| # | Problem | Detection solution | Prevention / response solution | Status |
|---|---------|--------------------|--------------------------------|--------|
| 1.1 | **Direct abuse** (repeated insults, threats, mockery about appearance, identity, grades) | Text toxicity classifier (`textdetox/xlmr-large-toxicity-classifier-v2`) plus Detoxify multi-label scores (insult, threat, identity attack) | Warn sender before posting; auto-hide message from target; one-tap report and block | **Built** |
| 1.2 | **Pile-ons** (many accounts target one person in a short window) | Volume-spike detection per target; count of distinct senders mentioning one user in a time window | Rate-limit comments on the targeted post/profile; collapse replies; alert moderators | **Built** |
| 1.3 | **Exclusion** (removing someone from groups, mocking in side chats) | Group-membership change monitoring plus negative mentions of a person in groups they were removed from | Notify group admins; offer "review before posting" mode | **Roadmap** |
| 1.4 | **Impersonation and fake accounts** | Account-similarity features (name, photo hash, bio); new-account-age flags | Fast-track impersonation report; verification prompts; automatic suspension review | **Roadmap** |
| 1.5 | **Outing and doxxing** (exposing private info or identity) | PII detection (phone, email, address patterns) combined with toxicity context | Auto-remove or blur PII; escalate to urgent queue | **Built** |
| 1.6 | **Humiliating media** (embarrassing screenshots, edited or AI-generated images) | Image NSFW and manipulation detection; perceptual-hash matching of reported images | Block re-uploads via hash; takedown workflow | **Built** (hash/NSFW) |
| 1.7 | **Indirect harassment** (subtweeting, vague posts aimed at someone, recruiting others) | Mention-graph analysis; sentiment toward a target across multiple authors | Surface patterns to moderators with explanations; support for the target | **Roadmap** |
| 1.8 | **Coded language to evade filters** | Embedding similarity to known abusive terms; moderator feedback loop | See challenge C3 in Part 2 | **Simulated** |

### B. Cyberstalking

| # | Problem | Detection solution | Prevention / response solution | Status |
|---|---------|--------------------|--------------------------------|--------|
| 2.1 | **Persistent one-sided contact** (high volume, no replies, tone may be polite) | Behavioral ML (XGBoost/LightGBM) on frequency, reply ratio, and persistence per sender→recipient pair | Throttle messages to non-responding recipients; "message request" gating | **Built** |
| 2.2 | **Contact after refusal** (after block, mute, or being told to stop) | Detect messages sent after block/mute/"stop" signals; count of attempts after refusal | Hard-block with auto-escalation; warn sender; log evidence for the victim | **Built** (own demo app) |
| 2.3 | **Block evasion** (new accounts, alternate numbers, mutual friends, platform switching) | New-account-to-same-target detection; account similarity (device/behavior fingerprint, writing style) | Linked-account suspension review; friction on new accounts contacting blocked targets | **Simulated** |
| 2.4 | **Monitoring and surveillance** (repeated profile/story views, commenting on whereabouts) | Repeated-view and location-reference signals; comments revealing knowledge of the victim's movements | Privacy-setting nudges; hide view lists/location tags; safety alerts to the victim | **Roadmap** |
| 2.5 | **Escalation** (attention → guilt-tripping → veiled threats → explicit threats) | Escalation slope: toxicity/threat score trend over a conversation; window features or LSTM | Tiered alerts (low → review → urgent); proactive moderator outreach | **Built** |
| 2.6 | **Spyware / account compromise** | Anomalous login, device, and location-request patterns | Security checkup prompts; forced password reset; 2FA recommendation | **Roadmap** |
| 2.7 | **Unusual timing patterns** (late-night bursts, bursts after triggers such as a breakup) | Time-of-day anomaly features; burst detection | Quiet hours for non-contacts; surface timing in explanations | **Built** |
| 2.8 | **Third-party involvement** (contacting the victim's employer, friends, partner) | Multi-target contact graph linking one sender to the victim's connections | Notify the victim; restrict sender's ability to contact the victim's network | **Roadmap** |

### C. Online sexual harassment

| # | Problem | Detection solution | Prevention / response solution | Status |
|---|---------|--------------------|--------------------------------|--------|
| 3.1 | **Unsolicited explicit content / cyberflashing** | NSFW image classifier (`Marqo/nsfw-image-detection-384` or `Freepik/nsfw_image_detector`) | Auto-blur incoming images from non-contacts with "view anyway" option; report/block button | **Built** |
| 3.2 | **Sexualized comments** (on posts, photos, livestreams) | Text toxicity plus sexual-content classification | Comment filters; hold-for-review; stronger defaults for minors and public figures | **Built** |
| 3.3 | **Sexual coercion and sextortion** (pressure or threats for images/acts, demands for money) | Text patterns for coercion/threats plus rapid escalation and payment-demand context | Urgent queue; victim-support resources; advise not to pay and to preserve evidence | **Simulated** |
| 3.4 | **Non-consensual intimate imagery (NCII)** | Perceptual hashing (pHash, `imagehash`) against hashes of reported images; no storage of the images themselves | Hash-based blocking across uploads; fast takedown; support for victims | **Simulated** (own test hashes) |
| 3.5 | **Deepfakes** (synthetic explicit images/video of real people) | Face detection plus deepfake classifier (e.g. `HoopitAI/video-deepfake-detection-GenD_DINOv3_L_FF`) combined with NSFW score | Flag and hold for human review; label or remove | **Simulated** (low confidence) |
| 3.6 | **Grooming-adjacent patterns** (adult-to-minor trust building, shift to sexual topics, moving to private channels) | Age-gap plus conversation-trajectory features; topic shift; request to move platforms | Highest-severity tier; immediate human review; restrict adult-to-minor DMs | **Roadmap** |
| 3.7 | **Rape and violence threats** (often used to silence women) | Threat classifier (Detoxify "threat", XLM-R toxicity) with severity weighting | Urgent escalation; preserve evidence; law-enforcement reporting guidance | **Built** |
| 3.8 | **Non-consensual image sharing in groups** | Image hash-matching and NSFW detection in group media | Block and remove; notify admins; sanction the sharer | **Simulated** |

### D. Image, screenshot, and video-based harassment

| # | Problem | Detection solution | Prevention / response solution | Status |
|---|---------|--------------------|--------------------------------|--------|
| 4.1 | **Abusive text hidden inside images** (screenshots, memes, edited photos) that text filters miss | OCR (Tesseract, EasyOCR, or PaddleOCR), then the text pipeline | Treat extracted text like a message; blur or hide the image; flag with the OCR text as the reason | **Built** |
| 4.2 | **Hateful memes** (image and caption harmless alone, harmful together) | Image-text model (CLIP-style); Hateful Memes benchmark for evaluation | Hold for review; collapse or label | **Roadmap** |
| 4.3 | **Personal info in screenshots** (phone number, address, ID shown in an image) | OCR, then PII regex/NER | Blur the region; remove the post; alert the affected person | **Built** |
| 4.4 | **Re-sharing of a humiliating image** | Perceptual hashing of reported images | Block re-uploads; notify the target | **Built** |
| 4.5 | **Abusive or explicit video** | Frame sampling every 1–2 seconds through the image pipeline, plus audio track through Whisper | Blur, hold for review, report | **Simulated** |

### E. Voice and phone-call harassment

| # | Problem | Detection solution | Prevention / response solution | Status |
|---|---------|--------------------|--------------------------------|--------|
| 5.1 | **Abusive voice notes and audio messages** | Whisper transcription, then the text pipeline | Warn or hide; offer the recipient a transcript instead of playing audio; log evidence | **Built** |
| 5.2 | **Aggressive or threatening tone** (shouting, intimidation) | Speech-emotion model as a supporting signal only (noisy, varies by speaker and culture) | Raise severity modestly; never flag on tone alone | **Simulated** |
| 5.3 | **Cloned or synthetic voices used to impersonate or harass** | Synthetic-voice detector (ASVspoof-style models) | Label as low-confidence; route to human review | **Roadmap** |
| 5.4 | **Abuse in voice chat and livestreams** | Short audio chunks (5–10 s) streamed to the audio pipeline | Real-time warning, slow mode, mute suggestion, moderator alert | **Simulated** |
| 5.5 | **Repeated, silent, or hang-up calls; calls at unusual hours** | Call-log behavioral analysis: call frequency, duration, time of day, missed-call patterns (same module as message stalking, different input) | Call-screening suggestions; auto-block after threshold; evidence timeline for the victim | **Built** (synthetic or user-uploaded logs) |
| 5.6 | **Calls after a block; switching numbers** | Contact-after-block and new-number-to-same-target features on call logs | Block-and-escalate workflow; evidence export | **Simulated** |
| 5.7 | **Abusive voicemails and recorded calls** | User uploads a recording; Whisper transcript, text and tone scoring, timestamped evidence report | Evidence report for platform, operator, or police | **Built** |
| 5.8 | **Abuse during a live phone call** | Would need on-device or operator-level audio access (not available to ordinary apps) | Operator/OS-level integration | **Roadmap** |

---

## Part 2: Detection and system challenges (and how to handle them)

| # | Challenge | Why it matters | Solution |
|---|-----------|----------------|----------|
| C1 | **Keyword-only filters miss stalking** | A stalker can send entirely polite messages relentlessly | Behavioral AI on frequency, persistence, and tone over time (deliverable 1). Demo a "polite stalker" case that keywords miss |
| C2 | **Multimodal content** (text, DMs, audio, images, calls) | Harassment spans formats (deliverable 2) | One module per modality (text, OCR, image, audio, call logs), merged by a risk engine |
| C3 | **Evolving slang and code words** | Abusers change terms to evade filters (deliverable 3) | Embedding similarity to known abusive terms, moderator confirm/reject feedback, scheduled retraining; learn from labeled public data rather than hand-built term lists |
| C4 | **False positives: heavy but benign messaging or calling** (couples, friends, fandoms, family) | Flagging healthy conversations destroys trust | Include a "benign heavy-user" class in synthetic data; use reply ratio and reciprocity; report precision explicitly |
| C5 | **Banter, reclaimed language, sarcasm, quoted abuse** | Text models over-flag these | Use context and sender-recipient history, not single-message toxicity; calibrate thresholds; keep human review |
| C6 | **Victims replying angrily may be mislabeled as aggressors** | Harms the people we want to protect | Use direction, who initiated, and prior history; weigh behavior over a single hostile reply |
| C7 | **Scarce public stalking and call-harassment data** | Real logs are private and sensitive | Synthetic-data generator based on documented behavior patterns; state this in methodology |
| C8 | **Deepfake and synthetic-voice detectors generalize poorly** | Models trained on benchmark data drop on newer generators and compressed real-world media | Combine with other signals; report limits honestly; route uncertain cases to human review |
| C9 | **Cross-platform harassment** | Abuse follows the victim across apps when blocked | Account-level and behavioral fingerprints; future work: shared signals across platforms |
| C10 | **Anonymity and multi-accounting / number switching** | Harassers re-create accounts or change numbers | New-account contact flags, similarity features, friction on new accounts targeting blocked users |
| C11 | **Privacy and data protection** (voice is also personal data) | Moderation can itself invade privacy | Minimal retention, hash-only storage for explicit media, on-device processing where possible, human-in-the-loop review, consent for any analyzed audio |
| C12 | **Class imbalance** | Harassment is rare relative to normal traffic, so accuracy is misleading | Report precision, recall, F1, false-positive rate, confusion matrix, and precision-recall curves |
| C13 | **Explainability** | Moderators and users need to trust flags | Risk engine outputs plain-language reasons plus SHAP feature contributions |
| C14 | **Multilingual and regional coverage** | Gaps for languages such as Hindi, Konkani, Marathi | Start with XLM-R multilingual models; fine-tune on a small labeled set; test one regional language explicitly |
| C15 | **OCR errors on stylized, low-quality, or mixed-script text** | Missed or garbled text in memes and screenshots | Pre-process images (resize, contrast); use a multilingual OCR; report OCR accuracy separately |
| C16 | **Voice test data is scarce and unsafe to collect** | Cannot record real abuse | Convert public text samples to speech with TTS; use acted emotional speech datasets for tone testing; label all audio as synthetic |
| C17 | **Audio and video latency** | Live demos stall on slow models | Small Whisper model or GPU/Colab; chunked streaming; pre-processed clips as fallback |
| C18 | **No live phone-call access for ordinary apps** | OS restrictions and recording-consent laws | Use call-log analysis and user-uploaded recordings; present live-call analysis as an operator/OS-level roadmap item |

---

## Part 3: Problem-statement deliverables mapped to solutions

| Deliverable | Requirement | Planned solution |
|-------------|-------------|------------------|
| 1 | Behavioral AI analysis (frequency, persistence, tone) | XGBoost/LightGBM on per-pair sliding-window features for messages **and call logs**; optional LSTM; SHAP explanations |
| 2 | Multimodal threat filtering (text, DMs, audio, images, deepfakes, NCII) | XLM-R/Detoxify (text), **OCR (text in images)**, NSFW classifier, deepfake detector, perceptual hashing, Whisper plus text pipeline (voice notes, voicemails), call-log analysis |
| 3 | Sentiment and slang adaptation | Embedding similarity, moderator feedback queue, scheduled retraining |
| 4 | PoC solution that accurately detects and reports | End-to-end pipeline with risk engine, alert queue, dashboard, and **evidence reports** |
| 5 | Performance metrics (precision, recall, detection rate) | Per-module and overall metrics, confusion matrix, PR curves, keyword-baseline ablation, time-to-detection |

---

## Part 4: Platform and medium problems with solutions

| Medium / platform type | Typical problems | Solution focus |
|------------------------|------------------|----------------|
| Direct messages (Instagram, WhatsApp, Telegram, Snapchat, Discord) | Stalking, sexual harassment, sextortion, grooming | Frequency and one-sidedness analysis, contact-after-block detection, explicit-media blur, message-request gating (own demo app; platform-side integration is roadmap) |
| Public comments (YouTube, Instagram, Facebook, Reddit) | Pile-ons, bullying, sexualized comments | Volume-spike detection, toxicity filters, comment hold-for-review |
| Group chats (WhatsApp, school groups, Discord servers) | Bullying, exclusion, non-consensual image sharing | Group-dynamics monitoring, targeted-mention analysis, hash-based image blocking |
| Social feeds (X, Facebook, TikTok) | Public shaming, doxxing, quote-post pile-ons | Mention-network and reshare-cascade analysis, PII detection |
| Images, screenshots, memes | Abusive text in images, doxxing screenshots, humiliating media | OCR plus text pipeline, PII detection, perceptual hashing, NSFW scoring |
| Voice notes and audio messages | Abusive or threatening audio | Whisper transcription, text pipeline, supporting tone signal |
| Gaming and voice chat | Verbal abuse, sexual harassment, stalking via friend lists | Chunked audio transcription, in-game report pattern analysis |
| Livestreams | Real-time chat abuse, raids | Message-velocity detection, coordinated-account detection, slow mode |
| Phone calls and voicemail | Repeated or silent calls, calls after a block, abusive voicemails | Call-log behavioral analysis, voicemail/recording upload with evidence report; live-call analysis is operator/OS roadmap |
| Anonymous apps and forums | Unattributed bullying, rumors | Content-level moderation, rate limits, behavioral signals where identity is unavailable |
| Email and workplace tools (Slack, Teams) | Professional harassment, intimidation | Persistent-targeting and tone-over-time analysis |
| Location and tracking tools | Stalking | Anomalous location-request detection, privacy nudges |

---

## Part 5: Suggested response tiers for the risk engine

| Severity score | Tier | Example triggers | Action |
|----------------|------|------------------|--------|
| 0–30 | Low | Isolated mild toxicity | Log only, optional sender nudge |
| 31–60 | Review | Rising persistence, repeated unreplied messages or calls | Queue for moderator review, soft warning to sender |
| 61–85 | High | Contact after block, escalating threats, unsolicited explicit media | Auto-blur/hide, throttle sender, alert moderator |
| 86–100 | Urgent | Explicit threats, sextortion, grooming patterns, NCII or deepfake of a real person | Immediate human review, restrict account, victim-support and reporting guidance |

---

## Part 6: Evaluation checklist

- Precision, recall, F1, and false-positive rate per module (text, OCR, image, audio, call-log, behavior)
- Overall detection rate on the combined pipeline, plus time-to-detection
- Confusion matrix and precision-recall curves
- Keyword-baseline vs. our system on "polite stalker" cases
- Benign heavy-user false-positive analysis (messages and calls)
- OCR accuracy reported separately
- Documented limitations (deepfake and synthetic-voice generalization, sarcasm, reclaimed language, synthetic data)

---

*Related files: `PS5_roadblocks_and_feasibility.md` and `PS5_what_to_build.md`. Re-verify pretrained model names, benchmark figures, and licenses before the hackathon.*
