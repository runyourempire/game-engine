//! Matrix block codegen — coupling, color, and transition matrices.
//!
//! Matrix is GAME's structured interconnection primitive. It provides three
//! forms for different use cases:
//!
//! 1. **Coupling matrix** — bidirectional NxM parameter coupling grid.
//!    Generates a `GameCouplingMatrix` JS class that propagates weighted
//!    influence between sources and targets with damping and multi-hop cascade.
//!
//! 2. **Color matrix** — 3x3 RGB color grading transform.
//!    Generates a WGSL/GLSL function snippet injected into the fragment shader.
//!    User writes row-major; codegen transposes to WGSL column-major `mat3x3f`.
//!
//! 3. **Transition matrix** — Markov chain probabilistic scene sequencing.
//!    Generates a `GameTransitionMatrix` JS class with weighted random state
//!    transitions, hold timers, and history tracking.
//!
//! ```game
//! matrix coupling {
//!   sources: bass, treble, energy
//!   targets: core.scale, noise.intensity, ring.opacity
//!   weights: [
//!     0.3, 0.1, 0.0,
//!     0.0, 0.5, 0.2,
//!     0.1, 0.0, 0.7
//!   ]
//!   damping: 0.92
//!   depth: 3
//! }
//!
//! matrix color {
//!   1.2, -0.1,  0.0,
//!   0.1,  1.1, -0.05,
//!  -0.05, 0.0,  1.3
//! }
//!
//! matrix transitions "flow" {
//!   states: intro, build, peak, resolve
//!   weights: [
//!     0.0, 0.7, 0.2, 0.1,
//!     0.0, 0.0, 0.8, 0.2,
//!     0.0, 0.1, 0.0, 0.9,
//!     0.6, 0.0, 0.0, 0.4
//!   ]
//!   hold: 5s
//! }
//! ```

use crate::ast::{Duration, MatrixColor, MatrixCoupling, MatrixTransitions};

/// Default damping factor for coupling propagation.
const DEFAULT_COUPLING_DAMPING: f64 = 0.92;

/// Default maximum cascade depth for multi-hop propagation.
const DEFAULT_COUPLING_DEPTH: u32 = 3;

/// Convert a Duration to seconds.
fn duration_to_seconds(d: &Duration) -> f64 {
    match d {
        Duration::Seconds(v) => *v,
        Duration::Millis(v) => *v / 1000.0,
        Duration::Bars(v) => *v as f64 * 2.0, // default 120 BPM
    }
}

// ---------------------------------------------------------------------------
// 1. Coupling matrix codegen
// ---------------------------------------------------------------------------

