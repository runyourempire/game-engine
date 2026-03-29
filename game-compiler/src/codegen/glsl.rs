//! GLSL ES 3.0 shader generation from GAME AST.
//!
//! CRITICAL: This module produces correct GLSL, NOT WGSL syntax.
//! Key differences from WGSL:
//!   - Function params: `float func(vec2 p, float r)` NOT `func(p: vec2, r: float)`
//!   - Entry point: `void main()` NOT `void fs_main(input: VertexOutput)`
//!   - Variables: `float x = ...;` NOT `let x = ...;`
//!   - Output: `fragColor = ...;` NOT `return ...;`
//!   - No `select()` — use ternary `? :`
//!   - `mod(x, y)` for float modulo, NOT `%`
//!   - `atan(y, x)` for atan2
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
    let mut s = String::with_capacity(8192);

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
    let needs_noise = cinematic.layers.iter().any(|l| {
        has_stage(l, "fbm")
            || has_stage(l, "domain_warp")
            || has_stage(l, "curl_noise")
            || has_stage(l, "displace")
            || has_stage(l, "voronoi")
    });
    let needs_fbm = cinematic.layers.iter().any(|l| has_stage(l, "fbm"));
    let needs_simplex = cinematic.layers.iter().any(|l| has_stage(l, "simplex"));
    let needs_voronoi = cinematic.layers.iter().any(|l| has_stage(l, "voronoi"));

    // C-style function declarations — NOT WGSL style
    if needs_circle {
        s.push_str("float sdf_circle(vec2 p, float radius){\n");
        s.push_str("    return length(p) - radius;\n");
        s.push_str("}\n\n");
    }

    s.push_str("float apply_glow(float d, float intensity){\n");
    s.push_str("    float edge = 0.005;\n");
    s.push_str("    float core = smoothstep(edge, -edge, d);\n");
    s.push_str("    float halo = intensity / (1.0 + max(d, 0.0) * max(d, 0.0) * intensity * intensity * 16.0);\n");
    s.push_str("    return core + halo;\n");
    s.push_str("}\n\n");

    // Noise helpers — conditionally emitted
    if needs_noise || needs_fbm {
        emit_glsl_noise_helpers(s);
    }

    if needs_fbm {
        emit_glsl_fbm(s);
    }

    if needs_simplex {
        emit_glsl_simplex(s);
    }

    if needs_voronoi {
        emit_glsl_voronoi(s);
    }
}

fn emit_glsl_noise_helpers(s: &mut String) {
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
}

fn emit_glsl_fbm(s: &mut String) {
    // fbm2: C-style params with domain rotation
    s.push_str("float fbm2(vec2 p, int octaves, float persistence, float lacunarity){\n");
    s.push_str("    float value = 0.0;\n");
    s.push_str("    float amplitude = 1.0;\n");
    s.push_str("    float frequency = 1.0;\n");
    s.push_str("    float max_val = 0.0;\n");
    s.push_str("    vec2 q = p;\n");
    s.push_str("    for (int i = 0; i < octaves; i++) {\n");
    s.push_str("        value += noise2(q * frequency) * amplitude;\n");
    s.push_str("        max_val += amplitude;\n");
    s.push_str("        amplitude *= persistence;\n");
    s.push_str("        frequency *= lacunarity;\n");
    s.push_str("        q = vec2(q.x * 0.8 - q.y * 0.6, q.x * 0.6 + q.y * 0.8);\n");
    s.push_str("    }\n");
    s.push_str("    return value / max_val;\n");
    s.push_str("}\n\n");
}

