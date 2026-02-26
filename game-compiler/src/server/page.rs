use crate::codegen::CompileOutput;
use super::css::build_css;
use super::toolbar::{build_toolbar, build_warnings_html};
use super::panels::{build_wgsl_viewer, build_api_panel, build_param_monitor, build_editor_panel};
use super::timeline::{build_timeline, build_shortcut_overlay};
use super::inline_js::build_inline_js;
use super::util::{html_escape, serde_json_inline, calc_timeline_duration};

pub(super) fn build_preview_page(output: &CompileOutput, tag_name: &str, source: &str) -> String {
    let has_timeline = !output.arc_moments.is_empty();
    let duration = if has_timeline {
        calc_timeline_duration(&output.arc_moments)
    } else {
        0.0
    };

    let css = build_css(tag_name);
    let toolbar = build_toolbar(output, tag_name, has_timeline);
    let warnings_html = build_warnings_html(&output.warnings);
    let wgsl_viewer = build_wgsl_viewer(output);
    let api_panel = build_api_panel(output, tag_name);
    let param_monitor = build_param_monitor(output);
    let editor_panel = build_editor_panel(source);
    let timeline_html = build_timeline(output);
    let shortcut_overlay = build_shortcut_overlay(has_timeline);

    let html_full = crate::runtime::wrap_html_full(output);
    let html_json = serde_json_inline(&html_full);
    let inline_js = build_inline_js(output, &html_json, has_timeline, duration, tag_name);

    // Calculate top offset
    let has_warnings = !output.warnings.is_empty();
    let tl_height = if has_timeline { 70 } else { 0 };
    let top_offset = if has_warnings {
        format!("{}px", 36 + 28 + tl_height)
    } else {
        format!("{}px", 36 + tl_height)
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>GAME Dev — {tag_name}</title>
<style>
  :root {{ --top-offset: {top_offset}; }}
  {css}
</style>
</head>
<body>
{toolbar}
{warnings_html}
<div class="xray-bar" id="xray-bar"></div>
<div class="split">
  <div class="panel">
    <div id="pane-preview" class="tab-pane active" style="height:100%">
      <div class="panel-label">html preview</div>
      <iframe id="preview" srcdoc=""></iframe>
    </div>
    <div id="pane-wgsl" class="tab-pane" style="height:100%">
      {wgsl_viewer}
    </div>
    <div id="pane-api" class="tab-pane" style="height:100%">
      {api_panel}
    </div>
    <div id="pane-editor" class="tab-pane" style="height:100%">
      {editor_panel}
    </div>
  </div>
  <div class="divider" id="divider"></div>
  <div class="panel component-panel">
    <div class="panel-label">component</div>
    <div class="component-view">
      <div class="comp-wrapper size-sm" id="comp-wrapper">
        <{tag_name} id="comp"></{tag_name}>
      </div>
    </div>
    <div class="size-bar">
      <button class="active" onclick="setSize('sm', this)">SM</button>
      <button onclick="setSize('md', this)">MD</button>
      <button onclick="setSize('lg', this)">LG</button>
    </div>
    {param_monitor}
  </div>
</div>
{timeline_html}
{shortcut_overlay}
<div class="autopsy-tooltip" id="autopsy-tooltip"></div>
{inline_js}
</body>
</html>"##,
        tag_name = tag_name,
        top_offset = top_offset,
        css = css,
        toolbar = toolbar,
        warnings_html = warnings_html,
        wgsl_viewer = wgsl_viewer,
        api_panel = api_panel,
        editor_panel = editor_panel,
        param_monitor = param_monitor,
        timeline_html = timeline_html,
        shortcut_overlay = shortcut_overlay,
        inline_js = inline_js,
    )
}

pub(super) fn build_error_page(tag_name: &str, error: &str) -> String {
    let escaped = html_escape(error);
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>GAME Dev — Error</title>
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  html, body {{ width: 100%; height: 100%; background: #0A0A0A; color: #A0A0A0;
    font-family: 'JetBrains Mono', monospace; }}
  .error-container {{
    max-width: 640px; margin: 80px auto; padding: 32px;
    border: 1px solid #3F1818; border-radius: 8px;
    background: rgba(239,68,68,0.04);
  }}
  .error-header {{
    display: flex; align-items: center; gap: 10px; margin-bottom: 20px;
    font-size: 13px; color: #EF4444;
  }}
  .error-header .tag {{ color: #D4AF37; }}
  .error-message {{
    font-size: 12px; color: #EF4444; line-height: 1.7;
    white-space: pre-wrap; word-break: break-word;
    padding: 16px; background: #141414; border-radius: 4px;
    border-left: 3px solid #EF4444;
  }}
  .pulse {{
    margin-top: 24px; font-size: 11px; color: #666;
    display: flex; align-items: center; gap: 8px;
  }}
  .pulse-dot {{
    width: 6px; height: 6px; border-radius: 50%; background: #EF4444;
    animation: pulse 2s ease-in-out infinite;
  }}
  @keyframes pulse {{
    0%, 100% {{ opacity: 0.3; }}
    50% {{ opacity: 1; }}
  }}
</style>
</head>
<body>
<div class="error-container">
  <div class="error-header">
    <span>GAME dev</span>
    <span class="tag">&lt;{tag_name}&gt;</span>
    <span>— compilation error</span>
  </div>
  <div class="error-message">{escaped}</div>
  <div class="pulse">
    <div class="pulse-dot"></div>
    waiting for fix...
  </div>
</div>
</body>
</html>"##,
        tag_name = html_escape(tag_name),
        escaped = escaped,
    )
}
