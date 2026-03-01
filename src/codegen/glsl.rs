//! GLSL ES 3.0 shader generation from GAME AST.
//!
//! CRITICAL: This module produces correct GLSL, NOT WGSL syntax.
//! Key differences from WGSL:
//!   - Function params: `float func(vec2 p, float r)` NOT `func(p: vec2, r: float)`
//!   - Entry point: `void main()` NOT `void fs_main(input: VertexOutput)`
//!   - Variables: `float x = ...;` NOT `let x = ...;`
//!   - Output: `fragColor = ...;` NOT `return ...;`
//!   - No `select()` — use ternary `? :`
//!   - Uniforms: individual `uniform float u_xxx;`

use crate::ast::*;
use crate::codegen::memory;
use crate::codegen::stages::get_arg;
use crate::codegen::UniformInfo;

/// Generate a GLSL ES 3.0 fragment shader for a cinematic.
pub fn generate_fragment(
    cinematic: &Cinematic,
    uniforms: &[UniformInfo],
) -> String {
    let mut s = String::with_capacity(4096);

    // Header
    s.push_str("#version 300 es\nprecision highp float;\n\n");

    // Uniforms — individual declarations (NOT a struct)
    s.push_str("uniform float u_time;\n");
    s.push_str("uniform float u_audio_bass;\n");
    s.push_str("uniform float u_audio_mid;\n");
    s.push_str("uniform float u_audio_treble;\n");
    s.push_str("uniform float u_audio_energy;\n");
    s.push_str("uniform float u_audio_beat;\n");
    s.push_str("uniform vec2 u_resolution;\n");
    s.push_str("uniform vec2 u_mouse;\n");
    for u in uniforms {
        s.push_str(&format!("uniform float u_p_{};\n", u.name));
    }

    // Memory texture uniform (before varyings)
    if memory::any_layer_uses_memory(&cinematic.layers) {
        memory::emit_glsl_memory_bindings(&mut s);
    }

    s.push_str("\nin vec2 v_uv;\nout vec4 fragColor;\n\n");

    // Built-in helper functions (C-style params!)
    emit_glsl_builtins(&mut s, cinematic);

    // Entry point: void main()
    s.push_str("void main(){\n");
    s.push_str("    vec2 uv = v_uv * 2.0 - 1.0;\n");
    s.push_str("    float aspect = u_resolution.x / u_resolution.y;\n");
    s.push_str("    float time = fract(u_time / 120.0) * 120.0;\n\n");

    // Uniform param aliases
    for u in uniforms {
        s.push_str(&format!("    float {} = u_p_{};\n", u.name, u.name));
    }
    if !uniforms.is_empty() {
        s.push('\n');
    }

    let multi_layer = cinematic.layers.len() > 1;
    if multi_layer {
        s.push_str("    vec4 final_color = vec4(0.0, 0.0, 0.0, 1.0);\n\n");
    }

    for (i, layer) in cinematic.layers.iter().enumerate() {
        emit_glsl_layer(&mut s, layer, i, multi_layer);
    }

    if multi_layer {
        s.push_str("    fragColor = final_color;\n");
    }
    s.push_str("}\n");
    s
}

fn emit_glsl_builtins(s: &mut String, cinematic: &Cinematic) {
    let needs_circle = cinematic.layers.iter().any(|l| has_stage(l, "circle"));
    let needs_fbm = cinematic.layers.iter().any(|l| has_stage(l, "fbm"));

    // C-style function declarations — NOT WGSL style
    if needs_circle {
        s.push_str("float sdf_circle(vec2 p, float radius){\n");
        s.push_str("    return length(p) - radius;\n");
        s.push_str("}\n\n");
    }

    s.push_str("float apply_glow(float d, float intensity){\n");
    s.push_str("    return exp(-max(d, 0.0) * intensity * 8.0);\n");
    s.push_str("}\n\n");

    if needs_fbm {
        emit_glsl_fbm(s);
    }
}

