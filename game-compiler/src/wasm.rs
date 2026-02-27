//! WASM bindings for the GAME compiler.
//!
//! Exposes the compiler's core functions to JavaScript via wasm-bindgen.
//! Build with: `wasm-pack build --target web --features wasm`

use wasm_bindgen::prelude::*;

/// Compile `.game` source to WGSL shader code.
///
/// Returns the WGSL string on success, or throws a JS error on failure.
#[wasm_bindgen]
pub fn compile_to_wgsl(source: &str) -> Result<String, JsError> {
    let tokens = crate::lexer::lex(source).map_err(|e| JsError::new(&e.to_string()))?;
    let mut parser = crate::parser::Parser::new(tokens);
    let cinematic = parser.parse().map_err(|e| JsError::new(&e.to_string()))?;
    crate::codegen::generate_wgsl(&cinematic).map_err(|e| JsError::new(&e.to_string()))
}

/// Compile `.game` source to a self-contained HTML file with WebGPU rendering.
///
/// Returns the HTML string on success, or throws a JS error on failure.
#[wasm_bindgen]
pub fn compile_to_html(source: &str) -> Result<String, JsError> {
    let tokens = crate::lexer::lex(source).map_err(|e| JsError::new(&e.to_string()))?;
    let mut parser = crate::parser::Parser::new(tokens);
    let cinematic = parser.parse().map_err(|e| JsError::new(&e.to_string()))?;
    let output =
        crate::codegen::generate_full(&cinematic).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(crate::runtime::wrap_html_full(&output))
}

/// Compile `.game` source to a Web Component ES module.
///
/// `tag_name` must be a valid custom element name (must contain a hyphen).
/// Returns the JavaScript module string on success, or throws a JS error on failure.
#[wasm_bindgen]
pub fn compile_to_component(source: &str, tag_name: &str) -> Result<String, JsError> {
    let tokens = crate::lexer::lex(source).map_err(|e| JsError::new(&e.to_string()))?;
    let mut parser = crate::parser::Parser::new(tokens);
    let cinematic = parser.parse().map_err(|e| JsError::new(&e.to_string()))?;
    let output =
        crate::codegen::generate_full(&cinematic).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(crate::runtime::wrap_web_component(&output, tag_name))
}

/// Validate `.game` source without full compilation.
///
/// Returns a JSON object with:
/// - `valid`: boolean
/// - `error`: string (only if invalid)
/// - `warnings`: string[] (only if valid)
/// - `layers`: number (only if valid)
/// - `params`: string[] (only if valid)
/// - `uses_audio`: boolean (only if valid)
/// - `uses_mouse`: boolean (only if valid)
#[wasm_bindgen]
pub fn validate(source: &str) -> JsValue {
    let result = (|| -> Result<crate::codegen::CompileOutput, String> {
        let tokens = crate::lexer::lex(source).map_err(|e| e.to_string())?;
        let mut parser = crate::parser::Parser::new(tokens);
        let cinematic = parser.parse().map_err(|e| e.to_string())?;
        crate::codegen::generate_full(&cinematic).map_err(|e| e.to_string())
    })();

    match result {
        Ok(output) => {
            let obj = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&obj, &"valid".into(), &JsValue::TRUE);
            let warnings = js_sys::Array::new();
            for w in &output.warnings {
                warnings.push(&JsValue::from_str(w));
            }
            let _ = js_sys::Reflect::set(&obj, &"warnings".into(), &warnings.into());
            let _ = js_sys::Reflect::set(
                &obj,
                &"layers".into(),
                &JsValue::from_f64(output.layer_count as f64),
            );
            let params = js_sys::Array::new();
            for p in &output.params {
                params.push(&JsValue::from_str(&p.name));
            }
            let _ = js_sys::Reflect::set(&obj, &"params".into(), &params.into());
            let _ = js_sys::Reflect::set(
                &obj,
                &"uses_audio".into(),
                &JsValue::from_bool(output.uses_audio),
            );
            let _ = js_sys::Reflect::set(
                &obj,
                &"uses_mouse".into(),
                &JsValue::from_bool(output.uses_mouse),
            );
            obj.into()
        }
        Err(msg) => {
            let obj = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&obj, &"valid".into(), &JsValue::FALSE);
            let _ = js_sys::Reflect::set(&obj, &"error".into(), &JsValue::from_str(&msg));
            obj.into()
        }
    }
}
