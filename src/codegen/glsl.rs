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
use crate::codegen::wgsl::substitute_fn_args;
use crate::codegen::UniformInfo;

fn named_palette_glsl(name: &str) -> Option<(&str, &str, &str, &str)> {
    match name {
        "fire" => Some((
            "vec3(0.5, 0.3, 0.1)",
            "vec3(0.5, 0.2, 0.1)",
            "vec3(1.0, 1.0, 1.0)",
            "vec3(0.0, 0.25, 0.25)",
        )),
        "ocean" => Some((
            "vec3(0.0, 0.3, 0.5)",
            "vec3(0.0, 0.3, 0.5)",
            "vec3(1.0, 1.0, 1.0)",
            "vec3(0.0, 0.1, 0.2)",
        )),
        "neon" => Some((
            "vec3(0.5, 0.5, 0.5)",
            "vec3(0.5, 0.5, 0.5)",
            "vec3(1.0, 1.0, 1.0)",
            "vec3(0.0, 0.33, 0.67)",
        )),
        "aurora" => Some((
            "vec3(0.0, 0.5, 0.3)",
            "vec3(0.2, 0.5, 0.4)",
            "vec3(1.0, 1.0, 1.0)",
            "vec3(0.0, 0.1, 0.3)",
        )),
        "sunset" => Some((
            "vec3(0.5, 0.3, 0.2)",
            "vec3(0.5, 0.2, 0.3)",
            "vec3(1.0, 1.0, 0.5)",
            "vec3(0.8, 0.9, 0.3)",
        )),
        "ice" => Some((
            "vec3(0.5, 0.7, 0.9)",
            "vec3(0.2, 0.2, 0.1)",
            "vec3(1.0, 1.0, 1.0)",
            "vec3(0.0, 0.05, 0.15)",
        )),
        "ember" => Some((
            "vec3(0.6, 0.2, 0.05)",
            "vec3(0.4, 0.2, 0.1)",
            "vec3(1.0, 0.5, 0.5)",
            "vec3(0.0, 0.15, 0.2)",
        )),
        "lava" => Some((
            "vec3(0.5, 0.2, 0.0)",
            "vec3(0.5, 0.3, 0.1)",
            "vec3(0.8, 0.5, 0.5)",
            "vec3(0.0, 0.2, 0.3)",
        )),
        "magma" => Some((
            "vec3(0.55, 0.2, 0.08)",
            "vec3(0.45, 0.25, 0.1)",
            "vec3(1.0, 0.7, 0.4)",
            "vec3(0.0, 0.15, 0.25)",
        )),
        "inferno" => Some((
            "vec3(0.5, 0.3, 0.15)",
            "vec3(0.5, 0.35, 0.2)",
            "vec3(1.0, 1.0, 0.7)",
            "vec3(0.0, 0.15, 0.3)",
        )),
        "plasma" => Some((
            "vec3(0.5, 0.2, 0.5)",
            "vec3(0.5, 0.3, 0.5)",
            "vec3(1.0, 1.0, 1.0)",
            "vec3(0.15, 0.0, 0.5)",
        )),
        "electric" => Some((
            "vec3(0.1, 0.4, 0.8)",
            "vec3(0.3, 0.4, 0.2)",
            "vec3(1.0, 1.0, 1.0)",
            "vec3(0.0, 0.1, 0.3)",
        )),
        "cyber" => Some((
            "vec3(0.0, 0.5, 0.3)",
            "vec3(0.1, 0.5, 0.4)",
            "vec3(1.0, 1.0, 0.5)",
            "vec3(0.0, 0.2, 0.5)",
        )),
        "matrix" => Some((
            "vec3(0.0, 0.3, 0.0)",
            "vec3(0.0, 0.5, 0.0)",
            "vec3(0.0, 1.0, 0.0)",
            "vec3(0.0, 0.2, 0.0)",
        )),
        "forest" => Some((
            "vec3(0.2, 0.35, 0.1)",
            "vec3(0.15, 0.25, 0.1)",
            "vec3(0.8, 1.0, 0.5)",
            "vec3(0.0, 0.2, 0.4)",
        )),
        "moss" => Some((
            "vec3(0.25, 0.3, 0.15)",
            "vec3(0.15, 0.2, 0.1)",
            "vec3(0.7, 0.8, 0.5)",
            "vec3(0.1, 0.2, 0.3)",
        )),
        "earth" => Some((
            "vec3(0.4, 0.3, 0.2)",
            "vec3(0.2, 0.15, 0.1)",
            "vec3(0.8, 0.7, 0.5)",
            "vec3(0.0, 0.1, 0.2)",
        )),
        "desert" => Some((
            "vec3(0.6, 0.4, 0.25)",
            "vec3(0.3, 0.2, 0.15)",
            "vec3(0.7, 0.5, 0.4)",
            "vec3(0.0, 0.1, 0.2)",
        )),
        "blood" => Some((
            "vec3(0.4, 0.05, 0.05)",
            "vec3(0.4, 0.1, 0.05)",
            "vec3(1.0, 0.5, 0.5)",
            "vec3(0.0, 0.15, 0.3)",
        )),
        "rose" => Some((
            "vec3(0.6, 0.3, 0.4)",
            "vec3(0.3, 0.2, 0.3)",
            "vec3(1.0, 0.8, 1.0)",
            "vec3(0.0, 0.1, 0.3)",
        )),
        "candy" => Some((
            "vec3(0.6, 0.3, 0.6)",
            "vec3(0.4, 0.3, 0.4)",
            "vec3(1.0, 1.0, 1.0)",
            "vec3(0.0, 0.33, 0.67)",
        )),
        "royal" => Some((
            "vec3(0.3, 0.1, 0.5)",
            "vec3(0.3, 0.2, 0.3)",
            "vec3(0.8, 0.5, 1.0)",
            "vec3(0.2, 0.0, 0.3)",
        )),
        "deep_sea" => Some((
            "vec3(0.0, 0.1, 0.3)",
            "vec3(0.0, 0.2, 0.3)",
            "vec3(0.5, 0.8, 1.0)",
            "vec3(0.0, 0.1, 0.2)",
        )),
        "coral" => Some((
            "vec3(0.6, 0.35, 0.3)",
            "vec3(0.3, 0.25, 0.2)",
            "vec3(0.8, 0.7, 0.8)",
            "vec3(0.0, 0.1, 0.25)",
        )),
        "arctic" => Some((
            "vec3(0.7, 0.8, 0.95)",
            "vec3(0.2, 0.15, 0.1)",
            "vec3(1.0, 1.0, 1.0)",
            "vec3(0.0, 0.05, 0.1)",
        )),
        "twilight" => Some((
            "vec3(0.4, 0.2, 0.5)",
            "vec3(0.3, 0.3, 0.3)",
            "vec3(1.0, 0.8, 0.5)",
            "vec3(0.3, 0.1, 0.0)",
        )),
        "vapor" => Some((
            "vec3(0.5, 0.3, 0.6)",
            "vec3(0.5, 0.3, 0.4)",
            "vec3(1.0, 1.0, 0.8)",
            "vec3(0.3, 0.2, 0.0)",
        )),
        "gold" => Some((
            "vec3(0.55, 0.42, 0.15)",
            "vec3(0.3, 0.25, 0.1)",
            "vec3(0.8, 0.6, 0.4)",
            "vec3(0.0, 0.1, 0.2)",
        )),
        "silver" => Some((
            "vec3(0.5, 0.5, 0.55)",
            "vec3(0.2, 0.2, 0.2)",
            "vec3(1.0, 1.0, 1.0)",
            "vec3(0.0, 0.0, 0.1)",
        )),
        "monochrome" => Some((
            "vec3(0.5, 0.5, 0.5)",
            "vec3(0.3, 0.3, 0.3)",
            "vec3(1.0, 1.0, 1.0)",
            "vec3(0.0, 0.0, 0.0)",
        )),
        _ => None,
    }
}

