# Hacksploit 2026 – PS 5: Roadblocks & Feasibility

This file covers the roadblocks for each deliverable (including image, voice, and phone-call work), the feasibility of each solution in `PS5_problems_and_solutions.md`, platform access limits, and non-technical risks.

### Status legend

| Tag | Meaning |
|-----|---------|
| **Built** | Implemented and evaluated in the PoC |
| **Simulated** | Demonstrated with synthetic data, benign proxies, or a mock event |
| **Roadmap** | Described as a future direction, not implemented |

---

## Part 1: Roadblocks by deliverable

### Deliverable 1 – Behavioral AI analysis (stalking patterns, messages and calls)

| Roadblock | Why it matters | Mitigation |
|-----------|----------------|------------|
| **No real stalking or harassing-call data exists** | Logs are private and sensitive, so training and testing use synthetic data | Randomize generator parameters widely; add noise and ambiguous cases; test on data from a differently configured generator than the one used for training; state clearly that results are on synthetic data |
| **Circularity risk** | The model can learn the generator's own rules, making metrics look unrealistically good | Use separate generators or parameter ranges for train and test; include hand-written edge cases |
| **Key signals exist only in your own app** | "Contact after block" and "reply ratio" need block and reply events that Discord or Instagram comments don't expose | Use the demo chat app as the main home of this module |
| **Call-log data is personal** | Real call logs reveal who someone talks to | Use synthetic logs, or let a user upload an export of their own log with clear consent |
| **Benign heavy users look like stalkers** | Couples, close friends, and family send many messages and make many calls | Include a "benign heavy-user" class in synthetic data; use reply ratio, call duration, and reciprocity; report precision explicitly |

### Deliverable 2 – Multimodal threat filtering

| Roadblock | Why it matters | Mitigation |
|-----------|----------------|------------|
| **Cannot ethically gather real explicit, NCII, or deepfake-abuse data** | Collecting or storing such material is unsafe and may be illegal | Use benign proxies (public benchmark samples handled carefully, or blurred placeholders); state what was tested on |
| **NCII hash matching needs a hash database** | Industry hash lists are partner-only | Demonstrate the mechanism by hashing your own test images; do not claim real-world coverage |
| **Deepfake detectors generalize poorly** | Models trained on FaceForensics++ drop on newer generators and compressed social-media images | Combine with NSFW score; report limits honestly; route uncertain cases to human review |
| **Dataset access terms** | FaceForensics++ requires agreeing to a usage agreement | Check terms before using or showing it |
| **OCR errors** | Stylized fonts, low resolution, and mixed scripts garble extracted text | Pre-process images; use a multilingual OCR; report OCR accuracy separately |
| **Hateful-meme detection is hard** | Meaning depends on image and caption together; benchmarks are difficult | Keep as roadmap, or demo on a few examples with clear caveats |
| **Video is expensive** | Per-frame processing multiplies compute | Sample one frame every 1–2 seconds; process short clips only |

### Deliverable 2 (continued) – Voice and audio

| Roadblock | Why it matters | Mitigation |
|-----------|----------------|------------|
| **Audio is slow** | Whisper on CPU is too slow for live demos | Use a small Whisper model, a GPU/Colab, or pre-processed demo clips |
| **No safe real abusive-speech data** | Cannot record or distribute real abuse | Convert public text samples to audio with a text-to-speech model; use acted emotional-speech datasets (such as RAVDESS or CREMA-D) for tone testing; label all test audio as synthetic |
| **Speech-emotion models are noisy** | Tone varies by speaker, accent, and culture; shouting is not always aggression | Use tone only as a supporting signal that adjusts severity slightly; never flag on tone alone |
| **Synthetic-voice detection generalizes poorly** | Detectors trained on ASVspoof-style data struggle with newer voice-cloning systems | Keep as roadmap; if shown, label as low-confidence and send to human review |
| **Transcription errors on accents, code-mixing, and noise** | Indian-language and code-mixed speech can degrade Whisper accuracy | Test a representative sample; report word-error rate informally; allow human review of transcripts |
| **Real-time chunking adds complexity** | Streaming requires buffering, partial transcripts, and incremental scoring | Build the simulated live demo with 5–10 second chunks from a prepared file |

