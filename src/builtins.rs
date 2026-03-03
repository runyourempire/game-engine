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

static CIRCLE_PARAMS: &[BuiltinParam] = &[BuiltinParam {
    name: "radius",
    default: Some(0.2),
}];

static RING_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "radius",
        default: Some(0.3),
    },
    BuiltinParam {
        name: "width",
        default: Some(0.02),
    },
];

static STAR_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "points",
        default: Some(5.0),
    },
    BuiltinParam {
        name: "radius",
        default: Some(0.3),
    },
    BuiltinParam {
        name: "inner",
        default: Some(0.15),
    },
];

static GLOW_PARAMS: &[BuiltinParam] = &[BuiltinParam {
    name: "intensity",
    default: Some(1.5),
}];

static TINT_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "r",
        default: Some(1.0),
    },
    BuiltinParam {
        name: "g",
        default: Some(1.0),
    },
    BuiltinParam {
        name: "b",
        default: Some(1.0),
    },
];

static BLOOM_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "threshold",
        default: Some(0.3),
    },
    BuiltinParam {
        name: "strength",
        default: Some(2.0),
    },
];

static GRAIN_PARAMS: &[BuiltinParam] = &[BuiltinParam {
    name: "amount",
    default: Some(0.1),
}];

static TRANSLATE_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "x",
        default: Some(0.0),
    },
    BuiltinParam {
        name: "y",
        default: Some(0.0),
    },
];

static ROTATE_PARAMS: &[BuiltinParam] = &[BuiltinParam {
    name: "angle",
    default: Some(0.0),
}];

static SCALE_PARAMS: &[BuiltinParam] = &[BuiltinParam {
    name: "s",
    default: Some(1.0),
}];

static SHADE_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "r",
        default: Some(1.0),
    },
    BuiltinParam {
        name: "g",
        default: Some(1.0),
    },
    BuiltinParam {
        name: "b",
        default: Some(1.0),
    },
];

static EMISSIVE_PARAMS: &[BuiltinParam] = &[BuiltinParam {
    name: "intensity",
    default: Some(1.0),
}];

static FBM_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "scale",
        default: Some(1.0),
    },
    BuiltinParam {
        name: "octaves",
        default: Some(4.0),
    },
    BuiltinParam {
        name: "persistence",
        default: Some(0.5),
    },
    BuiltinParam {
        name: "lacunarity",
        default: Some(2.0),
    },
];

static SIMPLEX_PARAMS: &[BuiltinParam] = &[BuiltinParam {
    name: "scale",
    default: Some(1.0),
}];

static MASK_ARC_PARAMS: &[BuiltinParam] = &[BuiltinParam {
    name: "angle",
    default: None,
}];

// ── Phase 7: Visual quality stages ──────────────────────

static WARP_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "scale",
        default: Some(3.0),
    },
    BuiltinParam {
        name: "octaves",
        default: Some(4.0),
    },
    BuiltinParam {
        name: "persistence",
        default: Some(0.5),
    },
    BuiltinParam {
        name: "lacunarity",
        default: Some(2.0),
    },
    BuiltinParam {
        name: "strength",
        default: Some(0.3),
    },
];

static DISTORT_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "scale",
        default: Some(3.0),
    },
    BuiltinParam {
        name: "speed",
        default: Some(1.0),
    },
    BuiltinParam {
        name: "strength",
        default: Some(0.2),
    },
];

static VORONOI_PARAMS: &[BuiltinParam] = &[BuiltinParam {
    name: "scale",
    default: Some(5.0),
}];

static PALETTE_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "a_r",
        default: Some(0.5),
    },
    BuiltinParam {
        name: "a_g",
        default: Some(0.5),
    },
    BuiltinParam {
        name: "a_b",
        default: Some(0.5),
    },
    BuiltinParam {
        name: "b_r",
        default: Some(0.5),
    },
    BuiltinParam {
        name: "b_g",
        default: Some(0.5),
    },
    BuiltinParam {
        name: "b_b",
        default: Some(0.5),
    },
    BuiltinParam {
        name: "c_r",
        default: Some(1.0),
    },
    BuiltinParam {
        name: "c_g",
        default: Some(1.0),
    },
    BuiltinParam {
        name: "c_b",
        default: Some(1.0),
    },
    BuiltinParam {
        name: "d_r",
        default: Some(0.0),
    },
    BuiltinParam {
        name: "d_g",
        default: Some(0.33),
    },
    BuiltinParam {
        name: "d_b",
        default: Some(0.67),
    },
];

