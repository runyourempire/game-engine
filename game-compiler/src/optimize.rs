//! AST-level optimizer and semantic analyzer for the GAME compiler.
//!
//! Six passes that operate directly on AST nodes (no IR dependency):
//! 1. Constant folding on expressions (including strength reduction)
//! 2. No-op stage elimination in pipelines
//! 3. Dead uniform detection across a cinematic
//! 4. Dead define elimination (DCE)
//! 5. Define body validation (semantic analysis)
//! 6. Arity checking for builtin calls

use crate::ast::*;
use crate::builtins;

/// Results from running all optimization passes on a cinematic.
#[derive(Debug, Clone)]
pub struct OptimizeStats {
    pub constants_folded: usize,
    pub noop_stages_removed: usize,
    pub dead_uniforms: Vec<String>,
    pub dead_defines_removed: usize,
    pub semantic_warnings: Vec<String>,
}

/// Run all optimization passes on a single `Cinematic`.
pub fn optimize_cinematic(cinematic: &mut Cinematic) -> OptimizeStats {
    let mut constants_folded = 0;

    // Fold constants in every layer's body, opts, and modulation expressions
    for layer in &mut cinematic.layers {
        for param in &mut layer.opts {
            constants_folded += constant_fold(&mut param.value);
            if let Some(ref mut m) = param.modulation {
                constants_folded += constant_fold(m);
            }
        }
        match &mut layer.body {
            LayerBody::Params(params) => {
                for param in params {
                    constants_folded += constant_fold(&mut param.value);
                    if let Some(ref mut m) = param.modulation {
                        constants_folded += constant_fold(m);
                    }
                }
            }
            LayerBody::Pipeline(stages) => {
                for stage in stages.iter_mut() {
                    for arg in &mut stage.args {
                        constants_folded += constant_fold(&mut arg.value);
                    }
                }
            }
        }
    }

    // Eliminate no-op stages in pipeline layers
    let mut noop_stages_removed = 0;
    for layer in &mut cinematic.layers {
        if let LayerBody::Pipeline(ref mut stages) = layer.body {
            noop_stages_removed += eliminate_noop_stages(stages);
        }
    }

    let dead_uniforms = find_dead_uniforms(cinematic);

    // Dead code elimination: remove unreferenced defines
    let dead_defines_removed = eliminate_dead_defines(cinematic);

    // Semantic analysis passes (non-destructive — produce warnings only)
    let mut semantic_warnings = Vec::new();
    semantic_warnings.extend(check_define_semantics(cinematic));
    semantic_warnings.extend(check_arity(cinematic));

    OptimizeStats {
        constants_folded,
        noop_stages_removed,
        dead_uniforms,
        dead_defines_removed,
        semantic_warnings,
    }
}

// ── Helpers ─────────────────────────────────────────────

fn is_zero(v: f64) -> bool {
    v.abs() < f64::EPSILON
}

fn is_one(v: f64) -> bool {
    (v - 1.0).abs() < f64::EPSILON
}

/// Extract a numeric literal from an expression, seeing through `Paren`.
pub fn as_number(expr: &Expr) -> Option<f64> {
    match expr {
        Expr::Number(v) => Some(*v),
        Expr::Paren(inner) => as_number(inner),
        _ => None,
    }
}

// ── Pass 1: Constant Folding ────────────────────────────

