---
name: Adapt Harassment Risk Engine Moderator Dashboard
description: Material 3 Expressive (M3E) Design System for Trust & Safety Incident Triage
colors:
  primary: "#3f51b5"
  primary-container: "#e0e5ff"
  on-primary: "#ffffff"
  on-primary-container: "#001453"
  secondary: "#5c5d72"
  secondary-container: "#e1e0f9"
  tertiary: "#77536d"
  tertiary-container: "#ffd7f3"
  surface: "#fbf8fd"
  surface-dim: "#ded8e1"
  surface-bright: "#fbf8fd"
  surface-container-lowest: "#ffffff"
  surface-container-low: "#f5f2fa"
  surface-container: "#efecf4"
  surface-container-high: "#e9e7ef"
  surface-container-highest: "#e3e1e9"
  on-surface: "#1a1b21"
  on-surface-variant: "#45464f"
  outline: "#767680"
  outline-variant: "#c6c5d0"
  tier-low: "#0f766e"
  tier-low-bg: "#ccfbf1"
  tier-review: "#b45309"
  tier-review-bg: "#fef3c7"
  tier-high: "#c2410c"
  tier-high-bg: "#ffedd5"
  tier-urgent: "#b91c1c"
  tier-urgent-bg: "#fee2e2"
  status-built: "#059669"
  status-simulated: "#7c3aed"
  status-simulated-bg: "#ede9fe"
  status-roadmap: "#4b5563"
  status-roadmap-bg: "#f3f4f6"
typography:
  headline-large:
    fontFamily: "Roboto Flex, system-ui, -apple-system, sans-serif"
    fontSize: "32px"
    fontWeight: 500
    lineHeight: "40px"
  headline-medium:
    fontFamily: "Roboto Flex, system-ui, -apple-system, sans-serif"
    fontSize: "28px"
    fontWeight: 500
    lineHeight: "36px"
  title-large:
    fontFamily: "Roboto Flex, system-ui, -apple-system, sans-serif"
    fontSize: "22px"
    fontWeight: 500
    lineHeight: "28px"
  title-medium:
    fontFamily: "Roboto Flex, system-ui, -apple-system, sans-serif"
    fontSize: "16px"
    fontWeight: 500
    lineHeight: "24px"
  body-large:
    fontFamily: "Roboto, system-ui, -apple-system, sans-serif"
    fontSize: "16px"
    fontWeight: 400
    lineHeight: "24px"
  body-medium:
    fontFamily: "Roboto, system-ui, -apple-system, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: "20px"
  body-small:
    fontFamily: "Roboto, system-ui, -apple-system, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: "16px"
  label-large:
    fontFamily: "Roboto, system-ui, -apple-system, sans-serif"
    fontSize: "14px"
    fontWeight: 600
    lineHeight: "20px"
  label-medium:
    fontFamily: "Roboto, system-ui, -apple-system, sans-serif"
    fontSize: "12px"
    fontWeight: 600
    lineHeight: "16px"
  label-small:
    fontFamily: "Roboto, system-ui, -apple-system, sans-serif"
    fontSize: "11px"
    fontWeight: 600
    lineHeight: "16px"
rounded:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "28px"
  full: "9999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "16px"
  lg: "24px"
  xl: "32px"
---

# Design System: Material 3 Expressive (M3E)

## Overview

The Adapt Moderator Dashboard is designed for high-throughput, high-stress Trust & Safety operations. Built on Material 3 Expressive (M3E), it combines vibrant expressive color palettes, generous structural rhythm, clear surface elevation tiers, and purposeful feedback animations with strict ergonomic efficiency.

Key operational characteristics:
- **Operate Mode**: Scanning efficiency, high contrast, predictable keyboard navigation, zero visual distraction.
- **Radical Honesty**: Machine-generated signals, simulated overrides, and degraded backends are visually differentiated with unmistakable status badges.
- **Privacy Assurance**: Sensitive user hashes, masked transcripts, and reason codes are presented cleanly without raw abusive media exposure.

## Colors

### Theme System
The interface supports both Light and Dark themes via CSS custom properties and M3 color roles.

#### Dark Mode Palette Roles
- Background: `#121318`
- Surface: `#121318`
- Surface Container Lowest: `#0d0e13`
- Surface Container Low: `#1a1b21`
- Surface Container: `#1f1f25`
- Surface Container High: `#292a30`
- Surface Container Highest: `#34343b`
- On Surface: `#e3e1e9`
- On Surface Variant: `#c6c5d0`
- Outline: `#90909a`
- Primary: `#bac3ff`
- Primary Container: `#26399c`
- On Primary: `#06218c`

