//! Base HTML Shell layout for the Adapt Moderator Dashboard.

pub fn render_page(title: &str, active_nav: &str, content: &str) -> String {
    let nav_items = [
        (
            "queue",
            "/dashboard/queue",
            "Live Queue",
            r#"<path d="M4 6h16v2H4zm0 5h16v2H4zm0 5h16v2H4z"/>"#,
        ),
        (
            "overview",
            "/dashboard",
            "Overview",
            r#"<path d="M3 13h8V3H3v10zm0 8h8v-6H3v6zm10 0h8V11h-8v10zm0-18v6h8V3h-8z"/>"#,
        ),
        (
            "timeline",
            "/dashboard/timelines",
            "Timelines",
            r#"<path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm-2 10h-4v4h-2v-4H7v-2h4V7h2v4h4v2z"/>"#,
        ),
        (
            "slang",
            "/dashboard/slang",
            "Slang Intelligence",
            r#"<path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/>"#,
        ),
        (
            "metrics",
            "/dashboard/metrics",
            "Evaluation Metrics",
            r#"<path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zM9 17H7v-7h2v7zm4 0h-2V7h2v10zm4 0h-2v-4h2v4z"/>"#,
        ),
        (
            "calls",
            "/dashboard/calls",
            "Calls & Audio",
            r#"<path d="M20.01 15.38c-1.23 0-2.42-.2-3.53-.56-.35-.12-.74-.03-1.01.24l-1.57 1.97c-2.83-1.35-5.48-3.9-6.89-6.83l1.95-1.66c.27-.28.35-.67.24-1.02-.37-1.11-.56-2.3-.56-3.53 0-.54-.45-.99-.99-.99H4.19C3.65 3 3 3.24 3 3.99 3 13.28 10.73 21 20.01 21c.71 0 .99-.63.99-1.18v-3.45c0-.54-.45-.99-.99-.99z"/>"#,
        ),
        (
            "evidence",
            "/dashboard/evidence",
            "Evidence Export",
            r#"<path d="M14 2H6c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6zm2 16H8v-2h8v2zm0-4H8v-2h8v2zm-3-5V3.5L18.5 9H13z"/>"#,
        ),
        (
            "demo",
            "/dashboard/demo",
            "Demo Scenarios",
            r#"<path d="M8 5v14l11-7z"/>"#,
        ),
    ];

    let mut nav_html = String::new();
    for (id, url, label, svg_path) in nav_items {
        let active_class = if active_nav == id { "active" } else { "" };
        nav_html.push_str(&format!(
            r##"<a href="{}" class="nav-item {}">
  <svg class="nav-icon" viewBox="0 0 24 24">{}</svg>
  <span>{}</span>
</a>"##,
            url, active_class, svg_path, label
        ));
    }

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{} — Adapt Moderation Dashboard</title>
  <link rel="stylesheet" href="/dashboard/static/css/styles.css">
  <script src="/dashboard/static/vendor/htmx/htmx.min.js"></script>
  <script src="/dashboard/static/vendor/htmx/sse.js"></script>
  <script type="module" src="/dashboard/static/vendor/m3e/m3e.min.js"></script>
</head>
<body>
  <div class="app-shell">
    <header class="top-bar">
      <a href="/dashboard" class="top-bar-brand">
        <div class="top-bar-logo">A</div>
        <span class="top-bar-title">Adapt Harassment Engine</span>
      </a>

      <div class="top-bar-actions">
        <div id="connection-status" class="badge badge-built">
          <span style="width: 8px; height: 8px; border-radius: 50%; background: currentColor; display: inline-block;"></span>
          Feed Active
        </div>

        <!-- Moderator Token Input for secure Bearer session -->
        <div style="display: flex; align-items: center; gap: 8px;">
          <input type="password" id="mod-token-input" placeholder="Moderator Token" class="form-control" style="width: 140px; height: 32px; font-size: 12px;" />
          <button type="button" class="btn btn-secondary" style="height: 32px; padding: 0 10px; font-size: 12px;" onclick="saveToken()">Set</button>
        </div>

        <!-- Theme Toggle -->
        <button type="button" class="btn btn-secondary" style="height: 32px; width: 32px; padding: 0;" title="Toggle Dark/Light Mode" onclick="toggleTheme()">
          <svg style="width: 16px; height: 16px; fill: currentColor;" viewBox="0 0 24 24">
            <path d="M12 3c-4.97 0-9 4.03-9 9s4.03 9 9 9 9-4.03 9-9c0-.46-.04-.92-.1-1.36-.98 1.37-2.58 2.26-4.4 2.26-2.98 0-5.4-2.42-5.4-5.4 0-1.81.89-3.42 2.26-4.4-.44-.06-.9-.1-1.36-.1z"/>
          </svg>
        </button>
      </div>
    </header>

    <div class="main-layout">
      <nav class="nav-rail" aria-label="Main Navigation">
        {}
      </nav>

      <main class="content-area" id="main-content">
        {}
      </main>
    </div>
  </div>

  <div id="toast-container" class="toast-container"></div>

  <script>
    // Theme Switcher
    function toggleTheme() {{
      const current = document.documentElement.getAttribute('data-theme');
      const next = current === 'dark' ? 'light' : 'dark';
      document.documentElement.setAttribute('data-theme', next);
      localStorage.setItem('adapt_theme', next);
    }}
    const savedTheme = localStorage.getItem('adapt_theme');
    if (savedTheme) {{
      document.documentElement.setAttribute('data-theme', savedTheme);
    }}

    // Moderator Token Storage & HTMX request injection
    function saveToken() {{
      const val = document.getElementById('mod-token-input').value;
      if (val) {{
        localStorage.setItem('adapt_mod_token', val);
        showToast('Moderator token saved');
      }}
    }}
    const savedToken = localStorage.getItem('adapt_mod_token');
    if (savedToken) {{
      document.getElementById('mod-token-input').value = savedToken;
    }}

    document.body.addEventListener('htmx:configRequest', function(evt) {{
      const token = localStorage.getItem('adapt_mod_token');
      if (token) {{
        evt.detail.headers['Authorization'] = 'Bearer ' + token;
      }}
    }});

    function showToast(msg) {{
      const container = document.getElementById('toast-container');
      const el = document.createElement('div');
      el.className = 'toast';
      el.textContent = msg;
      container.appendChild(el);
      setTimeout(() => el.remove(), 3500);
    }}
  </script>
</body>
</html>"##,
        title, nav_html, content
    )
}
