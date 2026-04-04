//! Standalone HTML output format.
//!
//! Generates a single `.html` file with embedded shaders and renderer.

use crate::codegen::ShaderOutput;

/// Generate a self-contained HTML page.
pub fn generate_html(shader: &ShaderOutput) -> String {
    let wgsl_v = escape_html_js(&shader.wgsl_vertex);
    let wgsl_f = escape_html_js(&shader.wgsl_fragment);
    let glsl_v = escape_html_js(&shader.glsl_vertex);
    let glsl_f = escape_html_js(&shader.glsl_fragment);

    let uniform_defs_json = shader
        .uniforms
        .iter()
        .map(|u| format!("{{name:'{}',default:{}}}", u.name, u.default))
        .collect::<Vec<_>>()
        .join(",");

    // Build incrementally to avoid stack overflow from giant format! macro
    let mut s = String::with_capacity(16384);

    s.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    s.push_str("<meta charset=\"utf-8\">\n");
    s.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n");
    s.push_str(&format!("<title>{} — GAME</title>\n", shader.name));
    s.push_str("<style>*{margin:0;padding:0}html,body{width:100%;height:100%;overflow:hidden;background:#000}canvas{width:100%;height:100%;display:block}</style>\n");
    s.push_str("</head>\n<body>\n<canvas id=\"c\"></canvas>\n<script>\n");

    s.push_str(&format!("const WGSL_V = `{wgsl_v}`;\n"));
    s.push_str(&format!("const WGSL_F = `{wgsl_f}`;\n"));
    s.push_str(&format!("const GLSL_V = `{glsl_v}`;\n"));
    s.push_str(&format!("const GLSL_F = `{glsl_f}`;\n"));
    s.push_str(&format!("const UNIFORMS = [{uniform_defs_json}];\n\n"));

    let needs_prev_frame = shader.uses_memory || shader.uses_feedback;
    let pass_count = shader.pass_count;

    // Pass shader constants for HTML output
    if pass_count > 0 {
        for (i, pass_wgsl) in shader.pass_wgsl.iter().enumerate() {
            let escaped = escape_html_js(pass_wgsl);
            s.push_str(&format!("const PASS_WGSL_{i} = `{escaped}`;\n"));
        }
        let pass_refs: Vec<String> = (0..pass_count).map(|i| format!("PASS_WGSL_{i}")).collect();
        s.push_str(&format!(
            "const PASS_SHADERS = [{}];\n",
            pass_refs.join(",")
        ));
    }

    let tex_count = shader.textures.len();
    let tex_names: Vec<String> = shader.textures.iter().map(|t| t.name.clone()).collect();

    s.push_str(&super::helpers::webgpu_renderer(
        needs_prev_frame,
        pass_count,
        None,
        tex_count,
    ));
    s.push_str("\n\n");
    s.push_str(&super::helpers::webgl2_renderer(needs_prev_frame, &tex_names));
    s.push_str("\n\n");

    // Inject feature JS modules (listen, voice, score, temporal, gravity, breed)
    for module_js in &shader.js_modules {
        s.push_str(module_js);
        s.push_str("\n\n");
    }

    // Texture loading helper for standalone HTML
    if tex_count > 0 {
        s.push_str("async function loadImageTexture(renderer, index, url, name) {\n");
        s.push_str("  try {\n");
        s.push_str("    const img = new Image();\n");
        s.push_str("    img.crossOrigin = 'anonymous';\n");
        s.push_str("    img.src = url;\n");
        s.push_str("    await img.decode();\n");
        s.push_str("    const bitmap = await createImageBitmap(img);\n");
        s.push_str("    if (renderer.setUserTexture && renderer.device) {\n");
        s.push_str("      const tex = renderer.device.createTexture({\n");
        s.push_str("        size: [bitmap.width, bitmap.height],\n");
        s.push_str("        format: 'rgba8unorm',\n");
        s.push_str("        usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST | GPUTextureUsage.RENDER_ATTACHMENT,\n");
        s.push_str("      });\n");
        s.push_str("      renderer.device.queue.copyExternalImageToTexture({ source: bitmap }, { texture: tex }, [bitmap.width, bitmap.height]);\n");
        s.push_str("      renderer.setUserTexture(index, tex);\n");
        s.push_str("    } else if (renderer.setUserTextureGL && renderer.gl) {\n");
        s.push_str("      const gl = renderer.gl;\n");
        s.push_str("      const glTex = gl.createTexture();\n");
        s.push_str("      gl.bindTexture(gl.TEXTURE_2D, glTex);\n");
        s.push_str("      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, bitmap);\n");
        s.push_str("      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);\n");
        s.push_str("      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);\n");
        s.push_str("      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);\n");
        s.push_str("      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);\n");
        s.push_str("      gl.bindTexture(gl.TEXTURE_2D, null);\n");
        s.push_str("      renderer.setUserTextureGL(name, glTex);\n");
        s.push_str("    }\n");
        s.push_str("  } catch(e) { console.warn('Texture load failed:', name, e); }\n");
        s.push_str("}\n\n");
    }

    s.push_str("(async function() {\n");
    s.push_str("  const canvas = document.getElementById('c');\n");
    s.push_str("  function resize() {\n");
    s.push_str("    canvas.width = window.innerWidth * devicePixelRatio;\n");
    s.push_str("    canvas.height = window.innerHeight * devicePixelRatio;\n");
    s.push_str("  }\n");
    s.push_str("  window.addEventListener('resize', resize);\n");
    s.push_str("  resize();\n\n");

    s.push_str("  let renderer = null;\n");
    s.push_str("  const gpu = new GameRenderer(canvas, WGSL_V, WGSL_F, UNIFORMS");
    if pass_count > 0 {
        s.push_str(", PASS_SHADERS");
    }
    if tex_count > 0 {
        s.push_str(&format!(", {tex_count}"));
    }
    s.push_str(");\n");
    s.push_str("  if (await gpu.init()) { renderer = gpu; }\n");
    s.push_str("  else {\n");
    s.push_str("    const gl = new GameRendererGL(canvas, GLSL_V, GLSL_F, UNIFORMS);\n");
    s.push_str("    if (gl.init()) { renderer = gl; }\n");
    s.push_str("  }\n");
    s.push_str("  if (!renderer) { document.body.textContent = 'No WebGPU or WebGL2 support.'; return; }\n\n");

    // Load textures before starting
    if tex_count > 0 {
        s.push_str("  // Load declared textures\n");
        for (i, tex) in shader.textures.iter().enumerate() {
            if let Some(ref url) = tex.source {
                s.push_str(&format!(
                    "  await loadImageTexture(renderer, {i}, '{}', '{}');\n",
                    escape_html_js(url),
                    escape_html_js(&tex.name)
                ));
            }
        }
        s.push_str("\n");
    }

    s.push_str("  renderer.start();\n");
    s.push_str("})();\n");

    s.push_str("</script>\n</body>\n</html>\n");

    s
}

