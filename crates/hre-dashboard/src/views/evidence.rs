//! Cryptographic Evidence Export & Bharatiya Sakshya Adhiniyam (BSA) 2023 Section 63(4) Draft Aid View.

pub fn render_evidence_page() -> String {
    r##"<div class="page-header">
  <div>
    <h1 class="page-title">Cryptographic Evidence Export</h1>
    <p class="page-subtitle">Export tamper-evident, SHA-256 hash-chained incident records with HMAC server signatures and BSA 2023 certificate draft aid.</p>
  </div>

  <div>
    <span class="badge badge-built" style="font-size: 13px;">HMAC-SHA256 Integrity Verified</span>
  </div>
</div>

<div style="display: grid; grid-template-columns: 1fr 1fr; gap: 24px; align-items: start;">
  <!-- Export Request Form -->
  <div class="card">
    <h2 class="card-title">Generate Incident Evidence Bundle</h2>
    <p style="font-size: 13px; color: var(--md-sys-color-on-surface-variant); margin-bottom: 16px;">
      Compiles chronological interaction records, masked transcripts, OCR hashes, and behavioral velocity metrics into a verifiable hash-chain manifest.
    </p>

    <form id="evidence-form" hx-post="/v1/evidence/export" hx-target="#export-result" hx-swap="innerHTML">
      <div class="form-group">
        <label class="form-label" for="subject-id">Subject / Victim Pseudonymous ID</label>
        <input type="text" id="subject-id" name="subject_id" value="sub_9a2f7c4e11" class="form-control" required />
      </div>

      <div class="form-group">
        <label class="form-label" for="from-date">Date Range Start (UTC)</label>
        <input type="date" id="from-date" name="from_date" value="2026-10-01" class="form-control" />
      </div>

      <div class="form-group">
        <label class="form-label" for="to-date">Date Range End (UTC)</label>
        <input type="date" id="to-date" name="to_date" value="2026-10-28" class="form-control" />
      </div>

      <button type="submit" class="btn btn-primary" style="width: 100%;">
        Export Hash-Chained Bundle (JSON + HTML)
      </button>
    </form>

    <div id="export-result" style="margin-top: 16px;"></div>
  </div>

  <!-- Verification Tool -->
  <div class="card">
    <h2 class="card-title">Verify Bundle Cryptographic Integrity</h2>
    <p style="font-size: 13px; color: var(--md-sys-color-on-surface-variant); margin-bottom: 16px;">
      Validates the sequential SHA-256 hash chain and checks the server's HMAC signature to prove zero post-export tampering.
    </p>

    <form id="verify-form" hx-post="/v1/evidence/verify" hx-target="#verify-result" hx-swap="innerHTML">
      <div class="form-group">
        <label class="form-label" for="export-json">Paste Export JSON or Manifest Hash</label>
        <textarea id="export-json" name="manifest_json" class="form-control" rows="5" placeholder='{"manifest_hash": "e3b0c44298fc...", "signature": "..."}'></textarea>
      </div>

      <button type="submit" class="btn btn-secondary" style="width: 100%;">
        Verify Cryptographic Signatures
      </button>
    </form>

    <div id="verify-result" style="margin-top: 16px;"></div>
  </div>
</div>

<!-- Legal & Compliance Notice -->
<div class="card" style="margin-top: 24px; border: 1px solid var(--hre-tier-review);">
  <h3 style="font-size: 15px; font-weight: 700; color: var(--hre-tier-review); margin-bottom: 6px;">
    Bharatiya Sakshya Adhiniyam (BSA) 2023 Section 63(4) Disclaimer
  </h3>
  <p style="font-size: 13px; line-height: 1.5; color: var(--md-sys-color-on-surface-variant);">
    This electronic export certificate draft is a technical aid and <strong>not legal advice</strong>. Part A captures electronic hashes and transmission timestamps. Part B requires independent certification by a qualified forensic examiner. <em>Never refer to this automated export as "court-certified." Always review with qualified legal counsel.</em>
  </p>

  <div style="margin-top: 14px; padding-top: 12px; border-top: 1px solid var(--md-sys-color-outline-variant); display: flex; flex-wrap: wrap; gap: 16px; font-size: 12px; color: var(--md-sys-color-on-surface-variant);">
    <span>National Cybercrime Reporting: <strong style="color: var(--md-sys-color-primary);">cybercrime.gov.in (1930)</strong></span>
    <span>Women Helpline: <strong style="color: var(--md-sys-color-primary);">181</strong></span>
    <span>Emergency Police: <strong style="color: var(--md-sys-color-primary);">112</strong></span>
    <span>Intimate Image Abuse Shield: <strong style="color: var(--md-sys-color-primary);">StopNCII.org</strong></span>
  </div>
</div>"##
        .to_string()
}
