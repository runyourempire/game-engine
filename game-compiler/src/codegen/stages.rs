use super::{WgslGen, ShaderState};
use crate::ast::*;
use crate::error::{GameError, Result};

impl WgslGen {
    // ── Flat mode fragment shader ──────────────────────────────────────

    /// Single-layer flat fragment shader (backwards-compatible).
    pub(super) fn emit_flat_fragment(&mut self, layer: &Layer) -> Result<()> {
        self.line("@fragment");
        self.line("fn fs_main(input: VertexOutput) -> @location(0) vec4f {");
        self.indent += 1;

        self.line("let uv = input.uv * 2.0 - 1.0;");
        self.line("let aspect = u.resolution.x / u.resolution.y;");
        self.line("let time = fract(u.time / 120.0) * 120.0;");
        self.blank();
        self.emit_param_bindings();

        let state = self.emit_flat_chain_stages(layer)?;

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

    /// Multi-layer flat fragment shader (additive compositing).
    pub(super) fn emit_multi_layer_fragment(&mut self, layers: &[&Layer]) -> Result<()> {
        self.line("@fragment");
        self.line("fn fs_main(input: VertexOutput) -> @location(0) vec4f {");
        self.indent += 1;

        self.line("let uv = input.uv * 2.0 - 1.0;");
        self.line("let aspect = u.resolution.x / u.resolution.y;");
        self.line("let time = fract(u.time / 120.0) * 120.0;");
        self.blank();
        self.emit_param_bindings();

        self.line("var final_color = vec4f(0.0, 0.0, 0.0, 1.0);");
        self.blank();

        for (i, layer) in layers.iter().enumerate() {
            let name = layer.name.as_deref().unwrap_or("unnamed");
            self.line(&format!("// ── Layer {i}: {name} ──"));
            self.line("{");
            self.indent += 1;

            let state = self.emit_flat_chain_stages(layer)?;

            // Extract layer color from final shader state
            match state {
                ShaderState::Sdf => {
                    self.used_builtins.insert("apply_glow");
                    self.line("let lc = vec3f(apply_glow(sdf_result, 2.0));");
                }
                ShaderState::Glow => {
                    self.line("let lc = vec3f(glow_result);");
                }
                ShaderState::Color => {
                    self.line("let lc = color_result.rgb;");
                }
                ShaderState::Position => {
                    self.line("let lc = vec3f(0.0);");
                }
            }
            // Composite using blend mode
            let opacity = layer.blend_opacity.unwrap_or(1.0);
            let blend_mode = layer.blend_mode.unwrap_or(crate::ast::BlendMode::Additive);
            match blend_mode {
                crate::ast::BlendMode::Additive => {
                    self.line(&format!(
                        "final_color = vec4f(final_color.rgb + lc * {opacity:.3}, 1.0);"
                    ));
                }
                crate::ast::BlendMode::Multiply => {
                    self.line(&format!(
                        "final_color = vec4f(mix(final_color.rgb, final_color.rgb * lc, {opacity:.3}), 1.0);"
                    ));
                }
                crate::ast::BlendMode::Screen => {
                    self.line(&format!(
                        "final_color = vec4f(mix(final_color.rgb, 1.0 - (1.0 - final_color.rgb) * (1.0 - lc), {opacity:.3}), 1.0);"
                    ));
                }
                crate::ast::BlendMode::Overlay => {
                    self.line("let ov_sel = select(1.0 - 2.0 * (1.0 - final_color.rgb) * (1.0 - lc), 2.0 * final_color.rgb * lc, final_color.rgb < vec3f(0.5));");
                    self.line(&format!(
                        "final_color = vec4f(mix(final_color.rgb, ov_sel, {opacity:.3}), 1.0);"
                    ));
                }
                crate::ast::BlendMode::Normal => {
                    self.line(&format!(
                        "final_color = vec4f(mix(final_color.rgb, lc, {opacity:.3}), 1.0);"
                    ));
                }
            }

            self.indent -= 1;
            self.line("}");
            self.blank();
        }

        self.line("return final_color;");

        self.indent -= 1;
        self.line("}");

        Ok(())
    }

    /// Emit the pipe chain stages for a single layer. Returns final ShaderState.
    /// Declares its own `var p` so it works inside block scopes for multi-layer.
    fn emit_flat_chain_stages(&mut self, layer: &Layer) -> Result<ShaderState> {
        let chain = layer.fn_chain.as_ref().ok_or_else(|| GameError {
            kind: crate::error::ErrorKind::Message("layer has no fn: chain".into()),
            span: None,
            source_text: None,
        })?;

        self.line("var p = vec2f(uv.x * aspect, uv.y);");

        let mut state = ShaderState::Position;
        let mut has_scale = false;
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

            // Track scale for SDF correction
            if stage.name == "scale" {
                has_scale = true;
            }

            self.emit_flat_stage(stage, &state, i)?;

            // Apply scale correction after SDF stages
            if has_scale && matches!(next_state, ShaderState::Sdf) {
                self.line("sdf_result *= scale_factor;");
                self.blank();
                has_scale = false;
            }

            state = next_state;
        }

        Ok(state)
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
                let oct = self.compile_int_arg(&stage.args, "octaves", "6")?;
                let per = self.compile_named_arg(&stage.args, "persistence", "0.5")?;
                let lac = self.compile_named_arg(&stage.args, "lacunarity", "2.0")?;
                self.used_builtins.insert("fbm2");
                self.line(&format!(
                    "var sdf_result = fbm2({pos}, {oct}, {per}, {lac});"
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
                self.line(&format!("let pp_bloom = max(pp_lum - {thresh}, 0.0);"));
                self.line("let pp_bloom_color = color_result.rgb * pp_bloom;");
                self.line(&format!("color_result = vec4f(color_result.rgb + pp_bloom_color * {intensity}, 1.0);"));
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

            // ── New SDF primitives ────────────────────────────────────
            "box" => {
                let w = self.compile_arg(&stage.args, 0, "0.5")?;
                let h = self.compile_arg(&stage.args, 1, "0.5")?;
                self.used_builtins.insert("sdf_box2");
                self.line(&format!("var sdf_result = sdf_box2(p, vec2f({w}, {h}));"));
            }
            "torus" => {
                let big_r = self.compile_arg(&stage.args, 0, "0.3")?;
                let small_r = self.compile_arg(&stage.args, 1, "0.05")?;
                self.line(&format!("var sdf_result = abs(length(p) - {big_r}) - {small_r};"));
            }
            "line" => {
                let x1 = self.compile_arg(&stage.args, 0, "-0.5")?;
                let y1 = self.compile_arg(&stage.args, 1, "0.0")?;
                let x2 = self.compile_arg(&stage.args, 2, "0.5")?;
                let y2 = self.compile_arg(&stage.args, 3, "0.0")?;
                let thickness = self.compile_arg(&stage.args, 4, "0.02")?;
                self.used_builtins.insert("sdf_line");
                self.line(&format!(
                    "var sdf_result = sdf_line(p, vec2f({x1}, {y1}), vec2f({x2}, {y2})) - {thickness};"
                ));
            }
            "polygon" => {
                let sides = self.compile_arg(&stage.args, 0, "6.0")?;
                let radius = self.compile_arg(&stage.args, 1, "0.3")?;
                self.used_builtins.insert("sdf_polygon");
                self.line(&format!("var sdf_result = sdf_polygon(p, {sides}, {radius});"));
            }
            "star" => {
                let points = self.compile_arg(&stage.args, 0, "5.0")?;
                let outer = self.compile_arg(&stage.args, 1, "0.4")?;
                let inner = self.compile_arg(&stage.args, 2, "0.2")?;
                self.used_builtins.insert("sdf_star");
                self.line(&format!("var sdf_result = sdf_star(p, {points}, {outer}, {inner});"));
            }

            // ── New domain operations ─────────────────────────────────
            "scale" => {
                let s = self.compile_arg(&stage.args, 0, "1.0")?;
                self.line(&format!("p = p / {s};"));
                // Note: after the SDF is computed, we need to scale the result back.
                // We store the scale factor for post-multiply. We emit a variable so
                // subsequent SDF stages can reference it.
                self.line(&format!("let scale_factor = {s};"));
            }
            "repeat" => {
                let spacing = self.compile_arg(&stage.args, 0, "1.0")?;
                self.line(&format!("p = p - {spacing} * round(p / {spacing});"));
            }
            "mirror" => {
                let axis = self.compile_arg(&stage.args, 0, "\"xy\"")?;
                // axis is a string expression like "x", "y", or "xy"
                match axis.trim_matches('"') {
                    "x" => self.line("p = vec2f(abs(p.x), p.y);"),
                    "y" => self.line("p = vec2f(p.x, abs(p.y));"),
                    _ => self.line("p = abs(p);"),
                }
            }
            "twist" => {
                let amount = self.compile_arg(&stage.args, 0, "1.0")?;
                self.line(&format!("{{ let tw_a = p.y * {amount};"));
                self.line("let tw_c = cos(tw_a); let tw_s = sin(tw_a);");
                self.line("p = vec2f(p.x * tw_c - p.y * tw_s, p.x * tw_s + p.y * tw_c); }");
            }
            "displace" => {
                let strength = self.compile_arg(&stage.args, 0, "0.1")?;
                self.used_builtins.insert("simplex2");
                self.line(&format!("sdf_result += simplex2(p * 3.0) * {strength};"));
            }
            "round" => {
                let r = self.compile_arg(&stage.args, 0, "0.05")?;
                self.line(&format!("sdf_result -= {r};"));
            }
            "onion" => {
                let thickness = self.compile_arg(&stage.args, 0, "0.02")?;
                self.line(&format!("sdf_result = abs(sdf_result) - {thickness};"));
            }

            // ── New noise stages ──────────────────────────────────────
            "simplex" => {
                let freq = self.compile_arg(&stage.args, 0, "1.0")?;
                self.used_builtins.insert("simplex2");
                self.line(&format!("var sdf_result = simplex2(p * {freq});"));
            }
            "voronoi" => {
                let freq = self.compile_arg(&stage.args, 0, "1.0")?;
                self.used_builtins.insert("voronoi2");
                self.line(&format!("var sdf_result = voronoi2(p * {freq});"));
            }

            // ── New post-processing stages ────────────────────────────
            "fog" => {
                let density = self.compile_arg(&stage.args, 0, "1.0")?;
                let fog_color = self.compile_arg(&stage.args, 1, "vec3f(0.0)")?;
                self.line(&format!(
                    "color_result = vec4f(mix(color_result.rgb, {fog_color}, 1.0 - exp(-length(uv) * {density})), 1.0);"
                ));
            }
            "glitch" => {
                let intensity = self.compile_arg(&stage.args, 0, "0.5")?;
                self.line("let gli_block = floor(input.uv.y * 20.0);");
                self.line("let gli_noise = fract(sin(gli_block * 43758.5453 + floor(time * 8.0)) * 12345.6789);");
                self.line(&format!(
                    "let gli_offset = select(0.0, (gli_noise * 2.0 - 1.0) * {intensity} * 0.1, gli_noise > 0.7);"
                ));
                self.line("let gli_r = fract(sin(dot(input.uv + gli_offset, vec2f(12.9898, 78.233))) * 43758.5453);");
                self.line(&format!(
                    "color_result = vec4f(mix(color_result.rgb, vec3f(gli_r, color_result.g, color_result.b * 0.8), step(0.85, gli_noise) * {intensity}), 1.0);"
                ));
            }
            "scanlines" => {
                let count = self.compile_arg(&stage.args, 0, "100.0")?;
                let intensity = self.compile_arg(&stage.args, 1, "0.3")?;
                self.line(&format!(
                    "color_result = vec4f(color_result.rgb * (1.0 - sin(input.uv.y * {count} * 3.14159) * {intensity}), 1.0);"
                ));
            }
            "tonemap" => {
                let exposure = self.compile_arg(&stage.args, 0, "1.0")?;
                self.line(&format!(
                    "color_result = vec4f(color_result.rgb * {exposure} / (1.0 + color_result.rgb * {exposure}), 1.0);"
                ));
            }
            "invert" => {
                self.line("color_result = vec4f(1.0 - color_result.rgb, 1.0);");
            }
            "saturate_color" => {
                let amount = self.compile_arg(&stage.args, 0, "1.5")?;
                self.line("let sat_lum = dot(color_result.rgb, vec3f(0.299, 0.587, 0.114));");
                self.line(&format!(
                    "color_result = vec4f(mix(vec3f(sat_lum), color_result.rgb, {amount}), 1.0);"
                ));
            }

            // ── New shading stages ────────────────────────────────────
            "gradient" => {
                let color_a = self.compile_arg(&stage.args, 0, "vec3f(0.0)")?;
                let color_b = self.compile_arg(&stage.args, 1, "vec3f(1.0)")?;
                let direction = self.compile_arg(&stage.args, 2, "\"y\"")?;
                let grad_t = match direction.trim_matches('"') {
                    "x" => "input.uv.x",
                    "radial" => "length(uv)",
                    "y" => "input.uv.y",
                    _ => {
                        // Treat as angle in radians
                        // We'll use a simple inline approach
                        "input.uv.y"
                    }
                };
                self.line(&format!(
                    "var color_result = vec4f(mix({color_a}, {color_b}, {grad_t}), 1.0);"
                ));
            }

            // ── Phase 1A: New primitives ─────────────────────────────
            "curl_noise" => {
                let pos = self.compile_arg(&stage.args, 0, "p")?;
                let freq = self.compile_named_arg(&stage.args, "frequency", "3.0")?;
                let amp = self.compile_named_arg(&stage.args, "amplitude", "0.5")?;
                self.used_builtins.insert("curl2");
                self.line(&format!("let curl_offset = curl2({pos}, {freq}, {amp});"));
                self.line("var sdf_result = length(curl_offset) - 0.01;");
            }
            "concentric_waves" => {
                let _origins = self.compile_arg(&stage.args, 0, "p")?;
                let decay = self.compile_named_arg(&stage.args, "decay", "2.0")?;
                let speed = self.compile_named_arg(&stage.args, "speed", "3.0")?;
                self.line(&format!(
                    "var sdf_result = sin(length(p) * 10.0 - time * {speed}) * exp(-length(p) * {decay});"
                ));
            }
            "iridescent" => {
                let strength = self.compile_arg(&stage.args, 0, "0.3")?;
                // Thin-film interference approximation applied to color_result
                self.line(&format!("{{ let iri_angle = atan2(p.y, p.x);"));
                self.line(&format!("let iri_r = length(p);"));
                self.line(&format!("let iri_phase = iri_angle * 3.0 + iri_r * 10.0 + time;"));
                self.line(&format!("let iri_shift = vec3f("));
                self.indent += 1;
                self.line("sin(iri_phase) * 0.5 + 0.5,");
                self.line("sin(iri_phase + 2.094) * 0.5 + 0.5,");
                self.line("sin(iri_phase + 4.189) * 0.5 + 0.5");
                self.indent -= 1;
                self.line(");");
                self.line(&format!(
                    "color_result = vec4f(mix(color_result.rgb, color_result.rgb * iri_shift, {strength}), 1.0); }}"
                ));
            }
            "particles" => {
                let count = self.compile_named_arg(&stage.args, "count", "100.0")?;
                let size = self.compile_named_arg(&stage.args, "size", "2.0")?;
                let color = self.compile_named_arg(&stage.args, "color", "vec3f(0.7)")?;
                let _trail = self.compile_named_arg(&stage.args, "trail", "0.5")?;
                self.used_builtins.insert("particle_field");
                self.line(&format!(
                    "let pf_brightness = particle_field(p, {count}, {size});"
                ));
                self.line(&format!(
                    "var color_result = vec4f({color} * pf_brightness, 1.0);"
                ));
            }
            "threshold" => {
                let value = self.compile_arg(&stage.args, 0, "0.5")?;
                self.line(&format!("sdf_result = step({value}, sdf_result);"));
            }

            // ── Smooth SDF boolean operations ─────────────────────────
            "smooth_union" => {
                let k = self.compile_arg(&stage.args, 0, "0.1")?;
                self.used_builtins.insert("sdf_smooth_union");
                self.line(&format!("sdf_result = sdf_smooth_union(sdf_result, sdf_result, {k});"));
            }
            "smooth_subtract" => {
                let k = self.compile_arg(&stage.args, 0, "0.1")?;
                self.used_builtins.insert("sdf_smooth_subtract");
                self.line(&format!("sdf_result = sdf_smooth_subtract(sdf_result, sdf_result, {k});"));
            }
            "smooth_intersect" => {
                let k = self.compile_arg(&stage.args, 0, "0.1")?;
                self.used_builtins.insert("sdf_smooth_intersect");
                self.line(&format!("sdf_result = sdf_smooth_intersect(sdf_result, sdf_result, {k});"));
            }

            // ── Color grading ─────────────────────────────────────────
            "color_grade" => {
                let contrast = self.compile_arg(&stage.args, 0, "1.0")?;
                let brightness = self.compile_arg(&stage.args, 1, "0.0")?;
                let gamma = self.compile_arg(&stage.args, 2, "1.0")?;
                self.line(&format!("color_result = vec4f((color_result.rgb - 0.5) * {contrast} + 0.5 + {brightness}, 1.0);"));
                self.line(&format!("color_result = vec4f(pow(max(color_result.rgb, vec3f(0.0)), vec3f(1.0 / {gamma})), 1.0);"));
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