/// Generate a GLSL ES 3.0 fragment shader with user-defined functions.
pub fn generate_fragment_with_fns(
    cinematic: &Cinematic,
    uniforms: &[UniformInfo],
    fns: &[FnDef],
) -> String {
    generate_fragment_inner(cinematic, uniforms, fns)
}

/// Generate a GLSL ES 3.0 fragment shader for a cinematic.
pub fn generate_fragment(cinematic: &Cinematic, uniforms: &[UniformInfo]) -> String {
    generate_fragment_inner(cinematic, uniforms, &[])
}

fn generate_fragment_inner(
    cinematic: &Cinematic,
    uniforms: &[UniformInfo],
    fns: &[FnDef],
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
    s.push_str("uniform float u_mouse_down;\n");
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

    // Color matrix function (if present)
    if let Some(ref mc) = cinematic.matrix_color {
        s.push_str(&super::matrix::generate_color_matrix_glsl(mc));
        s.push('\n');
    }

    // Entry point: void main()
    s.push_str("void main(){\n");
    s.push_str("    vec2 uv = v_uv * 2.0 - 1.0;\n");
    s.push_str("    float aspect = u_resolution.x / u_resolution.y;\n");
    s.push_str("    float time = fract(u_time / 120.0) * 120.0;\n");
    s.push_str("    float mouse_x = u_mouse.x;\n");
    s.push_str("    float mouse_y = u_mouse.y;\n");
    s.push_str("    float mouse_down = u_mouse_down;\n\n");

    // Uniform param aliases
    for u in uniforms {
        s.push_str(&format!("    float {} = u_p_{};\n", u.name, u.name));
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
        s.push_str("    vec4 final_color = vec4(0.0, 0.0, 0.0, 0.0);\n\n");
    }

    for (i, layer) in cinematic.layers.iter().enumerate() {
        if matches!(layer.body, LayerBody::Params(_)) {
            continue;
        }
        emit_glsl_layer(
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
            s.push_str(
                "    final_color = vec4(apply_color_matrix(final_color.rgb), final_color.a);\n",
            );
        }
        // Quality output pipeline: tonemap + dither
        s.push_str("    final_color = vec4(aces_tonemap(final_color.rgb), final_color.a);\n");
        s.push_str("    final_color += (dither_noise(v_uv * u_resolution) - 0.5) / 255.0;\n");
        s.push_str("    fragColor = final_color;\n");
    }
    s.push_str("}\n");
    s
}

fn emit_glsl_builtins(s: &mut String, cinematic: &Cinematic) {
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
    let needs_line = cinematic.layers.iter().any(|l| has_stage(l, "line"));
    let needs_capsule = cinematic.layers.iter().any(|l| has_stage(l, "capsule"));
    let needs_triangle = cinematic.layers.iter().any(|l| has_stage(l, "triangle"));
    let needs_arc_sdf = cinematic.layers.iter().any(|l| has_stage(l, "arc_sdf"));
    let needs_heart = cinematic.layers.iter().any(|l| has_stage(l, "heart"));
    let needs_egg = cinematic.layers.iter().any(|l| has_stage(l, "egg"));

    // C-style function declarations — NOT WGSL style
    if needs_circle {
        s.push_str("float sdf_circle(vec2 p, float radius){\n");
        s.push_str("    return length(p) - radius;\n");
        s.push_str("}\n\n");
    }

    if needs_star {
        emit_glsl_star(s);
    }

    if needs_box {
        s.push_str("float sdf_box(vec2 p, float w, float h){\n");
        s.push_str("    vec2 d = abs(p) - vec2(w, h);\n");
        s.push_str("    return length(max(d, vec2(0.0))) + min(max(d.x, d.y), 0.0);\n");
        s.push_str("}\n\n");
    }

    if needs_hex {
        s.push_str("float sdf_hex(vec2 p, float r){\n");
        s.push_str("    vec3 k = vec3(-0.866025, 0.5, 0.577350);\n");
        s.push_str("    vec2 q = abs(p);\n");
        s.push_str("    q -= 2.0 * min(dot(k.xy, q), 0.0) * k.xy;\n");
        s.push_str("    q -= vec2(clamp(q.x, -k.z * r, k.z * r), r);\n");
        s.push_str("    return length(q) * sign(q.y);\n");
        s.push_str("}\n\n");
    }

    s.push_str("float apply_glow(float d, float intensity){\n");
    s.push_str("    return exp(-max(d, 0.0) * intensity * 8.0);\n");
    s.push_str("}\n\n");

    if needs_fbm || needs_warp || needs_simplex {
        emit_glsl_fbm(s);
    }

    if needs_voronoi {
        emit_glsl_voronoi(s);
    }

    if needs_palette {
        emit_glsl_palette(s);
    }

    if needs_smin {
        s.push_str("float smin(float a, float b, float k){\n");
        s.push_str("    float h = max(k - abs(a - b), 0.0) / k;\n");
        s.push_str("    return min(a, b) - h * h * k * 0.25;\n");
        s.push_str("}\n\n");
    }

    if needs_line || needs_capsule {
        s.push_str("float sdf_line(vec2 p, vec2 a, vec2 b){\n");
        s.push_str("    vec2 pa = p - a;\n");
        s.push_str("    vec2 ba = b - a;\n");
        s.push_str("    float h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);\n");
        s.push_str("    return length(pa - ba * h);\n");
        s.push_str("}\n\n");
    }

    if needs_triangle {
        s.push_str("float sdf_triangle(vec2 p, float sz){\n");
        s.push_str("    float k = sqrt(3.0);\n");
        s.push_str("    vec2 q = vec2(abs(p.x) - sz, p.y + sz / k);\n");
        s.push_str("    if (q.x + k * q.y > 0.0) q = vec2(q.x - k * q.y, -k * q.x - q.y) / 2.0;\n");
        s.push_str("    q = vec2(q.x - clamp(q.x, -2.0 * sz, 0.0), q.y);\n");
        s.push_str("    return -length(q) * sign(q.y);\n");
        s.push_str("}\n\n");
    }

    if needs_arc_sdf {
        s.push_str("float sdf_arc(vec2 p, float ra, float angle, float rb){\n");
        s.push_str("    vec2 sca = vec2(sin(angle), cos(angle));\n");
        s.push_str("    vec2 q = vec2(abs(p.x), p.y);\n");
        s.push_str("    float k = (sca.y * q.x > sca.x * q.y) ? dot(q, sca) : length(q);\n");
        s.push_str("    return sqrt(dot(q, q) + ra * ra - 2.0 * ra * k) - rb;\n");
        s.push_str("}\n\n");
    }

    if needs_heart {
        s.push_str("float sdf_heart(vec2 p, float sz){\n");
        s.push_str("    vec2 q = vec2(abs(p.x), p.y);\n");
        s.push_str("    vec2 b = vec2(sz * 0.5, sz * 0.8);\n");
        s.push_str("    float r = 0.5 * (b.x + b.y);\n");
        s.push_str("    float d = length(q - vec2(0.0, r * 0.5)) - r;\n");
        s.push_str("    float a = atan(q.x, q.y - r * 0.5);\n");
        s.push_str("    float h = sz * (0.5 + 0.3 * cos(a));\n");
        s.push_str("    return length(q - vec2(0.0, r * 0.5)) - h;\n");
        s.push_str("}\n\n");
    }

    if needs_egg {
        s.push_str("float sdf_egg(vec2 p, float ra, float rb){\n");
        s.push_str("    vec2 q = vec2(abs(p.x), p.y);\n");
        s.push_str("    float r = ra - rb;\n");
        s.push_str("    float k = (q.y < 0.0) ? length(q + vec2(0.0, rb)) : length(q);\n");
        s.push_str("    return k - ra;\n");
        s.push_str("}\n\n");
    }

    // Quality output helpers — always emitted for tonemap + dither pipeline
    emit_glsl_quality_helpers(s);
}

