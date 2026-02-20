use crate::ast::*;
use crate::error::{ErrorKind, GameError, Result};

// ── Public types ───────────────────────────────────────────────────────

/// Full compilation output: WGSL shader + metadata for the runtime.
#[derive(Debug, Clone)]
pub struct CompileOutput {
    pub wgsl: String,
    pub title: String,
    pub audio_file: Option<String>,
    pub params: Vec<CompiledParam>,
    pub uses_audio: bool,
    pub uses_mouse: bool,
    pub uses_data: bool,
    /// Names of `data.*` fields referenced in modulation expressions.
    pub data_fields: Vec<String>,
    pub render_mode: RenderMode,
    /// Number of f32 slots in the uniform buffer.
    pub uniform_float_count: usize,
}

/// A parameter with modulation metadata for the JS runtime.
#[derive(Debug, Clone)]
pub struct CompiledParam {
    pub name: String,
    pub uniform_field: String,
    pub buffer_index: usize,
    pub base_value: f64,
    pub mod_js: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RenderMode {
    Flat,
    Raymarch {
        cam_radius: f64,
        cam_height: f64,
        cam_speed: f64,
    },
}

// ── System uniform layout ──────────────────────────────────────────────
// Float32Array indices:
//   [0] time
//   [1] audio_bass
//   [2] audio_mid
//   [3] audio_treble
//   [4] audio_energy
//   [5] audio_beat
//   [6] resolution.x  (vec2f at byte offset 24, which is 8-aligned)
//   [7] resolution.y
//   [8] mouse.x
//   [9] mouse.y
//   [10..] dynamic params
const SYSTEM_FLOAT_COUNT: usize = 10;

// ── Public API ─────────────────────────────────────────────────────────

/// Backward-compatible: compile to WGSL string only.
pub fn generate_wgsl(cinematic: &Cinematic) -> Result<String> {
    let output = generate_full(cinematic)?;
    Ok(output.wgsl)
}

/// Full compilation: WGSL + metadata for the runtime.
pub fn generate_full(cinematic: &Cinematic) -> Result<CompileOutput> {
    let mut gen = WgslGen::new();

    // Collect params from all layers
    gen.collect_params(cinematic);

    // Determine rendering mode from lens block
    gen.render_mode = determine_render_mode(cinematic);

    // Generate WGSL
    gen.generate(cinematic)?;

    let title = cinematic.name.clone().unwrap_or_else(|| "Untitled".to_string());
    let audio_file = extract_audio_file(cinematic);
    let param_count = gen.params.len();

    Ok(CompileOutput {
        wgsl: gen.output,
        title,
        audio_file,
        uses_audio: gen.uses_audio,
        uses_mouse: gen.uses_mouse,
        uses_data: gen.uses_data,
        data_fields: gen.data_fields,
        params: gen.params,
        render_mode: gen.render_mode,
        uniform_float_count: SYSTEM_FLOAT_COUNT + param_count,
    })
}

// ── WgslGen ────────────────────────────────────────────────────────────

struct WgslGen {
    output: String,
    indent: usize,
    params: Vec<CompiledParam>,
    uses_audio: bool,
    uses_mouse: bool,
    uses_data: bool,
    data_fields: Vec<String>,
    render_mode: RenderMode,
    used_builtins: std::collections::HashSet<&'static str>,
}

impl WgslGen {
    fn new() -> Self {
        Self {
            output: String::with_capacity(8192),
            indent: 0,
            params: Vec::new(),
            uses_audio: false,
            uses_mouse: false,
            uses_data: false,
            data_fields: Vec::new(),
            render_mode: RenderMode::Flat,
            used_builtins: std::collections::HashSet::new(),
        }
    }

