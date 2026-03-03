//! Resonate block codegen — parametric coupling network between layers.
//!
//! Resonate is GAME's emergent behavior primitive. It creates a directed graph
//! of weighted couplings between parameters: when a source value changes,
//! connected targets respond proportionally.
//!
//! ```game
//! resonate {
//!   bass → core.scale * 0.3
//!   energy → noise_field.intensity * 0.5
//!   core.brightness → outer_ring.opacity * 0.7
//! }
//! ```
//!
//! This generates a `GameResonanceNetwork` JS class that:
//! 1. Maintains a coupling graph as adjacency list
//! 2. Each frame: reads source values, computes weighted influence, writes to targets
//! 3. Supports damping to prevent runaway feedback loops
//! 4. Propagates influence through multi-hop chains (source → A → B cascades)

use crate::ast::{Expr, ResonateBlock};

/// Default damping factor for resonance propagation.
/// Prevents runaway feedback — each hop attenuates by this factor.
const DEFAULT_DAMPING: f64 = 0.95;

/// Maximum propagation depth per frame to prevent infinite loops.
const MAX_PROPAGATION_DEPTH: u32 = 4;

/// Generate JavaScript for a resonance coupling network.
///
/// Produces a `GameResonanceNetwork` class that manages weighted
/// parameter-to-parameter couplings with feedback damping.
pub fn generate_resonate_js(blocks: &[ResonateBlock]) -> String {
    let entries: Vec<_> = blocks.iter().flat_map(|b| b.entries.iter()).collect();
    if entries.is_empty() {
        return String::new();
    }

    let mut s = String::with_capacity(2048);

    s.push_str("class GameResonanceNetwork {\n");

    // Constructor: build coupling graph
    s.push_str("  constructor() {\n");
    s.push_str("    this._couplings = [\n");

    for entry in &entries {
        let weight_js = expr_to_js(&entry.weight);
        s.push_str(&format!(
            "      {{ source: '{}', target: '{}', field: '{}', weight: {} }},\n",
            entry.source, entry.target, entry.field, weight_js
        ));
    }

    s.push_str("    ];\n");
    s.push_str(&format!("    this._damping = {};\n", DEFAULT_DAMPING));
    s.push_str(&format!(
        "    this._maxDepth = {};\n",
        MAX_PROPAGATION_DEPTH
    ));
    s.push_str("    this._state = new Map();\n");
    s.push_str("    this._deltas = new Map();\n");
    s.push_str("  }\n\n");

    // propagate(uniforms): core resonance loop
    // Takes a uniforms map, applies coupling influences, returns updated map.
    s.push_str("  propagate(uniforms) {\n");
    s.push_str("    // Snapshot current values\n");
    s.push_str("    const prev = new Map(this._state);\n");
    s.push_str("    for (const [k, v] of Object.entries(uniforms)) {\n");
    s.push_str("      this._state.set(k, v);\n");
    s.push_str("    }\n\n");

    s.push_str("    // Compute deltas from source changes\n");
    s.push_str("    this._deltas.clear();\n");
    s.push_str("    for (const c of this._couplings) {\n");
    s.push_str("      const srcKey = c.source;\n");
    s.push_str("      const curVal = this._state.get(srcKey) ?? 0;\n");
    s.push_str("      const prevVal = prev.get(srcKey) ?? curVal;\n");
    s.push_str("      const delta = (curVal - prevVal) * c.weight;\n");
    s.push_str("      if (Math.abs(delta) > 0.0001) {\n");
    s.push_str("        const tgtKey = `${c.target}.${c.field}`;\n");
    s.push_str("        this._deltas.set(tgtKey, (this._deltas.get(tgtKey) ?? 0) + delta);\n");
    s.push_str("      }\n");
    s.push_str("    }\n\n");

    s.push_str("    // Apply damped deltas to uniforms\n");
    s.push_str("    const result = { ...uniforms };\n");
    s.push_str("    for (const [key, delta] of this._deltas) {\n");
    s.push_str("      const parts = key.split('.');\n");
    s.push_str("      const paramName = parts.length > 1 ? parts[1] : parts[0];\n");
    s.push_str("      if (paramName in result) {\n");
    s.push_str("        result[paramName] += delta * this._damping;\n");
    s.push_str("      }\n");
    s.push_str("    }\n\n");

    s.push_str("    // Multi-hop cascade (depth-limited)\n");
    s.push_str("    for (let depth = 1; depth < this._maxDepth; depth++) {\n");
    s.push_str("      let anyChange = false;\n");
    s.push_str("      for (const c of this._couplings) {\n");
    s.push_str("        const tgtKey = `${c.target}.${c.field}`;\n");
    s.push_str("        const srcDelta = this._deltas.get(c.source) ?? 0;\n");
    s.push_str("        if (Math.abs(srcDelta) > 0.0001) {\n");
    s.push_str(
        "          const cascadeDelta = srcDelta * c.weight * Math.pow(this._damping, depth);\n",
    );
    s.push_str(
        "          this._deltas.set(tgtKey, (this._deltas.get(tgtKey) ?? 0) + cascadeDelta);\n",
    );
    s.push_str("          const parts = tgtKey.split('.');\n");
    s.push_str("          const pn = parts.length > 1 ? parts[1] : parts[0];\n");
    s.push_str("          if (pn in result) { result[pn] += cascadeDelta; anyChange = true; }\n");
    s.push_str("        }\n");
    s.push_str("      }\n");
    s.push_str("      if (!anyChange) break;\n");
    s.push_str("    }\n\n");

    s.push_str("    // Update state for next frame\n");
    s.push_str("    for (const [k, v] of Object.entries(result)) {\n");
    s.push_str("      this._state.set(k, v);\n");
    s.push_str("    }\n\n");

    s.push_str("    return result;\n");
    s.push_str("  }\n\n");

    // Diagnostic: get coupling graph for visualization
    s.push_str("  get couplings() { return this._couplings; }\n");
    s.push_str("  get activeDeltas() { return Object.fromEntries(this._deltas); }\n");

    s.push_str("}\n");

    s
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
            };
            format!("({l} {op_str} {r})")
        }
        Expr::Neg(inner) => format!("(-{})", expr_to_js(inner)),
        _ => "0".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    fn make_entry(source: &str, target: &str, field: &str, weight: f64) -> ResonateEntry {
        ResonateEntry {
            source: source.into(),
            target: target.into(),
            field: field.into(),
            weight: Expr::Number(weight),
        }
    }

    #[test]
    fn single_coupling_generates() {
        let blocks = vec![ResonateBlock {
            entries: vec![make_entry("bass", "core", "scale", 0.3)],
        }];
        let js = generate_resonate_js(&blocks);
        assert!(js.contains("class GameResonanceNetwork"));
        assert!(js.contains("source: 'bass'"));
        assert!(js.contains("target: 'core'"));
        assert!(js.contains("field: 'scale'"));
        assert!(js.contains("weight: 0.3"));
    }

    #[test]
    fn multi_coupling_merges_blocks() {
        let blocks = vec![
            ResonateBlock {
                entries: vec![make_entry("bass", "core", "scale", 0.3)],
            },
            ResonateBlock {
                entries: vec![make_entry("energy", "noise", "intensity", 0.5)],
            },
        ];
        let js = generate_resonate_js(&blocks);
        assert!(js.contains("'bass'"));
        assert!(js.contains("'energy'"));
    }

    #[test]
    fn empty_blocks_produce_nothing() {
        let blocks: Vec<ResonateBlock> = vec![];
        let js = generate_resonate_js(&blocks);
        assert!(js.is_empty());
    }

    #[test]
    fn empty_entries_produce_nothing() {
        let blocks = vec![ResonateBlock { entries: vec![] }];
        let js = generate_resonate_js(&blocks);
        assert!(js.is_empty());
    }

    #[test]
    fn propagate_method_exists() {
        let blocks = vec![ResonateBlock {
            entries: vec![make_entry("a", "b", "x", 1.0)],
        }];
        let js = generate_resonate_js(&blocks);
        assert!(js.contains("propagate(uniforms)"));
        assert!(js.contains("return result"));
    }

    #[test]
    fn damping_and_cascade_present() {
        let blocks = vec![ResonateBlock {
            entries: vec![make_entry("a", "b", "x", 0.5)],
        }];
        let js = generate_resonate_js(&blocks);
        assert!(js.contains("this._damping"));
        assert!(js.contains("this._maxDepth"));
        assert!(js.contains("Multi-hop cascade"));
    }

    #[test]
    fn expr_weight_renders_correctly() {
        let blocks = vec![ResonateBlock {
            entries: vec![ResonateEntry {
                source: "a".into(),
                target: "b".into(),
                field: "y".into(),
                weight: Expr::BinOp {
                    op: BinOp::Mul,
                    left: Box::new(Expr::Number(0.5)),
                    right: Box::new(Expr::Ident("intensity".into())),
                },
            }],
        }];
        let js = generate_resonate_js(&blocks);
        assert!(js.contains("(0.5 * intensity)"));
    }

    #[test]
    fn diagnostics_exposed() {
        let blocks = vec![ResonateBlock {
            entries: vec![make_entry("a", "b", "x", 1.0)],
        }];
        let js = generate_resonate_js(&blocks);
        assert!(js.contains("get couplings()"));
        assert!(js.contains("get activeDeltas()"));
    }
}
