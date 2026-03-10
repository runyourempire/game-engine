//! GAME Lexer — transforms source text into a token stream.
//!
//! Uses the `logos` crate for fast zero-allocation lexing.

use logos::Logos;

use crate::error::CompileError;
use crate::token::Token;

/// Internal logos token — maps 1:1 to our Token enum but with logos derives.
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n]+")]
#[logos(skip r"//[^\n]*")]
enum LexToken {
    // ── Keywords ─────────────────────────────────────────
    #[token("cinematic")]
    Cinematic,
    #[token("layer")]
    Layer,
    #[token("import")]
    Import,
    #[token("as")]
    As,
    #[token("arc")]
    Arc,
    #[token("resonate")]
    Resonate,
    #[token("over")]
    Over,
    #[token("memory")]
    Memory,
    #[token("cast")]
    Cast,
    #[token("opacity")]
    Opacity,
    #[token("blend")]
    Blend,
    #[token("listen")]
    Listen,
    #[token("voice")]
    Voice,
    #[token("score")]
    Score,
    #[token("breed")]
    Breed,
    #[token("from")]
    From,
    #[token("inherit")]
    Inherit,
    #[token("mutate")]
    Mutate,
    #[token("gravity")]
    Gravity,
    #[token("project")]
    Project,
    #[token("react")]
    React,
    #[token("swarm")]
    Swarm,
    #[token("flow")]
    Flow,
    #[token("fn")]
    Fn,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("use")]
    Use,
    #[token("pass")]
    Pass,
    #[token("scene")]
    Scene,
    #[token("play")]
    Play,
    #[token("transition")]
    Transition,
    #[token("for")]
    For,
    #[token("feedback")]
    Feedback,
    #[token("matrix")]
    Matrix,
    #[token("true")]
    True,
    #[token("false")]
    False,

    // ── Literals ─────────────────────────────────────────
    #[regex(r"[0-9]+\.[0-9]+s", priority = 10)]
    FloatSeconds,
    #[regex(r"[0-9]+s", priority = 9)]
    IntSeconds,
    #[regex(r"[0-9]+\.[0-9]+ms", priority = 10)]
    FloatMillis,
    #[regex(r"[0-9]+ms", priority = 9)]
    IntMillis,
    #[regex(r"[0-9]+bars", priority = 9)]
    IntBars,
    #[regex(r"[0-9]+\.[0-9]+deg", priority = 10)]
    FloatDeg,
    #[regex(r"[0-9]+deg", priority = 9)]
    IntDeg,
    #[regex(r"[0-9]+\.[0-9]+", priority = 5)]
    Float,
    #[regex(r"[0-9]+", priority = 4)]
    Integer,
    #[regex(r#""[^"]*""#)]
    StringLit,

    // ── Hex colors ───────────────────────────────────────
    #[regex(r"#[0-9a-fA-F]{6}", priority = 8)]
    HexColor,

    // ── Identifiers ──────────────────────────────────────
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", priority = 1)]
    Ident,

    // ── Multi-char operators ─────────────────────────────
    #[token(">>")]
    ShiftRight,
    #[token("<>")]
    Diamond,
    #[token("!!")]
    BangBang,
    #[token("..")]
    DotDot,
    #[token("->")]
    Arrow,
    #[token(">=")]
    Gte,
    #[token("<=")]
    Lte,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token(">")]
    Gt,
    #[token("<")]
    Lt,

    // ── Single-char operators ────────────────────────────
    #[token("|")]
    Pipe,
    #[token("~")]
    Tilde,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("^")]
    Caret,
    #[token("=")]
    Eq,

    // ── Delimiters ───────────────────────────────────────
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
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
}

/// Lex source text into a vector of `(Token, start, end)` triples.
pub fn lex(source: &str) -> Result<Vec<(Token, usize, usize)>, CompileError> {
    let mut result = Vec::new();
    let mut lexer = LexToken::lexer(source);

    while let Some(tok_result) = lexer.next() {
        let span = lexer.span();
        let slice = &source[span.start..span.end];

        let token = match tok_result {
            Ok(lt) => convert(lt, slice)?,
            Err(()) => {
                return Err(CompileError::lex(
                    span.start,
                    span.end,
                    format!("unexpected character: '{slice}'"),
                ));
            }
        };

        result.push((token, span.start, span.end));
    }

    Ok(result)
}