    fn line(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn blank(&mut self) {
        self.output.push('\n');
    }

    // ── Parameter collection ───────────────────────────────────────────

    fn collect_params(&mut self, cinematic: &Cinematic) {
        for layer in &cinematic.layers {
            for param in &layer.params {
                let idx = SYSTEM_FLOAT_COUNT + self.params.len();
                let uniform_field = format!("p_{}", param.name);

                let mod_js = param.modulation.as_ref().map(|m| {
                    let js = compile_expr_js(&m.signal);
                    // Check for audio/mouse/data signals
                    if expr_uses_audio(&m.signal) {
                        self.uses_audio = true;
                    }
                    if expr_uses_mouse(&m.signal) {
                        self.uses_mouse = true;
                    }
                    if expr_uses_data(&m.signal) {
                        self.uses_data = true;
                        collect_data_fields_into(&m.signal, &mut self.data_fields);
                    }
                    js
                });

                let base_value = extract_number(&param.base_value).unwrap_or(0.0);

                self.params.push(CompiledParam {
                    name: param.name.clone(),
                    uniform_field,
                    buffer_index: idx,
                    base_value,
                    mod_js,
                });
            }
        }
    }

    // ── Main generation ────────────────────────────────────────────────

    fn generate(&mut self, cinematic: &Cinematic) -> Result<()> {
        // Phase 1: Emit fragment shader to a temp buffer to collect used_builtins
        let saved_output = std::mem::take(&mut self.output);

        match &self.render_mode {
            RenderMode::Flat => {
                if let Some(layer) = cinematic.layers.first() {
                    self.emit_flat_fragment(layer)?;
                } else {
                    self.emit_empty_fragment();
                }
            }
            RenderMode::Raymarch { cam_radius, cam_height, cam_speed } => {
                let cr = *cam_radius;
                let ch = *cam_height;
                let cs = *cam_speed;
                if let Some(layer) = cinematic.layers.first() {
                    self.emit_raymarch_helpers(layer)?;
                    self.emit_raymarch_fragment(layer, cr, ch, cs)?;
                } else {
                    self.emit_empty_fragment();
                }
            }
        }

        let fragment_output = std::mem::replace(&mut self.output, saved_output);

        // Phase 2: Now emit in correct order with tree-shaken builtins
        self.emit_header();
        self.emit_uniforms();
        self.emit_vertex_shader();
        self.emit_builtin_functions();
        self.output.push_str(&fragment_output);

        Ok(())
    }

    // ── Structural WGSL ────────────────────────────────────────────────

    fn emit_header(&mut self) {
        self.line("// Generated by GAME compiler v0.2.0");
        self.line("// https://github.com/runyourempire/game-engine");
        self.blank();
    }

    fn emit_uniforms(&mut self) {
        // Collect param field names to avoid borrow conflict
        let param_fields: Vec<String> = self.params.iter()
            .map(|p| p.uniform_field.clone())
            .collect();

        self.line("struct Uniforms {");
        self.indent += 1;
        self.line("time: f32,");
        self.line("audio_bass: f32,");
        self.line("audio_mid: f32,");
        self.line("audio_treble: f32,");
        self.line("audio_energy: f32,");
        self.line("audio_beat: f32,");
        self.line("resolution: vec2f,");
        self.line("mouse: vec2f,");
        for field in &param_fields {
            self.line(&format!("{field}: f32,"));
        }
        self.indent -= 1;
        self.line("}");
        self.blank();
        self.line("@group(0) @binding(0) var<uniform> u: Uniforms;");
        self.blank();
    }

    fn emit_vertex_shader(&mut self) {
        self.line("struct VertexOutput {");
        self.indent += 1;
        self.line("@builtin(position) position: vec4f,");
        self.line("@location(0) uv: vec2f,");
        self.indent -= 1;
        self.line("}");
        self.blank();
        self.line("@vertex");
        self.line("fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {");
        self.indent += 1;
        self.line("var pos = array<vec2f, 4>(");
        self.indent += 1;
        self.line("vec2f(-1.0, -1.0),");
        self.line("vec2f( 1.0, -1.0),");
        self.line("vec2f(-1.0,  1.0),");
        self.line("vec2f( 1.0,  1.0),");
        self.indent -= 1;
        self.line(");");
        self.line("var out: VertexOutput;");
        self.line("out.position = vec4f(pos[vi], 0.0, 1.0);");
        self.line("out.uv = pos[vi] * 0.5 + 0.5;");
        self.line("return out;");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_builtin_functions(&mut self) {
        let mut emitted_any = false;

        if self.used_builtins.contains("sdf_circle") {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                emitted_any = true;
            }
            self.line("fn sdf_circle(p: vec2f, radius: f32) -> f32 {");
            self.indent += 1;
            self.line("return length(p) - radius;");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }

        if self.used_builtins.contains("sdf_sphere") {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                emitted_any = true;
            }
            self.line("fn sdf_sphere(p: vec3f, radius: f32) -> f32 {");
            self.indent += 1;
            self.line("return length(p) - radius;");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }

        if self.used_builtins.contains("apply_glow") {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                emitted_any = true;
            }
            self.line("fn apply_glow(d: f32, intensity: f32) -> f32 {");
            self.indent += 1;
            self.line("return exp(-max(d, 0.0) * intensity * 8.0);");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }

        // fbm2 depends on noise2, which depends on hash2
        if self.used_builtins.contains("fbm2") {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                // emitted_any = true; // not needed, last block
            }
            self.line("fn hash2(p: vec2f) -> f32 {");
            self.indent += 1;
            self.line("var p3 = fract(vec3f(p.x, p.y, p.x) * 0.1031);");
            self.line("p3 += dot(p3, p3.yzx + 33.33);");
            self.line("return fract((p3.x + p3.y) * p3.z);");
            self.indent -= 1;
            self.line("}");
            self.blank();

            self.line("fn noise2(p: vec2f) -> f32 {");
            self.indent += 1;
            self.line("let i = floor(p);");
            self.line("let f = fract(p);");
            self.line("let u = f * f * (3.0 - 2.0 * f);");
            self.line("return mix(");
            self.indent += 1;
            self.line("mix(hash2(i), hash2(i + vec2f(1.0, 0.0)), u.x),");
            self.line("mix(hash2(i + vec2f(0.0, 1.0)), hash2(i + vec2f(1.0, 1.0)), u.x),");
            self.line("u.y");
            self.indent -= 1;
            self.line(") * 2.0 - 1.0;");
            self.indent -= 1;
            self.line("}");
            self.blank();

