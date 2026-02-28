//! React block compilation — maps user inputs to actions via JS event listeners.
//!
//! The `react {}` block specifies how user input drives the cinematic:
//!   react {
//!     mouse.click -> ripples.add_origin(at: mouse.uv, strength: 1.0)
//!     key("space") -> arc.pause_toggle
//!     mouse.x -> fire.intensity.bias(0.3)
//!     scroll -> intensity
//!   }
//!
//! This compiles to JS event listeners that are injected into the runtime.

use crate::ast::{Expr, ReactBlock};
use crate::codegen::CompiledParam;
use crate::codegen::expr::compile_expr_js;

/// Compile a ReactBlock into JS event listener code.
pub fn compile_react(
    block: &ReactBlock,
    params: &[CompiledParam],
) -> String {
    if block.reactions.is_empty() {
        return String::new();
    }

    let mut js = String::with_capacity(512);
    js.push_str("// ── React: user interaction handlers ──\n");

    for reaction in &block.reactions {
        let action_js = compile_expr_js(&reaction.action);

        // Detect signal type to generate appropriate event listener
        match categorize_signal(&reaction.signal) {
            SignalType::MouseClick => {
                emit_click_handler(&mut js, &reaction.action, &action_js, params);
            }
            SignalType::KeyPress(key) => {
                emit_key_handler(&mut js, &key, &reaction.action, &action_js, params);
            }
            SignalType::MouseAxis(axis) => {
                emit_mouse_axis_handler(&mut js, &axis, &reaction.action, &action_js, params);
            }
            SignalType::Scroll => {
                emit_scroll_handler(&mut js, &reaction.action, &action_js, params);
            }
            SignalType::Hover => {
                emit_hover_handler(&mut js, &action_js);
            }
            SignalType::Other => {
                let signal_js = compile_expr_js(&reaction.signal);
                js.push_str(&format!(
                    "// Unrecognized react signal: {signal_js} -> {action_js}\n"
                ));
            }
        }
    }

    js
}

// ── Signal classification ──────────────────────────────────────────

enum SignalType {
    MouseClick,
    KeyPress(String),
    MouseAxis(String),
    Scroll,
    Hover,
    Other,
}

fn categorize_signal(signal: &Expr) -> SignalType {
    match signal {
        Expr::FieldAccess { object, field } => {
            if let Expr::Ident(obj) = object.as_ref() {
                if obj == "mouse" && field == "click" {
                    return SignalType::MouseClick;
                }
                if obj == "mouse" && (field == "x" || field == "y") {
                    return SignalType::MouseAxis(field.clone());
                }
                if obj == "mouse" && field == "move" {
                    return SignalType::Hover;
                }
            }
        }
        Expr::Call(call) if call.name == "key" => {
            if let Some(crate::ast::Arg::Positional(Expr::String(key))) = call.args.first() {
                return SignalType::KeyPress(key.clone());
            }
        }
        Expr::Ident(name) => {
            if name == "scroll" {
                return SignalType::Scroll;
            }
            if name == "hover" {
                return SignalType::Hover;
            }
        }
        _ => {}
    }
    SignalType::Other
}

// ── Handler emitters ───────────────────────────────────────────────

/// Resolve an action expression to a param index. Handles:
/// - Simple ident: `intensity` -> find param named "intensity"
/// - Field access: `fire.intensity` -> find param named "intensity"
fn resolve_param_index(action: &Expr, params: &[CompiledParam]) -> Option<usize> {
    match action {
        Expr::Ident(name) => {
            params.iter().position(|p| p.name == *name)
        }
        Expr::FieldAccess { field, .. } => {
            // e.g., fire.intensity -> look up "intensity"
            params.iter().position(|p| p.name == *field)
        }
        _ => None,
    }
}

/// Generate a click event listener with canvas coordinate mapping.
fn emit_click_handler(
    js: &mut String,
    action: &Expr,
    action_js: &str,
    params: &[CompiledParam],
) {
    js.push_str("document.addEventListener('click', (e) => {\n");
    js.push_str("  const canvas = document.getElementById('canvas');\n");
    js.push_str("  const rect = canvas.getBoundingClientRect();\n");
    js.push_str("  const x = (e.clientX - rect.left) / rect.width;\n");
    js.push_str("  const y = 1.0 - (e.clientY - rect.top) / rect.height;\n");

    // Try to resolve the action to a param update
    if let Some(idx) = resolve_param_index(action, params) {
        js.push_str(&format!(
            "  // React: click -> update param '{}'\n",
            params[idx].name
        ));
        js.push_str(&format!(
            "  params[{idx}].base = {action_js};\n"
        ));
    } else {
        js.push_str(&format!("  // Action: {action_js}\n"));
    }

    js.push_str("  console.log('[GAME react] click at', x.toFixed(2), y.toFixed(2));\n");
    js.push_str("});\n");
}

