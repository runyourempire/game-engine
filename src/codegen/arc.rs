//! Arc block codegen — animated parameter sweeps with easing and lifecycle states.
//!
//! Arc is GAME's temporal evolution primitive. It drives parameters through
//! value ranges over time, creating visual systems that develop and transform.
//!
//! ```game
//! arc {
//!   scale: 0.1 -> 1.0 over 3s ease-out
//!   intensity: 0.0 -> 0.8 over 5s ease-in-out
//! }
//!
//! arc enter {
//!   opacity: 0.0 -> 1.0 over 200ms ease-out
//! }
//!
//! arc exit {
//!   opacity: 1.0 -> 0.0 over 300ms ease-in
//! }
//!
//! arc hover {
//!   glow: 0.0 -> 1.0 over 150ms ease-out
//! }
//! ```
//!
//! Generates timeline classes per state:
//! - **Unnamed / idle**: `GameArcTimeline` -- loops continuously (backward compatible)
//! - **enter**: `GameArcEnter` -- plays once on connectedCallback, holds final value
//! - **exit**: `GameArcExit` -- plays once on programmatic trigger, holds final value
//! - **hover**: `GameArcHover` -- plays on mouseenter, reverses on mouseleave

use crate::ast::{ArcBlock, ArcEntry, ArcState, Duration, Expr};

/// Convert a Duration to seconds.
fn duration_to_seconds(d: &Duration) -> f64 {
    match d {
        Duration::Seconds(v) => *v,
        Duration::Millis(v) => *v / 1000.0,
        Duration::Bars(v) => *v as f64 * 2.0, // default 120 BPM
    }
}

/// Convert an Expr to a JS literal.
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
/// Returns the function expression for `t` in [0, 1].
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

/// Collect all unique easing functions from a set of arc entries.
fn collect_easings(entries: &[&ArcEntry]) -> std::collections::HashSet<String> {
    let mut easings = std::collections::HashSet::new();
    for entry in entries {
        if let Some(ref e) = entry.easing {
            easings.insert(e.clone());
        }
    }
    easings
}

/// Emit the shared easing function map.
fn emit_easings(s: &mut String, easings: &std::collections::HashSet<String>) {
    s.push_str("const _gameEasings = {\n");
    s.push_str("  linear: t => t,\n");
    for name in easings {
        let body = easing_fn_js(name);
        let js_name = name.replace('-', "_");
        s.push_str(&format!("  {js_name}: t => {body},\n"));
    }
    s.push_str("};\n\n");
}

/// Emit the entries array for a timeline constructor.
fn emit_entries_array(s: &mut String, entries: &[&ArcEntry]) {
    s.push_str("    this._entries = [\n");
    for entry in entries {
        let from_js = expr_to_js(&entry.from);
        let to_js = expr_to_js(&entry.to);
        let dur_secs = duration_to_seconds(&entry.duration);
        let easing_name = entry
            .easing
            .as_deref()
            .unwrap_or("linear")
            .replace('-', "_");
        s.push_str(&format!(
            "      {{ target: '{}', from: {}, to: {}, duration: {}, easing: '{}' }},\n",
            entry.target, from_js, to_js, dur_secs, easing_name
        ));
    }
    s.push_str("    ];\n");
}

/// Emit the standard evaluate/isComplete/reset/progress methods.
fn emit_timeline_methods(s: &mut String) {
    s.push_str("  evaluate(elapsedSec) {\n");
    s.push_str("    if (this._startTime === null) this._startTime = elapsedSec;\n");
    s.push_str("    const t = elapsedSec - this._startTime;\n");
    s.push_str("    const values = {};\n\n");
    s.push_str("    for (const e of this._entries) {\n");
    s.push_str("      const progress = Math.min(t / e.duration, 1.0);\n");
    s.push_str("      const easeFn = _gameEasings[e.easing] || _gameEasings.linear;\n");
    s.push_str("      const eased = easeFn(progress);\n");
    s.push_str("      values[e.target] = e.from + (e.to - e.from) * eased;\n");
    s.push_str("    }\n\n");
    s.push_str("    return values;\n");
    s.push_str("  }\n\n");

    s.push_str("  isComplete(elapsedSec) {\n");
    s.push_str("    if (this._startTime === null) return false;\n");
    s.push_str("    const t = elapsedSec - this._startTime;\n");
    s.push_str("    return this._entries.every(e => t >= e.duration);\n");
    s.push_str("  }\n\n");

    s.push_str("  reset() { this._startTime = null; }\n\n");

    s.push_str("  progress(elapsedSec) {\n");
    s.push_str("    if (this._startTime === null) return 0;\n");
    s.push_str("    const t = elapsedSec - this._startTime;\n");
    s.push_str("    const maxDur = Math.max(...this._entries.map(e => e.duration));\n");
    s.push_str("    return Math.min(t / maxDur, 1.0);\n");
    s.push_str("  }\n");
}