            self.line("fn fbm2(p: vec2f, octaves: i32, persistence: f32, lacunarity: f32) -> f32 {");
            self.indent += 1;
            self.line("var value: f32 = 0.0;");
            self.line("var amplitude: f32 = 1.0;");
            self.line("var frequency: f32 = 1.0;");
            self.line("var max_val: f32 = 0.0;");
            self.line("for (var i: i32 = 0; i < octaves; i++) {");
            self.indent += 1;
            self.line("value += noise2(p * frequency) * amplitude;");
            self.line("max_val += amplitude;");
            self.line("amplitude *= persistence;");
            self.line("frequency *= lacunarity;");
            self.indent -= 1;
            self.line("}");
            self.line("return value / max_val;");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }
    }

    // ── Flat mode fragment shader ──────────────────────────────────────

    fn emit_flat_fragment(&mut self, layer: &Layer) -> Result<()> {
        let chain = layer.fn_chain.as_ref().ok_or_else(|| GameError {
            kind: crate::error::ErrorKind::Message("layer has no fn: chain".into()),
            span: None,
            source_text: None,
        })?;

        self.line("@fragment");
        self.line("fn fs_main(input: VertexOutput) -> @location(0) vec4f {");
        self.indent += 1;

        self.line("let uv = input.uv * 2.0 - 1.0;");
        self.line("let aspect = u.resolution.x / u.resolution.y;");
        self.line("var p = vec2f(uv.x * aspect, uv.y);");
        self.line("let time = fract(u.time / 120.0) * 120.0;");
        self.blank();

        // Emit param bindings
        self.emit_param_bindings();

        // Walk pipe chain
        let mut state = ShaderState::Position;
        for (i, stage) in chain.stages.iter().enumerate() {
            let next_state = self.classify_stage(&stage.name)?;

            // State transitions: bridge to Color when needed
            if matches!(next_state, ShaderState::Color) && matches!(state, ShaderState::Sdf) {
                self.line("let height = clamp(sdf_result * 0.5 + 0.5, 0.0, 1.0);");
                self.blank();
            }
            if self.is_postprocess(&stage.name) && matches!(state, ShaderState::Glow) {
                self.line("var color_result = vec4f(vec3f(glow_result), 1.0);");
                self.blank();
            }
            if self.is_postprocess(&stage.name) && matches!(state, ShaderState::Sdf) {
                self.used_builtins.insert("apply_glow");
                self.line("var color_result = vec4f(vec3f(apply_glow(sdf_result, 2.0)), 1.0);");
                self.blank();
            }

            self.emit_flat_stage(stage, &state, i)?;
            state = next_state;
        }

        match state {
            ShaderState::Sdf => {
                self.used_builtins.insert("apply_glow");
                self.line("let final_glow = apply_glow(sdf_result, 2.0);");
                self.line("return vec4f(vec3f(final_glow), 1.0);");
            }
            ShaderState::Glow => {
                self.line("return vec4f(vec3f(glow_result), 1.0);");
            }
            ShaderState::Color => {
                self.line("return color_result;");
            }
            ShaderState::Position => {
                self.line("return vec4f(0.0, 0.0, 0.0, 1.0);");
            }
        }

        self.indent -= 1;
        self.line("}");

        Ok(())
    }

    fn emit_flat_stage(&mut self, stage: &FnCall, prev: &ShaderState, index: usize) -> Result<()> {
        self.line(&format!("// stage {}: {}(...)", index, stage.name));

        match stage.name.as_str() {
            "translate" => {
                let tx = self.compile_arg(&stage.args, 0, "0.0")?;
                let ty = self.compile_arg(&stage.args, 1, "0.0")?;
                self.line(&format!("p = p - vec2f({tx}, {ty});"));
            }
            "circle" => {
                let r = self.compile_arg(&stage.args, 0, "0.5")?;
                self.used_builtins.insert("sdf_circle");
                self.line(&format!("var sdf_result = sdf_circle(p, {r});"));
            }
            "sphere" => {
                let r = self.compile_arg(&stage.args, 0, "0.5")?;
                self.used_builtins.insert("sdf_sphere");
                self.line(&format!("var sdf_result = sdf_sphere(vec3f(p, 0.0), {r});"));
            }
            "ring" => {
                let r = self.compile_arg(&stage.args, 0, "0.3")?;
                let thickness = self.compile_arg(&stage.args, 1, "0.04")?;
                self.line(&format!("var sdf_result = abs(length(p) - {r}) - {thickness};"));
            }
            "glow" => {
                let i = self.compile_arg(&stage.args, 0, "2.0")?;
                self.used_builtins.insert("apply_glow");
                self.line(&format!("let glow_result = apply_glow(sdf_result, {i});"));
            }
            "rotate" => {
                let angle = self.compile_arg(&stage.args, 0, "0.0")?;
                self.line(&format!("{{ let rc = cos({angle}); let rs = sin({angle});"));
                self.line("p = vec2f(p.x * rc - p.y * rs, p.x * rs + p.y * rc); }");
            }
            "mask_arc" => {
                let angle = self.compile_arg(&stage.args, 0, "6.283")?;
                self.line("let arc_theta = atan2(p.x, p.y) + 3.14159265359;");
                self.line(&format!("sdf_result = select(999.0, sdf_result, arc_theta < {angle});"));
            }
            "fbm" => {
                let pos = self.compile_arg(&stage.args, 0, "p")?;
                let oct = self.compile_named_arg(&stage.args, "octaves", "6")?;
                let per = self.compile_named_arg(&stage.args, "persistence", "0.5")?;
                let lac = self.compile_named_arg(&stage.args, "lacunarity", "2.0")?;
                self.used_builtins.insert("fbm2");
                self.line(&format!(
                    "var sdf_result = fbm2({pos}, i32({oct}), {per}, {lac});"
                ));
            }
            "shade" => {
                self.emit_shade_flat(stage)?;
            }
            "emissive" => {
                self.line("var color_result = vec4f(1.0, 0.8, 0.2, 1.0);");
            }
            "colormap" => {
                self.line("let t = clamp(sdf_result * 0.5 + 0.5, 0.0, 1.0);");
                self.line("var color_result = vec4f(mix(vec3f(0.0, 0.0, 0.2), vec3f(1.0, 0.8, 0.2), t), 1.0);");
            }
            "spectrum" => {
                self.emit_spectrum_flat(stage)?;
            }
            "tint" => {
                let color = self.compile_tint_color(&stage.args)?;
                // If coming from glow, create colored result; if from color, multiply
                if matches!(prev, ShaderState::Glow) {
                    self.line(&format!("var color_result = vec4f(vec3f(glow_result) * {color}, 1.0);"));
                } else {
                    self.line(&format!("color_result = vec4f(color_result.rgb * {color}, 1.0);"));
                }
            }
            "bloom" => {
                let thresh = self.compile_arg(&stage.args, 0, "0.6")?;
                let intensity = self.compile_arg(&stage.args, 1, "1.5")?;
                self.line("let pp_lum = dot(color_result.rgb, vec3f(0.299, 0.587, 0.114));");
                self.line(&format!("color_result = vec4f(color_result.rgb + max(pp_lum - {thresh}, 0.0) * {intensity}, 1.0);"));
            }
            "vignette" => {
                let strength = self.compile_arg(&stage.args, 0, "0.3")?;
                self.line(&format!("color_result = vec4f(color_result.rgb * (1.0 - length(uv) * {strength}), 1.0);"));
            }
            "chromatic" => {
                let strength = self.compile_arg(&stage.args, 0, "0.5")?;
                self.line(&format!("let ca_d = length(uv) * length(uv) * {strength};"));
                self.line("color_result = vec4f(color_result.r * (1.0 + ca_d * 0.5), color_result.g, color_result.b * (1.0 - ca_d * 0.3), 1.0);");
            }
            "grain" => {
                let amount = self.compile_arg(&stage.args, 0, "0.02")?;
                self.line("let gr_n = fract(sin(dot(input.uv * (time * 97.0 + 1.0), vec2f(12.9898, 78.233))) * 43758.5453) * 2.0 - 1.0;");
                self.line(&format!("color_result = vec4f(color_result.rgb + gr_n * {amount}, 1.0);"));
            }
            other => {
                return Err(GameError::unknown_function(other));
            }
        }

        self.blank();
        Ok(())
    }

    fn emit_shade_flat(&mut self, stage: &FnCall) -> Result<()> {
        let albedo_expr = self.compile_named_arg(&stage.args, "albedo", "vec3f(0.8)")?;
        let emissive_expr = self.compile_named_arg(&stage.args, "emissive", "vec3f(0.0)")?;

        self.line(&format!("let shade_albedo = {albedo_expr};"));
        self.line(&format!("let shade_emissive = {emissive_expr};"));
        self.line("var color_result = vec4f(shade_albedo + shade_emissive, 1.0);");
        Ok(())
    }

    fn emit_spectrum_flat(&mut self, stage: &FnCall) -> Result<()> {
        let bass_arg = self.compile_arg(&stage.args, 0, "0.0")?;
        let mid_arg = self.compile_arg(&stage.args, 1, "0.0")?;
        let treble_arg = self.compile_arg(&stage.args, 2, "0.0")?;

        self.line(&format!("let sp_bass = {bass_arg};"));
        self.line(&format!("let sp_mid = {mid_arg};"));
        self.line(&format!("let sp_treble = {treble_arg};"));
        self.blank();
        self.line("// Concentric rings — each frequency band at a different radius");
        self.line("let d_bass = abs(length(p) - 0.15) - 0.02;");
        self.line("let d_mid = abs(length(p) - 0.35) - 0.015;");
        self.line("let d_treble = abs(length(p) - 0.55) - 0.01;");
        self.blank();
        self.line("// Core glow (always-on subtle center)");
        self.line("let sp_core = exp(-length(p) * 6.0) * 0.12;");
        self.blank();
        self.line("// Band-reactive glows — sharpen + intensify with signal");
        self.line("let g_bass = exp(-max(d_bass, 0.0) * (4.0 + sp_bass * 20.0)) * sp_bass;");
        self.line("let g_mid = exp(-max(d_mid, 0.0) * (6.0 + sp_mid * 25.0)) * sp_mid;");
        self.line("let g_treble = exp(-max(d_treble, 0.0) * (8.0 + sp_treble * 30.0)) * sp_treble;");
        self.blank();
        self.line("// Frequency-specific colors");
        self.line("let c_bass = vec3f(1.0, 0.3, 0.05);");
        self.line("let c_mid = vec3f(0.05, 1.0, 0.7);");
        self.line("let c_treble = vec3f(0.6, 0.15, 1.0);");
        self.blank();
        self.line("var color_result = vec4f(");
        self.indent += 1;
        self.line("sp_core * vec3f(0.5, 0.4, 0.3) +");
        self.line("g_bass * c_bass +");
        self.line("g_mid * c_mid +");
        self.line("g_treble * c_treble,");
        self.line("1.0");
        self.indent -= 1;
        self.line(");");

        Ok(())
    }

    // ── Raymarch mode ──────────────────────────────────────────────────

    fn emit_raymarch_helpers(&mut self, layer: &Layer) -> Result<()> {
        let chain = layer.fn_chain.as_ref().ok_or_else(|| GameError {
            kind: crate::error::ErrorKind::Message("layer has no fn: chain".into()),
            span: None,
            source_text: None,
        })?;

        // Find the SDF/field stages (everything before shade)
        let field_stages: Vec<&FnCall> = chain.stages.iter()
            .take_while(|s| !matches!(s.name.as_str(), "shade" | "emissive" | "colormap"))
            .collect();

        // field_at: evaluates the height field at a 2D point
        self.line("fn field_at(p: vec2f) -> f32 {");
        self.indent += 1;

        // Emit param reads from uniforms
        let bindings: Vec<(String, String)> = self.params.iter()
            .map(|p| (p.name.clone(), p.uniform_field.clone()))
            .collect();
        for (name, field) in &bindings {
            self.line(&format!("let {name} = u.{field};"));
        }
        self.line("let time = fract(u.time / 120.0) * 120.0;");

        // Compile field stages
        if let Some(stage) = field_stages.first() {
            match stage.name.as_str() {
                "fbm" => {
                    let pos = self.compile_arg(&stage.args, 0, "p")?;
                    let oct = self.compile_named_arg(&stage.args, "octaves", "6")?;
                    let per = self.compile_named_arg(&stage.args, "persistence", "0.5")?;
                    let lac = self.compile_named_arg(&stage.args, "lacunarity", "2.0")?;
                    self.used_builtins.insert("fbm2");
                    self.line(&format!("return fbm2({pos}, i32({oct}), {per}, {lac});"));
                }
                "circle" => {
                    let r = self.compile_arg(&stage.args, 0, "0.5")?;
                    self.used_builtins.insert("sdf_circle");
                    self.line(&format!("return sdf_circle(p, {r});"));
                }
                _ => {
                    self.line("return 0.0;");
                }
            }
        } else {
            self.line("return 0.0;");
        }

        self.indent -= 1;
        self.line("}");
        self.blank();

        // map_scene: SDF for raymarching (terrain = y - height)
        self.line("fn map_scene(pos: vec3f) -> f32 {");
        self.indent += 1;
        self.line("return pos.y - field_at(pos.xz);");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // calc_normal: central differences
        self.line("fn calc_normal(pos: vec3f) -> vec3f {");
        self.indent += 1;
        self.line("let e = 0.001;");
        self.line("return normalize(vec3f(");
        self.indent += 1;
        self.line("map_scene(pos + vec3f(e, 0.0, 0.0)) - map_scene(pos - vec3f(e, 0.0, 0.0)),");
        self.line("map_scene(pos + vec3f(0.0, e, 0.0)) - map_scene(pos - vec3f(0.0, e, 0.0)),");
        self.line("map_scene(pos + vec3f(0.0, 0.0, e)) - map_scene(pos - vec3f(0.0, 0.0, e))");
        self.indent -= 1;
        self.line("));");
        self.indent -= 1;
        self.line("}");
        self.blank();

        Ok(())
    }

    fn emit_raymarch_fragment(
        &mut self,
        layer: &Layer,
        cam_radius: f64,
        cam_height: f64,
        cam_speed: f64,
    ) -> Result<()> {
        self.line("@fragment");
        self.line("fn fs_main(input: VertexOutput) -> @location(0) vec4f {");
        self.indent += 1;

        // Coordinate setup
        self.line("let uv = input.uv * 2.0 - 1.0;");
        self.line("let aspect = u.resolution.x / u.resolution.y;");
        self.line("let time = fract(u.time / 120.0) * 120.0;");
        self.blank();

        // Param bindings
        self.emit_param_bindings();

        // Orbit camera
        self.line(&format!("let cam_angle = time * {:.6};", cam_speed));
        self.line(&format!(
            "let cam_pos = vec3f(cos(cam_angle) * {r:.2}, {h:.2}, sin(cam_angle) * {r:.2});",
            r = cam_radius,
            h = cam_height,
        ));
        self.line("let target = vec3f(0.0);");
        self.line("let forward = normalize(target - cam_pos);");
        self.line("let right = normalize(cross(vec3f(0.0, 1.0, 0.0), forward));");
        self.line("let up = cross(forward, right);");
        self.line("let rd = normalize(forward + right * uv.x * aspect + up * uv.y);");
        self.blank();

        // Raymarch loop
        self.line("// Raymarch");
        self.line("var t: f32 = 0.0;");
        self.line("var hit = false;");
        self.line("for (var i: i32 = 0; i < 128; i++) {");
        self.indent += 1;
        self.line("let pos = cam_pos + rd * t;");
        self.line("let d = map_scene(pos);");
        self.line("if (abs(d) < 0.001) { hit = true; break; }");
        self.line("t += d * 0.8;  // relaxation factor");
        self.line("if (t > 50.0) { break; }");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Sky color for misses
        self.line("if (!hit) {");
        self.indent += 1;
        self.line("let sky = mix(vec3f(0.0, 0.0, 0.05), vec3f(0.0, 0.0, 0.15), uv.y * 0.5 + 0.5);");
        self.line("return vec4f(sky, 1.0);");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Surface shading
        self.line("let hit_pos = cam_pos + rd * t;");
        self.line("let normal = calc_normal(hit_pos);");
        self.line("let height = clamp(field_at(hit_pos.xz) * 0.5 + 0.5, 0.0, 1.0);");
        self.blank();

        // Lighting (sun + ambient)
        self.line("let sun_dir = normalize(vec3f(0.5, 0.8, 1.0));");
        self.line("let sun_intensity = 0.8;");
        self.line("let ambient = 0.15;");
        self.line("let ndotl = max(dot(normal, sun_dir), 0.0);");
        self.line("let lighting = ndotl * sun_intensity + ambient;");
        self.blank();

        // Material from shade() stage
        let chain = layer.fn_chain.as_ref().ok_or_else(|| {
            GameError {
                kind: ErrorKind::Message("raymarch layer requires a fn: chain".to_string()),
                span: None,
                source_text: None,
            }
        })?;
        let shade_stage = chain.stages.iter().find(|s| s.name == "shade");

        if let Some(shade) = shade_stage {
            let albedo_expr = self.compile_named_arg(&shade.args, "albedo", "vec3f(0.8)")?;
            let emissive_expr = self.compile_named_arg(&shade.args, "emissive", "vec3f(0.0)")?;
            self.line(&format!("let albedo = {albedo_expr};"));
            self.line(&format!("let emissive_color = {emissive_expr};"));
        } else {
            self.line("let albedo = vec3f(0.8);");
            self.line("let emissive_color = vec3f(0.0);");
        }
        self.line("var color = albedo * lighting + emissive_color;");
        self.blank();

        // Fog
        self.line("// Distance fog");
        self.line("let fog_amount = 1.0 - exp(-t * 0.03);");
        self.line("color = mix(color, vec3f(0.0, 0.0, 0.05), fog_amount);");
        self.blank();

        // Post-processing (inline bloom + vignette)
        self.line("// Post-processing");
        self.line("let lum = dot(color, vec3f(0.299, 0.587, 0.114));");
        self.line("color += max(lum - 0.7, 0.0) * 1.2;  // bloom");
        self.line("color *= 1.0 - length(uv) * 0.3;  // vignette");
        self.blank();

        self.line("return vec4f(color, 1.0);");

        self.indent -= 1;
        self.line("}");

        Ok(())
    }

    // ── Common helpers ─────────────────────────────────────────────────

    fn emit_param_bindings(&mut self) {
        if self.params.is_empty() {
            return;
        }
        let bindings: Vec<(String, String)> = self.params.iter()
            .map(|p| (p.name.clone(), p.uniform_field.clone()))
            .collect();
        for (name, field) in &bindings {
            self.line(&format!("let {name} = u.{field};"));
        }
        self.blank();
    }

    fn is_postprocess(&self, name: &str) -> bool {
        matches!(name, "bloom" | "vignette" | "chromatic" | "grain")
    }

    fn classify_stage(&self, name: &str) -> Result<ShaderState> {
        match name {
            "circle" | "sphere" | "ring" | "box" | "torus" | "cylinder" | "plane" => Ok(ShaderState::Sdf),
            "glow" => Ok(ShaderState::Glow),
            "shade" | "emissive" | "colormap" | "spectrum" | "tint" => Ok(ShaderState::Color),
            "fbm" | "simplex" | "voronoi" | "noise" => Ok(ShaderState::Sdf),
            "mask_arc" => Ok(ShaderState::Sdf),
            "translate" | "rotate" | "scale" | "repeat" | "mirror" | "twist" | "displace"
            | "round" => Ok(ShaderState::Position),
            "bloom" | "chromatic" | "vignette" | "grain" => Ok(ShaderState::Color),
            _ => Err(GameError::unknown_function(name)),
        }
    }

    fn emit_empty_fragment(&mut self) {
        self.line("@fragment");
        self.line("fn fs_main(input: VertexOutput) -> @location(0) vec4f {");
        self.indent += 1;
        self.line("return vec4f(0.0, 0.0, 0.0, 1.0);");
        self.indent -= 1;
        self.line("}");
    }

    // ── Expression compilation (WGSL) ──────────────────────────────────

    fn compile_arg(&self, args: &[Arg], index: usize, default: &str) -> Result<String> {
        match args.get(index) {
            Some(Arg::Positional(expr)) => self.compile_expr(expr),
            Some(Arg::Named { value, .. }) => self.compile_expr(value),
            None => Ok(default.to_string()),
        }
    }

    fn compile_named_arg(&self, args: &[Arg], name: &str, default: &str) -> Result<String> {
        for arg in args {
            if let Arg::Named { name: n, value } = arg {
                if n == name {
                    return self.compile_expr(value);
                }
            }
        }
        Ok(default.to_string())
    }

    /// Resolve tint color from args. Supports named colors (gold, red, etc.) or vec3f.
    fn compile_tint_color(&self, args: &[Arg]) -> Result<String> {
        if let Some(arg) = args.first() {
            match arg {
                Arg::Positional(expr) => self.compile_expr(expr),
                Arg::Named { value, .. } => self.compile_expr(value),
            }
        } else {
            Ok("vec3f(1.0)".to_string())
        }
    }

    fn compile_expr(&self, expr: &Expr) -> Result<String> {
        match expr {
            Expr::Number(n) => {
                if n.fract() == 0.0 {
                    Ok(format!("{n:.1}"))
                } else {
                    Ok(format!("{n}"))
                }
            }
            Expr::String(s) => Ok(format!("\"{s}\"")),
            Expr::Ident(name) => Ok(compile_ident(name)),
            Expr::FieldAccess { object, field } => {
                let obj = self.compile_expr(object)?;
                Ok(format!("{obj}.{field}"))
            }
            Expr::BinaryOp { left, op, right } => {
                let l = self.compile_expr(left)?;
                let r = self.compile_expr(right)?;
                let op_str = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "/",
                    BinOp::Gt => ">",
                    BinOp::Lt => "<",
                };
                Ok(format!("({l} {op_str} {r})"))
            }
            Expr::Negate(inner) => {
                let s = self.compile_expr(inner)?;
                Ok(format!("(-{s})"))
            }
            Expr::Call(call) => self.compile_call(call),
            Expr::Array(elements) => {
                let compiled: Result<Vec<String>> =
                    elements.iter().map(|e| self.compile_expr(e)).collect();
                let compiled = compiled?;
                match compiled.len() {
                    2 => Ok(format!("vec2f({})", compiled.join(", "))),
                    3 => Ok(format!("vec3f({})", compiled.join(", "))),
                    4 => Ok(format!("vec4f({})", compiled.join(", "))),
                    _ => Ok(format!("array({})", compiled.join(", "))),
                }
            }
            Expr::Ternary { condition, if_true, if_false } => {
                let cond = self.compile_expr(condition)?;
                let t = self.compile_expr(if_true)?;
                let f = self.compile_expr(if_false)?;
                Ok(format!("select({f}, {t}, {cond})"))
            }
        }
    }

    fn compile_call(&self, call: &FnCall) -> Result<String> {
        let args: Result<Vec<String>> = call.args.iter()
            .map(|a| match a {
                Arg::Positional(e) => self.compile_expr(e),
                Arg::Named { value, .. } => self.compile_expr(value),
            })
            .collect();
        let args = args?;

        match call.name.as_str() {
            "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "sqrt" | "abs" | "sign"
            | "floor" | "ceil" | "round" | "fract" | "length" | "normalize" | "exp" | "log"
            | "log2" | "saturate" => Ok(format!("{}({})", call.name, args.join(", "))),
            "pow" | "min" | "max" | "dot" | "cross" | "distance" | "atan2" | "step" => {
                Ok(format!("{}({})", call.name, args.join(", ")))
            }
            "mix" | "lerp" => Ok(format!("mix({})", args.join(", "))),
            "clamp" => Ok(format!("clamp({})", args.join(", "))),
            "smoothstep" => Ok(format!("smoothstep({})", args.join(", "))),
            "mod" => Ok(format!("(({}) % ({}))", args[0], args[1])),
            _ => Err(GameError::unknown_function(&call.name)),
        }
    }
}

