//! WGSL shader generation from GAME AST.

use crate::ast::*;
use crate::codegen::memory;
use crate::codegen::stages::get_arg;
use crate::codegen::UniformInfo;

/// Generate a WGSL fragment shader for a cinematic.
pub fn generate_fragment(cinematic: &Cinematic, uniforms: &[UniformInfo]) -> String {
    let mut s = String::with_capacity(4096);

    // Uniform struct
    s.push_str("struct Uniforms {\n");
    s.push_str("    time: f32,\n");
    s.push_str("    audio_bass: f32,\n");
    s.push_str("    audio_mid: f32,\n");
    s.push_str("    audio_treble: f32,\n");
    s.push_str("    audio_energy: f32,\n");
    s.push_str("    audio_beat: f32,\n");
    s.push_str("    resolution: vec2<f32>,\n");
    s.push_str("    mouse: vec2<f32>,\n");
    for u in uniforms {
        s.push_str(&format!("    p_{}: f32,\n", u.name));
    }
    s.push_str("};\n\n");
    s.push_str("@group(0) @binding(0) var<uniform> u: Uniforms;\n\n");

    // Memory bindings (Group 1) — only when any layer uses memory
    if memory::any_layer_uses_memory(&cinematic.layers) {
        memory::emit_wgsl_memory_bindings(&mut s);
    }

    // Vertex output struct
    s.push_str("struct VertexOutput {\n");
    s.push_str("    @builtin(position) pos: vec4<f32>,\n");
    s.push_str("    @location(0) uv: vec2<f32>,\n");
    s.push_str("};\n\n");

    // Built-in helper functions
    emit_wgsl_builtins(&mut s, cinematic);

    // Fragment entry
    s.push_str("@fragment\n");
    s.push_str("fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {\n");
    s.push_str("    let uv = input.uv * 2.0 - 1.0;\n");
    s.push_str("    let aspect = u.resolution.x / u.resolution.y;\n");
    s.push_str("    let time = fract(u.time / 120.0) * 120.0;\n\n");

    // Uniform param aliases
    for u in uniforms {
        s.push_str(&format!("    let {} = u.p_{};\n", u.name, u.name));
    }
    if !uniforms.is_empty() {
        s.push('\n');
    }

    let multi_layer = cinematic.layers.len() > 1;
    if multi_layer {
        s.push_str("    var final_color = vec4<f32>(0.0, 0.0, 0.0, 1.0);\n\n");
    }

    for (i, layer) in cinematic.layers.iter().enumerate() {
        emit_wgsl_layer(&mut s, layer, i, multi_layer);
    }

    if multi_layer {
        s.push_str("    return final_color;\n");
    }
    s.push_str("}\n");
    s
}

fn emit_wgsl_builtins(s: &mut String, cinematic: &Cinematic) {
    let needs_circle = cinematic.layers.iter().any(|l| has_stage(l, "circle"));
    let needs_fbm = cinematic.layers.iter().any(|l| has_stage(l, "fbm"));
    let needs_warp = cinematic.layers.iter().any(|l| has_stage(l, "warp"));
    let needs_simplex = cinematic.layers.iter().any(|l| has_stage(l, "simplex"));
    let needs_voronoi = cinematic.layers.iter().any(|l| has_stage(l, "voronoi"));
    let needs_palette = cinematic.layers.iter().any(|l| has_stage(l, "palette"));

    if needs_circle {
        s.push_str("fn sdf_circle(p: vec2<f32>, radius: f32) -> f32 {\n");
        s.push_str("    return length(p) - radius;\n");
        s.push_str("}\n\n");
    }

    s.push_str("fn apply_glow(d: f32, intensity: f32) -> f32 {\n");
    s.push_str("    return exp(-max(d, 0.0) * intensity * 8.0);\n");
    s.push_str("}\n\n");

    if needs_fbm || needs_warp || needs_simplex {
        emit_wgsl_fbm(s);
    }

    if needs_voronoi {
        emit_wgsl_voronoi(s);
    }

    if needs_palette {
        emit_wgsl_palette(s);
    }
}

