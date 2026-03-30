//! Built-in function definitions for the GAME language.
//!
//! Each builtin describes its name, parameters, and what shader state
//! it consumes/produces in the pipeline.

/// Shader pipeline state — tracks what kind of data is flowing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShaderState {
    /// 2D position (`vec2 p`) — before any SDF evaluation.
    Position,
    /// Signed distance field result (`float sdf_result`).
    Sdf,
    /// RGBA color result (`vec4 color_result`).
    Color,
}

impl std::fmt::Display for ShaderState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Position => write!(f, "Position"),
            Self::Sdf => write!(f, "Sdf"),
            Self::Color => write!(f, "Color"),
        }
    }
}

/// A built-in function's parameter signature.
pub struct BuiltinParam {
    pub name: &'static str,
    pub default: Option<f64>,
}

/// A built-in function definition.
pub struct BuiltinFn {
    pub name: &'static str,
    pub params: &'static [BuiltinParam],
    pub input: ShaderState,
    pub output: ShaderState,
}

// ── Param lists ──────────────────────────────────────────

static CIRCLE_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "radius", default: Some(0.2) },
];

static RING_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "radius", default: Some(0.3) },
    BuiltinParam { name: "width", default: Some(0.02) },
];

static STAR_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "points", default: Some(5.0) },
    BuiltinParam { name: "radius", default: Some(0.3) },
    BuiltinParam { name: "inner", default: Some(0.15) },
];

static GLOW_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "intensity", default: Some(1.5) },
];

static TINT_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "r", default: Some(1.0) },
    BuiltinParam { name: "g", default: Some(1.0) },
    BuiltinParam { name: "b", default: Some(1.0) },
];

static BLOOM_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "threshold", default: Some(0.3) },
    BuiltinParam { name: "strength", default: Some(2.0) },
];

static GRAIN_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "amount", default: Some(0.1) },
];

static TRANSLATE_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "x", default: Some(0.0) },
    BuiltinParam { name: "y", default: Some(0.0) },
];

static ROTATE_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "angle", default: Some(0.0) },
];

static SCALE_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "s", default: Some(1.0) },
];

static SHADE_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "r", default: Some(1.0) },
    BuiltinParam { name: "g", default: Some(1.0) },
    BuiltinParam { name: "b", default: Some(1.0) },
];

static EMISSIVE_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "intensity", default: Some(1.0) },
];

static FBM_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "scale", default: Some(1.0) },
    BuiltinParam { name: "octaves", default: Some(4.0) },
    BuiltinParam { name: "persistence", default: Some(0.5) },
    BuiltinParam { name: "lacunarity", default: Some(2.0) },
];

static SIMPLEX_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "scale", default: Some(1.0) },
];

static MASK_ARC_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "angle", default: None },
];

static GRADIENT_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "color_a", default: None },
    BuiltinParam { name: "color_b", default: None },
    BuiltinParam { name: "mode", default: None },
];

static THRESHOLD_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "cutoff", default: Some(0.5) },
];

static TWIST_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "amount", default: Some(0.0) },
];

static MIRROR_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "axis", default: Some(0.0) },
];

static REPEAT_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "count", default: Some(4.0) },
];

static BLEND_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "factor", default: Some(0.5) },
];

static VIGNETTE_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "strength", default: Some(0.5) },
    BuiltinParam { name: "radius", default: Some(0.8) },
];

static VORONOI_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "scale", default: Some(5.0) },
];

static ONION_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "thickness", default: Some(0.02) },
];

static DOMAIN_WARP_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "amount", default: Some(0.1) },
    BuiltinParam { name: "freq", default: Some(3.0) },
];

static BOX_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "w", default: Some(0.2) },
    BuiltinParam { name: "h", default: Some(0.2) },
];

static ROUND_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "radius", default: Some(0.02) },
];

static POLYGON_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "sides", default: Some(6.0) },
    BuiltinParam { name: "radius", default: Some(0.3) },
];

static CURL_NOISE_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "frequency", default: Some(1.0) },
    BuiltinParam { name: "amplitude", default: Some(0.1) },
];

static TONEMAP_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "exposure", default: Some(1.0) },
];

static SCANLINES_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "frequency", default: Some(200.0) },
    BuiltinParam { name: "intensity", default: Some(0.3) },
];

static CHROMATIC_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "offset", default: Some(0.005) },
];

static SATURATE_COLOR_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "amount", default: Some(1.0) },
];

static GLITCH_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "intensity", default: Some(0.5) },
];

