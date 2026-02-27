//! React block compilation — maps user inputs to actions via JS event listeners.
//!
//! The `react {}` block specifies how user input drives the cinematic:
//!   react {
//!     mouse.click -> particles.burst(...)
//!     key("space") -> arc.pause_toggle
//!     key("r") -> arc.restart
//!     key("d") -> density.set(2.0)
//!     key("t") -> opacity.toggle(0.0, 1.0)
//!     mouse.x -> fire.intensity.bias(0.3)
//!     audio.bass > 0.5 -> arc.pause_toggle
//!   }
//!
//! This compiles to JS event listeners injected into the runtime.

use crate::ast::{Arg, Expr, ReactBlock};
use crate::codegen::CompiledParam;
use crate::codegen::expr::compile_expr_js;

/// Compile a ReactBlock into JS event listener code.
pub fn compile_react(
    block: &ReactBlock,
    params: &[CompiledParam],
    warnings: &mut Vec<String>,
) -> String {
    if block.reactions.is_empty() {
        return String::new();
    }

    let mut js = String::with_capacity(1024);
    js.push_str("// ── React: user interaction handlers ──\n");

    for reaction in &block.reactions {
        let action = categorize_action(&reaction.action, params);

        match categorize_signal(&reaction.signal) {
            SignalType::MouseClick => {
                js.push_str("el.addEventListener('click', (e) => {\n");
                emit_action(&mut js, &action, "  ");
                js.push_str("});\n");
            }
            SignalType::KeyPress(key) => {
                js.push_str("document.addEventListener('keydown', (e) => {\n");
                js.push_str(&format!("  if (e.key === '{key}') {{\n"));
                js.push_str("    e.preventDefault();\n");
                emit_action(&mut js, &action, "    ");
                js.push_str("  }\n");
                js.push_str("});\n");
            }
            SignalType::MouseAxis(axis) => {
                let mouse_var = match axis.as_str() {
                    "x" => "mouseX",
                    _ => "mouseY",
                };
                match &action {
                    Action::ParamBias { param_idx, amount } => {
                        js.push_str(&format!(
                            "// mouse.{axis} -> param[{param_idx}].bias({amount})\n"
                        ));
                        js.push_str(&format!(
                            "Object.defineProperty(window, '_game_mouse_{axis}_bias', {{ value: {{ idx: {param_idx}, amt: {amount} }}, writable: true }});\n"
                        ));
                        js.push_str(&format!(
                            "// Applied in frame loop: params[{param_idx}].base += ({mouse_var} - 0.5) * {amount}\n"
                        ));
                    }
                    _ => {
                        warnings.push(format!(
                            "mouse.{axis} signal only supports param.bias() action"
                        ));
                        js.push_str(&format!("// mouse.{axis} reaction (unsupported action)\n"));
                    }
                }
            }
            SignalType::AudioThreshold { field, threshold } => {
                js.push_str(&format!(
                    "// audio.{field} > {threshold} -> action (checked each frame)\n"
                ));
                js.push_str(&format!(
                    "window._game_threshold_{field} = {{ threshold: {threshold}, triggered: false }};\n"
                ));
                js.push_str(&format!(
                    "// Frame loop checks: if ({field} > {threshold} && !triggered) {{ action; triggered = true }}\n"
                ));
                js.push_str(&format!(
                    "// Reset: if ({field} <= {threshold}) triggered = false\n"
                ));
                // Emit the actual threshold check as a function for the frame loop
                js.push_str(&format!(
                    "window._game_threshold_{field}_fn = function({field}Val) {{\n"
                ));
                js.push_str(&format!(
                    "  const state = window._game_threshold_{field};\n"
                ));
                js.push_str(&format!(
                    "  if ({field}Val > state.threshold && !state.triggered) {{\n"
                ));
                js.push_str("    state.triggered = true;\n");
                emit_action(&mut js, &action, "    ");
                js.push_str("  }\n");
                js.push_str(&format!(
                    "  if ({field}Val <= state.threshold) state.triggered = false;\n"
                ));
                js.push_str("};\n");
            }
            SignalType::Other => {
                let signal_js = compile_expr_js(&reaction.signal);
                warnings.push(format!(
                    "unrecognized react signal '{signal_js}' — supported: mouse.click, key(\"x\"), mouse.x/y, audio.field > threshold"
                ));
                js.push_str(&format!("// Unrecognized react signal: {signal_js}\n"));
            }
        }
    }

    js
}

// ── Signal categorization ─────────────────────────────────────────────

