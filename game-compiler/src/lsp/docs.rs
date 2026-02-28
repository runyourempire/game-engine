/// Stage documentation for hover support.
///
/// Returns a Markdown-formatted documentation string for a given stage name,
/// or `None` if the name is not a known stage.
pub fn get_stage_docs(name: &str) -> Option<&'static str> {
    match name {
        // ── SDF Primitives ──────────────────────────────────────────
        "circle" => Some(
            "**circle** -- SDF circle primitive.\n\n\
             `circle(radius)`\n\n\
             Draws a circle centered at the current position.\n\n\
             - `radius` (default `0.5`): circle radius",
        ),
        "sphere" => Some(
            "**sphere** -- SDF sphere primitive.\n\n\
             `sphere(radius)`\n\n\
             Draws a sphere (projected to 2D) centered at the current position.\n\n\
             - `radius` (default `0.5`): sphere radius",
        ),
        "ring" => Some(
            "**ring** -- SDF ring (annulus) primitive.\n\n\
             `ring(radius, thickness)`\n\n\
             Draws a ring centered at the current position.\n\n\
             - `radius` (default `0.3`): ring radius\n\
             - `thickness` (default `0.04`): ring stroke width",
        ),
        "box" => Some(
            "**box** -- SDF box (rectangle) primitive.\n\n\
             `box(width, height)`\n\n\
             Draws a box centered at the current position.\n\n\
             - `width` (default `0.5`): box half-width\n\
             - `height` (default `0.5`): box half-height",
        ),
        "torus" => Some(
            "**torus** -- SDF torus primitive.\n\n\
             `torus(major_radius, minor_radius)`\n\n\
             Draws a torus (2D cross-section) centered at the current position.\n\n\
             - `major_radius` (default `0.3`): distance from center to tube center\n\
             - `minor_radius` (default `0.05`): tube radius",
        ),
        "cylinder" => Some(
            "**cylinder** -- SDF cylinder primitive.\n\n\
             `cylinder(radius, height)`\n\n\
             Draws a cylinder shape.",
        ),
        "plane" => Some(
            "**plane** -- SDF plane primitive.\n\n\
             `plane()`\n\n\
             Draws an infinite plane (useful for raymarch mode).",
        ),
        "line" => Some(
            "**line** -- SDF line segment primitive.\n\n\
             `line(x1, y1, x2, y2, thickness)`\n\n\
             Draws a line segment between two points.\n\n\
             - `x1, y1` (default `-0.5, 0.0`): start point\n\
             - `x2, y2` (default `0.5, 0.0`): end point\n\
             - `thickness` (default `0.02`): line stroke width",
        ),
        "polygon" => Some(
            "**polygon** -- SDF regular polygon primitive.\n\n\
             `polygon(sides, radius)`\n\n\
             Draws a regular polygon with N sides.\n\n\
             - `sides` (default `6`): number of sides\n\
             - `radius` (default `0.3`): circumscribed radius",
        ),
        "star" => Some(
            "**star** -- SDF star primitive.\n\n\
             `star(points, outer_radius, inner_radius)`\n\n\
             Draws a star shape.\n\n\
             - `points` (default `5`): number of star points\n\
             - `outer_radius` (default `0.4`): outer radius\n\
             - `inner_radius` (default `0.2`): inner radius",
        ),

        // ── Noise ───────────────────────────────────────────────────
        "fbm" => Some(
            "**fbm** -- Fractal Brownian Motion noise.\n\n\
             `fbm(position, octaves: N, persistence: P, lacunarity: L)`\n\n\
             Generates layered noise from multiple octaves.\n\n\
             - `position` (default `p`): 2D sample position\n\
             - `octaves` (default `6`): number of noise layers\n\
             - `persistence` (default `0.5`): amplitude decay per octave\n\
             - `lacunarity` (default `2.0`): frequency multiplier per octave",
        ),
        "simplex" => Some(
            "**simplex** -- Simplex noise.\n\n\
             `simplex(frequency)`\n\n\
             Generates 2D simplex noise.\n\n\
             - `frequency` (default `1.0`): noise frequency multiplier",
        ),
        "voronoi" => Some(
            "**voronoi** -- Voronoi cell noise.\n\n\
             `voronoi(frequency)`\n\n\
             Generates 2D Voronoi (cellular) noise.\n\n\
             - `frequency` (default `1.0`): cell frequency multiplier",
        ),
        "noise" => Some(
            "**noise** -- Generic noise function.\n\n\
             `noise(frequency)`\n\n\
             Generates basic 2D noise.",
        ),
        "curl_noise" => Some(
            "**curl_noise** -- Curl noise field.\n\n\
             `curl_noise(position, frequency: F, amplitude: A)`\n\n\
             Generates divergence-free 2D curl noise from simplex derivatives.\n\n\
             - `position` (default `p`): 2D sample position\n\
             - `frequency` (default `3.0`): noise frequency\n\
             - `amplitude` (default `0.5`): output amplitude",
        ),
        "concentric_waves" => Some(
            "**concentric_waves** -- Animated concentric waves.\n\n\
             `concentric_waves(origin, decay: D, speed: S)`\n\n\
             Generates expanding, decaying concentric ripples.\n\n\
             - `origin` (default `p`): wave center\n\
             - `decay` (default `2.0`): distance attenuation\n\
             - `speed` (default `3.0`): animation speed",
        ),

        // ── Domain Operations ───────────────────────────────────────
        "translate" => Some(
            "**translate** -- Move the coordinate origin.\n\n\
             `translate(x, y)`\n\n\
             Shifts the current position by (x, y).\n\n\
             - `x` (default `0.0`): horizontal offset\n\
             - `y` (default `0.0`): vertical offset",
        ),
        "rotate" => Some(
            "**rotate** -- Rotate coordinates.\n\n\
             `rotate(angle)`\n\n\
             Rotates the current position by the given angle in radians.\n\n\
             - `angle` (default `0.0`): rotation angle (radians)",
        ),
        "scale" => Some(
            "**scale** -- Scale coordinates.\n\n\
             `scale(factor)`\n\n\
             Scales the coordinate system. SDF distances are corrected automatically.\n\n\
             - `factor` (default `1.0`): scale multiplier",
        ),
        "repeat" => Some(
            "**repeat** -- Tile/repeat space.\n\n\
             `repeat(spacing)`\n\n\
             Tiles the coordinate space with the given cell spacing.\n\n\
             - `spacing` (default `1.0`): cell size",
        ),
        "mirror" => Some(
            "**mirror** -- Mirror coordinates.\n\n\
             `mirror(axis)`\n\n\
             Mirrors the coordinate space along the given axis.\n\n\
             - `axis` (default `\"xy\"`): `\"x\"`, `\"y\"`, or `\"xy\"`",
        ),
        "twist" => Some(
            "**twist** -- Twist coordinates.\n\n\
             `twist(amount)`\n\n\
             Applies a position-dependent rotation (twist effect).\n\n\
             - `amount` (default `1.0`): twist intensity",
        ),

        // ── SDF Modifiers ───────────────────────────────────────────
        "displace" => Some(
            "**displace** -- Displace SDF with noise.\n\n\
             `displace(strength)`\n\n\
             Adds simplex noise displacement to the current SDF.\n\n\
             - `strength` (default `0.1`): displacement amount",
        ),
        "round" => Some(
            "**round** -- Round SDF edges.\n\n\
             `round(radius)`\n\n\
             Rounds the edges of the current SDF shape.\n\n\
             - `radius` (default `0.05`): rounding radius",
        ),
        "onion" => Some(
            "**onion** -- Hollow out SDF (shell).\n\n\
             `onion(thickness)`\n\n\
             Converts a solid SDF shape into a hollow shell.\n\n\
             - `thickness` (default `0.02`): shell thickness",
        ),
        "mask_arc" => Some(
            "**mask_arc** -- Arc mask for SDF.\n\n\
             `mask_arc(angle)`\n\n\
             Masks the SDF to a wedge of the given angle.\n\n\
             - `angle` (default `6.283`): arc angle in radians (full circle = 2*pi)",
        ),
        "threshold" => Some(
            "**threshold** -- Step threshold for SDF.\n\n\
             `threshold(value)`\n\n\
             Applies a step function to the SDF at the given value.\n\n\
             - `value` (default `0.5`): threshold cutoff",
        ),

        // ── Smooth Boolean Operations ───────────────────────────────
        "smooth_union" => Some(
            "**smooth_union** -- Smooth SDF union.\n\n\
             `smooth_union(k)`\n\n\
             Smoothly blends two SDF shapes together.\n\n\
             - `k` (default `0.1`): smoothing radius",
        ),
        "smooth_subtract" => Some(
            "**smooth_subtract** -- Smooth SDF subtraction.\n\n\
             `smooth_subtract(k)`\n\n\
             Smoothly subtracts one SDF shape from another.\n\n\
             - `k` (default `0.1`): smoothing radius",
        ),
        "smooth_intersect" => Some(
            "**smooth_intersect** -- Smooth SDF intersection.\n\n\
             `smooth_intersect(k)`\n\n\
             Smoothly intersects two SDF shapes.\n\n\
             - `k` (default `0.1`): smoothing radius",
        ),

        // ── Glow ────────────────────────────────────────────────────
        "glow" => Some(
            "**glow** -- Apply glow effect to SDF.\n\n\
             `glow(intensity)`\n\n\
             Converts SDF distance to exponential glow.\n\n\
             - `intensity` (default `2.0`): glow brightness multiplier",
        ),

        // ── Color / Shading ─────────────────────────────────────────
        "shade" => Some(
            "**shade** -- Material shading.\n\n\
             `shade(albedo: COLOR, emissive: COLOR)`\n\n\
             Applies albedo and emissive color to the shape.\n\n\
             - `albedo` (default `vec3f(0.8)`): base color\n\
             - `emissive` (default `vec3f(0.0)`): emissive color",
        ),
        "emissive" => Some(
            "**emissive** -- Emissive color output.\n\n\
             `emissive()`\n\n\
             Outputs a warm emissive color (1.0, 0.8, 0.2).",
        ),
        "colormap" => Some(
            "**colormap** -- SDF-to-color mapping.\n\n\
             `colormap()`\n\n\
             Maps SDF distance to a blue-to-gold color gradient.",
        ),
        "spectrum" => Some(
            "**spectrum** -- Audio spectrum visualizer.\n\n\
             `spectrum(bass, mid, treble)`\n\n\
             Renders concentric rings that react to audio frequency bands.\n\n\
             - `bass` (default `0.0`): bass band signal\n\
             - `mid` (default `0.0`): mid band signal\n\
             - `treble` (default `0.0`): treble band signal",
        ),
        "tint" => Some(
            "**tint** -- Apply color tint.\n\n\
             `tint(color)`\n\n\
             Multiplies the current color by the given tint color.\n\n\
             - `color`: a named color (e.g., `gold`, `cyan`) or vec3f value",
        ),
        "gradient" => Some(
            "**gradient** -- Color gradient.\n\n\
             `gradient(color_a, color_b, direction)`\n\n\
             Creates a linear or radial gradient between two colors.\n\n\
             - `color_a` (default `vec3f(0.0)`): start color\n\
             - `color_b` (default `vec3f(1.0)`): end color\n\
             - `direction` (default `\"y\"`): `\"x\"`, `\"y\"`, or `\"radial\"`",
        ),
        "particles" => Some(
            "**particles** -- Particle field.\n\n\
             `particles(count: N, size: S, color: C, trail: T)`\n\n\
             Renders a field of scattered point particles.\n\n\
             - `count` (default `100`): number of particles\n\
             - `size` (default `2.0`): particle size\n\
             - `color` (default `vec3f(0.7)`): particle color\n\
             - `trail` (default `0.5`): trail persistence",
        ),

        // ── Post-Processing ─────────────────────────────────────────
        "bloom" => Some(
            "**bloom** -- Bloom post-processing.\n\n\
             `bloom(threshold, intensity)`\n\n\
             Extracts bright areas and adds glow.\n\n\
             - `threshold` (default `0.6`): brightness cutoff\n\
             - `intensity` (default `1.5`): bloom strength",
        ),
        "chromatic" => Some(
            "**chromatic** -- Chromatic aberration.\n\n\
             `chromatic(strength)`\n\n\
             Applies color fringing effect at screen edges.\n\n\
             - `strength` (default `0.5`): aberration intensity",
        ),
        "vignette" => Some(
            "**vignette** -- Vignette darkening.\n\n\
             `vignette(strength)`\n\n\
             Darkens the edges of the image.\n\n\
             - `strength` (default `0.3`): vignette intensity",
        ),
        "grain" => Some(
            "**grain** -- Film grain noise.\n\n\
             `grain(amount)`\n\n\
             Adds animated film grain noise.\n\n\
             - `amount` (default `0.02`): grain intensity",
        ),
        "fog" => Some(
            "**fog** -- Distance fog.\n\n\
             `fog(density, color)`\n\n\
             Blends toward a fog color based on distance from center.\n\n\
             - `density` (default `1.0`): fog density\n\
             - `color` (default `vec3f(0.0)`): fog color",
        ),
        "glitch" => Some(
            "**glitch** -- Glitch distortion.\n\n\
             `glitch(intensity)`\n\n\
             Applies digital glitch block artifacts.\n\n\
             - `intensity` (default `0.5`): glitch strength",
        ),
        "scanlines" => Some(
            "**scanlines** -- CRT scanlines.\n\n\
             `scanlines(count, intensity)`\n\n\
             Applies horizontal scanline effect.\n\n\
             - `count` (default `100`): number of scan lines\n\
             - `intensity` (default `0.3`): line darkness",
        ),
        "tonemap" => Some(
            "**tonemap** -- Tonemapping.\n\n\
             `tonemap(exposure)`\n\n\
             Applies Reinhard-style tonemapping.\n\n\
             - `exposure` (default `1.0`): exposure value",
        ),
        "invert" => Some(
            "**invert** -- Invert colors.\n\n\
             `invert()`\n\n\
             Inverts all color channels (1.0 - color).",
        ),
        "saturate_color" => Some(
            "**saturate_color** -- Saturation control.\n\n\
             `saturate_color(amount)`\n\n\
             Adjusts color saturation. Values > 1 increase, < 1 decrease.\n\n\
             - `amount` (default `1.5`): saturation multiplier",
        ),
        "iridescent" => Some(
            "**iridescent** -- Iridescent color shift.\n\n\
             `iridescent(strength)`\n\n\
             Applies thin-film interference color shifting.\n\n\
             - `strength` (default `0.3`): effect intensity",
        ),
        "color_grade" => Some(
            "**color_grade** -- Color grading.\n\n\
             `color_grade(contrast, brightness, gamma)`\n\n\
             Applies contrast, brightness, and gamma correction.\n\n\
             - `contrast` (default `1.0`): contrast multiplier\n\
             - `brightness` (default `0.0`): brightness offset\n\
             - `gamma` (default `1.0`): gamma correction exponent",
        ),

        _ => None,
    }
}

