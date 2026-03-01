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

    s.push_str(super::helpers::webgpu_renderer());
    s.push_str("\n\n");
    s.push_str(super::helpers::webgl2_renderer());
    s.push_str("\n\n");

    s.push_str("(async function() {\n");
    s.push_str("  const canvas = document.getElementById('c');\n");
    s.push_str("  function resize() {\n");
    s.push_str("    canvas.width = window.innerWidth * devicePixelRatio;\n");
    s.push_str("    canvas.height = window.innerHeight * devicePixelRatio;\n");
    s.push_str("  }\n");
    s.push_str("  window.addEventListener('resize', resize);\n");
    s.push_str("  resize();\n\n");
    s.push_str("  const gpu = new GameRenderer(canvas, WGSL_V, WGSL_F, UNIFORMS);\n");
    s.push_str("  if (await gpu.init()) { gpu.start(); return; }\n");
    s.push_str("  const gl = new GameRendererGL(canvas, GLSL_V, GLSL_F, UNIFORMS);\n");
    s.push_str("  if (gl.init()) { gl.start(); return; }\n");
    s.push_str("  document.body.textContent = 'No WebGPU or WebGL2 support.';\n");
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
        };
        let html = generate_html(&shader);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<title>demo"));
        assert!(html.contains("class GameRenderer"));
        assert!(html.contains("class GameRendererGL"));
        assert!(html.contains("</html>"));
    }
}