fn emit_glsl_star(s: &mut String) {
    s.push_str("float sdf_star(vec2 p, float n, float r, float ir){\n");
    s.push_str("    float an = 3.14159265 / n;\n");
    s.push_str("    float a = atan(p.y, p.x);\n");
    s.push_str("    float period = 2.0 * an;\n");
    s.push_str("    float sa = mod(a + an, period) - an;\n");
    s.push_str("    vec2 q = length(p) * vec2(cos(sa), abs(sin(sa)));\n");
    s.push_str("    vec2 tip = vec2(r, 0.0);\n");
    s.push_str("    vec2 valley = vec2(ir * cos(an), ir * sin(an));\n");
    s.push_str("    vec2 e = tip - valley;\n");
    s.push_str("    vec2 d = q - valley;\n");
    s.push_str("    float t = clamp(dot(d, e) / dot(e, e), 0.0, 1.0);\n");
    s.push_str("    vec2 closest = valley + e * t;\n");
    s.push_str("    float dist = length(q - closest);\n");
    s.push_str("    float cross_val = d.x * e.y - d.y * e.x;\n");
    s.push_str("    return cross_val > 0.0 ? -dist : dist;\n");
    s.push_str("}\n\n");
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

fn emit_glsl_voronoi(s: &mut String) {
    s.push_str("vec2 hash2v(vec2 p){\n");
    s.push_str("    vec3 p3 = fract(vec3(p.x, p.y, p.x) * vec3(0.1031, 0.1030, 0.0973));\n");
    s.push_str("    vec3 pp = p3 + vec3(dot(p3, p3.yzx + 33.33));\n");
    s.push_str("    return fract(vec2((pp.x + pp.y) * pp.z, (pp.x + pp.z) * pp.y));\n");
    s.push_str("}\n\n");

    s.push_str("float voronoi2(vec2 p){\n");
    s.push_str("    vec2 n = floor(p);\n");
    s.push_str("    vec2 f = fract(p);\n");
    s.push_str("    float md = 8.0;\n");
    s.push_str("    for (int j = -1; j <= 1; j++) {\n");
    s.push_str("        for (int i = -1; i <= 1; i++) {\n");
    s.push_str("            vec2 g = vec2(float(i), float(j));\n");
    s.push_str("            vec2 o = hash2v(n + g);\n");
    s.push_str("            vec2 r = g + o - f;\n");
    s.push_str("            float d = dot(r, r);\n");
    s.push_str("            md = min(md, d);\n");
    s.push_str("        }\n");
    s.push_str("    }\n");
    s.push_str("    return sqrt(md);\n");
    s.push_str("}\n\n");
}

fn emit_glsl_palette(s: &mut String) {
    s.push_str("vec3 cosine_palette(float t, vec3 a, vec3 b, vec3 c, vec3 d){\n");
    s.push_str("    return a + b * cos(6.28318 * (c * t + d));\n");
    s.push_str("}\n\n");
}

fn emit_glsl_quality_helpers(s: &mut String) {
    s.push_str("vec3 aces_tonemap(vec3 x) {\n");
    s.push_str("    vec3 a = x * (2.51 * x + 0.03);\n");
    s.push_str("    vec3 b = x * (2.43 * x + 0.59) + 0.14;\n");
    s.push_str("    return clamp(a / b, 0.0, 1.0);\n");
    s.push_str("}\n\n");

    s.push_str("float dither_noise(vec2 uv) {\n");
    s.push_str("    return fract(52.9829189 * fract(dot(uv, vec2(0.06711056, 0.00583715))));\n");
    s.push_str("}\n\n");
}

/// Generate a GLSL ES 3.0 post-processing pass fragment shader.
///
/// A pass reads from a texture (previous pass output) and writes a processed result.
/// The pass pipeline operates on UV-sampled color values.
pub fn generate_pass_fragment_glsl(pass: &PassBlock) -> String {
    let mut s = String::with_capacity(2048);

    s.push_str("#version 300 es\nprecision highp float;\n\n");

    s.push_str("// Post-processing pass: ");
    s.push_str(&pass.name);
    s.push_str("\n\n");

    // Uniforms
    s.push_str("uniform float u_time;\n");
    s.push_str("uniform vec2 u_resolution;\n");
    s.push_str("uniform sampler2D u_pass_tex;\n\n");

    s.push_str("in vec2 v_uv;\nout vec4 fragColor;\n\n");

    s.push_str("void main(){\n");
    s.push_str("    vec2 uv = v_uv;\n");
    s.push_str("    vec4 pixel = texture(u_pass_tex, uv);\n");
    s.push_str("    vec4 color_result = pixel;\n\n");

    // Emit pass pipeline stages (operate on color_result)
    for stage in &pass.body {
        emit_pass_stage_glsl(&mut s, stage, "    ");
    }

    s.push_str("    fragColor = color_result;\n");
    s.push_str("}\n");

    s
}

/// Emit a GLSL post-processing stage operating on `color_result` and `pixel`.
fn emit_pass_stage_glsl(s: &mut String, stage: &Stage, indent: &str) {
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
            s.push_str(&format!("{indent}vec4 blurred = vec4(0.0);\n"));
            s.push_str(&format!("{indent}vec2 texel = 1.0 / u_resolution;\n"));
            s.push_str(&format!("{indent}int r = int({radius});\n"));
            s.push_str(&format!("{indent}float count = 0.0;\n"));
            s.push_str(&format!("{indent}for (int dy = -r; dy <= r; dy++) {{\n"));
            s.push_str(&format!(
                "{indent}    for (int dx = -r; dx <= r; dx++) {{\n"
            ));
            s.push_str(&format!(
                "{indent}        vec2 offset = vec2(float(dx), float(dy)) * texel;\n"
            ));
            s.push_str(&format!(
                "{indent}        blurred += texture(u_pass_tex, uv + offset);\n"
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
                "{indent}float lum = dot(color_result.rgb, vec3(0.299, 0.587, 0.114));\n"
            ));
            // GLSL: use ternary, NOT select()
            s.push_str(&format!(
                "{indent}color_result = (lum > {t}) ? color_result : vec4(0.0, 0.0, 0.0, 0.0);\n"
            ));
        }
        "invert" => {
            s.push_str(&format!(
                "{indent}color_result = vec4(1.0 - color_result.rgb, color_result.a);\n"
            ));
        }
        "blend_add" => {
            s.push_str(&format!(
                "{indent}color_result = vec4(min(pixel.rgb + color_result.rgb, vec3(1.0)), max(pixel.a, color_result.a));\n"
            ));
        }
        "vignette" => {
            let strength = if !args.is_empty() {
                get_arg(args, "strength", 0, "vignette")
            } else {
                "0.5".to_string()
            };
            s.push_str(&format!(
                "{indent}float vign = 1.0 - {strength} * length(uv - 0.5);\n"
            ));
            s.push_str(&format!(
                "{indent}color_result = vec4(color_result.rgb * vign, color_result.a * vign);\n"
            ));
        }
        "chromatic_aberration" => {
            let strength = if !args.is_empty() {
                get_arg(args, "strength", 0, "chromatic_aberration")
            } else {
                "0.005".to_string()
            };
            s.push_str(&format!("{indent}// chromatic aberration\n"));
            s.push_str(&format!(
                "{indent}vec2 ca_dir = normalize(uv - 0.5) * {strength};\n"
            ));
            s.push_str(&format!(
                "{indent}float ca_r = texture(u_pass_tex, uv + ca_dir).r;\n"
            ));
            s.push_str(&format!("{indent}float ca_g = color_result.g;\n"));
            s.push_str(&format!(
                "{indent}float ca_b = texture(u_pass_tex, uv - ca_dir).b;\n"
            ));
            s.push_str(&format!(
                "{indent}color_result = vec4(ca_r, ca_g, ca_b, color_result.a);\n"
            ));
        }
        "sharpen" => {
            let strength = if !args.is_empty() {
                get_arg(args, "strength", 0, "sharpen")
            } else {
                "0.5".to_string()
            };
            s.push_str(&format!("{indent}// unsharp mask sharpen\n"));
            s.push_str(&format!("{indent}vec2 sh_texel = 1.0 / u_resolution;\n"));
            s.push_str(&format!(
                "{indent}vec4 sh_n = texture(u_pass_tex, uv + vec2(0.0, sh_texel.y));\n"
            ));
            s.push_str(&format!(
                "{indent}vec4 sh_s = texture(u_pass_tex, uv - vec2(0.0, sh_texel.y));\n"
            ));
            s.push_str(&format!(
                "{indent}vec4 sh_e = texture(u_pass_tex, uv + vec2(sh_texel.x, 0.0));\n"
            ));
            s.push_str(&format!(
                "{indent}vec4 sh_w = texture(u_pass_tex, uv - vec2(sh_texel.x, 0.0));\n"
            ));
            s.push_str(&format!(
                "{indent}vec4 sh_avg = (sh_n + sh_s + sh_e + sh_w) * 0.25;\n"
            ));
            s.push_str(&format!(
                "{indent}color_result = mix(color_result, color_result + (color_result - sh_avg), {strength});\n"
            ));
        }
        "film_grain" => {
            let amount = if !args.is_empty() {
                get_arg(args, "amount", 0, "film_grain")
            } else {
                "0.05".to_string()
            };
            s.push_str(&format!("{indent}// film grain\n"));
            s.push_str(&format!(
                "{indent}vec2 grain_seed = uv * u_resolution + vec2(u_time * 1000.0, 0.0);\n"
            ));
            s.push_str(&format!(
                "{indent}float grain_val = (fract(sin(dot(grain_seed, vec2(12.9898, 78.233))) * 43758.5453) - 0.5) * {amount};\n"
            ));
            s.push_str(&format!(
                "{indent}color_result = vec4(color_result.rgb + grain_val, color_result.a);\n"
            ));
        }
        _ => {
            // Unknown pass stage — passthrough
            s.push_str(&format!("{indent}// unknown pass stage: {}\n", stage.name));
        }
    }
}

