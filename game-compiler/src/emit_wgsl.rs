//! IR → AST reconstruction.
//!
//! Converts an optimized ShaderIR back into a Cinematic AST that the
//! existing WgslGen pipeline can emit as WGSL. This roundtrip approach
//! lets the optimizer add value without requiring a full emission rewrite.
//!
//! Dead uniforms are excluded from the reconstructed AST, so the
//! WgslGen's `collect_params()` step naturally omits them.

use crate::ast::{
    Arg, BlendMode, Cinematic, Expr, FnCall, Layer, Modulation, ParamDecl, PipeChain, Property,
};
use crate::codegen::{CompiledParam, RenderMode};
use crate::ir::*;

/// Reconstruct an optimized Cinematic AST from the IR.
///
/// Returns (cinematic, params, render_mode) — the params list has dead
/// uniforms removed so the downstream pipeline uses the optimized set.
pub fn reconstruct(ir: &ShaderIR) -> (Cinematic, Vec<CompiledParam>, RenderMode) {
    let layers: Vec<Layer> = ir.layers.iter().map(reconstruct_layer).collect();

    // Filter out dead uniforms and rebuild CompiledParams with correct indices
    let live_uniforms: Vec<&IrUniform> = ir.uniforms.iter().filter(|u| !u.dead).collect();
    let system_float_count = 10; // matches SYSTEM_FLOAT_COUNT in codegen/mod.rs

    let params: Vec<CompiledParam> = live_uniforms
        .iter()
        .enumerate()
        .map(|(i, u)| CompiledParam {
            name: u.name.clone(),
            uniform_field: u.field_name.clone(),
            buffer_index: system_float_count + i,
            base_value: u.base_value,
            mod_js: u.mod_js.clone(),
        })
        .collect();

    let render_mode = match &ir.render_mode {
        IrRenderMode::Flat => RenderMode::Flat,
        IrRenderMode::Raymarch {
            cam_radius,
            cam_height,
            cam_speed,
        } => RenderMode::Raymarch {
            cam_radius: *cam_radius,
            cam_height: *cam_height,
            cam_speed: *cam_speed,
        },
    };

    let cinematic = Cinematic {
        name: Some(ir.title.clone()),
        imports: Vec::new(),
        properties: Vec::new(),
        layers,
        lenses: Vec::new(),     // lens already parsed into render_mode
        arc: None,              // arc compiled separately from original AST
        react: None,            // react compiled separately from original AST
        resonance: None,        // resonance compiled separately from original AST
        defines: Vec::new(),    // defines already expanded
    };

    (cinematic, params, render_mode)
}

// ── Layer reconstruction ──────────────────────────────────────────────

fn reconstruct_layer(layer: &IrLayer) -> Layer {
    let fn_chain = if layer.stages.is_empty() {
        None
    } else {
        Some(PipeChain {
            stages: layer.stages.iter().map(reconstruct_stage).collect(),
        })
    };

    let params = layer
        .params
        .iter()
        .map(|p| ParamDecl {
            name: p.name.clone(),
            base_value: reconstruct_expr(&p.base_value),
            modulation: p
                .modulation
                .as_ref()
                .map(|m| Modulation {
                    signal: reconstruct_expr(m),
                }),
        })
        .collect();

    let properties = layer
        .properties
        .iter()
        .map(|p| Property {
            name: p.name.clone(),
            value: reconstruct_expr(&p.value),
        })
        .collect();

    let blend_mode = match layer.blend_mode {
        BlendMode::Additive => None, // default
        other => Some(other),
    };

    Layer {
        name: layer.name.clone(),
        fn_chain,
        params,
        properties,
        blend_mode,
        blend_opacity: if (layer.blend_opacity - 1.0).abs() < 1e-10 {
            None
        } else {
            Some(layer.blend_opacity)
        },
    }
}

// ── Stage reconstruction ──────────────────────────────────────────────