// ── Free functions ─────────────────────────────────────────────────────

fn compile_ident(name: &str) -> String {
    match name {
        "time" => "time".to_string(),
        "p" => "p".to_string(),
        "uv" => "input.uv".to_string(),
        "height" => "height".to_string(),
        "pi" => "3.14159265359".to_string(),
        "tau" => "6.28318530718".to_string(),
        "e" => "2.71828182846".to_string(),
        "phi" => "1.61803398875".to_string(),
        // Colors
        "black" => "vec3f(0.0)".to_string(),
        "white" => "vec3f(1.0)".to_string(),
        "red" => "vec3f(1.0, 0.0, 0.0)".to_string(),
        "green" => "vec3f(0.0, 1.0, 0.0)".to_string(),
        "blue" => "vec3f(0.0, 0.0, 1.0)".to_string(),
        "gold" => "vec3f(0.831, 0.686, 0.216)".to_string(),
        "midnight" => "vec3f(0.0, 0.0, 0.1)".to_string(),
        "obsidian" => "vec3f(0.04, 0.04, 0.06)".to_string(),
        "ember" => "vec3f(0.8, 0.2, 0.05)".to_string(),
        "cyan" => "vec3f(0.0, 1.0, 1.0)".to_string(),
        "ivory" => "vec3f(1.0, 0.97, 0.92)".to_string(),
        "frost" => "vec3f(0.85, 0.92, 1.0)".to_string(),
        "orange" => "vec3f(1.0, 0.5, 0.0)".to_string(),
        "deep_blue" => "vec3f(0.0, 0.02, 0.15)".to_string(),
        _ => name.to_string(),
    }
}