static RADIAL_FADE_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "inner",
        default: Some(0.0),
    },
    BuiltinParam {
        name: "outer",
        default: Some(1.0),
    },
];

static POLAR_PARAMS: &[BuiltinParam] = &[];

// ── Registry ─────────────────────────────────────────────

static BUILTINS: &[BuiltinFn] = &[
    // SDF generators: Position -> Sdf
    BuiltinFn {
        name: "circle",
        params: CIRCLE_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "ring",
        params: RING_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "star",
        params: STAR_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "fbm",
        params: FBM_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "simplex",
        params: SIMPLEX_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    // Bridges: Sdf -> Color
    BuiltinFn {
        name: "glow",
        params: GLOW_PARAMS,
        input: ShaderState::Sdf,
        output: ShaderState::Color,
    },
    BuiltinFn {
        name: "shade",
        params: SHADE_PARAMS,
        input: ShaderState::Sdf,
        output: ShaderState::Color,
    },
    BuiltinFn {
        name: "emissive",
        params: EMISSIVE_PARAMS,
        input: ShaderState::Sdf,
        output: ShaderState::Color,
    },
    // Color processors: Color -> Color
    BuiltinFn {
        name: "tint",
        params: TINT_PARAMS,
        input: ShaderState::Color,
        output: ShaderState::Color,
    },
    BuiltinFn {
        name: "bloom",
        params: BLOOM_PARAMS,
        input: ShaderState::Color,
        output: ShaderState::Color,
    },
    BuiltinFn {
        name: "grain",
        params: GRAIN_PARAMS,
        input: ShaderState::Color,
        output: ShaderState::Color,
    },
    // Transforms: Position -> Position
    BuiltinFn {
        name: "translate",
        params: TRANSLATE_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Position,
    },
    BuiltinFn {
        name: "rotate",
        params: ROTATE_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Position,
    },
    BuiltinFn {
        name: "scale",
        params: SCALE_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Position,
    },
    // Sdf modifiers: Sdf -> Sdf
    BuiltinFn {
        name: "mask_arc",
        params: MASK_ARC_PARAMS,
        input: ShaderState::Sdf,
        output: ShaderState::Sdf,
    },
    // Phase 7: Visual quality stages
    // Domain warping: Position -> Position
    BuiltinFn {
        name: "warp",
        params: WARP_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Position,
    },
    BuiltinFn {
        name: "distort",
        params: DISTORT_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Position,
    },
    BuiltinFn {
        name: "polar",
        params: POLAR_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Position,
    },
    // Cellular noise: Position -> Sdf
    BuiltinFn {
        name: "voronoi",
        params: VORONOI_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    // Radial falloff: Position -> Sdf
    BuiltinFn {
        name: "radial_fade",
        params: RADIAL_FADE_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    // Cosine palette: Sdf -> Color
    BuiltinFn {
        name: "palette",
        params: PALETTE_PARAMS,
        input: ShaderState::Sdf,
        output: ShaderState::Color,
    },
];

/// Look up a built-in function by name.
pub fn lookup(name: &str) -> Option<&'static BuiltinFn> {
    BUILTINS.iter().find(|b| b.name == name)
}

/// Get all builtin names.
pub fn all_names() -> impl Iterator<Item = &'static str> {
    BUILTINS.iter().map(|b| b.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtins_reachable() {
        for name in [
            "circle",
            "ring",
            "star",
            "glow",
            "tint",
            "bloom",
            "grain",
            "translate",
            "rotate",
            "scale",
            "shade",
            "emissive",
            "fbm",
            "simplex",
            "mask_arc",
            // Phase 7: Visual quality stages
            "warp",
            "distort",
            "polar",
            "voronoi",
            "radial_fade",
            "palette",
        ] {
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
}