/// Generate JavaScript for a coupling matrix.
///
/// Produces a `GameCouplingMatrix` class that stores an NxM weight grid,
/// propagates source changes through the matrix to targets, and supports
/// multi-hop cascade with configurable damping.
pub fn generate_coupling_js(matrix: &MatrixCoupling) -> String {
    if matrix.sources.is_empty() || matrix.targets.is_empty() {
        return String::new();
    }

    let num_sources = matrix.sources.len();
    let num_targets = matrix.targets.len();
    let damping = if matrix.damping > 0.0 {
        matrix.damping
    } else {
        DEFAULT_COUPLING_DAMPING
    };
    let depth = if matrix.depth > 0 {
        matrix.depth
    } else {
        DEFAULT_COUPLING_DEPTH
    };

    let mut s = String::with_capacity(2048);

    s.push_str("class GameCouplingMatrix {\n");

    // Constructor
    s.push_str("  constructor() {\n");

    // Sources array
    s.push_str("    this._sources = [");
    for (i, src) in matrix.sources.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        s.push('\'');
        s.push_str(src);
        s.push('\'');
    }
    s.push_str("];\n");

    // Targets array
    s.push_str("    this._targets = [\n");
    for (i, tgt) in matrix.targets.iter().enumerate() {
        s.push_str("      { layer: '");
        s.push_str(&tgt.layer);
        s.push_str("', field: '");
        s.push_str(&tgt.field);
        s.push_str("' }");
        if i < num_targets - 1 {
            s.push(',');
        }
        s.push('\n');
    }
    s.push_str("    ];\n");

    // Weight matrix (targets x sources)
    s.push_str("    this._weights = [\n");
    for t in 0..num_targets {
        s.push_str("      [");
        for ss in 0..num_sources {
            let idx = t * num_sources + ss;
            let w = matrix.weights.get(idx).copied().unwrap_or(0.0);
            if ss > 0 {
                s.push_str(", ");
            }
            s.push_str(&format!("{w}"));
        }
        s.push(']');
        if t < num_targets - 1 {
            s.push(',');
        }
        s.push('\n');
    }
    s.push_str("    ];\n");

    s.push_str(&format!("    this._damping = {};\n", damping));
    s.push_str(&format!("    this._maxDepth = {};\n", depth));
    s.push_str("    this._state = new Map();\n");
    s.push_str("  }\n\n");

    // propagate(uniforms)
    s.push_str("  propagate(uniforms) {\n");
    s.push_str("    const prev = new Map(this._state);\n");
    s.push_str("    for (const [k, v] of Object.entries(uniforms)) {\n");
    s.push_str("      this._state.set(k, v);\n");
    s.push_str("    }\n\n");

    s.push_str("    const result = { ...uniforms };\n\n");

    // Matrix multiplication: each target = sum(source_i * weight[target_idx][source_idx])
    s.push_str("    // Matrix multiplication: target[t] += sum(delta[s] * weight[t][s])\n");
    s.push_str("    for (let t = 0; t < this._targets.length; t++) {\n");
    s.push_str("      let influence = 0;\n");
    s.push_str("      for (let s = 0; s < this._sources.length; s++) {\n");
    s.push_str("        const srcKey = this._sources[s];\n");
    s.push_str("        const curVal = this._state.get(srcKey) ?? 0;\n");
    s.push_str("        const prevVal = prev.get(srcKey) ?? curVal;\n");
    s.push_str("        const delta = curVal - prevVal;\n");
    s.push_str("        influence += delta * this._weights[t][s];\n");
    s.push_str("      }\n\n");

    s.push_str("      if (Math.abs(influence) > 0.0001) {\n");
    s.push_str("        const tgt = this._targets[t];\n");
    s.push_str("        const paramName = tgt.field;\n");
    s.push_str("        if (paramName in result) {\n");
    s.push_str("          result[paramName] += influence * this._damping;\n");
    s.push_str("        }\n");
    s.push_str("      }\n");
    s.push_str("    }\n\n");

    // Cascade decay — applies progressive damping to smooth parameter changes
    s.push_str("    // Cascade decay — progressive damping for smooth transitions\n");
    s.push_str("    for (let d = 1; d < this._maxDepth; d++) {\n");
    s.push_str("      let anyChange = false;\n");
    s.push_str("      for (let t = 0; t < this._targets.length; t++) {\n");
    s.push_str("        let cascadeInfluence = 0;\n");
    s.push_str("        for (let s = 0; s < this._sources.length; s++) {\n");
    s.push_str("          const srcKey = this._sources[s];\n");
    s.push_str("          const curVal = this._state.get(srcKey) ?? 0;\n");
    s.push_str("          const prevVal = prev.get(srcKey) ?? curVal;\n");
    s.push_str("          const delta = curVal - prevVal;\n");
    s.push_str("          if (Math.abs(delta) > 0.0001) {\n");
    s.push_str(
        "            cascadeInfluence += delta * this._weights[t][s] * Math.pow(this._damping, d);\n",
    );
    s.push_str("            anyChange = true;\n");
    s.push_str("          }\n");
    s.push_str("        }\n");
    s.push_str("        if (Math.abs(cascadeInfluence) > 0.0001) {\n");
    s.push_str("          const tgt = this._targets[t];\n");
    s.push_str("          if (tgt.field in result) {\n");
    s.push_str("            result[tgt.field] += cascadeInfluence;\n");
    s.push_str("          }\n");
    s.push_str("        }\n");
    s.push_str("      }\n");
    s.push_str("      if (!anyChange) break;\n");
    s.push_str("    }\n\n");

    // Update state
    s.push_str("    for (const [k, v] of Object.entries(result)) {\n");
    s.push_str("      this._state.set(k, v);\n");
    s.push_str("    }\n");
    s.push_str("    return result;\n");
    s.push_str("  }\n\n");

    // reset()
    s.push_str("  reset() { this._state = new Map(); }\n\n");

    // Diagnostic accessors
    s.push_str("  get weights() { return this._weights; }\n");
    s.push_str("  get activeInfluence() { return Object.fromEntries(this._state); }\n");

    s.push_str("}\n");

    s
}

