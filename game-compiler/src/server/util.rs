use crate::codegen::{CompiledMoment, CompiledParam};

pub(super) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

pub(super) fn format_time(secs: f64) -> String {
    let m = (secs / 60.0) as u32;
    let s = (secs % 60.0) as u32;
    format!("{m}:{s:02}")
}

pub(super) fn calc_timeline_duration(moments: &[CompiledMoment]) -> f64 {
    moments
        .iter()
        .map(|m| {
            let max_t = m
                .transitions
                .iter()
                .filter_map(|t| t.duration_secs)
                .fold(0.0f64, f64::max);
            m.time_seconds + max_t
        })
        .fold(0.0f64, f64::max)
}

pub(super) enum ParamKind {
    Data,
    Modulated,
    Arc,
    Static,
}

pub(super) fn classify_param(
    param: &CompiledParam,
    data_fields: &[String],
    arc_moments: &[CompiledMoment],
    param_index: usize,
) -> ParamKind {
    let in_arc = arc_moments
        .iter()
        .any(|m| m.transitions.iter().any(|t| t.param_index == param_index));
    if in_arc {
        return ParamKind::Arc;
    }
    if param.mod_js.is_some() {
        return ParamKind::Modulated;
    }
    if data_fields.contains(&param.name) {
        return ParamKind::Data;
    }
    ParamKind::Static
}

/// Simple JSON string encoding (avoid serde dependency just for this).
/// Also escapes `</` as `<\/` to prevent HTML parser from closing a
/// `<script>` block when this string is embedded in inline JS.
pub(super) fn serde_json_inline(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    let mut prev = '\0';
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '/' if prev == '<' => out.push_str("\\/"),
            c if c < '\x20' => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
        prev = c;
    }
    out.push('"');
    out
}
