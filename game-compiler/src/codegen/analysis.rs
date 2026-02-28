use std::collections::HashMap;

use crate::ast::*;

use super::RenderMode;

// ── Define expansion ──────────────────────────────────────────────────

/// Expand all define calls in pipe chains (macro-style inlining).
/// Mutates the cinematic in place, replacing define calls with their
/// expanded bodies after parameter substitution.
///
/// Uses multi-pass expansion to handle nested defines (define A calls
/// define B). Runs up to 10 passes or until no more expansions occur
/// (fixpoint).
pub(super) fn expand_defines(cinematic: &mut Cinematic) {
    if cinematic.defines.is_empty() {
        return;
    }

    let defines: HashMap<String, DefineBlock> = cinematic.defines.iter()
        .map(|d| (d.name.clone(), d.clone()))
        .collect();

    // Multi-pass expansion for nested defines (max 10 iterations)
    for _pass in 0..10 {
        let mut any_expanded = false;
        for layer in &mut cinematic.layers {
            if let Some(chain) = &mut layer.fn_chain {
                if expand_chain(chain, &defines) {
                    any_expanded = true;
                }
            }
        }
        if !any_expanded {
            break;
        }
    }
}

/// Expand define calls in a pipe chain. Returns `true` if any expansion
/// was performed (indicating another pass may be needed for nested defines).
fn expand_chain(chain: &mut PipeChain, defines: &HashMap<String, DefineBlock>) -> bool {
    let mut new_stages = Vec::new();
    let mut any_expanded = false;

    for stage in &chain.stages {
        if let Some(define) = defines.get(&stage.name) {
            any_expanded = true;
            // Build substitution map: formal param → actual argument expr
            let subs: HashMap<&str, Expr> = define.params.iter()
                .zip(stage.args.iter())
                .map(|(formal, actual)| {
                    let expr = match actual {
                        Arg::Positional(e) => e.clone(),
                        Arg::Named { value, .. } => value.clone(),
                    };
                    (formal.as_str(), expr)
                })
                .collect();

            // Clone body stages and substitute formal params with actual args
            for body_stage in &define.body.stages {
                let mut expanded = body_stage.clone();
                for arg in &mut expanded.args {
                    match arg {
                        Arg::Positional(e) => substitute_expr(e, &subs),
                        Arg::Named { value, .. } => substitute_expr(value, &subs),
                    }
                }
                new_stages.push(expanded);
            }
        } else {
            new_stages.push(stage.clone());
        }
    }

    chain.stages = new_stages;
    any_expanded
}

