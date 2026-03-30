//! React block codegen — compiles `react { signal -> action }` into JS event listeners.
//!
//! Categorizes signals by pattern matching on Expr variants:
//! - `mouse.click` → click listener
//! - `mouse.x`/`mouse.y` → mousemove with normalized coordinates
//! - `key("x")` → keydown listener
//! - `audio.field > threshold` → audio threshold check in animation loop

use crate::ast::{BinOp, Expr, ReactBlock};
use crate::codegen::expr;
use crate::codegen::UniformInfo;

/// Signal category determined by pattern matching the signal expression.
enum SignalKind {
    /// `mouse.click` — generates a click addEventListener
    MouseClick,
    /// `mouse.x` or `mouse.y` — generates a mousemove listener with normalized coords
    MouseMove(String),
    /// `key("x")` — generates a keydown listener for a specific key
    Key(String),
    /// `audio.field > threshold` — generates an audio threshold check
    AudioThreshold { field: String, threshold: f64 },
    /// Unrecognized signal — emit as a comment
    Unknown(String),
}

/// Classify a signal expression into a SignalKind.
fn classify_signal(signal: &Expr) -> SignalKind {
    match signal {
        Expr::DottedIdent { object, field } if object == "mouse" && field == "click" => {
            SignalKind::MouseClick
        }
        Expr::DottedIdent { object, field }
            if object == "mouse" && (field == "x" || field == "y") =>
        {
            SignalKind::MouseMove(field.clone())
        }
        Expr::Call { name, args } if name == "key" => {
            let key_name = args.first().and_then(|a| match &a.value {
                Expr::String(s) => Some(s.clone()),
                Expr::Ident(s) => Some(s.clone()),
                _ => None,
            });
            SignalKind::Key(key_name.unwrap_or_else(|| "unknown".into()))
        }
        Expr::BinOp {
            op: BinOp::Gt,
            left,
            right,
        } => {
            if let Expr::DottedIdent { object, field } = left.as_ref() {
                if object == "audio" {
                    let threshold = match right.as_ref() {
                        Expr::Number(v) => *v,
                        _ => 0.5,
                    };
                    return SignalKind::AudioThreshold {
                        field: field.clone(),
                        threshold,
                    };
                }
            }
            SignalKind::Unknown(expr::compile_js(signal))
        }
        _ => SignalKind::Unknown(expr::compile_js(signal)),
    }
}

/// Determine the action JS code from an action expression.
///
/// Recognizes:
/// - `Ident(name)` → set uniform by name
/// - `Call { name: "pulse", args }` → impulse with decay
/// - `Call { name: "toggle", args }` → toggle uniform between 0 and 1
/// - Other expressions → compile to JS
fn compile_action(action: &Expr, uniforms: &[UniformInfo]) -> String {
    match action {
        Expr::Ident(name) => {
            // Setting a uniform directly via the renderer
            if uniforms.iter().any(|u| u.name == *name) {
                format!("renderer.setParam('{name}', v);")
            } else {
                format!("// set {name} = v;")
            }
        }
        Expr::Call { name, args } if name == "pulse" => {
            let magnitude = args
                .first()
                .map(|a| expr::compile_js(&a.value))
                .unwrap_or_else(|| "1.0".into());
            format!("pulse({magnitude});")
        }
        Expr::Call { name, args } if name == "toggle" => {
            let target = args
                .first()
                .map(|a| expr::compile_js(&a.value))
                .unwrap_or_else(|| "0".into());
            format!("toggle({target});")
        }
        _ => {
            let js = expr::compile_js(action);
            format!("{js};")
        }
    }
}

