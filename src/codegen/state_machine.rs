//! State machine codegen — visual state transitions with interpolation.
//!
//! Generates a `GameStateMachine` JS class that manages named visual states
//! with smooth parameter interpolation between them. Each state can define
//! full layer overrides or individual parameter overrides relative to a
//! parent state.
//!
//! ```game
//! state idle {
//!   layer bg { box(0.9, 0.35) | shade(0.08, 0.08, 0.08) }
//!   layer glow { box(0.88, 0.33) | glow(0.5) | tint(0.83, 0.69, 0.22) }
//! }
//!
//! state hover from idle over 150ms ease-out {
//!   glow.intensity: 1.2
//! }
//! ```

use crate::ast::{Duration, Expr, StateBlock};

/// Convert a Duration to seconds.
fn duration_to_seconds(d: &Duration) -> f64 {
    match d {
        Duration::Seconds(v) => *v,
        Duration::Millis(v) => *v / 1000.0,
        Duration::Bars(v) => *v as f64 * 2.0, // default 120 BPM
    }
}

/// Convert an Expr to a JS literal string.
fn expr_to_js(e: &Expr) -> String {
    match e {
        Expr::Number(v) => format!("{v}"),
        Expr::Ident(name) => name.clone(),
        Expr::DottedIdent { object, field } => format!("{object}.{field}"),
        Expr::BinOp { op, left, right } => {
            let l = expr_to_js(left);
            let r = expr_to_js(right);
            let op_str = match op {
                crate::ast::BinOp::Add => "+",
                crate::ast::BinOp::Sub => "-",
                crate::ast::BinOp::Mul => "*",
                crate::ast::BinOp::Div => "/",
                crate::ast::BinOp::Pow => "**",
                crate::ast::BinOp::Gt => ">",
                crate::ast::BinOp::Lt => "<",
                crate::ast::BinOp::Gte => ">=",
                crate::ast::BinOp::Lte => "<=",
                crate::ast::BinOp::Eq => "===",
                crate::ast::BinOp::NotEq => "!==",
            };
            format!("({l} {op_str} {r})")
        }
        Expr::Neg(inner) => format!("(-{})", expr_to_js(inner)),
        _ => "0".into(),
    }
}

/// Map easing name to JS easing function body.
fn easing_fn_js(name: &str) -> &'static str {
    match name {
        "ease-in" | "ease_in" => "t * t",
        "ease-out" | "ease_out" => "t * (2 - t)",
        "ease-in-out" | "ease_in_out" => "t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t",
        "ease-in-cubic" | "ease_in_cubic" => "t * t * t",
        "ease-out-cubic" | "ease_out_cubic" => "(--t) * t * t + 1",
        "elastic" => {
            "Math.pow(2, -10 * t) * Math.sin((t - 0.075) * (2 * Math.PI) / 0.3) + 1"
        }
        "bounce" => {
            "(t < 1/2.75 ? 7.5625*t*t : t < 2/2.75 ? 7.5625*(t-=1.5/2.75)*t+0.75 : t < 2.5/2.75 ? 7.5625*(t-=2.25/2.75)*t+0.9375 : 7.5625*(t-=2.625/2.75)*t+0.984375)"
        }
        _ => "t", // linear default
    }
}

