//! Arc block codegen — animated parameter sweeps with easing.
//!
//! Arc is GAME's temporal evolution primitive. It drives parameters through
//! value ranges over time, creating visual systems that develop and transform.
//!
//! ```game
//! arc {
//!   scale: 0.1 -> 1.0 over 3s ease-out
//!   intensity: 0.0 -> 0.8 over 5s ease-in-out
//!   hue: 0.0 -> 360.0 over 10s
//! }
//! ```
//!
//! Generates a `GameArcTimeline` JS class that:
//! 1. Manages a list of parameter animations with start/end values
//! 2. Each frame: computes progress, applies easing, lerps values
//! 3. Supports standard easing functions (linear, ease-in, ease-out, etc.)
//! 4. Animations complete and hold at final value (one-shot, not looping)
//! 5. Exposes `isComplete` for composition with score/breed systems

use crate::ast::{ArcBlock, Duration, Expr};

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

/// Generate JavaScript for arc-based parameter animation timeline.
///
/// Produces a `GameArcTimeline` class that manages value sweeps
/// with easing and temporal progression.
pub fn generate_arc_js(blocks: &[ArcBlock]) -> String {
    let entries: Vec<_> = blocks.iter().flat_map(|b| b.entries.iter()).collect();
    if entries.is_empty() {
        return String::new();
    }

    let mut s = String::with_capacity(2048);

    // Collect unique easing functions needed
    let mut easings: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in &entries {
        if let Some(ref e) = entry.easing {
            easings.insert(e.clone());
        }
    }

    // Emit easing function map
    s.push_str("const _gameEasings = {\n");
    s.push_str("  linear: t => t,\n");
    for name in &easings {
        let body = easing_fn_js(name);
        // Normalize name for JS (replace hyphens with underscores)
        let js_name = name.replace('-', "_");
        s.push_str(&format!("  {js_name}: t => {body},\n"));
    }
    s.push_str("};\n\n");

    s.push_str("class GameArcTimeline {\n");

    // Constructor: build animation entries
    s.push_str("  constructor() {\n");
    s.push_str("    this._startTime = null;\n");
    s.push_str("    this._entries = [\n");

    for entry in &entries {
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
    s.push_str("  }\n\n");

    // evaluate(elapsed): compute current values for all arc entries
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

    // isComplete: true when all animations have finished
    s.push_str("  isComplete(elapsedSec) {\n");
    s.push_str("    if (this._startTime === null) return false;\n");
    s.push_str("    const t = elapsedSec - this._startTime;\n");
    s.push_str("    return this._entries.every(e => t >= e.duration);\n");
    s.push_str("  }\n\n");

    // reset: restart the timeline
    s.push_str("  reset() { this._startTime = null; }\n\n");

    // progress: 0..1 overall completion
    s.push_str("  progress(elapsedSec) {\n");
    s.push_str("    if (this._startTime === null) return 0;\n");
    s.push_str("    const t = elapsedSec - this._startTime;\n");
    s.push_str("    const maxDur = Math.max(...this._entries.map(e => e.duration));\n");
    s.push_str("    return Math.min(t / maxDur, 1.0);\n");
    s.push_str("  }\n");

    s.push_str("}\n");

    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    fn make_entry(target: &str, from: f64, to: f64, secs: f64, easing: Option<&str>) -> ArcEntry {
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
            entries: vec![make_entry("x", 0.0, 1.0, 2.0, Some("ease-out"))],
        }];
        let js = generate_arc_js(&blocks);
        assert!(js.contains("ease_out: t =>"));
        assert!(js.contains("easing: 'ease_out'"));
    }

    #[test]
    fn multiple_easings_collected() {
        let blocks = vec![ArcBlock {
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
        let blocks = vec![ArcBlock { entries: vec![] }];
        let js = generate_arc_js(&blocks);
        assert!(js.is_empty());
    }

    #[test]
    fn evaluate_method_exists() {
        let blocks = vec![ArcBlock {
            entries: vec![make_entry("x", 0.0, 1.0, 1.0, None)],
        }];
        let js = generate_arc_js(&blocks);
        assert!(js.contains("evaluate(elapsedSec)"));
        assert!(js.contains("return values"));
    }

    #[test]
    fn is_complete_and_reset() {
        let blocks = vec![ArcBlock {
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
                entries: vec![make_entry("a", 0.0, 1.0, 1.0, None)],
            },
            ArcBlock {
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
            entries: vec![make_entry("y", 0.0, 100.0, 2.0, Some("bounce"))],
        }];
        let js = generate_arc_js(&blocks);
        assert!(js.contains("bounce: t =>"));
        assert!(js.contains("7.5625"));
    }
}
