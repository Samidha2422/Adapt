//! Slang Intelligence and Emerging Harm Lexicon Management View.

pub fn render_slang_page() -> String {
    r##"<div class="page-header">
  <div>
    <h1 class="page-title">Slang & Evolving Harm Intelligence</h1>
    <p class="page-subtitle">Review emerging harassment colloquialisms, approve regional slang terms, and manage the dynamic lexical retraining pipeline.</p>
  </div>

  <div style="display: flex; gap: 12px;">
    <button type="button" class="btn btn-primary" onclick="showAddTermModal()">+ Add Term</button>
    <button type="button" class="btn btn-secondary" hx-post="/v1/slang/retrain" hx-swap="none" onclick="showToast('Retraining pipeline triggered...')">Trigger Retrain</button>
    <button type="button" class="btn btn-secondary" hx-post="/v1/slang/rollback" hx-swap="none" onclick="showToast('Rolling back to previous slang lexicon...')">Rollback</button>
  </div>
</div>

<!-- Pending Slang Candidates Table -->
<div class="card">
  <div style="display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 14px;">
    <h2 class="card-title" style="margin-bottom: 0;">Pending Slang Candidates for Moderator Approval</h2>
    <span style="font-size: 12px; color: var(--md-sys-color-on-surface-variant);">Scenario 8 Active: Dynamic lexicon updates</span>
  </div>

  <div class="table-container">
    <table class="table">
      <thead>
        <tr>
          <th>Candidate Term</th>
          <th>Language / Dialect</th>
          <th>Detected Frequency</th>
          <th>Contextual Severity Hint</th>
          <th>Discovered Via</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
        <tr id="slang-cand-1">
          <td><strong style="color: var(--md-sys-color-primary); font-size: 15px;">gandalf_threat</strong></td>
          <td>Hinglish / Metaphor</td>
          <td class="table-numbers">47 occurrences</td>
          <td><span class="badge badge-high">High Risk (0.78)</span></td>
          <td>Autonomous Candidate Extractor</td>
          <td>
            <div style="display: flex; gap: 8px;">
              <button type="button" class="btn btn-success" style="font-size: 12px; padding: 4px 10px;" onclick="confirmTerm('slang-cand-1', 'gandalf_threat')">Confirm</button>
              <button type="button" class="btn btn-danger" style="font-size: 12px; padding: 4px 10px;" onclick="rejectTerm('slang-cand-1')">Reject</button>
            </div>
          </td>
        </tr>

        <tr id="slang-cand-2">
          <td><strong style="color: var(--md-sys-color-primary); font-size: 15px;">chaprasi_dox</strong></td>
          <td>Hindi Colloquial</td>
          <td class="table-numbers">19 occurrences</td>
          <td><span class="badge badge-urgent">Urgent Risk (0.89)</span></td>
          <td>Slang Drift Heuristic</td>
          <td>
            <div style="display: flex; gap: 8px;">
              <button type="button" class="btn btn-success" style="font-size: 12px; padding: 4px 10px;" onclick="confirmTerm('slang-cand-2', 'chaprasi_dox')">Confirm</button>
              <button type="button" class="btn btn-danger" style="font-size: 12px; padding: 4px 10px;" onclick="rejectTerm('slang-cand-2')">Reject</button>
            </div>
          </td>
        </tr>

        <tr id="slang-cand-3">
          <td><strong style="color: var(--md-sys-color-primary); font-size: 15px;">vibe_check_night</strong></td>
          <td>Slang Euphemism</td>
          <td class="table-numbers">12 occurrences</td>
          <td><span class="badge badge-review">Review (0.45)</span></td>
          <td>Clustering Engine</td>
          <td>
            <div style="display: flex; gap: 8px;">
              <button type="button" class="btn btn-success" style="font-size: 12px; padding: 4px 10px;" onclick="confirmTerm('slang-cand-3', 'vibe_check_night')">Confirm</button>
              <button type="button" class="btn btn-danger" style="font-size: 12px; padding: 4px 10px;" onclick="rejectTerm('slang-cand-3')">Reject</button>
            </div>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</div>

<!-- Active Approved Slang Lexicon -->
<div class="card">
  <h2 class="card-title">Active Harassment Lexicon Sample</h2>
  <div style="display: flex; flex-wrap: wrap; gap: 8px; margin-top: 12px;">
    <span class="badge badge-built">tharki (Weight: 0.85)</span>
    <span class="badge badge-built">kutte (Weight: 0.70)</span>
    <span class="badge badge-built">mar ja (Weight: 0.95)</span>
    <span class="badge badge-built">chup chap sun (Weight: 0.65)</span>
    <span class="badge badge-built">doxx_you (Weight: 0.90)</span>
    <span class="badge badge-built">leak_kardoonga (Weight: 0.95)</span>
    <span class="badge badge-simulated">cyberflash_sim (Weight: 0.85)</span>
  </div>
</div>

<script>
  function confirmTerm(rowId, term) {
    document.getElementById(rowId).style.opacity = '0.3';
    showToast('Confirmed slang term: ' + term + '. Next matching message will be flagged.');
  }

  function rejectTerm(rowId) {
    document.getElementById(rowId).remove();
    showToast('Rejected slang term candidate.');
  }

  function showAddTermModal() {
    const term = prompt('Enter new slang term to register:');
    if (term) {
      showToast('Term \"' + term + '\" added to slang registry.');
    }
  }
</script>"##
        .to_string()
}