### Deliverable 2 (continued) – Phone calls

| Roadblock | Why it matters | Mitigation |
|-----------|----------------|------------|
| **Ordinary apps cannot tap live phone calls** | iOS does not allow third-party apps to record or process phone calls, and Android has heavily restricted call-recording access in recent versions (verify the current rules) | Use call-log analysis and user-uploaded recordings; present live-call analysis as an operator or OS-level roadmap item |
| **Speakerphone capture is fragile and ethically problematic** | The other party has not consented, and recording quality is poor | Do not build covert recording |
| **Recording-consent laws vary** | Rules differ by country and state; India's privacy law is still evolving | Verify the current legal position before making claims; only analyze recordings the user lawfully holds and chooses to upload |
| **Call-log access on phones is restricted** | Apps need special permissions, and stores restrict them | For the PoC, use synthetic logs or a CSV export the user uploads |
| **Voice is personal and biometric-adjacent data** | Storing voice recordings raises privacy risk | Process transiently, keep transcripts and scores rather than audio, and delete uploads after analysis unless the user chooses to keep evidence |

### Deliverable 3 – Sentiment and slang adaptation

| Roadblock | Why it matters | Mitigation |
|-----------|----------------|------------|
| **No ground truth for slang that has not appeared yet** | Adaptation can only be simulated | Use a temporal split: hold out some terms during training, then show the system picking them up after moderator feedback |
| **Embedding similarity can over-flag** | Neutral words may sit near abusive ones | Require moderator confirm/reject before any lexicon change; no auto-updates |
| **Feedback can be abused** | Bad actors could poison the lexicon | Restrict confirmation to authenticated moderator roles; log changes |
| **Hand-built term lists are risky and quickly outdated** | Static lists miss new terms and may include sensitive material | Learn from labeled public datasets and moderator feedback instead |

### Deliverable 4 – PoC that detects and reports

| Roadblock | Why it matters | Mitigation |
|-----------|----------------|------------|
| **Integration setup eats time** | Meta developer setup and webhook verification can take longer than the ML work | Start early; use a tunnel (e.g. ngrok) for webhooks; prioritize Discord or Telegram first |
| **Compute and latency** | XLM-R large, Whisper, and image models are heavy on CPU | Use smaller models for the live demo; run large models only for evaluation |
| **External dependency failures during demo** | Network or platform issues can break a live demo | Make the demo chat app work with no external dependencies; keep a Discord or Telegram connector as backup |
| **Scope creep across modalities** | Text, image, audio, and call modules together are a lot for one hackathon | Follow the phased build order; cut deepfake, synthetic voice, and video first |

### Deliverable 5 – Performance metrics

| Roadblock | Why it matters | Mitigation |
|-----------|----------------|------------|
| **Subjective labels** | Annotators disagree on what counts as harassment | Have two teammates label a small test set and report their agreement |
| **Data leakage** | Pretrained text models were trained on public toxicity datasets; testing on the same data inflates results | Build part of the test set from fresh, hand-written examples |
| **Undefined "detection rate"** | Judges cannot compare results without a definition | Define it as the share of harassment conversations flagged, plus time-to-detection (messages or calls needed before the flag) |
| **Class imbalance** | Accuracy is misleading when harassment is rare | Report precision, recall, F1, false-positive rate, confusion matrix, and precision-recall curves |
| **Different metrics per modality** | OCR, ASR, and classifiers use different measures | Report per-module metrics separately (OCR accuracy, transcription error, classifier precision/recall), then an end-to-end detection rate |

---

## Part 2: Feasibility of the solutions in the problems/solutions file

