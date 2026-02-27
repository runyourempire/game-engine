//! IR optimization passes.
//!
//! Operates on ShaderIR in-place, applying:
//! 1. **Constant folding** — evaluate compile-time-known expressions
//! 2. **No-op stage elimination** — remove stages that have no effect
//! 3. **Dead uniform elimination** — mark uniforms not referenced in any stage

use crate::ast::BinOp;
use crate::ir::*;

/// Optimization results for diagnostics.
#[derive(Debug, Default)]
pub struct OptimizeStats {
    pub constants_folded: usize,
    pub noop_stages_removed: usize,
    pub dead_uniforms_marked: usize,
}

/// Run all optimization passes on the IR.
pub fn optimize(ir: &mut ShaderIR) -> OptimizeStats {
    let mut stats = OptimizeStats::default();

    // Pass 1: Constant folding on all expressions
    for layer in &mut ir.layers {
        for stage in &mut layer.stages {
            for arg in &mut stage.args {
                let expr = match arg {
                    IrArg::Positional(ref mut e) => e,
                    IrArg::Named { ref mut value, .. } => value,
                };
                stats.constants_folded += constant_fold(expr);
            }
        }
    }

    // Pass 2: No-op stage elimination
    for layer in &mut ir.layers {
        stats.noop_stages_removed += eliminate_noops(&mut layer.stages);
    }

    // Pass 3: Dead uniform elimination
    // Disabled for now — removing uniforms breaks x-ray variant generation
    // which requires a consistent uniform struct across all variants.
    // TODO: Enable when x-ray variants share a pre-collected uniform set.
    // stats.dead_uniforms_marked += mark_dead_uniforms(ir);

    stats
}

// ── Pass 1: Constant folding ──────────────────────────────────────────

/// Recursively fold constant expressions. Returns count of folds performed.
///
/// Folds:
/// - `Literal op Literal` → `Literal` (for +, -, *, /)
/// - `Neg(Literal)` → `Literal(-v)`
/// - `sin(0)` → `0`, `cos(0)` → `1`
/// - `x * 1.0` → `x`, `x * 0.0` → `0`
/// - `x + 0.0` → `x`, `x - 0.0` → `x`
fn constant_fold(expr: &mut IrExpr) -> usize {
    let mut count = 0;

    // Recurse first (bottom-up folding)
    match expr {
        IrExpr::BinOp { left, right, .. } => {
            count += constant_fold(left);
            count += constant_fold(right);
        }
        IrExpr::Neg(inner) => {
            count += constant_fold(inner);
        }
        IrExpr::Call { args, .. } => {
            for a in args.iter_mut() {
                count += constant_fold(a);
            }
        }
        IrExpr::Array(elems) => {
            for e in elems.iter_mut() {
                count += constant_fold(e);
            }
        }
        IrExpr::Ternary {
            condition,
            if_true,
            if_false,
        } => {
            count += constant_fold(condition);
            count += constant_fold(if_true);
            count += constant_fold(if_false);
        }
        _ => {}
    }

    // Now try to fold this node
    let folded = try_fold(expr);
    if folded {
        count += 1;
    }

    count
}

