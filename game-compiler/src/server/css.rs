pub(super) fn build_css(tag_name: &str) -> String {
    format!(
        r#"* {{ margin: 0; padding: 0; box-sizing: border-box; }}
html, body {{ width: 100%; height: 100%; background: #0A0A0A; color: #A0A0A0;
  font-family: 'JetBrains Mono', monospace; font-size: 12px; }}

/* ── Toolbar ─────────────────────────────────────── */
.toolbar {{
  height: 36px; display: flex; align-items: center; padding: 0 14px;
  border-bottom: 1px solid #2A2A2A; font-size: 11px; gap: 10px;
  background: #0A0A0A; user-select: none;
}}
.toolbar .label {{ color: #666; }}
.toolbar .tag {{ color: #D4AF37; font-weight: 600; }}
.toolbar .badge {{
  color: #A0A0A0; background: #1F1F1F; padding: 2px 8px;
  border-radius: 3px; font-size: 10px;
}}
.toolbar .sep {{ color: #333; }}
.toolbar .actions {{ margin-left: auto; display: flex; gap: 6px; }}
.toolbar .actions button, .toolbar .actions a {{
  background: #1F1F1F; border: 1px solid #2A2A2A; color: #A0A0A0;
  padding: 3px 10px; border-radius: 3px; font-size: 10px; cursor: pointer;
  font-family: inherit; text-decoration: none; display: inline-flex;
  align-items: center;
}}
.toolbar .actions button:hover, .toolbar .actions a:hover {{
  color: #FFF; border-color: #444;
}}
.toolbar .actions button.copied {{
  color: #22C55E; border-color: #22C55E; transition: none;
}}

/* ── Tab system ──────────────────────────────────── */
.tab-bar {{ display: flex; gap: 2px; }}
.tab-btn {{
  background: transparent; border: 1px solid transparent; color: #666;
  padding: 3px 12px; border-radius: 3px 3px 0 0; font-size: 10px;
  cursor: pointer; font-family: inherit; transition: color 0.15s;
}}
.tab-btn:hover {{ color: #A0A0A0; }}
.tab-btn.active {{ color: #D4AF37; border-color: #2A2A2A; border-bottom-color: #0A0A0A; background: #0A0A0A; }}
.tab-pane {{ display: none; }}
.tab-pane.active {{ display: block; }}

/* ── Warnings ────────────────────────────────────── */
.warnings-bar {{
  background: rgba(245,158,11,0.06); border-bottom: 1px solid #2A2A2A;
  padding: 0 14px; cursor: pointer; user-select: none;
}}
.warnings-header {{
  height: 28px; display: flex; align-items: center; gap: 8px;
  font-size: 11px; color: #F59E0B;
}}
.warnings-header .arrow {{ transition: transform 0.15s; display: inline-block; }}
.warnings-bar.open .warnings-header .arrow {{ transform: rotate(90deg); }}
.warnings-list {{
  display: none; padding: 0 0 8px 0;
}}
.warnings-bar.open .warnings-list {{ display: block; }}
.warnings-list .warn-item {{
  padding: 4px 0 4px 12px; font-size: 11px; color: #F59E0B;
  border-left: 2px solid #F59E0B; margin: 4px 0;
}}

/* ── Split view ──────────────────────────────────── */
.split {{
  display: grid; grid-template-columns: 1fr 4px 1fr;
  height: calc(100vh - var(--top-offset, 36px));
}}
.panel {{ position: relative; overflow: hidden; }}
.panel-label {{
  position: absolute; top: 8px; left: 12px; font-size: 10px; color: #444;
  z-index: 10; text-transform: uppercase; letter-spacing: 1px;
}}
.divider {{
  background: #2A2A2A; cursor: col-resize; position: relative;
}}
.divider:hover {{ background: #444; }}
iframe {{ width: 100%; height: 100%; border: none; }}

/* ── WGSL viewer ─────────────────────────────────── */
.wgsl-viewer {{
  height: 100%; overflow: auto; padding: 0;
  font-family: 'JetBrains Mono', monospace; font-size: 11px;
  line-height: 1.6;
}}
.wgsl-header {{
  padding: 8px 14px; border-bottom: 1px solid #2A2A2A;
  font-size: 10px; color: #666; display: flex; align-items: center; gap: 12px;
}}
.wgsl-header button {{
  background: #1F1F1F; border: 1px solid #2A2A2A; color: #A0A0A0;
  padding: 2px 8px; border-radius: 3px; font-size: 10px; cursor: pointer;
  font-family: inherit;
}}
.wgsl-header button:hover {{ color: #FFF; border-color: #444; }}
.wgsl-code {{
  padding: 12px 14px; white-space: pre; overflow-x: auto;
  counter-reset: line;
}}
.wgsl-code .line {{ display: block; }}
.wgsl-code .line::before {{
  counter-increment: line; content: counter(line);
  display: inline-block; width: 3em; text-align: right; margin-right: 1em;
  color: #333; user-select: none;
}}
.wgsl-kw {{ color: #D4AF37; }}
.wgsl-type {{ color: #22C55E; }}
.wgsl-num {{ color: #EF4444; }}
.wgsl-comment {{ color: #666; font-style: italic; }}
.wgsl-deco {{ color: #F59E0B; }}

/* ── API panel ───────────────────────────────────── */
.api-panel {{
  height: 100%; overflow: auto; padding: 16px;
  font-family: 'JetBrains Mono', monospace; font-size: 11px;
}}
.api-section {{ margin-bottom: 20px; }}
.api-section h3 {{
  font-size: 10px; color: #666; text-transform: uppercase;
  letter-spacing: 1px; margin-bottom: 8px; font-weight: 600;
}}
.api-code {{
  background: #141414; border: 1px solid #2A2A2A; border-radius: 4px;
  padding: 10px 14px; font-size: 11px; color: #A0A0A0;
  position: relative; cursor: pointer;
}}
.api-code:hover {{ border-color: #444; }}
.api-code::after {{
  content: 'click to copy'; position: absolute; right: 8px; top: 8px;
  font-size: 9px; color: #444;
}}
.api-table {{
  width: 100%; border-collapse: collapse;
}}
.api-table th {{
  text-align: left; font-size: 10px; color: #444; padding: 4px 8px;
  border-bottom: 1px solid #2A2A2A; font-weight: 500;
}}
.api-table td {{
  padding: 4px 8px; color: #A0A0A0; border-bottom: 1px solid #1F1F1F;
}}
.cap-grid {{ display: flex; gap: 6px; flex-wrap: wrap; }}
.cap-badge {{
  padding: 3px 10px; border-radius: 3px; font-size: 10px;
  border: 1px solid #2A2A2A;
}}
.cap-on {{ color: #22C55E; border-color: #22C55E33; background: #22C55E0A; }}
.cap-off {{ color: #444; }}

/* ── Component panel ─────────────────────────────── */
.component-panel {{ display: flex; flex-direction: column; }}
.component-view {{
  flex: 1; display: flex; align-items: center; justify-content: center;
  padding: 24px; position: relative;
}}
.comp-wrapper {{ display: flex; align-items: center; justify-content: center; }}
.comp-wrapper.size-sm {{ width: 200px; height: 200px; }}
.comp-wrapper.size-md {{ width: 400px; height: 400px; }}
.comp-wrapper.size-lg {{ width: 100%; aspect-ratio: 1; max-height: 100%; }}
.comp-wrapper {tag_name} {{ width: 100%; height: 100%; }}

/* ── Size toggle ─────────────────────────────────── */
.size-bar {{
  display: flex; gap: 4px; padding: 8px 14px;
  border-top: 1px solid #2A2A2A;
}}
.size-bar button {{
  background: #1F1F1F; border: 1px solid #2A2A2A; color: #666;
  padding: 3px 10px; border-radius: 3px; font-size: 10px;
  cursor: pointer; font-family: inherit;
}}
.size-bar button:hover {{ color: #A0A0A0; border-color: #444; }}
.size-bar button.active {{ color: #D4AF37; border-color: #D4AF37; }}

/* ── Parameter monitor ───────────────────────────── */
.param-monitor {{
  padding: 10px 14px; border-top: 1px solid #2A2A2A; font-size: 11px;
  max-height: 200px; overflow-y: auto;
}}
.param-title {{
  font-size: 10px; color: #444; text-transform: uppercase;
  letter-spacing: 1px; margin-bottom: 6px;
}}
.param-row {{
  display: flex; align-items: center; gap: 8px; margin-bottom: 5px;
  height: 22px;
}}
.param-dot {{
  width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0;
}}
.param-dot.data {{ background: #D4AF37; }}
.param-dot.modulated {{ background: #22D3EE; }}
.param-dot.arc {{ background: #F59E0B; }}
.param-dot.static {{ background: #666; }}
.param-name {{ width: 80px; color: #666; overflow: hidden; text-overflow: ellipsis; }}
.param-slider {{ flex: 1; }}
.param-slider input[type="range"] {{
  width: 100%; -webkit-appearance: none; height: 3px;
  background: #333; border-radius: 2px; outline: none;
}}
.param-slider input[type="range"]::-webkit-slider-thumb {{
  -webkit-appearance: none; width: 10px; height: 10px;
  border-radius: 50%; background: #D4AF37; cursor: pointer;
}}
.param-expr {{ font-size: 9px; color: #22D3EE; overflow: hidden; text-overflow: ellipsis; max-width: 120px; }}
.param-badge {{ font-size: 9px; color: #F59E0B; background: #F59E0B15; padding: 1px 5px; border-radius: 2px; }}
.param-val {{ width: 40px; text-align: right; color: #888; font-size: 10px; }}

/* ── Timeline ────────────────────────────────────── */
.timeline {{
  padding: 8px 14px 10px; border-top: 1px solid #2A2A2A;
  user-select: none;
}}
.timeline-controls {{
  display: flex; align-items: center; gap: 10px; margin-bottom: 6px;
}}
.timeline-play {{
  background: #1F1F1F; border: 1px solid #2A2A2A; color: #A0A0A0;
  width: 24px; height: 24px; border-radius: 3px; cursor: pointer;
  font-size: 10px; display: flex; align-items: center; justify-content: center;
  font-family: inherit;
}}
.timeline-play:hover {{ color: #FFF; border-color: #444; }}
.timeline-rail {{
  flex: 1; height: 20px; position: relative; cursor: pointer;
}}
.timeline-track {{
  position: absolute; top: 9px; left: 0; right: 0; height: 2px;
  background: #2A2A2A; border-radius: 1px;
}}
.timeline-fill {{
  position: absolute; top: 9px; left: 0; height: 2px;
  background: #D4AF37; border-radius: 1px;
}}
.timeline-head {{
  position: absolute; top: 5px; width: 10px; height: 10px;
  background: #D4AF37; border-radius: 50%; margin-left: -5px;
  cursor: grab;
}}
.timeline-head:active {{ cursor: grabbing; }}
.timeline-moment {{
  position: absolute; top: 3px; width: 14px; height: 14px;
  border-radius: 50%; background: #1F1F1F; border: 1.5px solid #666;
  margin-left: -7px; cursor: pointer; z-index: 2;
}}
.timeline-moment:hover {{ border-color: #D4AF37; }}
.timeline-moment-label {{
  position: absolute; top: 18px; left: 50%; transform: translateX(-50%);
  font-size: 9px; color: #444; white-space: nowrap;
}}
.timeline-time {{ font-size: 10px; color: #666; min-width: 60px; text-align: right; }}
.timeline-moments {{
  display: flex; gap: 4px; flex-wrap: wrap; margin-top: 2px;
}}
.timeline-moment-btn {{
  background: #1F1F1F; border: 1px solid #2A2A2A; color: #666;
  padding: 2px 8px; border-radius: 3px; font-size: 9px; cursor: pointer;
  font-family: inherit;
}}
.timeline-moment-btn:hover {{ color: #D4AF37; border-color: #D4AF37; }}

/* ── Shortcut overlay ────────────────────────────── */
.shortcut-overlay {{
  display: none; position: fixed; inset: 0; background: rgba(0,0,0,0.8);
  z-index: 1000; align-items: center; justify-content: center;
}}
.shortcut-overlay.open {{ display: flex; }}
.shortcut-box {{
  background: #141414; border: 1px solid #2A2A2A; border-radius: 8px;
  padding: 24px; min-width: 280px;
}}
.shortcut-box h3 {{ color: #D4AF37; font-size: 12px; margin-bottom: 12px; }}
.shortcut-row {{ display: flex; justify-content: space-between; padding: 4px 0; font-size: 11px; }}
.shortcut-key {{ color: #D4AF37; font-size: 10px; background: #1F1F1F; padding: 2px 6px; border-radius: 3px; }}
.shortcut-desc {{ color: #666; }}

/* ── X-Ray bar ───────────────────────────────────── */
.xray-bar {{
  display: none; padding: 4px 14px; border-bottom: 1px solid #2A2A2A;
  gap: 4px; flex-wrap: wrap; align-items: center;
  background: #0A0A0A; user-select: none;
}}
.xray-bar.visible {{ display: flex; }}
.xray-label {{
  font-size: 9px; color: #444; text-transform: uppercase;
  letter-spacing: 1px; margin-right: 4px;
}}
.xray-sep {{
  width: 1px; height: 14px; background: #2A2A2A; margin: 0 4px;
}}
.xray-chip {{
  background: #1F1F1F; border: 1px solid #2A2A2A; color: #666;
  padding: 2px 10px; border-radius: 3px; font-size: 10px;
  cursor: pointer; font-family: inherit; transition: all 0.1s;
}}
.xray-chip:hover {{ color: #A0A0A0; border-color: #444; }}
.xray-chip.active {{ color: #D4AF37; border-color: #D4AF37; background: #D4AF3710; }}
.xray-chip.all {{ color: #22C55E; }}

/* ── Editor ─────────────────────────────────────── */
.editor-panel {{ height: 100%; display: flex; flex-direction: column; }}
.editor-header {{
  padding: 8px 14px; border-bottom: 1px solid #2A2A2A;
  font-size: 10px; color: #666; display: flex; align-items: center; gap: 12px;
}}
.editor-status {{ margin-left: auto; font-size: 10px; }}
.editor-status.ok {{ color: #22C55E; }}
.editor-status.err {{ color: #EF4444; }}
.editor-status.saving {{ color: #F59E0B; }}
.editor-save {{
  background: #1F1F1F; border: 1px solid #2A2A2A; color: #A0A0A0;
  padding: 2px 8px; border-radius: 3px; font-size: 10px; cursor: pointer;
  font-family: inherit;
}}
.editor-save:hover {{ color: #FFF; border-color: #444; }}
.editor-textarea {{
  flex: 1; width: 100%; resize: none; padding: 12px 14px;
  background: #0A0A0A; color: #A0A0A0; border: none; outline: none;
  font-family: 'JetBrains Mono', monospace; font-size: 12px;
  line-height: 1.6; tab-size: 2;
}}

/* ── Pixel Autopsy ──────────────────────────────── */
.autopsy-mode #comp-wrapper {{ cursor: crosshair; }}
.autopsy-mode #comp-wrapper * {{ cursor: crosshair; }}
.autopsy-tooltip {{
  display: none; position: fixed; z-index: 500;
  background: #141414; border: 1px solid #2A2A2A; border-radius: 6px;
  padding: 10px 14px; min-width: 180px;
  font-family: 'JetBrains Mono', monospace; font-size: 11px;
  pointer-events: none; box-shadow: 0 4px 12px rgba(0,0,0,0.5);
}}
.autopsy-tooltip.visible {{ display: block; }}
.autopsy-swatch {{
  width: 100%; height: 24px; border-radius: 3px; margin-bottom: 8px;
  border: 1px solid #2A2A2A;
}}
.autopsy-row {{
  display: flex; justify-content: space-between; padding: 2px 0;
  color: #A0A0A0;
}}
.autopsy-label {{ color: #666; width: 40px; }}

/* ── Export dropdown ─────────────────────────────── */
.export-dropdown {{ position: relative; display: inline-flex; }}
.export-trigger {{
  background: #1F1F1F; border: 1px solid #2A2A2A; color: #A0A0A0;
  padding: 3px 10px; border-radius: 3px; font-size: 10px; cursor: pointer;
  font-family: inherit;
}}
.export-trigger:hover {{ color: #FFF; border-color: #444; }}
.export-menu {{
  display: none; position: absolute; top: 100%; right: 0; margin-top: 4px;
  background: #141414; border: 1px solid #2A2A2A; border-radius: 4px;
  min-width: 160px; z-index: 100; padding: 4px 0;
  box-shadow: 0 4px 12px rgba(0,0,0,0.5);
}}
.export-dropdown.open .export-menu {{ display: block; }}
.export-menu button, .export-menu a {{
  display: block; width: 100%; text-align: left; background: none;
  border: none; color: #A0A0A0; padding: 6px 14px; font-size: 10px;
  cursor: pointer; font-family: inherit; text-decoration: none;
}}
.export-menu button:hover, .export-menu a:hover {{
  background: #1F1F1F; color: #FFF;
}}"#,
        tag_name = tag_name,
    )
}