fn emit_glsl_fbm(s: &mut String) {
    // hash2: vec2 -> float (C-style params, vec3 return from fract)
    s.push_str("float hash2(vec2 p){\n");
    s.push_str("    vec3 p3 = fract(vec3(p.x, p.y, p.x) * 0.1031);\n");
    s.push_str("    p3 += vec3(dot(p3, p3.yzx + 33.33));\n");
    s.push_str("    return fract((p3.x + p3.y) * p3.z);\n");
    s.push_str("}\n\n");

    // noise2: vec2 -> float (correct types throughout)
    s.push_str("float noise2(vec2 p){\n");
    s.push_str("    vec2 i = floor(p);\n");
    s.push_str("    vec2 f = fract(p);\n");
    s.push_str("    vec2 u = f * f * (3.0 - 2.0 * f);\n");
    s.push_str("    return mix(\n");
    s.push_str("        mix(hash2(i), hash2(i + vec2(1.0, 0.0)), u.x),\n");
    s.push_str("        mix(hash2(i + vec2(0.0, 1.0)), hash2(i + vec2(1.0, 1.0)), u.x),\n");
    s.push_str("        u.y\n");
    s.push_str("    ) * 2.0 - 1.0;\n");
    s.push_str("}\n\n");

    // fbm2: C-style params, proper variable declarations (NOT colon syntax)
    s.push_str("float fbm2(vec2 p, int octaves, float persistence, float lacunarity){\n");
    s.push_str("    float value = 0.0;\n");
    s.push_str("    float amplitude = 1.0;\n");
    s.push_str("    float frequency = 1.0;\n");
    s.push_str("    float max_val = 0.0;\n");
    s.push_str("    for (int i = 0; i < octaves; i++) {\n");
    s.push_str("        value += noise2(p * frequency) * amplitude;\n");
    s.push_str("        max_val += amplitude;\n");
    s.push_str("        amplitude *= persistence;\n");
    s.push_str("        frequency *= lacunarity;\n");
    s.push_str("    }\n");
    s.push_str("    return value / max_val;\n");
    s.push_str("}\n\n");
}

fn emit_glsl_layer(s: &mut String, layer: &Layer, idx: usize, multi: bool) {
    let body = match &layer.body {
        LayerBody::Pipeline(stages) => stages,
        _ => return,
    };

    s.push_str(&format!("    // ── Layer {idx}: {} ──\n", layer.name));
    if multi {
        s.push_str("    {\n");
    }
    let indent = if multi { "        " } else { "    " };

    s.push_str(&format!("{indent}vec2 p = vec2(uv.x * aspect, uv.y);\n"));

    for stage in body {
        emit_glsl_stage(s, stage, indent);
    }

    // Memory: mix with previous frame if this layer has memory
    if let Some(decay) = layer.memory {
        memory::emit_glsl_memory_mix(s, decay, indent);
    }

    if multi {
        s.push_str(&format!("{indent}vec3 lc = color_result.rgb;\n"));
        s.push_str(&format!("{indent}final_color = vec4(final_color.rgb + lc * 1.000, 1.0);\n"));
        s.push_str("    }\n\n");
    } else {
        s.push_str(&format!("{indent}fragColor = color_result;\n"));
    }
}

