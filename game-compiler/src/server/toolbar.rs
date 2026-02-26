use crate::codegen::CompileOutput;
use super::util::html_escape;

pub(super) fn build_toolbar(output: &CompileOutput, tag_name: &str, has_timeline: bool) -> String {
    // Badges
    let mut badges = Vec::new();
    badges.push(r#"<span class="label">GAME dev</span>"#.to_string());
    badges.push(format!(
        r#"<span class="sep">|</span><span class="tag">&lt;{}&gt;</span>"#,
        html_escape(tag_name)
    ));
    if !output.params.is_empty() {
        badges.push(format!(
            r#"<span class="badge">{} param{}</span>"#,
            output.params.len(),
            if output.params.len() == 1 { "" } else { "s" }
        ));
    }
    if !output.data_fields.is_empty() {
        badges.push(format!(
            r#"<span class="badge">{} data</span>"#,
            output.data_fields.len()
        ));
    }
    if output.uses_audio {
        badges.push(r#"<span class="badge">audio</span>"#.to_string());
    }
    let mode = match &output.render_mode {
        crate::codegen::RenderMode::Flat => "flat",
        crate::codegen::RenderMode::Raymarch { .. } => "raymarch",
    };
    badges.push(format!(r#"<span class="badge">{mode}</span>"#));
    if !output.warnings.is_empty() {
        badges.push(format!(
            r#"<span class="badge" style="color:#F59E0B">&#9888; {}</span>"#,
            output.warnings.len()
        ));
    }

    // Tab bar
    let tab_bar = r#"<div class="tab-bar">
  <button class="tab-btn active" data-tab="preview">Preview</button>
  <button class="tab-btn" data-tab="wgsl">WGSL</button>
  <button class="tab-btn" data-tab="api">API</button>
  <button class="tab-btn" data-tab="editor">Editor</button>
</div>"#;

    // Actions
    let pause_btn = if has_timeline {
        r#"<button id="pause-btn" onclick="togglePause()" title="Pause timeline">&#9646;&#9646;</button>"#
    } else {
        ""
    };
    let actions = format!(
        r#"<div class="actions">
  <button onclick="copyText('&lt;script type=&quot;module&quot; src=&quot;./component.js&quot;&gt;&lt;/script&gt;', this)" title="Copy import tag">Copy Import</button>
  <button onclick="copyText('&lt;{tag}&gt;&lt;/{tag}&gt;', this)" title="Copy HTML tag">Copy HTML</button>
  <a href="/preview.html" target="_blank" title="Open standalone HTML">Fullscreen</a>
  <div class="export-dropdown">
    <button class="export-trigger" onclick="this.parentElement.classList.toggle('open')">Export</button>
    <div class="export-menu">
      <button onclick="exportPng()">PNG Screenshot</button>
      <button onclick="exportVideo()">Video (5s WebM)</button>
      <a href="/export/react" download="GameComponent.jsx">React Wrapper</a>
      <a href="/export/vue" download="GameComponent.vue">Vue Wrapper</a>
      <a href="/export/css" download="fallback.css">CSS Fallback</a>
    </div>
  </div>
  {pause_btn}
  <button onclick="toggleShortcuts()" title="Keyboard shortcuts">?</button>
</div>"#,
        tag = html_escape(tag_name),
        pause_btn = pause_btn,
    );

    format!(
        r#"<div class="toolbar">
  {badges}
  {tab_bar}
  {actions}
</div>"#,
        badges = badges.join("\n  "),
        tab_bar = tab_bar,
        actions = actions,
    )
}

pub(super) fn build_warnings_html(warnings: &[String]) -> String {
    if warnings.is_empty() {
        return String::new();
    }
    let mut html = String::from(r#"<div class="warnings-bar" id="warnings-bar" onclick="this.classList.toggle('open')">"#);
    html.push_str(&format!(
        r#"<div class="warnings-header"><span class="arrow">&#9654;</span> {} compiler warning{} — click to expand</div>"#,
        warnings.len(),
        if warnings.len() == 1 { "" } else { "s" }
    ));
    html.push_str(r#"<div class="warnings-list">"#);
    for w in warnings {
        html.push_str(&format!(
            r#"<div class="warn-item">{}</div>"#,
            html_escape(w)
        ));
    }
    html.push_str("</div></div>");
    html
}
