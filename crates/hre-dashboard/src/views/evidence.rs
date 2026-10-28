//! Cryptographic Evidence Export & Bharatiya Sakshya Adhiniyam (BSA) 2023 Section 63(4) Draft Aid View.

pub fn render_evidence_page() -> String {
    r##"<div class="page-header">
  <div>
    <h1 class="page-title">Evidence Export</h1>
  </div>

  <div>
    <span class="badge badge-roadmap">Verify an exported bundle below</span>
  </div>
</div>

<div style="display: grid; grid-template-columns: 1fr 1fr; gap: 24px; align-items: start;">
  <!-- Export Request Form -->
  <div class="card">
    <h2 class="card-title">Generate Evidence Bundle</h2>

    <form id="evidence-form" onsubmit="exportEvidence(event)">
      <div class="form-group">
        <label class="form-label" for="subject-id">Subject / Victim Pseudonymous ID</label>
        <input type="text" id="subject-id" name="subject_id" class="form-control" required />
      </div>
      <button type="submit" class="btn btn-primary" style="width: 100%;">
        Export Hash-Chained Bundle (JSON + HTML)
      </button>
    </form>

    <div id="export-result" style="margin-top: 16px;"></div>
  </div>

  <!-- Verification Tool -->
  <div class="card">
    <h2 class="card-title">Verify Bundle Integrity</h2>

    <form id="verify-form" onsubmit="verifyEvidence(event)">
      <div class="form-group">
        <label class="form-label" for="export-json">Paste Complete Export JSON</label>
        <textarea id="export-json" name="manifest_json" class="form-control" rows="5" required></textarea>
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
</div>
<script>
async function exportEvidence(event) {
  event.preventDefault();
  const form = event.target;
  const result = document.getElementById('export-result');
  const button = form.querySelector('button[type="submit"]');
  button.disabled = true;
  try {
    const data = await dashboardRequest('/v1/evidence/export', { subject_id: form.elements.subject_id.value });
    const pre = document.createElement('pre');
    pre.style.whiteSpace = 'pre-wrap';
    pre.textContent = JSON.stringify(data, null, 2);
    const printable = document.createElement('a');
    printable.href = '/v1/evidence/' + encodeURIComponent(form.elements.subject_id.value) + '/html';
    printable.className = 'btn btn-secondary';
    printable.textContent = 'Printable HTML';
    result.replaceChildren(printable, pre);
  } catch (error) {
    result.textContent = error.message;
  } finally {
    button.disabled = false;
  }
}

async function verifyEvidence(event) {
  event.preventDefault();
  const result = document.getElementById('verify-result');
  const button = event.target.querySelector('button[type="submit"]');
  button.disabled = true;
  try {
    const bundle = JSON.parse(event.target.elements.manifest_json.value);
    const response = await dashboardRequest('/v1/evidence/verify', bundle);
    result.textContent = response.valid === true ? 'Bundle integrity verified.' : 'Bundle integrity invalid.';
  } catch (error) {
    result.textContent = error.message;
  } finally {
    button.disabled = false;
  }
}
</script>"##
        .to_string()
}
