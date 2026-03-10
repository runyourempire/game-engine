//! WGSL shader generation from GAME AST.

use crate::ast::*;
use crate::codegen::memory;
use crate::codegen::stages::get_arg;
use crate::codegen::UniformInfo;

/// Named color preset definitions (a, b, c, d vectors for cosine palette).
fn named_palette(name: &str) -> Option<(&str, &str, &str, &str)> {
    match name {
        "fire" => Some((
            "vec3<f32>(0.5, 0.3, 0.1)",
            "vec3<f32>(0.5, 0.2, 0.1)",
            "vec3<f32>(1.0, 1.0, 1.0)",
            "vec3<f32>(0.0, 0.25, 0.25)",
        )),
        "ocean" => Some((
            "vec3<f32>(0.0, 0.3, 0.5)",
            "vec3<f32>(0.0, 0.3, 0.5)",
            "vec3<f32>(1.0, 1.0, 1.0)",
            "vec3<f32>(0.0, 0.1, 0.2)",
        )),
        "neon" => Some((
            "vec3<f32>(0.5, 0.5, 0.5)",
            "vec3<f32>(0.5, 0.5, 0.5)",
            "vec3<f32>(1.0, 1.0, 1.0)",
            "vec3<f32>(0.0, 0.33, 0.67)",
        )),
        "aurora" => Some((
            "vec3<f32>(0.0, 0.5, 0.3)",
            "vec3<f32>(0.2, 0.5, 0.4)",
            "vec3<f32>(1.0, 1.0, 1.0)",
            "vec3<f32>(0.0, 0.1, 0.3)",
        )),
        "sunset" => Some((
            "vec3<f32>(0.5, 0.3, 0.2)",
            "vec3<f32>(0.5, 0.2, 0.3)",
            "vec3<f32>(1.0, 1.0, 0.5)",
            "vec3<f32>(0.8, 0.9, 0.3)",
        )),
        "ice" => Some((
            "vec3<f32>(0.5, 0.7, 0.9)",
            "vec3<f32>(0.2, 0.2, 0.1)",
            "vec3<f32>(1.0, 1.0, 1.0)",
            "vec3<f32>(0.0, 0.05, 0.15)",
        )),
        "ember" => Some((
            "vec3<f32>(0.6, 0.2, 0.05)",
            "vec3<f32>(0.4, 0.2, 0.1)",
            "vec3<f32>(1.0, 0.5, 0.5)",
            "vec3<f32>(0.0, 0.15, 0.2)",
        )),
        "lava" => Some((
            "vec3<f32>(0.5, 0.2, 0.0)",
            "vec3<f32>(0.5, 0.3, 0.1)",
            "vec3<f32>(0.8, 0.5, 0.5)",
            "vec3<f32>(0.0, 0.2, 0.3)",
        )),
        "magma" => Some((
            "vec3<f32>(0.55, 0.2, 0.08)",
            "vec3<f32>(0.45, 0.25, 0.1)",
            "vec3<f32>(1.0, 0.7, 0.4)",
            "vec3<f32>(0.0, 0.15, 0.25)",
        )),
        "inferno" => Some((
            "vec3<f32>(0.5, 0.3, 0.15)",
            "vec3<f32>(0.5, 0.35, 0.2)",
            "vec3<f32>(1.0, 1.0, 0.7)",
            "vec3<f32>(0.0, 0.15, 0.3)",
        )),
        "plasma" => Some((
            "vec3<f32>(0.5, 0.2, 0.5)",
            "vec3<f32>(0.5, 0.3, 0.5)",
            "vec3<f32>(1.0, 1.0, 1.0)",
            "vec3<f32>(0.15, 0.0, 0.5)",
        )),
        "electric" => Some((
            "vec3<f32>(0.1, 0.4, 0.8)",
            "vec3<f32>(0.3, 0.4, 0.2)",
            "vec3<f32>(1.0, 1.0, 1.0)",
            "vec3<f32>(0.0, 0.1, 0.3)",
        )),
        "cyber" => Some((
            "vec3<f32>(0.0, 0.5, 0.3)",
            "vec3<f32>(0.1, 0.5, 0.4)",
            "vec3<f32>(1.0, 1.0, 0.5)",
            "vec3<f32>(0.0, 0.2, 0.5)",
        )),
        "matrix" => Some((
            "vec3<f32>(0.0, 0.3, 0.0)",
            "vec3<f32>(0.0, 0.5, 0.0)",
            "vec3<f32>(0.0, 1.0, 0.0)",
            "vec3<f32>(0.0, 0.2, 0.0)",
        )),
        "forest" => Some((
            "vec3<f32>(0.2, 0.35, 0.1)",
            "vec3<f32>(0.15, 0.25, 0.1)",
            "vec3<f32>(0.8, 1.0, 0.5)",
            "vec3<f32>(0.0, 0.2, 0.4)",
        )),
        "moss" => Some((
            "vec3<f32>(0.25, 0.3, 0.15)",
            "vec3<f32>(0.15, 0.2, 0.1)",
            "vec3<f32>(0.7, 0.8, 0.5)",
            "vec3<f32>(0.1, 0.2, 0.3)",
        )),
        "earth" => Some((
            "vec3<f32>(0.4, 0.3, 0.2)",
            "vec3<f32>(0.2, 0.15, 0.1)",
            "vec3<f32>(0.8, 0.7, 0.5)",
            "vec3<f32>(0.0, 0.1, 0.2)",
        )),
        "desert" => Some((
            "vec3<f32>(0.6, 0.4, 0.25)",
            "vec3<f32>(0.3, 0.2, 0.15)",
            "vec3<f32>(0.7, 0.5, 0.4)",
            "vec3<f32>(0.0, 0.1, 0.2)",
        )),
        "blood" => Some((
            "vec3<f32>(0.4, 0.05, 0.05)",
            "vec3<f32>(0.4, 0.1, 0.05)",
            "vec3<f32>(1.0, 0.5, 0.5)",
            "vec3<f32>(0.0, 0.15, 0.3)",
        )),
        "rose" => Some((
            "vec3<f32>(0.6, 0.3, 0.4)",
            "vec3<f32>(0.3, 0.2, 0.3)",
            "vec3<f32>(1.0, 0.8, 1.0)",
            "vec3<f32>(0.0, 0.1, 0.3)",
        )),
        "candy" => Some((
            "vec3<f32>(0.6, 0.3, 0.6)",
            "vec3<f32>(0.4, 0.3, 0.4)",
            "vec3<f32>(1.0, 1.0, 1.0)",
            "vec3<f32>(0.0, 0.33, 0.67)",
        )),
        "royal" => Some((
            "vec3<f32>(0.3, 0.1, 0.5)",
            "vec3<f32>(0.3, 0.2, 0.3)",
            "vec3<f32>(0.8, 0.5, 1.0)",
            "vec3<f32>(0.2, 0.0, 0.3)",
        )),
        "deep_sea" => Some((
            "vec3<f32>(0.0, 0.1, 0.3)",
            "vec3<f32>(0.0, 0.2, 0.3)",
            "vec3<f32>(0.5, 0.8, 1.0)",
            "vec3<f32>(0.0, 0.1, 0.2)",
        )),
        "coral" => Some((
            "vec3<f32>(0.6, 0.35, 0.3)",
            "vec3<f32>(0.3, 0.25, 0.2)",
            "vec3<f32>(0.8, 0.7, 0.8)",
            "vec3<f32>(0.0, 0.1, 0.25)",
        )),
        "arctic" => Some((
            "vec3<f32>(0.7, 0.8, 0.95)",
            "vec3<f32>(0.2, 0.15, 0.1)",
            "vec3<f32>(1.0, 1.0, 1.0)",
            "vec3<f32>(0.0, 0.05, 0.1)",
        )),
        "twilight" => Some((
            "vec3<f32>(0.4, 0.2, 0.5)",
            "vec3<f32>(0.3, 0.3, 0.3)",
            "vec3<f32>(1.0, 0.8, 0.5)",
            "vec3<f32>(0.3, 0.1, 0.0)",
        )),
        "vapor" => Some((
            "vec3<f32>(0.5, 0.3, 0.6)",
            "vec3<f32>(0.5, 0.3, 0.4)",
            "vec3<f32>(1.0, 1.0, 0.8)",
            "vec3<f32>(0.3, 0.2, 0.0)",
        )),
        "gold" => Some((
            "vec3<f32>(0.55, 0.42, 0.15)",
            "vec3<f32>(0.3, 0.25, 0.1)",
            "vec3<f32>(0.8, 0.6, 0.4)",
            "vec3<f32>(0.0, 0.1, 0.2)",
        )),
        "silver" => Some((
            "vec3<f32>(0.5, 0.5, 0.55)",
            "vec3<f32>(0.2, 0.2, 0.2)",
            "vec3<f32>(1.0, 1.0, 1.0)",
            "vec3<f32>(0.0, 0.0, 0.1)",
        )),
        "monochrome" => Some((
            "vec3<f32>(0.5, 0.5, 0.5)",
            "vec3<f32>(0.3, 0.3, 0.3)",
            "vec3<f32>(1.0, 1.0, 1.0)",
            "vec3<f32>(0.0, 0.0, 0.0)",
        )),
        _ => None,
    }
}

