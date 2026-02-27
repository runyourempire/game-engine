//! Intermediate Representation (IR) for the GAME compiler.
//!
//! The IR sits between the AST and codegen, enabling optimization passes
//! and future backend targets. It mirrors AST types closely for easy
//! roundtrip conversion, but adds semantic metadata (stage classification,
//! builtin dependencies) that the optimizer and emitter need.
//!
//! Pipeline: AST → lower → IR → optimize → reconstruct → WgslGen → WGSL

use crate::ast::{BinOp, BlendMode};

// ── Top-level IR ──────────────────────────────────────────────────────

/// Complete IR representation of a compiled GAME shader.
#[derive(Debug, Clone)]
pub struct ShaderIR {
    pub title: String,
    pub layers: Vec<IrLayer>,
    pub uniforms: Vec<IrUniform>,
    pub render_mode: IrRenderMode,
    pub uses_audio: bool,
    pub uses_mouse: bool,
    pub uses_data: bool,
    pub data_fields: Vec<String>,
    pub warnings: Vec<String>,
}

/// Rendering mode determined from lens block.
#[derive(Debug, Clone, PartialEq)]
pub enum IrRenderMode {
    Flat,
    Raymarch {
        cam_radius: f64,
        cam_height: f64,
        cam_speed: f64,
    },
}

// ── Layers ────────────────────────────────────────────────────────────

/// A visual layer with classified stages.
#[derive(Debug, Clone)]
pub struct IrLayer {
    pub name: Option<String>,
    pub stages: Vec<IrStage>,
    pub blend_mode: BlendMode,
    pub blend_opacity: f64,
    pub params: Vec<IrParam>,
    pub properties: Vec<IrProperty>,
}

/// A parameter with optional modulation.
#[derive(Debug, Clone)]
pub struct IrParam {
    pub name: String,
    pub base_value: IrExpr,
    pub modulation: Option<IrExpr>,
}

/// A named layer property.
#[derive(Debug, Clone)]
pub struct IrProperty {
    pub name: String,
    pub value: IrExpr,
}

// ── Stages ────────────────────────────────────────────────────────────

/// A single pipe chain stage with semantic classification.
#[derive(Debug, Clone)]
pub struct IrStage {
    pub kind: StageKind,
    pub name: String,
    pub args: Vec<IrArg>,
    pub span: Option<std::ops::Range<usize>>,
}

/// Stage classification for validation and optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageKind {
    /// Domain transforms: translate, rotate, scale, repeat, mirror, twist
    Position,
    /// SDF primitives: circle, sphere, ring, box, etc.
    Sdf,
    /// SDF modifiers: mask_arc, displace, round, onion, threshold
    SdfModifier,
    /// Noise generators: fbm, simplex, voronoi, curl_noise, concentric_waves
    Noise,
    /// Distance-to-intensity: glow
    Glow,
    /// Color operations: shade, emissive, tint, gradient, colormap, spectrum, particles
    Color,
    /// Screen-space effects: bloom, vignette, grain, fog, glitch, etc.
    PostProcess,
}

/// A function argument — positional or named.
#[derive(Debug, Clone)]
pub enum IrArg {
    Positional(IrExpr),
    Named { name: String, value: IrExpr },
}

// ── Expressions ───────────────────────────────────────────────────────

/// IR expression — mirrors AST Expr for easy roundtrip conversion.
/// Optimization passes transform these in-place (constant folding, etc.).
#[derive(Debug, Clone)]
pub enum IrExpr {
    Literal(f64),
    String(String),
    Ident(String),
    FieldAccess {
        object: Box<IrExpr>,
        field: String,
    },
    BinOp {
        left: Box<IrExpr>,
        op: BinOp,
        right: Box<IrExpr>,
    },
    Neg(Box<IrExpr>),
    Call {
        name: String,
        args: Vec<IrExpr>,
    },
    Array(Vec<IrExpr>),
    Ternary {
        condition: Box<IrExpr>,
        if_true: Box<IrExpr>,
        if_false: Box<IrExpr>,
    },
}

// ── Uniforms ──────────────────────────────────────────────────────────

/// A compiled uniform parameter.
#[derive(Debug, Clone)]
pub struct IrUniform {
    pub name: String,
    pub field_name: String,
    pub index: usize,
    pub base_value: f64,
    pub mod_js: Option<String>,
    /// Set to true by the dead-uniform-elimination pass if unreferenced.
    pub dead: bool,
}

// ── Classification ────────────────────────────────────────────────────