fn emit_glsl_layer(
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

    s.push_str(&format!("{indent}vec2 p = vec2(uv.x * aspect, uv.y);\n"));

    match &layer.body {
        LayerBody::Pipeline(stages) => {
            for stage in stages {
                emit_glsl_stage_with_fns(s, stage, indent, fns);
            }
        }
        LayerBody::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            s.push_str(&format!("{indent}vec4 color_result;\n"));
            s.push_str(&format!("{indent}{{\n"));
            let inner = &format!("{indent}    ");
            s.push_str(&format!("{inner}vec2 p_then = p;\n"));
            s.push_str(&format!("{inner}vec4 then_color;\n"));
            s.push_str(&format!("{inner}vec4 else_color;\n"));
            s.push_str(&format!("{inner}{{ vec2 p = p_then;\n"));
            for stage in then_branch {
                emit_glsl_stage_with_fns(s, stage, inner, fns);
            }
            s.push_str(&format!("{inner}then_color = color_result; }}\n"));
            s.push_str(&format!("{inner}{{ vec2 p = p_then;\n"));
            for stage in else_branch {
                emit_glsl_stage_with_fns(s, stage, inner, fns);
            }
            s.push_str(&format!("{inner}else_color = color_result; }}\n"));
            let cond_str = emit_glsl_expr(condition);
            s.push_str(&format!(
                "{inner}color_result = {cond_str} ? then_color : else_color;\n"
            ));
            s.push_str(&format!("{indent}}}\n"));
        }
        LayerBody::Params(_) => return,
    };

    // Memory: mix with previous frame if this layer has memory
    if let Some(decay) = layer.memory {
        memory::emit_glsl_memory_mix(s, decay, indent);
    }

    if multi {
        // Apply opacity if specified
        if let Some(opacity) = layer.opacity {
            s.push_str(&format!(
                "{indent}float la = color_result.a * {opacity:.6};\n"
            ));
            s.push_str(&format!(
                "{indent}vec3 lc = color_result.rgb * {opacity:.6};\n"
            ));
        } else {
            s.push_str(&format!("{indent}float la = color_result.a;\n"));
            s.push_str(&format!("{indent}vec3 lc = color_result.rgb;\n"));
        }
        match layer.blend {
            BlendMode::Add => {
                // Premultiplied alpha "over" compositing
                s.push_str(&format!(
                    "{indent}final_color = vec4(final_color.rgb * (1.0 - la) + lc, final_color.a * (1.0 - la) + la);\n"
                ));
            }
            BlendMode::Screen => {
                s.push_str(&format!(
                    "{indent}final_color = vec4(vec3(1.0) - (vec3(1.0) - final_color.rgb) * (vec3(1.0) - lc), max(final_color.a, la));\n"
                ));
            }
            BlendMode::Multiply => {
                s.push_str(&format!(
                    "{indent}final_color = vec4(final_color.rgb * lc, max(final_color.a, la));\n"
                ));
            }
            BlendMode::Overlay => {
                s.push_str(&format!("{indent}{{ vec3 base = final_color.rgb;\n"));
                s.push_str(&format!("{indent}vec3 lo = 2.0 * base * lc;\n"));
                s.push_str(&format!(
                    "{indent}vec3 hi = vec3(1.0) - 2.0 * (vec3(1.0) - base) * (vec3(1.0) - lc);\n"
                ));
                s.push_str(&format!(
                    "{indent}final_color = vec4(mix(hi, lo, step(base, vec3(0.5))), max(final_color.a, la)); }}\n"
                ));
            }
        }
        s.push_str("    }\n\n");
    } else {
        if has_color_matrix {
            s.push_str(&format!("{indent}color_result = vec4(apply_color_matrix(color_result.rgb), color_result.a);\n"));
        }
        // Quality output pipeline: tonemap + dither
        s.push_str(&format!(
            "{indent}color_result = vec4(aces_tonemap(color_result.rgb), color_result.a);\n"
        ));
        s.push_str(&format!(
            "{indent}color_result += (dither_noise(v_uv * u_resolution) - 0.5) / 255.0;\n"
        ));
        s.push_str(&format!("{indent}fragColor = color_result;\n"));
    }
}