/// Generate a WGSL fragment shader for a cinematic with user-defined functions.
pub fn generate_fragment_with_fns(
    cinematic: &Cinematic,
    uniforms: &[UniformInfo],
    fns: &[FnDef],
) -> String {
    generate_fragment_inner(cinematic, uniforms, fns)
}

/// Generate a WGSL fragment shader for a cinematic.
pub fn generate_fragment(cinematic: &Cinematic, uniforms: &[UniformInfo]) -> String {
    generate_fragment_inner(cinematic, uniforms, &[])
}

/// Generate a WGSL post-processing pass fragment shader.
///
/// A pass reads from a texture (previous pass output) and writes a processed result.
/// The pass pipeline operates on UV-sampled color values.
pub fn generate_pass_fragment(pass: &PassBlock) -> String {
    let mut s = String::with_capacity(2048);

    s.push_str("// Post-processing pass: ");
    s.push_str(&pass.name);
    s.push_str("\n\n");

    // Struct definitions (must be self-contained — each shader module needs its own)
    s.push_str("struct Uniforms {\n");
    s.push_str("    time: f32,\n");
    s.push_str("    audio_bass: f32,\n");
    s.push_str("    audio_mid: f32,\n");
    s.push_str("    audio_treble: f32,\n");
    s.push_str("    audio_energy: f32,\n");
    s.push_str("    audio_beat: f32,\n");
    s.push_str("    resolution: vec2<f32>,\n");
    s.push_str("    mouse: vec2<f32>,\n");
    s.push_str("};\n\n");
    s.push_str("struct VertexOutput {\n");
    s.push_str("    @builtin(position) pos: vec4<f32>,\n");
    s.push_str("    @location(0) uv: vec2<f32>,\n");
    s.push_str("};\n\n");

    // Bindings: uniforms + input texture
    s.push_str("@group(0) @binding(0) var<uniform> u: Uniforms;\n");
    s.push_str("@group(0) @binding(3) var pass_tex: texture_2d<f32>;\n");
    s.push_str("@group(0) @binding(4) var pass_sampler: sampler;\n\n");

    s.push_str("@fragment\n");
    s.push_str("fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {\n");
    s.push_str("    let uv = input.uv;\n");
    s.push_str("    let pixel = textureSample(pass_tex, pass_sampler, uv);\n");
    s.push_str("    var color_result = pixel;\n\n");

    // Emit pass pipeline stages (operate on color_result)
    for stage in &pass.body {
        emit_pass_stage(&mut s, stage, "    ");
    }

    s.push_str("    return color_result;\n");
    s.push_str("}\n");

    s
}

/// Emit a post-processing stage operating on `color_result` and `pixel`.
fn emit_pass_stage(s: &mut String, stage: &Stage, indent: &str) {
    let args = &stage.args;
    match stage.name.as_str() {
        "blur" | "gaussian_blur" => {
            // Simple box blur approximation
            let radius = if !args.is_empty() {
                get_arg(args, "radius", 0, "blur")
            } else {
                "2.0".to_string()
            };
            s.push_str(&format!("{indent}// blur pass\n"));
            s.push_str(&format!("{indent}var blurred = vec4<f32>(0.0);\n"));
            s.push_str(&format!("{indent}let texel = 1.0 / u.resolution;\n"));
            s.push_str(&format!("{indent}let r = i32({radius});\n"));
            s.push_str(&format!("{indent}var count = 0.0;\n"));
            s.push_str(&format!("{indent}for (var dy = -r; dy <= r; dy++) {{\n"));
            s.push_str(&format!(
                "{indent}    for (var dx = -r; dx <= r; dx++) {{\n"
            ));
            s.push_str(&format!(
                "{indent}        let offset = vec2<f32>(f32(dx), f32(dy)) * texel;\n"
            ));
            s.push_str(&format!(
                "{indent}        blurred += textureSample(pass_tex, pass_sampler, uv + offset);\n"
            ));
            s.push_str(&format!("{indent}        count += 1.0;\n"));
            s.push_str(&format!("{indent}    }}\n"));
            s.push_str(&format!("{indent}}}\n"));
            s.push_str(&format!("{indent}color_result = blurred / count;\n"));
        }
        "threshold" => {
            let t = if !args.is_empty() {
                get_arg(args, "value", 0, "threshold")
            } else {
                "0.5".to_string()
            };
            s.push_str(&format!(
                "{indent}let lum = dot(color_result.rgb, vec3<f32>(0.299, 0.587, 0.114));\n"
            ));
            s.push_str(&format!(
                "{indent}color_result = select(vec4<f32>(0.0, 0.0, 0.0, 0.0), color_result, lum > {t});\n"
            ));
        }
        "invert" => {
            s.push_str(&format!(
                "{indent}color_result = vec4<f32>(1.0 - color_result.rgb, color_result.a);\n"
            ));
        }
        "blend_add" => {
            s.push_str(&format!(
                "{indent}color_result = vec4<f32>(min(pixel.rgb + color_result.rgb, vec3<f32>(1.0)), max(pixel.a, color_result.a));\n"
            ));
        }
        "vignette" => {
            let strength = if !args.is_empty() {
                get_arg(args, "strength", 0, "vignette")
            } else {
                "0.5".to_string()
            };
            s.push_str(&format!(
                "{indent}let vign = 1.0 - {strength} * length(uv - 0.5);\n"
            ));
            s.push_str(&format!(
                "{indent}color_result = vec4<f32>(color_result.rgb * vign, color_result.a * vign);\n"
            ));
        }
        _ => {
            // Unknown pass stage — passthrough
            s.push_str(&format!("{indent}// unknown pass stage: {}\n", stage.name));
        }
    }
}

fn generate_fragment_inner(
    cinematic: &Cinematic,
    uniforms: &[UniformInfo],
    fns: &[FnDef],
) -> String {
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

    // Color matrix function (if present)
    if let Some(ref mc) = cinematic.matrix_color {
        s.push_str(&super::matrix::generate_color_matrix_wgsl(mc));
        s.push('\n');
    }

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

    let render_layers = cinematic
        .layers
        .iter()
        .filter(|l| !matches!(l.body, LayerBody::Params(_)))
        .count();
    let multi_layer = render_layers > 1;
    if multi_layer {
        s.push_str("    var final_color = vec4<f32>(0.0, 0.0, 0.0, 0.0);\n\n");
    }

    for (i, layer) in cinematic.layers.iter().enumerate() {
        if matches!(layer.body, LayerBody::Params(_)) {
            continue;
        }
        emit_wgsl_layer(
            &mut s,
            layer,
            i,
            multi_layer,
            fns,
            cinematic.matrix_color.is_some(),
        );
    }

    if multi_layer {
        if cinematic.matrix_color.is_some() {
            s.push_str("    final_color = vec4<f32>(apply_color_matrix(final_color.rgb), final_color.a);\n");
        }
        s.push_str("    return final_color;\n");
    }
    s.push_str("}\n");
    s
}