impl StageKind {
    /// Classify a stage by function name.
    pub fn classify(name: &str) -> Option<StageKind> {
        match name {
            "translate" | "rotate" | "scale" | "repeat" | "mirror" | "twist" => {
                Some(StageKind::Position)
            }
            "circle" | "sphere" | "ring" | "box" | "torus" | "cylinder" | "plane"
            | "line" | "polygon" | "star" | "progress_arc" | "hexgrid" | "shield"
            | "pulse_wave" => Some(StageKind::Sdf),
            "mask_arc" | "displace" | "round" | "onion" | "threshold" => {
                Some(StageKind::SdfModifier)
            }
            "fbm" | "simplex" | "voronoi" | "noise" | "curl_noise"
            | "concentric_waves" => Some(StageKind::Noise),
            "glow" => Some(StageKind::Glow),
            "shade" | "emissive" | "colormap" | "spectrum" | "tint" | "gradient"
            | "particles" => Some(StageKind::Color),
            "bloom" | "chromatic" | "vignette" | "grain" | "fog" | "glitch"
            | "scanlines" | "tonemap" | "invert" | "saturate_color" | "iridescent" => {
                Some(StageKind::PostProcess)
            }
            _ => None,
        }
    }

    /// Whether this stage produces SDF-like output (SDF, modifier, or noise).
    pub fn is_sdf_family(self) -> bool {
        matches!(self, Self::Sdf | Self::SdfModifier | Self::Noise)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

impl IrExpr {
    /// Check if this expression is a constant literal.
    pub fn as_literal(&self) -> Option<f64> {
        match self {
            IrExpr::Literal(v) => Some(*v),
            _ => None,
        }
    }

    /// Check if this expression references a specific identifier.
    pub fn references_ident(&self, name: &str) -> bool {
        match self {
            IrExpr::Ident(n) => n == name,
            IrExpr::FieldAccess { object, .. } => object.references_ident(name),
            IrExpr::BinOp { left, right, .. } => {
                left.references_ident(name) || right.references_ident(name)
            }
            IrExpr::Neg(inner) => inner.references_ident(name),
            IrExpr::Call { args, .. } => args.iter().any(|a| a.references_ident(name)),
            IrExpr::Array(elems) => elems.iter().any(|e| e.references_ident(name)),
            IrExpr::Ternary {
                condition,
                if_true,
                if_false,
            } => {
                condition.references_ident(name)
                    || if_true.references_ident(name)
                    || if_false.references_ident(name)
            }
            IrExpr::Literal(_) | IrExpr::String(_) => false,
        }
    }
}

impl IrStage {
    /// Get a positional argument's expression, or None if out of bounds.
    pub fn positional_arg(&self, index: usize) -> Option<&IrExpr> {
        let mut pos = 0;
        for arg in &self.args {
            match arg {
                IrArg::Positional(expr) => {
                    if pos == index {
                        return Some(expr);
                    }
                    pos += 1;
                }
                IrArg::Named { .. } => {}
            }
        }
        None
    }
}

impl ShaderIR {
    /// Collect all identifier names referenced across all layer stages.
    pub fn referenced_idents(&self) -> std::collections::HashSet<String> {
        let mut idents = std::collections::HashSet::new();
        for layer in &self.layers {
            for stage in &layer.stages {
                for arg in &stage.args {
                    let expr = match arg {
                        IrArg::Positional(e) => e,
                        IrArg::Named { value, .. } => value,
                    };
                    collect_idents(expr, &mut idents);
                }
            }
        }
        idents
    }
}

fn collect_idents(expr: &IrExpr, out: &mut std::collections::HashSet<String>) {
    match expr {
        IrExpr::Ident(name) => {
            out.insert(name.clone());
        }
        IrExpr::FieldAccess { object, .. } => collect_idents(object, out),
        IrExpr::BinOp { left, right, .. } => {
            collect_idents(left, out);
            collect_idents(right, out);
        }
        IrExpr::Neg(inner) => collect_idents(inner, out),
        IrExpr::Call { args, .. } => {
            for a in args {
                collect_idents(a, out);
            }
        }
        IrExpr::Array(elems) => {
            for e in elems {
                collect_idents(e, out);
            }
        }
        IrExpr::Ternary {
            condition,
            if_true,
            if_false,
        } => {
            collect_idents(condition, out);
            collect_idents(if_true, out);
            collect_idents(if_false, out);
        }
        IrExpr::Literal(_) | IrExpr::String(_) => {}
    }
}
