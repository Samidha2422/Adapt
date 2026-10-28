//! Call Log & Voicemail Processing View.

pub fn render_calls_page() -> String {
    r##"<div class="page-header">
  <div>
    <h1 class="page-title">Call Logs & Audio Ingestion</h1>
    <p class="page-subtitle">Batch CSV telephony metadata import, voicemail tone analysis, and asynchronous audio job tracking.</p>
  </div>

  <div>
    <span class="badge badge-built" style="font-size: 13px;">Transient Audio Shield Active</span>
  </div>
</div>

<div style="display: grid; grid-template-columns: 1fr 1fr; gap: 24px; align-items: start;">
  <!-- Left: CSV Call Log Import Form -->
  <div class="card">
    <h2 class="card-title">Import Telephony Call Logs (CSV)</h2>
    <p style="font-size: 13px; color: var(--md-sys-color-on-surface-variant); margin-bottom: 16px;">
      Upload carrier or device call records (timestamp, caller, callee, duration_sec, answered, direction) to evaluate stalking velocity and overnight bursts.
    </p>

    <form id="call-csv-form" hx-post="/v1/calllogs/import" hx-encoding="multipart/form-data" hx-target="#csv-result" hx-swap="innerHTML">
      <div class="form-group">
        <label class="form-label" for="csv-file">Select CSV File (Max 1MB)</label>
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
    <h2 class="card-title">Voicemail & Audio Note Screening</h2>
    <p style="font-size: 13px; color: var(--md-sys-color-on-surface-variant); margin-bottom: 16px;">
      Submit consensual voicemail recording for ASR transcription, acoustic aggression analysis, and threat scoring. Raw audio is wiped transiently.
    </p>

    <form id="audio-form" hx-post="/v1/recordings/analyze" hx-encoding="multipart/form-data" hx-target="#audio-result" hx-swap="innerHTML">
      <div class="form-group">
        <label class="form-label" for="audio-file">Select Audio File (WAV/MP3/OGG)</label>
        <input type="file" id="audio-file" name="recording" accept="audio/*" class="form-control" required />
      </div>

      <div class="form-group">
        <label class="form-label" for="audio-sender">Sender Identifier (Pseudonymous)</label>
        <input type="text" id="audio-sender" name="sender_id" value="anon_caller_382" class="form-control" />
      </div>

      <button type="submit" class="btn btn-primary" style="width: 100%;">
        Submit Audio Analysis Job
      </button>
    </form>

    <div id="audio-result" style="margin-top: 16px;"></div>
  </div>
</div>

<!-- Active Telephony & Audio Jobs Table -->
<div class="card" style="margin-top: 24px;">
  <div style="display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 14px;">
    <h2 class="card-title" style="margin-bottom: 0;">Recent Async Ingestion Jobs</h2>
    <span style="font-size: 12px; color: var(--md-sys-color-on-surface-variant);">Auto-refreshed via <code>/v1/jobs/{{id}}</code></span>
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
      <tbody>
        <tr>
          <td><code>job-8f2a1b9c</code></td>
          <td>Call Log CSV</td>
          <td class="table-numbers">40 calls (Overnight)</td>
          <td><span class="badge badge-urgent">Urgent (92/100)</span></td>
          <td><span class="badge badge-built">Completed</span></td>
          <td class="table-numbers">2.4s</td>
        </tr>
        <tr>
          <td><code>job-3c7d9e4a</code></td>
          <td>Voicemail Audio</td>
          <td class="table-numbers">18.4s ASR (Threat)</td>
          <td><span class="badge badge-high">High (84/100)</span></td>
          <td><span class="badge badge-built">Completed</span></td>
          <td class="table-numbers">1.1s</td>
        </tr>
        <tr>
          <td><code>job-1e4b8f0d</code></td>
          <td>Telephony CSV</td>
          <td class="table-numbers">12 calls (Couple)</td>
          <td><span class="badge badge-low">Low (14/100)</span></td>
          <td><span class="badge badge-built">Completed</span></td>
          <td class="table-numbers">0.8s</td>
        </tr>
      </tbody>
    </table>
  </div>
</div>"##
        .to_string()
}