fn emit_wgsl_builtins(s: &mut String, cinematic: &Cinematic) {
    let needs_circle = cinematic.layers.iter().any(|l| has_stage(l, "circle"));
    let needs_star = cinematic.layers.iter().any(|l| has_stage(l, "star"));
    let needs_box = cinematic
        .layers
        .iter()
        .any(|l| has_stage(l, "box") || has_stage(l, "cross"));
    let needs_hex = cinematic.layers.iter().any(|l| has_stage(l, "hex"));
    let needs_fbm = cinematic.layers.iter().any(|l| has_stage(l, "fbm"));
    let needs_warp = cinematic.layers.iter().any(|l| has_stage(l, "warp"));
    let needs_simplex = cinematic.layers.iter().any(|l| has_stage(l, "simplex"));
    let needs_voronoi = cinematic.layers.iter().any(|l| has_stage(l, "voronoi"));
    let needs_palette = cinematic.layers.iter().any(|l| has_stage(l, "palette"));
    let needs_smin = cinematic.layers.iter().any(|l| {
        has_stage(l, "smooth_union")
            || has_stage(l, "smooth_subtract")
            || has_stage(l, "smooth_intersect")
    });
    let needs_repeat = cinematic.layers.iter().any(|l| has_stage(l, "repeat"));
    let needs_radial = cinematic.layers.iter().any(|l| has_stage(l, "radial"));
    let needs_grid = cinematic.layers.iter().any(|l| has_stage(l, "grid"));
    let needs_line = cinematic.layers.iter().any(|l| has_stage(l, "line"));
    let needs_capsule = cinematic.layers.iter().any(|l| has_stage(l, "capsule"));
    let needs_triangle = cinematic.layers.iter().any(|l| has_stage(l, "triangle"));
    let needs_arc_sdf = cinematic.layers.iter().any(|l| has_stage(l, "arc_sdf"));
    let needs_heart = cinematic.layers.iter().any(|l| has_stage(l, "heart"));
    let needs_egg = cinematic.layers.iter().any(|l| has_stage(l, "egg"));

    if needs_circle {
        s.push_str("fn sdf_circle(p: vec2<f32>, radius: f32) -> f32 {\n");
        s.push_str("    return length(p) - radius;\n");
        s.push_str("}\n\n");
    }

    if needs_star {
        emit_wgsl_star(s);
    }

    if needs_box {
        s.push_str("fn sdf_box(p: vec2<f32>, w: f32, h: f32) -> f32 {\n");
        s.push_str("    let d = abs(p) - vec2<f32>(w, h);\n");
        s.push_str("    return length(max(d, vec2<f32>(0.0, 0.0))) + min(max(d.x, d.y), 0.0);\n");
        s.push_str("}\n\n");
    }

    if needs_hex {
        s.push_str("fn sdf_hex(p: vec2<f32>, r: f32) -> f32 {\n");
        s.push_str("    let k = vec3<f32>(-0.866025, 0.5, 0.577350);\n");
        s.push_str("    var q = abs(p);\n");
        s.push_str("    q = q - 2.0 * min(dot(k.xy, q), 0.0) * k.xy;\n");
        s.push_str("    q = q - vec2<f32>(clamp(q.x, -k.z * r, k.z * r), r);\n");
        s.push_str("    return length(q) * sign(q.y);\n");
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

    // Smooth min for smooth boolean operations
    if needs_smin {
        s.push_str("fn smin(a: f32, b: f32, k: f32) -> f32 {\n");
        s.push_str("    let h = max(k - abs(a - b), 0.0) / k;\n");
        s.push_str("    return min(a, b) - h * h * k * 0.25;\n");
        s.push_str("}\n\n");
    }

    // Floor-based mod for spatial repeat (WGSL % is trunc-based!)
    if needs_repeat || needs_radial || needs_grid {
        s.push_str("fn game_mod(x: f32, y: f32) -> f32 {\n");
        s.push_str("    return x - y * floor(x / y);\n");
        s.push_str("}\n\n");
    }

    // New SDF primitive helpers
    if needs_line || needs_capsule {
        s.push_str("fn sdf_line(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {\n");
        s.push_str("    let pa = p - a;\n");
        s.push_str("    let ba = b - a;\n");
        s.push_str("    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);\n");
        s.push_str("    return length(pa - ba * h);\n");
        s.push_str("}\n\n");
    }

    if needs_triangle {
        s.push_str("fn sdf_triangle(p: vec2<f32>, sz: f32) -> f32 {\n");
        s.push_str("    let k = sqrt(3.0);\n");
        s.push_str("    var q = vec2<f32>(abs(p.x) - sz, p.y + sz / k);\n");
        s.push_str("    if (q.x + k * q.y > 0.0) { q = vec2<f32>(q.x - k * q.y, -k * q.x - q.y) / 2.0; }\n");
        s.push_str("    q = vec2<f32>(q.x - clamp(q.x, -2.0 * sz, 0.0), q.y);\n");
        s.push_str("    return -length(q) * sign(q.y);\n");
        s.push_str("}\n\n");
    }

    if needs_arc_sdf {
        s.push_str("fn sdf_arc(p: vec2<f32>, ra: f32, angle: f32, rb: f32) -> f32 {\n");
        s.push_str("    let sca = vec2<f32>(sin(angle), cos(angle));\n");
        s.push_str("    var q = vec2<f32>(abs(p.x), p.y);\n");
        s.push_str("    let k = select(length(q), dot(q, sca), sca.y * q.x > sca.x * q.y);\n");
        s.push_str("    return sqrt(dot(q, q) + ra * ra - 2.0 * ra * k) - rb;\n");
        s.push_str("}\n\n");
    }

    if needs_heart {
        s.push_str("fn sdf_heart(p: vec2<f32>, sz: f32) -> f32 {\n");
        s.push_str("    let q = vec2<f32>(abs(p.x), p.y);\n");
        s.push_str("    let b = vec2<f32>(sz * 0.5, sz * 0.8);\n");
        s.push_str("    let r = 0.5 * (b.x + b.y);\n");
        s.push_str("    let d = length(q - vec2<f32>(0.0, r * 0.5)) - r;\n");
        s.push_str("    let a = atan2(q.x, q.y - r * 0.5);\n");
        s.push_str("    let h = sz * (0.5 + 0.3 * cos(a));\n");
        s.push_str("    return length(q - vec2<f32>(0.0, r * 0.5)) - h;\n");
        s.push_str("}\n\n");
    }

    if needs_egg {
        s.push_str("fn sdf_egg(p: vec2<f32>, ra: f32, rb: f32) -> f32 {\n");
        s.push_str("    let q = vec2<f32>(abs(p.x), p.y);\n");
        s.push_str("    let r = ra - rb;\n");
        s.push_str("    let k = select(length(q), length(q + vec2<f32>(0.0, rb)), q.y < 0.0);\n");
        s.push_str("    return k - ra;\n");
        s.push_str("}\n\n");
    }
}

fn emit_wgsl_star(s: &mut String) {
    s.push_str("fn sdf_star(p: vec2<f32>, n: f32, r: f32, ir: f32) -> f32 {\n");
    s.push_str("    let an = 3.14159265 / n;\n");
    s.push_str("    let a = atan2(p.y, p.x);\n");
    s.push_str("    let period = 2.0 * an;\n");
    s.push_str("    let sa = (a + an) - floor((a + an) / period) * period - an;\n");
    s.push_str("    let q = length(p) * vec2<f32>(cos(sa), abs(sin(sa)));\n");
    s.push_str("    let tip = vec2<f32>(r, 0.0);\n");
    s.push_str("    let valley = vec2<f32>(ir * cos(an), ir * sin(an));\n");
    s.push_str("    let e = tip - valley;\n");
    s.push_str("    let d = q - valley;\n");
    s.push_str("    let t = clamp(dot(d, e) / dot(e, e), 0.0, 1.0);\n");
    s.push_str("    let closest = valley + e * t;\n");
    s.push_str("    let dist = length(q - closest);\n");
    s.push_str("    let cross_val = d.x * e.y - d.y * e.x;\n");
    s.push_str("    return select(dist, -dist, cross_val > 0.0);\n");
    s.push_str("}\n\n");
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

fn emit_wgsl_layer(
    s: &mut String,
    layer: &Layer,
    idx: usize,
    multi: bool,
    fns: &[FnDef],
    has_color_matrix: bool,
) {
    s.push_str(&format!("    // ── Layer {idx}: {} ──\n", layer.name));
    if multi {
        s.push_str("    {\n");
    }
    let indent = if multi { "        " } else { "    " };

    s.push_str(&format!(
        "{indent}var p = vec2<f32>(uv.x * aspect, uv.y);\n"
    ));

    match &layer.body {
        LayerBody::Pipeline(stages) => {
            for stage in stages {
                emit_wgsl_stage_with_fns(s, stage, indent, fns);
            }
        }
        LayerBody::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            // Emit both branches, then select
            s.push_str(&format!("{indent}var color_result: vec4<f32>;\n"));
            s.push_str(&format!("{indent}{{\n"));
            let inner = &format!("{indent}    ");
            s.push_str(&format!("{inner}var p_then = p;\n"));
            s.push_str(&format!("{inner}var then_color: vec4<f32>;\n"));
            s.push_str(&format!("{inner}var else_color: vec4<f32>;\n"));
            // We use a fresh `p` for the then branch
            s.push_str(&format!("{inner}{{ var p = p_then;\n"));
            for stage in then_branch {
                emit_wgsl_stage_with_fns(s, stage, inner, fns);
            }
            s.push_str(&format!("{inner}then_color = color_result; }}\n"));
            // Else branch
            s.push_str(&format!("{inner}{{ var p = p_then;\n"));
            for stage in else_branch {
                emit_wgsl_stage_with_fns(s, stage, inner, fns);
            }
            s.push_str(&format!("{inner}else_color = color_result; }}\n"));
            // Conditional select
            let cond_str = emit_wgsl_expr(condition);
            s.push_str(&format!(
                "{inner}color_result = select(else_color, then_color, {cond_str});\n"
            ));
            s.push_str(&format!("{indent}}}\n"));
        }
        LayerBody::Params(_) => return,
    };

    // Memory: mix with previous frame if this layer has memory
    if let Some(decay) = layer.memory {
        memory::emit_wgsl_memory_mix(s, decay, indent);
    }

    if multi {
        // Apply opacity if specified
        if let Some(opacity) = layer.opacity {
            s.push_str(&format!(
                "{indent}let la = color_result.a * {opacity:.6};\n"
            ));
            s.push_str(&format!(
                "{indent}let lc = color_result.rgb * {opacity:.6};\n"
            ));
        } else {
            s.push_str(&format!("{indent}let la = color_result.a;\n"));
            s.push_str(&format!("{indent}let lc = color_result.rgb;\n"));
        }
        match layer.blend {
            BlendMode::Add => {
                // Premultiplied alpha "over" compositing: src + dst * (1 - srcAlpha)
                s.push_str(&format!(
                    "{indent}final_color = vec4<f32>(final_color.rgb * (1.0 - la) + lc, final_color.a * (1.0 - la) + la);\n"
                ));
            }
            BlendMode::Screen => {
                s.push_str(&format!(
                    "{indent}final_color = vec4<f32>(1.0 - (1.0 - final_color.rgb) * (1.0 - lc), max(final_color.a, la));\n"
                ));
            }
            BlendMode::Multiply => {
                s.push_str(&format!(
                    "{indent}final_color = vec4<f32>(final_color.rgb * lc, max(final_color.a, la));\n"
                ));
            }
            BlendMode::Overlay => {
                s.push_str(&format!("{indent}{{ let base = final_color.rgb;\n"));
                s.push_str(&format!("{indent}let lo = 2.0 * base * lc;\n"));
                s.push_str(&format!(
                    "{indent}let hi = 1.0 - 2.0 * (1.0 - base) * (1.0 - lc);\n"
                ));
                s.push_str(&format!(
                    "{indent}final_color = vec4<f32>(select(hi, lo, base < vec3<f32>(0.5)), max(final_color.a, la)); }}\n"
                ));
            }
        }
        s.push_str("    }\n\n");
    } else {
        if has_color_matrix {
            s.push_str(&format!("{indent}color_result = vec4<f32>(apply_color_matrix(color_result.rgb), color_result.a);\n"));
        }
        s.push_str(&format!("{indent}return color_result;\n"));
    }
}

/// Emit a WGSL expression string from an AST Expr.
pub fn emit_wgsl_expr(expr: &Expr) -> String {
    match expr {
        Expr::Number(v) => format!("{v:.6}"),
        Expr::Ident(name) => match name.as_str() {
            "time" => "time".to_string(),
            "bass" => "u.audio_bass".to_string(),
            "mid" => "u.audio_mid".to_string(),
            "treble" => "u.audio_treble".to_string(),
            "energy" => "u.audio_energy".to_string(),
            "beat" => "u.audio_beat".to_string(),
            _ => name.clone(),
        },
        Expr::DottedIdent { object, field } => {
            let obj = match object.as_str() {
                "audio" => "u.audio_",
                _ => return format!("{object}_{field}"),
            };
            format!("{obj}{field}")
        }
        Expr::BinOp { op, left, right } => {
            let l = emit_wgsl_expr(left);
            let r = emit_wgsl_expr(right);
            let op_str = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Pow => return format!("pow({l}, {r})"),
                BinOp::Gt => ">",
                BinOp::Lt => "<",
                BinOp::Gte => ">=",
                BinOp::Lte => "<=",
                BinOp::Eq => "==",
                BinOp::NotEq => "!=",
            };
            format!("({l} {op_str} {r})")
        }
        Expr::Neg(inner) => {
            let i = emit_wgsl_expr(inner);
            format!("(-{i})")
        }
        Expr::Paren(inner) => {
            let i = emit_wgsl_expr(inner);
            format!("({i})")
        }
        Expr::Call { name, args } => {
            let arg_strs: Vec<String> = args.iter().map(|a| emit_wgsl_expr(&a.value)).collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
        _ => "0.0".to_string(),
    }
}

/// Emit a stage with fn-inlining support.
fn emit_wgsl_stage_with_fns(s: &mut String, stage: &Stage, indent: &str, fns: &[FnDef]) {
    // Check if this is a user-defined fn call
    if let Some(fn_def) = fns.iter().find(|f| f.name == stage.name) {
        // Inline the fn body with argument substitution
        for fn_stage in &fn_def.body {
            let substituted = substitute_fn_args(fn_stage, &fn_def.params, &stage.args);
            emit_wgsl_stage(s, &substituted, indent);
        }
        return;
    }
    emit_wgsl_stage(s, stage, indent);
}

/// Substitute fn param names with caller's arg values in a stage.
pub fn substitute_fn_args(stage: &Stage, params: &[String], caller_args: &[Arg]) -> Stage {
    Stage {
        name: stage.name.clone(),
        args: stage
            .args
            .iter()
            .map(|arg| Arg {
                name: arg.name.clone(),
                value: substitute_expr(&arg.value, params, caller_args),
            })
            .collect(),
    }
}

fn substitute_expr(expr: &Expr, params: &[String], caller_args: &[Arg]) -> Expr {
    match expr {
        Expr::Ident(name) => {
            if let Some(idx) = params.iter().position(|p| p == name) {
                if let Some(arg) = caller_args.get(idx) {
                    return arg.value.clone();
                }
            }
            expr.clone()
        }
        Expr::BinOp { op, left, right } => Expr::BinOp {
            op: op.clone(),
            left: Box::new(substitute_expr(left, params, caller_args)),
            right: Box::new(substitute_expr(right, params, caller_args)),
        },
        Expr::Neg(inner) => Expr::Neg(Box::new(substitute_expr(inner, params, caller_args))),
        Expr::Paren(inner) => Expr::Paren(Box::new(substitute_expr(inner, params, caller_args))),
        Expr::Call { name, args } => Expr::Call {
            name: name.clone(),
            args: args
                .iter()
                .map(|a| Arg {
                    name: a.name.clone(),
                    value: substitute_expr(&a.value, params, caller_args),
                })
                .collect(),
        },
        _ => expr.clone(),
    }
}

fn emit_wgsl_stage(s: &mut String, stage: &Stage, indent: &str) {
    let args = &stage.args;
    match stage.name.as_str() {
        "circle" => {
            let r = get_arg(args, "radius", 0, "circle");
            s.push_str(&format!("{indent}var sdf_result = sdf_circle(p, {r});\n"));
        }
        "ring" => {
            let r = get_arg(args, "radius", 0, "ring");
            let w = get_arg(args, "width", 1, "ring");
            s.push_str(&format!(
                "{indent}var sdf_result = abs(length(p) - {r}) - {w};\n"
            ));
        }
        "star" => {
            let n = get_arg(args, "points", 0, "star");
            let r = get_arg(args, "radius", 1, "star");
            let ir = get_arg(args, "inner", 2, "star");
            s.push_str(&format!(
                "{indent}var sdf_result = sdf_star(p, {n}, {r}, {ir});\n"
            ));
        }
        "box" => {
            let w = get_arg(args, "width", 0, "box");
            let h = get_arg(args, "height", 1, "box");
            s.push_str(&format!("{indent}var sdf_result = sdf_box(p, {w}, {h});\n"));
        }
        "hex" => {
            let r = get_arg(args, "radius", 0, "hex");
            s.push_str(&format!("{indent}var sdf_result = sdf_hex(p, {r});\n"));
        }
        "glow" => {
            let intensity = get_arg(args, "intensity", 0, "glow");
            s.push_str(&format!(
                "{indent}let glow_pulse = {intensity} * (0.9 + 0.1 * sin(time * 2.0));\n"
            ));
            s.push_str(&format!(
                "{indent}let glow_result = apply_glow(sdf_result, glow_pulse);\n"
            ));
            s.push_str(&format!(
                "{indent}var color_result = vec4<f32>(vec3<f32>(glow_result), glow_result);\n"
            ));
        }
        "tint" => {
            let r = get_arg(args, "r", 0, "tint");
            let g = get_arg(args, "g", 1, "tint");
            let b = get_arg(args, "b", 2, "tint");
            s.push_str(&format!("{indent}color_result = vec4<f32>(color_result.rgb * vec3<f32>({r}, {g}, {b}), color_result.a);\n"));
        }
        "bloom" => {
            let thresh = get_arg(args, "threshold", 0, "bloom");
            let strength = get_arg(args, "strength", 1, "bloom");
            s.push_str(&format!(
                "{indent}let pp_lum = dot(color_result.rgb, vec3<f32>(0.299, 0.587, 0.114));\n"
            ));
            s.push_str(&format!("{indent}color_result = vec4<f32>(color_result.rgb + max(pp_lum - {thresh}, 0.0) * {strength}, color_result.a);\n"));
        }
        "rotate" => {
            let speed = get_arg(args, "speed", 0, "rotate");
            s.push_str(&format!(
                "{indent}{{ let ra = time * {speed}; let rc = cos(ra); let rs = sin(ra);\n"
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
            s.push_str(&format!("{indent}var color_result = vec4<f32>(vec3<f32>({r}, {g}, {b}) * (1.0 - clamp(sdf_result, 0.0, 1.0)), 1.0 - clamp(sdf_result, 0.0, 1.0));\n"));
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
                "{indent}var sdf_result = fbm2((p * {sc} + vec2<f32>(time * 0.1, time * 0.07)), i32({oct}), {pers}, {lac});\n"
            ));
        }
        "grain" => {
            let amount = get_arg(args, "amount", 0, "grain");
            s.push_str(&format!("{indent}let grain_noise = fract(sin(dot(p, vec2<f32>(12.9898, 78.233)) + time) * 43758.5453);\n"));
            s.push_str(&format!("{indent}color_result = vec4<f32>(color_result.rgb + (grain_noise - 0.5) * {amount}, color_result.a);\n"));
        }
        "simplex" => {
            let sc = get_arg(args, "scale", 0, "simplex");
            s.push_str(&format!(
                "{indent}var sdf_result = noise2(p * {sc} + vec2<f32>(time * 0.1, time * 0.07));\n"
            ));
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
            s.push_str(&format!("{indent}var sdf_result = voronoi2(p * {sc} + vec2<f32>(time * 0.05, time * 0.03));\n"));
        }
        "radial_fade" => {
            let inner = get_arg(args, "inner", 0, "radial_fade");
            let outer = get_arg(args, "outer", 1, "radial_fade");
            s.push_str(&format!(
                "{indent}let sdf_result = smoothstep({inner}, {outer}, length(p));\n"
            ));
        }
        "palette" => {
            // Check for named preset: palette(fire), palette(ocean), etc.
            let preset = args.first().and_then(|a| {
                if let Expr::Ident(name) = &a.value {
                    named_palette(name)
                } else {
                    None
                }
            });
            if let Some((a, b, c, d)) = preset {
                s.push_str(&format!(
                    "{indent}let pal_rgb = cosine_palette(sdf_result, {a}, {b}, {c}, {d});\n{indent}var color_result = vec4<f32>(pal_rgb, clamp(dot(pal_rgb, vec3<f32>(0.299, 0.587, 0.114)) * 2.0, 0.0, 1.0));\n"
                ));
            } else {
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
                    "{indent}let pal_rgb = cosine_palette(sdf_result, vec3<f32>({a_r}, {a_g}, {a_b}), vec3<f32>({b_r}, {b_g}, {b_b}), vec3<f32>({c_r}, {c_g}, {c_b}), vec3<f32>({d_r}, {d_g}, {d_b}));\n{indent}var color_result = vec4<f32>(pal_rgb, clamp(dot(pal_rgb, vec3<f32>(0.299, 0.587, 0.114)) * 2.0, 0.0, 1.0));\n"
                ));
            }
        }
        // ── SDF Morph ────────────────────────────────────
        "morph" => {
            emit_wgsl_morph(s, stage, indent);
        }
        // ── SDF Boolean operations ──────────────────────
        "union" | "subtract" | "intersect" | "xor" => {
            emit_wgsl_bool_op(s, stage, indent);
        }
        "smooth_union" | "smooth_subtract" | "smooth_intersect" => {
            emit_wgsl_smooth_bool_op(s, stage, indent);
        }
        // ── Spatial operations ──────────────────────────
        "repeat" => {
            let sx = get_arg(args, "spacing_x", 0, "repeat");
            let sy = get_arg(args, "spacing_y", 1, "repeat");
            s.push_str(&format!(
                "{indent}p = vec2<f32>(game_mod(p.x + {sx} * 0.5, {sx}) - {sx} * 0.5, game_mod(p.y + {sy} * 0.5, {sy}) - {sy} * 0.5);\n"
            ));
        }
        "mirror" => {
            s.push_str(&format!("{indent}p = vec2<f32>(abs(p.x), p.y);\n"));
        }
        "radial" => {
            let count = get_arg(args, "count", 0, "radial");
            s.push_str(&format!("{indent}{{ let r_angle = atan2(p.y, p.x);\n"));
            s.push_str(&format!("{indent}let r_sector = 6.28318 / {count};\n"));
            s.push_str(&format!(
                "{indent}let r_a = game_mod(r_angle + r_sector * 0.5, r_sector) - r_sector * 0.5;\n"
            ));
            s.push_str(&format!("{indent}let r_r = length(p);\n"));
            s.push_str(&format!(
                "{indent}p = vec2<f32>(r_r * cos(r_a), r_r * sin(r_a)); }}\n"
            ));
        }
        // ── Shape modifiers ─────────────────────────────
        "round" => {
            let r = get_arg(args, "radius", 0, "round");
            s.push_str(&format!("{indent}sdf_result = sdf_result - {r};\n"));
        }
        "shell" => {
            let w = get_arg(args, "width", 0, "shell");
            s.push_str(&format!("{indent}sdf_result = abs(sdf_result) - {w};\n"));
        }
        "onion" => {
            let count = get_arg(args, "count", 0, "onion");
            let w = get_arg(args, "width", 1, "onion");
            s.push_str(&format!(
                "{indent}for (var onion_i: i32 = 0; onion_i < i32({count}); onion_i = onion_i + 1) {{ sdf_result = abs(sdf_result) - {w}; }}\n"
            ));
        }
        "outline" => {
            let w = get_arg(args, "width", 0, "outline");
            // outline is Color->Color: use the sdf approach on the color's luminance
            s.push_str(&format!(
                "{indent}{{ let out_lum = dot(color_result.rgb, vec3<f32>(0.299, 0.587, 0.114));\n"
            ));
            s.push_str(&format!("{indent}let out_edge = abs(out_lum) - {w};\n"));
            s.push_str(&format!("{indent}color_result = vec4<f32>(color_result.rgb * (1.0 - smoothstep(0.0, 0.02, out_edge)), color_result.a * (1.0 - smoothstep(0.0, 0.02, out_edge))); }}\n"));
        }
        // ── New SDF primitives ──────────────────────────
        "line" => {
            let x1 = get_arg(args, "x1", 0, "line");
            let y1 = get_arg(args, "y1", 1, "line");
            let x2 = get_arg(args, "x2", 2, "line");
            let y2 = get_arg(args, "y2", 3, "line");
            let w = get_arg(args, "width", 4, "line");
            s.push_str(&format!(
                "{indent}var sdf_result = sdf_line(p, vec2<f32>({x1}, {y1}), vec2<f32>({x2}, {y2})) - {w};\n"
            ));
        }
        "capsule" => {
            let len = get_arg(args, "length", 0, "capsule");
            let r = get_arg(args, "radius", 1, "capsule");
            s.push_str(&format!(
                "{indent}var sdf_result = sdf_line(p, vec2<f32>(-{len} * 0.5, 0.0), vec2<f32>({len} * 0.5, 0.0)) - {r};\n"
            ));
        }
        "triangle" => {
            let sz = get_arg(args, "size", 0, "triangle");
            s.push_str(&format!(
                "{indent}var sdf_result = sdf_triangle(p, {sz});\n"
            ));
        }
        "arc_sdf" => {
            let r = get_arg(args, "radius", 0, "arc_sdf");
            let angle = get_arg(args, "angle", 1, "arc_sdf");
            let w = get_arg(args, "width", 2, "arc_sdf");
            s.push_str(&format!(
                "{indent}var sdf_result = sdf_arc(p, {r}, {angle}, {w});\n"
            ));
        }
        "cross" => {
            let sz = get_arg(args, "size", 0, "cross");
            let aw = get_arg(args, "arm_width", 1, "cross");
            s.push_str(&format!(
                "{indent}var sdf_result = min(sdf_box(p, {sz}, {aw}), sdf_box(p, {aw}, {sz}));\n"
            ));
        }
        "heart" => {
            let sz = get_arg(args, "size", 0, "heart");
            s.push_str(&format!("{indent}var sdf_result = sdf_heart(p, {sz});\n"));
        }
        "egg" => {
            let r = get_arg(args, "radius", 0, "egg");
            let k = get_arg(args, "k", 1, "egg");
            s.push_str(&format!("{indent}var sdf_result = sdf_egg(p, {r}, {k});\n"));
        }
        "spiral" => {
            let turns = get_arg(args, "turns", 0, "spiral");
            let w = get_arg(args, "width", 1, "spiral");
            s.push_str(&format!("{indent}let sp_r = length(p);\n"));
            s.push_str(&format!("{indent}let sp_a = atan2(p.y, p.x);\n"));
            s.push_str(&format!(
                "{indent}let sp_d = sp_r - (sp_a + 3.14159265) / 6.28318 / {turns};\n"
            ));
            s.push_str(&format!(
                "{indent}var sdf_result = abs(sp_d - floor(sp_d + 0.5)) - {w};\n"
            ));
        }
        "grid" => {
            let spacing = get_arg(args, "spacing", 0, "grid");
            let w = get_arg(args, "width", 1, "grid");
            s.push_str(&format!("{indent}let gx = abs(game_mod(p.x + {spacing} * 0.5, {spacing}) - {spacing} * 0.5) - {w};\n"));
            s.push_str(&format!("{indent}let gy = abs(game_mod(p.y + {spacing} * 0.5, {spacing}) - {spacing} * 0.5) - {w};\n"));
            s.push_str(&format!("{indent}var sdf_result = min(gx, gy);\n"));
        }
        _ => {
            s.push_str(&format!("{indent}// Unknown stage: {}\n", stage.name));
        }
    }
}