/// Generate the backward-compatible `GameArcTimeline` class (looping idle).
fn generate_idle_timeline(s: &mut String, entries: &[&ArcEntry]) {
    s.push_str("class GameArcTimeline {\n");
    s.push_str("  constructor() {\n");
    s.push_str("    this._startTime = null;\n");
    emit_entries_array(s, entries);
    s.push_str("  }\n\n");
    emit_timeline_methods(s);
    s.push_str("}\n\n");
}

/// Generate a one-shot arc class (enter or exit) that plays once and holds.
fn generate_oneshot_timeline(s: &mut String, class_name: &str, entries: &[&ArcEntry]) {
    s.push_str(&format!("class {class_name} {{\n"));
    s.push_str("  constructor() {\n");
    s.push_str("    this._startTime = null;\n");
    s.push_str("    this._active = false;\n");
    emit_entries_array(s, entries);
    s.push_str("  }\n\n");

    s.push_str("  play(elapsedSec) {\n");
    s.push_str("    this._startTime = elapsedSec;\n");
    s.push_str("    this._active = true;\n");
    s.push_str("  }\n\n");

    emit_timeline_methods(s);

    s.push_str("  evaluateIfActive(elapsedSec) {\n");
    s.push_str("    if (!this._active) return null;\n");
    s.push_str("    return this.evaluate(elapsedSec);\n");
    s.push_str("  }\n");

    s.push_str("}\n\n");
}

/// Generate a hover arc class that plays forward on enter, reverse on leave.
fn generate_hover_timeline(s: &mut String, entries: &[&ArcEntry]) {
    s.push_str("class GameArcHover {\n");
    s.push_str("  constructor() {\n");
    s.push_str("    this._startTime = null;\n");
    s.push_str("    this._active = false;\n");
    s.push_str("    this._reverse = false;\n");
    s.push_str("    this._holdProgress = 0;\n");
    emit_entries_array(s, entries);
    s.push_str("  }\n\n");

    s.push_str("  enter(elapsedSec) {\n");
    s.push_str("    this._startTime = elapsedSec;\n");
    s.push_str("    this._reverse = false;\n");
    s.push_str("    this._active = true;\n");
    s.push_str("  }\n\n");

    s.push_str("  leave(elapsedSec) {\n");
    s.push_str("    this._startTime = elapsedSec;\n");
    s.push_str("    this._reverse = true;\n");
    s.push_str("    this._active = true;\n");
    s.push_str("  }\n\n");

    s.push_str("  evaluate(elapsedSec) {\n");
    s.push_str("    if (this._startTime === null) this._startTime = elapsedSec;\n");
    s.push_str("    const t = elapsedSec - this._startTime;\n");
    s.push_str("    const values = {};\n\n");
    s.push_str("    for (const e of this._entries) {\n");
    s.push_str("      let progress = Math.min(t / e.duration, 1.0);\n");
    s.push_str("      if (this._reverse) progress = Math.max(1.0 - progress, 0.0);\n");
    s.push_str("      const easeFn = _gameEasings[e.easing] || _gameEasings.linear;\n");
    s.push_str("      const eased = easeFn(progress);\n");
    s.push_str("      values[e.target] = e.from + (e.to - e.from) * eased;\n");
    s.push_str("    }\n\n");
    s.push_str("    return values;\n");
    s.push_str("  }\n\n");

    s.push_str("  evaluateIfActive(elapsedSec) {\n");
    s.push_str("    if (!this._active) return null;\n");
    s.push_str("    return this.evaluate(elapsedSec);\n");
    s.push_str("  }\n\n");

    s.push_str("  isComplete(elapsedSec) {\n");
    s.push_str("    if (this._startTime === null) return false;\n");
    s.push_str("    const t = elapsedSec - this._startTime;\n");
    s.push_str("    return this._entries.every(e => t >= e.duration);\n");
    s.push_str("  }\n\n");

    s.push_str(
        "  reset() { this._startTime = null; this._active = false; this._reverse = false; }\n\n",
    );

    s.push_str("  progress(elapsedSec) {\n");
    s.push_str("    if (this._startTime === null) return 0;\n");
    s.push_str("    const t = elapsedSec - this._startTime;\n");
    s.push_str("    const maxDur = Math.max(...this._entries.map(e => e.duration));\n");
    s.push_str("    let p = Math.min(t / maxDur, 1.0);\n");
    s.push_str("    if (this._reverse) p = 1.0 - p;\n");
    s.push_str("    return p;\n");
    s.push_str("  }\n");

    s.push_str("}\n\n");
}

