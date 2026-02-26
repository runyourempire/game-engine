use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::response::Html;
use axum::routing::{get, post};
use axum::Router;
use notify::{Event, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use tower_livereload::LiveReloadLayer;

use crate::codegen::CompileOutput;
use crate::codegen::XrayVariant;

mod css;
mod toolbar;
mod panels;
mod timeline;
mod inline_js;
mod page;
mod export;
pub(crate) mod util;

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
        .route("/xray.json", get(serve_xray))
        .route("/compile", post(serve_compile))
        .route("/save", post(serve_save))
        .route("/export/react", get(serve_export_react))
        .route("/export/vue", get(serve_export_vue))
        .route("/export/css", get(serve_export_css))
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

fn read_source(state: &Arc<Mutex<DevState>>) -> Option<String> {
    let source_path = state.lock().ok()?.source_path.clone();
    std::fs::read_to_string(&source_path).ok()
}

// ── Route handlers ────────────────────────────────────────────────────

/// Serve the full dev UI (split-pane, sliders, toolbar).
async fn serve_preview(State(state): State<Arc<Mutex<DevState>>>) -> Html<String> {
    let (tag_name, result) = compile_source(&state);
    let source = read_source(&state).unwrap_or_default();
    match result {
        CompileResult::Ok(output) => Html(page::build_preview_page(&output, &tag_name, &source)),
        CompileResult::Err(e) => Html(page::build_error_page(&tag_name, &e)),
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
        CompileResult::Err(e) => Html(page::build_error_page(&tag_name, &e)),
    }
}

/// Serve x-ray variants as JSON for pipeline swapping.
async fn serve_xray(
    State(state): State<Arc<Mutex<DevState>>>,
) -> ([(axum::http::header::HeaderName, &'static str); 1], String) {
    let source_path = match state.lock() {
        Ok(s) => s.source_path.clone(),
        Err(_) => {
            return (
                [(axum::http::header::CONTENT_TYPE, "application/json")],
                r#"{"variants":[]}"#.to_string(),
            );
        }
    };
    let source = match std::fs::read_to_string(&source_path) {
        Ok(s) => s,
        Err(_) => {
            return (
                [(axum::http::header::CONTENT_TYPE, "application/json")],
                r#"{"variants":[]}"#.to_string(),
            );
        }
    };
    let variants = match crate::compile_xray_variants(&source) {
        Ok(v) => v,
        Err(_) => {
            return (
                [(axum::http::header::CONTENT_TYPE, "application/json")],
                r#"{"variants":[]}"#.to_string(),
            );
        }
    };
    let json = xray_variants_to_json(&variants);
    ([(axum::http::header::CONTENT_TYPE, "application/json")], json)
}

// ── POST /compile — live editor recompilation ─────────────────────────

#[derive(Deserialize)]
struct CompileRequest {
    source: String,
}

#[derive(Serialize)]
struct CompileResponse {
    wgsl: Option<String>,
    component_js: Option<String>,
    params: Vec<CompileParamInfo>,
    warnings: Vec<String>,
    error: Option<String>,
    uniform_count: usize,
}

#[derive(Serialize)]
struct CompileParamInfo {
    name: String,
    base: f64,
}

async fn serve_compile(
    State(state): State<Arc<Mutex<DevState>>>,
    axum::Json(req): axum::Json<CompileRequest>,
) -> axum::Json<CompileResponse> {
    let tag_name = state
        .lock()
        .map(|s| s.tag_name.clone())
        .unwrap_or_else(|_| "game-component".to_string());

    match crate::compile_full(&req.source) {
        Ok(output) => {
            let component_js = crate::runtime::wrap_web_component(&output, &tag_name);
            axum::Json(CompileResponse {
                wgsl: Some(output.wgsl),
                component_js: Some(component_js),
                params: output
                    .params
                    .iter()
                    .map(|p| CompileParamInfo {
                        name: p.name.clone(),
                        base: p.base_value,
                    })
                    .collect(),
                warnings: output.warnings,
                error: None,
                uniform_count: output.uniform_float_count,
            })
        }
        Err(e) => axum::Json(CompileResponse {
            wgsl: None,
            component_js: None,
            params: Vec::new(),
            warnings: Vec::new(),
            error: Some(format!("{e}")),
            uniform_count: 0,
        }),
    }
}

// ── POST /save — write source to disk ─────────────────────────────────

#[derive(Deserialize)]
struct SaveRequest {
    source: String,
}

#[derive(Serialize)]
struct SaveResponse {
    ok: bool,
    error: Option<String>,
}

async fn serve_save(
    State(state): State<Arc<Mutex<DevState>>>,
    axum::Json(req): axum::Json<SaveRequest>,
) -> axum::Json<SaveResponse> {
    let source_path = match state.lock() {
        Ok(s) => s.source_path.clone(),
        Err(e) => {
            return axum::Json(SaveResponse {
                ok: false,
                error: Some(format!("Lock error: {e}")),
            });
        }
    };
    match std::fs::write(&source_path, &req.source) {
        Ok(()) => axum::Json(SaveResponse {
            ok: true,
            error: None,
        }),
        Err(e) => axum::Json(SaveResponse {
            ok: false,
            error: Some(format!("Write error: {e}")),
        }),
    }
}

// ── Export routes ─────────────────────────────────────────────────────

async fn serve_export_react(
    State(state): State<Arc<Mutex<DevState>>>,
) -> ([(axum::http::header::HeaderName, &'static str); 2], String) {
    let (tag_name, result) = compile_source(&state);
    let body = match result {
        CompileResult::Ok(output) => export::generate_react(&output, &tag_name),
        CompileResult::Err(e) => format!("// Compile error: {e}"),
    };
    ([
        (axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8"),
        (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"GameComponent.jsx\""),
    ], body)
}

async fn serve_export_vue(
    State(state): State<Arc<Mutex<DevState>>>,
) -> ([(axum::http::header::HeaderName, &'static str); 2], String) {
    let (tag_name, result) = compile_source(&state);
    let body = match result {
        CompileResult::Ok(output) => export::generate_vue(&output, &tag_name),
        CompileResult::Err(e) => format!("<!-- Compile error: {} -->", e),
    };
    ([
        (axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8"),
        (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"GameComponent.vue\""),
    ], body)
}

async fn serve_export_css(
    State(state): State<Arc<Mutex<DevState>>>,
) -> ([(axum::http::header::HeaderName, &'static str); 2], String) {
    let (tag_name, result) = compile_source(&state);
    let body = match result {
        CompileResult::Ok(output) => export::generate_css(&output, &tag_name),
        CompileResult::Err(e) => format!("/* Compile error: {} */", e),
    };
    ([
        (axum::http::header::CONTENT_TYPE, "text/css; charset=utf-8"),
        (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"fallback.css\""),
    ], body)
}

// ── Helpers ───────────────────────────────────────────────────────────

fn xray_variants_to_json(variants: &[XrayVariant]) -> String {
    let mut json = String::from(r#"{"variants":["#);
    for (i, v) in variants.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        json.push_str(&format!(
            r#"{{"layer":{},"layerName":{},"stage":{},"stageName":{},"wgsl":{}}}"#,
            v.layer_index,
            util::serde_json_inline(&v.layer_name),
            v.stage_index,
            util::serde_json_inline(&v.stage_name),
            util::serde_json_inline(&v.wgsl),
        ));
    }
    json.push_str("]}");
    json
}