// ---------------------------------------------------------------------------
// 2. Color matrix codegen
// ---------------------------------------------------------------------------

/// Generate a WGSL snippet that applies a 3x3 color transform.
///
/// The user writes values in row-major order (natural reading):
///   `[a, b, c, d, e, f, g, h, i]`
///
/// WGSL `mat3x3f` is column-major, so we transpose:
///   column 0 = [a, d, g], column 1 = [b, e, h], column 2 = [c, f, i]
pub fn generate_color_matrix_wgsl(matrix: &MatrixColor) -> String {
    let v = &matrix.values;

    let mut s = String::with_capacity(512);

    s.push_str("fn apply_color_matrix(color: vec3f) -> vec3f {\n");
    s.push_str("    let m = mat3x3f(\n");

    // Column 0: row0[0], row1[0], row2[0] = v[0], v[3], v[6]
    s.push_str(&format!("        vec3f({}, {}, {}),\n", v[0], v[3], v[6]));
    // Column 1: row0[1], row1[1], row2[1] = v[1], v[4], v[7]
    s.push_str(&format!("        vec3f({}, {}, {}),\n", v[1], v[4], v[7]));
    // Column 2: row0[2], row1[2], row2[2] = v[2], v[5], v[8]
    s.push_str(&format!("        vec3f({}, {}, {})\n", v[2], v[5], v[8]));

    s.push_str("    );\n");
    s.push_str("    return clamp(m * color, vec3f(0.0), vec3f(1.0));\n");
    s.push_str("}\n");

    s
}

/// Generate a GLSL snippet that applies a 3x3 color transform.
///
/// GLSL `mat3` is also column-major like WGSL, so the same transpose applies.
pub fn generate_color_matrix_glsl(matrix: &MatrixColor) -> String {
    let v = &matrix.values;

    let mut s = String::with_capacity(512);

    s.push_str("vec3 apply_color_matrix(vec3 color) {\n");
    s.push_str("    mat3 m = mat3(\n");

    // Column 0: v[0], v[3], v[6]
    s.push_str(&format!("        vec3({}, {}, {}),\n", v[0], v[3], v[6]));
    // Column 1: v[1], v[4], v[7]
    s.push_str(&format!("        vec3({}, {}, {}),\n", v[1], v[4], v[7]));
    // Column 2: v[2], v[5], v[8]
    s.push_str(&format!("        vec3({}, {}, {})\n", v[2], v[5], v[8]));

    s.push_str("    );\n");
    s.push_str("    return clamp(m * color, vec3(0.0), vec3(1.0));\n");
    s.push_str("}\n");

    s
}

// ---------------------------------------------------------------------------
// 3. Transition matrix codegen
// ---------------------------------------------------------------------------