/// Classify a block's effective state key for grouping.
fn block_state_key(block: &ArcBlock) -> &'static str {
    match &block.state {
        None => "idle",
        Some(ArcState::Enter) => "enter",
        Some(ArcState::Exit) => "exit",
        Some(ArcState::Hover) => "hover",
        Some(ArcState::Idle) => "idle",
    }
}

/// Generate JavaScript for arc-based parameter animation timelines.
///
/// Produces timeline classes grouped by lifecycle state:
/// - Unnamed/idle blocks -> `GameArcTimeline` (backward compatible, looping)
/// - Enter blocks -> `GameArcEnter` (one-shot, auto-plays on connect)
/// - Exit blocks -> `GameArcExit` (one-shot, programmatic trigger)
/// - Hover blocks -> `GameArcHover` (forward on mouseenter, reverse on mouseleave)
pub fn generate_arc_js(blocks: &[ArcBlock]) -> String {
    let all_entries: Vec<_> = blocks.iter().flat_map(|b| b.entries.iter()).collect();
    if all_entries.is_empty() {
        return String::new();
    }

    let idle_entries: Vec<&ArcEntry> = blocks
        .iter()
        .filter(|b| block_state_key(b) == "idle")
        .flat_map(|b| b.entries.iter())
        .collect();
    let enter_entries: Vec<&ArcEntry> = blocks
        .iter()
        .filter(|b| block_state_key(b) == "enter")
        .flat_map(|b| b.entries.iter())
        .collect();
    let exit_entries: Vec<&ArcEntry> = blocks
        .iter()
        .filter(|b| block_state_key(b) == "exit")
        .flat_map(|b| b.entries.iter())
        .collect();
    let hover_entries: Vec<&ArcEntry> = blocks
        .iter()
        .filter(|b| block_state_key(b) == "hover")
        .flat_map(|b| b.entries.iter())
        .collect();

    let has_idle = !idle_entries.is_empty();
    let has_enter = !enter_entries.is_empty();
    let has_exit = !exit_entries.is_empty();
    let has_hover = !hover_entries.is_empty();

    let mut s = String::with_capacity(4096);

    // Collect ALL easings across all states for the shared map
    let easings = collect_easings(&all_entries);
    emit_easings(&mut s, &easings);

    // Generate per-state timeline classes
    if has_idle {
        generate_idle_timeline(&mut s, &idle_entries);
    }
    if has_enter {
        generate_oneshot_timeline(&mut s, "GameArcEnter", &enter_entries);
    }
    if has_exit {
        generate_oneshot_timeline(&mut s, "GameArcExit", &exit_entries);
    }
    if has_hover {
        generate_hover_timeline(&mut s, &hover_entries);
    }

    s
}