fn emit_wgsl_fbm(s: &mut String) {
    s.push_str("fn hash2(p: vec2<f32>) -> f32 {\n");
    s.push_str("    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);\n");
    s.push_str("    p3 = p3 + vec3<f32>(dot(p3, p3.yzx + 33.33));\n");
    s.push_str("    return fract((p3.x + p3.y) * p3.z);\n");
    s.push_str("}\n\n");

    s.push_str("fn noise2(p: vec2<f32>) -> f32 {\n");
    s.push_str("    let i = floor(p);\n");
    s.push_str("    let f = fract(p);\n");
    s.push_str("    let u_v = f * f * (3.0 - 2.0 * f);\n");
    s.push_str("    return mix(\n");
    s.push_str("        mix(hash2(i), hash2(i + vec2<f32>(1.0, 0.0)), u_v.x),\n");
    s.push_str(
        "        mix(hash2(i + vec2<f32>(0.0, 1.0)), hash2(i + vec2<f32>(1.0, 1.0)), u_v.x),\n",
    );
    s.push_str("        u_v.y\n");
    s.push_str("    ) * 2.0 - 1.0;\n");
    s.push_str("}\n\n");

    s.push_str("fn fbm2(p: vec2<f32>, octaves: i32, persistence: f32, lacunarity: f32) -> f32 {\n");
    s.push_str("    var value: f32 = 0.0;\n");
    s.push_str("    var amplitude: f32 = 1.0;\n");
    s.push_str("    var frequency: f32 = 1.0;\n");
    s.push_str("    var max_val: f32 = 0.0;\n");
    s.push_str("    for (var i: i32 = 0; i < octaves; i = i + 1) {\n");
    s.push_str("        value = value + noise2(p * frequency) * amplitude;\n");
    s.push_str("        max_val = max_val + amplitude;\n");
    s.push_str("        amplitude = amplitude * persistence;\n");
    s.push_str("        frequency = frequency * lacunarity;\n");
    s.push_str("    }\n");
    s.push_str("    return value / max_val;\n");
    s.push_str("}\n\n");
}

fn emit_wgsl_voronoi(s: &mut String) {
    s.push_str("fn hash2v(p: vec2<f32>) -> vec2<f32> {\n");
    s.push_str(
        "    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));\n",
    );
    s.push_str("    let pp = p3 + vec3<f32>(dot(p3, p3.yzx + 33.33));\n");
    s.push_str("    return fract(vec2<f32>((pp.x + pp.y) * pp.z, (pp.x + pp.z) * pp.y));\n");
    s.push_str("}\n\n");

    s.push_str("fn voronoi2(p: vec2<f32>) -> f32 {\n");
    s.push_str("    let n = floor(p);\n");
    s.push_str("    let f = fract(p);\n");
    s.push_str("    var md: f32 = 8.0;\n");
    s.push_str("    for (var j: i32 = -1; j <= 1; j = j + 1) {\n");
    s.push_str("        for (var i: i32 = -1; i <= 1; i = i + 1) {\n");
    s.push_str("            let g = vec2<f32>(f32(i), f32(j));\n");
    s.push_str("            let o = hash2v(n + g);\n");
    s.push_str("            let r = g + o - f;\n");
    s.push_str("            let d = dot(r, r);\n");
    s.push_str("            md = min(md, d);\n");
    s.push_str("        }\n");
    s.push_str("    }\n");
    s.push_str("    return sqrt(md);\n");
    s.push_str("}\n\n");
}

fn emit_wgsl_palette(s: &mut String) {
    s.push_str("fn cosine_palette(t: f32, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, d: vec3<f32>) -> vec3<f32> {\n");
    s.push_str("    return a + b * cos(6.28318 * (c * t + d));\n");
    s.push_str("}\n\n");
}