static CONCENTRIC_WAVES_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "amplitude", default: Some(1.0) },
    BuiltinParam { name: "width", default: Some(0.5) },
    BuiltinParam { name: "frequency", default: Some(3.0) },
];

static DISPLACE_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "strength", default: Some(0.1) },
];

static SPECTRUM_PARAMS: &[BuiltinParam] = &[
    BuiltinParam { name: "bass", default: Some(0.0) },
    BuiltinParam { name: "mid", default: Some(0.0) },
    BuiltinParam { name: "treble", default: Some(0.0) },
];

// ── Registry ─────────────────────────────────────────────

static BUILTINS: &[BuiltinFn] = &[
    // SDF generators: Position -> Sdf
    BuiltinFn { name: "circle",  params: CIRCLE_PARAMS,  input: ShaderState::Position, output: ShaderState::Sdf },
    BuiltinFn { name: "ring",    params: RING_PARAMS,    input: ShaderState::Position, output: ShaderState::Sdf },
    BuiltinFn { name: "star",    params: STAR_PARAMS,    input: ShaderState::Position, output: ShaderState::Sdf },
    BuiltinFn { name: "fbm",     params: FBM_PARAMS,     input: ShaderState::Position, output: ShaderState::Sdf },
    BuiltinFn { name: "simplex", params: SIMPLEX_PARAMS,  input: ShaderState::Position, output: ShaderState::Sdf },

    // Bridges: Sdf -> Color
    BuiltinFn { name: "glow",     params: GLOW_PARAMS,     input: ShaderState::Sdf, output: ShaderState::Color },
    BuiltinFn { name: "shade",    params: SHADE_PARAMS,    input: ShaderState::Sdf, output: ShaderState::Color },
    BuiltinFn { name: "emissive", params: EMISSIVE_PARAMS, input: ShaderState::Sdf, output: ShaderState::Color },

    // Color processors: Color -> Color
    BuiltinFn { name: "tint",  params: TINT_PARAMS,  input: ShaderState::Color, output: ShaderState::Color },
    BuiltinFn { name: "bloom", params: BLOOM_PARAMS, input: ShaderState::Color, output: ShaderState::Color },
    BuiltinFn { name: "grain", params: GRAIN_PARAMS, input: ShaderState::Color, output: ShaderState::Color },

    // Transforms: Position -> Position
    BuiltinFn { name: "translate", params: TRANSLATE_PARAMS, input: ShaderState::Position, output: ShaderState::Position },
    BuiltinFn { name: "rotate",    params: ROTATE_PARAMS,    input: ShaderState::Position, output: ShaderState::Position },
    BuiltinFn { name: "scale",     params: SCALE_PARAMS,     input: ShaderState::Position, output: ShaderState::Position },

    // Sdf modifiers: Sdf -> Sdf
    BuiltinFn { name: "mask_arc", params: MASK_ARC_PARAMS, input: ShaderState::Sdf, output: ShaderState::Sdf },
    BuiltinFn { name: "threshold", params: THRESHOLD_PARAMS, input: ShaderState::Sdf, output: ShaderState::Sdf },

    // Position modifiers: Position -> Position
    BuiltinFn { name: "twist",  params: TWIST_PARAMS,  input: ShaderState::Position, output: ShaderState::Position },
    BuiltinFn { name: "mirror", params: MIRROR_PARAMS, input: ShaderState::Position, output: ShaderState::Position },
    BuiltinFn { name: "repeat", params: REPEAT_PARAMS, input: ShaderState::Position, output: ShaderState::Position },

    // Full-screen generators: Position -> Color
    BuiltinFn { name: "gradient", params: GRADIENT_PARAMS, input: ShaderState::Position, output: ShaderState::Color },

    // Color mixers: Color -> Color
    BuiltinFn { name: "blend", params: BLEND_PARAMS, input: ShaderState::Color, output: ShaderState::Color },
    BuiltinFn { name: "vignette", params: VIGNETTE_PARAMS, input: ShaderState::Color, output: ShaderState::Color },

    // Additional SDF generators: Position -> Sdf
    BuiltinFn { name: "voronoi", params: VORONOI_PARAMS, input: ShaderState::Position, output: ShaderState::Sdf },

    // SDF modifiers: Sdf -> Sdf
    BuiltinFn { name: "onion", params: ONION_PARAMS, input: ShaderState::Sdf, output: ShaderState::Sdf },

    // Position modifiers: Position -> Position
    BuiltinFn { name: "domain_warp", params: DOMAIN_WARP_PARAMS, input: ShaderState::Position, output: ShaderState::Position },
    BuiltinFn { name: "curl_noise", params: CURL_NOISE_PARAMS, input: ShaderState::Position, output: ShaderState::Position },

    // Additional SDF generators: Position -> Sdf
    BuiltinFn { name: "box",     params: BOX_PARAMS,     input: ShaderState::Position, output: ShaderState::Sdf },
    BuiltinFn { name: "polygon", params: POLYGON_PARAMS, input: ShaderState::Position, output: ShaderState::Sdf },
    BuiltinFn { name: "concentric_waves", params: CONCENTRIC_WAVES_PARAMS, input: ShaderState::Position, output: ShaderState::Sdf },

    // SDF modifiers: Sdf -> Sdf
    BuiltinFn { name: "round", params: ROUND_PARAMS, input: ShaderState::Sdf, output: ShaderState::Sdf },

    // Post-processing: Color -> Color
    BuiltinFn { name: "tonemap",        params: TONEMAP_PARAMS,        input: ShaderState::Color, output: ShaderState::Color },
    BuiltinFn { name: "scanlines",      params: SCANLINES_PARAMS,      input: ShaderState::Color, output: ShaderState::Color },
    BuiltinFn { name: "chromatic",      params: CHROMATIC_PARAMS,      input: ShaderState::Color, output: ShaderState::Color },
    BuiltinFn { name: "saturate_color", params: SATURATE_COLOR_PARAMS, input: ShaderState::Color, output: ShaderState::Color },
    BuiltinFn { name: "glitch",         params: GLITCH_PARAMS,         input: ShaderState::Color, output: ShaderState::Color },

    // Domain warp: Position -> Position (noise displacement before SDF evaluation)
    BuiltinFn { name: "displace", params: DISPLACE_PARAMS, input: ShaderState::Position, output: ShaderState::Position },

    // Spectrum generator: Position -> Color
    BuiltinFn { name: "spectrum", params: SPECTRUM_PARAMS, input: ShaderState::Position, output: ShaderState::Color },
];