/// Emit WGSL code for a sub-expression SDF call (used by boolean ops).
fn emit_wgsl_sub_sdf(s: &mut String, expr: &Expr, var_name: &str, indent: &str) {
    if let Expr::Call { name, args } = expr {
        let sub_args: Vec<crate::ast::Arg> = args.clone();
        match name.as_str() {
            "circle" => {
                let r = get_arg(&sub_args, "radius", 0, "circle");
                s.push_str(&format!("{indent}let {var_name} = sdf_circle(p, {r});\n"));
            }
            "ring" => {
                let r = get_arg(&sub_args, "radius", 0, "ring");
                let w = get_arg(&sub_args, "width", 1, "ring");
                s.push_str(&format!(
                    "{indent}let {var_name} = abs(length(p) - {r}) - {w};\n"
                ));
            }
            "star" => {
                let n = get_arg(&sub_args, "points", 0, "star");
                let r = get_arg(&sub_args, "radius", 1, "star");
                let ir = get_arg(&sub_args, "inner", 2, "star");
                s.push_str(&format!(
                    "{indent}let {var_name} = sdf_star(p, {n}, {r}, {ir});\n"
                ));
            }
            "box" => {
                let w = get_arg(&sub_args, "width", 0, "box");
                let h = get_arg(&sub_args, "height", 1, "box");
                s.push_str(&format!("{indent}let {var_name} = sdf_box(p, {w}, {h});\n"));
            }
            "hex" => {
                let r = get_arg(&sub_args, "radius", 0, "hex");
                s.push_str(&format!("{indent}let {var_name} = sdf_hex(p, {r});\n"));
            }
            "line" => {
                let x1 = get_arg(&sub_args, "x1", 0, "line");
                let y1 = get_arg(&sub_args, "y1", 1, "line");
                let x2 = get_arg(&sub_args, "x2", 2, "line");
                let y2 = get_arg(&sub_args, "y2", 3, "line");
                let w = get_arg(&sub_args, "width", 4, "line");
                s.push_str(&format!(
                    "{indent}let {var_name} = sdf_line(p, vec2<f32>({x1}, {y1}), vec2<f32>({x2}, {y2})) - {w};\n"
                ));
            }
            "capsule" => {
                let len = get_arg(&sub_args, "length", 0, "capsule");
                let r = get_arg(&sub_args, "radius", 1, "capsule");
                s.push_str(&format!(
                    "{indent}let {var_name} = sdf_line(p, vec2<f32>(-{len} * 0.5, 0.0), vec2<f32>({len} * 0.5, 0.0)) - {r};\n"
                ));
            }
            "triangle" => {
                let sz = get_arg(&sub_args, "size", 0, "triangle");
                s.push_str(&format!(
                    "{indent}let {var_name} = sdf_triangle(p, {sz});\n"
                ));
            }
            "heart" => {
                let sz = get_arg(&sub_args, "size", 0, "heart");
                s.push_str(&format!("{indent}let {var_name} = sdf_heart(p, {sz});\n"));
            }
            "egg" => {
                let r = get_arg(&sub_args, "radius", 0, "egg");
                let k = get_arg(&sub_args, "k", 1, "egg");
                s.push_str(&format!("{indent}let {var_name} = sdf_egg(p, {r}, {k});\n"));
            }
            _ => {
                s.push_str(&format!(
                    "{indent}let {var_name} = length(p) - 0.2; // fallback\n"
                ));
            }
        }
    }
}

