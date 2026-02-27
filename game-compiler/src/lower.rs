//! Lower AST → IR.
//!
//! Converts an expanded, validated Cinematic AST into the intermediate
//! representation. This adds stage classification, uniform metadata,
//! and signal usage tracking that the optimizer needs.
//!
//! Expects define-expanded input (call `expand_defines()` first).

use crate::ast::{self, BlendMode, Cinematic, Expr, Layer};
use crate::codegen::{CompiledParam, RenderMode};
use crate::ir::*;

/// Lower a (define-expanded) Cinematic AST into a ShaderIR.
///
/// `params` should be collected from the WgslGen `collect_params()` step
/// so that uniform indices are already assigned.
pub fn lower(cinematic: &Cinematic, params: &[CompiledParam], render_mode: &RenderMode) -> ShaderIR {
    let title = cinematic
        .name
        .clone()
        .unwrap_or_else(|| "Untitled".to_string());

    let mut uses_audio = false;
    let mut uses_mouse = false;
    let mut uses_data = false;
    let mut data_fields = Vec::new();

    // Lower layers
    let layers: Vec<IrLayer> = cinematic
        .layers
        .iter()
        .map(|layer| lower_layer(layer))
        .collect();

    // Scan for signal usage across all layer params and stage args
    for layer in &layers {
        for param in &layer.params {
            if let Some(ref mod_expr) = param.modulation {
                check_signals(mod_expr, &mut uses_audio, &mut uses_mouse, &mut uses_data, &mut data_fields);
            }
        }
        for stage in &layer.stages {
            for arg in &stage.args {
                let expr = match arg {
                    IrArg::Positional(e) => e,
                    IrArg::Named { value, .. } => value,
                };
                check_signals(expr, &mut uses_audio, &mut uses_mouse, &mut uses_data, &mut data_fields);
            }
        }
    }

    // Lower uniforms from CompiledParams
    let uniforms: Vec<IrUniform> = params
        .iter()
        .map(|p| IrUniform {
            name: p.name.clone(),
            field_name: p.uniform_field.clone(),
            index: p.buffer_index,
            base_value: p.base_value,
            mod_js: p.mod_js.clone(),
            dead: false,
        })
        .collect();

    let ir_render_mode = match render_mode {
        RenderMode::Flat => IrRenderMode::Flat,
        RenderMode::Raymarch {
            cam_radius,
            cam_height,
            cam_speed,
        } => IrRenderMode::Raymarch {
            cam_radius: *cam_radius,
            cam_height: *cam_height,
            cam_speed: *cam_speed,
        },
    };

    ShaderIR {
        title,
        layers,
        uniforms,
        render_mode: ir_render_mode,
        uses_audio,
        uses_mouse,
        uses_data,
        data_fields,
        warnings: Vec::new(),
    }
}

// ── Layer lowering ────────────────────────────────────────────────────

fn lower_layer(layer: &Layer) -> IrLayer {
    let stages = match &layer.fn_chain {
        Some(chain) => chain
            .stages
            .iter()
            .map(|call| lower_fn_call(call))
            .collect(),
        None => Vec::new(),
    };

    let params = layer
        .params
        .iter()
        .map(|p| IrParam {
            name: p.name.clone(),
            base_value: lower_expr(&p.base_value),
            modulation: p.modulation.as_ref().map(|m| lower_expr(&m.signal)),
        })
        .collect();

    let properties = layer
        .properties
        .iter()
        .map(|p| IrProperty {
            name: p.name.clone(),
            value: lower_expr(&p.value),
        })
        .collect();

    let blend_mode = layer.blend_mode.clone().unwrap_or(BlendMode::Additive);
    let blend_opacity = layer.blend_opacity.unwrap_or(1.0);

    IrLayer {
        name: layer.name.clone(),
        stages,
        blend_mode,
        blend_opacity,
        params,
        properties,
    }
}

// ── Stage lowering ────────────────────────────────────────────────────

