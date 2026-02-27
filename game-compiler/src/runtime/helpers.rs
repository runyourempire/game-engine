//! Shared JS generation helpers used by both HTML and Web Component outputs.

use crate::codegen::CompiledParam;

/// Generate JS code to initialize the params array.
pub(super) fn generate_param_init_js(params: &[CompiledParam]) -> String {
    if params.is_empty() {
        return "const params = [];".to_string();
    }

    let mut lines = Vec::new();
    lines.push("const params = [".to_string());
    for p in params {
        let mod_expr = p.mod_js.as_deref().unwrap_or("0");
        lines.push(format!(
            "  {{ name: '{}', base: {}, modExpr: (audioBass, audioMid, audioTreble, audioEnergy, audioBeat, time, mouseX, mouseY) => {}, value: {} }},",
            p.name, p.base_value, mod_expr, p.base_value
        ));
    }
    lines.push("];".to_string());
    lines.join("\n")
}

/// Generate JS code to update param values each frame.
pub(super) fn generate_param_update_js(params: &[CompiledParam]) -> String {
    if params.is_empty() {
        return String::new();
    }

    let mut lines = Vec::new();
    for (i, _p) in params.iter().enumerate() {
        lines.push(format!(
            "  params[{i}].value = params[{i}].base + params[{i}].modExpr(audioBass, audioMid, audioTreble, audioEnergy, audioBeat, time, mouseX, mouseY);"
        ));
    }
    lines.join("\n")
}

/// Generate JS console.warn() calls for compiler warnings.
pub(super) fn generate_warnings_js(warnings: &[String]) -> String {
    if warnings.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n// ── Compiler warnings ─────────────────────────────────────────\n");
    for w in warnings {
        let escaped = w.replace('\\', "\\\\").replace('\'', "\\'");
        out.push_str(&format!("console.warn('[GAME]', '{escaped}');\n"));
    }
    out
}

/// Generate JS variable declarations for `data.*` signals.
pub(super) fn generate_data_vars_js(data_fields: &[String]) -> String {
    if data_fields.is_empty() {
        return String::new();
    }
    let mut lines = Vec::new();
    for field in data_fields {
        lines.push(format!("let data_{field} = 0;"));
    }
    lines.join("\n")
}