/// Emit a GLSL expression string from an AST Expr.
fn emit_glsl_expr(expr: &Expr) -> String {
    match expr {
        Expr::Number(v) => format!("{v:.6}"),
        Expr::Ident(name) => match name.as_str() {
            "time" => "time".to_string(),
            "bass" => "u_audio_bass".to_string(),
            "mid" => "u_audio_mid".to_string(),
            "treble" => "u_audio_treble".to_string(),
            "energy" => "u_audio_energy".to_string(),
            "beat" => "u_audio_beat".to_string(),
            "mouse_down" => "u_mouse_down".to_string(),
            "mouse_x" => "u_mouse.x".to_string(),
            "mouse_y" => "u_mouse.y".to_string(),
            _ => name.clone(),
        },
        Expr::DottedIdent { object, field } => format!("{object}_{field}"),
        Expr::BinOp { op, left, right } => {
            let l = emit_glsl_expr(left);
            let r = emit_glsl_expr(right);
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
        Expr::Neg(inner) => format!("(-{})", emit_glsl_expr(inner)),
        Expr::Paren(inner) => format!("({})", emit_glsl_expr(inner)),
        Expr::Call { name, args } => {
            let arg_strs: Vec<String> = args.iter().map(|a| emit_glsl_expr(&a.value)).collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
        _ => "0.0".to_string(),
    }
}

fn emit_glsl_stage_with_fns(s: &mut String, stage: &Stage, indent: &str, fns: &[FnDef]) {
    if let Some(fn_def) = fns.iter().find(|f| f.name == stage.name) {
        for fn_stage in &fn_def.body {
            let substituted = substitute_fn_args(fn_stage, &fn_def.params, &stage.args);
            emit_glsl_stage(s, &substituted, indent);
        }
        return;
    }
    emit_glsl_stage(s, stage, indent);
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
            s.push_str(&format!(
                "{indent}float sdf_result = abs(length(p) - {r}) - {w};\n"
            ));
        }
        "star" => {
            let n = get_arg(args, "points", 0, "star");
            let r = get_arg(args, "radius", 1, "star");
            let ir = get_arg(args, "inner", 2, "star");
            s.push_str(&format!(
                "{indent}float sdf_result = sdf_star(p, {n}, {r}, {ir});\n"
            ));
        }
        "box" => {
            let w = get_arg(args, "width", 0, "box");
            let h = get_arg(args, "height", 1, "box");
            s.push_str(&format!(
                "{indent}float sdf_result = sdf_box(p, {w}, {h});\n"
            ));
        }
        "hex" => {
            let r = get_arg(args, "radius", 0, "hex");
            s.push_str(&format!("{indent}float sdf_result = sdf_hex(p, {r});\n"));
        }
        "glow" => {
            let intensity = get_arg(args, "intensity", 0, "glow");
            s.push_str(&format!(
                "{indent}float glow_pulse = {intensity} * (0.9 + 0.1 * sin(time * 2.0));\n"
            ));
            s.push_str(&format!(
                "{indent}float glow_result = apply_glow(sdf_result, glow_pulse);\n\n"
            ));
            s.push_str(&format!(
                "{indent}vec4 color_result = vec4(vec3(glow_result), glow_result);\n"
            ));
        }
        "tint" => {
            let r = get_arg(args, "r", 0, "tint");
            let g = get_arg(args, "g", 1, "tint");
            let b = get_arg(args, "b", 2, "tint");
            s.push_str(&format!(
                "{indent}color_result = vec4(color_result.rgb * vec3({r}, {g}, {b}), color_result.a);\n"
            ));
        }
        "bloom" => {
            let thresh = get_arg(args, "threshold", 0, "bloom");
            let strength = get_arg(args, "strength", 1, "bloom");
            // GLSL: dot returns float, NOT vec3
            s.push_str(&format!(
                "{indent}float pp_lum = dot(color_result.rgb, vec3(0.299, 0.587, 0.114));\n"
            ));
            s.push_str(&format!("{indent}color_result = vec4(color_result.rgb + max(pp_lum - {thresh}, 0.0) * {strength}, color_result.a);\n"));
        }
        "rotate" => {
            let speed = get_arg(args, "speed", 0, "rotate");
            // GLSL: use `float`, NOT `let`
            s.push_str(&format!(
                "{indent}{{ float ra = time * {speed}; float rc = cos(ra); float rs = sin(ra);\n"
            ));
            s.push_str(&format!(
                "{indent}p = vec2(p.x * rc - p.y * rs, p.x * rs + p.y * rc); }}\n"
            ));
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
            s.push_str(&format!(
                "{indent}float arc_theta = atan(p.x, p.y) + 3.14159265359;\n"
            ));
            // GLSL: use ternary, NOT select()
            s.push_str(&format!(
                "{indent}sdf_result = (arc_theta < {angle} ? sdf_result : 999.0);\n"
            ));
        }
        "shade" => {
            let r = get_arg(args, "r", 0, "shade");
            let g = get_arg(args, "g", 1, "shade");
            let b = get_arg(args, "b", 2, "shade");
            s.push_str(&format!(
                "{indent}float shade_fw = fwidth(sdf_result);
{indent}float shade_alpha = 1.0 - smoothstep(-shade_fw, shade_fw, sdf_result);
{indent}vec4 color_result = vec4(vec3({r}, {g}, {b}) * shade_alpha, shade_alpha);\n"
            ));
        }
        "emissive" => {
            let intensity = get_arg(args, "intensity", 0, "emissive");
            s.push_str(&format!(
                "{indent}float glow_result = apply_glow(sdf_result, {intensity});\n"
            ));
            s.push_str(&format!(
                "{indent}vec4 color_result = vec4(vec3(glow_result), glow_result);\n"
            ));
        }
        "fbm" => {
            let sc = get_arg(args, "scale", 0, "fbm");
            let oct = get_arg(args, "octaves", 1, "fbm");
            let pers = get_arg(args, "persistence", 2, "fbm");
            let lac = get_arg(args, "lacunarity", 3, "fbm");
            s.push_str(&format!(
                "{indent}float sdf_result = fbm2((p * {sc} + vec2(time * 0.1, time * 0.07)), int({oct}), {pers}, {lac});\n"
            ));
        }
        "grain" => {
            let amount = get_arg(args, "amount", 0, "grain");
            s.push_str(&format!("{indent}float grain_noise = fract(sin(dot(p, vec2(12.9898, 78.233)) + time) * 43758.5453);\n"));
            s.push_str(&format!("{indent}color_result = vec4(color_result.rgb + (grain_noise - 0.5) * {amount}, color_result.a);\n"));
        }
        "simplex" => {
            let sc = get_arg(args, "scale", 0, "simplex");
            s.push_str(&format!(
                "{indent}float sdf_result = noise2(p * {sc} + vec2(time * 0.1, time * 0.07));\n"
            ));
        }
        "warp" => {
            let sc = get_arg(args, "scale", 0, "warp");
            let oct = get_arg(args, "octaves", 1, "warp");
            let pers = get_arg(args, "persistence", 2, "warp");
            let lac = get_arg(args, "lacunarity", 3, "warp");
            let str_ = get_arg(args, "strength", 4, "warp");
            s.push_str(&format!(
                "{indent}{{ float warp_x = fbm2(p * {sc} + vec2(0.0, 1.3), int({oct}), {pers}, {lac});\n"
            ));
            s.push_str(&format!(
                "{indent}float warp_y = fbm2(p * {sc} + vec2(1.7, 0.0), int({oct}), {pers}, {lac});\n"
            ));
            s.push_str(&format!(
                "{indent}p = p + vec2(warp_x, warp_y) * {str_}; }}\n"
            ));
        }
        "distort" => {
            let sc = get_arg(args, "scale", 0, "distort");
            let speed = get_arg(args, "speed", 1, "distort");
            let str_ = get_arg(args, "strength", 2, "distort");
            s.push_str(&format!(
                "{indent}p = p + vec2(sin(p.y * {sc} + time * {speed}), cos(p.x * {sc} + time * {speed})) * {str_};\n"
            ));
        }
        "polar" => {
            s.push_str(&format!("{indent}p = vec2(length(p), atan(p.y, p.x));\n"));
        }
        "voronoi" => {
            let sc = get_arg(args, "scale", 0, "voronoi");
            s.push_str(&format!(
                "{indent}float sdf_result = voronoi2(p * {sc} + vec2(time * 0.05, time * 0.03));\n"
            ));
        }
        "radial_fade" => {
            let inner = get_arg(args, "inner", 0, "radial_fade");
            let outer = get_arg(args, "outer", 1, "radial_fade");
            s.push_str(&format!(
                "{indent}float sdf_result = smoothstep({inner}, {outer}, length(p));\n"
            ));
        }
        "palette" => {
            let preset = args.first().and_then(|a| {
                if let Expr::Ident(name) = &a.value {
                    named_palette_glsl(name)
                } else {
                    None
                }
            });
            if let Some((a, b, c, d)) = preset {
                s.push_str(&format!(
                    "{indent}vec3 pal_rgb = cosine_palette(sdf_result, {a}, {b}, {c}, {d});\n{indent}vec4 color_result = vec4(pal_rgb, clamp(dot(pal_rgb, vec3(0.299, 0.587, 0.114)) * 2.0, 0.0, 1.0));\n"
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
                    "{indent}vec3 pal_rgb = cosine_palette(sdf_result, vec3({a_r}, {a_g}, {a_b}), vec3({b_r}, {b_g}, {b_b}), vec3({c_r}, {c_g}, {c_b}), vec3({d_r}, {d_g}, {d_b}));\n{indent}vec4 color_result = vec4(pal_rgb, clamp(dot(pal_rgb, vec3(0.299, 0.587, 0.114)) * 2.0, 0.0, 1.0));\n"
                ));
            }
        }
        // ── SDF Morph ────────────────────────────────────
        "morph" => {
            let args = &stage.args;
            if args.len() < 3 {
                s.push_str(&format!("{indent}float sdf_result = length(p) - 0.2;\n"));
            } else {
                emit_glsl_sub_sdf(s, &args[0].value, "sdf_a", indent);
                emit_glsl_sub_sdf(s, &args[1].value, "sdf_b", indent);
                let t = emit_glsl_expr(&args[2].value);
                s.push_str(&format!(
                    "{indent}float sdf_result = mix(sdf_a, sdf_b, {t});\n"
                ));
            }
        }
        // ── SDF Boolean operations ──────────────────────
        "union" | "subtract" | "intersect" | "xor" => {
            emit_glsl_bool_op(s, stage, indent);
        }
        "smooth_union" | "smooth_subtract" | "smooth_intersect" => {
            emit_glsl_smooth_bool_op(s, stage, indent);
        }
        // ── Spatial operations ──────────────────────────
        "repeat" => {
            let sx = get_arg(args, "spacing_x", 0, "repeat");
            let sy = get_arg(args, "spacing_y", 1, "repeat");
            // GLSL mod() is floor-based, safe to use directly
            s.push_str(&format!(
                "{indent}p = vec2(mod(p.x + {sx} * 0.5, {sx}) - {sx} * 0.5, mod(p.y + {sy} * 0.5, {sy}) - {sy} * 0.5);\n"
            ));
        }
        "mirror" => {
            s.push_str(&format!("{indent}p = vec2(abs(p.x), p.y);\n"));
        }
        "radial" => {
            let count = get_arg(args, "count", 0, "radial");
            s.push_str(&format!("{indent}{{ float r_angle = atan(p.y, p.x);\n"));
            s.push_str(&format!("{indent}float r_sector = 6.28318 / {count};\n"));
            s.push_str(&format!(
                "{indent}float r_a = mod(r_angle + r_sector * 0.5, r_sector) - r_sector * 0.5;\n"
            ));
            s.push_str(&format!("{indent}float r_r = length(p);\n"));
            s.push_str(&format!(
                "{indent}p = vec2(r_r * cos(r_a), r_r * sin(r_a)); }}\n"
            ));
        }
        // ── Shape modifiers ─────────────────────────────
        "round" => {
            let r = get_arg(args, "radius", 0, "round");
            s.push_str(&format!("{indent}sdf_result -= {r};\n"));
        }
        "shell" => {
            let w = get_arg(args, "width", 0, "shell");
            s.push_str(&format!("{indent}sdf_result = abs(sdf_result) - {w};\n"));
        }
        "onion" => {
            let count = get_arg(args, "count", 0, "onion");
            let w = get_arg(args, "width", 1, "onion");
            s.push_str(&format!(
                "{indent}for (int onion_i = 0; onion_i < int({count}); onion_i++) {{ sdf_result = abs(sdf_result) - {w}; }}\n"
            ));
        }
        "outline" => {
            let w = get_arg(args, "width", 0, "outline");
            s.push_str(&format!(
                "{indent}{{ float out_lum = dot(color_result.rgb, vec3(0.299, 0.587, 0.114));\n"
            ));
            s.push_str(&format!("{indent}float out_edge = abs(out_lum) - {w};\n"));
            s.push_str(&format!("{indent}float out_fw = fwidth(out_edge);
{indent}color_result = vec4(color_result.rgb * (1.0 - smoothstep(0.0, out_fw, out_edge)), color_result.a * (1.0 - smoothstep(0.0, out_fw, out_edge))); }}\n"));
        }
        // ── New SDF primitives ──────────────────────────
        "line" => {
            let x1 = get_arg(args, "x1", 0, "line");
            let y1 = get_arg(args, "y1", 1, "line");
            let x2 = get_arg(args, "x2", 2, "line");
            let y2 = get_arg(args, "y2", 3, "line");
            let w = get_arg(args, "width", 4, "line");
            s.push_str(&format!(
                "{indent}float sdf_result = sdf_line(p, vec2({x1}, {y1}), vec2({x2}, {y2})) - {w};\n"
            ));
        }
        "capsule" => {
            let len = get_arg(args, "length", 0, "capsule");
            let r = get_arg(args, "radius", 1, "capsule");
            s.push_str(&format!(
                "{indent}float sdf_result = sdf_line(p, vec2(-{len} * 0.5, 0.0), vec2({len} * 0.5, 0.0)) - {r};\n"
            ));
        }
        "triangle" => {
            let sz = get_arg(args, "size", 0, "triangle");
            s.push_str(&format!(
                "{indent}float sdf_result = sdf_triangle(p, {sz});\n"
            ));
        }
        "arc_sdf" => {
            let r = get_arg(args, "radius", 0, "arc_sdf");
            let angle = get_arg(args, "angle", 1, "arc_sdf");
            let w = get_arg(args, "width", 2, "arc_sdf");
            s.push_str(&format!(
                "{indent}float sdf_result = sdf_arc(p, {r}, {angle}, {w});\n"
            ));
        }
        "cross" => {
            let sz = get_arg(args, "size", 0, "cross");
            let aw = get_arg(args, "arm_width", 1, "cross");
            s.push_str(&format!(
                "{indent}float sdf_result = min(sdf_box(p, {sz}, {aw}), sdf_box(p, {aw}, {sz}));\n"
            ));
        }
        "heart" => {
            let sz = get_arg(args, "size", 0, "heart");
            s.push_str(&format!("{indent}float sdf_result = sdf_heart(p, {sz});\n"));
        }
        "egg" => {
            let r = get_arg(args, "radius", 0, "egg");
            let k = get_arg(args, "k", 1, "egg");
            s.push_str(&format!(
                "{indent}float sdf_result = sdf_egg(p, {r}, {k});\n"
            ));
        }
        "spiral" => {
            let turns = get_arg(args, "turns", 0, "spiral");
            let w = get_arg(args, "width", 1, "spiral");
            s.push_str(&format!("{indent}float sp_r = length(p);\n"));
            s.push_str(&format!("{indent}float sp_a = atan(p.y, p.x);\n"));
            s.push_str(&format!(
                "{indent}float sp_d = sp_r - (sp_a + 3.14159265) / 6.28318 / {turns};\n"
            ));
            s.push_str(&format!(
                "{indent}float sdf_result = abs(sp_d - floor(sp_d + 0.5)) - {w};\n"
            ));
        }
        "grid" => {
            let spacing = get_arg(args, "spacing", 0, "grid");
            let w = get_arg(args, "width", 1, "grid");
            s.push_str(&format!("{indent}float gx = abs(mod(p.x + {spacing} * 0.5, {spacing}) - {spacing} * 0.5) - {w};\n"));
            s.push_str(&format!("{indent}float gy = abs(mod(p.y + {spacing} * 0.5, {spacing}) - {spacing} * 0.5) - {w};\n"));
            s.push_str(&format!("{indent}float sdf_result = min(gx, gy);\n"));
        }
        _ => {
            s.push_str(&format!("{indent}// Unknown stage: {}\n", stage.name));
        }
    }
}

