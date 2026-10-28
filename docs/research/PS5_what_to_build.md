# Hacksploit 2026 – PS 5: What We Are Building

## Summary

The product is **a multimodal harassment risk-scoring engine**. It takes in events (messages, images, audio, call logs) from any source, remembers history per conversation, and returns a risk score, plain-language reasons, and a recommended action. Everything else (demo chat app, bot connectors, moderator dashboard) is a client of this engine.

---

## Part 1: Modality coverage

| Modality | What the engine does | Realistic scope |
|----------|----------------------|-----------------|
| **Text** (messages, comments, DMs in our demo app) | Toxicity, threats, PII, behavior over time | Full |
| **Images** | NSFW score, OCR for text inside images, PII in screenshots, hash matching, deepfake flag | Full for NSFW/OCR/hash; deepfake is low-confidence |
| **Voice notes and audio messages** | Whisper transcription, text pipeline, supporting tone signal | Full (tone as a supporting signal only) |
| **Phone calls** | Call-log behavior analysis, voicemail/recording evidence reports, simulated live-call demo | Call logs and uploads are realistic; live call tapping is not |
| **Video** | Frame sampling plus audio track | Short clips only |

---

## Part 2: Engine features

| # | Feature | What it does | How | Effort | Priority |
|---|---------|--------------|-----|--------|----------|
| 1 | **Message toxicity analysis** | Scores each message and labels it (threat, insult, identity attack, sexual content), in multiple languages | Pretrained XLM-R classifier plus Detoxify | Low | Must |
| 2 | **Conversation memory** | Tracks every sender→recipient pair over time windows | SQLite or Redis, sliding-window counters | Low–medium | Must |
| 3 | **Stalking-pattern detection** | Flags relentless or escalating contact even when every message is polite | Features (volume, reply ratio, contact after block, late-night bursts, escalation slope) feeding XGBoost, with SHAP reasons | Medium (centerpiece) | Must |
| 4 | **Pile-on detection** | Spots many different senders hitting one target in a short window | Count distinct senders per target per window, plus spike detection | Low | Should |
| 5 | **Image screening** | NSFW score, blur recommendation, hash matching against known test images | Marqo or Freepik classifier, `imagehash` perceptual hashing | Low–medium | Should |
| 6 | **Deepfake flagging** | "Possibly synthetic face" score for human review | Face detection plus a deepfake model, labeled low-confidence | Medium | Nice |
| 7 | **Audio screening** | Transcribes voice clips and runs the text pipeline on them | Whisper, then feature 1 | Low–medium | Should |
| 8 | **Doxxing / PII detection** | Catches phone numbers, emails, and addresses posted about someone | Regex plus a NER model | Low | Should |
| 9 | **Risk fusion** | Combines everything into a 0–100 score, a tier, reasons, and an action | Weighted scoring, calibrated on validation data | Medium | Must |
| 10 | **Slang adaptation loop** | Moderators confirm or reject suspicious new terms; the system learns from that | Embedding similarity, feedback queue, scheduled retraining | Medium | Should |
| 11 | **Pre-send nudge** | Checks a draft message and warns the sender before it posts | Same analysis endpoint with a `draft: true` flag | Low | Should |
| 12 | **Evidence report** | Exports a timestamped timeline of flagged events for a victim or moderator | Query stored scores, generate JSON or PDF | Low–medium | Should |
| 13 | **Evaluation module** | Computes precision, recall, F1, detection rate, and time-to-detection | scikit-learn on a labeled test set | Low | Must |
| 14 | **OCR on images and screenshots** | Extracts text from memes and screenshots and sends it to the text and PII pipelines | Tesseract, EasyOCR, or PaddleOCR | Low | **High** |
| 15 | **Voice-note analysis** | Transcript plus text score plus a supporting tone signal | Whisper plus speech-emotion model | Low–medium | **High** |
| 16 | **Call-log behavior analysis** | Detects repeated, silent, late-night, or post-block calls | Reuses the stalking module with call events as input | Low once feature 3 works | **High** |
| 17 | **Recorded call and voicemail evidence report** | User uploads a recording; engine transcribes, scores, and produces a timestamped report | Whisper plus features 1 and 12 | Low–medium | Medium |
| 18 | **Simulated live-call demo** | Streams a prepared clip in 5–10 second chunks with live transcript and severity updates | Chunked Whisper plus WebSocket | Medium | Medium |
| 19 | **Hateful meme detection** | Flags image-plus-caption combinations that are harmful together | CLIP-style image-text model | Medium–high | Low |
| 20 | **Synthetic-voice detection** | Flags possibly cloned voices | ASVspoof-style model, low-confidence | Medium | Low |
| 21 | **Video via frame sampling** | Screens short clips | One frame every 1–2 seconds through the image pipeline, plus audio track | Medium | Low |

Features 8, 12, 14, and 16 are cheap but make the project feel like a real product, since they address victims directly and show the engine working beyond plain text.

---

## Part 3: What the engine cannot do

Being clear on this helps in the pitch.

- Read private DMs on other platforms without their cooperation
- Tap or analyze live phone calls from an ordinary app (this needs OS or operator-level access)
- Link one person's accounts across platforms
- Reliably detect grooming, deepfakes from newer generators, or cloned voices
- Understand sarcasm or heavily coded language perfectly
- Judge intent from vocal tone alone
- Replace human moderators. It prioritizes and explains; people decide