/// Look up a built-in function by name.
pub fn lookup(name: &str) -> Option<&'static BuiltinFn> {
    BUILTINS.iter().find(|b| b.name == name)
}

/// Get all builtin names.
pub fn all_names() -> impl Iterator<Item = &'static str> {
    BUILTINS.iter().map(|b| b.name)
}

/// Suggest the closest builtin name to a misspelled input.
pub fn suggest(name: &str) -> Option<&'static str> {
    let names: Vec<&str> = BUILTINS.iter().map(|b| b.name).collect();
    crate::error::suggest_similar(name, &names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtins_reachable() {
        for name in ["circle", "ring", "star", "glow", "tint", "bloom",
                      "grain", "translate", "rotate", "scale", "shade",
                      "emissive", "fbm", "simplex", "mask_arc", "gradient",
                      "threshold", "twist", "mirror", "repeat", "blend",
                      "vignette", "voronoi", "onion", "domain_warp",
                      "box", "round", "polygon", "curl_noise", "tonemap",
                      "scanlines", "chromatic", "saturate_color", "glitch",
                      "concentric_waves", "displace", "spectrum"] {
            assert!(lookup(name).is_some(), "missing builtin: {name}");
        }
    }

    #[test]
    fn state_transitions_valid() {
        // SDF generators start from Position
        assert_eq!(lookup("circle").unwrap().input, ShaderState::Position);
        assert_eq!(lookup("circle").unwrap().output, ShaderState::Sdf);
        // Bridges go Sdf -> Color
        assert_eq!(lookup("glow").unwrap().input, ShaderState::Sdf);
        assert_eq!(lookup("glow").unwrap().output, ShaderState::Color);
        // Color processors stay in Color
        assert_eq!(lookup("tint").unwrap().input, ShaderState::Color);
        assert_eq!(lookup("tint").unwrap().output, ShaderState::Color);
    }

    #[test]
    fn unknown_builtin_returns_none() {
        assert!(lookup("nonexistent").is_none());
    }

    #[test]
    fn suggest_finds_close_builtin() {
        assert_eq!(suggest("cicle"), Some("circle"));
        assert_eq!(suggest("circl"), Some("circle"));
        assert_eq!(suggest("tnt"), Some("tint"));
        assert_eq!(suggest("glo"), Some("glow"));
        assert_eq!(suggest("blom"), Some("bloom"));
    }

    #[test]
    fn suggest_returns_none_for_gibberish() {
        assert_eq!(suggest("xyzxyzxyz"), None);
        assert_eq!(suggest("aaaaaaa"), None);
    }
}
