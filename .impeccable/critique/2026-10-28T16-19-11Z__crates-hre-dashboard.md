---
target_identity: "file:C:\\Users\\Vibhav\\Documents\\Projects\\Adapt\\crates\\hre-dashboard"
timestamp: 2026-10-28T16-19-11Z
slug: crates-hre-dashboard
---
# Impeccable Design Critique: crates/hre-dashboard

**Target**: `crates/hre-dashboard` (Moderator Dashboard for Adapt Harassment Risk Engine)  
**Method**: dual-agent (A: bbfcbaeb-4af6-433c-a117-227cea2a47f1 · B: 417ddc7a-39cc-44d6-a0e3-e7b078fc149d)  
**Evaluator**: Lead Design Director & UI/UX Specialist + Mechanical Detector / Browser Evidence  
**Scope**: 10 Askama/HTML views, `src/svg.rs`, `static/css/styles.css`, static vendored assets, and multi-viewport Playwright renders (desktop 1280px, tablet 768px, mobile 375px).

---

## 1. Nielsen's 10 Heuristics Scorecard

Scored on standard Nielsen 0–4 scale (0 = No issue / optimal, 4 = Usability catastrophe):

| # | Heuristic | Score | Status & Primary Findings |
|---|---|:---:|---|
| **1** | **Visibility of System Status** | **1 / 4** | **Contradictory Feed Status**: Global header hardcodes `<div class="badge badge-built">Feed Active</div>` unconditionally on every page (`base.rs:88`). In `desktop_queue.png`, the top bar claims "Feed Active" while the queue stream card directly below reports "Disconnected / Retrying". |
| **2** | **Match between System & Real World** | **2 / 4** | **Authentic Vocabulary, Abstracted Payloads**: Terminology is deeply grounded in Trust & Safety operations ("Adjudication Decision", "Reversible Action", "Bidirectional Reciprocity"), but incident detail (`detail.rs`) omits the actual communication transcript, channel type, and subject handles. |
| **3** | **User Control & Freedom** | **2 / 4** | **Irreversible UI Form State & Fragile Filters**: Submitting a review replaces the form with static text without an in-page undo or reopen capability. Queue filtering naively checks `innerText.includes(tier)`, and newly incoming SSE events break active filter views. |
| **4** | **Consistency & Standards** | **2 / 4** | **Design System Violations**: Slang term submission invokes native browser `window.prompt()`, breaking M3 styling and keyboard accessibility. Dropdown text in `<select>` is truncated in detail view. Overview displays F1 = 0.91, while Metrics reports F1 = 0.933 on the same benchmark. |
| **5** | **Error Prevention** | **2 / 4** | **Destructive Actions Lack Confirmation**: "Reset Demo State" (`demo.rs:11`) and "Rollback" (`slang.rs:13`) execute immediately on click via HTMX without confirmation dialogs. Substring queue filtering causes false matches if reason strings contain tier names. |
| **6** | **Recognition Rather than Recall** | **2 / 4** | **Truncated Identifiers in Triage Queue**: Queue cards only display truncated 8-character hex IDs (`Alert #00000000`) without sender/recipient pair monikers or incident summaries, requiring moderators to drill down to know who is involved. |
| **7** | **Flexibility & Efficiency of Use** | **1 / 4** | **Absence of Power Moderator Hotkeys**: Zero keyboard shortcuts (e.g. `1`/`2`/`3` or `C`/`R`/`E` for adjudication; `J`/`K` for queue traversal). In an operational environment targeting 42s per incident, requiring mouse-driven `<select>` interactions creates severe workflow friction. |
| **8** | **Aesthetic & Minimalist Design** | **3 / 4** | **Clean Tokens with Coordinate Flaws**: Strict M3E token adherence, but Precision-Recall curve SVG coordinate overflow truncates the label to `"Adapt Operating Point (F1=0"` (`svg.rs:162`). The 5th KPI card ("Time-to-Detection") hangs alone on an orphan row in `metrics.rs`. Responsive layout collapses on mobile viewports. |
| **9** | **Help Users Recognize & Recover from Errors** | **2 / 4** | **Uninformative Error Displays**: SSE disconnection changes the badge to "Disconnected / Retrying" without diagnostic feedback or a manual reconnect trigger. Degraded backend banners list missing modules without indicating applied fallbacks. |
| **10** | **Help & Documentation** | **3 / 4** | **Strong Legal Help, Missing Operational Rubrics**: Exemplary BSA 2023 legal disclaimers and emergency helplines, but lacks an operational triage guide, score threshold definitions, or explanations of action differences (`throttle` vs `blur_media` vs `warn_sender`). |

**Total Score: 20 / 40** (Band: *Developing / Functional with critical usability and workflow gaps*)