fn substitute_expr(expr: &mut Expr, subs: &HashMap<&str, Expr>) {
    match expr {
        Expr::Ident(name) => {
            if let Some(replacement) = subs.get(name.as_str()) {
                *expr = replacement.clone();
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            substitute_expr(left, subs);
            substitute_expr(right, subs);
        }
        Expr::Negate(inner) => substitute_expr(inner, subs),
        Expr::Call(call) => {
            for arg in &mut call.args {
                match arg {
                    Arg::Positional(e) => substitute_expr(e, subs),
                    Arg::Named { value, .. } => substitute_expr(value, subs),
                }
            }
        }
        Expr::FieldAccess { object, .. } => substitute_expr(object, subs),
        Expr::Array(elements) => {
            for e in elements {
                substitute_expr(e, subs);
            }
        }
        Expr::Ternary { condition, if_true, if_false } => {
            substitute_expr(condition, subs);
            substitute_expr(if_true, subs);
            substitute_expr(if_false, subs);
        }
        Expr::Number(_) | Expr::String(_) => {}
    }
}

/// Check if an expression references audio signals.
pub(super) fn expr_uses_audio(expr: &Expr) -> bool {
    match expr {
        Expr::Ident(s) => s == "audio",
        Expr::FieldAccess { object, .. } => expr_uses_audio(object),
        Expr::BinaryOp { left, right, .. } => expr_uses_audio(left) || expr_uses_audio(right),
        Expr::Negate(inner) => expr_uses_audio(inner),
        Expr::Call(call) => call.args.iter().any(|a| match a {
            Arg::Positional(e) | Arg::Named { value: e, .. } => expr_uses_audio(e),
        }),
        _ => false,
    }
}

/// Check if an expression references data signals.
pub(super) fn expr_uses_data(expr: &Expr) -> bool {
    match expr {
        Expr::Ident(s) => s == "data",
        Expr::FieldAccess { object, .. } => expr_uses_data(object),
        Expr::BinaryOp { left, right, .. } => expr_uses_data(left) || expr_uses_data(right),
        Expr::Negate(inner) => expr_uses_data(inner),
        Expr::Call(call) => call.args.iter().any(|a| match a {
            Arg::Positional(e) | Arg::Named { value: e, .. } => expr_uses_data(e),
        }),
        _ => false,
    }
}

/// Collect `data.*` field names from an expression.
pub(super) fn collect_data_fields_into(expr: &Expr, fields: &mut Vec<String>) {
    match expr {
        Expr::FieldAccess { object, field } => {
            if let Expr::Ident(obj) = object.as_ref() {
                if obj == "data" && !fields.contains(field) {
                    fields.push(field.clone());
                }
            }
            collect_data_fields_into(object, fields);
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_data_fields_into(left, fields);
            collect_data_fields_into(right, fields);
        }
        Expr::Negate(inner) => collect_data_fields_into(inner, fields),
        Expr::Call(call) => {
            for arg in &call.args {
                match arg {
                    Arg::Positional(e) | Arg::Named { value: e, .. } => {
                        collect_data_fields_into(e, fields);
                    }
                }
            }
        }
        _ => {}
    }
}

/// Check if an expression references mouse signals.
pub(super) fn expr_uses_mouse(expr: &Expr) -> bool {
    match expr {
        Expr::Ident(s) => s == "mouse",
        Expr::FieldAccess { object, .. } => expr_uses_mouse(object),
        Expr::BinaryOp { left, right, .. } => expr_uses_mouse(left) || expr_uses_mouse(right),
        Expr::Negate(inner) => expr_uses_mouse(inner),
        Expr::Call(call) => call.args.iter().any(|a| match a {
            Arg::Positional(e) | Arg::Named { value: e, .. } => expr_uses_mouse(e),
        }),
        _ => false,
    }
}

/// Extract a numeric value from an expression (for base_value).
pub(super) fn extract_number(expr: &Expr) -> Option<f64> {
    match expr {
        Expr::Number(n) => Some(*n),
        Expr::Negate(inner) => extract_number(inner).map(|n| -n),
        _ => None,
    }
}

/// Extract audio file path from cinematic properties.
pub(super) fn extract_audio_file(cinematic: &Cinematic) -> Option<String> {
    cinematic.properties.iter().find_map(|p| {
        if p.name == "audio" {
            if let Expr::String(s) = &p.value {
                return Some(s.clone());
            }
        }
        None
    })
}

/// Determine rendering mode from the first lens block.
pub(super) fn determine_render_mode(cinematic: &Cinematic) -> RenderMode {
    if let Some(lens) = cinematic.lenses.first() {
        // Check for mode: raymarch
        let is_raymarch = lens.properties.iter().any(|p| {
            p.name == "mode" && matches!(&p.value, Expr::Ident(s) if s == "raymarch")
        });

        if is_raymarch {
            // Extract camera params
            let (radius, height, speed) = extract_camera_params(lens);
            return RenderMode::Raymarch {
                cam_radius: radius,
                cam_height: height,
                cam_speed: speed,
            };
        }
    }
    RenderMode::Flat
}

fn extract_camera_params(lens: &Lens) -> (f64, f64, f64) {
    for prop in &lens.properties {
        if prop.name == "camera" {
            if let Expr::Call(call) = &prop.value {
                if call.name == "orbit" {
                    let radius = extract_named_number(&call.args, "radius", 5.0);
                    let height = extract_named_number(&call.args, "height", 2.0);
                    let speed = extract_named_number(&call.args, "speed", 0.05);
                    return (radius, height, speed);
                }
            }
        }
    }
    (5.0, 2.0, 0.05)
}

fn extract_named_number(args: &[Arg], name: &str, default: f64) -> f64 {
    for arg in args {
        if let Arg::Named { name: n, value } = arg {
            if n == name {
                return extract_number(value).unwrap_or(default);
            }
        }
    }
    default
}
