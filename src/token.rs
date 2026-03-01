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
    Over,

    // --- punctuation ---
    Pipe,      // |
    Tilde,     // ~
    LBrace,    // {
    RBrace,    // }
    LParen,    // (
    RParen,    // )
    LBracket,  // [
    RBracket,  // ]
    Colon,     // :
    Comma,     // ,
    Dot,       // .
    Plus,      // +
    Minus,     // -
    Star,      // *
    Slash,     // /
    Caret,     // ^
    Eq,        // =
    Arrow,     // ->
    ShiftRight,// >>
    Diamond,   // <>
    BangBang,  // !!
    DotDot,    // ..

    // --- literals ---
    Float(f64),
    Integer(i64),
    StringLit(String),
    Ident(String),

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
            Token::Over => write!(f, "over"),
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
            Token::Float(v) => write!(f, "{v}"),
            Token::Integer(v) => write!(f, "{v}"),
            Token::StringLit(s) => write!(f, "\"{s}\""),
            Token::Ident(s) => write!(f, "{s}"),
            Token::Seconds(v) => write!(f, "{v}s"),
            Token::Millis(v) => write!(f, "{v}ms"),
            Token::Bars(v) => write!(f, "{v}bars"),
            Token::Degrees(v) => write!(f, "{v}deg"),
            Token::Hz => write!(f, "Hz"),
            Token::Bpm => write!(f, "bpm"),
        }
    }
}
