//! WGSL shader generation from GAME AST.

use crate::ast::*;
use crate::codegen::memory;
use crate::codegen::stages::get_arg;
use crate::codegen::UniformInfo;

/// Generate a WGSL fragment shader for a cinematic.
pub fn generate_fragment(
    cinematic: &Cinematic,
    uniforms: &[UniformInfo],
) -> String {
    let mut s = String::with_capacity(8192);

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

// ── Helper function detection ───────────────────────────────────

fn needs_noise_helpers(cinematic: &Cinematic) -> bool {
    cinematic.layers.iter().any(|l| {
        has_stage(l, "fbm")
            || has_stage(l, "domain_warp")
            || has_stage(l, "curl_noise")
            || has_stage(l, "displace")
    })
}

fn needs_voronoi_helper(cinematic: &Cinematic) -> bool {
    cinematic.layers.iter().any(|l| has_stage(l, "voronoi"))
}

fn needs_simplex_helper(cinematic: &Cinematic) -> bool {
    cinematic.layers.iter().any(|l| has_stage(l, "simplex"))
}

// ── Built-in helper functions ───────────────────────────────────

fn emit_wgsl_builtins(s: &mut String, cinematic: &Cinematic) {
    let needs_circle = cinematic.layers.iter().any(|l| has_stage(l, "circle"));
    let needs_noise = needs_noise_helpers(cinematic);
    let needs_voronoi = needs_voronoi_helper(cinematic);
    let needs_simplex = needs_simplex_helper(cinematic);

    // SDF circle helper
    if needs_circle {
        s.push_str("fn sdf_circle(p: vec2<f32>, radius: f32) -> f32 {\n");
        s.push_str("    return length(p) - radius;\n");
        s.push_str("}\n\n");
    }

    // Modern glow: inverse-square falloff with soft core
    s.push_str("fn apply_glow(d: f32, intensity: f32) -> f32 {\n");
    s.push_str("    let edge = 0.005;\n");
    s.push_str("    let core = smoothstep(edge, -edge, d);\n");
    s.push_str("    let halo = intensity / (1.0 + max(d, 0.0) * max(d, 0.0) * intensity * intensity * 16.0);\n");
    s.push_str("    return core + halo;\n");
    s.push_str("}\n\n");

    // Noise helpers (hash2, noise2, fbm2) — shared by fbm, domain_warp, curl_noise, displace
    if needs_noise || needs_voronoi {
        emit_wgsl_noise_helpers(s);
    }

    // FBM with domain rotation
    if needs_noise {
        emit_wgsl_fbm(s);
    }

    // Voronoi distance helper
    if needs_voronoi {
        emit_wgsl_voronoi(s);
    }

    // Simplex noise helpers
    if needs_simplex {
        emit_wgsl_simplex(s);
    }
}

fn emit_wgsl_noise_helpers(s: &mut String) {
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
    s.push_str("        mix(hash2(i + vec2<f32>(0.0, 1.0)), hash2(i + vec2<f32>(1.0, 1.0)), u_v.x),\n");
    s.push_str("        u_v.y\n");
    s.push_str("    ) * 2.0 - 1.0;\n");
    s.push_str("}\n\n");
}

fn emit_wgsl_fbm(s: &mut String) {
    s.push_str("fn fbm2(p: vec2<f32>, octaves: i32, persistence: f32, lacunarity: f32) -> f32 {\n");
    s.push_str("    var value: f32 = 0.0;\n");
    s.push_str("    var amplitude: f32 = 1.0;\n");
    s.push_str("    var frequency: f32 = 1.0;\n");
    s.push_str("    var max_val: f32 = 0.0;\n");
    s.push_str("    var q = p;\n");
    s.push_str("    for (var i: i32 = 0; i < octaves; i = i + 1) {\n");
    s.push_str("        value = value + noise2(q * frequency) * amplitude;\n");
    s.push_str("        max_val = max_val + amplitude;\n");
    s.push_str("        amplitude = amplitude * persistence;\n");
    s.push_str("        frequency = frequency * lacunarity;\n");
    // Domain rotation to remove axis-aligned artifacts
    s.push_str("        q = vec2<f32>(q.x * 0.8 - q.y * 0.6, q.x * 0.6 + q.y * 0.8);\n");
    s.push_str("    }\n");
    s.push_str("    return value / max_val;\n");
    s.push_str("}\n\n");
}

fn emit_wgsl_voronoi(s: &mut String) {
    s.push_str("fn voronoi2(p: vec2<f32>, scale: f32) -> f32 {\n");
    s.push_str("    let sp = p * scale;\n");
    s.push_str("    let i_part = floor(sp);\n");
    s.push_str("    let f_part = fract(sp);\n");
    s.push_str("    var min_dist: f32 = 1.0;\n");
    s.push_str("    for (var y: i32 = -1; y <= 1; y = y + 1) {\n");
    s.push_str("        for (var x: i32 = -1; x <= 1; x = x + 1) {\n");
    s.push_str("            let neighbor = vec2<f32>(f32(x), f32(y));\n");
    s.push_str("            let point_val = hash2(i_part + neighbor);\n");
    s.push_str("            let point = neighbor + vec2<f32>(point_val, fract(point_val * 17.0)) - f_part;\n");
    s.push_str("            min_dist = min(min_dist, dot(point, point));\n");
    s.push_str("        }\n");
    s.push_str("    }\n");
    s.push_str("    return sqrt(min_dist);\n");
    s.push_str("}\n\n");
}

fn emit_wgsl_simplex(s: &mut String) {
    // Gradient hash helper for simplex noise
    s.push_str("fn hash2v(p: vec2<f32>) -> vec2<f32> {\n");
    s.push_str("    let q = vec2<f32>(dot(p, vec2<f32>(127.1, 311.7)), dot(p, vec2<f32>(269.5, 183.3)));\n");
    s.push_str("    return -1.0 + 2.0 * fract(sin(q) * 43758.5453);\n");
    s.push_str("}\n\n");

    s.push_str("fn simplex2(p: vec2<f32>) -> f32 {\n");
    s.push_str("    let K1: f32 = 0.366025404;\n");
    s.push_str("    let K2: f32 = 0.211324865;\n");
    s.push_str("    let i_part = floor(p + (p.x + p.y) * K1);\n");
    s.push_str("    let a = p - i_part + (i_part.x + i_part.y) * K2;\n");
    s.push_str("    let o = step(vec2<f32>(a.y, a.x), a);\n");
    s.push_str("    let b = a - o + K2;\n");
    s.push_str("    let c = a - 1.0 + 2.0 * K2;\n");
    s.push_str("    let h = max(vec3<f32>(0.5 - dot(a, a), 0.5 - dot(b, b), 0.5 - dot(c, c)), vec3<f32>(0.0));\n");
    s.push_str("    let n = h * h * h * h * vec3<f32>(dot(a, hash2v(i_part)), dot(b, hash2v(i_part + o)), dot(c, hash2v(i_part + 1.0)));\n");
    s.push_str("    return dot(n, vec3<f32>(70.0));\n");
    s.push_str("}\n\n");
}

// ── Layer emission ──────────────────────────────────────────────

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

    s.push_str(&format!("{indent}var p = vec2<f32>(uv.x * aspect, uv.y);\n"));

    for stage in body {
        emit_wgsl_stage(s, stage, indent);
    }

    // Memory: mix with previous frame if this layer has memory
    if let Some(decay) = layer.memory {
        memory::emit_wgsl_memory_mix(s, decay, indent);
    }

    if multi {
        // Screen blend: prevents blowout from additive compositing
        s.push_str(&format!("{indent}let lc = color_result.rgb;\n"));
        s.push_str(&format!("{indent}final_color = vec4<f32>(final_color.rgb + lc - final_color.rgb * lc, 1.0);\n"));
        s.push_str("    }\n\n");
    } else {
        s.push_str(&format!("{indent}return color_result;\n"));
    }
}