/// Emit WGSL code for SDF morph (interpolation between two SDFs).
fn emit_wgsl_morph(s: &mut String, stage: &Stage, indent: &str) {
    let args = &stage.args;
    if args.len() < 3 {
        s.push_str(&format!("{indent}var sdf_result = length(p) - 0.2;\n"));
        return;
    }
    emit_wgsl_sub_sdf(s, &args[0].value, "sdf_a", indent);
    emit_wgsl_sub_sdf(s, &args[1].value, "sdf_b", indent);
    let t = emit_wgsl_expr(&args[2].value);
    s.push_str(&format!(
        "{indent}var sdf_result = mix(sdf_a, sdf_b, {t});\n"
    ));
}

/// Emit WGSL code for a boolean SDF operation (union, subtract, intersect, xor).
fn emit_wgsl_bool_op(s: &mut String, stage: &Stage, indent: &str) {
    let args = &stage.args;
    if args.len() < 2 {
        s.push_str(&format!("{indent}var sdf_result = length(p) - 0.2;\n"));
        return;
    }
    emit_wgsl_sub_sdf(s, &args[0].value, "sdf_a", indent);
    emit_wgsl_sub_sdf(s, &args[1].value, "sdf_b", indent);
    let op = match stage.name.as_str() {
        "union" => "min(sdf_a, sdf_b)",
        "subtract" => "max(sdf_a, -sdf_b)",
        "intersect" => "max(sdf_a, sdf_b)",
        "xor" => "max(min(sdf_a, sdf_b), -max(sdf_a, sdf_b))",
        _ => "min(sdf_a, sdf_b)",
    };
    s.push_str(&format!("{indent}var sdf_result = {op};\n"));
}