fn lower_fn_call(call: &ast::FnCall) -> IrStage {
    let kind = StageKind::classify(&call.name).unwrap_or(StageKind::Color);

    let args = call
        .args
        .iter()
        .map(|arg| match arg {
            ast::Arg::Positional(expr) => IrArg::Positional(lower_expr(expr)),
            ast::Arg::Named { name, value } => IrArg::Named {
                name: name.clone(),
                value: lower_expr(value),
            },
        })
        .collect();

    IrStage {
        kind,
        name: call.name.clone(),
        args,
        span: call.span.clone(),
    }
}

// ── Expression lowering ───────────────────────────────────────────────

fn lower_expr(expr: &Expr) -> IrExpr {
    match expr {
        Expr::Number(v) => IrExpr::Literal(*v),
        Expr::String(s) => IrExpr::String(s.clone()),
        Expr::Ident(name) => IrExpr::Ident(name.clone()),
        Expr::FieldAccess { object, field } => IrExpr::FieldAccess {
            object: Box::new(lower_expr(object)),
            field: field.clone(),
        },
        Expr::BinaryOp { left, op, right } => IrExpr::BinOp {
            left: Box::new(lower_expr(left)),
            op: *op,
            right: Box::new(lower_expr(right)),
        },
        Expr::Negate(inner) => IrExpr::Neg(Box::new(lower_expr(inner))),
        Expr::Call(call) => IrExpr::Call {
            name: call.name.clone(),
            args: call
                .args
                .iter()
                .map(|a| match a {
                    ast::Arg::Positional(e) => lower_expr(e),
                    ast::Arg::Named { value, .. } => lower_expr(value),
                })
                .collect(),
        },
        Expr::Array(elems) => IrExpr::Array(elems.iter().map(lower_expr).collect()),
        Expr::Ternary {
            condition,
            if_true,
            if_false,
        } => IrExpr::Ternary {
            condition: Box::new(lower_expr(condition)),
            if_true: Box::new(lower_expr(if_true)),
            if_false: Box::new(lower_expr(if_false)),
        },
    }
}

// ── Signal detection ──────────────────────────────────────────────────

fn check_signals(
    expr: &IrExpr,
    uses_audio: &mut bool,
    uses_mouse: &mut bool,
    uses_data: &mut bool,
    data_fields: &mut Vec<String>,
) {
    match expr {
        IrExpr::FieldAccess { object, field } => {
            if let IrExpr::Ident(root) = object.as_ref() {
                match root.as_str() {
                    "audio" => *uses_audio = true,
                    "mouse" => *uses_mouse = true,
                    "data" => {
                        *uses_data = true;
                        if !data_fields.contains(field) {
                            data_fields.push(field.clone());
                        }
                    }
                    _ => {}
                }
            }
            check_signals(object, uses_audio, uses_mouse, uses_data, data_fields);
        }
        IrExpr::BinOp { left, right, .. } => {
            check_signals(left, uses_audio, uses_mouse, uses_data, data_fields);
            check_signals(right, uses_audio, uses_mouse, uses_data, data_fields);
        }
        IrExpr::Neg(inner) => {
            check_signals(inner, uses_audio, uses_mouse, uses_data, data_fields);
        }
        IrExpr::Call { args, .. } => {
            for a in args {
                check_signals(a, uses_audio, uses_mouse, uses_data, data_fields);
            }
        }
        IrExpr::Array(elems) => {
            for e in elems {
                check_signals(e, uses_audio, uses_mouse, uses_data, data_fields);
            }
        }
        IrExpr::Ternary {
            condition,
            if_true,
            if_false,
        } => {
            check_signals(condition, uses_audio, uses_mouse, uses_data, data_fields);
            check_signals(if_true, uses_audio, uses_mouse, uses_data, data_fields);
            check_signals(if_false, uses_audio, uses_mouse, uses_data, data_fields);
        }
        IrExpr::Literal(_) | IrExpr::String(_) | IrExpr::Ident(_) => {}
    }
}