/// Recursively fold constant expressions bottom-up. Returns the number of folds performed.
pub fn constant_fold(expr: &mut Expr) -> usize {
    let mut count = 0;

    // Bottom-up: recurse into children first
    match expr {
        Expr::BinOp { left, right, .. } => {
            count += constant_fold(left);
            count += constant_fold(right);
        }
        Expr::Neg(inner) => {
            count += constant_fold(inner);
        }
        Expr::Paren(inner) => {
            count += constant_fold(inner);
        }
        Expr::Call { args, .. } => {
            for arg in args.iter_mut() {
                count += constant_fold(&mut arg.value);
            }
        }
        Expr::Array(elems) => {
            for elem in elems.iter_mut() {
                count += constant_fold(elem);
            }
        }
        Expr::Ternary { condition, if_true, if_false } => {
            count += constant_fold(condition);
            count += constant_fold(if_true);
            count += constant_fold(if_false);
        }
        _ => {}
    }

    // Now try to fold this node
    let replacement = match expr {
        // Neg(Number) → Number
        Expr::Neg(inner) => {
            if let Some(v) = as_number(inner) {
                Some(Expr::Number(-v))
            } else {
                None
            }
        }

        // BinOp with two numeric operands → direct computation
        Expr::BinOp { op, left, right } => {
            let lv = as_number(left);
            let rv = as_number(right);

            match (lv, rv) {
                // Both sides are constants
                (Some(a), Some(b)) => match op {
                    BinOp::Add => Some(Expr::Number(a + b)),
                    BinOp::Sub => Some(Expr::Number(a - b)),
                    BinOp::Mul => Some(Expr::Number(a * b)),
                    BinOp::Div => {
                        if is_zero(b) { None } else { Some(Expr::Number(a / b)) }
                    }
                    BinOp::Pow => Some(Expr::Number(a.powf(b))),
                    _ => None,
                },

                // Right side is constant — identity simplifications + strength reduction
                (None, Some(b)) => match op {
                    BinOp::Add if is_zero(b) => Some(take_expr(left)),  // x + 0 → x
                    BinOp::Sub if is_zero(b) => Some(take_expr(left)),  // x - 0 → x
                    BinOp::Mul if is_one(b)  => Some(take_expr(left)),  // x * 1 → x
                    BinOp::Mul if is_zero(b) => Some(Expr::Number(0.0)), // x * 0 → 0
                    BinOp::Div if is_one(b)  => Some(take_expr(left)),  // x / 1 → x
                    // Strength reduction: x / C → x * (1/C) (division is slower than multiply)
                    BinOp::Div if !is_zero(b) => {
                        Some(Expr::BinOp {
                            op: BinOp::Mul,
                            left: Box::new(take_expr(left)),
                            right: Box::new(Expr::Number(1.0 / b)),
                        })
                    }
                    _ => None,
                },

                // Left side is constant — identity simplifications
                (Some(a), None) => match op {
                    BinOp::Add if is_zero(a) => Some(take_expr(right)), // 0 + x → x
                    BinOp::Mul if is_one(a)  => Some(take_expr(right)), // 1 * x → x
                    BinOp::Mul if is_zero(a) => Some(Expr::Number(0.0)), // 0 * x → 0
                    _ => None,
                },

                _ => None,
            }
        }

        // Known math function calls with constant args
        Expr::Call { name, args } => fold_known_call(name, args),

        _ => None,
    };

    if let Some(r) = replacement {
        *expr = r;
        count += 1;
    }

    count
}

/// Take ownership of an `Expr` behind a `Box`, replacing it with a dummy.
fn take_expr(boxed: &mut Box<Expr>) -> Expr {
    std::mem::replace(boxed.as_mut(), Expr::Number(0.0))
}

/// Try to fold a call to a known single-arg or two-arg math function.
fn fold_known_call(name: &str, args: &[Arg]) -> Option<Expr> {
    let positional: Vec<f64> = args.iter()
        .filter(|a| a.name.is_none())
        .filter_map(|a| as_number(&a.value))
        .collect();

    match (name, positional.as_slice()) {
        ("sin", [v]) if is_zero(*v) => Some(Expr::Number(0.0)),
        ("cos", [v]) if is_zero(*v) => Some(Expr::Number(1.0)),
        ("sin", [v])   => Some(Expr::Number(v.sin())),
        ("cos", [v])   => Some(Expr::Number(v.cos())),
        ("abs", [v])   => Some(Expr::Number(v.abs())),
        ("floor", [v]) => Some(Expr::Number(v.floor())),
        ("ceil", [v])  => Some(Expr::Number(v.ceil())),
        ("sqrt", [v]) if *v >= 0.0 => Some(Expr::Number(v.sqrt())),
        ("min", [a, b]) => Some(Expr::Number(a.min(*b))),
        ("max", [a, b]) => Some(Expr::Number(a.max(*b))),
        ("pow", [a, b]) => Some(Expr::Number(a.powf(*b))),
        _ => None,
    }
}

// ── Pass 2: No-op Stage Elimination ─────────────────────

/// Remove pipeline stages that are geometric identity transforms.
/// Returns the count of stages removed.
pub fn eliminate_noop_stages(stages: &mut Vec<Stage>) -> usize {
    let before = stages.len();
    stages.retain(|stage| !is_noop_stage(stage));
    before - stages.len()
}