/// Try to fold a single expression node. Returns true if folded.
fn try_fold(expr: &mut IrExpr) -> bool {
    match expr {
        // Negate literal: -3.0 → Literal(-3.0)
        IrExpr::Neg(inner) => {
            if let Some(v) = inner.as_literal() {
                *expr = IrExpr::Literal(-v);
                return true;
            }
        }

        // Binary op on two literals
        IrExpr::BinOp { left, op, right } => {
            if let (Some(lv), Some(rv)) = (left.as_literal(), right.as_literal()) {
                let result = match op {
                    BinOp::Add => Some(lv + rv),
                    BinOp::Sub => Some(lv - rv),
                    BinOp::Mul => Some(lv * rv),
                    BinOp::Div => {
                        if rv != 0.0 {
                            Some(lv / rv)
                        } else {
                            None
                        }
                    }
                    // Comparison ops produce 0/1, but we leave them for now
                    BinOp::Gt | BinOp::Lt => None,
                };
                if let Some(v) = result {
                    *expr = IrExpr::Literal(v);
                    return true;
                }
            }

            // Identity simplifications: x * 1 → x, x + 0 → x, etc.
            if let Some(rv) = right.as_literal() {
                match op {
                    BinOp::Mul if rv == 1.0 => {
                        *expr = *left.clone();
                        return true;
                    }
                    BinOp::Mul if rv == 0.0 => {
                        *expr = IrExpr::Literal(0.0);
                        return true;
                    }
                    BinOp::Add | BinOp::Sub if rv == 0.0 => {
                        *expr = *left.clone();
                        return true;
                    }
                    BinOp::Div if rv == 1.0 => {
                        *expr = *left.clone();
                        return true;
                    }
                    _ => {}
                }
            }
            if let Some(lv) = left.as_literal() {
                match op {
                    BinOp::Mul if lv == 1.0 => {
                        *expr = *right.clone();
                        return true;
                    }
                    BinOp::Mul if lv == 0.0 => {
                        *expr = IrExpr::Literal(0.0);
                        return true;
                    }
                    BinOp::Add if lv == 0.0 => {
                        *expr = *right.clone();
                        return true;
                    }
                    _ => {}
                }
            }
        }

        // Fold known math functions on constant args
        IrExpr::Call { name, args } => {
            if args.len() == 1 {
                if let Some(v) = args[0].as_literal() {
                    let result = match name.as_str() {
                        "sin" => Some(v.sin()),
                        "cos" => Some(v.cos()),
                        "abs" => Some(v.abs()),
                        "floor" => Some(v.floor()),
                        "ceil" => Some(v.ceil()),
                        "fract" => Some(v.fract()),
                        "sqrt" if v >= 0.0 => Some(v.sqrt()),
                        "exp" => Some(v.exp()),
                        "log" if v > 0.0 => Some(v.ln()),
                        _ => None,
                    };
                    if let Some(r) = result {
                        *expr = IrExpr::Literal(r);
                        return true;
                    }
                }
            }
            if args.len() == 2 {
                if let (Some(a), Some(b)) = (args[0].as_literal(), args[1].as_literal()) {
                    let result = match name.as_str() {
                        "min" => Some(a.min(b)),
                        "max" => Some(a.max(b)),
                        "pow" => Some(a.powf(b)),
                        _ => None,
                    };
                    if let Some(r) = result {
                        *expr = IrExpr::Literal(r);
                        return true;
                    }
                }
            }
        }

        _ => {}
    }

    false
}

// ── Pass 2: No-op stage elimination ───────────────────────────────────

/// Remove stages that have no visual effect.
///
/// - `translate(0.0, 0.0)` — zero offset
/// - `scale(1.0)` — identity scale
/// - `rotate(0.0)` — zero rotation
/// - `twist(0.0)` — zero twist
fn eliminate_noops(stages: &mut Vec<IrStage>) -> usize {
    let before = stages.len();
    stages.retain(|stage| !is_noop(stage));
    before - stages.len()
}

fn is_noop(stage: &IrStage) -> bool {
    match stage.name.as_str() {
        "translate" => {
            let x = stage.positional_arg(0).and_then(|e| e.as_literal());
            let y = stage.positional_arg(1).and_then(|e| e.as_literal());
            matches!((x, y), (Some(0.0), Some(0.0)) | (Some(0.0), None))
                && stage.positional_arg(1).map_or(true, |e| e.as_literal() == Some(0.0))
        }
        "scale" => {
            let s = stage.positional_arg(0).and_then(|e| e.as_literal());
            s == Some(1.0)
        }
        "rotate" => {
            let a = stage.positional_arg(0).and_then(|e| e.as_literal());
            a == Some(0.0)
        }
        "twist" => {
            let a = stage.positional_arg(0).and_then(|e| e.as_literal());
            a == Some(0.0)
        }
        _ => false,
    }
}

// ── Pass 3: Dead uniform elimination ──────────────────────────────────