fn emit_glsl_stage(s: &mut String, stage: &Stage, indent: &str) {
    let args = &stage.args;
    match stage.name.as_str() {
        "circle" => {
            let r = get_arg(args, "radius", 0, "circle");
            s.push_str(&format!("{indent}float sdf_result = sdf_circle(p, {r});\n"));
        }
        "ring" => {
            let r = get_arg(args, "radius", 0, "ring");
            let w = get_arg(args, "width", 1, "ring");
            s.push_str(&format!("{indent}float sdf_result = abs(length(p) - {r}) - {w};\n"));
        }
        "glow" => {
            let intensity = get_arg(args, "intensity", 0, "glow");
            s.push_str(&format!("{indent}float glow_result = apply_glow(sdf_result, {intensity});\n\n"));
            s.push_str(&format!("{indent}vec4 color_result = vec4(vec3(glow_result), 1.0);\n"));
        }
        "tint" => {
            let r = get_arg(args, "r", 0, "tint");
            let g = get_arg(args, "g", 1, "tint");
            let b = get_arg(args, "b", 2, "tint");
            s.push_str(&format!("{indent}color_result = vec4(color_result.rgb * vec3({r}, {g}, {b}), 1.0);\n"));
        }
        "bloom" => {
            let thresh = get_arg(args, "threshold", 0, "bloom");
            let strength = get_arg(args, "strength", 1, "bloom");
            // GLSL: dot returns float, NOT vec3
            s.push_str(&format!("{indent}float pp_lum = dot(color_result.rgb, vec3(0.299, 0.587, 0.114));\n"));
            s.push_str(&format!("{indent}color_result = vec4(color_result.rgb + max(pp_lum - {thresh}, 0.0) * {strength}, 1.0);\n"));
        }
        "rotate" => {
            let angle = get_arg(args, "angle", 0, "rotate");
            // GLSL: use `float`, NOT `let`
            s.push_str(&format!("{indent}{{ float rc = cos({angle}); float rs = sin({angle});\n"));
            s.push_str(&format!("{indent}p = vec2(p.x * rc - p.y * rs, p.x * rs + p.y * rc); }}\n"));
        }
        "translate" => {
            let x = get_arg(args, "x", 0, "translate");
            let y = get_arg(args, "y", 1, "translate");
            s.push_str(&format!("{indent}p = p - vec2({x}, {y});\n"));
        }
        "scale" => {
            let sc = get_arg(args, "s", 0, "scale");
            s.push_str(&format!("{indent}p = p / {sc};\n"));
        }
        "mask_arc" => {
            let angle = get_arg(args, "angle", 0, "mask_arc");
            s.push_str(&format!("{indent}float arc_theta = atan(p.x, p.y) + 3.14159265359;\n"));
            // GLSL: use ternary, NOT select()
            s.push_str(&format!("{indent}sdf_result = (arc_theta < {angle} ? sdf_result : 999.0);\n"));
        }
        "shade" => {
            let r = get_arg(args, "r", 0, "shade");
            let g = get_arg(args, "g", 1, "shade");
            let b = get_arg(args, "b", 2, "shade");
            s.push_str(&format!("{indent}vec4 color_result = vec4(vec3({r}, {g}, {b}) * (1.0 - clamp(sdf_result, 0.0, 1.0)), 1.0);\n"));
        }
        "emissive" => {
            let intensity = get_arg(args, "intensity", 0, "emissive");
            s.push_str(&format!("{indent}float glow_result = apply_glow(sdf_result, {intensity});\n"));
            s.push_str(&format!("{indent}vec4 color_result = vec4(vec3(glow_result), glow_result);\n"));
        }
        "fbm" => {
            let sc = get_arg(args, "scale", 0, "fbm");
            let oct = get_arg(args, "octaves", 1, "fbm");
            let pers = get_arg(args, "persistence", 2, "fbm");
            let lac = get_arg(args, "lacunarity", 3, "fbm");
            s.push_str(&format!("{indent}float sdf_result = fbm2((p * {sc}), int({oct}), {pers}, {lac});\n"));
        }
        "grain" => {
            let amount = get_arg(args, "amount", 0, "grain");
            s.push_str(&format!("{indent}float grain_noise = fract(sin(dot(p, vec2(12.9898, 78.233)) + time) * 43758.5453);\n"));
            s.push_str(&format!("{indent}color_result = vec4(color_result.rgb + (grain_noise - 0.5) * {amount}, color_result.a);\n"));
        }
        _ => {
            s.push_str(&format!("{indent}// Unknown stage: {}\n", stage.name));
        }
    }
}

fn has_stage(layer: &Layer, name: &str) -> bool {
    match &layer.body {
        LayerBody::Pipeline(stages) => stages.iter().any(|s| s.name == name),
        _ => false,
    }
}

/// Generate the standard GLSL ES 3.0 vertex shader.
pub fn vertex_shader() -> &'static str {
    r#"#version 300 es