fn is_noop_stage(stage: &Stage) -> bool {
    let positional: Vec<Option<f64>> = stage.args.iter()
        .filter(|a| a.name.is_none())
        .map(|a| as_number(&a.value))
        .collect();

    match stage.name.as_str() {
        "translate" => {
            // translate(0, 0) or translate(0) — all positional args must be zero
            !positional.is_empty()
                && positional.iter().all(|v| matches!(v, Some(n) if is_zero(*n)))
        }
        "scale" => {
            // scale(1) — single positional arg equal to 1
            positional.len() == 1
                && matches!(positional[0], Some(n) if is_one(n))
        }
        "rotate" | "twist" => {
            // rotate(0) or twist(0) — single zero arg
            positional.len() == 1
                && matches!(positional[0], Some(n) if is_zero(n))
        }
        _ => false,
    }
}

// ── Pass 3: Dead Uniform Detection ──────────────────────

/// Find uniforms (params defined in `Params`-body layers) that are never
/// referenced by any pipeline stage argument or modulation expression.
pub fn find_dead_uniforms(cinematic: &Cinematic) -> Vec<String> {
    // Collect all declared uniform names
    let mut declared: Vec<String> = Vec::new();
    for layer in &cinematic.layers {
        if let LayerBody::Params(params) = &layer.body {
            for param in params {
                declared.push(param.name.clone());
            }
        }
    }

    if declared.is_empty() {
        return Vec::new();
    }

    // Collect all referenced identifiers
    let mut referenced = std::collections::HashSet::new();
    for layer in &cinematic.layers {
        // Check opts
        for param in &layer.opts {
            collect_idents(&param.value, &mut referenced);
            if let Some(ref m) = param.modulation {
                collect_idents(m, &mut referenced);
            }
        }
        match &layer.body {
            LayerBody::Pipeline(stages) => {
                for stage in stages {
                    for arg in &stage.args {
                        collect_idents(&arg.value, &mut referenced);
                    }
                }
            }
            LayerBody::Params(params) => {
                for param in params {
                    if let Some(ref m) = param.modulation {
                        collect_idents(m, &mut referenced);
                    }
                    // The value itself can reference other uniforms
                    collect_idents(&param.value, &mut referenced);
                }
            }
        }
    }

    // Also check lenses, arcs, resonates, defines, react
    for lens in &cinematic.lenses {
        for param in &lens.properties {
            collect_idents(&param.value, &mut referenced);
            if let Some(ref m) = param.modulation {
                collect_idents(m, &mut referenced);
            }
        }
        for stage in &lens.post {
            for arg in &stage.args {
                collect_idents(&arg.value, &mut referenced);
            }
        }
    }
    for arc in &cinematic.arcs {
        for entry in &arc.entries {
            collect_idents(&entry.from, &mut referenced);
            collect_idents(&entry.to, &mut referenced);
        }
    }
    for res in &cinematic.resonates {
        for entry in &res.entries {
            collect_idents(&entry.weight, &mut referenced);
            referenced.insert(entry.source.clone());
            referenced.insert(entry.target.clone());
        }
    }
    for define in &cinematic.defines {
        for stage in &define.body {
            for arg in &stage.args {
                collect_idents(&arg.value, &mut referenced);
            }
        }
    }
    if let Some(ref react) = cinematic.react {
        for reaction in &react.reactions {
            collect_idents(&reaction.signal, &mut referenced);
            collect_idents(&reaction.action, &mut referenced);
        }
    }

    declared.into_iter()
        .filter(|name| !referenced.contains(name))
        .collect()
}

/// Recursively collect all `Ident` and `DottedIdent` names from an expression.
fn collect_idents(expr: &Expr, out: &mut std::collections::HashSet<String>) {
    match expr {
        Expr::Ident(name) => { out.insert(name.clone()); }
        Expr::DottedIdent { object, field } => {
            out.insert(object.clone());
            out.insert(format!("{}.{}", object, field));
        }
        Expr::BinOp { left, right, .. } => {
            collect_idents(left, out);
            collect_idents(right, out);
        }
        Expr::Neg(inner) | Expr::Paren(inner) => collect_idents(inner, out),
        Expr::Call { args, .. } => {
            for arg in args {
                collect_idents(&arg.value, out);
            }
        }
        Expr::Array(elems) => {
            for elem in elems {
                collect_idents(elem, out);
            }
        }
        Expr::Ternary { condition, if_true, if_false } => {
            collect_idents(condition, out);
            collect_idents(if_true, out);
            collect_idents(if_false, out);
        }
        _ => {}
    }
}

// ── Pass 4: Dead Define Elimination ─────────────────────

