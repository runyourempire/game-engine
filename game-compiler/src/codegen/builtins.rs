use super::WgslGen;

impl WgslGen {
    pub(super) fn emit_builtin_functions(&mut self) {
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
}