precision highp float;
out vec2 v_uv;
void main(){
    vec2 pos[3] = vec2[3](
        vec2(-1.0, -1.0),
        vec2(3.0, -1.0),
        vec2(-1.0, 3.0)
    );
    gl_Position = vec4(pos[gl_VertexID], 0.0, 1.0);
    v_uv = pos[gl_VertexID] * 0.5 + 0.5;
}
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cinematic(stages: Vec<Stage>) -> Cinematic {
        Cinematic {
            name: "test".into(),
            layers: vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                cast: None,
                body: LayerBody::Pipeline(stages),
            }],
            arcs: vec![],
            resonates: vec![],
        }
    }

    #[test]
    fn glsl_has_void_main() {
        let cin = make_cinematic(vec![
            Stage { name: "circle".into(), args: vec![] },
            Stage { name: "glow".into(), args: vec![] },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("void main()"), "must use void main(), got:\n{output}");
        assert!(!output.contains("fs_main"), "must NOT contain fs_main");
    }

    #[test]
    fn glsl_has_c_style_params() {
        let cin = make_cinematic(vec![
            Stage { name: "circle".into(), args: vec![] },
            Stage { name: "glow".into(), args: vec![] },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("float sdf_circle(vec2 p, float radius)"), "C-style params");
        assert!(output.contains("float apply_glow(float d, float intensity)"), "C-style params");
        assert!(!output.contains("p: vec2"), "must NOT have WGSL-style params");
    }

    #[test]
    fn glsl_uses_fragcolor_not_return() {
        let cin = make_cinematic(vec![
            Stage { name: "circle".into(), args: vec![] },
            Stage { name: "glow".into(), args: vec![] },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("fragColor = "), "must assign fragColor");
    }

    #[test]
    fn glsl_uses_float_not_let() {
        let cin = make_cinematic(vec![
            Stage { name: "circle".into(), args: vec![] },
            Stage { name: "glow".into(), args: vec![] },
            Stage { name: "tint".into(), args: vec![] },
        ]);
        let output = generate_fragment(&cin, &[]);
        // Main body should use `float` and `vec2`, not `let`
        assert!(!output.contains("\n    let "), "must NOT use `let` in GLSL body");
        assert!(output.contains("vec2 uv = "), "must use typed declarations");
    }

    #[test]
    fn glsl_bloom_uses_float_lum() {
        let cin = make_cinematic(vec![
            Stage { name: "circle".into(), args: vec![] },
            Stage { name: "glow".into(), args: vec![] },
            Stage { name: "bloom".into(), args: vec![] },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("float pp_lum = dot("), "dot() must return float");
        assert!(!output.contains("vec3 pp_lum"), "must NOT use vec3 for lum");
    }

    #[test]
    fn glsl_mask_arc_uses_ternary() {
        let cin = make_cinematic(vec![
            Stage { name: "ring".into(), args: vec![] },
            Stage { name: "mask_arc".into(), args: vec![
                Arg { name: None, value: Expr::Number(4.0) },
            ] },
            Stage { name: "glow".into(), args: vec![] },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("? sdf_result : 999.0"), "must use ternary");
        assert!(!output.contains("select("), "must NOT use select()");
    }

    #[test]
    fn glsl_rotate_uses_float_not_let() {
        let cin = make_cinematic(vec![
            Stage { name: "rotate".into(), args: vec![
                Arg { name: None, value: Expr::Number(1.0) },
            ] },
            Stage { name: "circle".into(), args: vec![] },
            Stage { name: "glow".into(), args: vec![] },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("float rc = cos("), "must use float, not let");
        assert!(output.contains("float rs = sin("), "must use float, not let");
    }

    #[test]
    fn glsl_vertex_shader_valid() {
        let vs = vertex_shader();
        assert!(vs.contains("void main()"));
        assert!(vs.contains("#version 300 es"));
        assert!(vs.contains("gl_Position"));
    }

    #[test]
    fn glsl_multi_layer_uses_fragcolor() {
        let cin = Cinematic {
            name: "multi".into(),
            layers: vec![
                Layer {
                    name: "a".into(), opts: vec![], memory: None, cast: None,
                    body: LayerBody::Pipeline(vec![
                        Stage { name: "circle".into(), args: vec![] },
                        Stage { name: "glow".into(), args: vec![] },
                    ]),
                },
                Layer {
                    name: "b".into(), opts: vec![], memory: None, cast: None,
                    body: LayerBody::Pipeline(vec![
                        Stage { name: "ring".into(), args: vec![] },
                        Stage { name: "glow".into(), args: vec![] },
                    ]),
                },
            ],
            arcs: vec![],
            resonates: vec![],
        };
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("vec4 final_color"));
        assert!(output.contains("fragColor = final_color"));
        assert!(!output.contains("return final_color"), "GLSL must NOT return in void main");
    }

    #[test]
    fn glsl_fbm_correct_types() {
        let cin = make_cinematic(vec![
            Stage { name: "fbm".into(), args: vec![] },
            Stage { name: "glow".into(), args: vec![] },
        ]);
        let output = generate_fragment(&cin, &[]);
        // hash2: vec2 param, vec3 local
        assert!(output.contains("float hash2(vec2 p)"), "C-style hash2");
        assert!(output.contains("vec3 p3 = fract("), "vec3 not float for p3");
        // noise2: vec2 params and locals
        assert!(output.contains("float noise2(vec2 p)"), "C-style noise2");
        assert!(output.contains("vec2 i = floor(p)"), "vec2 not float for i");
        assert!(output.contains("vec2 f = fract(p)"), "vec2 not float for f");
        assert!(output.contains("vec2 u = f * f"), "vec2 not float for u");
        // fbm2: C-style params, no colon syntax
        assert!(output.contains("float fbm2(vec2 p, int octaves, float persistence, float lacunarity)"));
        assert!(!output.contains("float value: float"), "no colon syntax in GLSL");
    }
}