/// Return documentation for a built-in identifier (non-stage).
pub fn get_builtin_docs(name: &str) -> Option<&'static str> {
    match name {
        "time" => Some("**time** -- Elapsed time in seconds (wraps at 120s).\n\nType: `f32`"),
        "p" => Some("**p** -- Current 2D position in aspect-corrected coordinates.\n\nType: `vec2f`"),
        "uv" => Some("**uv** -- UV coordinates mapped to (-1, 1) range.\n\nType: `vec2f`"),
        "height" => Some("**height** -- SDF distance mapped to 0..1 range.\n\nDerived from `sdf_result`."),
        "pi" => Some("**pi** -- Mathematical constant pi (3.14159...).\n\nType: `f32`"),
        "tau" => Some("**tau** -- Mathematical constant tau (6.28318... = 2*pi).\n\nType: `f32`"),
        "e" => Some("**e** -- Euler's number (2.71828...).\n\nType: `f32`"),
        "phi" => Some("**phi** -- Golden ratio (1.61803...).\n\nType: `f32`"),
        _ => None,
    }
}

/// Return documentation for a color name.
pub fn get_color_docs(name: &str) -> Option<&'static str> {
    match name {
        "black" => Some("**black** -- Color `vec3f(0.0, 0.0, 0.0)`"),
        "white" => Some("**white** -- Color `vec3f(1.0, 1.0, 1.0)`"),
        "red" => Some("**red** -- Color `vec3f(1.0, 0.0, 0.0)`"),
        "green" => Some("**green** -- Color `vec3f(0.0, 1.0, 0.0)`"),
        "blue" => Some("**blue** -- Color `vec3f(0.0, 0.0, 1.0)`"),
        "gold" => Some("**gold** -- Color `vec3f(1.0, 0.84, 0.0)`"),
        "midnight" => Some("**midnight** -- Deep blue/black color"),
        "obsidian" => Some("**obsidian** -- Very dark grey/black color"),
        "ember" => Some("**ember** -- Warm orange-red color"),
        "cyan" => Some("**cyan** -- Color `vec3f(0.0, 1.0, 1.0)`"),
        "ivory" => Some("**ivory** -- Off-white warm color"),
        "frost" => Some("**frost** -- Cool light blue color"),
        "orange" => Some("**orange** -- Color `vec3f(1.0, 0.5, 0.0)`"),
        "deep_blue" => Some("**deep_blue** -- Rich saturated blue"),
        "ash" => Some("**ash** -- Light grey color"),
        "charcoal" => Some("**charcoal** -- Dark grey color"),
        "plasma" => Some("**plasma** -- Vibrant purple-pink color"),
        "violet" => Some("**violet** -- Purple color"),
        "magenta" => Some("**magenta** -- Color `vec3f(1.0, 0.0, 1.0)`"),
        _ => None,
    }
}