/// Generate an Art Blocks / fxhash compatible HTML page.
///
/// Differences from standard HTML:
/// - Injects seeded PRNG (splitmix32)
/// - Reads `fxhash` from URL hash or platform variable
/// - Deterministic: same seed → same output
/// - Self-contained with no external dependencies
pub fn generate_artblocks_html(shader: &ShaderOutput, seed: Option<u64>) -> String {
    let wgsl_v = escape_html_js(&shader.wgsl_vertex);
    let wgsl_f = escape_html_js(&shader.wgsl_fragment);
    let glsl_v = escape_html_js(&shader.glsl_vertex);
    let glsl_f = escape_html_js(&shader.glsl_fragment);

    let uniform_defs_json = shader
        .uniforms
        .iter()
        .map(|u| format!("{{name:'{}',default:{}}}", u.name, u.default))
        .collect::<Vec<_>>()
        .join(",");

    let mut s = String::with_capacity(16384);

    s.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    s.push_str("<meta charset=\"utf-8\">\n");
    s.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n");
    s.push_str(&format!(
        "<title>{} — GAME (Art Blocks)</title>\n",
        shader.name
    ));
    s.push_str("<style>*{margin:0;padding:0}html,body{width:100%;height:100%;overflow:hidden;background:#000}canvas{width:100%;height:100%;display:block}</style>\n");
    s.push_str("</head>\n<body>\n<canvas id=\"c\"></canvas>\n<script>\n");

    // Seeded PRNG (splitmix32)
    s.push_str("// Deterministic PRNG — splitmix32\n");
    s.push_str("let _fxhash = window.fxhash || window.location.hash.slice(1) || '");
    s.push_str(&format!("{}", seed.unwrap_or(0)));
    s.push_str("';\n");
    s.push_str("let _seed = parseInt(_fxhash, 16) || parseInt(_fxhash, 10) || 0;\n");
    s.push_str("function fxrand() {\n");
    s.push_str("  _seed |= 0; _seed = _seed + 0x9e3779b9 | 0;\n");
    s.push_str("  let t = _seed ^ _seed >>> 16;\n");
    s.push_str("  t = Math.imul(t, 0x21f0aaad);\n");
    s.push_str("  t = t ^ t >>> 15;\n");
    s.push_str("  t = Math.imul(t, 0x735a2d97);\n");
    s.push_str("  t = t ^ t >>> 15;\n");
    s.push_str("  return (t >>> 0) / 4294967296;\n");
    s.push_str("}\n");
    s.push_str("// Override Math.random for determinism\n");
    s.push_str("Math.random = fxrand;\n\n");

    s.push_str(&format!("const WGSL_V = `{wgsl_v}`;\n"));
    s.push_str(&format!("const WGSL_F = `{wgsl_f}`;\n"));
    s.push_str(&format!("const GLSL_V = `{glsl_v}`;\n"));
    s.push_str(&format!("const GLSL_F = `{glsl_f}`;\n"));
    s.push_str(&format!("const UNIFORMS = [{uniform_defs_json}];\n\n"));

    let needs_prev_frame_ab = shader.uses_memory || shader.uses_feedback;
    let pass_count_ab = shader.pass_count;

    if pass_count_ab > 0 {
        for (i, pass_wgsl) in shader.pass_wgsl.iter().enumerate() {
            let escaped = escape_html_js(pass_wgsl);
            s.push_str(&format!("const PASS_WGSL_{i} = `{escaped}`;\n"));
        }
        let pass_refs: Vec<String> = (0..pass_count_ab)
            .map(|i| format!("PASS_WGSL_{i}"))
            .collect();
        s.push_str(&format!(
            "const PASS_SHADERS = [{}];\n",
            pass_refs.join(",")
        ));
    }

    s.push_str(&super::helpers::webgpu_renderer(
        needs_prev_frame_ab,
        pass_count_ab,
        None,
        0,
    ));
    s.push_str("\n\n");
    s.push_str(&super::helpers::webgl2_renderer(needs_prev_frame_ab, &[]));
    s.push_str("\n\n");

    for module_js in &shader.js_modules {
        s.push_str(module_js);
        s.push_str("\n\n");
    }

    s.push_str("(async function() {\n");
    s.push_str("  const canvas = document.getElementById('c');\n");
    s.push_str("  function resize() {\n");
    s.push_str("    canvas.width = window.innerWidth * devicePixelRatio;\n");
    s.push_str("    canvas.height = window.innerHeight * devicePixelRatio;\n");
    s.push_str("  }\n");
    s.push_str("  window.addEventListener('resize', resize);\n");
    s.push_str("  resize();\n\n");
    s.push_str("  const gpu = new GameRenderer(canvas, WGSL_V, WGSL_F, UNIFORMS");
    if pass_count_ab > 0 {
        s.push_str(", PASS_SHADERS");
    }
    s.push_str(");\n");
    s.push_str("  if (await gpu.init()) { gpu.start(); return; }\n");
    s.push_str("  const gl = new GameRendererGL(canvas, GLSL_V, GLSL_F, UNIFORMS);\n");
    s.push_str("  if (gl.init()) { gl.start(); return; }\n");
    s.push_str("  document.body.textContent = 'No WebGPU or WebGL2 support.';\n");
    s.push_str("})();\n");

    // fxhash preview trigger
    s.push_str("// Art Blocks preview trigger\n");
    s.push_str("if (typeof window.fxpreview === 'function') { setTimeout(fxpreview, 2000); }\n");

    s.push_str("</script>\n</body>\n</html>\n");

    s
}