/// Compile an AST expression to JavaScript (for runtime modulation).
pub fn compile_expr_js(expr: &Expr) -> String {
    match expr {
        Expr::Number(n) => {
            if n.fract() == 0.0 {
                format!("{n:.1}")
            } else {
                format!("{n}")
            }
        }
        Expr::Ident(name) => match name.as_str() {
            "time" => "time".to_string(),
            _ => name.clone(),
        },
        Expr::FieldAccess { object, field } => {
            let obj = compile_expr_js(object);
            match (obj.as_str(), field.as_str()) {
                ("audio", "bass") => "audioBass".to_string(),
                ("audio", "mid") => "audioMid".to_string(),
                ("audio", "treble") => "audioTreble".to_string(),
                ("audio", "energy") => "audioEnergy".to_string(),
                ("audio", "beat") => "audioBeat".to_string(),
                ("mouse", "x") => "mouseX".to_string(),
                ("mouse", "y") => "mouseY".to_string(),
                ("data", f) => format!("data_{f}"),
                _ => format!("{obj}_{field}"),
            }
        }
        Expr::BinaryOp { left, op, right } => {
            let l = compile_expr_js(left);
            let r = compile_expr_js(right);
            let op_str = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Gt => ">",
                BinOp::Lt => "<",
            };
            format!("({l} {op_str} {r})")
        }
        Expr::Negate(inner) => {
            let s = compile_expr_js(inner);
            format!("(-{s})")
        }
        Expr::Call(call) => {
            let args: Vec<String> = call.args.iter()
                .map(|a| match a {
                    Arg::Positional(e) => compile_expr_js(e),
                    Arg::Named { value, .. } => compile_expr_js(value),
                })
                .collect();
            format!("Math.{}({})", call.name, args.join(", "))
        }
        _ => "0".to_string(),
    }
}