fn emit_glsl_simplex(s: &mut String) {
    s.push_str("vec3 simplex_permute(vec3 x){\n");
    s.push_str("    return mod(((x * 34.0) + 1.0) * x, 289.0);\n");
    s.push_str("}\n\n");
    s.push_str("float simplex2d(vec2 v){\n");
    s.push_str("    vec4 C = vec4(0.211324865405187, 0.366025403784439,\n");
    s.push_str("                  -0.577350269189626, 0.024390243902439);\n");
    s.push_str("    vec2 i = floor(v + dot(v, C.yy));\n");
    s.push_str("    vec2 x0 = v - i + dot(i, C.xx);\n");
    s.push_str("    vec2 i1 = (x0.x > x0.y) ? vec2(1.0, 0.0) : vec2(0.0, 1.0);\n");
    s.push_str("    vec4 x12 = x0.xyxy + C.xxzz;\n");
    s.push_str("    x12.xy -= i1;\n");
    s.push_str("    i = mod(i, 289.0);\n");
    s.push_str("    vec3 pm = simplex_permute(simplex_permute(i.y + vec3(0.0, i1.y, 1.0)) + i.x + vec3(0.0, i1.x, 1.0));\n");
    s.push_str("    vec3 m = max(0.5 - vec3(dot(x0, x0), dot(x12.xy, x12.xy), dot(x12.zw, x12.zw)), 0.0);\n");
    s.push_str("    m = m * m;\n");
    s.push_str("    m = m * m;\n");
    s.push_str("    vec3 x_vals = 2.0 * fract(pm * C.www) - 1.0;\n");
    s.push_str("    vec3 h = abs(x_vals) - 0.5;\n");
    s.push_str("    vec3 ox = floor(x_vals + 0.5);\n");
    s.push_str("    vec3 a0 = x_vals - ox;\n");
    s.push_str("    m *= 1.79284291400159 - 0.85373472095314 * (a0 * a0 + h * h);\n");
    s.push_str("    vec3 g;\n");
    s.push_str("    g.x = a0.x * x0.x + h.x * x0.y;\n");
    s.push_str("    g.yz = a0.yz * x12.xz + h.yz * x12.yw;\n");
    s.push_str("    return 130.0 * dot(m, g);\n");
    s.push_str("}\n\n");
}

fn emit_glsl_voronoi(s: &mut String) {
    s.push_str("float voronoi2d(vec2 p, float scale){\n");
    s.push_str("    vec2 ps = p * scale;\n");
    s.push_str("    vec2 ip = floor(ps);\n");
    s.push_str("    vec2 fp = fract(ps);\n");
    s.push_str("    float d = 1.0;\n");
    s.push_str("    for (int y = -1; y <= 1; y++) {\n");
    s.push_str("        for (int x = -1; x <= 1; x++) {\n");
    s.push_str("            vec2 neighbor = vec2(float(x), float(y));\n");
    s.push_str("            vec2 cell = ip + neighbor;\n");
    s.push_str("            vec2 pt = neighbor + fract(sin(vec2(\n");
    s.push_str("                dot(cell, vec2(127.1, 311.7)),\n");
    s.push_str("                dot(cell, vec2(269.5, 183.3))\n");
    s.push_str("            )) * 43758.5453) - fp;\n");
    s.push_str("            d = min(d, dot(pt, pt));\n");
    s.push_str("        }\n");
    s.push_str("    }\n");
    s.push_str("    return sqrt(d);\n");
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
        // Screen blend instead of pure additive
        s.push_str(&format!("{indent}vec3 lc = color_result.rgb;\n"));
        s.push_str(&format!("{indent}final_color = vec4(final_color.rgb + lc - final_color.rgb * lc, 1.0);\n"));
        s.push_str("    }\n\n");
    } else {
        s.push_str(&format!("{indent}fragColor = color_result;\n"));
    }
}