/// Emit GLSL code for a sub-expression SDF call (used by boolean ops).
fn emit_glsl_sub_sdf(s: &mut String, expr: &Expr, var_name: &str, indent: &str) {
    if let Expr::Call { name, args } = expr {
        let sub_args: Vec<crate::ast::Arg> = args.clone();
        match name.as_str() {
            "circle" => {
                let r = get_arg(&sub_args, "radius", 0, "circle");
                s.push_str(&format!("{indent}float {var_name} = sdf_circle(p, {r});\n"));
            }
            "ring" => {
                let r = get_arg(&sub_args, "radius", 0, "ring");
                let w = get_arg(&sub_args, "width", 1, "ring");
                s.push_str(&format!(
                    "{indent}float {var_name} = abs(length(p) - {r}) - {w};\n"
                ));
            }
            "star" => {
                let n = get_arg(&sub_args, "points", 0, "star");
                let r = get_arg(&sub_args, "radius", 1, "star");
                let ir = get_arg(&sub_args, "inner", 2, "star");
                s.push_str(&format!(
                    "{indent}float {var_name} = sdf_star(p, {n}, {r}, {ir});\n"
                ));
            }
            "box" => {
                let w = get_arg(&sub_args, "width", 0, "box");
                let h = get_arg(&sub_args, "height", 1, "box");
                s.push_str(&format!(
                    "{indent}float {var_name} = sdf_box(p, {w}, {h});\n"
                ));
            }
            "hex" => {
                let r = get_arg(&sub_args, "radius", 0, "hex");
                s.push_str(&format!("{indent}float {var_name} = sdf_hex(p, {r});\n"));
            }
            "line" => {
                let x1 = get_arg(&sub_args, "x1", 0, "line");
                let y1 = get_arg(&sub_args, "y1", 1, "line");
                let x2 = get_arg(&sub_args, "x2", 2, "line");
                let y2 = get_arg(&sub_args, "y2", 3, "line");
                let w = get_arg(&sub_args, "width", 4, "line");
                s.push_str(&format!(
                    "{indent}float {var_name} = sdf_line(p, vec2({x1}, {y1}), vec2({x2}, {y2})) - {w};\n"
                ));
            }
            "capsule" => {
                let len = get_arg(&sub_args, "length", 0, "capsule");
                let r = get_arg(&sub_args, "radius", 1, "capsule");
                s.push_str(&format!(
                    "{indent}float {var_name} = sdf_line(p, vec2(-{len} * 0.5, 0.0), vec2({len} * 0.5, 0.0)) - {r};\n"
                ));
            }
            "triangle" => {
                let sz = get_arg(&sub_args, "size", 0, "triangle");
                s.push_str(&format!(
                    "{indent}float {var_name} = sdf_triangle(p, {sz});\n"
                ));
            }
            "heart" => {
                let sz = get_arg(&sub_args, "size", 0, "heart");
                s.push_str(&format!("{indent}float {var_name} = sdf_heart(p, {sz});\n"));
            }
            "egg" => {
                let r = get_arg(&sub_args, "radius", 0, "egg");
                let k = get_arg(&sub_args, "k", 1, "egg");
                s.push_str(&format!(
                    "{indent}float {var_name} = sdf_egg(p, {r}, {k});\n"
                ));
            }
            _ => {
                s.push_str(&format!(
                    "{indent}float {var_name} = length(p) - 0.2; // fallback\n"
                ));
            }
        }
    }
}