/// Check if an expression references audio signals.
fn expr_uses_audio(expr: &Expr) -> bool {
    match expr {
        Expr::Ident(s) => s == "audio",
        Expr::FieldAccess { object, .. } => expr_uses_audio(object),
        Expr::BinaryOp { left, right, .. } => expr_uses_audio(left) || expr_uses_audio(right),
        Expr::Negate(inner) => expr_uses_audio(inner),
        Expr::Call(call) => call.args.iter().any(|a| match a {
            Arg::Positional(e) | Arg::Named { value: e, .. } => expr_uses_audio(e),
        }),
        _ => false,
    }
}

/// Check if an expression references data signals.
fn expr_uses_data(expr: &Expr) -> bool {
    match expr {
        Expr::Ident(s) => s == "data",
        Expr::FieldAccess { object, .. } => expr_uses_data(object),
        Expr::BinaryOp { left, right, .. } => expr_uses_data(left) || expr_uses_data(right),
        Expr::Negate(inner) => expr_uses_data(inner),
        Expr::Call(call) => call.args.iter().any(|a| match a {
            Arg::Positional(e) | Arg::Named { value: e, .. } => expr_uses_data(e),
        }),
        _ => false,
    }
}

/// Collect `data.*` field names from an expression.
fn collect_data_fields_into(expr: &Expr, fields: &mut Vec<String>) {
    match expr {
        Expr::FieldAccess { object, field } => {
            if let Expr::Ident(obj) = object.as_ref() {
                if obj == "data" && !fields.contains(field) {
                    fields.push(field.clone());
                }
            }
            collect_data_fields_into(object, fields);
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_data_fields_into(left, fields);
            collect_data_fields_into(right, fields);
        }
        Expr::Negate(inner) => collect_data_fields_into(inner, fields),
        Expr::Call(call) => {
            for arg in &call.args {
                match arg {
                    Arg::Positional(e) | Arg::Named { value: e, .. } => {
                        collect_data_fields_into(e, fields);
                    }
                }
            }
        }
        _ => {}
    }
}