/// Returns true if any block in the slice has the given state.
pub fn has_arc_state(blocks: &[ArcBlock], state: &str) -> bool {
    blocks.iter().any(|b| block_state_key(b) == state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    fn make_entry(
        target: &str,
        from: f64,
        to: f64,
        secs: f64,
        easing: Option<&str>,
    ) -> ArcEntry {
        ArcEntry {
            target: target.into(),
            from: Expr::Number(from),
            to: Expr::Number(to),
            duration: Duration::Seconds(secs),
            easing: easing.map(|s| s.into()),
        }
    }

    #[test]
    fn single_arc_generates() {
        let blocks = vec![ArcBlock {
            state: None,
            entries: vec![make_entry("scale", 0.1, 1.0, 3.0, None)],
        }];
        let js = generate_arc_js(&blocks);
        assert!(js.contains("class GameArcTimeline"));
        assert!(js.contains("target: 'scale'"));
        assert!(js.contains("from: 0.1"));
        assert!(js.contains("to: 1"));
        assert!(js.contains("duration: 3"));
    }

    #[test]
    fn easing_function_included() {
        let blocks = vec![ArcBlock {
            state: None,
            entries: vec![make_entry("x", 0.0, 1.0, 2.0, Some("ease-out"))],
        }];
        let js = generate_arc_js(&blocks);
        assert!(js.contains("ease_out: t =>"));
        assert!(js.contains("easing: 'ease_out'"));
    }

    #[test]
    fn multiple_easings_collected() {
        let blocks = vec![ArcBlock {
            state: None,
            entries: vec![
                make_entry("a", 0.0, 1.0, 1.0, Some("ease-in")),
                make_entry("b", 0.0, 1.0, 2.0, Some("elastic")),
            ],
        }];
        let js = generate_arc_js(&blocks);
        assert!(js.contains("ease_in: t =>"));
        assert!(js.contains("elastic: t =>"));
    }

    #[test]
    fn empty_blocks_produce_nothing() {
        let blocks: Vec<ArcBlock> = vec![];
        let js = generate_arc_js(&blocks);
        assert!(js.is_empty());
    }

    #[test]
    fn empty_entries_produce_nothing() {
        let blocks = vec![ArcBlock {
            state: None,
            entries: vec![],
        }];
        let js = generate_arc_js(&blocks);
        assert!(js.is_empty());
    }

    #[test]
    fn evaluate_method_exists() {
        let blocks = vec![ArcBlock {
            state: None,
            entries: vec![make_entry("x", 0.0, 1.0, 1.0, None)],
        }];
        let js = generate_arc_js(&blocks);
        assert!(js.contains("evaluate(elapsedSec)"));
        assert!(js.contains("return values"));
    }

    #[test]
    fn is_complete_and_reset() {
        let blocks = vec![ArcBlock {
            state: None,
            entries: vec![make_entry("x", 0.0, 1.0, 1.0, None)],
        }];
        let js = generate_arc_js(&blocks);
        assert!(js.contains("isComplete(elapsedSec)"));
        assert!(js.contains("reset()"));
        assert!(js.contains("progress(elapsedSec)"));
    }

    #[test]
    fn millis_duration_converted() {
        let blocks = vec![ArcBlock {
            state: None,
            entries: vec![ArcEntry {
                target: "x".into(),
                from: Expr::Number(0.0),
                to: Expr::Number(1.0),
                duration: Duration::Millis(500.0),
                easing: None,
            }],
        }];
        let js = generate_arc_js(&blocks);
        assert!(js.contains("duration: 0.5"));
    }

    #[test]
    fn bars_duration_converted() {
        let blocks = vec![ArcBlock {
            state: None,
            entries: vec![ArcEntry {
                target: "x".into(),
                from: Expr::Number(0.0),
                to: Expr::Number(1.0),
                duration: Duration::Bars(4),
                easing: None,
            }],
        }];
        let js = generate_arc_js(&blocks);
        assert!(js.contains("duration: 8")); // 4 bars * 2s/bar at 120BPM
    }

    #[test]
    fn multi_block_merges_entries() {
        let blocks = vec![
            ArcBlock {
                state: None,
                entries: vec![make_entry("a", 0.0, 1.0, 1.0, None)],
            },
            ArcBlock {
                state: None,
                entries: vec![make_entry("b", 1.0, 0.0, 2.0, None)],
            },
        ];
        let js = generate_arc_js(&blocks);
        assert!(js.contains("target: 'a'"));
        assert!(js.contains("target: 'b'"));
    }

    #[test]
    fn bounce_easing_generates() {
        let blocks = vec![ArcBlock {
            state: None,
            entries: vec![make_entry("y", 0.0, 100.0, 2.0, Some("bounce"))],
        }];
        let js = generate_arc_js(&blocks);
        assert!(js.contains("bounce: t =>"));
        assert!(js.contains("7.5625"));
    }

    // -- Lifecycle state tests --

    #[test]
    fn enter_arc_generates_oneshot_class() {
        let blocks = vec![ArcBlock {
            state: Some(ArcState::Enter),
            entries: vec![make_entry("opacity", 0.0, 1.0, 0.2, Some("ease-out"))],
        }];
        let js = generate_arc_js(&blocks);
        assert!(js.contains("class GameArcEnter"));
        assert!(js.contains("play(elapsedSec)"));
        assert!(js.contains("evaluateIfActive(elapsedSec)"));
        assert!(js.contains("target: 'opacity'"));
        assert!(!js.contains("class GameArcTimeline"));
    }

    #[test]
    fn exit_arc_generates_oneshot_class() {
        let blocks = vec![ArcBlock {
            state: Some(ArcState::Exit),
            entries: vec![make_entry("opacity", 1.0, 0.0, 0.3, Some("ease-in"))],
        }];
        let js = generate_arc_js(&blocks);
        assert!(js.contains("class GameArcExit"));
        assert!(js.contains("play(elapsedSec)"));
        assert!(!js.contains("class GameArcTimeline"));
    }

    #[test]
    fn hover_arc_generates_bidirectional_class() {
        let blocks = vec![ArcBlock {
            state: Some(ArcState::Hover),
            entries: vec![make_entry("glow", 0.0, 1.0, 0.15, Some("ease-out"))],
        }];
        let js = generate_arc_js(&blocks);
        assert!(js.contains("class GameArcHover"));
        assert!(js.contains("enter(elapsedSec)"));
        assert!(js.contains("leave(elapsedSec)"));
        assert!(js.contains("this._reverse"));
        assert!(!js.contains("class GameArcTimeline"));
    }

    #[test]
    fn idle_state_uses_standard_timeline() {
        let blocks = vec![ArcBlock {
            state: Some(ArcState::Idle),
            entries: vec![make_entry("x", 0.0, 1.0, 2.0, None)],
        }];
        let js = generate_arc_js(&blocks);
        assert!(js.contains("class GameArcTimeline"));
    }

    #[test]
    fn mixed_states_generate_multiple_classes() {
        let blocks = vec![
            ArcBlock {
                state: None,
                entries: vec![make_entry("scale", 0.5, 1.5, 4.0, None)],
            },
            ArcBlock {
                state: Some(ArcState::Enter),
                entries: vec![make_entry("opacity", 0.0, 1.0, 0.2, Some("ease-out"))],
            },
            ArcBlock {
                state: Some(ArcState::Exit),
                entries: vec![make_entry("opacity", 1.0, 0.0, 0.3, None)],
            },
            ArcBlock {
                state: Some(ArcState::Hover),
                entries: vec![make_entry("glow", 0.0, 1.0, 0.15, None)],
            },
        ];
        let js = generate_arc_js(&blocks);
        assert!(js.contains("class GameArcTimeline"));
        assert!(js.contains("class GameArcEnter"));
        assert!(js.contains("class GameArcExit"));
        assert!(js.contains("class GameArcHover"));
        // Single shared easing map
        assert_eq!(js.matches("const _gameEasings").count(), 1);
    }

    #[test]
    fn has_arc_state_helper() {
        let blocks = vec![
            ArcBlock {
                state: None,
                entries: vec![make_entry("x", 0.0, 1.0, 1.0, None)],
            },
            ArcBlock {
                state: Some(ArcState::Enter),
                entries: vec![make_entry("y", 0.0, 1.0, 0.2, None)],
            },
        ];
        assert!(has_arc_state(&blocks, "idle"));
        assert!(has_arc_state(&blocks, "enter"));
        assert!(!has_arc_state(&blocks, "exit"));
        assert!(!has_arc_state(&blocks, "hover"));
    }
}
