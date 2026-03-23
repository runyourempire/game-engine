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

static BOX_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "width",
        default: Some(0.3),
    },
    BuiltinParam {
        name: "height",
        default: Some(0.2),
    },
];

static HEX_PARAMS: &[BuiltinParam] = &[BuiltinParam {
    name: "radius",
    default: Some(0.3),
}];

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
    name: "speed",
    default: Some(1.0),
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

// ── SDF Morph (interpolation) ──────────────────────────
// Takes two sub-expression SDFs + interpolation factor
static MORPH_PARAMS: &[BuiltinParam] = &[];

// ── SDF Boolean operations ──────────────────────────────
// These take sub-expressions as args; params are empty here
// because validation is handled specially in stages.rs.

static BOOL_OP_PARAMS: &[BuiltinParam] = &[];
static SMOOTH_BOOL_OP_PARAMS: &[BuiltinParam] = &[];

// ── Spatial operations ──────────────────────────────────

static REPEAT_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "spacing_x",
        default: Some(0.5),
    },
    BuiltinParam {
        name: "spacing_y",
        default: Some(0.5),
    },
];

static MIRROR_PARAMS: &[BuiltinParam] = &[];

static RADIAL_PARAMS: &[BuiltinParam] = &[BuiltinParam {
    name: "count",
    default: Some(6.0),
}];

// ── Shape modifiers ─────────────────────────────────────

static ROUND_PARAMS: &[BuiltinParam] = &[BuiltinParam {
    name: "radius",
    default: Some(0.02),
}];

static SHELL_PARAMS: &[BuiltinParam] = &[BuiltinParam {
    name: "width",
    default: Some(0.02),
}];

static ONION_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "count",
        default: Some(3.0),
    },
    BuiltinParam {
        name: "width",
        default: Some(0.02),
    },
];

static OUTLINE_PARAMS: &[BuiltinParam] = &[BuiltinParam {
    name: "width",
    default: Some(0.01),
}];

// ── New SDF primitives ──────────────────────────────────

static LINE_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "x1",
        default: Some(-0.2),
    },
    BuiltinParam {
        name: "y1",
        default: Some(0.0),
    },
    BuiltinParam {
        name: "x2",
        default: Some(0.2),
    },
    BuiltinParam {
        name: "y2",
        default: Some(0.0),
    },
    BuiltinParam {
        name: "width",
        default: Some(0.01),
    },
];

static CAPSULE_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "length",
        default: Some(0.3),
    },
    BuiltinParam {
        name: "radius",
        default: Some(0.05),
    },
];

static TRIANGLE_PARAMS: &[BuiltinParam] = &[BuiltinParam {
    name: "size",
    default: Some(0.3),
}];

static ARC_SDF_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "radius",
        default: Some(0.3),
    },
    BuiltinParam {
        name: "angle",
        default: Some(1.5),
    },
    BuiltinParam {
        name: "width",
        default: Some(0.02),
    },
];

static CROSS_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "size",
        default: Some(0.3),
    },
    BuiltinParam {
        name: "arm_width",
        default: Some(0.08),
    },
];

static HEART_PARAMS: &[BuiltinParam] = &[BuiltinParam {
    name: "size",
    default: Some(0.3),
}];

static EGG_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "radius",
        default: Some(0.2),
    },
    BuiltinParam {
        name: "k",
        default: Some(0.1),
    },
];

static SPIRAL_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "turns",
        default: Some(3.0),
    },
    BuiltinParam {
        name: "width",
        default: Some(0.02),
    },
];

static GRID_PARAMS: &[BuiltinParam] = &[
    BuiltinParam {
        name: "spacing",
        default: Some(0.2),
    },
    BuiltinParam {
        name: "width",
        default: Some(0.005),
    },
];

// ── Texture sampling ─────────────────────────────────────