/// Generate a wallpaper-optimized HTML page.
///
/// Differences from standard HTML:
/// - Adaptive FPS governor with configurable default (30fps)
/// - Auto-pause on document hidden / fullscreen app
/// - Resolution scaling for power management
/// - Wallpaper Engine compatibility bridge (`window.wallpaperPropertyListener`)
/// - Complexity metadata for runtime power decisions
/// - Black background, no UI chrome
pub fn generate_wallpaper_html(shader: &ShaderOutput) -> String {
    let wgsl_v = escape_html_js(&shader.wgsl_vertex);
    let wgsl_f = escape_html_js(&shader.wgsl_fragment);
    let glsl_v = escape_html_js(&shader.glsl_vertex);
    let glsl_f = escape_html_js(&shader.glsl_fragment);

    let uniform_defs_json = shader
        .uniforms
        .iter()
        .map(|u| format!("{{name:'{}',default:{}}}", u.name, u.default))
        .collect::<Vec<_>>()
        .join(",");

    let mut s = String::with_capacity(16384);

    s.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    s.push_str("<meta charset=\"utf-8\">\n");
    s.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n");
    s.push_str(&format!("<title>{} — GAME Wallpaper</title>\n", shader.name));
    s.push_str("<style>*{margin:0;padding:0;box-sizing:border-box}html,body{width:100%;height:100%;overflow:hidden;background:#000}canvas{width:100%;height:100%;display:block;image-rendering:auto}</style>\n");
    s.push_str("</head>\n<body>\n<canvas id=\"c\"></canvas>\n<script>\n");

    s.push_str(&format!("const WGSL_V = `{wgsl_v}`;\n"));
    s.push_str(&format!("const WGSL_F = `{wgsl_f}`;\n"));
    s.push_str(&format!("const GLSL_V = `{glsl_v}`;\n"));
    s.push_str(&format!("const GLSL_F = `{glsl_f}`;\n"));
    s.push_str(&format!("const UNIFORMS = [{uniform_defs_json}];\n"));

    // Complexity metadata
    s.push_str(&format!(
        "const COMPLEXITY = {{layers:{},fbmOctaves:{},passes:{},memory:{},compute:{},is3d:{},tier:'{}'}};\n\n",
        shader.complexity.layer_count,
        shader.complexity.total_fbm_octaves,
        shader.complexity.pass_count,
        shader.complexity.uses_memory,
        shader.complexity.uses_compute,
        shader.complexity.is_3d,
        shader.complexity.tier,
    ));

    let needs_prev_frame = shader.uses_memory || shader.uses_feedback;
    let pass_count = shader.pass_count;

    if pass_count > 0 {
        for (i, pass_wgsl) in shader.pass_wgsl.iter().enumerate() {
            let escaped = escape_html_js(pass_wgsl);
            s.push_str(&format!("const PASS_WGSL_{i} = `{escaped}`;\n"));
        }
        let pass_refs: Vec<String> = (0..pass_count).map(|i| format!("PASS_WGSL_{i}")).collect();
        s.push_str(&format!(
            "const PASS_SHADERS = [{}];\n",
            pass_refs.join(",")
        ));
    }

    let wp_tex_count = shader.textures.len();
    let wp_tex_names: Vec<String> = shader.textures.iter().map(|t| t.name.clone()).collect();

    s.push_str(&super::helpers::webgpu_renderer(
        needs_prev_frame,
        pass_count,
        None,
        wp_tex_count,
    ));
    s.push_str("\n\n");
    s.push_str(&super::helpers::webgl2_renderer(needs_prev_frame, &wp_tex_names));
    s.push_str("\n\n");

    // Inject feature JS modules
    for module_js in &shader.js_modules {
        s.push_str(module_js);
        s.push_str("\n\n");
    }

    // Texture loading helper
    if wp_tex_count > 0 {
        s.push_str("async function loadImageTexture(renderer, index, url, name) {\n");
        s.push_str("  try {\n");
        s.push_str("    const img = new Image(); img.crossOrigin = 'anonymous'; img.src = url;\n");
        s.push_str("    await img.decode();\n");
        s.push_str("    const bitmap = await createImageBitmap(img);\n");
        s.push_str("    if (renderer.setUserTexture && renderer.device) {\n");
        s.push_str("      const tex = renderer.device.createTexture({ size: [bitmap.width, bitmap.height], format: 'rgba8unorm', usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST | GPUTextureUsage.RENDER_ATTACHMENT });\n");
        s.push_str("      renderer.device.queue.copyExternalImageToTexture({ source: bitmap }, { texture: tex }, [bitmap.width, bitmap.height]);\n");
        s.push_str("      renderer.setUserTexture(index, tex);\n");
        s.push_str("    } else if (renderer.setUserTextureGL && renderer.gl) {\n");
        s.push_str("      const gl = renderer.gl;\n");
        s.push_str("      const glTex = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, glTex);\n");
        s.push_str("      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, bitmap);\n");
        s.push_str("      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);\n");
        s.push_str("      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);\n");
        s.push_str("      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);\n");
        s.push_str("      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);\n");
        s.push_str("      gl.bindTexture(gl.TEXTURE_2D, null);\n");
        s.push_str("      renderer.setUserTextureGL(name, glTex);\n");
        s.push_str("    }\n");
        s.push_str("  } catch(e) { console.warn('Texture load failed:', name, e); }\n");
        s.push_str("}\n\n");
    }

    // Wallpaper Engine compatibility bridge
    s.push_str("// Wallpaper Engine API bridge\n");
    s.push_str("let _wpFps = 30;\n");
    s.push_str("window.wallpaperPropertyListener = {\n");
    s.push_str("  applyGeneralProperties: function(props) {\n");
    s.push_str("    if (props.fps) { _wpFps = props.fps; if (_renderer) _renderer.setFPS(_wpFps); }\n");
    s.push_str("  },\n");
    s.push_str("  applyUserProperties: function(props) {\n");
    s.push_str("    if (_renderer) {\n");
    s.push_str("      for (const [k, v] of Object.entries(props)) {\n");
    s.push_str("        if (v && typeof v.value !== 'undefined') _renderer.setParam(k, parseFloat(v.value));\n");
    s.push_str("      }\n");
    s.push_str("    }\n");
    s.push_str("  }\n");
    s.push_str("};\n\n");

    // Main initialization
    s.push_str("let _renderer = null;\n");
    s.push_str("(async function() {\n");
    s.push_str("  const canvas = document.getElementById('c');\n");
    s.push_str("  let resScale = 1.0;\n");
    s.push_str("  if (COMPLEXITY.tier === 'extreme' && devicePixelRatio > 1.5) resScale = 0.5;\n");
    s.push_str("  else if (COMPLEXITY.tier === 'heavy' && devicePixelRatio > 2) resScale = 0.75;\n");
    s.push_str("  function resize() {\n");
    s.push_str("    canvas.width = Math.round(window.innerWidth * devicePixelRatio * resScale);\n");
    s.push_str("    canvas.height = Math.round(window.innerHeight * devicePixelRatio * resScale);\n");
    s.push_str("    if (_renderer?._resizeMemory) _renderer._resizeMemory();\n");
    s.push_str("    if (_renderer?._resizePassFBOs) _renderer._resizePassFBOs();\n");
    s.push_str("  }\n");
    s.push_str("  window.addEventListener('resize', resize);\n");
    s.push_str("  resize();\n\n");

    s.push_str("  const gpu = new GameRenderer(canvas, WGSL_V, WGSL_F, UNIFORMS");
    if pass_count > 0 {
        s.push_str(", PASS_SHADERS");
    }
    if wp_tex_count > 0 {
        s.push_str(&format!(", {wp_tex_count}"));
    }
    s.push_str(");\n");
    s.push_str("  if (await gpu.init()) { _renderer = gpu; }\n");
    s.push_str("  else {\n");
    s.push_str("    const gl = new GameRendererGL(canvas, GLSL_V, GLSL_F, UNIFORMS);\n");
    s.push_str("    if (gl.init()) { _renderer = gl; }\n");
    s.push_str("  }\n");
    s.push_str("  if (!_renderer) { document.body.textContent = 'No GPU support.'; return; }\n\n");

    // Load textures before starting
    if wp_tex_count > 0 {
        for (i, tex) in shader.textures.iter().enumerate() {
            if let Some(ref url) = tex.source {
                s.push_str(&format!(
                    "  await loadImageTexture(_renderer, {i}, '{}', '{}');\n",
                    escape_html_js(url),
                    escape_html_js(&tex.name)
                ));
            }
        }
        s.push_str("\n");
    }

    s.push_str("  _renderer.setFPS(_wpFps);\n");
    s.push_str("  _renderer.start();\n");
    s.push_str("})();\n");

    s.push_str("</script>\n</body>\n</html>\n");

    s
}

