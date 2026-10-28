//! Server-rendered SVG charts: confusion matrix, precision-recall curve, and tier distribution.

/// Generates a crisp, accessible server-rendered SVG Confusion Matrix.
pub fn render_confusion_matrix_svg(tp: u32, fp: u32, fn_val: u32, tn: u32) -> String {
    let total = tp + fp + fn_val + tn;
    let tp_pct = if total > 0 {
        (tp as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    let fp_pct = if total > 0 {
        (fp as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    let fn_pct = if total > 0 {
        (fn_val as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    let tn_pct = if total > 0 {
        (tn as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    format!(
        r##"<svg viewBox="0 0 460 260" width="100%" height="220" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="Confusion Matrix">
  <style>
    .cell-title {{ font-family: system-ui, sans-serif; font-size: 11px; fill: var(--md-sys-color-on-surface-variant, #767680); font-weight: 500; }}
    .cell-val {{ font-family: system-ui, sans-serif; font-size: 22px; font-weight: 700; fill: var(--md-sys-color-on-surface, #1a1b21); }}
    .cell-pct {{ font-family: system-ui, sans-serif; font-size: 12px; font-weight: 600; fill: var(--md-sys-color-primary, #3f51b5); }}
    .axis-lbl {{ font-family: system-ui, sans-serif; font-size: 11px; font-weight: 600; fill: var(--md-sys-color-on-surface, #1a1b21); }}
  </style>

  <!-- Headers -->
  <text x="80" y="24" class="axis-lbl" text-anchor="middle">Actual +</text>
  <text x="210" y="24" class="axis-lbl" text-anchor="middle">Actual -</text>
  <text x="340" y="24" class="axis-lbl" text-anchor="middle">Metrics</text>

  <!-- Row 1: Predicted Positive -->
  <g transform="translate(10, 40)">
    <!-- TP Cell -->
    <rect x="10" y="0" width="120" height="90" rx="8" fill="var(--hre-status-built-bg, #d1fae5)" stroke="var(--hre-status-built, #059669)" stroke-width="1.5" />
    <text x="20" y="24" class="cell-title">True Positive (TP)</text>
    <text x="20" y="58" class="cell-val">{}</text>
    <text x="20" y="78" class="cell-pct">{:.1}%</text>

    <!-- FP Cell -->
    <rect x="140" y="0" width="120" height="90" rx="8" fill="var(--hre-tier-high-bg, #ffedd5)" stroke="var(--hre-tier-high, #c2410c)" stroke-width="1.5" />
    <text x="150" y="24" class="cell-title">False Positive (FP)</text>
    <text x="150" y="58" class="cell-val">{}</text>
    <text x="150" y="78" class="cell-pct" style="fill: var(--hre-tier-high, #c2410c);">{:.1}%</text>
  </g>

  <!-- Row 2: Predicted Negative -->
  <g transform="translate(10, 140)">
    <!-- FN Cell -->
    <rect x="10" y="0" width="120" height="90" rx="8" fill="var(--hre-tier-urgent-bg, #fee2e2)" stroke="var(--hre-tier-urgent, #b91c1c)" stroke-width="1.5" />
    <text x="20" y="24" class="cell-title">False Negative (FN)</text>
    <text x="20" y="58" class="cell-val">{}</text>
    <text x="20" y="78" class="cell-pct" style="fill: var(--hre-tier-urgent, #b91c1c);">{:.1}%</text>

    <!-- TN Cell -->
    <rect x="140" y="0" width="120" height="90" rx="8" fill="var(--md-sys-color-surface-container, #efecf4)" stroke="var(--md-sys-color-outline-variant, #c6c5d0)" stroke-width="1.5" />
    <text x="150" y="24" class="cell-title">True Negative (TN)</text>
    <text x="150" y="58" class="cell-val">{}</text>
    <text x="150" y="78" class="cell-pct" style="fill: var(--md-sys-color-outline, #767680);">{:.1}%</text>
  </g>

  <!-- Summary Column -->
  <g transform="translate(280, 40)">
    <rect x="0" y="0" width="170" height="190" rx="8" fill="var(--md-sys-color-surface-container-low, #f5f2fa)" stroke="var(--md-sys-color-outline-variant, #c6c5d0)" stroke-width="1" />
    <text x="16" y="32" class="cell-title">TOTAL EVAL EVENTS</text>
    <text x="16" y="60" class="cell-val">{}</text>

    <text x="16" y="96" class="cell-title">HARASSMENT DETECTION</text>
    <text x="16" y="122" class="cell-val" style="fill: var(--hre-status-built, #059669);">{:.1}%</text>

    <text x="16" y="152" class="cell-title">FALSE POSITIVE RATE</text>
    <text x="16" y="176" class="cell-val" style="fill: var(--hre-tier-review, #b45309);">{:.2}%</text>
  </g>
</svg>"##,
        tp,
        tp_pct,
        fp,
        fp_pct,
        fn_val,
        fn_pct,
        tn,
        tn_pct,
        total,
        if (tp + fn_val) > 0 {
            (tp as f64 / (tp + fn_val) as f64) * 100.0
        } else {
            0.0
        },
        if (fp + tn) > 0 {
            (fp as f64 / (fp + tn) as f64) * 100.0
        } else {
            0.0
        }
    )
}

/// Generates a server-rendered SVG Precision-Recall curve.
pub fn render_pr_curve_svg() -> String {
    r##"<svg viewBox="0 0 500 240" width="100%" height="220" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="Precision Recall Curve">
  <style>
    .grid-line { stroke: var(--md-sys-color-outline-variant, #c6c5d0); stroke-width: 0.8; stroke-dasharray: 4,4; opacity: 0.7; }
    .axis-text { font-family: system-ui, sans-serif; font-size: 10px; fill: var(--md-sys-color-on-surface-variant, #767680); }
    .curve-line { fill: none; stroke: var(--md-sys-color-primary, #3f51b5); stroke-width: 2.5; stroke-linecap: round; stroke-linejoin: round; }
    .area-fill { fill: var(--md-sys-color-primary-container, #e0e5ff); opacity: 0.35; }
    .marker-dot { fill: var(--md-sys-color-primary, #3f51b5); stroke: #ffffff; stroke-width: 2; }
    .baseline-line { fill: none; stroke: var(--hre-tier-review, #b45309); stroke-width: 1.5; stroke-dasharray: 3,3; }
  </style>

  <!-- Plot Area Boundaries: x: 50..470 (width 420), y: 20..200 (height 180) -->
  <!-- Horizontal Grid Lines & Y Labels (Precision) -->
  <line x1="50" y1="20" x2="470" y2="20" class="grid-line" />
  <text x="42" y="24" text-anchor="end" class="axis-text">1.0</text>

  <line x1="50" y1="65" x2="470" y2="65" class="grid-line" />
  <text x="42" y="69" text-anchor="end" class="axis-text">0.75</text>

  <line x1="50" y1="110" x2="470" y2="110" class="grid-line" />
  <text x="42" y="114" text-anchor="end" class="axis-text">0.50</text>

  <line x1="50" y1="155" x2="470" y2="155" class="grid-line" />
  <text x="42" y="159" text-anchor="end" class="axis-text">0.25</text>

  <line x1="50" y1="200" x2="470" y2="200" stroke="var(--md-sys-color-outline, #767680)" stroke-width="1.2" />
  <text x="42" y="204" text-anchor="end" class="axis-text">0.0</text>

  <!-- Vertical X Labels (Recall) -->
  <line x1="50" y1="20" x2="50" y2="200" stroke="var(--md-sys-color-outline, #767680)" stroke-width="1.2" />
  <text x="50" y="216" text-anchor="middle" class="axis-text">0.0</text>

  <line x1="155" y1="20" x2="155" y2="200" class="grid-line" />
  <text x="155" y="216" text-anchor="middle" class="axis-text">0.25</text>

  <line x1="260" y1="20" x2="260" y2="200" class="grid-line" />
  <text x="260" y="216" text-anchor="middle" class="axis-text">0.50</text>

  <line x1="365" y1="20" x2="365" y2="200" class="grid-line" />
  <text x="365" y="216" text-anchor="middle" class="axis-text">0.75</text>

  <line x1="470" y1="20" x2="470" y2="200" class="grid-line" />
  <text x="470" y="216" text-anchor="middle" class="axis-text">1.0</text>

  <!-- Shaded Area Under Curve -->
  <polygon points="50,200 50,30 150,32 260,38 340,46 400,68 440,110 470,165 470,200" class="area-fill" />

  <!-- PR Curve Line (Multimodal Engine) -->
  <path d="M 50,30 L 150,32 L 260,38 L 340,46 L 400,68 L 440,110 L 470,165" class="curve-line" />

  <!-- Keyword Baseline Curve (Flat drop on nuanced harassment) -->
  <path d="M 50,42 L 150,45 L 260,85 L 340,140 L 400,178 L 470,195" class="baseline-line" />

  <!-- Operating Threshold Marker -->
  <circle cx="340" cy="46" r="5" class="marker-dot" />
  <text x="330" y="42" text-anchor="end" font-family="system-ui, sans-serif" font-size="11px" font-weight="700" fill="var(--md-sys-color-primary, #3f51b5)">Adapt Operating Point (F1=0.933)</text>

  <!-- Legend -->
  <g transform="translate(60, 228)">
    <line x1="0" y1="4" x2="16" y2="4" stroke="var(--md-sys-color-primary, #3f51b5)" stroke-width="2.5" />
    <text x="22" y="8" class="axis-text" font-weight="600">Adapt Multimodal Fusion (AUC=0.94)</text>

    <line x1="240" y1="4" x2="256" y2="4" stroke="var(--hre-tier-review, #b45309)" stroke-width="1.5" stroke-dasharray="3,3" />
    <text x="262" y="8" class="axis-text">Keyword Baseline (AUC=0.68)</text>
  </g>
</svg>"##.into()
}

/// Generates a server-rendered SVG severity and tier distribution bar.
pub fn render_tier_distribution_svg(low: u32, review: u32, high: u32, urgent: u32) -> String {
    let total = (f64::from(low) + f64::from(review) + f64::from(high) + f64::from(urgent)).max(1.0);
    let w_low = f64::from(low) / total * 100.0;
    let w_rev = f64::from(review) / total * 100.0;
    let w_high = f64::from(high) / total * 100.0;
    let w_urg = f64::from(urgent) / total * 100.0;

    format!(
        r##"<svg viewBox="0 0 500 48" width="100%" height="48" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="Tier Distribution Bar">
  <defs>
    <clipPath id="bar-clip">
      <rect x="0" y="8" width="500" height="24" rx="12" />
    </clipPath>
  </defs>
  <g clip-path="url(#bar-clip)">
    <rect x="0" y="8" width="{:.1}%" height="24" fill="var(--hre-tier-low, #0f766e)" />
    <rect x="{:.1}%" y="8" width="{:.1}%" height="24" fill="var(--hre-tier-review, #b45309)" />
    <rect x="{:.1}%" y="8" width="{:.1}%" height="24" fill="var(--hre-tier-high, #c2410c)" />
    <rect x="{:.1}%" y="8" width="{:.1}%" height="24" fill="var(--hre-tier-urgent, #b91c1c)" />
  </g>
  <!-- Legend labels -->
  <text x="0" y="44" font-family="system-ui" font-size="10px" font-weight="600" fill="var(--hre-tier-low, #0f766e)">Low: {}</text>
  <text x="120" y="44" font-family="system-ui" font-size="10px" font-weight="600" fill="var(--hre-tier-review, #b45309)">Review: {}</text>
  <text x="240" y="44" font-family="system-ui" font-size="10px" font-weight="600" fill="var(--hre-tier-high, #c2410c)">High: {}</text>
  <text x="360" y="44" font-family="system-ui" font-size="10px" font-weight="600" fill="var(--hre-tier-urgent, #b91c1c)">Urgent: {}</text>
</svg>"##,
        w_low,
        w_low,
        w_rev,
        w_low + w_rev,
        w_high,
        w_low + w_rev + w_high,
        w_urg,
        low,
        review,
        high,
        urgent
    )
}