fn emit_wgsl_layer(s: &mut String, layer: &Layer, idx: usize, multi: bool) {
    let body = match &layer.body {
        LayerBody::Pipeline(stages) => stages,
        _ => return,
    };

    s.push_str(&format!("    // ── Layer {idx}: {} ──\n", layer.name));
    if multi {
        s.push_str("    {\n");
    }
    let indent = if multi { "        " } else { "    " };

    s.push_str(&format!(
        "{indent}var p = vec2<f32>(uv.x * aspect, uv.y);\n"
    ));

    for stage in body {
        emit_wgsl_stage(s, stage, indent);
    }

    // Memory: mix with previous frame if this layer has memory
    if let Some(decay) = layer.memory {
        memory::emit_wgsl_memory_mix(s, decay, indent);
    }

    if multi {
        s.push_str(&format!("{indent}let lc = color_result.rgb;\n"));
        s.push_str(&format!(
            "{indent}final_color = vec4<f32>(final_color.rgb + lc * 1.0, 1.0);\n"
        ));
        s.push_str("    }\n\n");
    } else {
        s.push_str(&format!("{indent}return color_result;\n"));
    }
}

fn emit_wgsl_stage(s: &mut String, stage: &Stage, indent: &str) {
    let args = &stage.args;
    match stage.name.as_str() {
        "circle" => {
            let r = get_arg(args, "radius", 0, "circle");
            s.push_str(&format!("{indent}let sdf_result = sdf_circle(p, {r});\n"));
        }
        "ring" => {
            let r = get_arg(args, "radius", 0, "ring");
            let w = get_arg(args, "width", 1, "ring");
            s.push_str(&format!(
                "{indent}let sdf_result = abs(length(p) - {r}) - {w};\n"
            ));
        }
        "glow" => {
            let intensity = get_arg(args, "intensity", 0, "glow");
            s.push_str(&format!(
                "{indent}let glow_result = apply_glow(sdf_result, {intensity});\n"
            ));
            s.push_str(&format!(
                "{indent}var color_result = vec4<f32>(vec3<f32>(glow_result), 1.0);\n"
            ));
        }
        "tint" => {
            let r = get_arg(args, "r", 0, "tint");
            let g = get_arg(args, "g", 1, "tint");
            let b = get_arg(args, "b", 2, "tint");
            s.push_str(&format!("{indent}color_result = vec4<f32>(color_result.rgb * vec3<f32>({r}, {g}, {b}), 1.0);\n"));
        }
        "bloom" => {
            let thresh = get_arg(args, "threshold", 0, "bloom");
            let strength = get_arg(args, "strength", 1, "bloom");
            s.push_str(&format!(
                "{indent}let pp_lum = dot(color_result.rgb, vec3<f32>(0.299, 0.587, 0.114));\n"
            ));
            s.push_str(&format!("{indent}color_result = vec4<f32>(color_result.rgb + max(pp_lum - {thresh}, 0.0) * {strength}, 1.0);\n"));
        }
        "rotate" => {
            let angle = get_arg(args, "angle", 0, "rotate");
            s.push_str(&format!(
                "{indent}{{ let rc = cos({angle}); let rs = sin({angle});\n"
            ));
            s.push_str(&format!(
                "{indent}p = vec2<f32>(p.x * rc - p.y * rs, p.x * rs + p.y * rc); }}\n"
            ));
        }
        "translate" => {
            let x = get_arg(args, "x", 0, "translate");
            let y = get_arg(args, "y", 1, "translate");
            s.push_str(&format!("{indent}p = p - vec2<f32>({x}, {y});\n"));
        }
        "scale" => {
            let sc = get_arg(args, "s", 0, "scale");
            s.push_str(&format!("{indent}p = p / {sc};\n"));
        }
        "mask_arc" => {
            let angle = get_arg(args, "angle", 0, "mask_arc");
            s.push_str(&format!(
                "{indent}let arc_theta = atan2(p.x, p.y) + 3.14159265359;\n"
            ));
            s.push_str(&format!(
                "{indent}sdf_result = select(999.0, sdf_result, arc_theta < {angle});\n"
            ));
        }
        "shade" => {
            let r = get_arg(args, "r", 0, "shade");
            let g = get_arg(args, "g", 1, "shade");
            let b = get_arg(args, "b", 2, "shade");
            s.push_str(&format!("{indent}var color_result = vec4<f32>(vec3<f32>({r}, {g}, {b}) * (1.0 - clamp(sdf_result, 0.0, 1.0)), 1.0);\n"));
        }
        "emissive" => {
            let intensity = get_arg(args, "intensity", 0, "emissive");
            s.push_str(&format!(
                "{indent}let glow_result = apply_glow(sdf_result, {intensity});\n"
            ));
            s.push_str(&format!(
                "{indent}var color_result = vec4<f32>(vec3<f32>(glow_result), glow_result);\n"
            ));
        }
        "fbm" => {
            let sc = get_arg(args, "scale", 0, "fbm");
            let oct = get_arg(args, "octaves", 1, "fbm");
            let pers = get_arg(args, "persistence", 2, "fbm");
            let lac = get_arg(args, "lacunarity", 3, "fbm");
            s.push_str(&format!(
                "{indent}let sdf_result = fbm2((p * {sc}), i32({oct}), {pers}, {lac});\n"
            ));
        }
        "grain" => {
            let amount = get_arg(args, "amount", 0, "grain");
            s.push_str(&format!("{indent}let grain_noise = fract(sin(dot(p, vec2<f32>(12.9898, 78.233)) + time) * 43758.5453);\n"));
            s.push_str(&format!("{indent}color_result = vec4<f32>(color_result.rgb + (grain_noise - 0.5) * {amount}, color_result.a);\n"));
        }
        "simplex" => {
            let sc = get_arg(args, "scale", 0, "simplex");
            s.push_str(&format!("{indent}let sdf_result = noise2(p * {sc});\n"));
        }
        "warp" => {
            let sc = get_arg(args, "scale", 0, "warp");
            let oct = get_arg(args, "octaves", 1, "warp");
            let pers = get_arg(args, "persistence", 2, "warp");
            let lac = get_arg(args, "lacunarity", 3, "warp");
            let str_ = get_arg(args, "strength", 4, "warp");
            s.push_str(&format!(
                "{indent}{{ let warp_x = fbm2(p * {sc} + vec2<f32>(0.0, 1.3), i32({oct}), {pers}, {lac});\n"
            ));
            s.push_str(&format!(
                "{indent}let warp_y = fbm2(p * {sc} + vec2<f32>(1.7, 0.0), i32({oct}), {pers}, {lac});\n"
            ));
            s.push_str(&format!(
                "{indent}p = p + vec2<f32>(warp_x, warp_y) * {str_}; }}\n"
            ));
        }
        "distort" => {
            let sc = get_arg(args, "scale", 0, "distort");
            let speed = get_arg(args, "speed", 1, "distort");
            let str_ = get_arg(args, "strength", 2, "distort");
            s.push_str(&format!(
                "{indent}p = p + vec2<f32>(sin(p.y * {sc} + time * {speed}), cos(p.x * {sc} + time * {speed})) * {str_};\n"
            ));
        }
        "polar" => {
            s.push_str(&format!(
                "{indent}p = vec2<f32>(length(p), atan2(p.y, p.x));\n"
            ));
        }
        "voronoi" => {
            let sc = get_arg(args, "scale", 0, "voronoi");
            s.push_str(&format!("{indent}let sdf_result = voronoi2(p * {sc});\n"));
        }
        "radial_fade" => {
            let inner = get_arg(args, "inner", 0, "radial_fade");
            let outer = get_arg(args, "outer", 1, "radial_fade");
            s.push_str(&format!(
                "{indent}let sdf_result = smoothstep({inner}, {outer}, length(p));\n"
            ));
        }
        "palette" => {
            let a_r = get_arg(args, "a_r", 0, "palette");
            let a_g = get_arg(args, "a_g", 1, "palette");
            let a_b = get_arg(args, "a_b", 2, "palette");
            let b_r = get_arg(args, "b_r", 3, "palette");
            let b_g = get_arg(args, "b_g", 4, "palette");
            let b_b = get_arg(args, "b_b", 5, "palette");
            let c_r = get_arg(args, "c_r", 6, "palette");
            let c_g = get_arg(args, "c_g", 7, "palette");
            let c_b = get_arg(args, "c_b", 8, "palette");
            let d_r = get_arg(args, "d_r", 9, "palette");
            let d_g = get_arg(args, "d_g", 10, "palette");
            let d_b = get_arg(args, "d_b", 11, "palette");
            s.push_str(&format!(
                "{indent}var color_result = vec4<f32>(cosine_palette(sdf_result, vec3<f32>({a_r}, {a_g}, {a_b}), vec3<f32>({b_r}, {b_g}, {b_b}), vec3<f32>({c_r}, {c_g}, {c_b}), vec3<f32>({d_r}, {d_g}, {d_b})), 1.0);\n"
            ));
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

/// Generate the standard WGSL vertex shader.
pub fn vertex_shader() -> &'static str {
    r#"struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var out: VertexOutput;
    out.pos = vec4<f32>(positions[vid], 0.0, 1.0);
    out.uv = positions[vid] * 0.5 + 0.5;
    return out;
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
            listen: None,
            voice: None,
            score: None,
            gravity: None,
            react: None,
            swarm: None,
            flow: None,
        }
    }

    #[test]
    fn basic_wgsl_output() {
        let cin = make_cinematic(vec![
            Stage {
                name: "circle".into(),
                args: vec![Arg {
                    name: None,
                    value: Expr::Number(0.2),
                }],
            },
            Stage {
                name: "glow".into(),
                args: vec![Arg {
                    name: None,
                    value: Expr::Number(1.5),
                }],
            },
            Stage {
                name: "tint".into(),
                args: vec![
                    Arg {
                        name: None,
                        value: Expr::Number(0.831),
                    },
                    Arg {
                        name: None,
                        value: Expr::Number(0.686),
                    },
                    Arg {
                        name: None,
                        value: Expr::Number(0.216),
                    },
                ],
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("fn fs_main"));
        assert!(output.contains("sdf_circle"));
        assert!(output.contains("apply_glow"));
        assert!(output.contains("color_result.rgb * vec3<f32>"));
        assert!(output.contains("return color_result"));
    }

    #[test]
    fn wgsl_vertex_shader_valid() {
        let vs = vertex_shader();
        assert!(vs.contains("fn vs_main"));
        assert!(vs.contains("@vertex"));
    }

    #[test]
    fn wgsl_warp_voronoi_palette_pipeline() {
        let cin = make_cinematic(vec![
            Stage {
                name: "warp".into(),
                args: vec![],
            },
            Stage {
                name: "voronoi".into(),
                args: vec![],
            },
            Stage {
                name: "palette".into(),
                args: vec![],
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("fn voronoi2"), "voronoi helper emitted");
        assert!(
            output.contains("fn cosine_palette"),
            "palette helper emitted"
        );
        assert!(output.contains("fn fbm2"), "fbm helper emitted for warp");
        assert!(output.contains("warp_x"), "warp displacement x");
        assert!(output.contains("voronoi2(p *"), "voronoi stage call");
        assert!(
            output.contains("cosine_palette(sdf_result"),
            "palette stage call"
        );
    }

    #[test]
    fn wgsl_distort_radial_fade_glow() {
        let cin = make_cinematic(vec![
            Stage {
                name: "distort".into(),
                args: vec![],
            },
            Stage {
                name: "radial_fade".into(),
                args: vec![],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("sin(p.y *"), "distort sin displacement");
        assert!(output.contains("cos(p.x *"), "distort cos displacement");
        assert!(output.contains("smoothstep("), "radial_fade smoothstep");
    }

    #[test]
    fn wgsl_polar_simplex() {
        let cin = make_cinematic(vec![
            Stage {
                name: "polar".into(),
                args: vec![],
            },
            Stage {
                name: "simplex".into(),
                args: vec![],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("atan2(p.y, p.x)"), "polar transform");
        assert!(output.contains("noise2(p *"), "simplex noise");
        assert!(output.contains("fn noise2"), "noise2 helper emitted");
    }
}
