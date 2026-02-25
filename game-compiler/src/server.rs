use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use notify::{Event, RecursiveMode, Watcher};
use tower_livereload::LiveReloadLayer;

use crate::codegen::CompileOutput;

struct DevState {
    source_path: PathBuf,
    tag_name: String,
}

/// Start the dev server with hot-reload for a `.game` file.
pub async fn run_dev_server(path: PathBuf, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let tag_name = crate::derive_tag_name(&path);
    let state = Arc::new(Mutex::new(DevState {
        source_path: path.clone(),
        tag_name,
    }));

    let livereload = LiveReloadLayer::new();
    let reloader = livereload.reloader();

    // File watcher
    let watch_path = path.clone();
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
        if let Ok(event) = res {
            if event.kind.is_modify() {
                reloader.reload();
            }
        }
    })?;
    watcher.watch(path.parent().unwrap_or(path.as_ref()), RecursiveMode::NonRecursive)?;

    let app = Router::new()
        .route("/", get(serve_preview))
        .route("/component.js", get(serve_component))
        .route("/preview.html", get(serve_fullscreen))
        .layer(livereload)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    eprintln!("GAME dev server");
    eprintln!("  file:       {}", watch_path.display());
    eprintln!("  preview:    http://localhost:{port}/");
    eprintln!("  component:  http://localhost:{port}/component.js");
    eprintln!("  fullscreen: http://localhost:{port}/preview.html");
    eprintln!("  watching for changes...");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    // Keep watcher alive
    drop(watcher);
    Ok(())
}

// ── Compile helper ────────────────────────────────────────────────────

enum CompileResult {
    Ok(CompileOutput),
    Err(String),
}

