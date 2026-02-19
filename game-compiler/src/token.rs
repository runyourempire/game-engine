use logos::Logos;

/// Tokens produced by lexing a `.game` file.
///
/// Design: keywords are kept minimal — only tokens that change parsing structure.
/// Everything else (fn, mode, depth, etc.) is an Ident and the parser gives it meaning.
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n]+|#[^\n]*")]
pub enum Token {
    // ── Structural keywords ────────────────────────────────────────────
    #[token("cinematic")]
    Cinematic,
    #[token("layer")]
    Layer,
    #[token("lens")]
    Lens,
    #[token("arc")]
    Arc,
    #[token("react")]
    React,
    #[token("resonate")]
    Resonate,
    #[token("define")]
    Define,
    #[token("import")]
    Import,
    #[token("expose")]
    Expose,
    #[token("ease")]
    Ease,
    #[token("over")]
    Over,
    #[token("ALL")]
    All,

    // ── Literals ───────────────────────────────────────────────────────
    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    Float(f64),

    #[regex(r"[0-9]+", priority = 2, callback = |lex| lex.slice().parse::<u64>().ok())]
    Int(u64),

    #[regex(r#""[^"]*""#, |lex| {
        let s = lex.slice();
        Some(s[1..s.len()-1].to_string())
    })]
    String(String),

    // ── Identifiers (catch-all for names, property keys, colors, etc.) ─
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", priority = 1, callback = |lex| Some(lex.slice().to_string()))]
    Ident(String),

    // ── Operators ──────────────────────────────────────────────────────
    #[token("|")]
    Pipe,
    #[token("~")]
    Tilde,
    #[token("->")]
    Arrow,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token(">")]
    Greater,
    #[token("<")]
    Less,
    #[token("?")]
    Question,

    // ── Delimiters ─────────────────────────────────────────────────────
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
}

impl Token {
    /// Human-readable name for error messages.
    pub fn describe(&self) -> &'static str {
        match self {
            Token::Cinematic => "'cinematic'",
            Token::Layer => "'layer'",
            Token::Lens => "'lens'",
            Token::Arc => "'arc'",
            Token::React => "'react'",
            Token::Resonate => "'resonate'",
            Token::Define => "'define'",
            Token::Import => "'import'",
            Token::Expose => "'expose'",
            Token::Ease => "'ease'",
            Token::Over => "'over'",
            Token::All => "'ALL'",
            Token::Float(_) => "float",
            Token::Int(_) => "integer",
            Token::String(_) => "string",
            Token::Ident(_) => "identifier",
            Token::Pipe => "'|'",
            Token::Tilde => "'~'",
            Token::Arrow => "'->'",
            Token::Colon => "':'",
            Token::Comma => "','",
            Token::Dot => "'.'",
            Token::Plus => "'+'",
            Token::Minus => "'-'",
            Token::Star => "'*'",
            Token::Slash => "'/'",
            Token::Greater => "'>'",
            Token::Less => "'<'",
            Token::Question => "'?'",
            Token::LBrace => "'{'",
            Token::RBrace => "'}'",
            Token::LParen => "'('",
            Token::RParen => "')'",
            Token::LBracket => "'['",
            Token::RBracket => "']'",
        }
    }
}

/// A token with its source location (byte offset span).
#[derive(Debug, Clone)]
pub struct Spanned {
    pub token: Token,
    pub span: std::ops::Range<usize>,
}