// ── Stage emission: ALL 37 builtins ─────────────────────────────

fn emit_wgsl_stage(s: &mut String, stage: &Stage, indent: &str) {
    let args = &stage.args;
    match stage.name.as_str() {
        // ── SDF Generators: Position -> Sdf ─────────────────

        "circle" => {
            let r = get_arg(args, "radius", 0, "circle");
            s.push_str(&format!("{indent}var sdf_result = sdf_circle(p, {r});\n"));
        }
        "ring" => {
            let r = get_arg(args, "radius", 0, "ring");
            let w = get_arg(args, "width", 1, "ring");
            s.push_str(&format!("{indent}var sdf_result = abs(length(p) - {r}) - {w};\n"));
        }
        "star" => {
            let points = get_arg(args, "points", 0, "star");
            let radius = get_arg(args, "radius", 1, "star");
            let inner = get_arg(args, "inner", 2, "star");
            s.push_str(&format!("{indent}var sdf_result: f32;\n"));
            s.push_str(&format!("{indent}{{ let st_angle = atan2(p.y, p.x);\n"));
            s.push_str(&format!("{indent}let st_r = length(p);\n"));
            s.push_str(&format!("{indent}let st_n = {points};\n"));
            s.push_str(&format!("{indent}let st_seg = 6.28318 / st_n;\n"));
            s.push_str(&format!("{indent}let st_raw = ((st_angle % st_seg) + st_seg) % st_seg;\n"));
            s.push_str(&format!("{indent}let st_a = abs(st_raw - st_seg * 0.5);\n"));
            s.push_str(&format!("{indent}sdf_result = st_r * cos(st_a) - {radius} + (st_r * sin(st_a) - {inner}) * 0.5; }}\n"));
        }
        "box" => {
            let w = get_arg(args, "w", 0, "box");
            let h = get_arg(args, "h", 1, "box");
            s.push_str(&format!("{indent}let box_d = abs(p) - vec2<f32>({w}, {h});\n"));
            s.push_str(&format!("{indent}var sdf_result = length(max(box_d, vec2<f32>(0.0))) + min(max(box_d.x, box_d.y), 0.0);\n"));
        }
        "polygon" => {
            let sides = get_arg(args, "sides", 0, "polygon");
            let radius = get_arg(args, "radius", 1, "polygon");
            s.push_str(&format!("{indent}var sdf_result: f32;\n"));
            s.push_str(&format!("{indent}{{ let pg_a = atan2(p.y, p.x);\n"));
            s.push_str(&format!("{indent}let pg_r = length(p);\n"));
            s.push_str(&format!("{indent}let pg_n = {sides};\n"));
            s.push_str(&format!("{indent}let pg_seg = 6.28318 / pg_n;\n"));
            s.push_str(&format!("{indent}let pg_raw = ((pg_a % pg_seg) + pg_seg) % pg_seg;\n"));
            s.push_str(&format!("{indent}let pg_ha = abs(pg_raw - pg_seg * 0.5);\n"));
            s.push_str(&format!("{indent}sdf_result = pg_r * cos(pg_ha) - {radius}; }}\n"));
        }
        "fbm" => {
            let sc = get_arg(args, "scale", 0, "fbm");
            let oct = get_arg(args, "octaves", 1, "fbm");
            let pers = get_arg(args, "persistence", 2, "fbm");
            let lac = get_arg(args, "lacunarity", 3, "fbm");
            s.push_str(&format!("{indent}var sdf_result = fbm2((p * {sc}), i32({oct}), {pers}, {lac});\n"));
        }
        "simplex" => {
            let sc = get_arg(args, "scale", 0, "simplex");
            s.push_str(&format!("{indent}var sdf_result = simplex2(p * {sc});\n"));
        }
        "voronoi" => {
            let sc = get_arg(args, "scale", 0, "voronoi");
            s.push_str(&format!("{indent}var sdf_result = voronoi2(p, {sc});\n"));
        }
        "concentric_waves" => {
            let amp = get_arg(args, "amplitude", 0, "concentric_waves");
            let width = get_arg(args, "width", 1, "concentric_waves");
            let freq = get_arg(args, "frequency", 2, "concentric_waves");
            s.push_str(&format!("{indent}let cw_r = length(p) * {freq};\n"));
            s.push_str(&format!("{indent}var sdf_result = sin(cw_r - time * 2.0) * {amp} * exp(-length(p) * {width});\n"));
        }

        // ── Sdf -> Color bridges ────────────────────────────

        "glow" => {
            let intensity = get_arg(args, "intensity", 0, "glow");
            s.push_str(&format!("{indent}let glow_result = apply_glow(sdf_result, {intensity});\n"));
            s.push_str(&format!("{indent}var color_result = vec4<f32>(vec3<f32>(glow_result), 1.0);\n"));
        }
        "shade" => {
            let r = get_arg(args, "r", 0, "shade");
            let g = get_arg(args, "g", 1, "shade");
            let b = get_arg(args, "b", 2, "shade");
            // Anti-aliased edge using smoothstep with screen-space awareness
            s.push_str(&format!("{indent}let aa = 0.005;\n"));
            s.push_str(&format!("{indent}var color_result = vec4<f32>(vec3<f32>({r}, {g}, {b}) * smoothstep(aa, -aa, sdf_result), 1.0);\n"));
        }
        "emissive" => {
            let intensity = get_arg(args, "intensity", 0, "emissive");
            s.push_str(&format!("{indent}let glow_result = apply_glow(sdf_result, {intensity});\n"));
            s.push_str(&format!("{indent}var color_result = vec4<f32>(vec3<f32>(glow_result), glow_result);\n"));
        }

        // ── Color processors: Color -> Color ────────────────

        "tint" => {
            let r = get_arg(args, "r", 0, "tint");
            let g = get_arg(args, "g", 1, "tint");
            let b = get_arg(args, "b", 2, "tint");
            s.push_str(&format!("{indent}color_result = vec4<f32>(color_result.rgb * vec3<f32>({r}, {g}, {b}), 1.0);\n"));
        }
        "bloom" => {
            let thresh = get_arg(args, "threshold", 0, "bloom");
            let strength = get_arg(args, "strength", 1, "bloom");
            s.push_str(&format!("{indent}let pp_lum = dot(color_result.rgb, vec3<f32>(0.299, 0.587, 0.114));\n"));
            s.push_str(&format!("{indent}color_result = vec4<f32>(color_result.rgb + max(pp_lum - {thresh}, 0.0) * {strength}, 1.0);\n"));
        }
        "grain" => {
            let amount = get_arg(args, "amount", 0, "grain");
            s.push_str(&format!("{indent}let grain_noise = fract(sin(dot(p, vec2<f32>(12.9898, 78.233)) + time) * 43758.5453);\n"));
            s.push_str(&format!("{indent}color_result = vec4<f32>(color_result.rgb + (grain_noise - 0.5) * {amount}, color_result.a);\n"));
        }
        "vignette" => {
            let strength = get_arg(args, "strength", 0, "vignette");
            let radius = get_arg(args, "radius", 1, "vignette");
            s.push_str(&format!("{indent}{{ let vig_d = length(uv);\n"));
            s.push_str(&format!("{indent}let vig = smoothstep({radius} + 0.3, {radius} - 0.2, vig_d);\n"));
            s.push_str(&format!("{indent}color_result = vec4<f32>(color_result.rgb * mix(1.0, vig, {strength}), color_result.a); }}\n"));
        }
        "chromatic" => {
            let offset = get_arg(args, "offset", 0, "chromatic");
            s.push_str(&format!("{indent}{{ let chr_d = length(uv) * {offset};\n"));
            s.push_str(&format!("{indent}color_result = vec4<f32>(\n"));
            s.push_str(&format!("{indent}    color_result.r + chr_d * 0.5,\n"));
            s.push_str(&format!("{indent}    color_result.g,\n"));
            s.push_str(&format!("{indent}    color_result.b - chr_d * 0.5,\n"));
            s.push_str(&format!("{indent}    color_result.a\n"));
            s.push_str(&format!("{indent}); }}\n"));
        }
        "tonemap" => {
            let exposure = get_arg(args, "exposure", 0, "tonemap");
            // ACES filmic tonemapping
            s.push_str(&format!("{indent}{{ let tm_x = color_result.rgb * {exposure};\n"));
            s.push_str(&format!("{indent}let tm_a = 2.51;\n"));
            s.push_str(&format!("{indent}let tm_b = 0.03;\n"));
            s.push_str(&format!("{indent}let tm_c = 2.43;\n"));
            s.push_str(&format!("{indent}let tm_d = 0.59;\n"));
            s.push_str(&format!("{indent}let tm_e = 0.14;\n"));
            s.push_str(&format!("{indent}color_result = vec4<f32>(clamp((tm_x * (tm_a * tm_x + tm_b)) / (tm_x * (tm_c * tm_x + tm_d) + tm_e), vec3<f32>(0.0), vec3<f32>(1.0)), color_result.a); }}\n"));
        }
        "scanlines" => {
            let freq = get_arg(args, "frequency", 0, "scanlines");
            let intensity = get_arg(args, "intensity", 1, "scanlines");
            s.push_str(&format!("{indent}{{ let scan = sin(uv.y * {freq} * 3.14159) * 0.5 + 0.5;\n"));
            s.push_str(&format!("{indent}color_result = vec4<f32>(color_result.rgb * (1.0 - {intensity} * (1.0 - scan)), color_result.a); }}\n"));
        }
        "saturate_color" => {
            let amount = get_arg(args, "amount", 0, "saturate_color");
            s.push_str(&format!("{indent}{{ let sat_gray = dot(color_result.rgb, vec3<f32>(0.299, 0.587, 0.114));\n"));
            s.push_str(&format!("{indent}color_result = vec4<f32>(mix(vec3<f32>(sat_gray), color_result.rgb, {amount}), color_result.a); }}\n"));
        }
        "glitch" => {
            let intensity = get_arg(args, "intensity", 0, "glitch");
            s.push_str(&format!("{indent}{{ let gl_t = floor(time * 8.0);\n"));
            s.push_str(&format!("{indent}let gl_offset = (fract(sin(gl_t * 12.9898) * 43758.5453) - 0.5) * {intensity} * 0.1;\n"));
            s.push_str(&format!("{indent}let gl_block = step(0.9 - {intensity} * 0.3, fract(sin(dot(vec2<f32>(uv.y * 20.0, gl_t), vec2<f32>(12.9898, 78.233))) * 43758.5453));\n"));
            s.push_str(&format!("{indent}color_result = vec4<f32>(mix(color_result.rgb, vec3<f32>(color_result.r + gl_offset, color_result.g - gl_offset * 0.5, color_result.b), gl_block), color_result.a); }}\n"));
        }
        "blend" => {
            let factor = get_arg(args, "factor", 0, "blend");
            s.push_str(&format!("{indent}color_result = vec4<f32>(color_result.rgb * {factor}, color_result.a);\n"));
        }

        // ── Position transforms: Position -> Position ───────

        "rotate" => {
            let angle = get_arg(args, "angle", 0, "rotate");
            s.push_str(&format!("{indent}{{ let rc = cos({angle}); let rs = sin({angle});\n"));
            s.push_str(&format!("{indent}p = vec2<f32>(p.x * rc - p.y * rs, p.x * rs + p.y * rc); }}\n"));
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
        "twist" => {
            let amount = get_arg(args, "amount", 0, "twist");
            s.push_str(&format!("{indent}{{ let tw = {amount} * p.y;\n"));
            s.push_str(&format!("{indent}p = vec2<f32>(p.x * cos(tw) - p.y * sin(tw), p.x * sin(tw) + p.y * cos(tw)); }}\n"));
        }
        "mirror" => {
            let axis = get_arg(args, "axis", 0, "mirror");
            // axis 0.0 = mirror X, 1.0 = mirror Y
            s.push_str(&format!("{indent}p = select(vec2<f32>(abs(p.x), p.y), vec2<f32>(p.x, abs(p.y)), {axis} > 0.5);\n"));
        }
        "repeat" => {
            let count = get_arg(args, "count", 0, "repeat");
            // WGSL % is truncation-based, double-mod for floor behavior
            s.push_str(&format!("{indent}{{ let rep_size = 2.0 / {count};\n"));
            s.push_str(&format!("{indent}let rep_v = vec2<f32>(rep_size);\n"));
            s.push_str(&format!("{indent}p = ((p % rep_v) + rep_v) % rep_v - rep_v * 0.5; }}\n"));
        }
        "domain_warp" => {
            let amount = get_arg(args, "amount", 0, "domain_warp");
            let freq = get_arg(args, "freq", 1, "domain_warp");
            s.push_str(&format!("{indent}p = p + vec2<f32>(noise2(p * {freq}), noise2(p * {freq} + vec2<f32>(5.2, 1.3))) * {amount};\n"));
        }
        "curl_noise" => {
            let freq = get_arg(args, "frequency", 0, "curl_noise");
            let amp = get_arg(args, "amplitude", 1, "curl_noise");
            s.push_str(&format!("{indent}{{ let eps = 0.01;\n"));
            s.push_str(&format!("{indent}let cn_x = noise2((p + vec2<f32>(eps, 0.0)) * {freq}) - noise2((p - vec2<f32>(eps, 0.0)) * {freq});\n"));
            s.push_str(&format!("{indent}let cn_y = noise2((p + vec2<f32>(0.0, eps)) * {freq}) - noise2((p - vec2<f32>(0.0, eps)) * {freq});\n"));
            s.push_str(&format!("{indent}p = p + vec2<f32>(cn_y, -cn_x) * {amp} / (2.0 * eps); }}\n"));
        }
        "displace" => {
            let strength = get_arg(args, "strength", 0, "displace");
            s.push_str(&format!("{indent}p = p + vec2<f32>(noise2(p * 3.0), noise2(p * 3.0 + vec2<f32>(5.0, 3.0))) * {strength};\n"));
        }

        // ── SDF modifiers: Sdf -> Sdf ───────────────────────

        "mask_arc" => {
            let angle = get_arg(args, "angle", 0, "mask_arc");
            s.push_str(&format!("{indent}let arc_theta = atan2(p.x, p.y) + 3.14159265359;\n"));
            s.push_str(&format!("{indent}sdf_result = select(999.0, sdf_result, arc_theta < {angle});\n"));
        }
        "threshold" => {
            let cutoff = get_arg(args, "cutoff", 0, "threshold");
            s.push_str(&format!("{indent}sdf_result = step({cutoff}, sdf_result);\n"));
        }
        "onion" => {
            let thickness = get_arg(args, "thickness", 0, "onion");
            s.push_str(&format!("{indent}sdf_result = abs(sdf_result) - {thickness};\n"));
        }
        "round" => {
            let radius = get_arg(args, "radius", 0, "round");
            s.push_str(&format!("{indent}sdf_result = sdf_result - {radius};\n"));
        }

        // ── Position -> Color generators ────────────────────

        "gradient" => {
            // Default gradient: vertical dark-to-light
            s.push_str(&format!("{indent}var color_result = vec4<f32>(vec3<f32>(uv.y * 0.5 + 0.5), 1.0);\n"));
        }
        "spectrum" => {
            let bass = get_arg(args, "bass", 0, "spectrum");
            let mid = get_arg(args, "mid", 1, "spectrum");
            let treble = get_arg(args, "treble", 2, "spectrum");
            s.push_str(&format!("{indent}{{ let sp_d = length(p);\n"));
            s.push_str(&format!("{indent}let sp_bass = smoothstep(0.35, 0.15, sp_d) * {bass};\n"));
            s.push_str(&format!("{indent}let sp_mid = smoothstep(0.55, 0.35, sp_d) * smoothstep(0.15, 0.35, sp_d) * {mid};\n"));
            s.push_str(&format!("{indent}let sp_treble = smoothstep(0.75, 0.55, sp_d) * smoothstep(0.35, 0.55, sp_d) * {treble};\n"));
            s.push_str(&format!("{indent}var color_result = vec4<f32>(\n"));
            s.push_str(&format!("{indent}    sp_bass * vec3<f32>(1.0, 0.2, 0.1) +\n"));
            s.push_str(&format!("{indent}    sp_mid * vec3<f32>(0.1, 1.0, 0.3) +\n"));
            s.push_str(&format!("{indent}    sp_treble * vec3<f32>(0.2, 0.3, 1.0),\n"));
            s.push_str(&format!("{indent}    1.0\n"));
            s.push_str(&format!("{indent}); }}\n"));
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
            listen: None, voice: None, score: None, gravity: None,
            lenses: vec![], react: None, defines: vec![],
        }
    }

    #[test]
    fn basic_wgsl_output() {
        let cin = make_cinematic(vec![
            Stage { name: "circle".into(), args: vec![Arg { name: None, value: Expr::Number(0.2) }] },
            Stage { name: "glow".into(), args: vec![Arg { name: None, value: Expr::Number(1.5) }] },
            Stage { name: "tint".into(), args: vec![
                Arg { name: None, value: Expr::Number(0.831) },
                Arg { name: None, value: Expr::Number(0.686) },
                Arg { name: None, value: Expr::Number(0.216) },
            ] },
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
}