---

## Part 4: Engine input and response

### Input event types

| Type | Example content |
|------|-----------------|
| `message` | Text, sender and recipient hashes, timestamp, context (blocked status, account age) |
| `image` | Image file or reference, optional caption |
| `audio` | Audio clip (voice note, voicemail, recording) or a chunk of a stream |
| `call_event` | Caller and callee hashes, timestamp, duration, answered or missed |

### Response format

| Field | Meaning |
|-------|---------|
| `severity` | 0–100 |
| `tier` | low, review, high, or urgent |
| `reasons` | Plain-language explanation, e.g. "58 messages in 3 hours, no replies, toxicity rising" or "14 calls in 2 hours, all unanswered" |
| `signals` | Per-module scores (text, OCR, image, audio, call, behavior) |
| `recommended_action` | log, warn sender, blur media, throttle, or escalate to a moderator |

Every client (demo app, Discord bot, dashboard) just reads this response and acts on it.

---

## Part 5: What surrounds the engine

| Component | Role |
|-----------|------|
| **Demo chat app** | Where stalking, bullying, image, and voice-note scenarios play out live |
| **Moderator dashboard** | Alert queue, explanations, confirm/reject buttons, metrics page, evidence export |
| **Call-log upload page** | Lets a user upload a call-log CSV or a voicemail for analysis |
| **Platform connector** | Discord first; Instagram comments on your own test account if time allows |

---

## Part 6: Build order

### Phase 1 – Working core
The API, features 1, 2, 3, and 9, and a basic chat UI. At the end of this phase the polite stalker can already be caught on screen.

### Phase 2 – Breadth
Features 4, 5, 8, 11, 14 (OCR), and 16 (call logs), the dashboard, and the Discord connector.

### Phase 3 – Depth
Features 7 and 15 (audio and voice notes), 10, 12, 13, 17, and 18 if time allows.

### Cut first if short on time
Video, synthetic-voice detection, hateful memes, deepfake flagging, the simulated live-call demo, and the PDF export.

---

## Part 7: Demo scenarios

| # | Scenario | What it proves |
|---|----------|----------------|
| 1 | **The polite stalker:** 80 friendly messages, no replies, continuing after a block. Keyword filters pass it; the engine flags it with reasons | Behavioral AI beats keyword filtering (deliverable 1) |
| 2 | **The pile-on:** ten accounts attack one user in a minute, and the target's feed gets protected | Group and volume detection |
| 3 | **Cyberflashing:** an unsolicited image arrives from a stranger and is blurred with a "view anyway" option | Multimodal filtering (deliverable 2) |
| 4 | **The screenshot meme:** an image with abusive text inside slips past text filters; OCR extracts it and the engine flags it | Multimodal filtering beyond plain text |
| 5 | **The voice note:** a threatening audio clip is transcribed, scored, and shown to the recipient as a transcript instead of audio | Voice coverage |
| 6 | **The call log:** a synthetic log shows 40 calls in a night, many unanswered, continuing after a block; the same behavioral engine flags it | Behavioral module works beyond chat; phone-call coverage |
| 7 | **The heavy-but-healthy couple:** high volume, lots of replies and long answered calls, no flag | Precision and false-positive control (deliverable 5) |
| 8 | **Slang update:** a moderator confirms a new term and the next message containing it is caught | Slang adaptation (deliverable 3) |
| 9 | **Evidence export:** one click produces a timeline (messages, images, voice notes, calls) the victim can send to a platform or the police | Detect and report (deliverable 4) |

Pick six or seven of these for a three-minute story; scenarios 1, 4, 6, 7, and 9 give the best coverage per minute.

---

## Part 8: Test data plan

| Data | Source | Notes |
|------|--------|-------|
| Text | Jigsaw, HateXplain, Davidson, TextDetox, plus hand-written fresh examples | Keep fresh examples to avoid leakage |
| Behavior (messages) | Synthetic generator with normal, heavy-but-benign, and stalker classes | Train and test on differently configured generators |
| Call logs | Synthetic generator with normal, heavy-but-benign, and harassing-caller classes | Same design as the message generator |
| Images | Benign proxies and public benchmark samples; blurred placeholders for the demo | Never collect real explicit or abusive imagery |
| Screenshots and memes | Self-made benign images with text rendered on them | Used to test OCR and the text pipeline |
| Audio | Text-to-speech from public text samples; acted emotional-speech datasets (RAVDESS, CREMA-D) | Label all audio as synthetic |

---

## Part 9: Suggested next steps

1. Design the folder structure and data model (tables for events, pair-state, call-events, alerts, and feedback).
2. Build the behavioral feature pipeline first, since it is the centerpiece.
3. Generate synthetic message and call-log data with benign, heavy-but-benign, and harassing classes.
4. Wire up the text model and the risk-fusion layer.
5. Add OCR and call-log analysis.
6. Build the demo chat app and dashboard around the API.
7. Add the Discord connector, then voice notes.
8. Run the evaluation and record the metrics.

---

*Related files: `PS5_problems_and_solutions.md` and `PS5_roadblocks_and_feasibility.md`.*
