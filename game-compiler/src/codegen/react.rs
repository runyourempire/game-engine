//! React block compilation — maps user inputs to actions via JS event listeners.
//!
//! The `react {}` block specifies how user input drives the cinematic:
//!   react {
//!     mouse.click -> ripples.add_origin(at: mouse.uv, strength: 1.0)
//!     key("space") -> arc.pause_toggle
//!     mouse.x -> fire.intensity.bias(0.3)
//!   }
//!
//! This compiles to JS event listeners that are injected into the runtime.

use crate::ast::{Expr, ReactBlock};
use crate::codegen::CompiledParam;
use crate::codegen::expr::compile_expr_js;

/// Compile a ReactBlock into JS event listener code.
pub fn compile_react(
    block: &ReactBlock,
    _params: &[CompiledParam],
) -> String {
    if block.reactions.is_empty() {
        return String::new();
    }

    let mut js = String::with_capacity(512);
    js.push_str("// ── React: user interaction handlers ──\n");

    for reaction in &block.reactions {
        let signal_js = compile_expr_js(&reaction.signal);
        let action_js = compile_expr_js(&reaction.action);

        // Detect signal type to generate appropriate event listener
        match categorize_signal(&reaction.signal) {
            SignalType::MouseClick => {
                js.push_str("document.addEventListener('click', (e) => {\n");
                js.push_str(&format!("  // Action: {action_js}\n"));
                js.push_str("  console.log('[GAME react] mouse.click triggered');\n");
                js.push_str("});\n");
            }
            SignalType::KeyPress(key) => {
                js.push_str(&format!(
                    "document.addEventListener('keydown', (e) => {{\n"
                ));
                js.push_str(&format!("  if (e.key === '{key}') {{\n"));
                js.push_str(&format!("    e.preventDefault();\n"));

                // Handle common arc actions
                if action_js.contains("pause_toggle") {
                    js.push_str("    if (typeof btnToggle !== 'undefined') btnToggle.click();\n");
                } else {
                    js.push_str(&format!("    console.log('[GAME react] key({key}) triggered');\n"));
                }

                js.push_str("  }\n");
                js.push_str("});\n");
            }
            SignalType::MouseAxis(axis) => {
                // mouse.x or mouse.y -> param.bias(amount)
                // This modulates a param based on mouse position
                js.push_str(&format!(
                    "// mouse.{axis} reaction: {action_js}\n"
                ));
            }
            SignalType::Other => {
                js.push_str(&format!("// Unrecognized react signal: {signal_js} -> {action_js}\n"));
            }
        }
    }

    js
}

enum SignalType {
    MouseClick,
    KeyPress(String),
    MouseAxis(String),
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
            }
        }
        Expr::Call(call) if call.name == "key" => {
            if let Some(crate::ast::Arg::Positional(Expr::String(key))) = call.args.first() {
                return SignalType::KeyPress(key.clone());
            }
        }
        _ => {}
    }
    SignalType::Other
}