/// Emit GLSL code for a boolean SDF operation.
fn emit_glsl_bool_op(s: &mut String, stage: &Stage, indent: &str) {
    let args = &stage.args;
    if args.len() < 2 {
        s.push_str(&format!("{indent}float sdf_result = length(p) - 0.2;\n"));
        return;
    }
    emit_glsl_sub_sdf(s, &args[0].value, "sdf_a", indent);
    emit_glsl_sub_sdf(s, &args[1].value, "sdf_b", indent);
    let op = match stage.name.as_str() {
        "union" => "min(sdf_a, sdf_b)",
        "subtract" => "max(sdf_a, -sdf_b)",
        "intersect" => "max(sdf_a, sdf_b)",
        "xor" => "max(min(sdf_a, sdf_b), -max(sdf_a, sdf_b))",
        _ => "min(sdf_a, sdf_b)",
    };
    s.push_str(&format!("{indent}float sdf_result = {op};\n"));
}

/// Emit GLSL code for a smooth boolean SDF operation.
fn emit_glsl_smooth_bool_op(s: &mut String, stage: &Stage, indent: &str) {
    let args = &stage.args;
    if args.len() < 2 {
        s.push_str(&format!("{indent}float sdf_result = length(p) - 0.2;\n"));
        return;
    }
    emit_glsl_sub_sdf(s, &args[0].value, "sdf_a", indent);
    emit_glsl_sub_sdf(s, &args[1].value, "sdf_b", indent);
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
    s.push_str(&format!("{indent}float sdf_result = {op};\n"));
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
    fn glsl_has_void_main() {
        let cin = make_cinematic(vec![
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
        assert!(
            output.contains("void main()"),
            "must use void main(), got:\n{output}"
        );
        assert!(!output.contains("fs_main"), "must NOT contain fs_main");
    }

    #[test]
    fn glsl_has_c_style_params() {
        let cin = make_cinematic(vec![
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
        assert!(
            output.contains("float sdf_circle(vec2 p, float radius)"),
            "C-style params"
        );
        assert!(
            output.contains("float apply_glow(float d, float intensity)"),
            "C-style params"
        );
        assert!(
            !output.contains("p: vec2"),
            "must NOT have WGSL-style params"
        );
    }

    #[test]
    fn glsl_uses_fragcolor_not_return() {
        let cin = make_cinematic(vec![
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
        assert!(output.contains("fragColor = "), "must assign fragColor");
    }

    #[test]
    fn glsl_uses_float_not_let() {
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
                name: "tint".into(),
                args: vec![],
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        // Main body should use `float` and `vec2`, not `let`
        assert!(
            !output.contains("\n    let "),
            "must NOT use `let` in GLSL body"
        );
        assert!(output.contains("vec2 uv = "), "must use typed declarations");
    }

    #[test]
    fn glsl_bloom_uses_float_lum() {
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
                name: "bloom".into(),
                args: vec![],
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(
            output.contains("float pp_lum = dot("),
            "dot() must return float"
        );
        assert!(!output.contains("vec3 pp_lum"), "must NOT use vec3 for lum");
    }

    #[test]
    fn glsl_mask_arc_uses_ternary() {
        let cin = make_cinematic(vec![
            Stage {
                name: "ring".into(),
                args: vec![],
            },
            Stage {
                name: "mask_arc".into(),
                args: vec![Arg {
                    name: None,
                    value: Expr::Number(4.0),
                }],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(output.contains("? sdf_result : 999.0"), "must use ternary");
        assert!(!output.contains("select("), "must NOT use select()");
    }

    #[test]
    fn glsl_rotate_uses_float_not_let() {
        let cin = make_cinematic(vec![
            Stage {
                name: "rotate".into(),
                args: vec![Arg {
                    name: None,
                    value: Expr::Number(1.0),
                }],
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
        assert!(
            output.contains("float rc = cos("),
            "must use float, not let"
        );
        assert!(
            output.contains("float rs = sin("),
            "must use float, not let"
        );
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
                    name: "a".into(),
                    opts: vec![],
                    memory: None,
                    opacity: None,
                    cast: None,
                    blend: BlendMode::Add,
                    feedback: false,
                    body: LayerBody::Pipeline(vec![
                        Stage {
                            name: "circle".into(),
                            args: vec![],
                        },
                        Stage {
                            name: "glow".into(),
                            args: vec![],
                        },
                    ]),
                },
                Layer {
                    name: "b".into(),
                    opts: vec![],
                    memory: None,
                    opacity: None,
                    cast: None,
                    blend: BlendMode::Add,
                    feedback: false,
                    body: LayerBody::Pipeline(vec![
                        Stage {
                            name: "ring".into(),
                            args: vec![],
                        },
                        Stage {
                            name: "glow".into(),
                            args: vec![],
                        },
                    ]),
                },
            ],
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
        assert!(output.contains("vec4 final_color"));
        assert!(output.contains("fragColor = final_color"));
        assert!(
            !output.contains("return final_color"),
            "GLSL must NOT return in void main"
        );
    }

    #[test]
    fn glsl_warp_voronoi_palette_pipeline() {
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
        assert!(
            output.contains("float voronoi2(vec2 p)"),
            "GLSL voronoi helper"
        );
        assert!(
            output.contains("vec3 cosine_palette(float t"),
            "GLSL palette helper"
        );
        assert!(output.contains("float fbm2(vec2 p"), "fbm helper for warp");
        assert!(output.contains("warp_x"), "warp displacement");
        assert!(output.contains("voronoi2(p *"), "voronoi stage");
        assert!(
            output.contains("cosine_palette(sdf_result"),
            "palette stage"
        );
        assert!(!output.contains("vec2<f32>"), "must NOT have WGSL types");
    }

    #[test]
    fn glsl_distort_uses_float() {
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
        assert!(output.contains("sin(p.y *"), "distort sin");
        assert!(output.contains("smoothstep("), "radial_fade");
        assert!(!output.contains("let "), "no WGSL let in GLSL body");
    }

    #[test]
    fn glsl_polar_uses_atan() {
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
        assert!(
            output.contains("atan(p.y, p.x)"),
            "GLSL uses atan not atan2"
        );
        assert!(output.contains("noise2(p *"), "simplex noise");
    }

    #[test]
    fn glsl_fbm_correct_types() {
        let cin = make_cinematic(vec![
            Stage {
                name: "fbm".into(),
                args: vec![],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
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
        assert!(
            output.contains("float fbm2(vec2 p, int octaves, float persistence, float lacunarity)")
        );
        assert!(
            !output.contains("float value: float"),
            "no colon syntax in GLSL"
        );
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
    fn glsl_screen_blend_emits_formula() {
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
            output.contains("vec3(1.0) - (vec3(1.0)"),
            "GLSL screen blend: {output}"
        );
    }

    #[test]
    fn glsl_multiply_blend_emits_formula() {
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
            "GLSL multiply blend: {output}"
        );
    }

    #[test]
    fn glsl_overlay_blend_emits_mix_step() {
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
            output.contains("mix(hi, lo, step("),
            "GLSL overlay uses mix+step: {output}"
        );
    }

    #[test]
    fn glsl_union_emits_min() {
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
        assert!(!output.contains("vec2<f32>"), "must NOT have WGSL types");
    }

    #[test]
    fn glsl_smooth_union_emits_smin() {
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
        assert!(output.contains("float smin("), "GLSL smin helper");
        assert!(
            output.contains("smin(sdf_a, sdf_b,"),
            "smooth union uses smin"
        );
    }

    #[test]
    fn glsl_repeat_uses_mod() {
        let cin = make_cinematic(vec![
            Stage {
                name: "repeat".into(),
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
        assert!(output.contains("mod(p.x"), "GLSL repeat uses mod()");
    }

    #[test]
    fn glsl_new_primitives() {
        let cin = make_cinematic(vec![
            Stage {
                name: "triangle".into(),
                args: vec![],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output = generate_fragment(&cin, &[]);
        assert!(
            output.contains("float sdf_triangle(vec2 p"),
            "GLSL C-style triangle helper"
        );

        let cin2 = make_cinematic(vec![
            Stage {
                name: "spiral".into(),
                args: vec![],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output2 = generate_fragment(&cin2, &[]);
        assert!(
            output2.contains("float sp_r = length(p)"),
            "spiral code emitted"
        );
    }

    // ── Mouse interaction tests ─────────────────────────────

    #[test]
    fn glsl_mouse_uniforms() {
        let cin = make_cinematic(vec![
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
        assert!(
            output.contains("uniform vec2 u_mouse;"),
            "must declare u_mouse uniform, got:\n{output}"
        );
        assert!(
            output.contains("uniform float u_mouse_down;"),
            "must declare u_mouse_down uniform, got:\n{output}"
        );
    }

    #[test]
    fn glsl_mouse_alias_variables() {
        let cin = make_cinematic(vec![
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
        assert!(
            output.contains("float mouse_x = u_mouse.x;"),
            "must declare mouse_x alias, got:\n{output}"
        );
        assert!(
            output.contains("float mouse_y = u_mouse.y;"),
            "must declare mouse_y alias, got:\n{output}"
        );
        assert!(
            output.contains("float mouse_down = u_mouse_down;"),
            "must declare mouse_down alias, got:\n{output}"
        );
    }
}
