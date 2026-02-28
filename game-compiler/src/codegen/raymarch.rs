use super::WgslGen;
use crate::ast::*;
use crate::error::{ErrorKind, GameError, Result};

impl WgslGen {
    // ── Raymarch mode ──────────────────────────────────────────────────

    pub(super) fn emit_raymarch_helpers(&mut self, layer: &Layer) -> Result<()> {
        let chain = layer.fn_chain.as_ref().ok_or_else(|| GameError {
            kind: ErrorKind::Message("layer has no fn: chain".into()),
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
        self.line("let height = clamp(field_at(hit_pos.xz) * 0.5 + 0.5, 0.0, 1.0);");
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
}
