use crate::ast::*;
use crate::error::Result;

mod builtins;
mod stages;
mod raymarch;
mod expr;
mod analysis;
pub mod glsl;
pub mod resonance;
pub mod react;

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
    /// Arc timeline — moments with parameter transitions over time.
    pub arc_moments: Vec<CompiledMoment>,
    /// Compiler warnings (non-fatal issues the user should know about).
    pub warnings: Vec<String>,
    /// Resonance JS code (cross-layer param modulation). Empty if no resonate block.
    pub resonance_js: String,
    /// React JS code (event listeners for user interaction). Empty if no react block.
    pub react_js: String,
    /// Number of layers in the cinematic.
    pub layer_count: usize,
    /// GLSL ES 3.0 vertex shader for WebGL2 fallback.
    pub glsl_vertex: String,
    /// GLSL ES 3.0 fragment shader for WebGL2 fallback.
    pub glsl_fragment: String,
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

/// A compiled arc moment — a named point in the timeline.
#[derive(Debug, Clone)]
pub struct CompiledMoment {
    pub time_seconds: f64,
    pub name: Option<String>,
    pub transitions: Vec<CompiledTransition>,
}

/// A compiled arc transition — targets a specific param by index.
#[derive(Debug, Clone)]
pub struct CompiledTransition {
    /// Index into `CompileOutput.params`. None if target couldn't be resolved.
    pub param_index: usize,
    pub target_value: f64,
    pub is_animated: bool,
    /// Easing function name (linear, expo_in, expo_out, cubic_in_out, smooth, elastic, bounce).
    pub easing: String,
    /// Transition duration in seconds. None means "until next moment".
    pub duration_secs: Option<f64>,
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

// ── X-Ray variant ─────────────────────────────────────────────────────

/// A single x-ray variant: WGSL for a truncated pipe chain.
#[derive(Debug, Clone)]
pub struct XrayVariant {
    pub layer_index: usize,
    pub layer_name: String,
    /// Stage index (0-based) — the last stage included in this variant.
    pub stage_index: usize,
    pub stage_name: String,
    pub wgsl: String,
}

/// Generate x-ray variants: one WGSL shader per chain prefix per layer.
/// Each variant truncates one layer's chain at stage K while keeping all
/// other layers fully rendered. Uniform struct is identical across all.
pub fn generate_xray_variants(cinematic: &Cinematic) -> Result<Vec<XrayVariant>> {
    let mut cinematic = cinematic.clone();
    expand_defines(&mut cinematic);

    let mut variants = Vec::new();

    for (layer_idx, layer) in cinematic.layers.iter().enumerate() {
        let chain = match &layer.fn_chain {
            Some(c) => c,
            None => continue,
        };

        for stage_idx in 0..chain.stages.len() {
            let mut modified = cinematic.clone();
            if let Some(ref mut c) = modified.layers[layer_idx].fn_chain {
                c.stages.truncate(stage_idx + 1);
            }

            let output = generate_full(&modified)?;

            variants.push(XrayVariant {
                layer_index: layer_idx,
                layer_name: layer
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("layer_{}", layer_idx)),
                stage_index: stage_idx,
                stage_name: chain.stages[stage_idx].name.clone(),
                wgsl: output.wgsl,
            });
        }
    }

    Ok(variants)
}

// ── Public API ─────────────────────────────────────────────────────────

/// Backward-compatible: compile to WGSL string only.
pub fn generate_wgsl(cinematic: &Cinematic) -> Result<String> {
    let output = generate_full(cinematic)?;
    Ok(output.wgsl)
}