---

## 2. Design Specificity Verdict: Deep Authoring vs. Generic Dashboard Slop

The visual language, layout structure, and conceptual architecture exhibit a **hybrid profile**: strong, deeply authored domain semantics in architectural intent, paired with superficial facade patterns in key interactive workflows.

### Where it is Deeply Authored for Adapt:
- **8-Signal Multi-Modality Matrix**: The dashboard explicitly models the 8-signal fusion engine (Text NLP, Vision OCR, Image NSFW, Audio Tone/ASR, Telephony Patterns, Behavioral Trajectory, Social Pile-On, PII Leakage) rather than treating risk as a monolithic black-box score.
- **Statutory Indian Legal Alignment**: The Evidence Export module is explicitly grounded in the *Bharatiya Sakshya Adhiniyam (BSA) 2023 Section 63(4)* electronic evidence framework, delineating Part A (tamper-evident SHA-256 hash chains and HMAC signatures) and Part B (forensic examiner certification), alongside national victim helplines (1930 Cybercrime, 181 Women, 112 Emergency, StopNCII.org).
- **Radical Honesty Badging**: Dedicated visual roles for `Built` (emerald `#047857`), `Simulated` (dashed violet `#6d28d9`), and `Roadmap` (slate `#4b5563`) directly embody Adapt's brand commitment against disguising mock fallbacks as production AI.
- **Asymmetric Harassment Dynamics**: The Directional Interaction Timeline recognizes behavioral stalking patterns (e.g. 80 unreciprocated outbound messages, post-block platform jumping, overnight call bursts) rather than generic chat histories.

### Where it Slips into Generic or Skin-Deep Facade Patterns:
- **Adjudication Blindness in Incident Detail**: The alert investigation view (`detail.rs`) displays score dials and attribution percentages, but contains **zero incident communication payload**—no masked transcript snippet, no sender/recipient pair handles, and no platform metadata.
- **Performative Media Shield**: The "Media & Content Shield" blurs and unblurs the static literal string `"[Protected Harassment Payload — Click Below to Unblur]"` rather than rendering real masked media or contextual image hashes.
- **Cosmetic Demo & Slang Execution**: The Demo Scenario Control Center uses a `setTimeout` toast simulating execution rather than executing scenario fixtures; the Slang Intelligence view uses a blocking browser `window.prompt()` for term addition and purely local DOM opacity tweaks for approvals.
- **Developer Scaffold in Global Header**: A raw text/password input for "Moderator Token" (`<input type=\"password\" placeholder=\"Moderator Token\">`) with an unstyled "Set" button sits permanently in the main app bar on every screen.

---

## 3. Overall Impression

The Adapt Moderator Dashboard is a clean, purposeful, and architecturally sound foundation built on genuine Material 3 Expressive tokens and high-performance server-rendered Rust/Askama templates. It avoids common AI-generated aesthetic tropes (no generic purple gradients, no floating decorative glassmorphism, no fake animated charts). 

However, its primary limitation is **operational fidelity**: several core workflow loops stop short at the presentation boundary. A triage moderator cannot see the text they are judging, cannot navigate using high-speed keyboard shortcuts, and is presented with a permanent "Feed Active" badge even when the real-time stream has crashed. Closing these operational gaps will elevate the dashboard from an impressive hackathon showcase to a genuinely production-grade Trust & Safety platform.

---

## 4. Key Strengths

1. **Principled Radical Honesty in AI Provenance**:
   Clear visual differentiation between `Built`, `Simulated`, and `Roadmap` ensures complete transparency regarding system capabilities, preventing false trust in simulated demo heuristics.
2. **Statutory Indian Jurisprudence & Legal Integration**:
   The BSA 2023 Section 63(4) draft aid thoughtfully structures digital evidence for Indian court admissibility (Part A hash chain vs. Part B examiner certification) and integrates essential emergency helplines (1930, 181, 112).
3. **High-Performance Zero-CDN Architecture**:
   Zero remote network dependencies, vendored HTMX/SSE/M3E assets, and server-rendered inline SVG charts guarantee sub-millisecond local rendering, complete offline reliability, and strong privacy isolation.

---

## 5. Priority Issues (P0–P3)

### [P0] Contradictory Global Connection Status (Deceptive System State)
- **Severity**: P0 (Catastrophic)
- **Location**: `crates/hre-dashboard/src/views/base.rs:88`
- **Impact**: The top bar permanently hardcodes a green `Feed Active` badge, while the queue feed below reports `Disconnected / Retrying`. Moderators triage under the false assumption that live feeds are active during catastrophic silent stream drops.
- **Fix**: Remove hardcoded "Feed Active" badge. Bind topbar status indicator directly to HTMX SSE lifecycle events (`htmx:sseOpen`, `htmx:sseError`, `htmx:sseClose`).