/// Check if an expression references mouse signals.
fn expr_uses_mouse(expr: &Expr) -> bool {
    match expr {
        Expr::Ident(s) => s == "mouse",
        Expr::FieldAccess { object, .. } => expr_uses_mouse(object),
        Expr::BinaryOp { left, right, .. } => expr_uses_mouse(left) || expr_uses_mouse(right),
        Expr::Negate(inner) => expr_uses_mouse(inner),
        Expr::Call(call) => call.args.iter().any(|a| match a {
            Arg::Positional(e) | Arg::Named { value: e, .. } => expr_uses_mouse(e),
        }),
        _ => false,
    }
}

/// Extract a numeric value from an expression (for base_value).
fn extract_number(expr: &Expr) -> Option<f64> {
    match expr {
        Expr::Number(n) => Some(*n),
        Expr::Negate(inner) => extract_number(inner).map(|n| -n),
        _ => None,
    }
}

/// Extract audio file path from cinematic properties.
fn extract_audio_file(cinematic: &Cinematic) -> Option<String> {
    cinematic.properties.iter().find_map(|p| {
        if p.name == "audio" {
            if let Expr::String(s) = &p.value {
                return Some(s.clone());
            }
        }
        None
    })
}

/// Determine rendering mode from the first lens block.
fn determine_render_mode(cinematic: &Cinematic) -> RenderMode {
    if let Some(lens) = cinematic.lenses.first() {
        // Check for mode: raymarch
        let is_raymarch = lens.properties.iter().any(|p| {
            p.name == "mode" && matches!(&p.value, Expr::Ident(s) if s == "raymarch")
        });

        if is_raymarch {
            // Extract camera params
            let (radius, height, speed) = extract_camera_params(lens);
            return RenderMode::Raymarch {
                cam_radius: radius,
                cam_height: height,
                cam_speed: speed,
            };
        }
    }
    RenderMode::Flat
}

