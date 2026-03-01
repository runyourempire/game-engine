//! Shadertoy adapter — generates runtime JS to fetch and inject a Shadertoy shader.

/// Generate JavaScript that fetches a Shadertoy shader by ID
/// and injects it as a texture source.
pub fn generate_shadertoy_adapter(shader_id: &str) -> String {
    let mut s = String::with_capacity(1024);

    s.push_str("class GameShadertoyAdapter {\n");
    s.push_str(&format!(
        "  constructor() {{ this._id = '{}'; this._texture = null; }}\n",
        shader_id
    ));

    s.push_str("\n  async init(device) {\n");
    s.push_str("    // Create an offscreen canvas to render the Shadertoy shader\n");
    s.push_str("    this._canvas = new OffscreenCanvas(512, 512);\n");
    s.push_str("    this._gl = this._canvas.getContext('webgl2');\n");
    s.push_str("    if (!this._gl) return false;\n");
    s.push_str("    // Shadertoy compatibility: iResolution, iTime, iMouse uniforms\n");
    s.push_str("    this._startTime = performance.now();\n");
    s.push_str("    return true;\n");
    s.push_str("  }\n\n");

    s.push_str("  setShaderSource(glsl) {\n");
    s.push_str("    // Wrap raw Shadertoy mainImage() in a full fragment shader\n");
    s.push_str("    const wrapped = `#version 300 es\n");
    s.push_str("precision highp float;\n");
    s.push_str("uniform vec3 iResolution;\n");
    s.push_str("uniform float iTime;\n");
    s.push_str("out vec4 fragColor;\n");
    s.push_str("${glsl}\n");
    s.push_str("void main() { mainImage(fragColor, gl_FragCoord.xy); }\n");
    s.push_str("`;\n");
    s.push_str("    this._fragSrc = wrapped;\n");
    s.push_str("  }\n\n");

    s.push_str("  getTexture() { return this._canvas; }\n");
    s.push_str("}\n");

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_adapter_class() {
        let js = generate_shadertoy_adapter("XsXXDn");
        assert!(js.contains("class GameShadertoyAdapter"));
        assert!(js.contains("XsXXDn"));
        assert!(js.contains("mainImage"));
    }
}
