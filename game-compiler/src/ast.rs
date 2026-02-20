//! Abstract Syntax Tree for the `.game` language.
//!
//! The AST is the contract between the parser and all downstream phases
//! (resolver, codegen, runtime). Every `.game` concept has exactly one
//! representation here.

/// Root node — a single `.game` file parses into one Cinematic.
#[derive(Debug, Clone)]
pub struct Cinematic {
    pub name: Option<String>,
    pub properties: Vec<Property>,
    pub layers: Vec<Layer>,
    pub lenses: Vec<Lens>,
    pub arc: Option<ArcBlock>,
    pub react: Option<ReactBlock>,
    pub resonance: Option<ResonanceBlock>,
    pub defines: Vec<DefineBlock>,
}

/// A key-value property: `resolution: 1920x1080` or `audio: "track.ogg"`
#[derive(Debug, Clone)]
pub struct Property {
    pub name: String,
    pub value: Expr,
}

/// A generative layer — the fundamental visual unit.
#[derive(Debug, Clone)]
pub struct Layer {
    pub name: Option<String>,
    pub fn_chain: Option<PipeChain>,
    pub params: Vec<ParamDecl>,
    pub properties: Vec<Property>,
}

/// A parameter declaration with optional modulation.
/// Example: `scale: 2.0 ~ audio.bass * 1.5`
#[derive(Debug, Clone)]
pub struct ParamDecl {
    pub name: String,
    pub base_value: Expr,
    pub modulation: Option<Modulation>,
}

/// The `~` modulation binding.
#[derive(Debug, Clone)]
pub struct Modulation {
    pub signal: Expr,
}

/// A chain of piped function calls: `sphere(0.5) | twist(time) | glow(2.0)`
#[derive(Debug, Clone)]
pub struct PipeChain {
    pub stages: Vec<FnCall>,
}

/// A function call: `circle(0.3)` or `shade(albedo: gold, roughness: 0.3)`
#[derive(Debug, Clone)]
pub struct FnCall {
    pub name: String,
    pub args: Vec<Arg>,
}

/// Function argument — positional or named.
#[derive(Debug, Clone)]
pub enum Arg {
    Positional(Expr),
    Named { name: String, value: Expr },
}

/// Expression — the universal value type.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Numeric literal: `0.3`, `42`
    Number(f64),
    /// String literal: `"track.ogg"`
    String(String),
    /// Identifier or dotted path: `time`, `audio.bass`, `gold`
    Ident(String),
    /// Dotted field access: `audio.bass`
    FieldAccess {
        object: Box<Expr>,
        field: String,
    },
    /// Binary operation: `0.3 + sin(time) * 0.05`
    BinaryOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
    /// Unary negation: `-0.5`
    Negate(Box<Expr>),
    /// Function call in expression position: `sin(time)`, `mix(a, b, t)`
    Call(FnCall),
    /// Array/vector literal: `[0.5, 0.8, 1.0]`
    Array(Vec<Expr>),
    /// Ternary: `height > 0.7 ? gold : black`
    Ternary {
        condition: Box<Expr>,
        if_true: Box<Expr>,
        if_false: Box<Expr>,
    },
}

/// Binary operators with standard precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,    // +
    Sub,    // -
    Mul,    // *
    Div,    // /
    Gt,     // >
    Lt,     // <
}

impl BinOp {
    /// Precedence level (higher binds tighter).
    pub fn precedence(self) -> u8 {
        match self {
            BinOp::Gt | BinOp::Lt => 1,
            BinOp::Add | BinOp::Sub => 2,
            BinOp::Mul | BinOp::Div => 3,
        }
    }
}

// ── Lens ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Lens {
    pub name: Option<String>,
    pub properties: Vec<Property>,
    pub post: Vec<FnCall>,
}

// ── Arc ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ArcBlock {
    pub moments: Vec<Moment>,
}

#[derive(Debug, Clone)]
pub struct Moment {
    pub time_seconds: f64,
    pub name: Option<String>,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone)]
pub struct Transition {
    pub target: String,
    pub value: Expr,
    pub is_animated: bool,
    pub easing: Option<String>,
    pub duration_secs: Option<f64>,
}

// ── React ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ReactBlock {
    pub reactions: Vec<Reaction>,
}

#[derive(Debug, Clone)]
pub struct Reaction {
    pub signal: Expr,
    pub action: Expr,
}

// ── Resonate ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ResonanceBlock {
    pub bindings: Vec<ResonanceBinding>,
    pub damping: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ResonanceBinding {
    pub target: String,
    pub source: Expr,
}

// ── Define ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DefineBlock {
    pub name: String,
    pub params: Vec<String>,
    pub body: PipeChain,
}
