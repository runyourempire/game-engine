use crate::ast::*;

use super::RenderMode;

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