/// Return documentation for a block keyword.
pub fn get_keyword_docs(name: &str) -> Option<&'static str> {
    match name {
        "cinematic" => Some(
            "**cinematic** -- Top-level block declaring a visual composition.\n\n\
             ```\ncinematic \"Title\" {\n  layer { ... }\n}\n```",
        ),
        "layer" => Some(
            "**layer** -- Declares a visual layer with a pipeline chain.\n\n\
             ```\nlayer \"name\" {\n  fn: circle(0.3) | glow(2.0)\n}\n```",
        ),
        "lens" => Some(
            "**lens** -- Camera/rendering configuration block.\n\n\
             ```\nlens {\n  mode: raymarch\n  cam_radius: 3.0\n}\n```",
        ),
        "arc" => Some(
            "**arc** -- Timeline block for parameter animation.\n\n\
             ```\narc {\n  0s { param -> 1.0 ease expo_in }\n  2s { param -> 0.0 }\n}\n```",
        ),
        "react" => Some(
            "**react** -- User interaction event handlers.\n\n\
             ```\nreact {\n  click { brightness -> 1.0 }\n}\n```",
        ),
        "resonate" => Some(
            "**resonate** -- Cross-layer parameter modulation.\n\n\
             ```\nresonate {\n  audio.bass -> layer1.intensity\n}\n```",
        ),
        "define" => Some(
            "**define** -- Reusable macro/pattern definition.\n\n\
             ```\ndefine sparkle {\n  circle(0.1) | glow(3.0)\n}\n```",
        ),
        "import" => Some(
            "**import** -- Import definitions from another .game file.\n\n\
             ```\nimport \"stdlib/effects.game\"\n```",
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_stages_have_docs() {
        let stages = [
            "circle", "sphere", "ring", "box", "torus", "line", "polygon", "star",
            "glow", "bloom", "chromatic", "vignette", "grain",
            "translate", "rotate", "scale", "repeat", "mirror", "twist",
            "shade", "tint", "gradient", "spectrum", "particles",
            "fbm", "simplex", "voronoi", "curl_noise", "concentric_waves",
            "displace", "round", "onion", "mask_arc", "threshold",
            "smooth_union", "smooth_subtract", "smooth_intersect",
            "fog", "glitch", "scanlines", "tonemap", "invert",
            "saturate_color", "iridescent", "color_grade",
            "emissive", "colormap",
        ];
        for stage in &stages {
            assert!(
                get_stage_docs(stage).is_some(),
                "missing docs for stage: {stage}"
            );
        }
    }

    #[test]
    fn unknown_stage_returns_none() {
        assert!(get_stage_docs("nonexistent_stage").is_none());
        assert!(get_stage_docs("").is_none());
        assert!(get_stage_docs("foobar123").is_none());
    }

    #[test]
    fn builtin_docs_for_known_names() {
        assert!(get_builtin_docs("time").is_some());
        assert!(get_builtin_docs("p").is_some());
        assert!(get_builtin_docs("uv").is_some());
        assert!(get_builtin_docs("pi").is_some());
    }

    #[test]
    fn color_docs_for_known_names() {
        assert!(get_color_docs("gold").is_some());
        assert!(get_color_docs("cyan").is_some());
        assert!(get_color_docs("unknown_color").is_none());
    }
}