fn escape_html_js(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${")
        .replace("</", "<\\/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::ShaderOutput;

    #[test]
    fn html_output_is_valid() {
        let shader = ShaderOutput {
            name: "demo".into(),
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
            is_3d: false,
            has_arc_enter: false,
            has_arc_exit: false,
            has_arc_hover: false,
            textures: vec![],
            has_states: false,
            states_js: None,
            particles_sim_wgsl: None,
            particles_raster_wgsl: None,
            complexity: crate::codegen::ShaderComplexity::default(),
        };
        let html = generate_html(&shader);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<title>demo"));
        assert!(html.contains("class GameRenderer"));
        assert!(html.contains("class GameRendererGL"));
        assert!(html.contains("</html>"));
    }

    #[test]
    fn wallpaper_html_has_all_features() {
        let shader = ShaderOutput {
            name: "aurora".into(),
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
            is_3d: false,
            has_arc_enter: false,
            has_arc_exit: false,
            has_arc_hover: false,
            textures: vec![],
            has_states: false,
            states_js: None,
            particles_sim_wgsl: None,
            particles_raster_wgsl: None,
            complexity: crate::codegen::ShaderComplexity::default(),
        };
        let html = generate_wallpaper_html(&shader);
        // Basic structure
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("GAME Wallpaper"));
        assert!(html.contains("</html>"));
        // Wallpaper Engine API bridge
        assert!(html.contains("wallpaperPropertyListener"), "should have WE API bridge");
        assert!(html.contains("applyGeneralProperties"), "should handle WE general props");
        assert!(html.contains("applyUserProperties"), "should handle WE user props");
        // Complexity metadata
        assert!(html.contains("const COMPLEXITY = {"), "should embed complexity");
        assert!(html.contains("tier:'minimal'"), "should have tier");
        // FPS governor
        assert!(html.contains("setFPS(_wpFps)"), "should set default FPS");
        // Resolution scaling
        assert!(html.contains("resScale"), "should have resolution scaling");
        assert!(html.contains("COMPLEXITY.tier"), "should use complexity for auto-scaling");
        // Renderers
        assert!(html.contains("class GameRenderer"));
        assert!(html.contains("class GameRendererGL"));
    }
}