/// Compile a ReactBlock into a JS setup function.
///
/// Generates a `_gameReactSetup(canvas, renderer)` function that attaches event
/// listeners and wires audio threshold checks into the renderer's `_onRender`
/// callback. Called by the component/HTML after the renderer is initialized.
pub fn generate_react_js(block: &ReactBlock, uniforms: &[UniformInfo]) -> String {
    if block.reactions.is_empty() {
        return String::new();
    }

    // Collect audio threshold fields to wire into the render loop
    let audio_fields: Vec<String> = block
        .reactions
        .iter()
        .filter_map(|r| match classify_signal(&r.signal) {
            SignalKind::AudioThreshold { field, .. } => Some(field),
            _ => None,
        })
        .collect();

    let mut s = String::with_capacity(1024);
    s.push_str("// GAME react — event listeners + audio threshold checks\n");
    s.push_str("function _gameReactSetup(canvas, renderer) {\n");

    // Collect pulse/toggle helpers if needed
    let needs_pulse = block
        .reactions
        .iter()
        .any(|r| matches!(&r.action, Expr::Call { name, .. } if name == "pulse"));
    let needs_toggle = block
        .reactions
        .iter()
        .any(|r| matches!(&r.action, Expr::Call { name, .. } if name == "toggle"));

    if needs_pulse {
        s.push_str("  let _pulseVal = 0;\n");
        s.push_str("  function pulse(mag) { _pulseVal = mag; }\n");
    }
    if needs_toggle {
        s.push_str("  let _toggleState = 0;\n");
        s.push_str("  function toggle(name) { _toggleState = 1 - _toggleState; renderer.setParam(name, _toggleState); }\n");
    }

    for reaction in &block.reactions {
        let kind = classify_signal(&reaction.signal);
        let action_js = compile_action(&reaction.action, uniforms);

        match kind {
            SignalKind::MouseClick => {
                s.push_str("  canvas.addEventListener('click', function(e) {\n");
                s.push_str("    const v = 1.0;\n");
                s.push_str(&format!("    {action_js}\n"));
                s.push_str("  });\n");
            }
            SignalKind::MouseMove(axis) => {
                s.push_str("  canvas.addEventListener('mousemove', function(e) {\n");
                s.push_str("    const rect = canvas.getBoundingClientRect();\n");
                if axis == "x" {
                    s.push_str(
                        "    const v = (e.clientX - rect.left) / rect.width;\n",
                    );
                } else {
                    s.push_str(
                        "    const v = 1.0 - (e.clientY - rect.top) / rect.height;\n",
                    );
                }
                s.push_str(&format!("    {action_js}\n"));
                s.push_str("  });\n");
            }
            SignalKind::Key(key) => {
                s.push_str("  document.addEventListener('keydown', function(e) {\n");
                s.push_str(&format!(
                    "    if (e.key === '{key}') {{\n"
                ));
                s.push_str("      const v = 1.0;\n");
                s.push_str(&format!("      {action_js}\n"));
                s.push_str("    }\n");
                s.push_str("  });\n");
            }
            SignalKind::AudioThreshold { field, threshold } => {
                s.push_str(&format!(
                    "  // audio threshold: {field} > {threshold}\n"
                ));
                s.push_str(&format!(
                    "  function _checkAudio_{field}() {{\n"
                ));
                s.push_str(&format!(
                    "    const audioData = renderer.audioData;\n"
                ));
                s.push_str(&format!(
                    "    if (audioData && audioData.{field} > {threshold}) {{\n"
                ));
                s.push_str("      const v = 1.0;\n");
                s.push_str(&format!("      {action_js}\n"));
                s.push_str("    }\n");
                s.push_str("  }\n");
            }
            SignalKind::Unknown(sig_js) => {
                s.push_str(&format!("  // unknown signal: {sig_js}\n"));
            }
        }
    }

    // Wire audio threshold checks into the render loop via _onRender callback
    if !audio_fields.is_empty() {
        s.push_str("  renderer._onRender = function() {\n");
        for field in &audio_fields {
            s.push_str(&format!("    _checkAudio_{field}();\n"));
        }
        s.push_str("  };\n");
    }

    s.push_str("}\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    #[test]
    fn empty_react_block_returns_empty() {
        let block = ReactBlock {
            reactions: vec![],
        };
        let js = generate_react_js(&block, &[]);
        assert!(js.is_empty());
    }

    #[test]
    fn mouse_click_generates_click_listener() {
        let block = ReactBlock {
            reactions: vec![Reaction {
                signal: Expr::DottedIdent {
                    object: "mouse".into(),
                    field: "click".into(),
                },
                action: Expr::Call {
                    name: "pulse".into(),
                    args: vec![Arg {
                        name: None,
                        value: Expr::Number(1.0),
                    }],
                },
            }],
        };
        let js = generate_react_js(&block, &[]);
        assert!(js.contains("addEventListener('click'"));
        assert!(js.contains("pulse(1.0)"));
    }

    #[test]
    fn mouse_x_generates_mousemove_listener() {
        let uniforms = vec![UniformInfo {
            name: "intensity".into(),
            default: 0.5,
        }];
        let block = ReactBlock {
            reactions: vec![Reaction {
                signal: Expr::DottedIdent {
                    object: "mouse".into(),
                    field: "x".into(),
                },
                action: Expr::Ident("intensity".into()),
            }],
        };
        let js = generate_react_js(&block, &uniforms);
        assert!(js.contains("addEventListener('mousemove'"));
        assert!(js.contains("rect.width"));
        assert!(js.contains("renderer.setParam('intensity', v);"));
    }

    #[test]
    fn key_signal_generates_keydown_listener() {
        let block = ReactBlock {
            reactions: vec![Reaction {
                signal: Expr::Call {
                    name: "key".into(),
                    args: vec![Arg {
                        name: None,
                        value: Expr::String("x".into()),
                    }],
                },
                action: Expr::Call {
                    name: "pulse".into(),
                    args: vec![Arg {
                        name: None,
                        value: Expr::Number(2.0),
                    }],
                },
            }],
        };
        let js = generate_react_js(&block, &[]);
        assert!(js.contains("addEventListener('keydown'"));
        assert!(js.contains("e.key === 'x'"));
    }

    #[test]
    fn audio_threshold_generates_check() {
        let block = ReactBlock {
            reactions: vec![Reaction {
                signal: Expr::BinOp {
                    op: BinOp::Gt,
                    left: Box::new(Expr::DottedIdent {
                        object: "audio".into(),
                        field: "bass".into(),
                    }),
                    right: Box::new(Expr::Number(0.8)),
                },
                action: Expr::Call {
                    name: "pulse".into(),
                    args: vec![Arg {
                        name: None,
                        value: Expr::Number(1.0),
                    }],
                },
            }],
        };
        let js = generate_react_js(&block, &[]);
        assert!(js.contains("audioData.bass > 0.8"));
        assert!(js.contains("renderer._onRender"), "audio checks must be wired into render loop");
        assert!(js.contains("_checkAudio_bass()"));
    }

    #[test]
    fn mouse_y_generates_inverted_coord() {
        let uniforms = vec![UniformInfo {
            name: "height".into(),
            default: 0.0,
        }];
        let block = ReactBlock {
            reactions: vec![Reaction {
                signal: Expr::DottedIdent {
                    object: "mouse".into(),
                    field: "y".into(),
                },
                action: Expr::Ident("height".into()),
            }],
        };
        let js = generate_react_js(&block, &uniforms);
        assert!(js.contains("1.0 - (e.clientY"));
    }

    #[test]
    fn unknown_signal_emits_comment() {
        let block = ReactBlock {
            reactions: vec![Reaction {
                signal: Expr::Number(42.0),
                action: Expr::Ident("x".into()),
            }],
        };
        let js = generate_react_js(&block, &[]);
        assert!(js.contains("// unknown signal"));
    }
}
