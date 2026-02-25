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

// ── Utility functions ─────────────────────────────────────────────────

fn format_time(secs: f64) -> String {
    let m = (secs / 60.0) as u32;
    let s = (secs % 60.0) as u32;
    format!("{m}:{s:02}")
}

fn calc_timeline_duration(moments: &[crate::codegen::CompiledMoment]) -> f64 {
    moments
        .iter()
        .map(|m| {
            let max_t = m
                .transitions
                .iter()
                .filter_map(|t| t.duration_secs)
                .fold(0.0f64, f64::max);
            m.time_seconds + max_t
        })
        .fold(0.0f64, f64::max)
}

enum ParamKind {
    Data,
    Modulated,
    Arc,
    Static,
}

fn classify_param(
    param: &crate::codegen::CompiledParam,
    data_fields: &[String],
    arc_moments: &[crate::codegen::CompiledMoment],
    param_index: usize,
) -> ParamKind {
    let in_arc = arc_moments
        .iter()
        .any(|m| m.transitions.iter().any(|t| t.param_index == param_index));
    if in_arc {
        return ParamKind::Arc;
    }
    if param.mod_js.is_some() {
        return ParamKind::Modulated;
    }
    if data_fields.contains(&param.name) {
        return ParamKind::Data;
    }
    ParamKind::Static
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

/* ── Tab system ──────────────────────────────────── */
.tab-bar {{ display: flex; gap: 2px; }}
.tab-btn {{
  background: transparent; border: 1px solid transparent; color: #666;
  padding: 3px 12px; border-radius: 3px 3px 0 0; font-size: 10px;
  cursor: pointer; font-family: inherit; transition: color 0.15s;
}}
.tab-btn:hover {{ color: #A0A0A0; }}
.tab-btn.active {{ color: #D4AF37; border-color: #2A2A2A; border-bottom-color: #0A0A0A; background: #0A0A0A; }}
.tab-pane {{ display: none; }}
.tab-pane.active {{ display: block; }}

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

/* ── WGSL viewer ─────────────────────────────────── */
.wgsl-viewer {{
  height: 100%; overflow: auto; padding: 0;
  font-family: 'JetBrains Mono', monospace; font-size: 11px;
  line-height: 1.6;
}}
.wgsl-header {{
  padding: 8px 14px; border-bottom: 1px solid #2A2A2A;
  font-size: 10px; color: #666; display: flex; align-items: center; gap: 12px;
}}
.wgsl-header button {{
  background: #1F1F1F; border: 1px solid #2A2A2A; color: #A0A0A0;
  padding: 2px 8px; border-radius: 3px; font-size: 10px; cursor: pointer;
  font-family: inherit;
}}
.wgsl-header button:hover {{ color: #FFF; border-color: #444; }}
.wgsl-code {{
  padding: 12px 14px; white-space: pre; overflow-x: auto;
  counter-reset: line;
}}
.wgsl-code .line {{ display: block; }}
.wgsl-code .line::before {{
  counter-increment: line; content: counter(line);
  display: inline-block; width: 3em; text-align: right; margin-right: 1em;
  color: #333; user-select: none;
}}
.wgsl-kw {{ color: #D4AF37; }}
.wgsl-type {{ color: #22C55E; }}
.wgsl-num {{ color: #EF4444; }}
.wgsl-comment {{ color: #666; font-style: italic; }}
.wgsl-deco {{ color: #F59E0B; }}

/* ── API panel ───────────────────────────────────── */
.api-panel {{
  height: 100%; overflow: auto; padding: 16px;
  font-family: 'JetBrains Mono', monospace; font-size: 11px;
}}
.api-section {{ margin-bottom: 20px; }}
.api-section h3 {{
  font-size: 10px; color: #666; text-transform: uppercase;
  letter-spacing: 1px; margin-bottom: 8px; font-weight: 600;
}}
.api-code {{
  background: #141414; border: 1px solid #2A2A2A; border-radius: 4px;
  padding: 10px 14px; font-size: 11px; color: #A0A0A0;
  position: relative; cursor: pointer;
}}
.api-code:hover {{ border-color: #444; }}
.api-code::after {{
  content: 'click to copy'; position: absolute; right: 8px; top: 8px;
  font-size: 9px; color: #444;
}}
.api-table {{
  width: 100%; border-collapse: collapse;
}}
.api-table th {{
  text-align: left; font-size: 10px; color: #444; padding: 4px 8px;
  border-bottom: 1px solid #2A2A2A; font-weight: 500;
}}
.api-table td {{
  padding: 4px 8px; color: #A0A0A0; border-bottom: 1px solid #1F1F1F;
}}
.cap-grid {{ display: flex; gap: 6px; flex-wrap: wrap; }}
.cap-badge {{
  padding: 3px 10px; border-radius: 3px; font-size: 10px;
  border: 1px solid #2A2A2A;
}}
.cap-on {{ color: #22C55E; border-color: #22C55E33; background: #22C55E0A; }}
.cap-off {{ color: #444; }}

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

/* ── Parameter monitor ───────────────────────────── */
.param-monitor {{
  padding: 10px 14px; border-top: 1px solid #2A2A2A; font-size: 11px;
  max-height: 200px; overflow-y: auto;
}}
.param-title {{
  font-size: 10px; color: #444; text-transform: uppercase;
  letter-spacing: 1px; margin-bottom: 6px;
}}
.param-row {{
  display: flex; align-items: center; gap: 8px; margin-bottom: 5px;
  height: 22px;
}}
.param-dot {{
  width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0;
}}
.param-dot.data {{ background: #D4AF37; }}
.param-dot.modulated {{ background: #22D3EE; }}
.param-dot.arc {{ background: #F59E0B; }}
.param-dot.static {{ background: #666; }}
.param-name {{ width: 80px; color: #666; overflow: hidden; text-overflow: ellipsis; }}
.param-slider {{ flex: 1; }}
.param-slider input[type="range"] {{
  width: 100%; -webkit-appearance: none; height: 3px;
  background: #333; border-radius: 2px; outline: none;
}}
.param-slider input[type="range"]::-webkit-slider-thumb {{
  -webkit-appearance: none; width: 10px; height: 10px;
  border-radius: 50%; background: #D4AF37; cursor: pointer;
}}
.param-expr {{ font-size: 9px; color: #22D3EE; overflow: hidden; text-overflow: ellipsis; max-width: 120px; }}
.param-badge {{ font-size: 9px; color: #F59E0B; background: #F59E0B15; padding: 1px 5px; border-radius: 2px; }}
.param-val {{ width: 40px; text-align: right; color: #888; font-size: 10px; }}

/* ── Timeline ────────────────────────────────────── */
.timeline {{
  padding: 8px 14px 10px; border-top: 1px solid #2A2A2A;
  user-select: none;
}}
.timeline-controls {{
  display: flex; align-items: center; gap: 10px; margin-bottom: 6px;
}}
.timeline-play {{
  background: #1F1F1F; border: 1px solid #2A2A2A; color: #A0A0A0;
  width: 24px; height: 24px; border-radius: 3px; cursor: pointer;
  font-size: 10px; display: flex; align-items: center; justify-content: center;
  font-family: inherit;
}}
.timeline-play:hover {{ color: #FFF; border-color: #444; }}
.timeline-rail {{
  flex: 1; height: 20px; position: relative; cursor: pointer;
}}
.timeline-track {{
  position: absolute; top: 9px; left: 0; right: 0; height: 2px;
  background: #2A2A2A; border-radius: 1px;
}}
.timeline-fill {{
  position: absolute; top: 9px; left: 0; height: 2px;
  background: #D4AF37; border-radius: 1px;
}}
.timeline-head {{
  position: absolute; top: 5px; width: 10px; height: 10px;
  background: #D4AF37; border-radius: 50%; margin-left: -5px;
  cursor: grab;
}}
.timeline-head:active {{ cursor: grabbing; }}
.timeline-moment {{
  position: absolute; top: 3px; width: 14px; height: 14px;
  border-radius: 50%; background: #1F1F1F; border: 1.5px solid #666;
  margin-left: -7px; cursor: pointer; z-index: 2;
}}
.timeline-moment:hover {{ border-color: #D4AF37; }}
.timeline-moment-label {{
  position: absolute; top: 18px; left: 50%; transform: translateX(-50%);
  font-size: 9px; color: #444; white-space: nowrap;
}}
.timeline-time {{ font-size: 10px; color: #666; min-width: 60px; text-align: right; }}
.timeline-moments {{
  display: flex; gap: 4px; flex-wrap: wrap; margin-top: 2px;
}}
.timeline-moment-btn {{
  background: #1F1F1F; border: 1px solid #2A2A2A; color: #666;
  padding: 2px 8px; border-radius: 3px; font-size: 9px; cursor: pointer;
  font-family: inherit;
}}
.timeline-moment-btn:hover {{ color: #D4AF37; border-color: #D4AF37; }}

/* ── Shortcut overlay ────────────────────────────── */
.shortcut-overlay {{
  display: none; position: fixed; inset: 0; background: rgba(0,0,0,0.8);
  z-index: 1000; align-items: center; justify-content: center;
}}
.shortcut-overlay.open {{ display: flex; }}
.shortcut-box {{
  background: #141414; border: 1px solid #2A2A2A; border-radius: 8px;
  padding: 24px; min-width: 280px;
}}
.shortcut-box h3 {{ color: #D4AF37; font-size: 12px; margin-bottom: 12px; }}
.shortcut-row {{ display: flex; justify-content: space-between; padding: 4px 0; font-size: 11px; }}
.shortcut-key {{ color: #D4AF37; font-size: 10px; background: #1F1F1F; padding: 2px 6px; border-radius: 3px; }}
.shortcut-desc {{ color: #666; }}"#,
        tag_name = tag_name,
    )
}

// ── Toolbar ───────────────────────────────────────────────────────────

fn build_toolbar(output: &CompileOutput, tag_name: &str, has_timeline: bool) -> String {
    // Badges
    let mut badges = Vec::new();
    badges.push(r#"<span class="label">GAME dev</span>"#.to_string());
    badges.push(format!(
        r#"<span class="sep">|</span><span class="tag">&lt;{}&gt;</span>"#,
        html_escape(tag_name)
    ));
    if !output.params.is_empty() {
        badges.push(format!(
            r#"<span class="badge">{} param{}</span>"#,
            output.params.len(),
            if output.params.len() == 1 { "" } else { "s" }
        ));
    }
    if !output.data_fields.is_empty() {
        badges.push(format!(
            r#"<span class="badge">{} data</span>"#,
            output.data_fields.len()
        ));
    }
    if output.uses_audio {
        badges.push(r#"<span class="badge">audio</span>"#.to_string());
    }
    let mode = match &output.render_mode {
        crate::codegen::RenderMode::Flat => "flat",
        crate::codegen::RenderMode::Raymarch { .. } => "raymarch",
    };
    badges.push(format!(r#"<span class="badge">{mode}</span>"#));
    if !output.warnings.is_empty() {
        badges.push(format!(
            r#"<span class="badge" style="color:#F59E0B">&#9888; {}</span>"#,
            output.warnings.len()
        ));
    }

    // Tab bar
    let tab_bar = r#"<div class="tab-bar">
  <button class="tab-btn active" data-tab="preview">Preview</button>
  <button class="tab-btn" data-tab="wgsl">WGSL</button>
  <button class="tab-btn" data-tab="api">API</button>
</div>"#;

    // Actions
    let pause_btn = if has_timeline {
        r#"<button id="pause-btn" onclick="togglePause()" title="Pause timeline">&#9646;&#9646;</button>"#
    } else {
        ""
    };
    let actions = format!(
        r#"<div class="actions">
  <button onclick="copyText('&lt;script type=&quot;module&quot; src=&quot;./component.js&quot;&gt;&lt;/script&gt;', this)" title="Copy import tag">Copy Import</button>
  <button onclick="copyText('&lt;{tag}&gt;&lt;/{tag}&gt;', this)" title="Copy HTML tag">Copy HTML</button>
  <a href="/preview.html" target="_blank" title="Open standalone HTML">Fullscreen</a>
  {pause_btn}
  <button onclick="toggleShortcuts()" title="Keyboard shortcuts">?</button>
</div>"#,
        tag = html_escape(tag_name),
        pause_btn = pause_btn,
    );

    format!(
        r#"<div class="toolbar">
  {badges}
  {tab_bar}
  {actions}
</div>"#,
        badges = badges.join("\n  "),
        tab_bar = tab_bar,
        actions = actions,
    )
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

// ── WGSL viewer ───────────────────────────────────────────────────────

fn build_wgsl_viewer(output: &CompileOutput) -> String {
    let escaped = html_escape(&output.wgsl);
    let line_count = output.wgsl.lines().count();
    let lines_html: String = escaped
        .lines()
        .map(|l| format!(r#"<span class="line">{}</span>"#, if l.is_empty() { " " } else { l }))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<div class="wgsl-viewer">
  <div class="wgsl-header">
    <span>Generated WGSL — {line_count} lines — {uniforms} uniforms</span>
    <button onclick="copyWgsl()">Copy</button>
  </div>
  <pre class="wgsl-code" id="wgsl-code">{lines_html}</pre>
</div>"#,
        line_count = line_count,
        uniforms = output.uniform_float_count,
        lines_html = lines_html,
    )
}

// ── API panel ─────────────────────────────────────────────────────────

fn build_api_panel(output: &CompileOutput, tag_name: &str) -> String {
    let mut html = String::from(r#"<div class="api-panel">"#);

    // Import section
    html.push_str(r#"<div class="api-section"><h3>Import</h3>"#);
    html.push_str(r#"<div class="api-code" onclick="copyInner(this)">&lt;script type="module" src="./component.js"&gt;&lt;/script&gt;</div></div>"#);

    // Usage section
    html.push_str(r#"<div class="api-section"><h3>Usage</h3>"#);
    if output.data_fields.is_empty() {
        html.push_str(&format!(
            r#"<div class="api-code" onclick="copyInner(this)">&lt;{tag}&gt;&lt;/{tag}&gt;</div></div>"#,
            tag = html_escape(tag_name),
        ));
    } else {
        let attrs: String = output
            .data_fields
            .iter()
            .map(|f| format!(r#" data-{f}="0.5""#))
            .collect();
        html.push_str(&format!(
            r#"<div class="api-code" onclick="copyInner(this)">&lt;{tag}{attrs}&gt;&lt;/{tag}&gt;</div></div>"#,
            tag = html_escape(tag_name),
            attrs = html_escape(&attrs),
        ));
    }

    // Data attributes section
    if !output.data_fields.is_empty() {
        html.push_str(r#"<div class="api-section"><h3>Data Attributes</h3><table class="api-table"><tr><th>Name</th><th>Type</th><th>Setter</th></tr>"#);
        for f in &output.data_fields {
            html.push_str(&format!(
                r#"<tr><td>data-{f}</td><td>number (0-1)</td><td>el.{f} = value</td></tr>"#,
            ));
        }
        html.push_str("</table></div>");
    }

    // Capabilities section
    html.push_str(r#"<div class="api-section"><h3>Capabilities</h3><div class="cap-grid">"#);
    let caps = [
        ("audio", output.uses_audio),
        ("mouse", output.uses_mouse),
        ("data", output.uses_data),
    ];
    for (name, on) in &caps {
        let cls = if *on { "cap-badge cap-on" } else { "cap-badge cap-off" };
        html.push_str(&format!(r#"<span class="{cls}">{name}</span>"#));
    }
    let mode_label = match &output.render_mode {
        crate::codegen::RenderMode::Flat => "flat".to_string(),
        crate::codegen::RenderMode::Raymarch { .. } => "raymarch".to_string(),
    };
    html.push_str(&format!(
        r#"<span class="cap-badge cap-on">{mode_label}</span>"#
    ));
    html.push_str("</div></div>");

    // Parameters section
    if !output.params.is_empty() {
        html.push_str(r#"<div class="api-section"><h3>Parameters</h3><table class="api-table"><tr><th>Name</th><th>Base</th><th>Modulation</th></tr>"#);
        for p in &output.params {
            let mod_str = match &p.mod_js {
                Some(js) => html_escape(js),
                None => "—".to_string(),
            };
            html.push_str(&format!(
                r#"<tr><td>{name}</td><td>{base:.2}</td><td>{mod_str}</td></tr>"#,
                name = p.name,
                base = p.base_value,
                mod_str = mod_str,
            ));
        }
        html.push_str("</table></div>");
    }

    // Timeline section
    if !output.arc_moments.is_empty() {
        let duration = calc_timeline_duration(&output.arc_moments);
        html.push_str(&format!(
            r#"<div class="api-section"><h3>Timeline</h3><p style="color:#A0A0A0">{} moment{}, total duration: {}</p></div>"#,
            output.arc_moments.len(),
            if output.arc_moments.len() == 1 { "" } else { "s" },
            format_time(duration),
        ));
    }

    html.push_str("</div>");
    html
}

// ── Parameter monitor ─────────────────────────────────────────────────

fn build_param_monitor(output: &CompileOutput) -> String {
    if output.params.is_empty() && output.data_fields.is_empty() {
        return String::new();
    }
    let mut html = String::from(r#"<div class="param-monitor">"#);

    // Data signal sliders (web component observed attributes — the PRIMARY interactive control)
    if !output.data_fields.is_empty() {
        html.push_str(&format!(
            r#"<div class="param-title">Data Signals ({})</div>"#,
            output.data_fields.len()
        ));
        for field in &output.data_fields {
            html.push_str(&format!(
                r#"<div class="param-row"><span class="param-dot data"></span><span class="param-name" title="{f}">{f}</span><div class="param-slider"><input type="range" min="0" max="1" step="0.01" value="0.5" data-field="{f}"></div><span class="param-val">0.50</span></div>"#,
                f = field,
            ));
        }
    }

    // Parameter list
    if !output.params.is_empty() {
        html.push_str(&format!(
            r#"<div class="param-title" style="margin-top:8px">Parameters ({})</div>"#,
            output.params.len()
        ));
    }

    for (i, param) in output.params.iter().enumerate() {
        let kind = classify_param(param, &output.data_fields, &output.arc_moments, i);
        let (dot_cls, row_content) = match kind {
            ParamKind::Data => {
                let slider = format!(
                    r#"<div class="param-slider"><input type="range" min="0" max="1" step="0.01" value="0.5" data-field="{}" data-index="{}"></div><span class="param-val" data-live="{}">0.50</span>"#,
                    param.name, i, i,
                );
                ("data", slider)
            }
            ParamKind::Modulated => {
                let expr = param.mod_js.as_deref().unwrap_or("");
                (
                    "modulated",
                    format!(
                        r#"<span class="param-expr" title="{}">{}</span><span class="param-val" data-live="{}">—</span>"#,
                        html_escape(expr),
                        html_escape(expr),
                        i,
                    ),
                )
            }
            ParamKind::Arc => (
                "arc",
                format!(
                    r#"<span class="param-badge">arc</span><span class="param-val" data-live="{}">—</span>"#,
                    i,
                ),
            ),
            ParamKind::Static => (
                "static",
                format!(
                    r#"<span class="param-val">{:.2}</span>"#,
                    param.base_value
                ),
            ),
        };

        html.push_str(&format!(
            r#"<div class="param-row"><span class="param-dot {dot_cls}"></span><span class="param-name" title="{name}">{name}</span>{row_content}</div>"#,
            dot_cls = dot_cls,
            name = param.name,
            row_content = row_content,
        ));
    }
    html.push_str("</div>");
    html
}

// ── Timeline ──────────────────────────────────────────────────────────

fn build_timeline(output: &CompileOutput) -> String {
    if output.arc_moments.is_empty() {
        return String::new();
    }
    let duration = calc_timeline_duration(&output.arc_moments);
    let dur_fmt = format_time(duration);

    // Moment dots on the rail
    let mut moment_dots = String::new();
    let mut moment_btns = String::new();
    for m in &output.arc_moments {
        let pct = if duration > 0.0 {
            (m.time_seconds / duration) * 100.0
        } else {
            0.0
        };
        let time_str = format_time(m.time_seconds);
        let label = m.name.as_deref().unwrap_or(&time_str);
        moment_dots.push_str(&format!(
            r#"<div class="timeline-moment" style="left:{pct:.1}%" data-time="{time}" title="{label}"><span class="timeline-moment-label">{label}</span></div>"#,
            pct = pct,
            time = m.time_seconds,
            label = html_escape(label),
        ));
        let btn_fallback = format!("@{}", time_str);
        let btn_label = m.name.as_deref().unwrap_or(&btn_fallback);
        moment_btns.push_str(&format!(
            r#"<button class="timeline-moment-btn" data-time="{time}">{label}</button>"#,
            time = m.time_seconds,
            label = html_escape(btn_label),
        ));
    }

    format!(
        r##"<div class="timeline" id="timeline" data-duration="{duration}">
  <div class="timeline-controls">
    <button class="timeline-play" id="tl-play" title="Play/Pause">&#9654;</button>
    <div class="timeline-rail" id="tl-rail">
      <div class="timeline-track"></div>
      <div class="timeline-fill" id="tl-fill"></div>
      <div class="timeline-head" id="tl-head"></div>
      {moment_dots}
    </div>
    <span class="timeline-time" id="tl-time">0:00 / {dur_fmt}</span>
  </div>
  <div class="timeline-moments">{moment_btns}</div>
</div>"##,
        duration = duration,
        moment_dots = moment_dots,
        dur_fmt = dur_fmt,
        moment_btns = moment_btns,
    )
}

// ── Shortcut overlay ──────────────────────────────────────────────────

fn build_shortcut_overlay(has_timeline: bool) -> String {
    let mut rows = String::new();
    rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">1</span><span class="shortcut-desc">Preview tab</span></div>"#);
    rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">2</span><span class="shortcut-desc">WGSL tab</span></div>"#);
    rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">3</span><span class="shortcut-desc">API tab</span></div>"#);
    rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">S / M / L</span><span class="shortcut-desc">Component size</span></div>"#);
    if has_timeline {
        rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">Space</span><span class="shortcut-desc">Play / Pause</span></div>"#);
        rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">&larr; / &rarr;</span><span class="shortcut-desc">Step -1s / +1s</span></div>"#);
        rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">Home</span><span class="shortcut-desc">Go to start</span></div>"#);
    }
    rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">?</span><span class="shortcut-desc">Toggle this overlay</span></div>"#);

    format!(
        r#"<div class="shortcut-overlay" id="shortcut-overlay" onclick="toggleShortcuts()">
  <div class="shortcut-box" onclick="event.stopPropagation()">
    <h3>Keyboard Shortcuts</h3>
    {rows}
  </div>
</div>"#,
        rows = rows,
    )
}

// ── Inline JS ─────────────────────────────────────────────────────────

fn build_inline_js(output: &CompileOutput, html_json: &str, has_timeline: bool, duration: f64) -> String {
    let wgsl_raw_json = serde_json_inline(&output.wgsl);
    let dur_fmt = format_time(duration);

    let timeline_js = if has_timeline {
        format!(r#"
  // Timeline
  var tlPlaying = true;
  var tlDuration = {duration};
  var tlPausedAt = 0;
  var tlDragActive = false;
  var tlPlay = document.getElementById('tl-play');
  var tlFill = document.getElementById('tl-fill');
  var tlHead = document.getElementById('tl-head');
  var tlRail = document.getElementById('tl-rail');
  var tlTime = document.getElementById('tl-time');
  var pauseBtn = document.getElementById('pause-btn');
  var comp = document.getElementById('comp');

  function fmtTime(s) {{
    var m = Math.floor(s / 60);
    var sec = Math.floor(s % 60);
    return m + ':' + (sec < 10 ? '0' : '') + sec;
  }}

  function getTlTime() {{
    if (!comp) return 0;
    var now = performance.now() / 1000;
    var start = comp._startTime || now;
    return now - start;
  }}

  function setTlTime(t) {{
    if (!comp) return;
    var now = performance.now() / 1000;
    comp._startTime = now - t;
  }}

  function updateTlUI(t) {{
    var frac = tlDuration > 0 ? Math.min(t / tlDuration, 1) : 0;
    tlFill.style.width = (frac * 100) + '%';
    tlHead.style.left = (frac * 100) + '%';
    tlTime.textContent = fmtTime(t) + ' / {dur_fmt}';
  }}

  function togglePause() {{
    if (tlPlaying) {{
      tlPausedAt = getTlTime();
      tlPlaying = false;
      if (comp) comp._paused = true;
      tlPlay.innerHTML = '&#9654;';
      if (pauseBtn) pauseBtn.innerHTML = '&#9654;';
    }} else {{
      setTlTime(tlPausedAt);
      tlPlaying = true;
      if (comp) comp._paused = false;
      tlPlay.innerHTML = '&#9646;&#9646;';
      if (pauseBtn) pauseBtn.innerHTML = '&#9646;&#9646;';
    }}
  }}

  function seekTo(t) {{
    t = Math.max(0, Math.min(t, tlDuration));
    if (tlPlaying) {{
      setTlTime(t);
    }} else {{
      tlPausedAt = t;
      setTlTime(t);
    }}
    updateTlUI(t);
  }}

  tlPlay.addEventListener('click', togglePause);

  // Rail click
  tlRail.addEventListener('mousedown', function(e) {{
    if (e.target.classList.contains('timeline-moment')) return;
    if (e.target.classList.contains('timeline-head')) {{ tlDragActive = true; return; }}
    var rect = tlRail.getBoundingClientRect();
    var frac = (e.clientX - rect.left) / rect.width;
    seekTo(frac * tlDuration);
  }});

  // Head drag
  tlHead.addEventListener('mousedown', function(e) {{
    tlDragActive = true;
    e.preventDefault();
  }});
  document.addEventListener('mousemove', function(e) {{
    if (!tlDragActive) return;
    var rect = tlRail.getBoundingClientRect();
    var frac = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    seekTo(frac * tlDuration);
  }});
  document.addEventListener('mouseup', function() {{ tlDragActive = false; }});

  // Moment buttons
  document.querySelectorAll('.timeline-moment, .timeline-moment-btn').forEach(function(el) {{
    el.addEventListener('click', function() {{
      var t = parseFloat(el.dataset.time);
      if (!isNaN(t)) seekTo(t);
    }});
  }});"#,
            duration = duration,
            dur_fmt = dur_fmt,
        )
    } else {
        String::new()
    };

    let timeline_raf = if has_timeline {
        r#"
    if (typeof tlPlaying !== 'undefined' && tlPlaying && typeof getTlTime === 'function') {
      var t = getTlTime();
      updateTlUI(t);
    }"#
    } else {
        ""
    };

    let timeline_keys = if has_timeline {
        r#"
      case ' ': e.preventDefault(); togglePause(); break;
      case 'ArrowLeft': seekTo((tlPlaying ? getTlTime() : tlPausedAt) - 1); break;
      case 'ArrowRight': seekTo((tlPlaying ? getTlTime() : tlPausedAt) + 1); break;
      case 'Home': seekTo(0); break;"#
    } else {
        ""
    };

    format!(r##"<script type="module" src="/component.js"></script>
<script>
  // Inject HTML preview
  var html = {html_json};
  document.getElementById('preview').srcdoc = html;

  // Tab switching
  var wgslHighlighted = false;
  document.querySelectorAll('.tab-btn').forEach(function(btn) {{
    btn.addEventListener('click', function() {{
      var tab = btn.dataset.tab;
      document.querySelectorAll('.tab-btn').forEach(function(b) {{ b.classList.remove('active'); }});
      btn.classList.add('active');
      document.querySelectorAll('.tab-pane').forEach(function(p) {{ p.classList.remove('active'); }});
      var pane = document.getElementById('pane-' + tab);
      if (pane) pane.classList.add('active');
      if (tab === 'wgsl' && !wgslHighlighted) {{ highlightWgsl(); wgslHighlighted = true; }}
    }});
  }});

  // WGSL syntax highlighting
  function highlightWgsl() {{
    var el = document.getElementById('wgsl-code');
    if (!el) return;
    el.querySelectorAll('.line').forEach(function(line) {{
      var t = line.innerHTML;
      // Comments first (greedy per line)
      t = t.replace(/(\/\/.*)$/gm, '<span class="wgsl-comment">$1</span>');
      // Decorators
      t = t.replace(/@(\w+)/g, '<span class="wgsl-deco">@$1</span>');
      // Keywords
      t = t.replace(/\b(fn|var|let|return|struct|if|for|else|loop|break|continue|switch|case|default|while)\b/g, '<span class="wgsl-kw">$1</span>');
      // Types
      t = t.replace(/\b(f32|vec2f|vec4f|vec3f|u32|i32|bool|array|mat4x4f|mat3x3f|ptr)\b/g, '<span class="wgsl-type">$1</span>');
      // Numbers
      t = t.replace(/\b(\d+\.?\d*)\b/g, '<span class="wgsl-num">$1</span>');
      line.innerHTML = t;
    }});
  }}

  // Copy WGSL
  function copyWgsl() {{
    var raw = {wgsl_raw_json};
    navigator.clipboard.writeText(raw);
  }}

  // API panel code block copy
  function copyInner(el) {{
    var ta = document.createElement('textarea');
    ta.innerHTML = el.textContent;
    navigator.clipboard.writeText(ta.value);
  }}

  // Size toggle
  function setSize(size, btn) {{
    var wrapper = document.getElementById('comp-wrapper');
    wrapper.className = 'comp-wrapper size-' + size;
    document.querySelectorAll('.size-bar button').forEach(function(b) {{ b.classList.remove('active'); }});
    btn.classList.add('active');
  }}

  // Copy helper
  function copyText(text, btn) {{
    var ta = document.createElement('textarea');
    ta.innerHTML = text;
    navigator.clipboard.writeText(ta.value).then(function() {{
      btn.classList.add('copied');
      var orig = btn.textContent;
      btn.textContent = 'Copied!';
      setTimeout(function() {{ btn.classList.remove('copied'); btn.textContent = orig; }}, 1200);
    }});
  }}

  // Slider wiring (data params only)
  document.querySelectorAll('.param-slider input[type="range"]').forEach(function(slider) {{
    var field = slider.dataset.field;
    var valSpan = slider.closest('.param-row').querySelector('.param-val');
    var comp = document.getElementById('comp');
    if (comp && field) comp[field] = parseFloat(slider.value);
    slider.addEventListener('input', function() {{
      var v = parseFloat(slider.value);
      if (valSpan) valSpan.textContent = v.toFixed(2);
      if (comp && field) comp[field] = v;
    }});
  }});

  // Divider drag-resize
  var divider = document.getElementById('divider');
  var split = divider.parentElement;
  var dragging = false;
  divider.addEventListener('mousedown', function(e) {{
    dragging = true;
    e.preventDefault();
  }});
  document.addEventListener('mousemove', function(e) {{
    if (!dragging) return;
    var rect = split.getBoundingClientRect();
    var frac = (e.clientX - rect.left) / rect.width;
    var clamped = Math.max(0.2, Math.min(0.8, frac));
    split.style.gridTemplateColumns = clamped + 'fr 4px ' + (1 - clamped) + 'fr';
  }});
  document.addEventListener('mouseup', function() {{ dragging = false; }});

  {timeline_js}

  // Param monitor RAF
  (function() {{
    var comp = document.getElementById('comp');
    function tick() {{
      if (comp && comp._paramValues) {{
        document.querySelectorAll('[data-live]').forEach(function(el) {{
          var idx = parseInt(el.dataset.live);
          var v = comp._paramValues[idx];
          if (v !== undefined) el.textContent = v.toFixed(2);
        }});
      }}
      {timeline_raf}
      requestAnimationFrame(tick);
    }}
    requestAnimationFrame(tick);
  }})();

  // Shortcuts overlay
  function toggleShortcuts() {{
    var ov = document.getElementById('shortcut-overlay');
    ov.classList.toggle('open');
  }}

  // Keyboard shortcuts
  document.addEventListener('keydown', function(e) {{
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
    switch(e.key) {{
      case '1': document.querySelector('[data-tab="preview"]').click(); break;
      case '2': document.querySelector('[data-tab="wgsl"]').click(); break;
      case '3': document.querySelector('[data-tab="api"]').click(); break;
      case 's': case 'S': document.querySelector('.size-bar button:nth-child(1)').click(); break;
      case 'm': case 'M': document.querySelector('.size-bar button:nth-child(2)').click(); break;
      case 'l': case 'L': document.querySelector('.size-bar button:nth-child(3)').click(); break;
      case '?': toggleShortcuts(); break;
      {timeline_keys}
    }}
  }});
</script>"##,
        html_json = html_json,
        wgsl_raw_json = wgsl_raw_json,
        timeline_js = timeline_js,
        timeline_raf = timeline_raf,
        timeline_keys = timeline_keys,
    )
}

// ── Success page assembly ─────────────────────────────────────────────

fn build_preview_page(output: &CompileOutput, tag_name: &str) -> String {
    let has_timeline = !output.arc_moments.is_empty();
    let duration = if has_timeline {
        calc_timeline_duration(&output.arc_moments)
    } else {
        0.0
    };

    let css = build_css(tag_name);
    let toolbar = build_toolbar(output, tag_name, has_timeline);
    let warnings_html = build_warnings_html(&output.warnings);
    let wgsl_viewer = build_wgsl_viewer(output);
    let api_panel = build_api_panel(output, tag_name);
    let param_monitor = build_param_monitor(output);
    let timeline_html = build_timeline(output);
    let shortcut_overlay = build_shortcut_overlay(has_timeline);

    let html_full = crate::runtime::wrap_html_full(output);
    let html_json = serde_json_inline(&html_full);
    let inline_js = build_inline_js(output, &html_json, has_timeline, duration);

    // Calculate top offset
    let has_warnings = !output.warnings.is_empty();
    let tl_height = if has_timeline { 70 } else { 0 };
    let top_offset = if has_warnings {
        format!("{}px", 36 + 28 + tl_height)
    } else {
        format!("{}px", 36 + tl_height)
    };

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
{toolbar}
{warnings_html}
<div class="split">
  <div class="panel">
    <div id="pane-preview" class="tab-pane active" style="height:100%">
      <div class="panel-label">html preview</div>
      <iframe id="preview" srcdoc=""></iframe>
    </div>
    <div id="pane-wgsl" class="tab-pane" style="height:100%">
      {wgsl_viewer}
    </div>
    <div id="pane-api" class="tab-pane" style="height:100%">
      {api_panel}
    </div>
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
    {param_monitor}
  </div>
</div>
{timeline_html}
{shortcut_overlay}
{inline_js}
</body>
</html>"##,
        tag_name = tag_name,
        top_offset = top_offset,
        css = css,
        toolbar = toolbar,
        warnings_html = warnings_html,
        wgsl_viewer = wgsl_viewer,
        api_panel = api_panel,
        param_monitor = param_monitor,
        timeline_html = timeline_html,
        shortcut_overlay = shortcut_overlay,
        inline_js = inline_js,
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