/// Emit WGSL code for a smooth boolean SDF operation.
fn emit_wgsl_smooth_bool_op(s: &mut String, stage: &Stage, indent: &str) {
    let args = &stage.args;
    if args.len() < 2 {
        s.push_str(&format!("{indent}var sdf_result = length(p) - 0.2;\n"));
        return;
    }
    emit_wgsl_sub_sdf(s, &args[0].value, "sdf_a", indent);
    emit_wgsl_sub_sdf(s, &args[1].value, "sdf_b", indent);
    let k = if args.len() >= 3 {
        get_arg(args, "k", 2, &stage.name)
    } else {
        "0.100000".into()
    };
    let op = match stage.name.as_str() {
        "smooth_union" => format!("smin(sdf_a, sdf_b, {k})"),
        "smooth_subtract" => format!("-smin(-sdf_a, sdf_b, {k})"),
        "smooth_intersect" => format!("-smin(-sdf_a, -sdf_b, {k})"),
        _ => format!("smin(sdf_a, sdf_b, {k})"),
    };
    s.push_str(&format!("{indent}var sdf_result = {op};\n"));
}

fn has_stage(layer: &Layer, name: &str) -> bool {
    match &layer.body {
        LayerBody::Pipeline(stages) => has_stage_in_stages(stages, name),
        LayerBody::Conditional {
            then_branch,
            else_branch,
            ..
        } => has_stage_in_stages(then_branch, name) || has_stage_in_stages(else_branch, name),
        LayerBody::Params(_) => false,
    }
}

fn has_stage_in_stages(stages: &[Stage], name: &str) -> bool {
    stages.iter().any(|s| {
        if s.name == name {
            return true;
        }
        s.args.iter().any(|a| has_stage_in_expr(&a.value, name))
    })
}