| Solution type | Examples | Feasibility |
|---------------|----------|-------------|
| **Detection** (scoring text, OCR text, images, voice notes, behavior) | Toxicity, NSFW, OCR, Whisper plus text, persistence, escalation | **Build it** |
| **Behavioral analysis on calls** | Call frequency, silent calls, calls after block (synthetic or uploaded logs) | **Build it** |
| **Actions in your own demo app** | Blur, warn sender, throttle, hold for review, report button | **Build it** |
| **Actions via a bot/API on a platform you run** | Hide a comment, delete a message | **Build one** (Discord, or Instagram on your own account) |
| **Evidence tooling** | Timeline export, voicemail or recording upload report | **Build it** |
| **Simulated real-time audio** | Chunked streaming demo from a prepared clip | **Simulate** |
| **Needs platform-internal data** | Block-evasion linking, spyware/login anomalies, location-request patterns, profile-view monitoring, group-membership monitoring | **Simulate or roadmap only** |
| **Needs data platforms rarely have** | Age-gap grooming features, impersonation detection from profile similarity | **Roadmap only** |
| **Needs OS or operator access** | Live phone-call analysis | **Roadmap only** |
| **Hard open problems** | Subtweeting and indirect harassment, cross-platform account linking, sarcasm, hateful memes, synthetic-voice detection | **Acknowledge as limitations** |

### Feasibility of the added features

| Feature | Feasibility | Notes |
|---------|-------------|-------|
| OCR on images and screenshots | **Build** | Cheap and high value; many text filters miss text inside images |
| Voice-note analysis (Whisper plus text plus tone) | **Build** (tone as supporting signal) | Test on TTS-generated and acted audio |
| Call-log behavior analysis | **Build** | Reuses the stalking module; needs a synthetic call generator |
| Recorded call and voicemail evidence report | **Build** (if time allows) | Consent-based upload |
| Simulated live-call demo | **Simulate** | Label clearly as simulation |
| Hateful meme detection | **Roadmap** | Hard problem |
| Synthetic-voice detection | **Roadmap** | Weak generalization |
| Video via frame sampling | **Roadmap or simulate** | Costly; short clips only |

### Specific cautions

- **Grooming detection** is the highest-stakes item. It needs reliable age data, which platforms often lack, and there is no safe public training data. Present it as a roadmap item handled by human review, not a working feature.
- **Spyware and account-compromise detection** is outside what a message-analysis engine sees. Move it to future work.
- **Sextortion detection** from text alone will produce false positives and false negatives. Frame it as one signal among several, not a verdict.
- **Tone from voice** must never be the sole reason for a flag.
- **Regional languages** (Hindi, Konkani, Marathi) may perform poorly without fine-tuning data. Test one explicitly and report the result.

---

## Part 3: Platform access reality

### Tier 1 – Open bot or webhook platforms (build these)

| Platform | Connection | Notes |
|----------|-----------|-------|
| Discord | Bot in a server you control | Reading message text needs the Message Content privileged intent. Bots in under 100 servers can usually enable it without manual review. Apps accessible to more than 10,000 users must apply for privileged intents (policy change announced June 2026) |
| Telegram | Bot via BotFather | Bots default to Privacy Mode and see limited group messages. Disable privacy mode with `/setprivacy` or make the bot an admin, then remove and re-add the bot. Cannot read people's private chats with each other. Users can send voice notes and images directly to your bot |
| Slack / Microsoft Teams | Events API / Bot Framework | Good for workplace harassment. Needs admin approval to install. Verify current scopes |
| Twitch / YouTube Live | IRC/EventSub, Live Chat API | Good for livestream raids and pile-ons. Verify quotas |
| Reddit / Matrix / Mastodon | Public APIs/bots | Public content only. Verify current access terms |

### Tier 2 – Business or partner access only

| Platform | Reality |
|----------|---------|
| Instagram / Facebook | Official APIs serve professional (business/creator) accounts; no access to arbitrary users' DMs. Comment moderation works on accounts you own or manage |
| WhatsApp | Business API is for businesses messaging customers, not for reading personal or group chats |
| X (Twitter) | Paid, tiered access; impractical for a hackathon |

### Tier 3 – Closed, encrypted, or OS-controlled

Snapchat, iMessage, encrypted DMs, and phone calls generally cannot be scanned from a server or by an ordinary app. Detection there would have to run **on the user's device** (victim-side filtering) or at the **operator/OS layer**. Treat this as a future, privacy-preserving direction.