fn convert(lt: LexToken, slice: &str) -> Result<Token, CompileError> {
    Ok(match lt {
        // Keywords
        LexToken::Cinematic => Token::Cinematic,
        LexToken::Layer => Token::Layer,
        LexToken::Import => Token::Import,
        LexToken::As => Token::As,
        LexToken::Arc => Token::Arc,
        LexToken::Resonate => Token::Resonate,
        LexToken::Over => Token::Over,
        LexToken::Memory => Token::Memory,
        LexToken::Cast => Token::Cast,
        LexToken::Opacity => Token::Opacity,
        LexToken::Blend => Token::Blend,
        LexToken::Listen => Token::Listen,
        LexToken::Voice => Token::Voice,
        LexToken::Score => Token::Score,
        LexToken::Breed => Token::Breed,
        LexToken::From => Token::From,
        LexToken::Inherit => Token::Inherit,
        LexToken::Mutate => Token::Mutate,
        LexToken::Gravity => Token::Gravity,
        LexToken::Project => Token::Project,
        LexToken::React => Token::React,
        LexToken::Swarm => Token::Swarm,
        LexToken::Flow => Token::Flow,
        LexToken::Fn => Token::Fn,
        LexToken::If => Token::If,
        LexToken::Else => Token::Else,
        LexToken::Use => Token::Use,
        LexToken::Pass => Token::Pass,
        LexToken::Scene => Token::Scene,
        LexToken::Play => Token::Play,
        LexToken::Transition => Token::Transition,
        LexToken::For => Token::For,
        LexToken::Feedback => Token::Feedback,
        LexToken::Matrix => Token::Matrix,
        LexToken::True => Token::Ident("true".into()),
        LexToken::False => Token::Ident("false".into()),

        // Units — parse number from slice
        LexToken::FloatSeconds => {
            let v: f64 = slice[..slice.len() - 1].parse().unwrap_or(0.0);
            Token::Seconds(v)
        }
        LexToken::IntSeconds => {
            let v: f64 = slice[..slice.len() - 1].parse().unwrap_or(0.0);
            Token::Seconds(v)
        }
        LexToken::FloatMillis => {
            let v: f64 = slice[..slice.len() - 2].parse().unwrap_or(0.0);
            Token::Millis(v)
        }
        LexToken::IntMillis => {
            let v: f64 = slice[..slice.len() - 2].parse().unwrap_or(0.0);
            Token::Millis(v)
        }
        LexToken::IntBars => {
            let v: i64 = slice[..slice.len() - 4].parse().unwrap_or(0);
            Token::Bars(v)
        }
        LexToken::FloatDeg => {
            let v: f64 = slice[..slice.len() - 3].parse().unwrap_or(0.0);
            Token::Degrees(v)
        }
        LexToken::IntDeg => {
            let v: f64 = slice[..slice.len() - 3].parse().unwrap_or(0.0);
            Token::Degrees(v)
        }

        // Numeric
        LexToken::Float => Token::Float(slice.parse().unwrap_or(0.0)),
        LexToken::Integer => Token::Integer(slice.parse().unwrap_or(0)),

        // String
        LexToken::StringLit => Token::StringLit(slice[1..slice.len() - 1].to_string()),

        // Hex color
        LexToken::HexColor => {
            let hex = &slice[1..]; // strip #
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f64 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f64 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f64 / 255.0;
            Token::HexColor(r, g, b)
        }

        // Ident
        LexToken::Ident => Token::Ident(slice.to_string()),

        // Operators
        LexToken::ShiftRight => Token::ShiftRight,
        LexToken::Diamond => Token::Diamond,
        LexToken::BangBang => Token::BangBang,
        LexToken::DotDot => Token::DotDot,
        LexToken::Arrow => Token::Arrow,
        LexToken::Gte => Token::Gte,
        LexToken::Lte => Token::Lte,
        LexToken::EqEq => Token::EqEq,
        LexToken::NotEq => Token::NotEq,
        LexToken::Gt => Token::Gt,
        LexToken::Lt => Token::Lt,
        LexToken::Pipe => Token::Pipe,
        LexToken::Tilde => Token::Tilde,
        LexToken::Plus => Token::Plus,
        LexToken::Minus => Token::Minus,
        LexToken::Star => Token::Star,
        LexToken::Slash => Token::Slash,
        LexToken::Caret => Token::Caret,
        LexToken::Eq => Token::Eq,

        // Delimiters
        LexToken::LBrace => Token::LBrace,
        LexToken::RBrace => Token::RBrace,
        LexToken::LParen => Token::LParen,
        LexToken::RParen => Token::RParen,
        LexToken::LBracket => Token::LBracket,
        LexToken::RBracket => Token::RBracket,
        LexToken::Colon => Token::Colon,
        LexToken::Comma => Token::Comma,
        LexToken::Dot => Token::Dot,
    })
}

// ── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(src: &str) -> Vec<Token> {
        lex(src).unwrap().into_iter().map(|(t, _, _)| t).collect()
    }

    #[test]
    fn lex_keywords() {
        assert_eq!(tokens("cinematic"), vec![Token::Cinematic]);
        assert_eq!(tokens("layer"), vec![Token::Layer]);
        assert_eq!(
            tokens("import as arc resonate over memory cast"),
            vec![
                Token::Import,
                Token::As,
                Token::Arc,
                Token::Resonate,
                Token::Over,
                Token::Memory,
                Token::Cast,
            ]
        );
    }

    #[test]
    fn lex_numbers() {
        assert_eq!(tokens("42"), vec![Token::Integer(42)]);
        assert_eq!(tokens("3.14"), vec![Token::Float(3.14)]);
    }

    #[test]
    fn lex_units() {
        assert_eq!(tokens("2s"), vec![Token::Seconds(2.0)]);
        assert_eq!(tokens("0.5s"), vec![Token::Seconds(0.5)]);
        assert_eq!(tokens("500ms"), vec![Token::Millis(500.0)]);
        assert_eq!(tokens("4bars"), vec![Token::Bars(4)]);
        assert_eq!(tokens("180deg"), vec![Token::Degrees(180.0)]);
    }

    #[test]
    fn lex_string() {
        assert_eq!(tokens(r#""hello""#), vec![Token::StringLit("hello".into())]);
    }

    #[test]
    fn lex_identifiers() {
        assert_eq!(
            tokens("foo bar_baz"),
            vec![Token::Ident("foo".into()), Token::Ident("bar_baz".into()),]
        );
    }

    #[test]
    fn lex_operators() {
        assert_eq!(
            tokens("| ~ >> <> !! .. ->"),
            vec![
                Token::Pipe,
                Token::Tilde,
                Token::ShiftRight,
                Token::Diamond,
                Token::BangBang,
                Token::DotDot,
                Token::Arrow,
            ]
        );
    }

    #[test]
    fn lex_full_layer() {
        let toks = tokens(r#"layer ring { circle(0.2) | glow(1.5) | tint(0.831, 0.686, 0.216) }"#);
        assert_eq!(toks[0], Token::Layer);
        assert_eq!(toks[1], Token::Ident("ring".into()));
        assert_eq!(toks[2], Token::LBrace);
        assert!(toks.contains(&Token::Pipe));
    }

    #[test]
    fn lex_comments_skipped() {
        assert_eq!(
            tokens("foo // comment\nbar"),
            vec![Token::Ident("foo".into()), Token::Ident("bar".into()),]
        );
    }

    #[test]
    fn lex_hex_color() {
        let toks = tokens("#FF6B35");
        assert_eq!(toks.len(), 1);
        match &toks[0] {
            Token::HexColor(r, g, b) => {
                assert!((r - 1.0).abs() < 0.01);
                assert!((g - 0.42).abs() < 0.01);
                assert!((b - 0.21).abs() < 0.01);
            }
            _ => panic!("expected HexColor"),
        }
    }

    #[test]
    fn lex_hex_color_in_pipeline() {
        let toks = tokens("tint(#D4AF37)");
        assert!(toks.iter().any(|t| matches!(t, Token::HexColor(_, _, _))));
    }

    #[test]
    fn lex_matrix_keyword() {
        assert_eq!(tokens("matrix"), vec![Token::Matrix]);
        assert_eq!(
            tokens("matrix coupling"),
            vec![Token::Matrix, Token::Ident("coupling".into())]
        );
    }

    #[test]
    fn lex_modulation() {
        let toks = tokens("radius: 0.3 ~ audio.bass * 0.1");
        assert!(toks.contains(&Token::Tilde));
    }
}