/// Generate a keydown event listener with key matching.
fn emit_key_handler(
    js: &mut String,
    key: &str,
    action: &Expr,
    action_js: &str,
    params: &[CompiledParam],
) {
    // Normalize key name for JS: "space" -> " "
    let js_key = match key {
        "space" => " ",
        other => other,
    };

    js.push_str(&format!("// key(\"{key}\") handler\n"));
    js.push_str("document.addEventListener('keydown', (e) => {\n");
    js.push_str(&format!("  if (e.key === '{js_key}') {{\n"));
    js.push_str("    e.preventDefault();\n");

    // Handle arc actions
    if action_js.contains("pause_toggle") {
        js.push_str("    if (typeof btnToggle !== 'undefined') btnToggle.click();\n");
    } else if action_js.contains("reset") {
        // Reset all params to their base values
        js.push_str("    for (let i = 0; i < params.length; i++) {\n");
        js.push_str("      params[i].value = params[i].base;\n");
        js.push_str("    }\n");
        js.push_str(&format!(
            "    console.log('[GAME react] key({key}) -> reset');\n"
        ));
    } else if let Some(idx) = resolve_param_index(action, params) {
        js.push_str(&format!(
            "    // React: key({key}) -> update param '{}'\n",
            params[idx].name
        ));
        js.push_str(&format!(
            "    params[{idx}].base = {action_js};\n"
        ));
    } else {
        js.push_str(&format!(
            "    console.log('[GAME react] key({key}) -> {action_js}');\n"
        ));
    }

    js.push_str("  }\n");
    js.push_str("});\n");
}

/// Generate a mousemove listener that drives a param from mouse axis position.
fn emit_mouse_axis_handler(
    js: &mut String,
    axis: &str,
    action: &Expr,
    action_js: &str,
    params: &[CompiledParam],
) {
    let coord_expr = match axis {
        "x" => "(e.clientX - rect.left) / rect.width",
        "y" => "1.0 - (e.clientY - rect.top) / rect.height",
        _ => "(e.clientX - rect.left) / rect.width",
    };

    if let Some(idx) = resolve_param_index(action, params) {
        let base_val = params[idx].base_value;
        js.push_str(&format!(
            "// mouse.{axis} -> param '{}' (index {idx})\n",
            params[idx].name
        ));
        js.push_str("document.addEventListener('mousemove', (e) => {\n");
        js.push_str("  const canvas = document.getElementById('canvas');\n");
        js.push_str("  const rect = canvas.getBoundingClientRect();\n");
        js.push_str(&format!("  const coord = {coord_expr};\n"));
        // Scale the param: mouse position [0,1] maps to [0, 2*base_value]
        // so at center (0.5) the param equals its original base value
        js.push_str(&format!(
            "  params[{idx}].base = coord * {base_val} * 2.0;\n",
            base_val = base_val,
        ));
        js.push_str("});\n");
    } else {
        js.push_str(&format!(
            "// mouse.{axis} -> {action_js} (param not resolved)\n"
        ));
        js.push_str("document.addEventListener('mousemove', (e) => {\n");
        js.push_str("  const canvas = document.getElementById('canvas');\n");
        js.push_str("  const rect = canvas.getBoundingClientRect();\n");
        js.push_str(&format!("  const coord = {coord_expr};\n"));
        js.push_str(&format!(
            "  console.log('[GAME react] mouse.{axis}', coord.toFixed(2));\n"
        ));
        js.push_str("});\n");
    }
}

/// Generate a wheel event listener with delta normalization.
fn emit_scroll_handler(
    js: &mut String,
    action: &Expr,
    action_js: &str,
    params: &[CompiledParam],
) {
    js.push_str("document.addEventListener('wheel', (e) => {\n");
    js.push_str("  e.preventDefault();\n");
    js.push_str("  const delta = Math.sign(e.deltaY) * -0.05;\n");

    if let Some(idx) = resolve_param_index(action, params) {
        js.push_str(&format!(
            "  // React: scroll -> update param '{}'\n",
            params[idx].name
        ));
        js.push_str(&format!(
            "  params[{idx}].base = Math.max(0.0, params[{idx}].base + delta);\n"
        ));
    } else {
        js.push_str(&format!("  // scroll -> {action_js}\n"));
        js.push_str("  console.log('[GAME react] scroll delta', delta.toFixed(3));\n");
    }

    js.push_str("}, { passive: false });\n");
}

/// Generate a mousemove listener for hover signal.
fn emit_hover_handler(js: &mut String, action_js: &str) {
    js.push_str("document.addEventListener('mousemove', (e) => {\n");
    js.push_str("  const canvas = document.getElementById('canvas');\n");
    js.push_str("  const rect = canvas.getBoundingClientRect();\n");
    js.push_str("  const x = (e.clientX - rect.left) / rect.width;\n");
    js.push_str("  const y = 1.0 - (e.clientY - rect.top) / rect.height;\n");
    js.push_str(&format!("  // hover action: {action_js}\n"));
    js.push_str("});\n");
}