/// Recursively check if an expression tree references a stage by name.
fn has_stage_in_expr(expr: &Expr, name: &str) -> bool {
    match expr {
        Expr::Call {
            name: call_name,
            args,
        } => {
            if call_name == name {
                return true;
            }
            args.iter().any(|a| has_stage_in_expr(&a.value, name))
        }
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
                opacity: None,
                cast: None,
                blend: BlendMode::Add,
                feedback: false,
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
            passes: vec![],
            cinematic_uses: vec![],
            matrix_coupling: None,
            matrix_color: None,
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

    fn make_multi_layer(layers: Vec<Layer>) -> Cinematic {
        Cinematic {
            name: "test".into(),
            layers,
            arcs: vec![],
            resonates: vec![],
            listen: None,
            voice: None,
            score: None,
            gravity: None,
            react: None,
            swarm: None,
            flow: None,
            passes: vec![],
            cinematic_uses: vec![],
            matrix_coupling: None,
            matrix_color: None,
        }
    }

    #[test]
    fn wgsl_screen_blend_emits_formula() {
        let cin = make_multi_layer(vec![
            Layer {
                name: "bg".into(),
                opts: vec![],
                memory: None,
                opacity: None,
                cast: None,
                blend: BlendMode::Add,
                feedback: false,
                body: LayerBody::Pipeline(vec![Stage {
                    name: "circle".into(),
                    args: vec![],
                }]),
            },
            Layer {
                name: "fx".into(),
                opts: vec![],
                memory: None,
                opacity: None,
                cast: None,
                blend: BlendMode::Screen,
                feedback: false,
                body: LayerBody::Pipeline(vec![Stage {
                    name: "circle".into(),
                    args: vec![],
                }]),
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(
            output.contains("1.0 - (1.0 - "),
            "screen blend formula: {output}"
        );
    }

    #[test]
    fn wgsl_multiply_blend_emits_formula() {
        let cin = make_multi_layer(vec![
            Layer {
                name: "bg".into(),
                opts: vec![],
                memory: None,
                opacity: None,
                cast: None,
                blend: BlendMode::Add,
                feedback: false,
                body: LayerBody::Pipeline(vec![Stage {
                    name: "circle".into(),
                    args: vec![],
                }]),
            },
            Layer {
                name: "fx".into(),
                opts: vec![],
                memory: None,
                opacity: None,
                cast: None,
                blend: BlendMode::Multiply,
                feedback: false,
                body: LayerBody::Pipeline(vec![Stage {
                    name: "circle".into(),
                    args: vec![],
                }]),
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(
            output.contains("final_color.rgb * lc"),
            "multiply blend formula: {output}"
        );
    }

    #[test]
    fn wgsl_overlay_blend_emits_select() {
        let cin = make_multi_layer(vec![
            Layer {
                name: "bg".into(),
                opts: vec![],
                memory: None,
                opacity: None,
                cast: None,
                blend: BlendMode::Add,
                feedback: false,
                body: LayerBody::Pipeline(vec![Stage {
                    name: "circle".into(),
                    args: vec![],
                }]),
            },
            Layer {
                name: "fx".into(),
                opts: vec![],
                memory: None,
                opacity: None,
                cast: None,
                blend: BlendMode::Overlay,
                feedback: false,
                body: LayerBody::Pipeline(vec![Stage {
                    name: "circle".into(),
                    args: vec![],
                }]),
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(
            output.contains("select("),
            "overlay blend uses select: {output}"
        );
    }

    #[test]
    fn wgsl_union_emits_min() {
        let cin = make_cinematic(vec![
            Stage {
                name: "union".into(),
                args: vec![
                    Arg {
                        name: None,
                        value: Expr::Call {
                            name: "circle".into(),
                            args: vec![Arg {
                                name: None,
                                value: Expr::Number(0.3),
                            }],
                        },
                    },
                    Arg {
                        name: None,
                        value: Expr::Call {
                            name: "box".into(),
                            args: vec![
                                Arg {
                                    name: None,
                                    value: Expr::Number(0.2),
                                },
                                Arg {
                                    name: None,
                                    value: Expr::Number(0.4),
                                },
                            ],
                        },
                    },
                ],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("sdf_circle"), "emits circle sub-sdf");
        assert!(output.contains("sdf_box"), "emits box sub-sdf");
        assert!(output.contains("min(sdf_a, sdf_b)"), "union = min");
    }

    #[test]
    fn wgsl_smooth_union_emits_smin() {
        let cin = make_cinematic(vec![
            Stage {
                name: "smooth_union".into(),
                args: vec![
                    Arg {
                        name: None,
                        value: Expr::Call {
                            name: "circle".into(),
                            args: vec![Arg {
                                name: None,
                                value: Expr::Number(0.3),
                            }],
                        },
                    },
                    Arg {
                        name: None,
                        value: Expr::Call {
                            name: "box".into(),
                            args: vec![
                                Arg {
                                    name: None,
                                    value: Expr::Number(0.2),
                                },
                                Arg {
                                    name: None,
                                    value: Expr::Number(0.4),
                                },
                            ],
                        },
                    },
                    Arg {
                        name: None,
                        value: Expr::Number(0.1),
                    },
                ],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("fn smin("), "smin helper emitted");
        assert!(
            output.contains("smin(sdf_a, sdf_b,"),
            "smooth union uses smin"
        );
    }

    #[test]
    fn wgsl_repeat_emits_game_mod() {
        let cin = make_cinematic(vec![
            Stage {
                name: "repeat".into(),
                args: vec![
                    Arg {
                        name: None,
                        value: Expr::Number(0.5),
                    },
                    Arg {
                        name: None,
                        value: Expr::Number(0.5),
                    },
                ],
            },
            Stage {
                name: "circle".into(),
                args: vec![],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("fn game_mod("), "game_mod helper emitted");
        assert!(output.contains("game_mod(p.x"), "repeat uses game_mod");
    }

    #[test]
    fn wgsl_mirror_emits_abs() {
        let cin = make_cinematic(vec![
            Stage {
                name: "mirror".into(),
                args: vec![],
            },
            Stage {
                name: "circle".into(),
                args: vec![],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("abs(p.x)"), "mirror uses abs(p.x)");
    }

    #[test]
    fn wgsl_round_shell_onion() {
        let cin = make_cinematic(vec![
            Stage {
                name: "circle".into(),
                args: vec![],
            },
            Stage {
                name: "round".into(),
                args: vec![Arg {
                    name: None,
                    value: Expr::Number(0.02),
                }],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(
            output.contains("sdf_result = sdf_result -"),
            "round subtracts radius"
        );

        let cin2 = make_cinematic(vec![
            Stage {
                name: "circle".into(),
                args: vec![],
            },
            Stage {
                name: "shell".into(),
                args: vec![],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output2 = generate_fragment(&cin2, &[]);
        assert!(output2.contains("abs(sdf_result)"), "shell uses abs");

        let cin3 = make_cinematic(vec![
            Stage {
                name: "circle".into(),
                args: vec![],
            },
            Stage {
                name: "onion".into(),
                args: vec![],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output3 = generate_fragment(&cin3, &[]);
        assert!(output3.contains("for (var onion_i"), "onion uses loop");
    }

    #[test]
    fn wgsl_new_primitives_emit_helpers() {
        let cin = make_cinematic(vec![
            Stage {
                name: "line".into(),
                args: vec![],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("fn sdf_line("), "sdf_line helper emitted");
        assert!(output.contains("sdf_line(p,"), "line stage uses sdf_line");

        let cin2 = make_cinematic(vec![
            Stage {
                name: "triangle".into(),
                args: vec![],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output2 = generate_fragment(&cin2, &[]);
        assert!(
            output2.contains("fn sdf_triangle("),
            "triangle helper emitted"
        );

        let cin3 = make_cinematic(vec![
            Stage {
                name: "heart".into(),
                args: vec![],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output3 = generate_fragment(&cin3, &[]);
        assert!(output3.contains("fn sdf_heart("), "heart helper emitted");

        let cin4 = make_cinematic(vec![
            Stage {
                name: "grid".into(),
                args: vec![],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output4 = generate_fragment(&cin4, &[]);
        assert!(output4.contains("game_mod(p.x"), "grid uses game_mod");
    }

    #[test]
    fn wgsl_subtract_emits_max_neg() {
        let cin = make_cinematic(vec![
            Stage {
                name: "subtract".into(),
                args: vec![
                    Arg {
                        name: None,
                        value: Expr::Call {
                            name: "circle".into(),
                            args: vec![Arg {
                                name: None,
                                value: Expr::Number(0.4),
                            }],
                        },
                    },
                    Arg {
                        name: None,
                        value: Expr::Call {
                            name: "circle".into(),
                            args: vec![Arg {
                                name: None,
                                value: Expr::Number(0.2),
                            }],
                        },
                    },
                ],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(
            output.contains("max(sdf_a, -sdf_b)"),
            "subtract = max(a, -b)"
        );
    }

    // v0.4 — morph

    #[test]
    fn wgsl_morph_generates_mix() {
        let cin = make_cinematic(vec![
            Stage {
                name: "morph".into(),
                args: vec![
                    Arg {
                        name: None,
                        value: Expr::Call {
                            name: "circle".into(),
                            args: vec![Arg {
                                name: None,
                                value: Expr::Number(0.3),
                            }],
                        },
                    },
                    Arg {
                        name: None,
                        value: Expr::Call {
                            name: "star".into(),
                            args: vec![],
                        },
                    },
                    Arg {
                        name: None,
                        value: Expr::Number(0.5),
                    },
                ],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("mix(sdf_a, sdf_b,"), "morph uses mix()");
        assert!(output.contains("sdf_a"), "morph emits sdf_a");
        assert!(output.contains("sdf_b"), "morph emits sdf_b");
    }

    // v0.4 — named palettes

    #[test]
    fn wgsl_named_palette_fire() {
        let cin = make_cinematic(vec![
            Stage {
                name: "circle".into(),
                args: vec![],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
            Stage {
                name: "palette".into(),
                args: vec![Arg {
                    name: None,
                    value: Expr::Ident("fire".into()),
                }],
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("cosine_palette"), "palette helper used");
    }

    #[test]
    fn wgsl_named_palette_ocean() {
        let cin = make_cinematic(vec![
            Stage {
                name: "circle".into(),
                args: vec![],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
            Stage {
                name: "palette".into(),
                args: vec![Arg {
                    name: None,
                    value: Expr::Ident("ocean".into()),
                }],
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("cosine_palette"), "palette helper used");
    }

    // v0.4 — fn inlining

    #[test]
    fn wgsl_fn_inlining() {
        let fns = vec![FnDef {
            name: "dot".into(),
            params: vec!["r".into()],
            body: vec![
                Stage {
                    name: "circle".into(),
                    args: vec![Arg {
                        name: None,
                        value: Expr::Ident("r".into()),
                    }],
                },
                Stage {
                    name: "glow".into(),
                    args: vec![],
                },
                Stage {
                    name: "tint".into(),
                    args: vec![],
                },
            ],
        }];
        let cin = make_cinematic(vec![Stage {
            name: "dot".into(),
            args: vec![Arg {
                name: None,
                value: Expr::Number(0.2),
            }],
        }]);
        let output = generate_fragment_with_fns(&cin, &[], &fns);
        // The fn body should be inlined — circle should appear with substituted arg
        assert!(output.contains("sdf_circle"), "inlined circle from fn");
        assert!(output.contains("apply_glow"), "inlined glow from fn");
    }

    // v0.4 — conditional layer

    #[test]
    fn wgsl_conditional_generates_select() {
        let cin = Cinematic {
            name: "test".into(),
            layers: vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                opacity: None,
                cast: None,
                blend: BlendMode::Add,
                feedback: false,
                body: LayerBody::Conditional {
                    condition: Expr::BinOp {
                        op: BinOp::Gt,
                        left: Box::new(Expr::Ident("bass".into())),
                        right: Box::new(Expr::Number(0.5)),
                    },
                    then_branch: vec![
                        Stage {
                            name: "circle".into(),
                            args: vec![],
                        },
                        Stage {
                            name: "glow".into(),
                            args: vec![],
                        },
                        Stage {
                            name: "tint".into(),
                            args: vec![],
                        },
                    ],
                    else_branch: vec![
                        Stage {
                            name: "ring".into(),
                            args: vec![],
                        },
                        Stage {
                            name: "glow".into(),
                            args: vec![],
                        },
                        Stage {
                            name: "tint".into(),
                            args: vec![],
                        },
                    ],
                },
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
            passes: vec![],
            cinematic_uses: vec![],
            matrix_coupling: None,
            matrix_color: None,
        };
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("select("), "conditional uses select()");
        assert!(output.contains("then_color"), "then branch result");
        assert!(output.contains("else_color"), "else branch result");
    }

    // v0.4 — emit_wgsl_expr

    #[test]
    fn emit_wgsl_expr_comparison() {
        let expr = Expr::BinOp {
            op: BinOp::Gt,
            left: Box::new(Expr::Ident("bass".into())),
            right: Box::new(Expr::Number(0.5)),
        };
        let result = emit_wgsl_expr(&expr);
        assert!(result.contains(">"), "greater than emitted");
        assert!(result.contains("bass"), "left side");
        assert!(result.contains("0.5"), "right side");
    }

    #[test]
    fn emit_wgsl_expr_arithmetic() {
        let expr = Expr::BinOp {
            op: BinOp::Mul,
            left: Box::new(Expr::Ident("time".into())),
            right: Box::new(Expr::Number(2.0)),
        };
        let result = emit_wgsl_expr(&expr);
        assert!(result.contains("time"), "time ident emitted");
        assert!(result.contains("*"), "mul operator");
        assert!(result.contains("2"), "right operand");
    }

    #[test]
    fn pass_shader_has_struct_definitions() {
        let pass = PassBlock {
            name: "blur".into(),
            body: vec![Stage {
                name: "blur".into(),
                args: vec![Arg {
                    name: None,
                    value: Expr::Number(2.0),
                }],
            }],
        };
        let wgsl = generate_pass_fragment(&pass);
        // Must have self-contained struct definitions
        assert!(
            wgsl.contains("struct Uniforms {"),
            "pass shader must define Uniforms"
        );
        assert!(
            wgsl.contains("struct VertexOutput {"),
            "pass shader must define VertexOutput"
        );
        assert!(
            wgsl.contains("resolution: vec2<f32>"),
            "Uniforms has resolution"
        );
        assert!(
            wgsl.contains("@location(0) uv: vec2<f32>"),
            "VertexOutput has uv"
        );
        // And the actual pass content
        assert!(
            wgsl.contains("@group(0) @binding(3) var pass_tex"),
            "has pass_tex binding"
        );
        assert!(wgsl.contains("fn fs_main"), "has fragment entry point");
        assert!(
            wgsl.contains("textureSample(pass_tex"),
            "samples pass texture"
        );
    }

    #[test]
    fn pass_vignette_generates_correct_code() {
        let pass = PassBlock {
            name: "v".into(),
            body: vec![Stage {
                name: "vignette".into(),
                args: vec![Arg {
                    name: None,
                    value: Expr::Number(0.6),
                }],
            }],
        };
        let wgsl = generate_pass_fragment(&pass);
        assert!(wgsl.contains("vign"), "vignette uses vign variable");
        assert!(wgsl.contains("0.6"), "vignette strength parameter");
        assert!(
            wgsl.contains("length(uv - 0.5)"),
            "vignette uses distance from center"
        );
    }

    #[test]
    fn pass_threshold_generates_luminance_check() {
        let pass = PassBlock {
            name: "t".into(),
            body: vec![Stage {
                name: "threshold".into(),
                args: vec![Arg {
                    name: None,
                    value: Expr::Number(0.3),
                }],
            }],
        };
        let wgsl = generate_pass_fragment(&pass);
        assert!(wgsl.contains("0.299"), "uses luminance coefficients");
        assert!(wgsl.contains("select"), "uses select for threshold");
    }

    #[test]
    fn pass_invert_flips_rgb() {
        let pass = PassBlock {
            name: "inv".into(),
            body: vec![Stage {
                name: "invert".into(),
                args: vec![],
            }],
        };
        let wgsl = generate_pass_fragment(&pass);
        assert!(
            wgsl.contains("1.0 - color_result.rgb"),
            "invert subtracts from 1.0"
        );
    }

    #[test]
    fn pass_blend_add_clamps_output() {
        let pass = PassBlock {
            name: "add".into(),
            body: vec![Stage {
                name: "blend_add".into(),
                args: vec![],
            }],
        };
        let wgsl = generate_pass_fragment(&pass);
        assert!(
            wgsl.contains("min(pixel.rgb + color_result.rgb"),
            "blend_add uses min to clamp"
        );
    }

    #[test]
    fn pass_multi_stage_chain() {
        let pass = PassBlock {
            name: "fx".into(),
            body: vec![
                Stage {
                    name: "blur".into(),
                    args: vec![Arg {
                        name: None,
                        value: Expr::Number(2.0),
                    }],
                },
                Stage {
                    name: "vignette".into(),
                    args: vec![Arg {
                        name: None,
                        value: Expr::Number(0.5),
                    }],
                },
            ],
        };
        let wgsl = generate_pass_fragment(&pass);
        // Both stages should be present in order
        assert!(wgsl.contains("blurred"), "blur stage emitted");
        assert!(wgsl.contains("vign"), "vignette stage emitted");
    }

    #[test]
    fn pass_unknown_stage_comments_through() {
        let pass = PassBlock {
            name: "custom".into(),
            body: vec![Stage {
                name: "nonexistent_effect".into(),
                args: vec![],
            }],
        };
        let wgsl = generate_pass_fragment(&pass);
        assert!(wgsl.contains("// unknown pass stage: nonexistent_effect"));
    }

    #[test]
    fn substitute_fn_args_works() {
        let stage = Stage {
            name: "circle".into(),
            args: vec![Arg {
                name: None,
                value: Expr::Ident("size".into()),
            }],
        };
        let params = vec!["size".to_string()];
        let caller_args = vec![Arg {
            name: None,
            value: Expr::Number(0.3),
        }];
        let result = substitute_fn_args(&stage, &params, &caller_args);
        match &result.args[0].value {
            Expr::Number(v) => assert_eq!(*v, 0.3),
            _ => panic!("expected Number after substitution"),
        }
    }
}
