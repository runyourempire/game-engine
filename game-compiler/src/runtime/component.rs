//! Web Component output format.
//!
//! Generates a self-contained `.js` file that defines a custom element
//! (`<game-xyz>`) with WebGPU primary and WebGL2 fallback.

use crate::codegen::ShaderOutput;
use crate::codegen::memory;

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

    let uses_memory = shader.uses_memory;

    // Build incrementally to avoid stack overflow from giant format! macro
    let mut s = String::with_capacity(16384);

    s.push_str(&format!("// GAME Component: {tag} — auto-generated, do not edit.\n"));
    s.push_str("(function(){\n");
    s.push_str(&format!("const WGSL_V = `{wgsl_v}`;\n"));
    s.push_str(&format!("const WGSL_F = `{wgsl_f}`;\n"));
    s.push_str(&format!("const GLSL_V = `{glsl_v}`;\n"));
    s.push_str(&format!("const GLSL_F = `{glsl_f}`;\n"));
    s.push_str(&format!("const UNIFORMS = [{uniform_defs_json}];\n"));
    s.push_str(&format!("const USES_MEMORY = {};\n\n", if uses_memory { "true" } else { "false" }));

    s.push_str(super::helpers::webgpu_renderer());
    s.push_str("\n\n");

    if uses_memory {
        s.push_str(memory::webgpu_memory_runtime());
        s.push_str("\n\n");
    }

    s.push_str(super::helpers::webgl2_renderer());
    s.push_str("\n\n");

    if uses_memory {
        s.push_str(memory::webgl2_memory_runtime());
        s.push_str("\n\n");
    }

    // Inject feature JS modules (listen, voice, score, temporal, gravity, breed)
    for module_js in &shader.js_modules {
        s.push_str(module_js);
        s.push_str("\n\n");
    }

    // Custom element class
    s.push_str(&format!("class {class} extends HTMLElement {{\n"));
    s.push_str("  constructor() {\n");
    s.push_str("    super();\n");
    s.push_str("    this.attachShadow({ mode: 'open' });\n");
    s.push_str("    this._renderer = null;\n");
    s.push_str("    this._resizeObserver = null;\n");
    s.push_str("  }\n\n");

    s.push_str("  connectedCallback() {\n");
    s.push_str("    const style = document.createElement('style');\n");
    s.push_str("    style.textContent = ':host{display:block;width:100%;height:100%}canvas{width:100%;height:100%;display:block}';\n");
    s.push_str("    const canvas = document.createElement('canvas');\n");
    s.push_str("    this.shadowRoot.appendChild(style);\n");
    s.push_str("    this.shadowRoot.appendChild(canvas);\n");
    s.push_str("    this._canvas = canvas;\n");
    s.push_str("    this._initRenderer();\n");
    s.push_str("    this._resizeObserver = new ResizeObserver(() => this._resize());\n");
    s.push_str("    this._resizeObserver.observe(this);\n");
    s.push_str("  }\n\n");

    s.push_str("  disconnectedCallback() {\n");
    s.push_str("    this._renderer?.destroy();\n");
    s.push_str("    this._renderer = null;\n");
    s.push_str("    this._resizeObserver?.disconnect();\n");
    s.push_str("  }\n\n");

    s.push_str("  async _initRenderer() {\n");
    s.push_str("    const gpu = new GameRenderer(this._canvas, WGSL_V, WGSL_F, UNIFORMS);\n");
    s.push_str("    if (await gpu.init()) {\n");
    s.push_str("      this._renderer = gpu;\n");
    s.push_str("    } else {\n");
    s.push_str("      const gl = new GameRendererGL(this._canvas, GLSL_V, GLSL_F, UNIFORMS);\n");
    s.push_str("      if (gl.init()) {\n");
    s.push_str("        this._renderer = gl;\n");
    s.push_str("      } else {\n");
    s.push_str(&format!("        console.warn('game-{tag}: no WebGPU or WebGL2 support');\n"));
    s.push_str("        return;\n");
    s.push_str("      }\n");
    s.push_str("    }\n");
    s.push_str("    this._resize();\n");
    s.push_str("    if (typeof _gameReactSetup === 'function') _gameReactSetup(this._canvas, this._renderer);\n");
    s.push_str("    this._renderer.start();\n");
    s.push_str("  }\n\n");

    s.push_str("  _resize() {\n");
    s.push_str("    const rect = this.getBoundingClientRect();\n");
    s.push_str("    const dpr = window.devicePixelRatio || 1;\n");
    s.push_str("    this._canvas.width = Math.round(rect.width * dpr);\n");
    s.push_str("    this._canvas.height = Math.round(rect.height * dpr);\n");
    s.push_str("  }\n\n");

    s.push_str("  setParam(name, value) { this._renderer?.setParam(name, value); }\n");
    s.push_str("  setAudioData(data) { this._renderer?.setAudioData(data); }\n");
    s.push_str("  setAudioSource(bridge) { bridge?.subscribe(d => this._renderer?.setAudioData(d)); }\n\n");

    s.push_str("  static get observedAttributes() { return UNIFORMS.map(u => u.name); }\n");
    s.push_str("  attributeChangedCallback(name, _, val) {\n");
    s.push_str("    if (val !== null) this.setParam(name, parseFloat(val));\n");
    s.push_str("  }\n");
    s.push_str("}\n\n");

    s.push_str(&format!("customElements.define('game-{tag}', {class});\n"));
    s.push_str("})();\n");

    s
}