/// Full compilation: WGSL + metadata for the runtime.
pub fn generate_full(cinematic: &Cinematic) -> Result<CompileOutput> {
    let mut gen = WgslGen::new();
    let mut warnings = Vec::new();

    // Expand define calls before any other processing
    let mut cinematic = cinematic.clone();
    expand_defines(&mut cinematic);

    // (resonance and react blocks are now compiled)

    // Validate pipe chain ordering for all layers (after define expansion)
    for layer in &cinematic.layers {
        if let Some(chain) = &layer.fn_chain {
            validate_pipe_chain(chain, &mut warnings);
        }
    }

    // Collect params from all layers
    gen.collect_params(&cinematic, &mut warnings);

    // Validate identifiers in expressions (after defines are expanded and params collected)
    let ident_warnings = validate_identifiers(&cinematic, &gen.params);
    warnings.extend(ident_warnings);

    // Determine rendering mode from lens block
    gen.render_mode = determine_render_mode(&cinematic);

    // (Raymarch mode now supports multi-layer SDF compositing)

    // Generate WGSL
    gen.generate(&cinematic)?;

    let title = cinematic.name.clone().unwrap_or_else(|| "Untitled".to_string());
    let audio_file = extract_audio_file(&cinematic);
    let param_count = gen.params.len();

    // Compile arc timeline
    let arc_moments = compile_arc(&cinematic, &gen.params);

    // Compile resonance block
    let resonance_js = if let Some(res_block) = &cinematic.resonance {
        // Warn about bindings with unresolvable targets
        let param_names: Vec<&str> = gen.params.iter().map(|p| p.name.as_str()).collect();
        for binding in &res_block.bindings {
            let target_name = binding.target.rsplit('.').next().unwrap_or(&binding.target);
            if !param_names.contains(&target_name) {
                warnings.push(format!(
                    "resonate: target '{}' does not match any known param — \
                     this binding will be silently ignored",
                    binding.target
                ));
            }
        }
        let compiled = resonance::compile_resonance(res_block, &gen.params);
        compiled.js_code
    } else {
        String::new()
    };

    // Compile react block
    let react_js = if let Some(react_block) = &cinematic.react {
        react::compile_react(react_block, &gen.params)
    } else {
        String::new()
    };

    // Generate GLSL fallback shaders
    let param_fields: Vec<String> = gen.params.iter()
        .map(|p| p.uniform_field.clone())
        .collect();
    let (glsl_vertex, glsl_fragment) = glsl::wgsl_to_glsl(&gen.output, &param_fields);

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
        arc_moments,
        warnings,
        resonance_js,
        react_js,
        layer_count: cinematic.layers.len(),
        glsl_vertex,
        glsl_fragment,
    })
}

/// Compile arc block into resolved moments with param indices.
fn compile_arc(cinematic: &Cinematic, params: &[CompiledParam]) -> Vec<CompiledMoment> {
    let arc = match &cinematic.arc {
        Some(a) => a,
        None => return Vec::new(),
    };

    arc.moments
        .iter()
        .map(|moment| {
            let transitions = moment
                .transitions
                .iter()
                .filter_map(|t| {
                    // Resolve "layer.param" or just "param" to a param index
                    let param_name = resolve_transition_target(&t.target);
                    let param_idx = params.iter().position(|p| p.name == param_name);
                    let param_idx = param_idx?; // skip unresolvable targets

                    let target_value = extract_number(&t.value).unwrap_or(0.0);
                    let easing = t.easing.clone().unwrap_or_else(|| "linear".to_string());

                    Some(CompiledTransition {
                        param_index: param_idx,
                        target_value,
                        is_animated: t.is_animated,
                        easing,
                        duration_secs: t.duration_secs,
                    })
                })
                .collect();

            CompiledMoment {
                time_seconds: moment.time_seconds,
                name: moment.name.clone(),
                transitions,
            }
        })
        .collect()
}

/// Extract the param name from a transition target like "terrain.scale" or "scale".
fn resolve_transition_target(target: &str) -> &str {
    // "layer.param" → "param", "param" → "param"
    target.rsplit('.').next().unwrap_or(target)
}

/// Validate pipe chain stage ordering and emit warnings for likely mistakes.
fn validate_pipe_chain(chain: &PipeChain, warnings: &mut Vec<String>) {
    if chain.stages.is_empty() {
        return;
    }

    let first = &chain.stages[0].name;
    let first_kind = classify_stage_kind(first);

    // First stage should produce geometry (SDF) or transform position
    if matches!(first_kind, StageKind::Glow | StageKind::PostProcess) {
        warnings.push(format!(
            "pipe chain starts with '{}' which expects a prior SDF stage; \
             consider starting with a shape (circle, box, ring, etc.)",
            first
        ));
    }

    // Track what state we've seen to catch ordering issues
    let mut has_sdf = false;
    let mut has_color = false;

    for stage in &chain.stages {
        let kind = classify_stage_kind(&stage.name);
        match kind {
            StageKind::Sdf | StageKind::Position => has_sdf = true,
            StageKind::Glow => {
                if !has_sdf {
                    warnings.push(format!(
                        "'{}' appears before any SDF shape — it needs an SDF input",
                        stage.name
                    ));
                }
            }
            StageKind::Color => has_color = true,
            StageKind::PostProcess => {
                if !has_color && !has_sdf {
                    warnings.push(format!(
                        "post-process '{}' appears before any visual content",
                        stage.name
                    ));
                }
            }
            StageKind::Unknown => {}
        }
    }
}

/// Lightweight stage classification for validation (separate from ShaderState).
enum StageKind {
    Position,
    Sdf,
    Glow,
    Color,
    PostProcess,
    Unknown,
}