### Severity & Tier Indicators
- **Low (0–30)**: Teal/Emerald (`tier-low`). Stable, no immediate danger.
- **Review (31–60)**: Warm Amber (`tier-review`). Queued for routine moderator verification.
- **High (61–85)**: Vivid Coral/Orange (`tier-high`). High risk; automated throttling/blurring suggested.
- **Urgent (86–100)**: Deep Crimson (`tier-urgent`). Critical threat or active harm requiring immediate intervention.

### Status Indicators
- **Built**: Forest Green (`status-built`). Fully functional ONNX or heuristic model.
- **Simulated**: Vivid Violet (`status-simulated`). Explicitly badged mock or demo override; never disguised as production AI.
- **Roadmap / Disabled**: Neutral Slate (`status-roadmap`). Inactive or future capability.

## Typography

- **Font Family**: Standard system fonts (`system-ui, -apple-system, "Segoe UI", Roboto, sans-serif`) ensuring fast, reliable local rendering without external web font downloads.
- **Hierarchy**: Strict ratio steps between Headline, Title, Body, and Label roles.
- **Data & Numbers**: Formatted with tabular numerals (`font-variant-numeric: tabular-nums`) for perfect alignment across queue rows and metric cards.

## Layout

### Ergonomic Layout Structure
1. **Top Application Bar**: Fixed height (64px), containing branding, system status indicator, search, theme switcher, and moderator session token status.
2. **Navigation Rail / Drawer**: Left-aligned (width 80px collapsed / 240px expanded on desktop, bottom bar on mobile) linking the 8 primary operational modules:
   - Live Queue (`/dashboard/queue`)
   - Overview & Health (`/dashboard`)
   - Incident Investigation (`/dashboard/alerts/{id}`)
   - Subject Timelines (`/dashboard/timelines`)
   - Slang Intelligence (`/dashboard/slang`)
   - Evaluator Metrics (`/dashboard/metrics`)
   - Call & Audio Ingestion (`/dashboard/calls`)
   - Cryptographic Evidence (`/dashboard/evidence`)
   - Demo Scenarios (`/dashboard/demo`)
3. **Workspace Canvas**: Scrollable container (`max-width: 1600px; margin: 0 auto; padding: 24px`).

## Elevation & Depth

- **Level 0**: Flat surface (`surface`), no shadow.
- **Level 1**: Card and list item default (`0 1px 3px 1px rgba(0,0,0,0.12), 0 1px 2px 0 rgba(0,0,0,0.24)`).
- **Level 2**: Hovered elements and active filters (`0 2px 6px 2px rgba(0,0,0,0.12)`).
- **Level 3**: Dialogs, dropdowns, and flyout navigation drawer.

## Shapes

- **Badges and Chips**: Pill shape (`rounded-full`, 9999px radius).
- **Buttons and Input Controls**: `rounded-sm` (8px radius) for tactile precision.
- **Cards and Modals**: `rounded-md` (12px) or `rounded-lg` (16px) for expressive containers.

## Components

- **M3E Web Components**: Leveraged via `@m3e/web` custom elements (`<m3e-button>`, `<m3e-badge>`, `<m3e-card>`, `<m3e-tabs>`, `<m3e-dialog>`, `<m3e-nav-rail>`).
- **Reactive Queue**: HTMX-driven live queue with Server-Sent Events (`hx-ext="sse"`, `sse-connect="/v1/stream/alerts"`), displaying incoming cards without layout shifts.
- **Attribution Bars**: Clean horizontal percentage bars visualizing multi-signal weight breakdown.
- **Server-rendered SVG Charts**: Real-time confusion matrix, PR curves, and latency distribution rendered as pure inline SVG.

## Do's and Don'ts

### Do
- Keep moderator actions (Confirm, Reject, Escalate) prominent, keyboard-accessible, and confirmed with unambiguous toast feedback.
- Visibly label every mock or simulated payload with `Simulated` pill badges.
- Ensure all views function seamlessly offline with vendored assets.

### Don't
- Never display raw abusive media images; always use blur filters or placeholder hashes.
- Never use external CDN links (`unpkg`, `jsdelivr`, `cdnjs`, Google Fonts).
- Never hide system degradation; if a model is missing, show an explicit alert notice.