fn to_kebab(s: &str) -> String {
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

fn to_pascal(s: &str) -> String {
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

    #[test]
    fn component_has_custom_element_define() {
        let shader = ShaderOutput {
            name: "test-viz".into(),
            wgsl_fragment: "fn fs_main() {}".into(),
            wgsl_vertex: "fn vs_main() {}".into(),
            glsl_fragment: "void main(){}".into(),
            glsl_vertex: "void main(){}".into(),
            uniforms: vec![],
            uses_memory: false,
            js_modules: vec![],
            compute_wgsl: None,
        };
        let js = generate_component(&shader);
        assert!(js.contains("customElements.define('game-test-viz'"));
        assert!(js.contains("class TestViz extends HTMLElement"));
    }

    #[test]
    fn component_includes_both_renderers() {
        let shader = ShaderOutput {
            name: "demo".into(),
            wgsl_fragment: "wgsl".into(),
            wgsl_vertex: "wgsl_v".into(),
            glsl_fragment: "glsl".into(),
            glsl_vertex: "glsl_v".into(),
            uniforms: vec![UniformInfo { name: "speed".into(), default: 1.0 }],
            uses_memory: false,
            js_modules: vec![],
            compute_wgsl: None,
        };
        let js = generate_component(&shader);
        assert!(js.contains("class GameRenderer"));
        assert!(js.contains("class GameRendererGL"));
        assert!(js.contains("{name:'speed',default:1}"));
    }

    #[test]
    fn component_with_memory_includes_pingpong() {
        let shader = ShaderOutput {
            name: "trails".into(),
            wgsl_fragment: "wgsl".into(),
            wgsl_vertex: "wgsl_v".into(),
            glsl_fragment: "glsl".into(),
            glsl_vertex: "glsl_v".into(),
            uniforms: vec![],
            uses_memory: true,
            js_modules: vec![],
            compute_wgsl: None,
        };
        let js = generate_component(&shader);
        assert!(js.contains("USES_MEMORY = true"));
        assert!(js.contains("_initMemory"));
        assert!(js.contains("_initMemoryGL"));
    }

    #[test]
    fn component_with_listen_includes_pipeline() {
        let shader = ShaderOutput {
            name: "audio-viz".into(),
            wgsl_fragment: "wgsl".into(),
            wgsl_vertex: "wgsl_v".into(),
            glsl_fragment: "glsl".into(),
            glsl_vertex: "glsl_v".into(),
            uniforms: vec![],
            uses_memory: false,
            js_modules: vec!["class GameListenPipeline { /* listen */ }".into()],
            compute_wgsl: None,
        };
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
}