/// Generate JavaScript for a Markov chain transition matrix.
///
/// Produces a `GameTransitionMatrix_{name}` class with NxN probability
/// matrix, weighted random state selection, hold timers, and history tracking.
pub fn generate_transition_js(matrix: &MatrixTransitions) -> String {
    if matrix.states.is_empty() {
        return String::new();
    }

    let num_states = matrix.states.len();
    let hold_seconds = duration_to_seconds(&matrix.hold);
    let class_name = matrix.name.replace('-', "_");

    let mut s = String::with_capacity(2048);

    s.push_str(&format!("class GameTransitionMatrix_{class_name} {{\n"));

    // Constructor
    s.push_str("  constructor() {\n");

    // States array
    s.push_str("    this._states = [");
    for (i, state) in matrix.states.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        s.push('\'');
        s.push_str(state);
        s.push('\'');
    }
    s.push_str("];\n");

    // NxN probability matrix
    s.push_str("    this._matrix = [\n");
    for row in 0..num_states {
        s.push_str("      [");
        for col in 0..num_states {
            let idx = row * num_states + col;
            let w = matrix.weights.get(idx).copied().unwrap_or(0.0);
            if col > 0 {
                s.push_str(", ");
            }
            s.push_str(&format!("{w}"));
        }
        s.push(']');
        if row < num_states - 1 {
            s.push(',');
        }
        s.push('\n');
    }
    s.push_str("    ];\n");

    s.push_str(&format!("    this._holdSeconds = {};\n", hold_seconds));
    s.push_str("    this._current = 0;\n");
    s.push_str("    this._lastTransition = null;\n");
    s.push_str("    this._history = [];\n");
    s.push_str("  }\n\n");

    // next(): weighted random state selection from current row
    s.push_str("  next() {\n");
    s.push_str("    const row = this._matrix[this._current];\n");
    s.push_str("    let r = Math.random();\n");
    s.push_str("    for (let i = 0; i < row.length; i++) {\n");
    s.push_str("      r -= row[i];\n");
    s.push_str("      if (r <= 0) {\n");
    s.push_str("        this._history.push({ from: this._current, to: i, time: performance.now() / 1000 });\n");
    s.push_str("        this._current = i;\n");
    s.push_str("        return this._states[i];\n");
    s.push_str("      }\n");
    s.push_str("    }\n");
    s.push_str("    return this._states[this._current];\n");
    s.push_str("  }\n\n");

    // evaluate(elapsed): manages hold timer and triggers transitions
    s.push_str("  evaluate(elapsed) {\n");
    s.push_str("    if (this._lastTransition === null) {\n");
    s.push_str("      this._lastTransition = elapsed;\n");
    s.push_str("      return { state: this._states[0], progress: 0, changed: true };\n");
    s.push_str("    }\n");
    s.push_str("    const dt = elapsed - this._lastTransition;\n");
    s.push_str("    if (dt >= this._holdSeconds) {\n");
    s.push_str("      this._lastTransition = elapsed;\n");
    s.push_str("      const nextState = this.next();\n");
    s.push_str("      return { state: nextState, progress: 0, changed: true };\n");
    s.push_str("    }\n");
    s.push_str("    return { state: this._states[this._current], progress: dt / this._holdSeconds, changed: false };\n");
    s.push_str("  }\n\n");

    // Diagnostic accessors
    s.push_str("  get currentState() { return this._states[this._current]; }\n");
    s.push_str("  get stateIndex() { return this._current; }\n");
    s.push_str("  get history() { return this._history; }\n");
    s.push_str("  get matrix() { return this._matrix; }\n");

    s.push_str("}\n");

    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    /// Convert an Expr to a JS literal string (test-only utility).
    fn expr_to_js(e: &Expr) -> String {
        match e {
            Expr::Number(v) => format!("{v}"),
            Expr::Ident(name) => name.clone(),
            Expr::DottedIdent { object, field } => format!("{object}.{field}"),
            Expr::BinOp { op, left, right } => {
                let l = expr_to_js(left);
                let r = expr_to_js(right);
                let op_str = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "/",
                    BinOp::Pow => "**",
                    BinOp::Gt => ">",
                    BinOp::Lt => "<",
                    BinOp::Gte => ">=",
                    BinOp::Lte => "<=",
                    BinOp::Eq => "===",
                    BinOp::NotEq => "!==",
                };
                format!("({l} {op_str} {r})")
            }
            Expr::Neg(inner) => format!("(-{})", expr_to_js(inner)),
            _ => "0".into(),
        }
    }

    // -----------------------------------------------------------------------
    // Coupling matrix tests
    // -----------------------------------------------------------------------

    fn make_target(layer: &str, field: &str) -> MatrixTarget {
        MatrixTarget {
            layer: layer.into(),
            field: field.into(),
        }
    }

    #[test]
    fn coupling_single_source_target() {
        let matrix = MatrixCoupling {
            sources: vec!["bass".into()],
            targets: vec![make_target("core", "scale")],
            weights: vec![0.5],
            damping: 0.9,
            depth: 2,
        };
        let js = generate_coupling_js(&matrix);
        assert!(js.contains("class GameCouplingMatrix"));
        assert!(js.contains("'bass'"));
        assert!(js.contains("layer: 'core'"));
        assert!(js.contains("field: 'scale'"));
        assert!(js.contains("0.5"));
    }

    #[test]
    fn coupling_3x3_generates_matrix() {
        let matrix = MatrixCoupling {
            sources: vec!["bass".into(), "treble".into(), "energy".into()],
            targets: vec![
                make_target("core", "scale"),
                make_target("noise", "intensity"),
                make_target("ring", "opacity"),
            ],
            weights: vec![0.3, 0.1, 0.0, 0.0, 0.5, 0.2, 0.1, 0.0, 0.7],
            damping: 0.92,
            depth: 3,
        };
        let js = generate_coupling_js(&matrix);
        assert!(js.contains("this._weights = ["));
        assert!(js.contains("[0.3, 0.1, 0]"));
        assert!(js.contains("[0, 0.5, 0.2]"));
        assert!(js.contains("[0.1, 0, 0.7]"));
    }

    #[test]
    fn coupling_custom_damping_depth() {
        let matrix = MatrixCoupling {
            sources: vec!["a".into()],
            targets: vec![make_target("b", "x")],
            weights: vec![1.0],
            damping: 0.85,
            depth: 5,
        };
        let js = generate_coupling_js(&matrix);
        assert!(js.contains("this._damping = 0.85"));
        assert!(js.contains("this._maxDepth = 5"));
    }

    #[test]
    fn coupling_propagate_method_exists() {
        let matrix = MatrixCoupling {
            sources: vec!["a".into()],
            targets: vec![make_target("b", "x")],
            weights: vec![1.0],
            damping: 0.9,
            depth: 2,
        };
        let js = generate_coupling_js(&matrix);
        assert!(js.contains("propagate(uniforms)"));
        assert!(js.contains("return result"));
        assert!(js.contains("Cascade decay"));
    }

    #[test]
    fn coupling_empty_sources_produces_nothing() {
        let matrix = MatrixCoupling {
            sources: vec![],
            targets: vec![make_target("b", "x")],
            weights: vec![],
            damping: 0.9,
            depth: 2,
        };
        let js = generate_coupling_js(&matrix);
        assert!(js.is_empty());
    }

    #[test]
    fn coupling_empty_targets_produces_nothing() {
        let matrix = MatrixCoupling {
            sources: vec!["a".into()],
            targets: vec![],
            weights: vec![],
            damping: 0.9,
            depth: 2,
        };
        let js = generate_coupling_js(&matrix);
        assert!(js.is_empty());
    }

    #[test]
    fn coupling_diagnostics_exposed() {
        let matrix = MatrixCoupling {
            sources: vec!["a".into()],
            targets: vec![make_target("b", "x")],
            weights: vec![1.0],
            damping: 0.9,
            depth: 2,
        };
        let js = generate_coupling_js(&matrix);
        assert!(js.contains("get weights()"));
        assert!(js.contains("get activeInfluence()"));
    }

    #[test]
    fn coupling_default_damping_when_zero() {
        let matrix = MatrixCoupling {
            sources: vec!["a".into()],
            targets: vec![make_target("b", "x")],
            weights: vec![1.0],
            damping: 0.0,
            depth: 0,
        };
        let js = generate_coupling_js(&matrix);
        assert!(js.contains(&format!("this._damping = {}", DEFAULT_COUPLING_DAMPING)));
        assert!(js.contains(&format!("this._maxDepth = {}", DEFAULT_COUPLING_DEPTH)));
    }

    // -----------------------------------------------------------------------
    // Color matrix tests
    // -----------------------------------------------------------------------

    #[test]
    fn color_identity_matrix() {
        let matrix = MatrixColor {
            values: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        };
        let wgsl = generate_color_matrix_wgsl(&matrix);
        assert!(wgsl.contains("fn apply_color_matrix(color: vec3f) -> vec3f"));
        assert!(wgsl.contains("mat3x3f"));
        // Column 0: v[0]=1, v[3]=0, v[6]=0
        assert!(wgsl.contains("vec3f(1, 0, 0)"));
        // Column 1: v[1]=0, v[4]=1, v[7]=0
        assert!(wgsl.contains("vec3f(0, 1, 0)"));
        // Column 2: v[2]=0, v[5]=0, v[8]=1
        assert!(wgsl.contains("vec3f(0, 0, 1)"));
    }

    #[test]
    fn color_custom_values() {
        let matrix = MatrixColor {
            values: [1.2, -0.1, 0.0, 0.1, 1.1, -0.05, -0.05, 0.0, 1.3],
        };
        let wgsl = generate_color_matrix_wgsl(&matrix);
        assert!(wgsl.contains("mat3x3f"));
        assert!(wgsl.contains("clamp(m * color, vec3f(0.0), vec3f(1.0))"));
    }

    #[test]
    fn color_wgsl_column_major_transpose() {
        // Row-major input: [a,b,c, d,e,f, g,h,i]
        // Expected columns: [a,d,g], [b,e,h], [c,f,i]
        let matrix = MatrixColor {
            values: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
        };
        let wgsl = generate_color_matrix_wgsl(&matrix);
        // Column 0: v[0]=1, v[3]=4, v[6]=7
        assert!(wgsl.contains("vec3f(1, 4, 7)"));
        // Column 1: v[1]=2, v[4]=5, v[7]=8
        assert!(wgsl.contains("vec3f(2, 5, 8)"));
        // Column 2: v[2]=3, v[5]=6, v[8]=9
        assert!(wgsl.contains("vec3f(3, 6, 9)"));
    }

    #[test]
    fn color_glsl_generates() {
        let matrix = MatrixColor {
            values: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        };
        let glsl = generate_color_matrix_glsl(&matrix);
        assert!(glsl.contains("vec3 apply_color_matrix(vec3 color)"));
        assert!(glsl.contains("mat3 m = mat3("));
        assert!(glsl.contains("clamp(m * color, vec3(0.0), vec3(1.0))"));
    }

    #[test]
    fn color_glsl_column_major_transpose() {
        let matrix = MatrixColor {
            values: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
        };
        let glsl = generate_color_matrix_glsl(&matrix);
        // Same transpose as WGSL — GLSL mat3 is also column-major
        assert!(glsl.contains("vec3(1, 4, 7)"));
        assert!(glsl.contains("vec3(2, 5, 8)"));
        assert!(glsl.contains("vec3(3, 6, 9)"));
    }

    // -----------------------------------------------------------------------
    // Transition matrix tests
    // -----------------------------------------------------------------------

    #[test]
    fn transitions_generates_class() {
        let matrix = MatrixTransitions {
            name: "flow".into(),
            states: vec!["intro".into(), "build".into()],
            weights: vec![0.0, 1.0, 0.5, 0.5],
            hold: Duration::Seconds(5.0),
        };
        let js = generate_transition_js(&matrix);
        assert!(js.contains("class GameTransitionMatrix_flow"));
    }

    #[test]
    fn transitions_states_and_matrix() {
        let matrix = MatrixTransitions {
            name: "seq".into(),
            states: vec!["a".into(), "b".into(), "c".into()],
            weights: vec![0.0, 0.7, 0.3, 0.2, 0.0, 0.8, 0.5, 0.5, 0.0],
            hold: Duration::Seconds(3.0),
        };
        let js = generate_transition_js(&matrix);
        assert!(js.contains("'a', 'b', 'c'"));
        assert!(js.contains("[0, 0.7, 0.3]"));
        assert!(js.contains("[0.2, 0, 0.8]"));
        assert!(js.contains("[0.5, 0.5, 0]"));
    }

    #[test]
    fn transitions_hold_duration() {
        let matrix = MatrixTransitions {
            name: "test".into(),
            states: vec!["x".into()],
            weights: vec![1.0],
            hold: Duration::Seconds(7.5),
        };
        let js = generate_transition_js(&matrix);
        assert!(js.contains("this._holdSeconds = 7.5"));
    }

    #[test]
    fn transitions_hold_duration_millis() {
        let matrix = MatrixTransitions {
            name: "test".into(),
            states: vec!["x".into()],
            weights: vec![1.0],
            hold: Duration::Millis(2500.0),
        };
        let js = generate_transition_js(&matrix);
        assert!(js.contains("this._holdSeconds = 2.5"));
    }

    #[test]
    fn transitions_evaluate_method() {
        let matrix = MatrixTransitions {
            name: "demo".into(),
            states: vec!["a".into(), "b".into()],
            weights: vec![0.0, 1.0, 1.0, 0.0],
            hold: Duration::Seconds(5.0),
        };
        let js = generate_transition_js(&matrix);
        assert!(js.contains("evaluate(elapsed)"));
        assert!(js.contains("this._lastTransition"));
        assert!(js.contains("this._holdSeconds"));
        assert!(js.contains("changed: true"));
        assert!(js.contains("changed: false"));
    }

    #[test]
    fn transitions_history_tracking() {
        let matrix = MatrixTransitions {
            name: "track".into(),
            states: vec!["a".into(), "b".into()],
            weights: vec![0.0, 1.0, 1.0, 0.0],
            hold: Duration::Seconds(5.0),
        };
        let js = generate_transition_js(&matrix);
        assert!(js.contains("this._history.push"));
        assert!(js.contains("from: this._current"));
        assert!(js.contains("get history()"));
    }

    #[test]
    fn transitions_next_method() {
        let matrix = MatrixTransitions {
            name: "rng".into(),
            states: vec!["a".into(), "b".into()],
            weights: vec![0.0, 1.0, 1.0, 0.0],
            hold: Duration::Seconds(1.0),
        };
        let js = generate_transition_js(&matrix);
        assert!(js.contains("next()"));
        assert!(js.contains("Math.random()"));
        assert!(js.contains("r -= row[i]"));
    }

    #[test]
    fn transitions_empty_states_produces_nothing() {
        let matrix = MatrixTransitions {
            name: "empty".into(),
            states: vec![],
            weights: vec![],
            hold: Duration::Seconds(1.0),
        };
        let js = generate_transition_js(&matrix);
        assert!(js.is_empty());
    }

    #[test]
    fn transitions_diagnostics_exposed() {
        let matrix = MatrixTransitions {
            name: "diag".into(),
            states: vec!["a".into()],
            weights: vec![1.0],
            hold: Duration::Seconds(1.0),
        };
        let js = generate_transition_js(&matrix);
        assert!(js.contains("get currentState()"));
        assert!(js.contains("get stateIndex()"));
        assert!(js.contains("get history()"));
        assert!(js.contains("get matrix()"));
    }

    #[test]
    fn transitions_hyphenated_name_sanitized() {
        let matrix = MatrixTransitions {
            name: "my-flow".into(),
            states: vec!["a".into()],
            weights: vec![1.0],
            hold: Duration::Seconds(1.0),
        };
        let js = generate_transition_js(&matrix);
        assert!(js.contains("class GameTransitionMatrix_my_flow"));
    }

    // -----------------------------------------------------------------------
    // expr_to_js tests
    // -----------------------------------------------------------------------

    #[test]
    fn expr_number_renders() {
        assert_eq!(expr_to_js(&Expr::Number(0.5)), "0.5");
    }

    #[test]
    fn expr_ident_renders() {
        assert_eq!(expr_to_js(&Expr::Ident("foo".into())), "foo");
    }

    #[test]
    fn expr_binop_renders() {
        let e = Expr::BinOp {
            op: BinOp::Mul,
            left: Box::new(Expr::Number(2.0)),
            right: Box::new(Expr::Ident("x".into())),
        };
        assert_eq!(expr_to_js(&e), "(2 * x)");
    }

    #[test]
    fn expr_neg_renders() {
        let e = Expr::Neg(Box::new(Expr::Number(1.0)));
        assert_eq!(expr_to_js(&e), "(-1)");
    }
}
