use crate::chain::EvidenceExport;

/// Renders a printable HTML report with print-optimized CSS and legal certificate draft aid.
pub fn render_printable_html(export: &EvidenceExport) -> String {
    let mut rows = String::new();
    for (i, r) in export.records.iter().enumerate() {
        let reasons_list = if r.reasons.is_empty() {
            "None".to_string()
        } else {
            r.reasons
                .iter()
                .map(|re| format!("<span class='reason-tag'>{}</span>", html_escape(&re.code)))
                .collect::<Vec<_>>()
                .join(" ")
        };

        rows.push_str(&format!(
            "<tr>\
                <td>{}</td>\
                <td><span class='badge'>{}</span></td>\
                <td>{}<br><small class='muted'>{}</small></td>\
                <td>{}</td>\
                <td><span class='tier-badge tier-{}'>{} ({})</span></td>\
                <td>{}</td>\
                <td class='hash-cell'><code>{}...</code></td>\
            </tr>",
            i + 1,
            html_escape(&r.kind),
            html_escape(&r.timestamp_local),
            html_escape(&r.timestamp_utc),
            html_escape(&r.summary),
            html_escape(&r.tier.to_lowercase()),
            html_escape(&r.tier),
            r.severity,
            reasons_list,
            &r.canonical_hash[..12]
        ));
    }

    let mut helplines = String::new();
    for h in &export.certificate_draft.emergency_helplines {
        helplines.push_str(&format!("<li>{}</li>", html_escape(h)));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Evidence Timeline Report - {subject}</title>
<style>
body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; margin: 2rem; color: #1e293b; background: #fff; }}
h1, h2, h3 {{ color: #0f172a; margin-bottom: 0.5rem; }}
.meta-box {{ background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 8px; padding: 1.25rem; margin-bottom: 2rem; }}
.meta-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1rem; }}
.meta-item {{ font-size: 0.9rem; }}
.meta-label {{ color: #64748b; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; }}
table {{ width: 100%; border-collapse: collapse; margin-bottom: 2rem; font-size: 0.9rem; }}
th, td {{ border: 1px solid #e2e8f0; padding: 0.6rem 0.75rem; text-align: left; vertical-align: top; }}
th {{ background: #f1f5f9; font-weight: 600; color: #334155; }}
.muted {{ color: #64748b; }}
.badge {{ background: #e2e8f0; padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.8rem; font-weight: 500; }}
.tier-badge {{ padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.8rem; font-weight: 600; }}
.tier-urgent {{ background: #fee2e2; color: #b91c1c; }}
.tier-high {{ background: #ffedd5; color: #c2410c; }}
.tier-review {{ background: #fef9c3; color: #a16207; }}
.tier-low {{ background: #dcfce7; color: #15803d; }}
.reason-tag {{ background: #f1f5f9; border: 1px solid #cbd5e1; border-radius: 3px; padding: 0.1rem 0.3rem; font-size: 0.75rem; }}
.hash-cell {{ font-family: monospace; font-size: 0.75rem; }}
.certificate-box {{ border: 2px dashed #94a3b8; border-radius: 8px; padding: 1.5rem; margin-top: 2rem; background: #fafafa; }}
.footer-resources {{ margin-top: 2rem; border-top: 1px solid #e2e8f0; padding-top: 1rem; font-size: 0.85rem; color: #475569; }}
@media print {{
    body {{ margin: 0.5in; }}
    .certificate-box {{ page-break-inside: avoid; }}
    table {{ page-break-inside: auto; }}
    tr {{ page-break-inside: avoid; page-break-after: auto; }}
}}
</style>
</head>
<body>
<h1>Multimodal Harassment Risk Engine - Evidence Timeline</h1>
<div class="meta-box">
    <div class="meta-grid">
        <div class="meta-item"><div class="meta-label">Subject Pseudonym</div><strong>{subject}</strong></div>
        <div class="meta-item"><div class="meta-label">Export ID</div><code>{export_id}</code></div>
        <div class="meta-item"><div class="meta-label">Generated UTC</div>{generated_at}</div>
        <div class="meta-item"><div class="meta-label">Record Count</div><strong>{record_count} events</strong></div>
        <div class="meta-item"><div class="meta-label">Manifest SHA-256</div><code class="hash-cell">{manifest_hash}</code></div>
        <div class="meta-item"><div class="meta-label">HMAC Signature</div><code class="hash-cell">{hmac_sig}</code></div>
    </div>
</div>

<h2>Cryptographic Event Chain</h2>
<table>
    <thead>
        <tr>
            <th>#</th>
            <th>Type</th>
            <th>Timestamp</th>
            <th>Summary / Redacted Content</th>
            <th>Tier (Severity)</th>
            <th>Reasons</th>
            <th>Hash</th>
        </tr>
    </thead>
    <tbody>
        {rows}
    </tbody>
</table>

<div class="certificate-box">
    <h3>Electronic Evidence Certificate Draft Aid</h3>
    <p><strong>Legal Framework:</strong> {act_ref}</p>
    <p><strong>Part A Statement (Integrity):</strong> {part_a}</p>
    <p><strong>Part B Advisory:</strong> <em>{part_b}</em></p>
</div>

<div class="footer-resources">
    <h3>Emergency Reporting & Victim Support Helplines</h3>
    <ul>
        {helplines}
    </ul>
</div>
</body>
</html>"#,
        subject = html_escape(&export.subject_pseudonym),
        export_id = export.export_id,
        generated_at = html_escape(&export.generated_at),
        record_count = export.record_count,
        manifest_hash = html_escape(&export.manifest_hash),
        hmac_sig = html_escape(&export.hmac_signature),
        rows = rows,
        act_ref = html_escape(&export.certificate_draft.act_reference),
        part_a = html_escape(&export.certificate_draft.part_a_statement),
        part_b = html_escape(&export.certificate_draft.part_b_advisory),
        helplines = helplines,
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
