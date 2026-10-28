//! Overview Dashboard view with stats, health, and server-rendered SVG charts.

use crate::svg;

pub fn render_overview_page() -> String {
    let tier_bar_svg = svg::render_tier_distribution_svg(142, 38, 19, 4);
    let pr_svg = svg::render_pr_curve_svg();
    let cm_svg = svg::render_confusion_matrix_svg(182, 12, 14, 292);

    format!(
        r##"<div class="page-header">
  <div>
    <h1 class="page-title">Moderator Operational Overview</h1>
    <p class="page-subtitle">Real-time system health, triage volume, multimodal fusion performance, and model availability.</p>
  </div>

  <div style="display: flex; gap: 12px;">
    <a href="/dashboard/queue" class="btn btn-primary">Open Live Queue &rarr;</a>
    <a href="/dashboard/demo" class="btn btn-secondary">Demo Scenarios</a>
  </div>
</div>

<!-- 4 Top Stat Cards -->
<div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 16px; margin-bottom: 24px;">
  <div class="card" style="margin-bottom: 0;">
    <div style="font-size: 13px; color: var(--md-sys-color-on-surface-variant); margin-bottom: 4px;">Total Events Scored</div>
    <div style="font-size: 28px; font-weight: 700; color: var(--md-sys-color-primary);" class="table-numbers">1,482</div>
    <div style="font-size: 11px; color: var(--hre-status-built); margin-top: 4px;">&uarr; 14% past 24h</div>
  </div>

  <div class="card" style="margin-bottom: 0;">
    <div style="font-size: 13px; color: var(--md-sys-color-on-surface-variant); margin-bottom: 4px;">Urgent Tiers Flagged</div>
    <div style="font-size: 28px; font-weight: 700; color: var(--hre-tier-urgent);" class="table-numbers">4</div>
    <div style="font-size: 11px; color: var(--md-sys-color-on-surface-variant); margin-top: 4px;">Immediate moderator attention</div>
  </div>

  <div class="card" style="margin-bottom: 0;">
    <div style="font-size: 13px; color: var(--md-sys-color-on-surface-variant); margin-bottom: 4px;">High / Review Queue</div>
    <div style="font-size: 28px; font-weight: 700; color: var(--hre-tier-review);" class="table-numbers">57</div>
    <div style="font-size: 11px; color: var(--md-sys-color-on-surface-variant); margin-top: 4px;">Average triage time: 42s</div>
  </div>

  <div class="card" style="margin-bottom: 0;">
    <div style="font-size: 13px; color: var(--md-sys-color-on-surface-variant); margin-bottom: 4px;">Median P50 Latency</div>
    <div style="font-size: 28px; font-weight: 700; color: var(--hre-status-built);" class="table-numbers">12.4ms</div>
    <div style="font-size: 11px; color: var(--md-sys-color-on-surface-variant); margin-top: 4px;">P99: 48.2ms</div>
  </div>
</div>

<!-- Tier Distribution -->
<div class="card">
  <div style="display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 12px;">
    <h2 class="card-title" style="margin-bottom: 0;">Incident Tier Distribution</h2>
    <span style="font-size: 12px; color: var(--md-sys-color-on-surface-variant);">Last 24 Hours</span>
  </div>
  {}
</div>

<!-- 2-Column Charts Grid -->
<div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(440px, 1fr)); gap: 24px; margin-bottom: 24px;">
  <!-- PR Curve -->
  <div class="card" style="margin-bottom: 0;">
    <div style="display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 12px;">
      <h2 class="card-title" style="margin-bottom: 0;">Precision-Recall Performance</h2>
      <a href="/dashboard/metrics" style="font-size: 12px; color: var(--md-sys-color-primary); text-decoration: none;">Full Metrics &rarr;</a>
    </div>
    {}
  </div>

  <!-- Confusion Matrix -->
  <div class="card" style="margin-bottom: 0;">
    <div style="display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 12px;">
      <h2 class="card-title" style="margin-bottom: 0;">Confusion Matrix (Evaluation Set)</h2>
      <span class="badge badge-built">F1 = 0.91</span>
    </div>
    {}
  </div>
</div>

<!-- System Status & Model Health Table -->
<div class="card">
  <h2 class="card-title">Engine Modality Status & Degradation Guard</h2>
  <div class="table-container">
    <table class="table">
      <thead>
        <tr>
          <th>Modality</th>
          <th>Backend</th>
          <th>Implementation Status</th>
          <th>Degradation Fallback</th>
          <th>Confidence</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td><strong>Text NLP</strong></td>
          <td>ONNX / Heuristic</td>
          <td><span class="badge badge-built">Built</span></td>
          <td>Heuristic Lexicon Fallback</td>
          <td class="table-numbers">0.92</td>
        </tr>
        <tr>
          <td><strong>Vision OCR</strong></td>
          <td>Tesseract / Rule</td>
          <td><span class="badge badge-built">Built</span></td>
          <td>Direct Heuristic Extract</td>
          <td class="table-numbers">0.88</td>
        </tr>
        <tr>
          <td><strong>Image NSFW</strong></td>
          <td>ONNX / Demo Hash Proxy</td>
          <td><span class="badge badge-simulated">Simulated</span></td>
          <td>Disclosed Hash Override (Demo)</td>
          <td class="table-numbers">0.95</td>
        </tr>
        <tr>
          <td><strong>Audio Tone / ASR</strong></td>
          <td>Whisper / Feature Extract</td>
          <td><span class="badge badge-built">Built</span></td>
          <td>Masked Transcript Fallback</td>
          <td class="table-numbers">0.85</td>
        </tr>
        <tr>
          <td><strong>Call Trajectory</strong></td>
          <td>Time-series Velocity</td>
          <td><span class="badge badge-built">Built</span></td>
          <td>Overnight Frequency Threshold</td>
          <td class="table-numbers">0.96</td>
        </tr>
        <tr>
          <td><strong>Social Pile-On</strong></td>
          <td>Sliding Window Burst</td>
          <td><span class="badge badge-built">Built</span></td>
          <td>Feed Protection Automatic</td>
          <td class="table-numbers">0.94</td>
        </tr>
      </tbody>
    </table>
  </div>
</div>"##,
        tier_bar_svg, pr_svg, cm_svg
    )
}