#[allow(dead_code)] // Disabled pending x-ray variant uniform sharing
/// Mark uniforms not referenced in any stage expression as dead.
/// Returns count of newly marked dead uniforms.
///
/// Conservative: only marks a uniform as dead if it has NO modulation
/// (no mod_js) AND is not referenced in any stage. Modulated params
/// are always kept because they're driven by runtime signals and may
/// be arc transition targets.
fn mark_dead_uniforms(ir: &mut ShaderIR) -> usize {
    let referenced = ir.referenced_idents();
    let mut count = 0;

    for uniform in &mut ir.uniforms {
        // Keep all modulated params — they're driven by runtime signals
        if uniform.mod_js.is_some() {
            continue;
        }

        // A uniform is referenced if its name appears in any stage expression.
        // The name in WGSL is the bare param name (e.g., "scale" not "p_scale"),
        // since emit_param_bindings does `let scale = u.p_scale;`.
        if !referenced.contains(&uniform.name) && !uniform.dead {
            uniform.dead = true;
            count += 1;
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_literal_addition() {
        let mut expr = IrExpr::BinOp {
            left: Box::new(IrExpr::Literal(2.0)),
            op: BinOp::Add,
            right: Box::new(IrExpr::Literal(3.0)),
        };
        let count = constant_fold(&mut expr);
        assert_eq!(count, 1);
        assert!(matches!(expr, IrExpr::Literal(v) if (v - 5.0).abs() < 1e-10));
    }

    #[test]
    fn fold_multiply_by_zero() {
        let mut expr = IrExpr::BinOp {
            left: Box::new(IrExpr::Ident("time".into())),
            op: BinOp::Mul,
            right: Box::new(IrExpr::Literal(0.0)),
        };
        let count = constant_fold(&mut expr);
        assert_eq!(count, 1);
        assert!(matches!(expr, IrExpr::Literal(v) if v == 0.0));
    }

    #[test]
    fn fold_multiply_by_one() {
        let mut expr = IrExpr::BinOp {
            left: Box::new(IrExpr::Ident("time".into())),
            op: BinOp::Mul,
            right: Box::new(IrExpr::Literal(1.0)),
        };
        let count = constant_fold(&mut expr);
        assert_eq!(count, 1);
        assert!(matches!(expr, IrExpr::Ident(ref n) if n == "time"));
    }

    #[test]
    fn fold_sin_zero() {
        let mut expr = IrExpr::Call {
            name: "sin".into(),
            args: vec![IrExpr::Literal(0.0)],
        };
        let count = constant_fold(&mut expr);
        assert_eq!(count, 1);
        assert!(matches!(expr, IrExpr::Literal(v) if v.abs() < 1e-10));
    }

    #[test]
    fn fold_nested_expression() {
        // (2.0 + 3.0) * 4.0 → 5.0 * 4.0 → 20.0
        let mut expr = IrExpr::BinOp {
            left: Box::new(IrExpr::BinOp {
                left: Box::new(IrExpr::Literal(2.0)),
                op: BinOp::Add,
                right: Box::new(IrExpr::Literal(3.0)),
            }),
            op: BinOp::Mul,
            right: Box::new(IrExpr::Literal(4.0)),
        };
        let count = constant_fold(&mut expr);
        assert_eq!(count, 2); // two folds: inner add, then outer mul
        assert!(matches!(expr, IrExpr::Literal(v) if (v - 20.0).abs() < 1e-10));
    }

    #[test]
    fn noop_translate_zero() {
        let stage = IrStage {
            kind: StageKind::Position,
            name: "translate".into(),
            args: vec![
                IrArg::Positional(IrExpr::Literal(0.0)),
                IrArg::Positional(IrExpr::Literal(0.0)),
            ],
            span: None,
        };
        assert!(is_noop(&stage));
    }

    #[test]
    fn noop_scale_one() {
        let stage = IrStage {
            kind: StageKind::Position,
            name: "scale".into(),
            args: vec![IrArg::Positional(IrExpr::Literal(1.0))],
            span: None,
        };
        assert!(is_noop(&stage));
    }

    #[test]
    fn not_noop_translate_nonzero() {
        let stage = IrStage {
            kind: StageKind::Position,
            name: "translate".into(),
            args: vec![
                IrArg::Positional(IrExpr::Literal(0.5)),
                IrArg::Positional(IrExpr::Literal(0.0)),
            ],
            span: None,
        };
        assert!(!is_noop(&stage));
    }

    #[test]
    fn not_noop_dynamic_scale() {
        let stage = IrStage {
            kind: StageKind::Position,
            name: "scale".into(),
            args: vec![IrArg::Positional(IrExpr::Ident("time".into()))],
            span: None,
        };
        assert!(!is_noop(&stage));
    }
}