/// Remove defines that are never referenced by any layer pipeline.
/// Returns the count of defines removed.
pub fn eliminate_dead_defines(cinematic: &mut Cinematic) -> usize {
    let mut used_defines: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Collect all stage names used in layer pipelines
    for layer in &cinematic.layers {
        if let LayerBody::Pipeline(stages) = &layer.body {
            for stage in stages {
                used_defines.insert(stage.name.clone());
            }
        }
    }

    // Also check lens post-processing pipelines
    for lens in &cinematic.lenses {
        for stage in &lens.post {
            used_defines.insert(stage.name.clone());
        }
    }

    let before = cinematic.defines.len();
    cinematic.defines.retain(|d| used_defines.contains(&d.name));
    before - cinematic.defines.len()
}

// ── Pass 5: Semantic Analysis — Define Body Validation ──

/// Check that define blocks have valid pipeline stages and that
/// all declared parameters are actually used in the body.
pub fn check_define_semantics(cinematic: &Cinematic) -> Vec<String> {
    let mut warnings = Vec::new();
    for define in &cinematic.defines {
        // Check that stages in the define body are valid builtins or other defines
        let define_names: std::collections::HashSet<&str> = cinematic
            .defines
            .iter()
            .map(|d| d.name.as_str())
            .collect();

        for stage in &define.body {
            if builtins::lookup(&stage.name).is_none() && !define_names.contains(stage.name.as_str()) {
                warnings.push(format!(
                    "define '{}': unknown function '{}'",
                    define.name, stage.name
                ));
            }
        }

        // Check that define parameters are actually used in the body
        for param in &define.params {
            let used = define.body.iter().any(|stage| {
                stage
                    .args
                    .iter()
                    .any(|arg| arg_references_name(&arg.value, param))
            });
            if !used {
                warnings.push(format!(
                    "define '{}': parameter '{}' is never used",
                    define.name, param
                ));
            }
        }
    }
    warnings
}

/// Recursively check if an expression references a given identifier name.
pub fn arg_references_name(expr: &Expr, name: &str) -> bool {
    match expr {
        Expr::Ident(s) => s == name,
        Expr::DottedIdent { object, .. } => object == name,
        Expr::BinOp { left, right, .. } => {
            arg_references_name(left, name) || arg_references_name(right, name)
        }
        Expr::Neg(inner) | Expr::Paren(inner) => arg_references_name(inner, name),
        Expr::Call { args, .. } => args.iter().any(|a| arg_references_name(&a.value, name)),
        Expr::Array(elems) => elems.iter().any(|e| arg_references_name(e, name)),
        Expr::Ternary {
            condition,
            if_true,
            if_false,
        } => {
            arg_references_name(condition, name)
                || arg_references_name(if_true, name)
                || arg_references_name(if_false, name)
        }
        _ => false,
    }
}

// ── Pass 6: Semantic Analysis — Arity Checking ──────────

/// Check that builtin calls in layer pipelines don't exceed the
/// declared parameter count.
pub fn check_arity(cinematic: &Cinematic) -> Vec<String> {
    let mut warnings = Vec::new();
    for layer in &cinematic.layers {
        if let LayerBody::Pipeline(stages) = &layer.body {
            for stage in stages {
                if let Some(builtin) = builtins::lookup(&stage.name) {
                    let max_params = builtin.params.len();
                    let positional_count =
                        stage.args.iter().filter(|a| a.name.is_none()).count();
                    if positional_count > max_params {
                        warnings.push(format!(
                            "layer '{}': '{}' accepts {} parameters, but {} were given",
                            layer.name, stage.name, max_params, positional_count
                        ));
                    }
                }
            }
        }
    }

    // Also check lens post-processing pipelines
    for lens in &cinematic.lenses {
        let lens_name = lens
            .name
            .as_deref()
            .unwrap_or("<unnamed>");
        for stage in &lens.post {
            if let Some(builtin) = builtins::lookup(&stage.name) {
                let max_params = builtin.params.len();
                let positional_count =
                    stage.args.iter().filter(|a| a.name.is_none()).count();
                if positional_count > max_params {
                    warnings.push(format!(
                        "lens '{}': '{}' accepts {} parameters, but {} were given",
                        lens_name, stage.name, max_params, positional_count
                    ));
                }
            }
        }
    }

    warnings
}

