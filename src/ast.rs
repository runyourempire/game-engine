/// Root of a GAME program.
#[derive(Debug, Clone)]
pub struct Program {
    pub imports: Vec<Import>,
    pub cinematics: Vec<Cinematic>,
}

/// `import "path" as alias`
#[derive(Debug, Clone)]
pub struct Import {
    pub path: String,
    pub alias: String,
}

/// `cinematic "name" { layers, arcs, resonates }`
#[derive(Debug, Clone)]
pub struct Cinematic {
    pub name: String,
    pub layers: Vec<Layer>,
    pub arcs: Vec<ArcBlock>,
    pub resonates: Vec<ResonateBlock>,
}

/// `layer ident [(opts)] [memory: f] [cast kind] { body }`
#[derive(Debug, Clone)]
pub struct Layer {
    pub name: String,
    pub opts: Vec<Param>,
    pub memory: Option<f64>,
    pub cast: Option<String>,
    pub body: LayerBody,
}

/// A layer body is either a list of named params or a stage pipeline.
#[derive(Debug, Clone)]
pub enum LayerBody {
    Params(Vec<Param>),
    Pipeline(Vec<Stage>),
}

/// `name: value [~ modulation]`
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub value: Expr,
    pub modulation: Option<Expr>,
}

/// A single stage in a pipeline: `name(args)`
#[derive(Debug, Clone)]
pub struct Stage {
    pub name: String,
    pub args: Vec<Arg>,
}

/// An argument — optionally named.
#[derive(Debug, Clone)]
pub struct Arg {
    pub name: Option<String>,
    pub value: Expr,
}

/// `arc { entries }`
#[derive(Debug, Clone)]
pub struct ArcBlock {
    pub entries: Vec<ArcEntry>,
}

/// `target: from -> to over duration [easing]`
#[derive(Debug, Clone)]
pub struct ArcEntry {
    pub target: String,
    pub from: Expr,
    pub to: Expr,
    pub duration: Duration,
    pub easing: Option<String>,
}

/// `resonate { entries }`
#[derive(Debug, Clone)]
pub struct ResonateBlock {
    pub entries: Vec<ResonateEntry>,
}

/// `source -> target.field * weight`
#[derive(Debug, Clone)]
pub struct ResonateEntry {
    pub source: String,
    pub target: String,
    pub field: String,
    pub weight: Expr,
}

/// Time durations supported by the language.
#[derive(Debug, Clone, PartialEq)]
pub enum Duration {
    Seconds(f64),
    Millis(f64),
    Bars(i64),
}

/// Binary operators.
#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

/// Expression tree.
#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    String(String),
    Ident(String),
    DottedIdent { object: String, field: String },
    Array(Vec<Expr>),
    Paren(Box<Expr>),
    Neg(Box<Expr>),
    BinOp { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    Call { name: String, args: Vec<Arg> },
    Duration(Duration),
}
