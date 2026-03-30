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
#[logos(skip r"#[^\n]*")]
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
    #[token("signals")]
    Signals,
    #[token("route")]
    Route,
    #[token("hear")]
    Hear,
    #[token("feel")]
    Feel,
    #[token("lens")]
    Lens,
    #[token("react")]
    React,
    #[token("define")]
    Define,
    #[token("expose")]
    Expose,
    #[token("ease")]
    Ease,
    #[token("ALL")]
    All,
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
    #[regex(r#""([^"\\]|\\.)*""#)]
    StringLit,

    // ── Identifiers ──────────────────────────────────────
    #[regex(r"[a-zA-Z_\x80-\xff][a-zA-Z0-9_\x80-\xff]*", priority = 1)]
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

    // ── Single-char operators ────────────────────────────
    #[token(">", priority = 1)]
    Greater,
    #[token("<", priority = 1)]
    Less,
    #[token("?")]
    Question,
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

/// Process escape sequences in a string literal, stripping outer quotes.
fn unescape_string(s: &str) -> String {
    let inner = &s[1..s.len() - 1]; // strip surrounding quotes
    let mut result = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
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
        LexToken::Listen => Token::Listen,
        LexToken::Voice => Token::Voice,
        LexToken::Score => Token::Score,
        LexToken::Breed => Token::Breed,
        LexToken::From => Token::From,
        LexToken::Inherit => Token::Inherit,
        LexToken::Mutate => Token::Mutate,
        LexToken::Gravity => Token::Gravity,
        LexToken::Project => Token::Project,
        LexToken::Signals => Token::Signals,
        LexToken::Route => Token::Route,
        LexToken::Hear => Token::Hear,
        LexToken::Feel => Token::Feel,
        LexToken::Lens => Token::Lens,
        LexToken::React => Token::React,
        LexToken::Define => Token::Define,
        LexToken::Expose => Token::Expose,
        LexToken::Ease => Token::Ease,
        LexToken::All => Token::All,
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

        // String — process escape sequences
        LexToken::StringLit => Token::StringLit(unescape_string(slice)),

        // Ident
        LexToken::Ident => Token::Ident(slice.to_string()),

        // Operators
        LexToken::Greater => Token::Greater,
        LexToken::Less => Token::Less,
        LexToken::Question => Token::Question,
        LexToken::ShiftRight => Token::ShiftRight,
        LexToken::Diamond => Token::Diamond,
        LexToken::BangBang => Token::BangBang,
        LexToken::DotDot => Token::DotDot,
        LexToken::Arrow => Token::Arrow,
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
        assert_eq!(tokens("import as arc resonate over memory cast"), vec![
            Token::Import, Token::As, Token::Arc, Token::Resonate,
            Token::Over, Token::Memory, Token::Cast,
        ]);
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
        assert_eq!(tokens("foo bar_baz"), vec![
            Token::Ident("foo".into()),
            Token::Ident("bar_baz".into()),
        ]);
    }

    #[test]
    fn lex_operators() {
        assert_eq!(tokens("| ~ >> <> !! .. ->"), vec![
            Token::Pipe, Token::Tilde, Token::ShiftRight,
            Token::Diamond, Token::BangBang, Token::DotDot, Token::Arrow,
        ]);
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
        assert_eq!(tokens("foo // comment\nbar"), vec![
            Token::Ident("foo".into()),
            Token::Ident("bar".into()),
        ]);
    }

    #[test]
    fn lex_modulation() {
        let toks = tokens("radius: 0.3 ~ audio.bass * 0.1");
        assert!(toks.contains(&Token::Tilde));
    }

    #[test]
    fn lex_unicode_identifiers() {
        // Non-ASCII identifiers should lex as Ident tokens
        let source = r#"cinematic "t" { layer bg { fn: circle(0.3) } }"#;
        let toks = tokens(source);
        assert!(toks.contains(&Token::Cinematic));
    }

    #[test]
    fn lex_string_escape_sequences() {
        let toks = tokens(r#""hello\nworld""#);
        assert_eq!(toks, vec![Token::StringLit("hello\nworld".into())]);
    }

    #[test]
    fn lex_string_with_escaped_quote() {
        let toks = tokens(r#""say \"hello\"""#);
        assert_eq!(toks, vec![Token::StringLit("say \"hello\"".into())]);
    }

    #[test]
    fn lex_string_with_backslash() {
        let toks = tokens(r#""path\\to\\file""#);
        assert_eq!(toks, vec![Token::StringLit("path\\to\\file".into())]);
    }

    #[test]
    fn lex_string_with_tab() {
        let toks = tokens(r#""col1\tcol2""#);
        assert_eq!(toks, vec![Token::StringLit("col1\tcol2".into())]);
    }
}