### [P1] Incident Evidence Void on Alert Investigation Page
- **Severity**: P1 (Critical)
- **Location**: `crates/hre-dashboard/src/views/detail.rs`
- **Impact**: `detail.rs` renders score gauges and attribution bars but completely omits the incident message transcript, channel type, timestamps, and pseudonymized sender/recipient tokens. Moderators cannot ethically or legally adjudicate without seeing the communication in question.
- **Fix**: Introduce an "Incident Payload & Context" card displaying pseudonymized pair handles, timestamps, communication channel (WhatsApp/SMS/Call), and masked message text.

### [P1] Inactive Modality Grid Noise Diluting Signal Salience
- **Severity**: P1 (High)
- **Location**: `crates/hre-dashboard/src/views/detail.rs` & `static/css/styles.css`
- **Impact**: 8 modality cards render uniformly across a 2-column grid even when 6 are `Roadmap / Disabled (0.00)`, burying the active contributing signals during rapid triage.
- **Fix**: Reorder the signals grid to elevate active contributing modalities (`score > 0`) to primary full-sized cards, while collapsing inactive/roadmap signals into a compact summary tray.

### [P2] Precision-Recall SVG Coordinate Overflow & Truncation Bug
- **Severity**: P2 (Medium)
- **Location**: `crates/hre-dashboard/src/svg.rs:162`
- **Impact**: Label `<text x="350" y="42">Adapt Operating Point (F1=0.91)</text>` overflows the 500px viewBox, truncating the label to `"Adapt Operating Point (F1=0"`.
- **Fix**: Adjust text anchor to `text-anchor="end"` at `x="460"` or place label below the marker dot.

### [P2] Mobile App Bar Layout Collapse
- **Severity**: P2 (Medium)
- **Location**: `crates/hre-dashboard/static/css/styles.css` & `crates/hre-dashboard/src/views/base.rs`
- **Impact**: At `<600px` viewports (`mobile_overview.png`), the brand title wraps into 3 lines, colliding with the logo and squishing the "Feed Active" chip and token input off the screen.
- **Fix**: Add responsive media query hiding the desktop token input and reducing logo typography on mobile viewports.

---

## 6. Persona Walkthroughs & Red Flags

### Alex (Senior Power Triage Moderator)
- **Red Flag 1 (No Keyboard Accelerators)**: Alex processes 150+ incidents per shift. The complete absence of hotkeys (`1`/`2`/`3` to adjudicate, `J`/`K` to step through queue) slows down triage velocity.
- **Red Flag 2 (Fragile Queue State)**: Alex filters for `Urgent`, but incoming SSE alerts ignore the active filter and prepend directly into the list, forcing manual re-filtering.

### Sam (First-Time Trust & Safety Investigator)
- **Red Flag 1 (Adjudication in the Dark)**: Sam opens an urgent 85/100 alert, sees 6 disabled roadmap modules and a blurred placeholder saying "Protected Harassment Payload", with no actual message text to evaluate.
- **Red Flag 2 (Ambiguous Action Consequence)**: Sam cannot tell whether selecting "Confirm" executes the recommended "throttle" action or escalates the ticket.

### Morgan (Compliance Auditor)
- **Red Flag 1 (Missing Audit Log Access)**: The system claims "Action recorded in cryptographic audit trail", but Morgan has no interface view to inspect audit records, decision histories, or moderator signatures.
- **Red Flag 2 (Metrics Discrepancy)**: Overview shows F1 = 0.91 on the confusion matrix, while Evaluation Metrics shows F1 = 0.933 on the exact same 500-event benchmark. Such discrepancies raise red flags during external regulatory audits.

---

## 7. Minor Observations & Provocative Questions

### Minor Observations
- Moderator token is stored in unencrypted `localStorage` and dispatched as a plain Bearer header via HTMX config, presenting XSS token theft vulnerability.
- In `metrics.rs`, 5 KPI cards in a responsive grid cause the 5th card ("Time-to-Detection") to hang alone on row 2, leaving an awkward 75% void.
- In `slang.rs`, clicking Confirm merely sets CSS `opacity: 0.3` without firing a backend persistence request.
- In `timeline.rs`, the "Load Pair" button has no form action or JavaScript handler attached.

### Provocative Questions
1. *If Adapt's guiding principle is "Human Authority", how can a moderator retain authority when the dashboard hides the actual message text and forces adjudication based solely on AI score numbers?*
2. *If an adjudication is advertised as "Reversible", why does the interface destroy the form on submission without an inline Undo or revision mechanism?*
3. *When an SSE connection drops silently while the global header continues to display "Feed Active", who carries ethical liability when a missed stalking alert escalates into physical harm?*