fn reconstruct_stage(stage: &IrStage) -> FnCall {
    FnCall {
        name: stage.name.clone(),
        args: stage.args.iter().map(reconstruct_arg).collect(),
        span: stage.span.clone(),
    }
}

fn reconstruct_arg(arg: &IrArg) -> Arg {
    match arg {
        IrArg::Positional(expr) => Arg::Positional(reconstruct_expr(expr)),
        IrArg::Named { name, value } => Arg::Named {
            name: name.clone(),
            value: reconstruct_expr(value),
        },
    }
}

// ── Expression reconstruction ─────────────────────────────────────────

fn reconstruct_expr(ir_expr: &IrExpr) -> Expr {
    match ir_expr {
        IrExpr::Literal(v) => Expr::Number(*v),
        IrExpr::String(s) => Expr::String(s.clone()),
        IrExpr::Ident(name) => Expr::Ident(name.clone()),
        IrExpr::FieldAccess { object, field } => Expr::FieldAccess {
            object: Box::new(reconstruct_expr(object)),
            field: field.clone(),
        },
        IrExpr::BinOp { left, op, right } => Expr::BinaryOp {
            left: Box::new(reconstruct_expr(left)),
            op: *op,
            right: Box::new(reconstruct_expr(right)),
        },
        IrExpr::Neg(inner) => Expr::Negate(Box::new(reconstruct_expr(inner))),
        IrExpr::Call { name, args } => Expr::Call(FnCall {
            name: name.clone(),
            args: args.iter().map(|a| Arg::Positional(reconstruct_expr(a))).collect(),
            span: None,
        }),
        IrExpr::Array(elems) => Expr::Array(elems.iter().map(reconstruct_expr).collect()),
        IrExpr::Ternary {
            condition,
            if_true,
            if_false,
        } => Expr::Ternary {
            condition: Box::new(reconstruct_expr(condition)),
            if_true: Box::new(reconstruct_expr(if_true)),
            if_false: Box::new(reconstruct_expr(if_false)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::BinOp;

    #[test]
    fn roundtrip_literal() {
        let ir = IrExpr::Literal(3.14);
        let ast = reconstruct_expr(&ir);
        assert!(matches!(ast, Expr::Number(v) if (v - 3.14).abs() < 1e-10));
    }

    #[test]
    fn roundtrip_binop() {
        let ir = IrExpr::BinOp {
            left: Box::new(IrExpr::Ident("time".into())),
            op: BinOp::Mul,
            right: Box::new(IrExpr::Literal(0.5)),
        };
        let ast = reconstruct_expr(&ir);
        match ast {
            Expr::BinaryOp { left, op, right } => {
                assert!(matches!(*left, Expr::Ident(ref n) if n == "time"));
                assert_eq!(op, BinOp::Mul);
                assert!(matches!(*right, Expr::Number(v) if (v - 0.5).abs() < 1e-10));
            }
            _ => panic!("expected BinaryOp"),
        }
    }

    #[test]
    fn dead_uniforms_excluded() {
        let ir = ShaderIR {
            title: "Test".into(),
            layers: Vec::new(),
            uniforms: vec![
                IrUniform {
                    name: "alive".into(),
                    field_name: "p_alive".into(),
                    index: 10,
                    base_value: 1.0,
                    mod_js: None,
                    dead: false,
                },
                IrUniform {
                    name: "dead_one".into(),
                    field_name: "p_dead_one".into(),
                    index: 11,
                    base_value: 0.0,
                    mod_js: None,
                    dead: true,
                },
            ],
            render_mode: IrRenderMode::Flat,
            uses_audio: false,
            uses_mouse: false,
            uses_data: false,
            data_fields: Vec::new(),
            warnings: Vec::new(),
        };

        let (_, params, _) = reconstruct(&ir);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "alive");
        assert_eq!(params[0].buffer_index, 10); // system_float_count + 0
    }
}
