//! Web Component output format.
//!
//! Generates a self-contained `.js` file that defines a custom element
//! (`<game-xyz>`) with WebGPU primary and WebGL2 fallback.

use crate::codegen::ShaderOutput;

/// Generate a zero-dependency Web Component JS file.
pub fn generate_component(shader: &ShaderOutput) -> String {
    let tag = to_kebab(&shader.name);
    let class = to_pascal(&shader.name);

    let uniform_defs_json = shader
        .uniforms
        .iter()
        .map(|u| format!("{{name:'{}',default:{}}}", u.name, u.default))
        .collect::<Vec<_>>()
        .join(",");

    let wgsl_v = escape_js(&shader.wgsl_vertex);
    let wgsl_f = escape_js(&shader.wgsl_fragment);
    let glsl_v = escape_js(&shader.glsl_vertex);
    let glsl_f = escape_js(&shader.glsl_fragment);

    let needs_prev_frame = shader.uses_memory || shader.uses_feedback;
    let pass_count = shader.pass_count;
    let has_passes = pass_count > 0;

    // Build incrementally to avoid stack overflow from giant format! macro
    let mut s = String::with_capacity(16384);

    s.push_str(&format!(
        "// GAME Component: {tag} — auto-generated, do not edit.\n"
    ));
    s.push_str("(function(){\n");
    s.push_str(&format!("const WGSL_V = `{wgsl_v}`;\n"));
    s.push_str(&format!("const WGSL_F = `{wgsl_f}`;\n"));
    s.push_str(&format!("const GLSL_V = `{glsl_v}`;\n"));
    s.push_str(&format!("const GLSL_F = `{glsl_f}`;\n"));
    s.push_str(&format!("const UNIFORMS = [{uniform_defs_json}];\n"));

    // Pass shader constants
    if has_passes {
        for (i, pass_wgsl) in shader.pass_wgsl.iter().enumerate() {
            let escaped = escape_js(pass_wgsl);
            s.push_str(&format!("const PASS_WGSL_{i} = `{escaped}`;\n"));
        }
        let pass_refs: Vec<String> = (0..pass_count).map(|i| format!("PASS_WGSL_{i}")).collect();
        s.push_str(&format!(
            "const PASS_SHADERS = [{}];\n",
            pass_refs.join(",")
        ));
    }

    // Compute shader constants
    if let Some(ref wgsl) = shader.compute_wgsl {
        s.push_str(&format!("const COMPUTE_WGSL = `{}`;\n", escape_js(wgsl)));
    }
    if let Some(ref wgsl) = shader.react_wgsl {
        s.push_str(&format!("const REACT_WGSL = `{}`;\n", escape_js(wgsl)));
    }
    if let Some(ref wgsl) = shader.swarm_agent_wgsl {
        s.push_str(&format!(
            "const SWARM_AGENT_WGSL = `{}`;\n",
            escape_js(wgsl)
        ));
    }
    if let Some(ref wgsl) = shader.swarm_trail_wgsl {
        s.push_str(&format!(
            "const SWARM_TRAIL_WGSL = `{}`;\n",
            escape_js(wgsl)
        ));
    }
    if let Some(ref wgsl) = shader.flow_wgsl {
        s.push_str(&format!("const FLOW_WGSL = `{}`;\n", escape_js(wgsl)));
    }
    s.push('\n');

    // Determine compute type for fragment shader wiring
    let compute_type = if shader.react_wgsl.is_some() {
        Some(super::helpers::ComputeType::React)
    } else if shader.swarm_agent_wgsl.is_some() {
        Some(super::helpers::ComputeType::Swarm)
    } else if shader.flow_wgsl.is_some() {
        Some(super::helpers::ComputeType::Flow)
    } else {
        None
    };

    // WebGPU renderer (with features)
    s.push_str(&super::helpers::webgpu_renderer(
        needs_prev_frame,
        pass_count,
        compute_type,
    ));
    s.push_str("\n\n");

    // WebGL2 fallback renderer (with memory, no passes)
    s.push_str(&super::helpers::webgl2_renderer(needs_prev_frame));
    s.push_str("\n\n");

    // Inject feature JS modules (listen, voice, score, temporal, gravity, breed)
    for module_js in &shader.js_modules {
        s.push_str(module_js);
        s.push_str("\n\n");
    }

    let has_dom = shader.dom_html.is_some();
    let has_string_props = !shader.string_props.is_empty();
    let has_events = !shader.event_handlers.is_empty();

    // Custom element class
    s.push_str(&format!("class {class} extends HTMLElement {{\n"));
    s.push_str("  constructor() {\n");
    s.push_str("    super();\n");
    s.push_str("    this.attachShadow({ mode: 'open' });\n");
    s.push_str("    this._renderer = null;\n");
    s.push_str("    this._resizeObserver = null;\n");
    s.push_str("    this._pendingParams = {};\n");
    if has_string_props {
        s.push_str("    this._stringProps = {\n");
        for sp in &shader.string_props {
            s.push_str(&format!(
                "      '{}': '{}',\n",
                sp.name,
                escape_js(&sp.default)
            ));
        }
        s.push_str("    };\n");
    }
    s.push_str("  }\n\n");

    // Build CSS: base + optional DOM overlay styles
    let mut css = String::from(":host{display:block;width:100%;height:100%;position:relative}canvas{width:100%;height:100%;display:block}");
    if has_dom {
        css.push_str(".game-overlay{position:absolute;top:0;left:0;width:100%;height:100%;pointer-events:none;overflow:hidden}.game-overlay>*{pointer-events:auto}");
        if let Some(ref dom_css) = shader.dom_css {
            css.push_str(dom_css);
        }
    }

    s.push_str("  connectedCallback() {\n");
    s.push_str("    const style = document.createElement('style');\n");
    s.push_str(&format!(
        "    style.textContent = '{}';\n",
        escape_js(&css)
    ));
    s.push_str("    const canvas = document.createElement('canvas');\n");
    s.push_str("    this.shadowRoot.appendChild(style);\n");
    s.push_str("    this.shadowRoot.appendChild(canvas);\n");
    s.push_str("    this._canvas = canvas;\n");

    // DOM overlay
    if has_dom {
        s.push_str("    const overlay = document.createElement('div');\n");
        s.push_str("    overlay.className = 'game-overlay';\n");
        if let Some(ref aria_role) = shader.aria_role {
            s.push_str(&format!(
                "    overlay.setAttribute('role', '{}');\n",
                escape_js(aria_role)
            ));
        }
        if let Some(ref dom_html) = shader.dom_html {
            s.push_str(&format!(
                "    overlay.innerHTML = '{}';\n",
                escape_js(dom_html)
            ));
        }
        s.push_str("    this.shadowRoot.appendChild(overlay);\n");
        s.push_str("    this._overlay = overlay;\n");
        s.push_str("    this._updateDOM();\n");
    }

    // Event handlers
    if has_events {
        for (event, emit) in &shader.event_handlers {
            if let Some(emit_name) = emit {
                s.push_str(&format!(
                    "    this.addEventListener('{}', () => this.dispatchEvent(new CustomEvent('{}', {{bubbles:true}})));\n",
                    escape_js(event),
                    escape_js(emit_name)
                ));
            }
        }
    }

    // Arc lifecycle: instantiate state timelines
    if shader.has_arc_enter {
        s.push_str("    this._arcEnter = new GameArcEnter();\n");
    }
    if shader.has_arc_exit {
        s.push_str("    this._arcExit = new GameArcExit();\n");
    }
    if shader.has_arc_hover {
        s.push_str("    this._arcHover = new GameArcHover();\n");
        s.push_str("    this.addEventListener('mouseenter', () => {\n");
        s.push_str("      if (this._arcHover && this._renderer) {\n");
        s.push_str("        this._arcHover.enter(this._renderer._elapsed || 0);\n");
        s.push_str("      }\n");
        s.push_str("    });\n");
        s.push_str("    this.addEventListener('mouseleave', () => {\n");
        s.push_str("      if (this._arcHover && this._renderer) {\n");
        s.push_str("        this._arcHover.leave(this._renderer._elapsed || 0);\n");
        s.push_str("      }\n");
        s.push_str("    });\n");
    }

    s.push_str("    this._initRenderer();\n");
    s.push_str("    this._resizeObserver = new ResizeObserver(() => this._resize());\n");
    s.push_str("    this._resizeObserver.observe(this);\n");

    // Auto-play enter arc after renderer init
    if shader.has_arc_enter {
        s.push_str("    if (this._arcEnter) this._arcEnter.play(0);\n");
    }

    s.push_str("  }\n\n");

    s.push_str("  disconnectedCallback() {\n");
    s.push_str("    this._renderer?.destroy();\n");
    s.push_str("    this._renderer = null;\n");
    s.push_str("    this._resizeObserver?.disconnect();\n");
    s.push_str("  }\n\n");

    s.push_str("  async _initRenderer() {\n");
    s.push_str("    const gpu = new GameRenderer(this._canvas, WGSL_V, WGSL_F, UNIFORMS");
    if has_passes {
        s.push_str(", PASS_SHADERS");
    }
    if let Some(ct) = compute_type {
        let ct_str = match ct {
            super::helpers::ComputeType::React => "react",
            super::helpers::ComputeType::Swarm => "swarm",
            super::helpers::ComputeType::Flow => "flow",
        };
        s.push_str(&format!(", '{ct_str}'"));
    }
    s.push_str(");\n");
    s.push_str("    if (await gpu.init()) {\n");
    s.push_str("      this._renderer = gpu;\n");
    s.push_str("    } else {\n");
    s.push_str("      const gl = new GameRendererGL(this._canvas, GLSL_V, GLSL_F, UNIFORMS);\n");
    s.push_str("      if (gl.init()) {\n");
    s.push_str("        this._renderer = gl;\n");
    s.push_str("      } else {\n");
    s.push_str(&format!(
        "        console.warn('game-{tag}: no WebGPU or WebGL2 support');\n"
    ));
    s.push_str("        return;\n");
    s.push_str("      }\n");
    s.push_str("    }\n");
    s.push_str("    this._resize();\n");

    // Initialize compute simulations (WebGPU only)
    let has_compute = shader.compute_wgsl.is_some()
        || shader.react_wgsl.is_some()
        || shader.swarm_agent_wgsl.is_some()
        || shader.flow_wgsl.is_some();
    if has_compute {
        s.push_str("    if (this._renderer.device) {\n");
        s.push_str("      const dev = this._renderer.device;\n");
        if shader.compute_wgsl.is_some() {
            s.push_str("      if (typeof COMPUTE_WGSL !== 'undefined') {\n");
            s.push_str("        const sim = new GameGravitySim(dev, COMPUTE_WGSL);\n");
            s.push_str("        await sim.init();\n");
            s.push_str("        this._gravitySim = sim;\n");
            s.push_str("      }\n");
        }
        if shader.react_wgsl.is_some() {
            s.push_str("      if (typeof REACT_WGSL !== 'undefined') {\n");
            s.push_str("        const sim = new GameReactionField(dev, REACT_WGSL);\n");
            s.push_str("        await sim.init();\n");
            s.push_str("        this._reactSim = sim;\n");
            if compute_type == Some(super::helpers::ComputeType::React) {
                s.push_str("        this._renderer.setComputeBuffer(sim.fieldBuffer, sim.width, sim.height);\n");
            }
            s.push_str("      }\n");
        }
        if shader.swarm_agent_wgsl.is_some() {
            s.push_str("      if (typeof SWARM_AGENT_WGSL !== 'undefined') {\n");
            s.push_str(
                "        const sim = new GameSwarmSim(dev, SWARM_AGENT_WGSL, SWARM_TRAIL_WGSL);\n",
            );
            s.push_str("        await sim.init();\n");
            s.push_str("        this._swarmSim = sim;\n");
            if compute_type == Some(super::helpers::ComputeType::Swarm) {
                s.push_str(
                    "        this._renderer.setComputeBuffer(sim.trailBuffer, sim._w, sim._h);\n",
                );
            }
            s.push_str("      }\n");
        }
        if shader.flow_wgsl.is_some() {
            s.push_str("      if (typeof FLOW_WGSL !== 'undefined') {\n");
            s.push_str("        const sim = new GameFlowField(dev, FLOW_WGSL);\n");
            s.push_str("        await sim.init();\n");
            s.push_str("        this._flowSim = sim;\n");
            if compute_type == Some(super::helpers::ComputeType::Flow) {
                s.push_str("        this._renderer.setComputeBuffer(sim.fieldBuffer, sim.width, sim.height);\n");
            }
            s.push_str("      }\n");
        }
        // Wire pre-render dispatch + buffer updates
        s.push_str("      this._renderer._preRender = () => {\n");
        s.push_str("        const dt = 1/60;\n");
        if shader.compute_wgsl.is_some() {
            s.push_str("        if (this._gravitySim) this._gravitySim.dispatch(dt);\n");
        }
        if shader.react_wgsl.is_some() {
            s.push_str("        if (this._reactSim) {\n");
            s.push_str("          this._reactSim.dispatch(4);\n");
            if compute_type == Some(super::helpers::ComputeType::React) {
                s.push_str("          this._renderer.setComputeBuffer(this._reactSim.fieldBuffer, this._reactSim.width, this._reactSim.height);\n");
            }
            s.push_str("        }\n");
        }
        if shader.swarm_agent_wgsl.is_some() {
            s.push_str("        if (this._swarmSim) {\n");
            s.push_str("          this._swarmSim.dispatch(dt);\n");
            if compute_type == Some(super::helpers::ComputeType::Swarm) {
                s.push_str("          this._renderer.setComputeBuffer(this._swarmSim.trailBuffer, this._swarmSim._w, this._swarmSim._h);\n");
            }
            s.push_str("        }\n");
        }
        if shader.flow_wgsl.is_some() {
            s.push_str("        if (this._flowSim) {\n");
            s.push_str("          this._flowSim.dispatch(dt);\n");
            if compute_type == Some(super::helpers::ComputeType::Flow) {
                s.push_str("          this._renderer.setComputeBuffer(this._flowSim.fieldBuffer, this._flowSim.width, this._flowSim.height);\n");
            }
            s.push_str("        }\n");
        }
        s.push_str("      };\n");
        s.push_str("    }\n");
    }

    // Initialize coupling matrix (CPU-side, no GPU required)
    if shader.has_coupling_matrix {
        s.push_str("    this._couplingMatrix = new GameCouplingMatrix();\n");
        s.push_str("    const existingPreRender = this._renderer._preRender;\n");
        s.push_str("    const comp = this;\n");
        s.push_str("    this._renderer._preRender = () => {\n");
        s.push_str("      if (existingPreRender) existingPreRender();\n");
        s.push_str("      if (comp._couplingMatrix && comp._renderer) {\n");
        s.push_str("        const params = comp._renderer.userParams || {};\n");
        s.push_str("        const result = comp._couplingMatrix.propagate(params);\n");
        s.push_str("        for (const [k, v] of Object.entries(result)) {\n");
        s.push_str("          if (k in params && params[k] !== v) comp.setParam(k, v);\n");
        s.push_str("        }\n");
        s.push_str("      }\n");
        s.push_str("    };\n");
    }

    // Apply any params set before renderer was ready
    s.push_str("    for (const [k, v] of Object.entries(this._pendingParams)) {\n");
    s.push_str("      this._renderer.setParam(k, v);\n");
    s.push_str("    }\n");
    s.push_str("    this._renderer.start();\n");
    s.push_str("  }\n\n");

    s.push_str("  _resize() {\n");
    s.push_str("    const rect = this.getBoundingClientRect();\n");
    s.push_str("    const dpr = window.devicePixelRatio || 1;\n");
    s.push_str("    this._canvas.width = Math.round(rect.width * dpr);\n");
    s.push_str("    this._canvas.height = Math.round(rect.height * dpr);\n");
    s.push_str("    if (this._renderer?._resizeMemory) this._renderer._resizeMemory();\n");
    if has_passes {
        s.push_str("    if (this._renderer?._resizePassFBOs) this._renderer._resizePassFBOs();\n");
    }
    s.push_str("  }\n\n");

    s.push_str("  setParam(name, value) {\n");
    s.push_str("    this._pendingParams[name] = value;\n");
    s.push_str("    this._renderer?.setParam(name, value);\n");
    s.push_str("  }\n");
    s.push_str("  setAudioData(data) { this._renderer?.setAudioData(data); }\n");
    s.push_str(
        "  setAudioSource(bridge) { bridge?.subscribe(d => this._renderer?.setAudioData(d)); }\n\n",
    );

    // playArc(name) — programmatic arc lifecycle trigger
    if shader.has_arc_enter || shader.has_arc_exit || shader.has_arc_hover {
        s.push_str("  playArc(name) {\n");
        s.push_str("    const t = this._renderer?._elapsed || 0;\n");
        if shader.has_arc_enter {
            s.push_str("    if (name === 'enter' && this._arcEnter) this._arcEnter.play(t);\n");
        }
        if shader.has_arc_exit {
            s.push_str("    if (name === 'exit' && this._arcExit) this._arcExit.play(t);\n");
        }
        if shader.has_arc_hover {
            s.push_str("    if (name === 'hover') { if (this._arcHover) this._arcHover.enter(t); }\n");
        }
        s.push_str("  }\n\n");
    }

    // DOM update method — syncs string props to bound DOM elements
    if has_dom && has_string_props {
        s.push_str("  _updateDOM() {\n");
        s.push_str("    if (!this._overlay) return;\n");
        s.push_str("    const els = this._overlay.querySelectorAll('[data-bind]');\n");
        s.push_str("    for (const el of els) {\n");
        s.push_str("      const prop = el.dataset.bind;\n");
        s.push_str("      if (prop in this._stringProps) el.textContent = this._stringProps[prop];\n");
        s.push_str("    }\n");
        s.push_str("  }\n\n");

        // String prop setters
        s.push_str("  setStringProp(name, value) {\n");
        s.push_str("    if (this._stringProps && name in this._stringProps) {\n");
        s.push_str("      this._stringProps[name] = value;\n");
        s.push_str("      this._updateDOM();\n");
        s.push_str("    }\n");
        s.push_str("  }\n\n");

        // Individual string prop getters/setters
        for sp in &shader.string_props {
            let name = &sp.name;
            let default = escape_js(&sp.default);
            s.push_str(&format!(
                "  get {name}() {{ return this._stringProps?.['{name}'] ?? '{default}'; }}\n"
            ));
            s.push_str(&format!(
                "  set {name}(v) {{ this._stringProps['{name}'] = v; this._updateDOM(); }}\n"
            ));
        }
        s.push('\n');
    }

    // Frame export API
    s.push_str("  getFrame() {\n");
    s.push_str("    if (!this._canvas) return null;\n");
    s.push_str("    const w = this._canvas.width, h = this._canvas.height;\n");
    s.push_str("    const offscreen = document.createElement('canvas');\n");
    s.push_str("    offscreen.width = w;\n");
    s.push_str("    offscreen.height = h;\n");
    s.push_str("    const ctx = offscreen.getContext('2d');\n");
    s.push_str("    ctx.drawImage(this._canvas, 0, 0);\n");
    s.push_str("    return ctx.getImageData(0, 0, w, h);\n");
    s.push_str("  }\n\n");

    s.push_str("  getFrameDataURL(type) {\n");
    s.push_str("    if (!this._canvas) return null;\n");
    s.push_str("    return this._canvas.toDataURL(type || 'image/png');\n");
    s.push_str("  }\n\n");

    // Generate property getters/setters for each uniform so el.fill_angle = 0.5 works
    s.push_str("  // Property accessors for each uniform\n");
    for u in &shader.uniforms {
        let name = &u.name;
        let default = u.default;
        s.push_str(&format!(
            "  get {name}() {{ return this._renderer?.userParams['{name}'] ?? this._pendingParams['{name}'] ?? {default}; }}\n"
        ));
        s.push_str(&format!(
            "  set {name}(v) {{ this.setParam('{name}', v); }}\n"
        ));
    }
    // Convenience alias: 'progress' maps to fill_angle (scaled to radians)
    // Only emit if fill_angle exists AND 'progress' isn't already a real uniform
    let has_fill_angle = shader.uniforms.iter().any(|u| u.name == "fill_angle");
    let has_progress = shader.uniforms.iter().any(|u| u.name == "progress");
    if has_fill_angle && !has_progress {
        s.push_str("  get progress() { return this.fill_angle / (2 * Math.PI); }\n");
        s.push_str("  set progress(v) { this.fill_angle = v * 2 * Math.PI; }\n");
    }
    // Convenience alias: 'health' maps to intensity
    // Only emit if intensity exists AND 'health' isn't already a real uniform
    let has_intensity = shader.uniforms.iter().any(|u| u.name == "intensity");
    let has_health = shader.uniforms.iter().any(|u| u.name == "health");
    if has_intensity && !has_health {
        s.push_str("  get health() { return this.intensity; }\n");
        s.push_str("  set health(v) { this.intensity = v; }\n");
    }
    s.push_str("\n");

    // Observed attributes: uniforms (numeric) + string props
    if has_string_props {
        let sp_names: Vec<String> = shader
            .string_props
            .iter()
            .map(|sp| format!("'{}'", sp.name))
            .collect();
        s.push_str(&format!(
            "  static get observedAttributes() {{ return [...UNIFORMS.map(u => u.name), {}]; }}\n",
            sp_names.join(",")
        ));
        s.push_str("  attributeChangedCallback(name, _, val) {\n");
        s.push_str("    if (val === null) return;\n");
        s.push_str("    if (this._stringProps && name in this._stringProps) {\n");
        s.push_str("      this._stringProps[name] = val;\n");
        s.push_str("      this._updateDOM();\n");
        s.push_str("    } else {\n");
        s.push_str("      this.setParam(name, parseFloat(val));\n");
        s.push_str("    }\n");
        s.push_str("  }\n");
    } else {
        s.push_str("  static get observedAttributes() { return UNIFORMS.map(u => u.name); }\n");
        s.push_str("  attributeChangedCallback(name, _, val) {\n");
        s.push_str("    if (val !== null) this.setParam(name, parseFloat(val));\n");
        s.push_str("  }\n");
    }
    s.push_str("}\n\n");

    s.push_str(&format!("customElements.define('game-{tag}', {class});\n"));
    s.push_str("})();\n");

    s
}

