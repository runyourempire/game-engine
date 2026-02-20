use super::{WgslGen, ShaderState};
use crate::ast::*;
use crate::error::{GameError, Result};

impl WgslGen {
    // ── Flat mode fragment shader ──────────────────────────────────────

    pub(super) fn emit_flat_fragment(&mut self, layer: &Layer) -> Result<()> {
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
}