enum SignalType {
    MouseClick,
    KeyPress(String),
    MouseAxis(String),
    AudioThreshold { field: String, threshold: f64 },
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
            if let Some(Arg::Positional(Expr::String(key))) = call.args.first() {
                return SignalType::KeyPress(key.clone());
            }
        }
        // audio.bass > 0.5 -> threshold trigger
        Expr::BinaryOp { left, op: crate::ast::BinOp::Gt, right } => {
            if let Expr::FieldAccess { object, field } = left.as_ref() {
                if let Expr::Ident(obj) = object.as_ref() {
                    if obj == "audio" {
                        if let Expr::Number(threshold) = right.as_ref() {
                            return SignalType::AudioThreshold {
                                field: field.clone(),
                                threshold: *threshold,
                            };
                        }
                    }
                }
            }
        }
        _ => {}
    }
    SignalType::Other
}

// ── Action categorization ─────────────────────────────────────────────

enum Action {
    ArcPauseToggle,
    ArcRestart,
    ParamSet { param_idx: usize, value: f64 },
    ParamToggle { param_idx: usize, val_a: f64, val_b: f64 },
    ParamBias { param_idx: usize, amount: f64 },
    ClickImpulse,
    Unknown(String),
}

fn categorize_action(action: &Expr, params: &[CompiledParam]) -> Action {
    match action {
        // arc.pause_toggle
        Expr::FieldAccess { object, field } => {
            if let Expr::Ident(obj) = object.as_ref() {
                if obj == "arc" && field == "pause_toggle" {
                    return Action::ArcPauseToggle;
                }
                if obj == "arc" && field == "restart" {
                    return Action::ArcRestart;
                }
            }
        }
        // param.set(value) or param.toggle(a, b) or param.bias(amount)
        Expr::Call(call) => {
            if let Some(dot_pos) = call.name.rfind('.') {
                let param_name = &call.name[..dot_pos];
                let method = &call.name[dot_pos + 1..];

                if let Some(idx) = params.iter().position(|p| p.name == param_name) {
                    return match method {
                        "set" => {
                            let val = extract_first_number(&call.args).unwrap_or(0.0);
                            Action::ParamSet { param_idx: idx, value: val }
                        }
                        "toggle" => {
                            let val_a = extract_number_at(&call.args, 0).unwrap_or(0.0);
                            let val_b = extract_number_at(&call.args, 1).unwrap_or(1.0);
                            Action::ParamToggle { param_idx: idx, val_a, val_b }
                        }
                        "bias" => {
                            let amount = extract_first_number(&call.args).unwrap_or(0.5);
                            Action::ParamBias { param_idx: idx, amount }
                        }
                        _ => Action::Unknown(compile_expr_js(action)),
                    };
                }
            }
            // mouse.click -> particles.burst(...) => impulse
            if call.name.contains("burst") || call.name.contains("spawn") {
                return Action::ClickImpulse;
            }
        }
        _ => {}
    }
    // Fallback: parse the action expression
    let action_str = compile_expr_js(action);
    if action_str.contains("pause_toggle") {
        Action::ArcPauseToggle
    } else if action_str.contains("restart") {
        Action::ArcRestart
    } else {
        Action::Unknown(action_str)
    }
}

fn extract_first_number(args: &[Arg]) -> Option<f64> {
    if let Some(Arg::Positional(Expr::Number(n))) = args.first() {
        Some(*n)
    } else {
        None
    }
}

fn extract_number_at(args: &[Arg], idx: usize) -> Option<f64> {
    if let Some(Arg::Positional(Expr::Number(n))) = args.get(idx) {
        Some(*n)
    } else {
        None
    }
}

// ── Action code emission ──────────────────────────────────────────────

fn emit_action(js: &mut String, action: &Action, indent: &str) {
    match action {
        Action::ArcPauseToggle => {
            js.push_str(&format!("{indent}if (typeof btnToggle !== 'undefined') btnToggle.click();\n"));
        }
        Action::ArcRestart => {
            js.push_str(&format!("{indent}if (typeof arcState !== 'undefined') {{\n"));
            js.push_str(&format!("{indent}  arcState.startTime = performance.now() / 1000;\n"));
            js.push_str(&format!("{indent}  arcState.currentMoment = 0;\n"));
            js.push_str(&format!("{indent}}}\n"));
        }
        Action::ParamSet { param_idx, value } => {
            js.push_str(&format!("{indent}if (params[{param_idx}]) params[{param_idx}].base = {value};\n"));
        }
        Action::ParamToggle { param_idx, val_a, val_b } => {
            js.push_str(&format!(
                "{indent}if (params[{param_idx}]) params[{param_idx}].base = (params[{param_idx}].base === {val_a}) ? {val_b} : {val_a};\n"
            ));
        }
        Action::ParamBias { param_idx, amount } => {
            js.push_str(&format!(
                "{indent}// bias applied in frame loop for param[{param_idx}] by {amount}\n"
            ));
        }
        Action::ClickImpulse => {
            js.push_str(&format!("{indent}// Impulse: set a decay uniform that fades over frames\n"));
            js.push_str(&format!("{indent}window._game_impulse = 1.0;\n"));
        }
        Action::Unknown(action_js) => {
            js.push_str(&format!("{indent}console.log('[GAME react]', {action_js:?});\n"));
        }
    }
}