/// Generate the `GameStateMachine` JS class from a list of state blocks.
pub fn generate_state_machine_js(states: &[StateBlock]) -> String {
    let mut s = String::with_capacity(4096);

    // Emit easing functions used by states
    let mut easings = std::collections::HashSet::new();
    for state in states {
        if let Some(ref e) = state.transition_easing {
            easings.insert(e.clone());
        }
    }

    s.push_str("const _gameStateEasings = {\n");
    s.push_str("  linear: t => t,\n");
    for name in &easings {
        let body = easing_fn_js(name);
        let js_name = name.replace('-', "_");
        s.push_str(&format!("  {js_name}: t => {body},\n"));
    }
    s.push_str("};\n\n");

    // Emit the state machine class
    s.push_str("class GameStateMachine {\n");
    s.push_str("  constructor() {\n");

    // Build state definitions
    s.push_str("    this._states = {\n");
    for state in states {
        let dur = state
            .transition_duration
            .as_ref()
            .map(duration_to_seconds)
            .unwrap_or(0.0);
        let easing_js = state
            .transition_easing
            .as_deref()
            .unwrap_or("linear")
            .replace('-', "_");
        let parent_js = state
            .parent
            .as_deref()
            .map(|p| format!("'{p}'"))
            .unwrap_or_else(|| "null".into());

        s.push_str(&format!(
            "      '{}': {{ parent: {}, duration: {}, easing: '{}', overrides: {{",
            state.name, parent_js, dur, easing_js
        ));

        for ovr in &state.overrides {
            let val = expr_to_js(&ovr.value);
            s.push_str(&format!(" '{}.{}': {},", ovr.layer, ovr.param, val));
        }
        s.push_str(" } },\n");
    }
    s.push_str("    };\n");

    // Current state tracking
    s.push_str("    this._current = null;\n");
    s.push_str("    this._target = null;\n");
    s.push_str("    this._transitionStart = null;\n");
    s.push_str("    this._transitionDuration = 0;\n");
    s.push_str("    this._transitionEasing = 'linear';\n");
    s.push_str("    this._fromParams = {};\n");
    s.push_str("    this._toParams = {};\n");

    // Find initial state (first state with no parent, or just the first state)
    if let Some(first) = states.first() {
        s.push_str(&format!("    this._current = '{}';\n", first.name));
    }

    s.push_str("  }\n\n");

    // resolveParams: build full param map by walking parent chain
    s.push_str("  _resolveParams(stateName) {\n");
    s.push_str("    const state = this._states[stateName];\n");
    s.push_str("    if (!state) return {};\n");
    s.push_str("    const parentParams = state.parent ? this._resolveParams(state.parent) : {};\n");
    s.push_str("    return Object.assign({}, parentParams, state.overrides);\n");
    s.push_str("  }\n\n");

    // transition(targetState): start a transition
    s.push_str("  transition(targetState, elapsed) {\n");
    s.push_str("    if (!this._states[targetState]) return;\n");
    s.push_str("    if (this._current === targetState && this._target === null) return;\n");
    s.push_str("    const targetDef = this._states[targetState];\n");
    s.push_str(
        "    this._fromParams = this._current ? this._resolveParams(this._current) : {};\n",
    );
    s.push_str("    this._toParams = this._resolveParams(targetState);\n");
    s.push_str("    this._target = targetState;\n");
    s.push_str("    this._transitionStart = elapsed;\n");
    s.push_str("    this._transitionDuration = targetDef.duration;\n");
    s.push_str("    this._transitionEasing = targetDef.easing;\n");
    s.push_str("  }\n\n");

    // evaluate(elapsed): return current param overrides (interpolated if transitioning)
    s.push_str("  evaluate(elapsed) {\n");
    s.push_str("    if (this._target === null) {\n");
    s.push_str("      return this._current ? this._resolveParams(this._current) : {};\n");
    s.push_str("    }\n");
    s.push_str("    const dt = elapsed - this._transitionStart;\n");
    s.push_str("    if (this._transitionDuration <= 0 || dt >= this._transitionDuration) {\n");
    s.push_str("      this._current = this._target;\n");
    s.push_str("      this._target = null;\n");
    s.push_str("      return this._resolveParams(this._current);\n");
    s.push_str("    }\n");
    s.push_str("    let t = dt / this._transitionDuration;\n");
    s.push_str(
        "    const easeFn = _gameStateEasings[this._transitionEasing] || _gameStateEasings.linear;\n",
    );
    s.push_str("    t = easeFn(Math.max(0, Math.min(1, t)));\n");
    s.push_str("    const result = {};\n");
    s.push_str(
        "    const allKeys = new Set([...Object.keys(this._fromParams), ...Object.keys(this._toParams)]);\n",
    );
    s.push_str("    for (const key of allKeys) {\n");
    s.push_str("      const from = this._fromParams[key] ?? 0;\n");
    s.push_str("      const to = this._toParams[key] ?? 0;\n");
    s.push_str("      result[key] = from + (to - from) * t;\n");
    s.push_str("    }\n");
    s.push_str("    return result;\n");
    s.push_str("  }\n\n");

    // currentState: getter
    s.push_str("  get currentState() { return this._current; }\n");
    s.push_str("  get isTransitioning() { return this._target !== null; }\n");

    s.push_str("}\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Duration, Expr, StateBlock, StateOverride};

    #[test]
    fn generates_class_with_states() {
        let states = vec![
            StateBlock {
                name: "idle".into(),
                parent: None,
                transition_duration: None,
                transition_easing: None,
                layers: vec![],
                overrides: vec![],
            },
            StateBlock {
                name: "hover".into(),
                parent: Some("idle".into()),
                transition_duration: Some(Duration::Millis(150.0)),
                transition_easing: Some("ease-out".into()),
                layers: vec![],
                overrides: vec![StateOverride {
                    layer: "glow".into(),
                    param: "intensity".into(),
                    value: Expr::Number(1.2),
                }],
            },
        ];

        let js = generate_state_machine_js(&states);
        assert!(js.contains("class GameStateMachine"));
        assert!(js.contains("'idle'"));
        assert!(js.contains("'hover'"));
        assert!(js.contains("parent: 'idle'"));
        assert!(js.contains("duration: 0.15"));
        assert!(js.contains("easing: 'ease_out'"));
        assert!(js.contains("'glow.intensity': 1.2"));
        assert!(js.contains("transition(targetState"));
        assert!(js.contains("evaluate(elapsed"));
    }

    #[test]
    fn idle_state_has_no_parent() {
        let states = vec![StateBlock {
            name: "idle".into(),
            parent: None,
            transition_duration: None,
            transition_easing: None,
            layers: vec![],
            overrides: vec![],
        }];

        let js = generate_state_machine_js(&states);
        assert!(js.contains("parent: null"));
    }

    #[test]
    fn active_state_chain() {
        let states = vec![
            StateBlock {
                name: "idle".into(),
                parent: None,
                transition_duration: None,
                transition_easing: None,
                layers: vec![],
                overrides: vec![],
            },
            StateBlock {
                name: "hover".into(),
                parent: Some("idle".into()),
                transition_duration: Some(Duration::Millis(150.0)),
                transition_easing: Some("ease-out".into()),
                layers: vec![],
                overrides: vec![],
            },
            StateBlock {
                name: "active".into(),
                parent: Some("hover".into()),
                transition_duration: Some(Duration::Millis(50.0)),
                transition_easing: Some("ease-in".into()),
                layers: vec![],
                overrides: vec![StateOverride {
                    layer: "glow".into(),
                    param: "intensity".into(),
                    value: Expr::Number(0.3),
                }],
            },
        ];

        let js = generate_state_machine_js(&states);
        assert!(js.contains("'active'"));
        assert!(js.contains("parent: 'hover'"));
        assert!(js.contains("duration: 0.05"));
        assert!(js.contains("easing: 'ease_in'"));
    }

    #[test]
    fn easing_functions_emitted() {
        let states = vec![StateBlock {
            name: "hover".into(),
            parent: Some("idle".into()),
            transition_duration: Some(Duration::Seconds(0.3)),
            transition_easing: Some("ease-in-out".into()),
            layers: vec![],
            overrides: vec![],
        }];

        let js = generate_state_machine_js(&states);
        assert!(js.contains("ease_in_out: t =>"));
        assert!(js.contains("_gameStateEasings"));
    }
}