fn emit_glsl_stage(s: &mut String, stage: &Stage, indent: &str) {
    let args = &stage.args;
    match stage.name.as_str() {
        // ── SDF Generators: Position -> Sdf ──────────────────
        "circle" => {
            let r = get_arg(args, "radius", 0, "circle");
            s.push_str(&format!("{indent}float sdf_result = sdf_circle(p, {r});\n"));
        }
        "ring" => {
            let r = get_arg(args, "radius", 0, "ring");
            let w = get_arg(args, "width", 1, "ring");
            s.push_str(&format!("{indent}float sdf_result = abs(length(p) - {r}) - {w};\n"));
        }
        "star" => {
            let points = get_arg(args, "points", 0, "star");
            let radius = get_arg(args, "radius", 1, "star");
            let inner = get_arg(args, "inner", 2, "star");
            // Star SDF via angular repetition
            s.push_str(&format!("{indent}{{ float star_an = 3.14159265359 / {points};\n"));
            s.push_str(&format!("{indent}float star_a = atan(p.y, p.x);\n"));
            s.push_str(&format!("{indent}float star_sector = mod(star_a + star_an, 2.0 * star_an) - star_an;\n"));
            s.push_str(&format!("{indent}vec2 star_p = vec2(cos(star_sector), abs(sin(star_sector))) * length(p);\n"));
            s.push_str(&format!("{indent}vec2 star_a_pt = vec2({radius}, 0.0);\n"));
            s.push_str(&format!("{indent}vec2 star_b_pt = vec2({inner} * cos(star_an), {inner} * sin(star_an));\n"));
            s.push_str(&format!("{indent}vec2 star_ba = star_b_pt - star_a_pt;\n"));
            s.push_str(&format!("{indent}float star_t = clamp(dot(star_p - star_a_pt, star_ba) / dot(star_ba, star_ba), 0.0, 1.0);\n"));
            s.push_str(&format!("{indent}float sdf_result = length(star_p - star_a_pt - star_ba * star_t); }}\n"));
        }
        "box" => {
            let w = get_arg(args, "w", 0, "box");
            let h = get_arg(args, "h", 1, "box");
            s.push_str(&format!("{indent}vec2 box_d = abs(p) - vec2({w}, {h});\n"));
            s.push_str(&format!("{indent}float sdf_result = length(max(box_d, 0.0)) + min(max(box_d.x, box_d.y), 0.0);\n"));
        }
        "polygon" => {
            let sides = get_arg(args, "sides", 0, "polygon");
            let radius = get_arg(args, "radius", 1, "polygon");
            s.push_str(&format!("{indent}{{ float poly_n = {sides};\n"));
            s.push_str(&format!("{indent}float poly_a = atan(p.y, p.x);\n"));
            s.push_str(&format!("{indent}float poly_r = length(p);\n"));
            s.push_str(&format!("{indent}float poly_an = 6.28318530718 / poly_n;\n"));
            s.push_str(&format!("{indent}float poly_sector = cos(floor(0.5 + poly_a / poly_an) * poly_an - poly_a) * poly_r;\n"));
            s.push_str(&format!("{indent}float sdf_result = poly_sector - {radius}; }}\n"));
        }
        "simplex" => {
            let sc = get_arg(args, "scale", 0, "simplex");
            s.push_str(&format!("{indent}float sdf_result = simplex2d(p * {sc});\n"));
        }
        "voronoi" => {
            let sc = get_arg(args, "scale", 0, "voronoi");
            s.push_str(&format!("{indent}float sdf_result = voronoi2d(p, {sc});\n"));
        }
        "concentric_waves" => {
            let amplitude = get_arg(args, "amplitude", 0, "concentric_waves");
            let width = get_arg(args, "width", 1, "concentric_waves");
            let freq = get_arg(args, "frequency", 2, "concentric_waves");
            s.push_str(&format!("{indent}float cw_dist = length(p);\n"));
            s.push_str(&format!("{indent}float sdf_result = {amplitude} * sin(cw_dist * {freq} * 6.28318530718 - time * {width});\n"));
        }
        "fbm" => {
            let sc = get_arg(args, "scale", 0, "fbm");
            let oct = get_arg(args, "octaves", 1, "fbm");
            let pers = get_arg(args, "persistence", 2, "fbm");
            let lac = get_arg(args, "lacunarity", 3, "fbm");
            s.push_str(&format!("{indent}float sdf_result = fbm2((p * {sc}), int({oct}), {pers}, {lac});\n"));
        }

        // ── Transforms: Position -> Position ─────────────────
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
        "twist" => {
            let amount = get_arg(args, "amount", 0, "twist");
            s.push_str(&format!("{indent}{{ float tw_a = p.y * {amount};\n"));
            s.push_str(&format!("{indent}float tw_c = cos(tw_a); float tw_s = sin(tw_a);\n"));
            s.push_str(&format!("{indent}p = vec2(p.x * tw_c - p.y * tw_s, p.x * tw_s + p.y * tw_c); }}\n"));
        }
        "mirror" => {
            let axis = get_arg(args, "axis", 0, "mirror");
            // axis 0 = mirror X, else mirror Y
            s.push_str(&format!("{indent}p = ({axis} < 0.5) ? vec2(abs(p.x), p.y) : vec2(p.x, abs(p.y));\n"));
        }
        "repeat" => {
            let count = get_arg(args, "count", 0, "repeat");
            s.push_str(&format!("{indent}{{ float rep_angle = 6.28318530718 / {count};\n"));
            s.push_str(&format!("{indent}float rep_a = atan(p.y, p.x);\n"));
            s.push_str(&format!("{indent}rep_a = mod(rep_a + rep_angle * 0.5, rep_angle) - rep_angle * 0.5;\n"));
            s.push_str(&format!("{indent}p = vec2(cos(rep_a), sin(rep_a)) * length(p); }}\n"));
        }
        "domain_warp" => {
            let amount = get_arg(args, "amount", 0, "domain_warp");
            let freq = get_arg(args, "freq", 1, "domain_warp");
            s.push_str(&format!("{indent}p = p + vec2(noise2(p * {freq}), noise2(p * {freq} + vec2(5.2, 1.3))) * {amount};\n"));
        }
        "curl_noise" => {
            let frequency = get_arg(args, "frequency", 0, "curl_noise");
            let amplitude = get_arg(args, "amplitude", 1, "curl_noise");
            s.push_str(&format!("{indent}{{ float cn_eps = 0.001;\n"));
            s.push_str(&format!("{indent}float cn_n0 = noise2(p * {frequency} + vec2(0.0, cn_eps));\n"));
            s.push_str(&format!("{indent}float cn_n1 = noise2(p * {frequency} - vec2(0.0, cn_eps));\n"));
            s.push_str(&format!("{indent}float cn_n2 = noise2(p * {frequency} + vec2(cn_eps, 0.0));\n"));
            s.push_str(&format!("{indent}float cn_n3 = noise2(p * {frequency} - vec2(cn_eps, 0.0));\n"));
            s.push_str(&format!("{indent}vec2 cn_curl = vec2((cn_n0 - cn_n1) / (2.0 * cn_eps), -(cn_n2 - cn_n3) / (2.0 * cn_eps));\n"));
            s.push_str(&format!("{indent}p = p + normalize(cn_curl) * {amplitude}; }}\n"));
        }
        "displace" => {
            let strength = get_arg(args, "strength", 0, "displace");
            s.push_str(&format!("{indent}p = p + vec2(noise2(p * 3.0 + time * 0.5), noise2(p * 3.0 + vec2(5.2, 1.3) + time * 0.5)) * {strength};\n"));
        }

        // ── SDF Modifiers: Sdf -> Sdf ────────────────────────
        "mask_arc" => {
            let angle = get_arg(args, "angle", 0, "mask_arc");
            s.push_str(&format!("{indent}float arc_theta = atan(p.x, p.y) + 3.14159265359;\n"));
            // GLSL: use ternary, NOT select()
            s.push_str(&format!("{indent}sdf_result = (arc_theta < {angle} ? sdf_result : 999.0);\n"));
        }
        "threshold" => {
            let cutoff = get_arg(args, "cutoff", 0, "threshold");
            s.push_str(&format!("{indent}sdf_result = (sdf_result < {cutoff}) ? -1.0 : 1.0;\n"));
        }
        "onion" => {
            let thickness = get_arg(args, "thickness", 0, "onion");
            s.push_str(&format!("{indent}sdf_result = abs(sdf_result) - {thickness};\n"));
        }
        "round" => {
            let radius = get_arg(args, "radius", 0, "round");
            s.push_str(&format!("{indent}sdf_result = sdf_result - {radius};\n"));
        }

        // ── Bridges: Sdf -> Color ────────────────────────────
        "glow" => {
            let intensity = get_arg(args, "intensity", 0, "glow");
            s.push_str(&format!("{indent}float glow_result = apply_glow(sdf_result, {intensity});\n\n"));
            s.push_str(&format!("{indent}vec4 color_result = vec4(vec3(glow_result), 1.0);\n"));
        }
        "shade" => {
            let r = get_arg(args, "r", 0, "shade");
            let g = get_arg(args, "g", 1, "shade");
            let b = get_arg(args, "b", 2, "shade");
            // Anti-aliased shade with fwidth-based smoothstep
            s.push_str(&format!("{indent}float aa = 0.005;\n"));
            s.push_str(&format!("{indent}vec4 color_result = vec4(vec3({r}, {g}, {b}) * smoothstep(aa, -aa, sdf_result), 1.0);\n"));
        }
        "emissive" => {
            let intensity = get_arg(args, "intensity", 0, "emissive");
            s.push_str(&format!("{indent}float glow_result = apply_glow(sdf_result, {intensity});\n"));
            s.push_str(&format!("{indent}vec4 color_result = vec4(vec3(glow_result), glow_result);\n"));
        }

        // ── Color Processors: Color -> Color ─────────────────
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
        "grain" => {
            let amount = get_arg(args, "amount", 0, "grain");
            s.push_str(&format!("{indent}float grain_noise = fract(sin(dot(p, vec2(12.9898, 78.233)) + time) * 43758.5453);\n"));
            s.push_str(&format!("{indent}color_result = vec4(color_result.rgb + (grain_noise - 0.5) * {amount}, color_result.a);\n"));
        }
        "vignette" => {
            let strength = get_arg(args, "strength", 0, "vignette");
            let radius = get_arg(args, "radius", 1, "vignette");
            s.push_str(&format!("{indent}float vig_dist = length(uv);\n"));
            s.push_str(&format!("{indent}float vig_factor = 1.0 - smoothstep({radius}, {radius} + {strength}, vig_dist);\n"));
            s.push_str(&format!("{indent}color_result = vec4(color_result.rgb * vig_factor, color_result.a);\n"));
        }
        "chromatic" => {
            let offset = get_arg(args, "offset", 0, "chromatic");
            s.push_str(&format!("{indent}{{ vec2 chr_dir = normalize(uv) * {offset};\n"));
            s.push_str(&format!("{indent}float chr_r = color_result.r;\n"));
            s.push_str(&format!("{indent}float chr_b = color_result.b;\n"));
            // Shift the color channels in opposite directions
            s.push_str(&format!("{indent}color_result = vec4(chr_r * (1.0 + chr_dir.x), color_result.g, chr_b * (1.0 - chr_dir.x), color_result.a); }}\n"));
        }
        "tonemap" => {
            let exposure = get_arg(args, "exposure", 0, "tonemap");
            // Reinhard tonemap
            s.push_str(&format!("{indent}{{ vec3 tm_c = color_result.rgb * {exposure};\n"));
            s.push_str(&format!("{indent}color_result = vec4(tm_c / (tm_c + vec3(1.0)), color_result.a); }}\n"));
        }
        "scanlines" => {
            let frequency = get_arg(args, "frequency", 0, "scanlines");
            let intensity = get_arg(args, "intensity", 1, "scanlines");
            s.push_str(&format!("{indent}float scan_val = sin(uv.y * {frequency} * 3.14159265359) * {intensity};\n"));
            s.push_str(&format!("{indent}color_result = vec4(color_result.rgb * (1.0 - scan_val * 0.5), color_result.a);\n"));
        }
        "saturate_color" => {
            let amount = get_arg(args, "amount", 0, "saturate_color");
            s.push_str(&format!("{indent}{{ float sat_lum = dot(color_result.rgb, vec3(0.299, 0.587, 0.114));\n"));
            s.push_str(&format!("{indent}color_result = vec4(mix(vec3(sat_lum), color_result.rgb, {amount}), color_result.a); }}\n"));
        }
        "glitch" => {
            let intensity = get_arg(args, "intensity", 0, "glitch");
            s.push_str(&format!("{indent}{{ float gl_t = floor(time * 10.0);\n"));
            s.push_str(&format!("{indent}float gl_noise = fract(sin(gl_t * 43758.5453 + p.y * 100.0) * 43758.5453);\n"));
            s.push_str(&format!("{indent}float gl_shift = (gl_noise - 0.5) * {intensity} * 0.1;\n"));
            s.push_str(&format!("{indent}float gl_block = step(1.0 - {intensity} * 0.3, fract(sin(gl_t * 12.9898 + floor(p.y * 20.0)) * 43758.5453));\n"));
            s.push_str(&format!("{indent}color_result = vec4(\n"));
            s.push_str(&format!("{indent}    color_result.r + gl_shift * gl_block,\n"));
            s.push_str(&format!("{indent}    color_result.g,\n"));
            s.push_str(&format!("{indent}    color_result.b - gl_shift * gl_block,\n"));
            s.push_str(&format!("{indent}    color_result.a); }}\n"));
        }
        "blend" => {
            let factor = get_arg(args, "factor", 0, "blend");
            s.push_str(&format!("{indent}color_result = vec4(color_result.rgb * {factor}, color_result.a);\n"));
        }

        // ── Full-screen Generators: Position -> Color ────────
        "gradient" => {
            let _color_a = get_arg(args, "color_a", 0, "gradient");
            let _color_b = get_arg(args, "color_b", 1, "gradient");
            let _mode = get_arg(args, "mode", 2, "gradient");
            // Default vertical gradient: black at bottom, white at top
            s.push_str(&format!("{indent}float grad_t = uv.y * 0.5 + 0.5;\n"));
            s.push_str(&format!("{indent}vec4 color_result = vec4(vec3(grad_t), 1.0);\n"));
        }
        "spectrum" => {
            let bass = get_arg(args, "bass", 0, "spectrum");
            let mid = get_arg(args, "mid", 1, "spectrum");
            let treble = get_arg(args, "treble", 2, "spectrum");
            s.push_str(&format!("{indent}float spec_x = uv.x * 0.5 + 0.5;\n"));
            s.push_str(&format!("{indent}float spec_r = smoothstep(0.0, 0.33, spec_x) * (u_audio_bass + {bass});\n"));
            s.push_str(&format!("{indent}float spec_g = smoothstep(0.33, 0.66, spec_x) * (u_audio_mid + {mid});\n"));
            s.push_str(&format!("{indent}float spec_b = smoothstep(0.66, 1.0, spec_x) * (u_audio_treble + {treble});\n"));
            s.push_str(&format!("{indent}vec4 color_result = vec4(spec_r, spec_g, spec_b, 1.0);\n"));
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
            listen: None, voice: None, score: None, gravity: None,
            lenses: vec![], react: None, defines: vec![],
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
            listen: None, voice: None, score: None, gravity: None,
            lenses: vec![], react: None, defines: vec![],
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
