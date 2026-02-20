use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use notify::{Event, RecursiveMode, Watcher};
use tower_livereload::LiveReloadLayer;

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
        .layer(livereload)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    eprintln!("GAME dev server");
    eprintln!("  file:      {}", watch_path.display());
    eprintln!("  preview:   http://localhost:{port}/");
    eprintln!("  component: http://localhost:{port}/component.js");
    eprintln!("  watching for changes...");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    // Keep watcher alive
    drop(watcher);
    Ok(())
}

/// Serve the HTML preview page with split view.
async fn serve_preview(State(state): State<Arc<Mutex<DevState>>>) -> Html<String> {
    let (source_path, tag_name) = match state.lock() {
        Ok(s) => (s.source_path.clone(), s.tag_name.clone()),
        Err(e) => return Html(format!("<pre>Internal error: {e}</pre>")),
    };

    let source = match std::fs::read_to_string(&source_path) {
        Ok(s) => s,
        Err(e) => return Html(format!("<pre>Error reading file: {e}</pre>")),
    };

    // Compile HTML version
    let html_output = match crate::compile_html(&source) {
        Ok(h) => h,
        Err(e) => return Html(format!("<pre>Compilation error: {e}</pre>")),
    };

    // Generate preview page with embedded HTML and component side-by-side
    let preview = format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>GAME Dev — {tag_name}</title>
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  html, body {{ width: 100%; height: 100%; background: #0A0A0A; color: #A0A0A0; font-family: 'JetBrains Mono', monospace; }}
  .toolbar {{
    height: 32px;
    display: flex;
    align-items: center;
    padding: 0 12px;
    border-bottom: 1px solid #2A2A2A;
    font-size: 11px;
    gap: 16px;
  }}
  .toolbar span {{ color: #666; }}
  .toolbar .tag {{ color: #D4AF37; }}
  .split {{
    display: grid;
    grid-template-columns: 1fr 1fr;
    height: calc(100vh - 32px);
  }}
  .panel {{
    position: relative;
    overflow: hidden;
  }}
  .panel-label {{
    position: absolute;
    top: 8px;
    left: 12px;
    font-size: 10px;
    color: #444;
    z-index: 10;
    text-transform: uppercase;
    letter-spacing: 1px;
  }}
  .divider {{
    width: 1px;
    background: #2A2A2A;
  }}
  iframe {{
    width: 100%;
    height: 100%;
    border: none;
  }}
  .component-panel {{
    display: flex;
    flex-direction: column;
  }}
  .component-view {{
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }}
  {tag_name} {{
    width: 200px;
    height: 200px;
  }}
  .sliders {{
    padding: 12px;
    border-top: 1px solid #2A2A2A;
    font-size: 11px;
  }}
  .slider-row {{
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }}
  .slider-row label {{ width: 80px; color: #666; }}
  .slider-row input[type="range"] {{
    flex: 1;
    -webkit-appearance: none;
    height: 3px;
    background: #333;
    border-radius: 2px;
    outline: none;
  }}
  .slider-row input[type="range"]::-webkit-slider-thumb {{
    -webkit-appearance: none;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #D4AF37;
    cursor: pointer;
  }}
  .slider-row .val {{ width: 40px; text-align: right; color: #888; }}
</style>
</head>
<body>
<div class="toolbar">
  <span>GAME dev</span>
  <span class="tag">&lt;{tag_name}&gt;</span>
  <span>|</span>
  <span>left: HTML preview</span>
  <span>right: component embed</span>
</div>
<div class="split">
  <div class="panel">
    <div class="panel-label">html preview</div>
    <iframe id="preview" srcdoc=""></iframe>
  </div>
  <div class="panel component-panel">
    <div class="panel-label">component</div>
    <div class="component-view">
      <{tag_name} id="comp"></{tag_name}>
    </div>
    <div class="sliders" id="sliders"></div>
  </div>
</div>
<script type="module" src="/component.js"></script>
<script>
  // Inject HTML preview into iframe
  const html = {html_json};
  document.getElementById('preview').srcdoc = html;
</script>
</body>
</html>"##,
        tag_name = tag_name,
        html_json = serde_json_inline(&html_output),
    );

    Html(preview)
}

/// Serve the compiled Web Component JS module.
async fn serve_component(State(state): State<Arc<Mutex<DevState>>>) -> (
    [(axum::http::header::HeaderName, &'static str); 1],
    String,
) {
    let (source_path, tag_name) = match state.lock() {
        Ok(s) => (s.source_path.clone(), s.tag_name.clone()),
        Err(e) => return (
            [(axum::http::header::CONTENT_TYPE, "text/javascript")],
            format!("console.error('GAME: Internal error: {e}');"),
        ),
    };

    let source = match std::fs::read_to_string(&source_path) {
        Ok(s) => s,
        Err(e) => return (
            [(axum::http::header::CONTENT_TYPE, "text/javascript")],
            format!("console.error('GAME: Error reading file: {e}');"),
        ),
    };

    let js = match crate::compile_component(&source, &tag_name) {
        Ok(js) => js,
        Err(e) => format!("console.error('GAME: Compilation error: {e}');"),
    };

    ([(axum::http::header::CONTENT_TYPE, "text/javascript")], js)
}

/// Simple JSON string encoding (avoid serde dependency just for this).
fn serde_json_inline(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < '\x20' => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