fn compile_source(state: &Arc<Mutex<DevState>>) -> (String, CompileResult) {
    let (source_path, tag_name) = match state.lock() {
        Ok(s) => (s.source_path.clone(), s.tag_name.clone()),
        Err(e) => return (String::new(), CompileResult::Err(format!("Lock error: {e}"))),
    };
    let source = match std::fs::read_to_string(&source_path) {
        Ok(s) => s,
        Err(e) => return (tag_name, CompileResult::Err(format!("Read error: {e}"))),
    };
    match crate::compile_full(&source) {
        Ok(output) => (tag_name, CompileResult::Ok(output)),
        Err(e) => (tag_name, CompileResult::Err(format!("{e}"))),
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

// ── CSS ───────────────────────────────────────────────────────────────

fn build_css(tag_name: &str) -> String {
    format!(
        r#"* {{ margin: 0; padding: 0; box-sizing: border-box; }}
html, body {{ width: 100%; height: 100%; background: #0A0A0A; color: #A0A0A0;
  font-family: 'JetBrains Mono', monospace; font-size: 12px; }}

/* ── Toolbar ─────────────────────────────────────── */
.toolbar {{
  height: 36px; display: flex; align-items: center; padding: 0 14px;
  border-bottom: 1px solid #2A2A2A; font-size: 11px; gap: 10px;
  background: #0A0A0A; user-select: none;
}}
.toolbar .label {{ color: #666; }}
.toolbar .tag {{ color: #D4AF37; font-weight: 600; }}
.toolbar .badge {{
  color: #A0A0A0; background: #1F1F1F; padding: 2px 8px;
  border-radius: 3px; font-size: 10px;
}}
.toolbar .sep {{ color: #333; }}
.toolbar .actions {{ margin-left: auto; display: flex; gap: 6px; }}
.toolbar .actions button, .toolbar .actions a {{
  background: #1F1F1F; border: 1px solid #2A2A2A; color: #A0A0A0;
  padding: 3px 10px; border-radius: 3px; font-size: 10px; cursor: pointer;
  font-family: inherit; text-decoration: none; display: inline-flex;
  align-items: center;
}}
.toolbar .actions button:hover, .toolbar .actions a:hover {{
  color: #FFF; border-color: #444;
}}
.toolbar .actions button.copied {{
  color: #22C55E; border-color: #22C55E; transition: none;
}}

/* ── Warnings ────────────────────────────────────── */
.warnings-bar {{
  background: rgba(245,158,11,0.06); border-bottom: 1px solid #2A2A2A;
  padding: 0 14px; cursor: pointer; user-select: none;
}}
.warnings-header {{
  height: 28px; display: flex; align-items: center; gap: 8px;
  font-size: 11px; color: #F59E0B;
}}
.warnings-header .arrow {{ transition: transform 0.15s; display: inline-block; }}
.warnings-bar.open .warnings-header .arrow {{ transform: rotate(90deg); }}
.warnings-list {{
  display: none; padding: 0 0 8px 0;
}}
.warnings-bar.open .warnings-list {{ display: block; }}
.warnings-list .warn-item {{
  padding: 4px 0 4px 12px; font-size: 11px; color: #F59E0B;
  border-left: 2px solid #F59E0B; margin: 4px 0;
}}

/* ── Split view ──────────────────────────────────── */
.split {{
  display: grid; grid-template-columns: 1fr 4px 1fr;
  height: calc(100vh - var(--top-offset, 36px));
}}
.panel {{ position: relative; overflow: hidden; }}
.panel-label {{
  position: absolute; top: 8px; left: 12px; font-size: 10px; color: #444;
  z-index: 10; text-transform: uppercase; letter-spacing: 1px;
}}
.divider {{
  background: #2A2A2A; cursor: col-resize; position: relative;
}}
.divider:hover {{ background: #444; }}
iframe {{ width: 100%; height: 100%; border: none; }}

/* ── Component panel ─────────────────────────────── */
.component-panel {{ display: flex; flex-direction: column; }}
.component-view {{
  flex: 1; display: flex; align-items: center; justify-content: center;
  padding: 24px; position: relative;
}}
.comp-wrapper {{ display: flex; align-items: center; justify-content: center; }}
.comp-wrapper.size-sm {{ width: 200px; height: 200px; }}
.comp-wrapper.size-md {{ width: 400px; height: 400px; }}
.comp-wrapper.size-lg {{ width: 100%; aspect-ratio: 1; max-height: 100%; }}
.comp-wrapper {tag_name} {{ width: 100%; height: 100%; }}

/* ── Size toggle ─────────────────────────────────── */
.size-bar {{
  display: flex; gap: 4px; padding: 8px 14px;
  border-top: 1px solid #2A2A2A;
}}
.size-bar button {{
  background: #1F1F1F; border: 1px solid #2A2A2A; color: #666;
  padding: 3px 10px; border-radius: 3px; font-size: 10px;
  cursor: pointer; font-family: inherit;
}}
.size-bar button:hover {{ color: #A0A0A0; border-color: #444; }}
.size-bar button.active {{ color: #D4AF37; border-color: #D4AF37; }}

/* ── Sliders ─────────────────────────────────────── */
.sliders {{
  padding: 10px 14px; border-top: 1px solid #2A2A2A; font-size: 11px;
}}
.sliders-title {{
  font-size: 10px; color: #444; text-transform: uppercase;
  letter-spacing: 1px; margin-bottom: 6px;
}}
.slider-row {{
  display: flex; align-items: center; gap: 8px; margin-bottom: 6px;
}}
.slider-row label {{ width: 80px; color: #666; }}
.slider-row input[type="range"] {{
  flex: 1; -webkit-appearance: none; height: 3px;
  background: #333; border-radius: 2px; outline: none;
}}
.slider-row input[type="range"]::-webkit-slider-thumb {{
  -webkit-appearance: none; width: 10px; height: 10px;
  border-radius: 50%; background: #D4AF37; cursor: pointer;
}}
.slider-row .val {{ width: 40px; text-align: right; color: #888; font-size: 10px; }}"#,
        tag_name = tag_name,
    )
}

// ── Toolbar label ─────────────────────────────────────────────────────

fn build_toolbar_label(output: &CompileOutput, tag_name: &str) -> String {
    let mut parts = Vec::new();
    parts.push(format!(r#"<span class="label">GAME dev</span>"#));
    parts.push(format!(
        r#"<span class="sep">|</span><span class="tag">&lt;{}&gt;</span>"#,
        html_escape(tag_name)
    ));
    if !output.params.is_empty() {
        parts.push(format!(
            r#"<span class="badge">{} param{}</span>"#,
            output.params.len(),
            if output.params.len() == 1 { "" } else { "s" }
        ));
    }
    if !output.data_fields.is_empty() {
        parts.push(format!(
            r#"<span class="badge">{} data</span>"#,
            output.data_fields.len()
        ));
    }
    if output.uses_audio {
        parts.push(r#"<span class="badge">audio</span>"#.to_string());
    }
    let mode = match &output.render_mode {
        crate::codegen::RenderMode::Flat => "flat",
        crate::codegen::RenderMode::Raymarch { .. } => "raymarch",
    };
    parts.push(format!(r#"<span class="badge">{mode}</span>"#));
    if !output.warnings.is_empty() {
        parts.push(format!(
            r#"<span class="badge" style="color:#F59E0B">&#9888; {}</span>"#,
            output.warnings.len()
        ));
    }
    parts.join("\n  ")
}

// ── Slider HTML ───────────────────────────────────────────────────────

fn build_sliders_html(output: &CompileOutput) -> String {
    if output.data_fields.is_empty() {
        return String::new();
    }
    let mut html = String::from(r#"<div class="sliders"><div class="sliders-title">data signals</div>"#);
    for field in &output.data_fields {
        html.push_str(&format!(
            r#"<div class="slider-row">
  <label>{field}</label>
  <input type="range" min="0" max="1" step="0.01" value="0.5" data-field="{field}">
  <span class="val">0.50</span>
</div>"#,
        ));
    }
    html.push_str("</div>");
    html
}

// ── Warnings HTML ─────────────────────────────────────────────────────

fn build_warnings_html(warnings: &[String]) -> String {
    if warnings.is_empty() {
        return String::new();
    }
    let mut html = String::from(r#"<div class="warnings-bar" id="warnings-bar" onclick="this.classList.toggle('open')">"#);
    html.push_str(&format!(
        r#"<div class="warnings-header"><span class="arrow">&#9654;</span> {} compiler warning{} — click to expand</div>"#,
        warnings.len(),
        if warnings.len() == 1 { "" } else { "s" }
    ));
    html.push_str(r#"<div class="warnings-list">"#);
    for w in warnings {
        html.push_str(&format!(
            r#"<div class="warn-item">{}</div>"#,
            html_escape(w)
        ));
    }
    html.push_str("</div></div>");
    html
}

// ── Quick actions HTML ────────────────────────────────────────────────

fn build_quick_actions_html(tag_name: &str) -> String {
    format!(
        r#"<div class="actions">
  <button onclick="copyText('&lt;script type=&quot;module&quot; src=&quot;./component.js&quot;&gt;&lt;/script&gt;', this)" title="Copy import tag">Copy Import</button>
  <button onclick="copyText('&lt;{tag}&gt;&lt;/{tag}&gt;', this)" title="Copy HTML tag">Copy HTML</button>
  <a href="/preview.html" target="_blank" title="Open standalone HTML">Fullscreen</a>
</div>"#,
        tag = html_escape(tag_name),
    )
}

// ── Success page assembly ─────────────────────────────────────────────

fn build_preview_page(output: &CompileOutput, tag_name: &str) -> String {
    let css = build_css(tag_name);
    let toolbar_label = build_toolbar_label(output, tag_name);
    let quick_actions = build_quick_actions_html(tag_name);
    let warnings_html = build_warnings_html(&output.warnings);
    let sliders_html = build_sliders_html(output);
    let html_full = crate::runtime::wrap_html_full(output);
    let html_json = serde_json_inline(&html_full);

    // Calculate top offset for split panel height
    let has_warnings = !output.warnings.is_empty();
    // 36px toolbar + (28px warnings bar if present, grows when open but that's JS)
    let top_offset = if has_warnings { "64px" } else { "36px" };

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>GAME Dev — {tag_name}</title>
<style>
  :root {{ --top-offset: {top_offset}; }}
  {css}
</style>
</head>
<body>
<div class="toolbar">
  {toolbar_label}
  {quick_actions}
</div>
{warnings_html}
<div class="split">
  <div class="panel">
    <div class="panel-label">html preview</div>
    <iframe id="preview" srcdoc=""></iframe>
  </div>
  <div class="divider" id="divider"></div>
  <div class="panel component-panel">
    <div class="panel-label">component</div>
    <div class="component-view">
      <div class="comp-wrapper size-sm" id="comp-wrapper">
        <{tag_name} id="comp"></{tag_name}>
      </div>
    </div>
    <div class="size-bar">
      <button class="active" onclick="setSize('sm', this)">SM</button>
      <button onclick="setSize('md', this)">MD</button>
      <button onclick="setSize('lg', this)">LG</button>
    </div>
    {sliders_html}
  </div>
</div>
<script type="module" src="/component.js"></script>
<script>
  // Inject HTML preview
  const html = {html_json};
  document.getElementById('preview').srcdoc = html;

  // Size toggle
  function setSize(size, btn) {{
    const wrapper = document.getElementById('comp-wrapper');
    wrapper.className = 'comp-wrapper size-' + size;
    document.querySelectorAll('.size-bar button').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
  }}

  // Copy helper
  function copyText(text, btn) {{
    // Decode HTML entities for clipboard
    const ta = document.createElement('textarea');
    ta.innerHTML = text;
    navigator.clipboard.writeText(ta.value).then(() => {{
      btn.classList.add('copied');
      const orig = btn.textContent;
      btn.textContent = 'Copied!';
      setTimeout(() => {{ btn.classList.remove('copied'); btn.textContent = orig; }}, 1200);
    }});
  }}

  // Slider wiring
  document.querySelectorAll('.slider-row input[type="range"]').forEach(slider => {{
    const field = slider.dataset.field;
    const valSpan = slider.nextElementSibling;
    const comp = document.getElementById('comp');

    // Set initial value
    if (comp && field) comp[field] = parseFloat(slider.value);

    slider.addEventListener('input', () => {{
      const v = parseFloat(slider.value);
      valSpan.textContent = v.toFixed(2);
      if (comp && field) comp[field] = v;
    }});
  }});

  // Divider drag-resize
  const divider = document.getElementById('divider');
  const split = divider.parentElement;
  let dragging = false;
  divider.addEventListener('mousedown', (e) => {{
    dragging = true;
    e.preventDefault();
  }});
  document.addEventListener('mousemove', (e) => {{
    if (!dragging) return;
    const rect = split.getBoundingClientRect();
    const frac = (e.clientX - rect.left) / rect.width;
    const clamped = Math.max(0.2, Math.min(0.8, frac));
    split.style.gridTemplateColumns = `${{clamped}}fr 4px ${{1 - clamped}}fr`;
  }});
  document.addEventListener('mouseup', () => {{ dragging = false; }});
</script>
</body>
</html>"##,
        tag_name = tag_name,
        top_offset = top_offset,
        css = css,
        toolbar_label = toolbar_label,
        quick_actions = quick_actions,
        warnings_html = warnings_html,
        sliders_html = sliders_html,
        html_json = html_json,
    )
}

// ── Error page ────────────────────────────────────────────────────────

fn build_error_page(tag_name: &str, error: &str) -> String {
    let escaped = html_escape(error);
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>GAME Dev — Error</title>
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  html, body {{ width: 100%; height: 100%; background: #0A0A0A; color: #A0A0A0;
    font-family: 'JetBrains Mono', monospace; }}
  .error-container {{
    max-width: 640px; margin: 80px auto; padding: 32px;
    border: 1px solid #3F1818; border-radius: 8px;
    background: rgba(239,68,68,0.04);
  }}
  .error-header {{
    display: flex; align-items: center; gap: 10px; margin-bottom: 20px;
    font-size: 13px; color: #EF4444;
  }}
  .error-header .tag {{ color: #D4AF37; }}
  .error-message {{
    font-size: 12px; color: #EF4444; line-height: 1.7;
    white-space: pre-wrap; word-break: break-word;
    padding: 16px; background: #141414; border-radius: 4px;
    border-left: 3px solid #EF4444;
  }}
  .pulse {{
    margin-top: 24px; font-size: 11px; color: #666;
    display: flex; align-items: center; gap: 8px;
  }}
  .pulse-dot {{
    width: 6px; height: 6px; border-radius: 50%; background: #EF4444;
    animation: pulse 2s ease-in-out infinite;
  }}
  @keyframes pulse {{
    0%, 100% {{ opacity: 0.3; }}
    50% {{ opacity: 1; }}
  }}
</style>
</head>
<body>
<div class="error-container">
  <div class="error-header">
    <span>GAME dev</span>
    <span class="tag">&lt;{tag_name}&gt;</span>
    <span>— compilation error</span>
  </div>
  <div class="error-message">{escaped}</div>
  <div class="pulse">
    <div class="pulse-dot"></div>
    waiting for fix...
  </div>
</div>
</body>
</html>"##,
        tag_name = html_escape(tag_name),
        escaped = escaped,
    )
}

// ── Route handlers ────────────────────────────────────────────────────

/// Serve the full dev UI (split-pane, sliders, toolbar).
async fn serve_preview(State(state): State<Arc<Mutex<DevState>>>) -> Html<String> {
    let (tag_name, result) = compile_source(&state);
    match result {
        CompileResult::Ok(output) => Html(build_preview_page(&output, &tag_name)),
        CompileResult::Err(e) => Html(build_error_page(&tag_name, &e)),
    }
}

/// Serve the compiled Web Component JS module.
async fn serve_component(State(state): State<Arc<Mutex<DevState>>>) -> (
    [(axum::http::header::HeaderName, &'static str); 1],
    String,
) {
    let (tag_name, result) = compile_source(&state);
    let js = match result {
        CompileResult::Ok(output) => crate::runtime::wrap_web_component(&output, &tag_name),
        CompileResult::Err(e) => {
            let escaped = e.replace('\\', "\\\\").replace('\'', "\\'");
            format!("console.error('GAME: {escaped}');")
        }
    };
    ([(axum::http::header::CONTENT_TYPE, "text/javascript")], js)
}

/// Serve standalone HTML preview (no dev chrome).
async fn serve_fullscreen(State(state): State<Arc<Mutex<DevState>>>) -> Html<String> {
    let (tag_name, result) = compile_source(&state);
    match result {
        CompileResult::Ok(output) => Html(crate::runtime::wrap_html_full(&output)),
        CompileResult::Err(e) => Html(build_error_page(&tag_name, &e)),
    }
}

// ── Utilities ─────────────────────────────────────────────────────────

/// Simple JSON string encoding (avoid serde dependency just for this).
/// Also escapes `</` as `<\/` to prevent HTML parser from closing a
/// `<script>` block when this string is embedded in inline JS.
fn serde_json_inline(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    let mut prev = '\0';
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Escape </ as <\/ to avoid </script> closing the HTML script block
            '/' if prev == '<' => out.push_str("\\/"),
            c if c < '\x20' => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
        prev = c;
    }
    out.push('"');
    out
}
