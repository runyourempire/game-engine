use super::WgslGen;

impl WgslGen {
    pub(super) fn emit_builtin_functions(&mut self) {
        let mut emitted_any = false;

        // ── SDF primitives ────────────────────────────────────────────

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

        if self.used_builtins.contains("sdf_box2") {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                emitted_any = true;
            }
            self.line("fn sdf_box2(p: vec2f, b: vec2f) -> f32 {");
            self.indent += 1;
            self.line("let d = abs(p) - b;");
            self.line("return length(max(d, vec2f(0.0))) + min(max(d.x, d.y), 0.0);");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }

        if self.used_builtins.contains("sdf_line") {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                emitted_any = true;
            }
            self.line("fn sdf_line(p: vec2f, a: vec2f, b: vec2f) -> f32 {");
            self.indent += 1;
            self.line("let pa = p - a;");
            self.line("let ba = b - a;");
            self.line("let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);");
            self.line("return length(pa - ba * h);");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }

        if self.used_builtins.contains("sdf_polygon") {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                emitted_any = true;
            }
            self.line("fn sdf_polygon(p: vec2f, n: f32, r: f32) -> f32 {");
            self.indent += 1;
            self.line("let an = 3.14159265359 / n;");
            self.line("let he = r * cos(an);");
            self.line("var q = vec2f(length(p), atan2(p.y, p.x));");
            self.line("let bn = an * (2.0 * floor(q.y / (2.0 * an) + 0.5));");
            self.line("q = vec2f(q.x, q.y - bn);");
            self.line("let cs = vec2f(cos(q.y), sin(q.y));");
            self.line("let k = vec2f(q.x * cs.x - 0.0 * cs.y, q.x * cs.y + 0.0 * cs.x);");
            self.line("return max(k.x - he, abs(k.y) - r * sin(an));");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }

        if self.used_builtins.contains("sdf_star") {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                emitted_any = true;
            }
            self.line("fn sdf_star(p: vec2f, n: f32, outer_r: f32, inner_r: f32) -> f32 {");
            self.indent += 1;
            self.line("let an = 3.14159265359 / n;");
            self.line("let en = 3.14159265359 / (n * 2.0);");
            self.line("let acs = vec2f(cos(an), sin(an));");
            self.line("let ecs = vec2f(cos(en), sin(en));");
            self.line("var q = abs(p);");
            self.line("q = vec2f(q.x * acs.x + q.y * acs.y, q.y * acs.x - q.x * acs.y);");
            self.line("q = vec2f(q.x - outer_r, q.y);");
            self.line("let w = vec2f(inner_r * ecs.x - outer_r, inner_r * ecs.y);");
            self.line("let h = clamp(dot(q, w) / dot(w, w), 0.0, 1.0);");
            self.line("return length(q - w * h) * sign(q.y * w.x - q.x * w.y);");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }

        // ── Smooth SDF boolean operations ─────────────────────────────

        if self.used_builtins.contains("sdf_smooth_union") {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                emitted_any = true;
            }
            self.line("fn sdf_smooth_union(d1: f32, d2: f32, k: f32) -> f32 {");
            self.indent += 1;
            self.line("let h = clamp(0.5 + 0.5 * (d2 - d1) / k, 0.0, 1.0);");
            self.line("return mix(d2, d1, h) - k * h * (1.0 - h);");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }

        if self.used_builtins.contains("sdf_smooth_subtract") {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                emitted_any = true;
            }
            self.line("fn sdf_smooth_subtract(d1: f32, d2: f32, k: f32) -> f32 {");
            self.indent += 1;
            self.line("let h = clamp(0.5 - 0.5 * (d2 + d1) / k, 0.0, 1.0);");
            self.line("return mix(d1, -d2, h) + k * h * (1.0 - h);");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }

        if self.used_builtins.contains("sdf_smooth_intersect") {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                emitted_any = true;
            }
            self.line("fn sdf_smooth_intersect(d1: f32, d2: f32, k: f32) -> f32 {");
            self.indent += 1;
            self.line("let h = clamp(0.5 - 0.5 * (d2 - d1) / k, 0.0, 1.0);");
            self.line("return mix(d2, d1, h) + k * h * (1.0 - h);");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }

        // ── Glow / effects ───────────────────────────────────────────

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

        // ── Noise functions ──────────────────────────────────────────

        // hash2 is a dependency shared by fbm2, simplex2, voronoi2, curl2, particle_field
        let needs_hash2 = self.used_builtins.contains("fbm2")
            || self.used_builtins.contains("simplex2")
            || self.used_builtins.contains("voronoi2")
            || self.used_builtins.contains("curl2")
            || self.used_builtins.contains("particle_field");

        if needs_hash2 {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                emitted_any = true;
            }
            self.line("fn hash2(p: vec2f) -> f32 {");
            self.indent += 1;
            self.line("var p3 = fract(vec3f(p.x, p.y, p.x) * 0.1031);");
            self.line("p3 += dot(p3, p3.yzx + 33.33);");
            self.line("return fract((p3.x + p3.y) * p3.z);");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }


        // hash2v is needed by voronoi2, simplex2, and curl2 (returns vec2f)
        let needs_hash2v = self.used_builtins.contains("simplex2")
            || self.used_builtins.contains("voronoi2")
            || self.used_builtins.contains("curl2");

        if needs_hash2v {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                emitted_any = true;
            }
            self.line("fn hash2v(p: vec2f) -> vec2f {");
            self.indent += 1;
            self.line("var p3 = fract(vec3f(p.x, p.y, p.x) * vec3f(0.1031, 0.1030, 0.0973));");
            self.line("p3 += dot(p3, p3.yzx + 33.33);");
            self.line("return fract((p3.xx + p3.yz) * p3.zy);");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }


        // noise2 is a dependency for fbm2, simplex2, and curl2
        let needs_noise2 = self.used_builtins.contains("fbm2")
            || self.used_builtins.contains("simplex2")
            || self.used_builtins.contains("curl2");

        if needs_noise2 {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                emitted_any = true;
            }
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
        }

        if self.used_builtins.contains("fbm2") {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                emitted_any = true;
            }
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

        if self.used_builtins.contains("simplex2") || self.used_builtins.contains("curl2") {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                emitted_any = true;
            }
            // Simplex noise via gradient lattice (classic 2D simplex)
            self.line("fn simplex2(p: vec2f) -> f32 {");
            self.indent += 1;
            self.line("let K1: f32 = 0.366025404;");  // (sqrt(3)-1)/2
            self.line("let K2: f32 = 0.211324865;");  // (3-sqrt(3))/6
            self.line("let s = (p.x + p.y) * K1;");
            self.line("let i = floor(p + s);");
            self.line("let a = p - i + (i.x + i.y) * K2;");
            self.line("let o = select(vec2f(0.0, 1.0), vec2f(1.0, 0.0), a.x > a.y);");
            self.line("let b = a - o + K2;");
            self.line("let c = a - 1.0 + 2.0 * K2;");
            self.line("var h = max(vec3f(0.5) - vec3f(dot(a, a), dot(b, b), dot(c, c)), vec3f(0.0));");
            self.line("h = h * h * h * h;");
            self.line("let ga = hash2v(i) * 2.0 - 1.0;");
            self.line("let gb = hash2v(i + o) * 2.0 - 1.0;");
            self.line("let gc = hash2v(i + 1.0) * 2.0 - 1.0;");
            self.line("let n = h * vec3f(dot(ga, a), dot(gb, b), dot(gc, c));");
            self.line("return dot(n, vec3f(70.0));");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }

        if self.used_builtins.contains("voronoi2") {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                emitted_any = true;
            }
            self.line("fn voronoi2(p: vec2f) -> f32 {");
            self.indent += 1;
            self.line("let n = floor(p);");
            self.line("let f = fract(p);");
            self.line("var md: f32 = 8.0;");
            self.line("for (var j: i32 = -1; j <= 1; j++) {");
            self.indent += 1;
            self.line("for (var i: i32 = -1; i <= 1; i++) {");
            self.indent += 1;
            self.line("let g = vec2f(f32(i), f32(j));");
            self.line("let o = hash2v(n + g);");
            self.line("let r = g + o - f;");
            self.line("let d = dot(r, r);");
            self.line("md = min(md, d);");
            self.indent -= 1;
            self.line("}");
            self.indent -= 1;
            self.line("}");
            self.line("return sqrt(md);");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }

        // ── Curl & particle functions ────────────────────────────

        if self.used_builtins.contains("curl2") {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                emitted_any = true;
            }
            self.line("fn curl2(p: vec2f, freq: f32, amp: f32) -> vec2f {");
            self.indent += 1;
            self.line("let eps: f32 = 0.001;");
            self.line("let n0 = simplex2(p * freq);");
            self.line("let nx = simplex2((p + vec2f(eps, 0.0)) * freq);");
            self.line("let ny = simplex2((p + vec2f(0.0, eps)) * freq);");
            self.line("let dndx = (nx - n0) / eps;");
            self.line("let dndy = (ny - n0) / eps;");
            self.line("return vec2f(dndy, -dndx) * amp;");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }

        if self.used_builtins.contains("particle_field") {
            if !emitted_any {
                self.line("// ── Built-in functions ──────────────────────────────────");
                self.blank();
                #[allow(unused_assignments)]
                { emitted_any = true; }
            }
            self.line("fn particle_field(p: vec2f, count: f32, size: f32) -> f32 {");
            self.indent += 1;
            self.line("var brightness: f32 = 0.0;");
            self.line("for (var i: f32 = 0.0; i < count; i += 1.0) {");
            self.indent += 1;
            self.line("let h = hash2(vec2f(i * 127.1, i * 311.7));");
            self.line("let h2 = hash2(vec2f(i * 269.5, i * 183.3));");
            self.line("let pp = vec2f(h * 2.0 - 1.0, h2 * 2.0 - 1.0);");
            self.line("let d = length(p - pp);");
            self.line("brightness += exp(-d * d / (size * size * 0.01)) * 0.5;");
            self.indent -= 1;
            self.line("}");
            self.line("return brightness;");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }
    }
}