fn extract_camera_params(lens: &Lens) -> (f64, f64, f64) {
    for prop in &lens.properties {
        if prop.name == "camera" {
            if let Expr::Call(call) = &prop.value {
                if call.name == "orbit" {
                    let radius = extract_named_number(&call.args, "radius", 5.0);
                    let height = extract_named_number(&call.args, "height", 2.0);
                    let speed = extract_named_number(&call.args, "speed", 0.05);
                    return (radius, height, speed);
                }
            }
        }
    }
    (5.0, 2.0, 0.05)
}

fn extract_named_number(args: &[Arg], name: &str, default: f64) -> f64 {
    for arg in args {
        if let Arg::Named { name: n, value } = arg {
            if n == name {
                return extract_number(value).unwrap_or(default);
            }
        }
    }
    default
}

// ── Types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum ShaderState {
    Position,
    Sdf,
    Glow,
    Color,
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;
    use crate::parser::Parser;

    fn compile(src: &str) -> String {
        let tokens = lexer::lex(src).expect("lex failed");
        let mut parser = Parser::new(tokens);
        let cin = parser.parse().expect("parse failed");
        generate_wgsl(&cin).expect("codegen failed")
    }

    fn compile_full_output(src: &str) -> CompileOutput {
        let tokens = lexer::lex(src).expect("lex failed");
        let mut parser = Parser::new(tokens);
        let cin = parser.parse().expect("parse failed");
        generate_full(&cin).expect("codegen failed")
    }

    #[test]
    fn codegen_hello_game() {
        let wgsl = compile(
            r#"cinematic "Hello" {
                layer {
                    fn: circle(0.3 + sin(time) * 0.05) | glow(2.0)
                }
            }"#,
        );

        assert!(wgsl.contains("struct Uniforms"));
        assert!(wgsl.contains("fn vs_main"));
        assert!(wgsl.contains("fn fs_main"));
        assert!(wgsl.contains("sdf_circle"));
        assert!(wgsl.contains("apply_glow"));
        assert!(wgsl.contains("(0.3 + (sin(time) * 0.05))"));
        assert!(wgsl.contains("2.0"));
    }

    #[test]
    fn codegen_produces_valid_structure() {
        let wgsl = compile("cinematic { layer { fn: circle(0.5) | glow(1.0) } }");

        let uni_pos = wgsl.find("struct Uniforms").unwrap();
        let vs_pos = wgsl.find("fn vs_main").unwrap();
        let fs_pos = wgsl.find("fn fs_main").unwrap();
        assert!(uni_pos < vs_pos);
        assert!(vs_pos < fs_pos);
    }

    #[test]
    fn codegen_audio_uniforms_present() {
        let wgsl = compile("cinematic { layer { fn: circle(0.5) } }");
        assert!(wgsl.contains("audio_bass: f32"));
        assert!(wgsl.contains("audio_energy: f32"));
        assert!(wgsl.contains("resolution: vec2f"));
        assert!(wgsl.contains("mouse: vec2f"));
    }

    #[test]
    fn codegen_params_collected() {
        let out = compile_full_output(
            r#"cinematic {
                layer x {
                    fn: circle(radius)
                    radius: 0.3 ~ audio.bass * 0.2
                    intensity: 2.0 ~ audio.energy * 3.0
                }
            }"#,
        );

        assert_eq!(out.params.len(), 2);
        assert_eq!(out.params[0].name, "radius");
        assert_eq!(out.params[0].base_value, 0.3);
        assert!(out.params[0].mod_js.is_some());
        assert_eq!(out.params[1].name, "intensity");
        assert!(out.uses_audio);
    }

    #[test]
    fn codegen_param_uniform_emitted() {
        let wgsl = compile(
            r#"cinematic {
                layer x {
                    fn: circle(radius)
                    radius: 0.3 ~ audio.bass * 0.2
                }
            }"#,
        );

        // Param should appear in uniform struct
        assert!(wgsl.contains("p_radius: f32"));
        // Param should be bound as let in fragment
        assert!(wgsl.contains("let radius = u.p_radius;"));
    }

    #[test]
    fn codegen_js_expression() {
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::FieldAccess {
                object: Box::new(Expr::Ident("audio".to_string())),
                field: "bass".to_string(),
            }),
            op: BinOp::Mul,
            right: Box::new(Expr::Number(2.0)),
        };
        let js = compile_expr_js(&expr);
        assert_eq!(js, "(audioBass * 2.0)");
    }

    #[test]
    fn codegen_fbm_with_persistence() {
        let wgsl = compile(
            r#"cinematic {
                layer { fn: fbm(p, octaves: 4, persistence: 0.6) }
            }"#,
        );
        assert!(wgsl.contains("fbm2(p, i32(4.0), 0.6, 2.0)"));
    }

    #[test]
    fn codegen_shade_with_albedo() {
        let wgsl = compile(
            r#"cinematic {
                layer { fn: circle(0.5) | shade(albedo: gold) }
            }"#,
        );
        assert!(wgsl.contains("shade_albedo"));
        assert!(wgsl.contains("vec3f(0.831, 0.686, 0.216)"));
    }

    #[test]
    fn codegen_raymarch_mode() {
        let out = compile_full_output(
            r#"cinematic {
                layer terrain {
                    fn: fbm(p * 2.0, octaves: 6, persistence: 0.5)
                }
                lens {
                    mode: raymarch
                    camera: orbit(radius: 4.0, height: 2.0, speed: 0.05)
                }
            }"#,
        );

        assert!(matches!(out.render_mode, RenderMode::Raymarch { .. }));
        assert!(out.wgsl.contains("fn field_at"));
        assert!(out.wgsl.contains("fn map_scene"));
        assert!(out.wgsl.contains("fn calc_normal"));
        assert!(out.wgsl.contains("cam_pos"));
    }
}
