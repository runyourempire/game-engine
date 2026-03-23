/// A source-location span: (start_byte, end_byte).
pub type Spanned<T> = (T, usize, usize);

/// Every lexeme the GAME language can produce.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // --- keywords ---
    Cinematic,
    Layer,
    Import,
    As,
    Arc,
    Resonate,
    Memory,
    Cast,
    Opacity,
    Blend,
    Over,
    Listen,
    Voice,
    Score,
    Breed,
    From,
    Inherit,
    Mutate,
    Gravity,
    Project,
    React,
    Swarm,
    Flow,
    Particles,
    Fn,
    If,
    Else,
    Use,
    Pass,
    Scene,
    Play,
    Transition,
    For,
    Feedback,
    Matrix,
    Props,
    Dom,

    // --- punctuation ---
    Pipe,       // |
    Tilde,      // ~
    LBrace,     // {
    RBrace,     // }
    LParen,     // (
    RParen,     // )
    LBracket,   // [
    RBracket,   // ]
    Colon,      // :
    Comma,      // ,
    Dot,        // .
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Caret,      // ^
    Eq,         // =
    Arrow,      // ->
    ShiftRight, // >>
    Diamond,    // <>
    BangBang,   // !!
    DotDot,     // ..
    Gt,         // >
    Lt,         // <
    Gte,        // >=
    Lte,        // <=
    EqEq,       // ==
    NotEq,      // !=

    // --- literals ---
    Float(f64),
    Integer(i64),
    StringLit(String),
    Ident(String),
    /// Pre-parsed hex color #RRGGBB as normalized RGB (0.0-1.0)
    HexColor(f64, f64, f64),

    // --- units (number already embedded) ---
    Seconds(f64),
    Millis(f64),
    Bars(i64),
    Degrees(f64),

    // --- unit keywords ---
    Hz,
    Bpm,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Cinematic => write!(f, "cinematic"),
            Token::Layer => write!(f, "layer"),
            Token::Import => write!(f, "import"),
            Token::As => write!(f, "as"),
            Token::Arc => write!(f, "arc"),
            Token::Resonate => write!(f, "resonate"),
            Token::Memory => write!(f, "memory"),
            Token::Cast => write!(f, "cast"),
            Token::Opacity => write!(f, "opacity"),
            Token::Blend => write!(f, "blend"),
            Token::Over => write!(f, "over"),
            Token::Listen => write!(f, "listen"),
            Token::Voice => write!(f, "voice"),
            Token::Score => write!(f, "score"),
            Token::Breed => write!(f, "breed"),
            Token::From => write!(f, "from"),
            Token::Inherit => write!(f, "inherit"),
            Token::Mutate => write!(f, "mutate"),
            Token::Gravity => write!(f, "gravity"),
            Token::Project => write!(f, "project"),
            Token::React => write!(f, "react"),
            Token::Swarm => write!(f, "swarm"),
            Token::Flow => write!(f, "flow"),
            Token::Particles => write!(f, "particles"),
            Token::Fn => write!(f, "fn"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::Use => write!(f, "use"),
            Token::Pass => write!(f, "pass"),
            Token::Scene => write!(f, "scene"),
            Token::Play => write!(f, "play"),
            Token::Transition => write!(f, "transition"),
            Token::For => write!(f, "for"),
            Token::Feedback => write!(f, "feedback"),
            Token::Matrix => write!(f, "matrix"),
            Token::Props => write!(f, "props"),
            Token::Dom => write!(f, "dom"),
            Token::Pipe => write!(f, "|"),
            Token::Tilde => write!(f, "~"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::Colon => write!(f, ":"),
            Token::Comma => write!(f, ","),
            Token::Dot => write!(f, "."),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Caret => write!(f, "^"),
            Token::Eq => write!(f, "="),
            Token::Arrow => write!(f, "->"),
            Token::ShiftRight => write!(f, ">>"),
            Token::Diamond => write!(f, "<>"),
            Token::BangBang => write!(f, "!!"),
            Token::DotDot => write!(f, ".."),
            Token::Gt => write!(f, ">"),
            Token::Lt => write!(f, "<"),
            Token::Gte => write!(f, ">="),
            Token::Lte => write!(f, "<="),
            Token::EqEq => write!(f, "=="),
            Token::NotEq => write!(f, "!="),
            Token::Float(v) => write!(f, "{v}"),
            Token::Integer(v) => write!(f, "{v}"),
            Token::StringLit(s) => write!(f, "\"{s}\""),
            Token::Ident(s) => write!(f, "{s}"),
            Token::HexColor(r, g, b) => write!(
                f,
                "#{:02X}{:02X}{:02X}",
                (r * 255.0) as u8,
                (g * 255.0) as u8,
                (b * 255.0) as u8
            ),
            Token::Seconds(v) => write!(f, "{v}s"),
            Token::Millis(v) => write!(f, "{v}ms"),
            Token::Bars(v) => write!(f, "{v}bars"),
            Token::Degrees(v) => write!(f, "{v}deg"),
            Token::Hz => write!(f, "Hz"),
            Token::Bpm => write!(f, "bpm"),
        }
    }
}
