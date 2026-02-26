use crate::codegen::CompileOutput;
use super::util::{html_escape, format_time, calc_timeline_duration, classify_param, ParamKind};

pub(super) fn build_wgsl_viewer(output: &CompileOutput) -> String {
    let escaped = html_escape(&output.wgsl);
    let line_count = output.wgsl.lines().count();
    let lines_html: String = escaped
        .lines()
        .map(|l| format!(r#"<span class="line">{}</span>"#, if l.is_empty() { " " } else { l }))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<div class="wgsl-viewer">
  <div class="wgsl-header">
    <span>Generated WGSL — {line_count} lines — {uniforms} uniforms</span>
    <button onclick="copyWgsl()">Copy</button>
  </div>
  <pre class="wgsl-code" id="wgsl-code">{lines_html}</pre>
</div>"#,
        line_count = line_count,
        uniforms = output.uniform_float_count,
        lines_html = lines_html,
    )
}

pub(super) fn build_api_panel(output: &CompileOutput, tag_name: &str) -> String {
    let mut html = String::from(r#"<div class="api-panel">"#);

    // Import section
    html.push_str(r#"<div class="api-section"><h3>Import</h3>"#);
    html.push_str(r#"<div class="api-code" onclick="copyInner(this)">&lt;script type="module" src="./component.js"&gt;&lt;/script&gt;</div></div>"#);

    // Usage section
    html.push_str(r#"<div class="api-section"><h3>Usage</h3>"#);
    if output.data_fields.is_empty() {
        html.push_str(&format!(
            r#"<div class="api-code" onclick="copyInner(this)">&lt;{tag}&gt;&lt;/{tag}&gt;</div></div>"#,
            tag = html_escape(tag_name),
        ));
    } else {
        let attrs: String = output
            .data_fields
            .iter()
            .map(|f| format!(r#" data-{f}="0.5""#))
            .collect();
        html.push_str(&format!(
            r#"<div class="api-code" onclick="copyInner(this)">&lt;{tag}{attrs}&gt;&lt;/{tag}&gt;</div></div>"#,
            tag = html_escape(tag_name),
            attrs = html_escape(&attrs),
        ));
    }

    // Data attributes section
    if !output.data_fields.is_empty() {
        html.push_str(r#"<div class="api-section"><h3>Data Attributes</h3><table class="api-table"><tr><th>Name</th><th>Type</th><th>Setter</th></tr>"#);
        for f in &output.data_fields {
            html.push_str(&format!(
                r#"<tr><td>data-{f}</td><td>number (0-1)</td><td>el.{f} = value</td></tr>"#,
            ));
        }
        html.push_str("</table></div>");
    }

    // Capabilities section
    html.push_str(r#"<div class="api-section"><h3>Capabilities</h3><div class="cap-grid">"#);
    let caps = [
        ("audio", output.uses_audio),
        ("mouse", output.uses_mouse),
        ("data", output.uses_data),
    ];
    for (name, on) in &caps {
        let cls = if *on { "cap-badge cap-on" } else { "cap-badge cap-off" };
        html.push_str(&format!(r#"<span class="{cls}">{name}</span>"#));
    }
    let mode_label = match &output.render_mode {
        crate::codegen::RenderMode::Flat => "flat".to_string(),
        crate::codegen::RenderMode::Raymarch { .. } => "raymarch".to_string(),
    };
    html.push_str(&format!(
        r#"<span class="cap-badge cap-on">{mode_label}</span>"#
    ));
    html.push_str("</div></div>");

    // Parameters section
    if !output.params.is_empty() {
        html.push_str(r#"<div class="api-section"><h3>Parameters</h3><table class="api-table"><tr><th>Name</th><th>Base</th><th>Modulation</th></tr>"#);
        for p in &output.params {
            let mod_str = match &p.mod_js {
                Some(js) => html_escape(js),
                None => "—".to_string(),
            };
            html.push_str(&format!(
                r#"<tr><td>{name}</td><td>{base:.2}</td><td>{mod_str}</td></tr>"#,
                name = p.name,
                base = p.base_value,
                mod_str = mod_str,
            ));
        }
        html.push_str("</table></div>");
    }

    // Timeline section
    if !output.arc_moments.is_empty() {
        let duration = calc_timeline_duration(&output.arc_moments);
        html.push_str(&format!(
            r#"<div class="api-section"><h3>Timeline</h3><p style="color:#A0A0A0">{} moment{}, total duration: {}</p></div>"#,
            output.arc_moments.len(),
            if output.arc_moments.len() == 1 { "" } else { "s" },
            format_time(duration),
        ));
    }

    html.push_str("</div>");
    html
}

pub(super) fn build_param_monitor(output: &CompileOutput) -> String {
    if output.params.is_empty() && output.data_fields.is_empty() {
        return String::new();
    }
    let mut html = String::from(r#"<div class="param-monitor">"#);

    // Data signal sliders
    if !output.data_fields.is_empty() {
        html.push_str(&format!(
            r#"<div class="param-title">Data Signals ({})</div>"#,
            output.data_fields.len()
        ));
        for field in &output.data_fields {
            html.push_str(&format!(
                r#"<div class="param-row"><span class="param-dot data"></span><span class="param-name" title="{f}">{f}</span><div class="param-slider"><input type="range" min="0" max="1" step="0.01" value="0.5" data-field="{f}"></div><span class="param-val">0.50</span></div>"#,
                f = field,
            ));
        }
    }

    // Parameter list
    if !output.params.is_empty() {
        html.push_str(&format!(
            r#"<div class="param-title" style="margin-top:8px">Parameters ({})</div>"#,
            output.params.len()
        ));
    }

    for (i, param) in output.params.iter().enumerate() {
        let kind = classify_param(param, &output.data_fields, &output.arc_moments, i);
        let (dot_cls, row_content) = match kind {
            ParamKind::Data => {
                let slider = format!(
                    r#"<div class="param-slider"><input type="range" min="0" max="1" step="0.01" value="0.5" data-field="{}" data-index="{}"></div><span class="param-val" data-live="{}">0.50</span>"#,
                    param.name, i, i,
                );
                ("data", slider)
            }
            ParamKind::Modulated => {
                let expr = param.mod_js.as_deref().unwrap_or("");
                (
                    "modulated",
                    format!(
                        r#"<span class="param-expr" title="{}">{}</span><span class="param-val" data-live="{}">—</span>"#,
                        html_escape(expr),
                        html_escape(expr),
                        i,
                    ),
                )
            }
            ParamKind::Arc => (
                "arc",
                format!(
                    r#"<span class="param-badge">arc</span><span class="param-val" data-live="{}">—</span>"#,
                    i,
                ),
            ),
            ParamKind::Static => (
                "static",
                format!(
                    r#"<span class="param-val">{:.2}</span>"#,
                    param.base_value
                ),
            ),
        };

        html.push_str(&format!(
            r#"<div class="param-row"><span class="param-dot {dot_cls}"></span><span class="param-name" title="{name}">{name}</span>{row_content}</div>"#,
            dot_cls = dot_cls,
            name = param.name,
            row_content = row_content,
        ));
    }
    html.push_str("</div>");
    html
}

pub(super) fn build_editor_panel(source: &str) -> String {
    let escaped = html_escape(source);
    format!(
        r#"<div class="editor-panel">
  <div class="editor-header">
    <span>.game source</span>
    <span class="editor-status" id="editor-status"></span>
    <button class="editor-save" id="editor-save" onclick="editorSave()">Save (Ctrl+S)</button>
  </div>
  <textarea id="editor-source" class="editor-textarea" spellcheck="false">{escaped}</textarea>
</div>"#,
        escaped = escaped,
    )
}