### Direct messages: what is and is not possible

| Approach | Possible? |
|----------|-----------|
| Scan other people's private DMs on WhatsApp, Instagram, Snapchat from outside | **No** |
| Own demo chat app with DMs between test users | **Yes** (best way to demo stalking detection) |
| Platform-side integration (platform calls your API) | **Roadmap** (needs partnership) |
| Victim-side on-device protection (blur, warn, evidence log) | **Yes** (browser extension as a hackathon-sized version) |
| User-submitted evidence (forwarded messages, screenshots, voice notes, recordings) | **Yes**, consent-based |
| Users DM your Telegram or Discord bot directly | **Yes**, covers only what users choose to share |

### Phone calls: what is and is not possible

| Approach | Possible? |
|----------|-----------|
| Analyze a call log (synthetic, or a CSV the user uploads) | **Yes** |
| Analyze an uploaded voicemail or recording the user lawfully holds | **Yes** |
| Simulated live call from a prepared audio clip, streamed in chunks | **Yes**, labeled as simulation |
| Tap or record live calls from an ordinary app | **No** (OS restrictions; consent laws) |
| Live call analysis by phone maker or telecom operator | **Roadmap** |

### Instagram comments: what is and is not possible

| Question | Answer |
|----------|--------|
| Works on your own or managed professional account? | **Yes**: read, reply, hide/unhide, delete comments and disable comments through the API; `comments` and `live_comments` webhooks available |
| Works on personal accounts? | **No** |
| Works on arbitrary public profiles? | **No** (not through the API; scraping would violate the terms) |
| Access level needed for a hackathon | **Standard Access** (accounts you own or manage). Advanced Access needs App Review and Business Verification |
| Main risk | Setup time (account switch, Page linking, webhook verification, permission configuration); some features may not work fully under Standard Access |

**Hackathon setup:** create a test Creator/Business account linked to a Facebook Page, create a Meta developer app, request comment permissions, subscribe to the `comments` webhook, and have teammates post normal and abusive-style comments on a test post. Flagged comments are hidden through the API live.

---

## Part 4: Non-technical roadblocks

| Area | Roadblock | Mitigation |
|------|-----------|------------|
| **Privacy and law** | Handling minors' data, real messages, voice, or call logs requires a data-protection story (e.g. India's DPDP Act; check current requirements) | Hashed IDs, minimal retention, transcripts and scores instead of stored audio, hash-only storage for explicit media, human-in-the-loop review, consent for any monitored space |
| **Call-recording law** | Consent rules differ by jurisdiction | Verify the current rules; analyze only recordings users lawfully hold and choose to upload |
| **Licenses** | Some models and datasets have restrictive terms (e.g. DINOv3-based models, FaceForensics++ agreement) | Check each license before presenting a model as "recommended" |
| **Consent** | Running bots on spaces you do not own is not acceptable | Only use servers, accounts, and groups you own, with participants informed |
| **Overclaiming** | Claiming to solve everything reduces credibility | Separate **Built / Simulated / Roadmap** clearly in slides |

---

## Part 5: Recommended approach

1. Keep the **Built / Simulated / Roadmap** tags in `PS5_problems_and_solutions.md` and use them in your slides.
2. Make the behavioral module and the "polite stalker" demo the centerpiece; show that the same module also handles **call logs**.
3. Add **OCR** and **voice notes** early; both are cheap and visibly extend the system beyond text.
4. Treat deepfake, synthetic voice, NCII, and grooming as "demonstrated mechanism plus honest limits."
5. Present live call analysis as an operator/OS-level roadmap item and explain why; this shows you understand the platform and legal limits.
6. Build the demo chat app first, then Discord, then Telegram or Instagram comments if time allows.
7. Keep a fallback for every live integration.
8. State the limits plainly in the pitch: synthetic data, benign proxies for explicit content, synthetic audio, and no access to private chats or live calls without consent.

---

*Related files: `PS5_problems_and_solutions.md` and `PS5_what_to_build.md`. Platform policies (Discord intents, Meta permissions, API versions, OS call-recording rules) change often; re-check current documentation shortly before the event.*
