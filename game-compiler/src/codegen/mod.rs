use crate::ast::*;
use crate::error::Result;

mod builtins;
mod stages;
mod raymarch;
mod expr;
mod analysis;

#[cfg(test)]
mod tests;

pub use self::expr::compile_expr_js;
use self::analysis::*;

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

pub(super) struct WgslGen {
    pub(super) output: String,
    pub(super) indent: usize,
    pub(super) params: Vec<CompiledParam>,
    pub(super) uses_audio: bool,
    pub(super) uses_mouse: bool,
    pub(super) uses_data: bool,
    pub(super) data_fields: Vec<String>,
    pub(super) render_mode: RenderMode,
    pub(super) used_builtins: std::collections::HashSet<&'static str>,
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

    pub(super) fn line(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
        self.output.push_str(s);
        self.output.push('\n');
    }

    pub(super) fn blank(&mut self) {
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

    // ── Common helpers ─────────────────────────────────────────────────

    pub(super) fn emit_param_bindings(&mut self) {
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

    pub(super) fn is_postprocess(&self, name: &str) -> bool {
        matches!(name, "bloom" | "vignette" | "chromatic" | "grain"
            | "fog" | "glitch" | "scanlines" | "tonemap" | "invert" | "saturate_color")
    }

    pub(super) fn classify_stage(&self, name: &str) -> Result<ShaderState> {
        match name {
            "circle" | "sphere" | "ring" | "box" | "torus" | "cylinder" | "plane"
            | "line" | "polygon" | "star" => Ok(ShaderState::Sdf),
            "glow" => Ok(ShaderState::Glow),
            "shade" | "emissive" | "colormap" | "spectrum" | "tint"
            | "gradient" => Ok(ShaderState::Color),
            "fbm" | "simplex" | "voronoi" | "noise" => Ok(ShaderState::Sdf),
            "mask_arc" => Ok(ShaderState::Sdf),
            "translate" | "rotate" | "scale" | "repeat" | "mirror" | "twist"
            => Ok(ShaderState::Position),
            "displace" | "round" | "onion" => Ok(ShaderState::Sdf),
            "bloom" | "chromatic" | "vignette" | "grain"
            | "fog" | "glitch" | "scanlines" | "tonemap" | "invert"
            | "saturate_color" => Ok(ShaderState::Color),
            _ => Err(crate::error::GameError::unknown_function(name)),
        }
    }

    pub(super) fn emit_empty_fragment(&mut self) {
        self.line("@fragment");
        self.line("fn fs_main(input: VertexOutput) -> @location(0) vec4f {");
        self.indent += 1;
        self.line("return vec4f(0.0, 0.0, 0.0, 1.0);");
        self.indent -= 1;
        self.line("}");
    }
}

// ── Types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub(super) enum ShaderState {
    Position,
    Sdf,
    Glow,
    Color,
}
