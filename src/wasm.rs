//! WebAssembly bindings for the GAME compiler.
//!
//! Enables browser-based compilation via wasm-bindgen.
//! Build with: `cargo build --target wasm32-unknown-unknown --features wasm`

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
use crate::{CompileConfig, OutputFormat, ShaderTarget};

/// Compile GAME source to a Web Component JS string.
///
/// # Arguments
/// * `source` - GAME DSL source code
/// * `target` - "webgpu", "webgl2", or "both"
///
/// # Returns
/// JSON string with `[{ name, js, wgsl?, glsl?, html? }]` or error message
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "compileGame")]
pub fn compile_game(source: &str, target: &str) -> Result<String, String> {
    let shader_target = match target {
        "webgpu" => ShaderTarget::WebGpu,
        "webgl2" => ShaderTarget::WebGl2,
        _ => ShaderTarget::Both,
    };

    let config = CompileConfig {
        output_format: OutputFormat::Component,
        target: shader_target,
        seed: None,
    };

    let results = crate::compile(source, &config).map_err(|e| e.to_string())?;

    // Re-parse to extract uniform metadata for the API consumer
    let program = crate::compile_to_ast(source).map_err(|e| e.to_string())?;

    let output: Vec<serde_json::Value> = results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let uniforms_json: Vec<serde_json::Value> = if i < program.cinematics.len() {
                let uniforms = crate::codegen::extract_uniforms_public(&program.cinematics[i]);
                uniforms
                    .iter()
                    .map(|u| {
                        serde_json::json!({
                            "name": u.name,
                            "default": u.default,
                        })
                    })
                    .collect()
            } else {
                vec![]
            };
            serde_json::json!({
                "name": r.name,
                "js": r.js,
                "wgsl": r.wgsl,
                "glsl": r.glsl,
                "html": r.html,
                "dts": r.dts,
                "uniforms": uniforms_json,
            })
        })
        .collect();

    serde_json::to_string(&output).map_err(|e| e.to_string())
}

/// Validate GAME source without generating output.
///
/// Returns "ok" or an error description.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "validateGame")]
pub fn validate_game(source: &str) -> String {
    match crate::compile_to_ast(source) {
        Ok(_) => "ok".to_string(),
        Err(e) => e.to_string(),
    }
}

/// Get all available builtin function signatures as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "getBuiltins")]
pub fn get_builtins() -> String {
    let items = crate::builtins::completions();
    let json: Vec<serde_json::Value> = items
        .iter()
        .map(|i| {
            serde_json::json!({
                "name": i.name,
                "signature": i.signature,
                "input": i.input,
                "output": i.output,
            })
        })
        .collect();
    serde_json::to_string(&json).unwrap_or_else(|_| "[]".to_string())
}

/// Get available named palette names as JSON array.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "getPaletteNames")]
pub fn get_palette_names() -> String {
    let names = vec![
        "fire",
        "ocean",
        "neon",
        "aurora",
        "sunset",
        "ice",
        "ember",
        "lava",
        "magma",
        "inferno",
        "plasma",
        "electric",
        "cyber",
        "matrix",
        "forest",
        "moss",
        "earth",
        "desert",
        "blood",
        "rose",
        "candy",
        "royal",
        "deep_sea",
        "coral",
        "arctic",
        "twilight",
        "vapor",
        "gold",
        "silver",
        "monochrome",
    ];
    serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string())
}
