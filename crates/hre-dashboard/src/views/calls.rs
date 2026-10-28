//! Call Log & Voicemail Processing View.

pub fn render_calls_page() -> String {
    r##"<div class="page-header">
  <div>
    <h1 class="page-title">Calls & Audio</h1>
  </div>

  <div>
    <span class="badge badge-roadmap">Audio upload unavailable</span>
  </div>
</div>

<div style="display: grid; grid-template-columns: 1fr 1fr; gap: 24px; align-items: start;">
  <!-- Left: CSV Call Log Import Form -->
  <div class="card">
    <h2 class="card-title">Call Logs (CSV)</h2>

    <form id="call-csv-form" onsubmit="importCallCsv(event)">
      <div class="form-group"><label class="form-label" for="call-platform">Platform</label><input id="call-platform" name="platform" class="form-control" required /></div>
      <div class="form-group"><label class="form-label" for="call-conversation">Conversation ID</label><input id="call-conversation" name="conversation_id" class="form-control" required maxlength="256" /></div>
      <div class="form-group">
        <label class="form-label" for="csv-file">CSV File (Max 2 MiB)</label>
        <input type="file" id="csv-file" name="file" accept=".csv" class="form-control" required />
      </div>

      <button type="submit" class="btn btn-primary" style="width: 100%;">
        Analyze Telephony Patterns
      </button>
    </form>

    <div id="csv-result" style="margin-top: 16px;"></div>
  </div>

  <!-- Right: Voicemail / Audio Clip Analysis -->
  <div class="card">
    <h2 class="card-title">Voicemail & Audio Screening</h2>

    <p>Recording bytes are not accepted by the current API. No transcription is available here.</p><form id="audio-form"><fieldset disabled style="border:0;padding:0">
      <div class="form-group">
        <label class="form-label" for="audio-file">Audio File (WAV/MP3/OGG)</label>
        <input type="file" id="audio-file" name="recording" accept="audio/*" class="form-control" required />
      </div>

      <div class="form-group">
        <label class="form-label" for="audio-sender">Sender Identifier (Pseudonymous)</label>
        <input type="text" id="audio-sender" name="sender_id" class="form-control" />
      </div>

      <button type="submit" class="btn btn-primary" style="width: 100%;">
        Submit Audio Analysis Job
      </button>
    </fieldset></form>

    <div id="audio-result" style="margin-top: 16px;"></div>
  </div>
</div>

<!-- Active Telephony & Audio Jobs Table -->
<div class="card" style="margin-top: 24px;">
  <div style="display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 14px;">
    <h2 class="card-title" style="margin-bottom: 0;">Recent Async Ingestion Jobs</h2>
    <span style="font-size: 12px; color: var(--md-sys-color-on-surface-variant);">Job history unavailable</span>
  </div>

  <div class="table-container">
    <table class="table">
      <thead>
        <tr>
          <th>Job ID</th>
          <th>Type</th>
          <th>Events / Audio Duration</th>
          <th>Risk Tier Flagged</th>
          <th>Processing Status</th>
          <th>Completed</th>
        </tr>
      </thead>
      <tbody><tr><td colspan="6">No job history connected.</td></tr></tbody>
    </table>
  </div>
</div>
<script>
async function importCallCsv(event) {
  event.preventDefault();
  const form = event.target;
  const result = document.getElementById('csv-result');
  const button = form.querySelector('button[type="submit"]');
  button.disabled = true;
  try {
    const file = form.elements.file.files[0];
    if (!file || file.size > 2 * 1024 * 1024) throw new Error('Select a CSV file up to 2 MiB.');
    const response = await dashboardRequest('/v1/calllogs/import', {
      platform: form.elements.platform.value,
      conversation_id: form.elements.conversation_id.value,
      csv: await file.text(), draft: false
    });
    const pre = document.createElement('pre');
    pre.style.whiteSpace = 'pre-wrap';
    pre.textContent = JSON.stringify(response, null, 2);
    result.replaceChildren(pre);
  } catch (error) {
    result.textContent = error.message;
  } finally {
    button.disabled = false;
  }
}
</script>"##
        .to_string()
}