fn classify_stage_kind(name: &str) -> StageKind {
    match name {
        "translate" | "rotate" | "scale" | "repeat" | "mirror" | "twist"
            => StageKind::Position,
        "circle" | "sphere" | "ring" | "box" | "torus" | "cylinder" | "plane"
        | "line" | "polygon" | "star" | "fbm" | "simplex" | "voronoi" | "noise"
        | "mask_arc" | "displace" | "round" | "onion"
        | "curl_noise" | "concentric_waves" | "threshold"
        | "smooth_union" | "smooth_subtract" | "smooth_intersect"
            => StageKind::Sdf,
        "glow" => StageKind::Glow,
        "shade" | "emissive" | "colormap" | "spectrum" | "tint" | "gradient"
        | "particles"
            => StageKind::Color,
        "bloom" | "chromatic" | "vignette" | "grain" | "fog" | "glitch"
        | "scanlines" | "tonemap" | "invert" | "saturate_color" | "iridescent"
        | "color_grade"
            => StageKind::PostProcess,
        _ => StageKind::Unknown,
    }
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

    fn collect_params(&mut self, cinematic: &Cinematic, warnings: &mut Vec<String>) {
        for layer in &cinematic.layers {
            let layer_name = layer.name.as_deref().unwrap_or("unnamed");

            // Collect explicit params (with ~ modulation)
            for param in &layer.params {
                // Check for cross-layer duplicate param names
                if self.params.iter().any(|p| p.name == param.name) {
                    warnings.push(format!(
                        "param '{}' in layer '{}' duplicates a param from an earlier layer; \
                         use unique names per layer to avoid invalid WGSL",
                        param.name, layer_name
                    ));
                    continue;
                }

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

            // Promote numeric layer properties to params (no modulation).
            // This ensures properties like `intensity: 2.0` become WGSL uniforms
            // that can be referenced in fn chains and targeted by arcs.
            for prop in &layer.properties {
                if let Some(num) = extract_number(&prop.value) {
                    // Skip non-numeric or already-collected names
                    if self.params.iter().any(|p| p.name == prop.name) {
                        continue;
                    }
                    let idx = SYSTEM_FLOAT_COUNT + self.params.len();
                    let uniform_field = format!("p_{}", prop.name);
                    self.params.push(CompiledParam {
                        name: prop.name.clone(),
                        uniform_field,
                        buffer_index: idx,
                        base_value: num,
                        mod_js: None,
                    });
                }
            }
        }
    }

    // ── Main generation ────────────────────────────────────────────────

    fn generate(&mut self, cinematic: &Cinematic) -> Result<()> {
        // Phase 1: Emit fragment shader to a temp buffer to collect used_builtins
        let saved_output = std::mem::take(&mut self.output);

        match &self.render_mode {
            RenderMode::Flat => {
                // Collect layers that have fn chains
                let flat_layers: Vec<&Layer> = cinematic.layers.iter()
                    .filter(|l| l.fn_chain.is_some())
                    .collect();

                if flat_layers.is_empty() {
                    self.emit_empty_fragment();
                } else if flat_layers.len() == 1 {
                    self.emit_flat_fragment(flat_layers[0])?;
                } else {
                    self.emit_multi_layer_fragment(&flat_layers)?;
                }
            }
            RenderMode::Raymarch { cam_radius, cam_height, cam_speed } => {
                let cr = *cam_radius;
                let ch = *cam_height;
                let cs = *cam_speed;
                // Extract post-processing stages from the lens block
                let post_stages: Vec<crate::ast::FnCall> = cinematic.lenses.first()
                    .map(|lens| lens.post.clone())
                    .unwrap_or_default();
                // Collect all layers with fn chains for multi-layer SDF compositing
                let raymarch_layers: Vec<&Layer> = cinematic.layers.iter()
                    .filter(|l| l.fn_chain.is_some())
                    .collect();
                if raymarch_layers.is_empty() {
                    self.emit_empty_fragment();
                } else {
                    self.emit_raymarch_helpers(&raymarch_layers)?;
                    self.emit_raymarch_fragment(&raymarch_layers, cr, ch, cs, &post_stages)?;
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
            | "fog" | "glitch" | "scanlines" | "tonemap" | "invert" | "saturate_color"
            | "color_grade")
    }

    pub(super) fn classify_stage(&self, name: &str) -> Result<ShaderState> {
        match name {
            "circle" | "sphere" | "ring" | "box" | "torus" | "cylinder" | "plane"
            | "line" | "polygon" | "star" => Ok(ShaderState::Sdf),
            "glow" => Ok(ShaderState::Glow),
            "shade" | "emissive" | "colormap" | "spectrum" | "tint"
            | "gradient" | "particles" => Ok(ShaderState::Color),
            "fbm" | "simplex" | "voronoi" | "noise"
            | "curl_noise" | "concentric_waves" => Ok(ShaderState::Sdf),
            "mask_arc" | "threshold" => Ok(ShaderState::Sdf),
            "translate" | "rotate" | "scale" | "repeat" | "mirror" | "twist"
            => Ok(ShaderState::Position),
            "displace" | "round" | "onion"
            | "smooth_union" | "smooth_subtract" | "smooth_intersect" => Ok(ShaderState::Sdf),
            "bloom" | "chromatic" | "vignette" | "grain"
            | "fog" | "glitch" | "scanlines" | "tonemap" | "invert"
            | "saturate_color" | "iridescent" | "color_grade" => Ok(ShaderState::Color),
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

// ── Identifier validation ──────────────────────────────────────────

/// Known built-in identifiers that are always valid in expressions.
const KNOWN_BUILTINS: &[&str] = &[
    "time", "p", "uv", "height", "pi", "tau", "e", "phi",
    // Colors
    "black", "white", "red", "green", "blue", "gold", "midnight",
    "obsidian", "ember", "cyan", "ivory", "frost", "orange",
    "deep_blue", "ash", "charcoal", "plasma", "violet", "magenta",
];

/// Objects that are valid as the root of field access expressions.
const KNOWN_FIELD_OBJECTS: &[&str] = &["audio", "mouse", "data", "arc"];

/// Collect all identifiers used in an expression tree.
fn collect_idents(expr: &Expr, idents: &mut Vec<String>) {
    match expr {
        Expr::Ident(name) => {
            if !idents.contains(name) {
                idents.push(name.clone());
            }
        }
        Expr::FieldAccess { object, .. } => {
            // Only collect the root object — field access is valid if root is known
            collect_idents(object, idents);
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_idents(left, idents);
            collect_idents(right, idents);
        }
        Expr::Negate(inner) => collect_idents(inner, idents),
        Expr::Call(call) => {
            for arg in &call.args {
                match arg {
                    Arg::Positional(e) | Arg::Named { value: e, .. } => {
                        collect_idents(e, idents);
                    }
                }
            }
        }
        Expr::Array(elements) => {
            for e in elements {
                collect_idents(e, idents);
            }
        }
        Expr::Ternary { condition, if_true, if_false } => {
            collect_idents(condition, idents);
            collect_idents(if_true, idents);
            collect_idents(if_false, idents);
        }
        Expr::Number(_) | Expr::String(_) => {}
    }
}

/// Validate all identifiers used in expressions across the cinematic.
/// Returns warnings for any unrecognized identifiers.
fn validate_identifiers(cinematic: &Cinematic, params: &[CompiledParam]) -> Vec<String> {
    let mut warnings = Vec::new();

    // Build the set of known names
    let param_names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
    let layer_names: Vec<&str> = cinematic.layers.iter()
        .filter_map(|l| l.name.as_deref())
        .collect();
    let define_names: Vec<&str> = cinematic.defines.iter()
        .map(|d| d.name.as_str())
        .collect();

    // Collect all identifiers from all layer expressions
    let mut all_idents = Vec::new();

    for layer in &cinematic.layers {
        // From pipe chain args
        if let Some(chain) = &layer.fn_chain {
            for stage in &chain.stages {
                for arg in &stage.args {
                    match arg {
                        Arg::Positional(e) | Arg::Named { value: e, .. } => {
                            collect_idents(e, &mut all_idents);
                        }
                    }
                }
            }
        }

        // From param modulations
        for param in &layer.params {
            if let Some(modulation) = &param.modulation {
                collect_idents(&modulation.signal, &mut all_idents);
            }
        }
    }

    // Check each identifier against known names
    for ident in &all_idents {
        let name = ident.as_str();
        if KNOWN_BUILTINS.contains(&name) {
            continue;
        }
        if KNOWN_FIELD_OBJECTS.contains(&name) {
            continue;
        }
        if param_names.contains(&name) {
            continue;
        }
        if layer_names.contains(&name) {
            continue;
        }
        if define_names.contains(&name) {
            continue;
        }
        warnings.push(format!(
            "unknown identifier '{}' — not a known builtin, param, layer, or define",
            name
        ));
    }

    warnings
}

#[derive(Debug, Clone)]
pub(super) enum ShaderState {
    Position,
    Sdf,
    Glow,
    Color,
}
