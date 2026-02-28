use super::WgslGen;
use crate::ast::*;
use crate::error::{ErrorKind, GameError, Result};

impl WgslGen {
    // ── Raymarch mode ──────────────────────────────────────────────────

    /// Emit a single `field_at_N(p)` function for the given layer index.
    fn emit_field_at_for_layer(&mut self, layer: &Layer, index: usize) -> Result<()> {
        let chain = layer.fn_chain.as_ref().ok_or_else(|| GameError {
            kind: ErrorKind::Message("layer has no fn: chain".into()),
            span: None,
            source_text: None,
        })?;

        // Find the SDF/field stages (everything before shade)
        let _field_stages: Vec<&FnCall> = chain.stages.iter()
            .take_while(|s| !matches!(s.name.as_str(), "shade" | "emissive" | "colormap"))
            .collect();

        self.line(&format!("fn field_at_{index}(p: vec2f) -> f32 {{"));
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
        if let Some(stage) = _field_stages.first() {
            match stage.name.as_str() {
                "fbm" => {
                    let pos = self.compile_arg(&stage.args, 0, "p")?;
                    let oct = self.compile_int_arg(&stage.args, "octaves", "6")?;
                    let per = self.compile_named_arg(&stage.args, "persistence", "0.5")?;
                    let lac = self.compile_named_arg(&stage.args, "lacunarity", "2.0")?;
                    self.used_builtins.insert("fbm2");
                    self.line(&format!("return fbm2({pos}, {oct}, {per}, {lac});"));
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

        Ok(())
    }

    /// Determine the SDF combination builtin name for a layer based on its blend mode.
    fn blend_mode_to_sdf_builtin(blend_mode: Option<BlendMode>) -> &'static str {
        match blend_mode.unwrap_or(BlendMode::Additive) {
            // Multiply: intersection (keep overlap)
            BlendMode::Multiply => "sdf_smooth_intersect",
            // All others: smooth union (merge surfaces)
            BlendMode::Additive | BlendMode::Normal | BlendMode::Overlay | BlendMode::Screen
                => "sdf_smooth_union",
        }
    }

    pub(super) fn emit_raymarch_helpers(&mut self, layers: &[&Layer]) -> Result<()> {
        // Emit a field_at_N function for each layer
        for (i, layer) in layers.iter().enumerate() {
            self.emit_field_at_for_layer(layer, i)?;
        }

        // For single-layer backward compatibility, also emit field_at() as alias
        if layers.len() == 1 {
            self.line("fn field_at(p: vec2f) -> f32 {");
            self.indent += 1;
            self.line("return field_at_0(p);");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }

        // map_scene: SDF for raymarching
        self.line("fn map_scene(pos: vec3f) -> f32 {");
        self.indent += 1;

        if layers.len() == 1 {
            self.line("return pos.y - field_at_0(pos.xz);");
        } else {
            // Compute per-layer terrain distances
            for i in 0..layers.len() {
                self.line(&format!("let d_{i} = pos.y - field_at_{i}(pos.xz);"));
            }

            // Build combination chain: start with d_0, combine each subsequent layer
            let mut result = "d_0".to_string();
            for i in 1..layers.len() {
                let builtin = Self::blend_mode_to_sdf_builtin(layers[i].blend_mode);
                self.used_builtins.insert(match builtin {
                    "sdf_smooth_intersect" => "sdf_smooth_intersect",
                    _ => "sdf_smooth_union",
                });
                result = format!("{builtin}({result}, d_{i}, 0.1)");
            }
            self.line(&format!("return {result};"));
        }

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

        // soft_shadow: cast ray from hit point toward light
        self.line("fn soft_shadow(ro: vec3f, rd: vec3f, mint: f32, maxt: f32, k: f32) -> f32 {");
        self.indent += 1;
        self.line("var result = 1.0;");
        self.line("var t = mint;");
        self.line("for (var i: i32 = 0; i < 32; i++) {");
        self.indent += 1;
        self.line("let h = map_scene(ro + rd * t);");
        self.line("result = min(result, k * h / t);");
        self.line("t += clamp(h, 0.01, 0.5);");
        self.line("if (t > maxt) { break; }");
        self.indent -= 1;
        self.line("}");
        self.line("return clamp(result, 0.0, 1.0);");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // calc_ao: ambient occlusion
        self.line("fn calc_ao(pos: vec3f, nor: vec3f) -> f32 {");
        self.indent += 1;
        self.line("var occ = 0.0;");
        self.line("var sca = 1.0;");
        self.line("for (var i: i32 = 0; i < 5; i++) {");
        self.indent += 1;
        self.line("let h = 0.01 + 0.12 * f32(i);");
        self.line("let d = map_scene(pos + nor * h);");
        self.line("occ += (h - d) * sca;");
        self.line("sca *= 0.95;");
        self.indent -= 1;
        self.line("}");
        self.line("return clamp(1.0 - 3.0 * occ, 0.0, 1.0);");
        self.indent -= 1;
        self.line("}");
        self.blank();

        Ok(())
    }

    pub(super) fn emit_raymarch_fragment(
        &mut self,
        layers: &[&Layer],
        cam_radius: f64,
        cam_height: f64,
        cam_speed: f64,
        post_stages: &[FnCall],
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
        self.line("let cam_target = vec3f(0.0);");
        self.line("let forward = normalize(cam_target - cam_pos);");
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
        self.line("t += max(d * 0.8, 0.001);  // relaxation factor + min step");
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
        self.blank();

        // Lighting (sun + ambient with soft shadows and AO)
        self.line("let sun_dir = normalize(vec3f(0.5, 0.8, 1.0));");
        self.line("let sun_intensity = 0.8;");
        self.line("let ambient = 0.15;");
        self.line("let ndotl = max(dot(normal, sun_dir), 0.0);");
        self.line("let shadow = soft_shadow(hit_pos + normal * 0.01, sun_dir, 0.02, 10.0, 8.0);");
        self.line("let ao = calc_ao(hit_pos, normal);");
        self.line("let lighting = ndotl * sun_intensity * shadow + ambient * ao;");
        self.blank();

        // Material blending from all layers' shade() stages
        if layers.len() == 1 {
            // Single-layer: backward-compatible path
            self.line("let height = clamp(field_at(hit_pos.xz) * 0.5 + 0.5, 0.0, 1.0);");

            let chain = layers[0].fn_chain.as_ref().ok_or_else(|| {
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
        } else {
            // Multi-layer: blend materials based on SDF proximity
            self.line("// Per-layer SDF distances at hit point for material blending");

            // Collect per-layer materials
            let mut albedo_exprs = Vec::new();
            let mut emissive_exprs = Vec::new();

            for (i, layer) in layers.iter().enumerate() {
                self.line(&format!("let d_hit_{i} = abs(field_at_{i}(hit_pos.xz));"));

                // Extract shade stage from each layer
                let (albedo_expr, emissive_expr) = if let Some(chain) = &layer.fn_chain {
                    let shade_stage = chain.stages.iter().find(|s| s.name == "shade");
                    if let Some(shade) = shade_stage {
                        let a = self.compile_named_arg(&shade.args, "albedo", "vec3f(0.8)")?;
                        let e = self.compile_named_arg(&shade.args, "emissive", "vec3f(0.0)")?;
                        (a, e)
                    } else {
                        ("vec3f(0.8)".to_string(), "vec3f(0.0)".to_string())
                    }
                } else {
                    ("vec3f(0.8)".to_string(), "vec3f(0.0)".to_string())
                };

                self.line(&format!("let albedo_{i} = {albedo_expr};"));
                self.line(&format!("let emissive_{i} = {emissive_expr};"));
                albedo_exprs.push(format!("albedo_{i}"));
                emissive_exprs.push(format!("emissive_{i}"));
            }
            self.blank();

            // Compute proximity weights using exponential falloff
            self.line("// Proximity-weighted material blending");
            for i in 0..layers.len() {
                self.line(&format!("let w_{i} = exp(-d_hit_{i} * 10.0);"));
            }

            // Sum weights
            let weight_sum: Vec<String> = (0..layers.len()).map(|i| format!("w_{i}")).collect();
            self.line(&format!("let w_total = {};", weight_sum.join(" + ")));
            self.blank();

            // Blend albedo
            let albedo_terms: Vec<String> = (0..layers.len())
                .map(|i| format!("{} * w_{i}", albedo_exprs[i]))
                .collect();
            self.line(&format!("let albedo = ({}) / w_total;", albedo_terms.join(" + ")));

            // Blend emissive
            let emissive_terms: Vec<String> = (0..layers.len())
                .map(|i| format!("{} * w_{i}", emissive_exprs[i]))
                .collect();
            self.line(&format!("let emissive_color = ({}) / w_total;", emissive_terms.join(" + ")));

            self.line("var color = albedo * lighting + emissive_color;");
        }
        self.blank();

        // Fog
        self.line("// Distance fog");
        self.line("let fog_amount = 1.0 - exp(-t * 0.03);");
        self.line("color = mix(color, vec3f(0.0, 0.0, 0.05), fog_amount);");
        self.blank();

        // Post-processing from lens block
        if post_stages.is_empty() {
            self.line("// No post-processing stages");
        } else {
            self.line("// Post-processing (from lens)");
            self.line("var color_result = vec4f(color, 1.0);");
            for (i, stage) in post_stages.iter().enumerate() {
                self.emit_raymarch_post_stage(stage, i)?;
            }
            self.line("color = color_result.rgb;");
        }
        self.blank();

        self.line("return vec4f(color, 1.0);");

        self.indent -= 1;
        self.line("}");

        Ok(())
    }

    /// Emit a single post-processing stage for raymarch mode.
    /// Operates on `color_result` (vec4f), same as flat mode post-processing.
    fn emit_raymarch_post_stage(&mut self, stage: &FnCall, index: usize) -> Result<()> {
        self.line(&format!("// post {}: {}(...)", index, stage.name));

        match stage.name.as_str() {
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
}