// ── Tests ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn num(v: f64) -> Expr { Expr::Number(v) }
    fn ident(s: &str) -> Expr { Expr::Ident(s.to_string()) }

    fn binop(op: BinOp, l: Expr, r: Expr) -> Expr {
        Expr::BinOp { op, left: Box::new(l), right: Box::new(r) }
    }

    fn call(name: &str, vals: Vec<Expr>) -> Expr {
        Expr::Call {
            name: name.to_string(),
            args: vals.into_iter().map(|v| Arg { name: None, value: v }).collect(),
        }
    }

    fn pos_arg(v: Expr) -> Arg { Arg { name: None, value: v } }

    #[test]
    fn fold_literal_add() {
        let mut e = binop(BinOp::Add, num(2.0), num(3.0));
        let n = constant_fold(&mut e);
        assert_eq!(n, 1);
        assert_eq!(as_number(&e), Some(5.0));
    }

    #[test]
    fn fold_literal_mul() {
        let mut e = binop(BinOp::Mul, num(3.0), num(4.0));
        let n = constant_fold(&mut e);
        assert_eq!(n, 1);
        assert_eq!(as_number(&e), Some(12.0));
    }

    #[test]
    fn fold_literal_div() {
        let mut e = binop(BinOp::Div, num(10.0), num(4.0));
        let n = constant_fold(&mut e);
        assert_eq!(n, 1);
        assert_eq!(as_number(&e), Some(2.5));
    }

    #[test]
    fn fold_nested() {
        // (2 + 3) * 4 → 20
        let mut e = binop(BinOp::Mul, binop(BinOp::Add, num(2.0), num(3.0)), num(4.0));
        let n = constant_fold(&mut e);
        assert_eq!(n, 2); // inner add + outer mul
        assert_eq!(as_number(&e), Some(20.0));
    }

    #[test]
    fn fold_multiply_by_zero() {
        let mut e = binop(BinOp::Mul, ident("x"), num(0.0));
        let n = constant_fold(&mut e);
        assert_eq!(n, 1);
        assert_eq!(as_number(&e), Some(0.0));
    }

    #[test]
    fn fold_multiply_by_one() {
        let mut e = binop(BinOp::Mul, ident("x"), num(1.0));
        constant_fold(&mut e);
        assert!(matches!(e, Expr::Ident(ref s) if s == "x"));
    }

    #[test]
    fn fold_add_zero() {
        let mut e = binop(BinOp::Add, ident("y"), num(0.0));
        constant_fold(&mut e);
        assert!(matches!(e, Expr::Ident(ref s) if s == "y"));
    }

    #[test]
    fn fold_identity_div() {
        let mut e = binop(BinOp::Div, ident("z"), num(1.0));
        constant_fold(&mut e);
        assert!(matches!(e, Expr::Ident(ref s) if s == "z"));
    }

    #[test]
    fn fold_neg_literal() {
        let mut e = Expr::Neg(Box::new(num(7.0)));
        let n = constant_fold(&mut e);
        assert_eq!(n, 1);
        assert_eq!(as_number(&e), Some(-7.0));
    }

    #[test]
    fn fold_sin_zero() {
        let mut e = call("sin", vec![num(0.0)]);
        constant_fold(&mut e);
        assert_eq!(as_number(&e), Some(0.0));
    }

    #[test]
    fn fold_cos_zero() {
        let mut e = call("cos", vec![num(0.0)]);
        constant_fold(&mut e);
        assert_eq!(as_number(&e), Some(1.0));
    }

    #[test]
    fn fold_min_max() {
        let mut e_min = call("min", vec![num(3.0), num(7.0)]);
        constant_fold(&mut e_min);
        assert_eq!(as_number(&e_min), Some(3.0));

        let mut e_max = call("max", vec![num(3.0), num(7.0)]);
        constant_fold(&mut e_max);
        assert_eq!(as_number(&e_max), Some(7.0));
    }

    #[test]
    fn noop_translate_zero() {
        let mut stages = vec![
            Stage { name: "translate".into(), args: vec![pos_arg(num(0.0)), pos_arg(num(0.0))] },
            Stage { name: "blur".into(), args: vec![pos_arg(num(5.0))] },
        ];
        let n = eliminate_noop_stages(&mut stages);
        assert_eq!(n, 1);
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0].name, "blur");
    }

    #[test]
    fn noop_scale_one() {
        let mut stages = vec![
            Stage { name: "scale".into(), args: vec![pos_arg(num(1.0))] },
        ];
        let n = eliminate_noop_stages(&mut stages);
        assert_eq!(n, 1);
        assert!(stages.is_empty());
    }

    #[test]
    fn noop_rotate_zero() {
        let mut stages = vec![
            Stage { name: "rotate".into(), args: vec![pos_arg(num(0.0))] },
        ];
        let n = eliminate_noop_stages(&mut stages);
        assert_eq!(n, 1);
        assert!(stages.is_empty());
    }

    #[test]
    fn not_noop_nonzero() {
        let mut stages = vec![
            Stage { name: "translate".into(), args: vec![pos_arg(num(1.0)), pos_arg(num(0.0))] },
            Stage { name: "scale".into(), args: vec![pos_arg(num(2.0))] },
        ];
        let n = eliminate_noop_stages(&mut stages);
        assert_eq!(n, 0);
        assert_eq!(stages.len(), 2);
    }

    #[test]
    fn not_noop_dynamic() {
        let mut stages = vec![
            Stage { name: "rotate".into(), args: vec![pos_arg(ident("angle"))] },
        ];
        let n = eliminate_noop_stages(&mut stages);
        assert_eq!(n, 0);
        assert_eq!(stages.len(), 1);
    }

    #[test]
    fn dead_uniform_detection() {
        let cinematic = Cinematic {
            name: "test".into(),
            layers: vec![
                // Uniform layer declaring "color" and "unused_param"
                Layer {
                    name: "uniforms".into(),
                    opts: vec![],
                    memory: None,
                    cast: None,
                    body: LayerBody::Params(vec![
                        Param {
                            name: "color".into(),
                            value: num(1.0),
                            modulation: None,
                            temporal_ops: vec![],
                        },
                        Param {
                            name: "unused_param".into(),
                            value: num(0.5),
                            modulation: None,
                            temporal_ops: vec![],
                        },
                    ]),
                },
                // Pipeline layer that only references "color"
                Layer {
                    name: "main".into(),
                    opts: vec![],
                    memory: None,
                    cast: None,
                    body: LayerBody::Pipeline(vec![
                        Stage {
                            name: "fill".into(),
                            args: vec![pos_arg(ident("color"))],
                        },
                    ]),
                },
            ],
            arcs: vec![],
            resonates: vec![],
            listen: None,
            voice: None,
            score: None,
            gravity: None,
            lenses: vec![],
            react: None,
            defines: vec![],
        };

        let dead = find_dead_uniforms(&cinematic);
        assert_eq!(dead, vec!["unused_param".to_string()]);
    }

    // ── Strength Reduction Tests ─────────────────────────

    #[test]
    fn strength_reduction_div_to_mul() {
        // x / 2.0 → x * 0.5
        let mut e = binop(BinOp::Div, ident("x"), num(2.0));
        let n = constant_fold(&mut e);
        assert_eq!(n, 1);
        match &e {
            Expr::BinOp { op, right, .. } => {
                assert_eq!(*op, BinOp::Mul);
                assert_eq!(as_number(right), Some(0.5));
            }
            _ => panic!("expected BinOp::Mul after strength reduction"),
        }
    }

    #[test]
    fn strength_reduction_div_by_4() {
        // x / 4.0 → x * 0.25
        let mut e = binop(BinOp::Div, ident("y"), num(4.0));
        let n = constant_fold(&mut e);
        assert_eq!(n, 1);
        match &e {
            Expr::BinOp { op, right, .. } => {
                assert_eq!(*op, BinOp::Mul);
                assert_eq!(as_number(right), Some(0.25));
            }
            _ => panic!("expected BinOp::Mul after strength reduction"),
        }
    }

    #[test]
    fn strength_reduction_preserves_div_by_one() {
        // x / 1.0 → x (identity, not strength reduction)
        let mut e = binop(BinOp::Div, ident("z"), num(1.0));
        constant_fold(&mut e);
        assert!(matches!(e, Expr::Ident(ref s) if s == "z"));
    }

    // ── Dead Define Elimination Tests ────────────────────

    fn make_test_cinematic(
        layers: Vec<Layer>,
        defines: Vec<DefineBlock>,
    ) -> Cinematic {
        Cinematic {
            name: "test".into(),
            layers,
            arcs: vec![],
            resonates: vec![],
            listen: None,
            voice: None,
            score: None,
            gravity: None,
            lenses: vec![],
            react: None,
            defines,
        }
    }

    #[test]
    fn dce_removes_unreferenced_defines() {
        let mut cin = make_test_cinematic(
            vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                cast: None,
                body: LayerBody::Pipeline(vec![
                    Stage { name: "circle".into(), args: vec![] },
                ]),
            }],
            vec![
                DefineBlock {
                    name: "used_shape".into(),
                    params: vec![],
                    body: vec![Stage { name: "circle".into(), args: vec![] }],
                },
                DefineBlock {
                    name: "dead_shape".into(),
                    params: vec![],
                    body: vec![Stage { name: "star".into(), args: vec![] }],
                },
            ],
        );

        let removed = eliminate_dead_defines(&mut cin);
        // Neither define is used by the pipeline (which uses "circle", a builtin)
        assert_eq!(removed, 2);
        assert!(cin.defines.is_empty());
    }

    #[test]
    fn dce_keeps_referenced_defines() {
        let mut cin = make_test_cinematic(
            vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                cast: None,
                body: LayerBody::Pipeline(vec![
                    Stage { name: "my_shape".into(), args: vec![] },
                    Stage { name: "glow".into(), args: vec![] },
                ]),
            }],
            vec![
                DefineBlock {
                    name: "my_shape".into(),
                    params: vec![],
                    body: vec![Stage { name: "circle".into(), args: vec![] }],
                },
                DefineBlock {
                    name: "unused_shape".into(),
                    params: vec![],
                    body: vec![Stage { name: "star".into(), args: vec![] }],
                },
            ],
        );

        let removed = eliminate_dead_defines(&mut cin);
        assert_eq!(removed, 1);
        assert_eq!(cin.defines.len(), 1);
        assert_eq!(cin.defines[0].name, "my_shape");
    }

    #[test]
    fn dce_no_defines_is_noop() {
        let mut cin = make_test_cinematic(
            vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                cast: None,
                body: LayerBody::Pipeline(vec![
                    Stage { name: "circle".into(), args: vec![] },
                ]),
            }],
            vec![],
        );

        let removed = eliminate_dead_defines(&mut cin);
        assert_eq!(removed, 0);
    }

    // ── Define Semantics Tests ───────────────────────────

    #[test]
    fn semantics_detects_unknown_function_in_define() {
        let cin = make_test_cinematic(
            vec![],
            vec![DefineBlock {
                name: "my_effect".into(),
                params: vec![],
                body: vec![
                    Stage { name: "circle".into(), args: vec![] },
                    Stage { name: "nonexistent_fn".into(), args: vec![] },
                ],
            }],
        );

        let warnings = check_define_semantics(&cin);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("unknown function 'nonexistent_fn'"));
        assert!(warnings[0].contains("define 'my_effect'"));
    }

    #[test]
    fn semantics_detects_unused_parameter() {
        let cin = make_test_cinematic(
            vec![],
            vec![DefineBlock {
                name: "my_shape".into(),
                params: vec!["r".into(), "unused_color".into()],
                body: vec![Stage {
                    name: "circle".into(),
                    args: vec![Arg {
                        name: None,
                        value: Expr::Ident("r".into()),
                    }],
                }],
            }],
        );

        let warnings = check_define_semantics(&cin);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("parameter 'unused_color' is never used"));
    }

    #[test]
    fn semantics_no_warnings_for_valid_define() {
        let cin = make_test_cinematic(
            vec![],
            vec![DefineBlock {
                name: "my_shape".into(),
                params: vec!["r".into()],
                body: vec![Stage {
                    name: "circle".into(),
                    args: vec![Arg {
                        name: None,
                        value: Expr::Ident("r".into()),
                    }],
                }],
            }],
        );

        let warnings = check_define_semantics(&cin);
        assert!(warnings.is_empty());
    }

    #[test]
    fn semantics_allows_cross_define_references() {
        // A define that calls another define should not produce a warning
        let cin = make_test_cinematic(
            vec![],
            vec![
                DefineBlock {
                    name: "base_shape".into(),
                    params: vec![],
                    body: vec![Stage { name: "circle".into(), args: vec![] }],
                },
                DefineBlock {
                    name: "styled_shape".into(),
                    params: vec![],
                    body: vec![
                        Stage { name: "base_shape".into(), args: vec![] },
                        Stage { name: "glow".into(), args: vec![] },
                    ],
                },
            ],
        );

        let warnings = check_define_semantics(&cin);
        assert!(warnings.is_empty());
    }

    // ── Arity Checking Tests ─────────────────────────────

    #[test]
    fn arity_detects_too_many_args() {
        // circle() accepts 1 param (radius), giving it 3 positional args should warn
        let cin = make_test_cinematic(
            vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                cast: None,
                body: LayerBody::Pipeline(vec![Stage {
                    name: "circle".into(),
                    args: vec![
                        pos_arg(num(0.5)),
                        pos_arg(num(0.3)),
                        pos_arg(num(0.1)),
                    ],
                }]),
            }],
            vec![],
        );

        let warnings = check_arity(&cin);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("'circle' accepts 1 parameters, but 3 were given"));
    }

    #[test]
    fn arity_no_warning_for_correct_count() {
        // tint() accepts 3 params (r, g, b)
        let cin = make_test_cinematic(
            vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                cast: None,
                body: LayerBody::Pipeline(vec![Stage {
                    name: "tint".into(),
                    args: vec![
                        pos_arg(num(1.0)),
                        pos_arg(num(0.5)),
                        pos_arg(num(0.0)),
                    ],
                }]),
            }],
            vec![],
        );

        let warnings = check_arity(&cin);
        assert!(warnings.is_empty());
    }

    #[test]
    fn arity_named_args_not_counted() {
        // Named args shouldn't count toward positional arity
        let cin = make_test_cinematic(
            vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                cast: None,
                body: LayerBody::Pipeline(vec![Stage {
                    name: "circle".into(),
                    args: vec![
                        Arg { name: Some("radius".into()), value: num(0.5) },
                        Arg { name: Some("extra".into()), value: num(0.3) },
                    ],
                }]),
            }],
            vec![],
        );

        let warnings = check_arity(&cin);
        assert!(warnings.is_empty()); // named args are not positional
    }

    #[test]
    fn arity_skips_non_builtin_stages() {
        // Custom define stages should not be arity-checked against builtins
        let cin = make_test_cinematic(
            vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                cast: None,
                body: LayerBody::Pipeline(vec![Stage {
                    name: "my_custom_define".into(),
                    args: vec![
                        pos_arg(num(1.0)),
                        pos_arg(num(2.0)),
                        pos_arg(num(3.0)),
                        pos_arg(num(4.0)),
                    ],
                }]),
            }],
            vec![],
        );

        let warnings = check_arity(&cin);
        assert!(warnings.is_empty()); // not a builtin, no check
    }

    // ── arg_references_name Tests ────────────────────────

    #[test]
    fn arg_references_simple_ident() {
        assert!(arg_references_name(&Expr::Ident("x".into()), "x"));
        assert!(!arg_references_name(&Expr::Ident("y".into()), "x"));
    }

    #[test]
    fn arg_references_nested_in_binop() {
        let expr = Expr::BinOp {
            op: BinOp::Add,
            left: Box::new(Expr::Number(1.0)),
            right: Box::new(Expr::Ident("target".into())),
        };
        assert!(arg_references_name(&expr, "target"));
        assert!(!arg_references_name(&expr, "other"));
    }

    #[test]
    fn arg_references_in_call() {
        let expr = Expr::Call {
            name: "sin".into(),
            args: vec![Arg {
                name: None,
                value: Expr::Ident("angle".into()),
            }],
        };
        assert!(arg_references_name(&expr, "angle"));
        assert!(!arg_references_name(&expr, "radius"));
    }

    #[test]
    fn arg_references_dotted_ident() {
        let expr = Expr::DottedIdent {
            object: "audio".into(),
            field: "bass".into(),
        };
        assert!(arg_references_name(&expr, "audio"));
        assert!(!arg_references_name(&expr, "bass"));
    }

    #[test]
    fn arg_references_in_ternary() {
        let expr = Expr::Ternary {
            condition: Box::new(Expr::Ident("flag".into())),
            if_true: Box::new(Expr::Number(1.0)),
            if_false: Box::new(Expr::Ident("fallback".into())),
        };
        assert!(arg_references_name(&expr, "flag"));
        assert!(arg_references_name(&expr, "fallback"));
        assert!(!arg_references_name(&expr, "other"));
    }

    // ── Integration: optimize_cinematic returns new fields ──

    #[test]
    fn optimize_cinematic_returns_all_stats() {
        let mut cin = make_test_cinematic(
            vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                cast: None,
                body: LayerBody::Pipeline(vec![
                    Stage { name: "circle".into(), args: vec![pos_arg(num(0.5))] },
                    Stage { name: "glow".into(), args: vec![] },
                ]),
            }],
            vec![DefineBlock {
                name: "dead_define".into(),
                params: vec![],
                body: vec![Stage { name: "star".into(), args: vec![] }],
            }],
        );

        let stats = optimize_cinematic(&mut cin);
        assert_eq!(stats.dead_defines_removed, 1);
        assert!(stats.semantic_warnings.is_empty());
        assert!(cin.defines.is_empty());
    }
}