static SAMPLE_PARAMS: &[BuiltinParam] = &[BuiltinParam {
    name: "name",
    default: None,
}];

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
        name: "box",
        params: BOX_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "hex",
        params: HEX_PARAMS,
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
    // ── SDF Morph: Position -> Sdf ────────────────────────
    BuiltinFn {
        name: "morph",
        params: MORPH_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    // ── SDF Boolean operations: Position -> Sdf ─────────
    BuiltinFn {
        name: "union",
        params: BOOL_OP_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "subtract",
        params: BOOL_OP_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "intersect",
        params: BOOL_OP_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "smooth_union",
        params: SMOOTH_BOOL_OP_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "smooth_subtract",
        params: SMOOTH_BOOL_OP_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "smooth_intersect",
        params: SMOOTH_BOOL_OP_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "xor",
        params: BOOL_OP_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    // ── Spatial operations: Position -> Position ────────
    BuiltinFn {
        name: "repeat",
        params: REPEAT_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Position,
    },
    BuiltinFn {
        name: "mirror",
        params: MIRROR_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Position,
    },
    BuiltinFn {
        name: "radial",
        params: RADIAL_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Position,
    },
    // ── Shape modifiers: Sdf -> Sdf ────────────────────
    BuiltinFn {
        name: "round",
        params: ROUND_PARAMS,
        input: ShaderState::Sdf,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "shell",
        params: SHELL_PARAMS,
        input: ShaderState::Sdf,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "onion",
        params: ONION_PARAMS,
        input: ShaderState::Sdf,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "outline",
        params: OUTLINE_PARAMS,
        input: ShaderState::Color,
        output: ShaderState::Color,
    },
    // ── New SDF primitives: Position -> Sdf ────────────
    BuiltinFn {
        name: "line",
        params: LINE_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "capsule",
        params: CAPSULE_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "triangle",
        params: TRIANGLE_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "arc_sdf",
        params: ARC_SDF_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "cross",
        params: CROSS_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "heart",
        params: HEART_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "egg",
        params: EGG_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "spiral",
        params: SPIRAL_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    BuiltinFn {
        name: "grid",
        params: GRID_PARAMS,
        input: ShaderState::Position,
        output: ShaderState::Sdf,
    },
    // ── Texture sampling: Position -> Color ────────────────
    BuiltinFn {
        name: "sample",
        params: SAMPLE_PARAMS,
        input: ShaderState::Position,
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

/// LSP completion item for a builtin.
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub name: String,
    pub signature: String,
    pub input: String,
    pub output: String,
}

/// Get all builtins as LSP completion items.
pub fn completions() -> Vec<CompletionItem> {
    BUILTINS
        .iter()
        .map(|b| {
            let params: Vec<String> = b
                .params
                .iter()
                .map(|p| {
                    if let Some(d) = p.default {
                        format!("{}={}", p.name, d)
                    } else {
                        p.name.to_string()
                    }
                })
                .collect();
            CompletionItem {
                name: b.name.to_string(),
                signature: format!("{}({})", b.name, params.join(", ")),
                input: format!("{:?}", b.input),
                output: format!("{:?}", b.output),
            }
        })
        .collect()
}

/// Get valid next stages for LSP completion based on current pipeline state.
pub fn valid_next_stages(state: ShaderState) -> Vec<&'static str> {
    BUILTINS
        .iter()
        .filter(|b| b.input == state)
        .map(|b| b.name)
        .collect()
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
            "box",
            "hex",
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
            // SDF morph
            "morph",
            // SDF boolean operations
            "union",
            "subtract",
            "intersect",
            "smooth_union",
            "smooth_subtract",
            "smooth_intersect",
            "xor",
            // Spatial operations
            "repeat",
            "mirror",
            "radial",
            // Shape modifiers
            "round",
            "shell",
            "onion",
            "outline",
            // New SDF primitives
            "line",
            "capsule",
            "triangle",
            "arc_sdf",
            "cross",
            "heart",
            "egg",
            "spiral",
            "grid",
            // Texture sampling
            "sample",
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