pub(crate) fn to_kebab(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub(crate) fn to_pascal(s: &str) -> String {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(c) => {
                    let mut s = c.to_uppercase().to_string();
                    s.extend(chars);
                    s
                }
                None => String::new(),
            }
        })
        .collect()
}

fn escape_js(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::{ShaderOutput, UniformInfo};

    fn make_shader(name: &str) -> ShaderOutput {
        ShaderOutput {
            name: name.into(),
            wgsl_fragment: "fn fs_main() {}".into(),
            wgsl_vertex: "fn vs_main() {}".into(),
            glsl_fragment: "void main(){}".into(),
            glsl_vertex: "void main(){}".into(),
            uniforms: vec![],
            uses_memory: false,
            js_modules: vec![],
            compute_wgsl: None,
            react_wgsl: None,
            swarm_agent_wgsl: None,
            swarm_trail_wgsl: None,
            flow_wgsl: None,
            pass_wgsl: vec![],
            pass_count: 0,
            uses_feedback: false,
            has_coupling_matrix: false,
            string_props: vec![],
            dom_html: None,
            dom_css: None,
            event_handlers: vec![],
            aria_role: None,
            has_arc_enter: false,
            has_arc_exit: false,
            has_arc_hover: false,
        }
    }

    #[test]
    fn component_has_custom_element_define() {
        let shader = make_shader("test-viz");
        let js = generate_component(&shader);
        assert!(js.contains("customElements.define('game-test-viz'"));
        assert!(js.contains("class TestViz extends HTMLElement"));
    }

    #[test]
    fn component_includes_both_renderers() {
        let mut shader = make_shader("demo");
        shader.uniforms = vec![UniformInfo {
            name: "speed".into(),
            default: 1.0,
        }];
        let js = generate_component(&shader);
        assert!(js.contains("class GameRenderer"));
        assert!(js.contains("class GameRendererGL"));
        assert!(js.contains("{name:'speed',default:1}"));
    }

    #[test]
    fn component_with_memory_includes_methods_inside_class() {
        let mut shader = make_shader("trails");
        shader.uses_memory = true;
        let js = generate_component(&shader);
        // Memory methods should be inside GameRenderer class
        assert!(js.contains("_initMemory()"));
        assert!(js.contains("_swapMemory(encoder"));
        assert!(js.contains("_resizeMemory()"));
        // Memory init should be called in init()
        assert!(js.contains("this._initMemory()"));
        // Memory bind group should be set in render
        assert!(js.contains("setBindGroup(1, this._memBindGroup)"));
        // Pipeline layout should include memory BGL
        assert!(js.contains("this._memBindGroupLayout"));
    }

    #[test]
    fn component_with_feedback_enables_memory() {
        let mut shader = make_shader("feedback-viz");
        shader.uses_feedback = true;
        let js = generate_component(&shader);
        assert!(js.contains("_initMemory()"));
        assert!(js.contains("setBindGroup(1, this._memBindGroup)"));
    }

    #[test]
    fn component_with_passes_has_fbo_chain() {
        let mut shader = make_shader("bloom");
        shader.pass_wgsl = vec!["// pass 0 shader".into(), "// pass 1 shader".into()];
        shader.pass_count = 2;
        let js = generate_component(&shader);
        // Pass shader constants
        assert!(js.contains("PASS_WGSL_0"));
        assert!(js.contains("PASS_WGSL_1"));
        assert!(js.contains("PASS_SHADERS"));
        // Pass pipelines
        assert!(js.contains("_passPipelines"));
        assert!(js.contains("_initPassFBOs()"));
        // Main render to FBO (not canvas) when passes exist
        assert!(js.contains("this._passFBOs[0].createView()"));
        // Pass rendering loop
        assert!(js.contains("for (let p = 0; p < 2; p++)"));
        // FBO resize
        assert!(js.contains("_resizePassFBOs"));
    }

    #[test]
    fn component_with_memory_and_passes() {
        let mut shader = make_shader("full");
        shader.uses_memory = true;
        shader.pass_wgsl = vec!["// blur pass".into()];
        shader.pass_count = 1;
        let js = generate_component(&shader);
        // Memory captures from FBO (not canvas) when passes exist
        assert!(js.contains("this._swapMemory(encoder, this._passFBOs[0])"));
        // Both memory and pass features present
        assert!(js.contains("_initMemory()"));
        assert!(js.contains("_initPassFBOs()"));
    }

    #[test]
    fn component_with_compute_has_dispatch() {
        let mut shader = make_shader("particles");
        shader.compute_wgsl = Some("// gravity compute shader".into());
        shader.js_modules = vec!["class GameGravitySim { dispatch(dt){} }".into()];
        let js = generate_component(&shader);
        // Compute WGSL constant
        assert!(js.contains("COMPUTE_WGSL"));
        // Compute init
        assert!(js.contains("new GameGravitySim(dev, COMPUTE_WGSL)"));
        assert!(js.contains("await sim.init()"));
        // Pre-render dispatch
        assert!(js.contains("_gravitySim"));
        assert!(js.contains("_preRender"));
    }

    #[test]
    fn component_with_react_has_dispatch() {
        let mut shader = make_shader("turing");
        shader.react_wgsl = Some("// react compute shader".into());
        shader.js_modules = vec!["class GameReactionField { dispatch(n){} }".into()];
        let js = generate_component(&shader);
        assert!(js.contains("REACT_WGSL"));
        assert!(js.contains("new GameReactionField(dev, REACT_WGSL)"));
        assert!(js.contains("_reactSim"));
    }

    #[test]
    fn component_without_features_has_simple_render() {
        let shader = make_shader("simple");
        let js = generate_component(&shader);
        // Direct render to canvas
        assert!(js.contains("this.ctx.getCurrentTexture().createView()"));
        // No memory or pass references
        assert!(!js.contains("_memBindGroup"));
        assert!(!js.contains("_passFBOs"));
        assert!(!js.contains("PASS_SHADERS"));
    }

    #[test]
    fn component_with_listen_includes_pipeline() {
        let mut shader = make_shader("audio-viz");
        shader.js_modules = vec!["class GameListenPipeline { /* listen */ }".into()];
        let js = generate_component(&shader);
        assert!(js.contains("GameListenPipeline"));
    }

    #[test]
    fn kebab_and_pascal() {
        assert_eq!(to_kebab("celebration-burst"), "celebration-burst");
        assert_eq!(to_kebab("My Cool Viz"), "my-cool-viz");
        assert_eq!(to_pascal("celebration-burst"), "CelebrationBurst");
        assert_eq!(to_pascal("test"), "Test");
    }

    #[test]
    fn progress_alias_only_when_fill_angle_exists() {
        let mut with_fill = make_shader("ring");
        with_fill.uniforms = vec![UniformInfo {
            name: "fill_angle".into(),
            default: 0.0,
        }];
        let js = generate_component(&with_fill);
        assert!(js.contains("set progress(v)"), "should have progress alias");
        assert!(
            js.contains("set fill_angle(v)"),
            "should have fill_angle setter"
        );

        let mut without_fill = make_shader("orb");
        without_fill.uniforms = vec![UniformInfo {
            name: "glow".into(),
            default: 1.0,
        }];
        let js = generate_component(&without_fill);
        assert!(
            !js.contains("set progress(v)"),
            "should NOT have progress alias"
        );
    }

    #[test]
    fn health_alias_only_when_intensity_exists() {
        let mut with_intensity = make_shader("orb");
        with_intensity.uniforms = vec![UniformInfo {
            name: "intensity".into(),
            default: 1.0,
        }];
        let js = generate_component(&with_intensity);
        assert!(js.contains("set health(v)"), "should have health alias");

        let mut without_intensity = make_shader("bars");
        without_intensity.uniforms = vec![UniformInfo {
            name: "glow_val".into(),
            default: 1.0,
        }];
        let js = generate_component(&without_intensity);
        assert!(
            !js.contains("set health(v)"),
            "should NOT have health alias"
        );
    }

    #[test]
    fn no_duplicate_progress_when_uniform_named_progress() {
        let mut shader = make_shader("countdown");
        shader.uniforms = vec![
            UniformInfo {
                name: "progress".into(),
                default: 0.0,
            },
            UniformInfo {
                name: "urgency".into(),
                default: 0.0,
            },
        ];
        let js = generate_component(&shader);
        let count = js.matches("set progress(v)").count();
        assert_eq!(
            count, 1,
            "expected exactly one progress setter, got {count}"
        );
        assert!(js.contains("set progress(v) { this.setParam('progress', v); }"));
        assert!(!js.contains("this.fill_angle"));
    }
}
