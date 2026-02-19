use logos::Logos;

use crate::error::{ErrorKind, GameError, Result};
use crate::token::{Spanned, Token};

/// Tokenize a `.game` source string into a vector of spanned tokens.
pub fn lex(source: &str) -> Result<Vec<Spanned>> {
    let mut tokens = Vec::new();
    let mut lexer = Token::lexer(source);

    while let Some(result) = lexer.next() {
        let span = lexer.span();
        match result {
            Ok(token) => {
                tokens.push(Spanned { token, span });
            }
            Err(()) => {
                let fragment = &source[span.clone()];
                return Err(GameError {
                    kind: ErrorKind::UnrecognizedToken(fragment.to_string()),
                    span: Some(span),
                    source_text: None,
                });
            }
        }
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_hello_game() {
        let source = r#"
            # Hello GAME
            cinematic "Hello" {
              layer {
                fn: circle(0.3 + sin(time) * 0.05) | glow(2.0)
              }
            }
        "#;

        let tokens = lex(source).expect("lexing should succeed");
        let kinds: Vec<_> = tokens.iter().map(|t| &t.token).collect();

        // cinematic "Hello" {
        assert_eq!(kinds[0], &Token::Cinematic);
        assert!(matches!(kinds[1], Token::String(s) if s == "Hello"));
        assert_eq!(kinds[2], &Token::LBrace);

        // layer {
        assert_eq!(kinds[3], &Token::Layer);
        assert_eq!(kinds[4], &Token::LBrace);

        // fn : circle ( 0.3 + sin ( time ) * 0.05 ) | glow ( 2.0 )
        assert!(matches!(kinds[5], Token::Ident(s) if s == "fn"));
        assert_eq!(kinds[6], &Token::Colon);
        assert!(matches!(kinds[7], Token::Ident(s) if s == "circle"));
        assert_eq!(kinds[8], &Token::LParen);
        assert!(matches!(kinds[9], Token::Float(v) if (*v - 0.3).abs() < 1e-10));
        assert_eq!(kinds[10], &Token::Plus);
        assert!(matches!(kinds[11], Token::Ident(s) if s == "sin"));
        assert_eq!(kinds[12], &Token::LParen);
        assert!(matches!(kinds[13], Token::Ident(s) if s == "time"));
        assert_eq!(kinds[14], &Token::RParen);
        assert_eq!(kinds[15], &Token::Star);
        assert!(matches!(kinds[16], Token::Float(v) if (*v - 0.05).abs() < 1e-10));
        assert_eq!(kinds[17], &Token::RParen);
        assert_eq!(kinds[18], &Token::Pipe);
        assert!(matches!(kinds[19], Token::Ident(s) if s == "glow"));
        assert_eq!(kinds[20], &Token::LParen);
        assert!(matches!(kinds[21], Token::Float(v) if (*v - 2.0).abs() < 1e-10));
        assert_eq!(kinds[22], &Token::RParen);

        // } }
        assert_eq!(kinds[23], &Token::RBrace);
        assert_eq!(kinds[24], &Token::RBrace);
        assert_eq!(tokens.len(), 25);
    }

    #[test]
    fn lex_comments_are_skipped() {
        let source = "# this is a comment\ncinematic {}";
        let tokens = lex(source).unwrap();
        assert_eq!(tokens[0].token, Token::Cinematic);
    }

    #[test]
    fn lex_pipe_and_tilde() {
        let source = "scale: 2.0 ~ audio.bass";
        let tokens = lex(source).unwrap();
        let kinds: Vec<_> = tokens.iter().map(|t| &t.token).collect();
        assert!(matches!(kinds[0], Token::Ident(s) if s == "scale"));
        assert_eq!(kinds[1], &Token::Colon);
        assert!(matches!(kinds[2], Token::Float(v) if (*v - 2.0).abs() < 1e-10));
        assert_eq!(kinds[3], &Token::Tilde);
        assert!(matches!(kinds[4], Token::Ident(s) if s == "audio"));
        assert_eq!(kinds[5], &Token::Dot);
        assert!(matches!(kinds[6], Token::Ident(s) if s == "bass"));
    }

    #[test]
    fn lex_arrow() {
        let source = "terrain.scale -> 2.0";
        let tokens = lex(source).unwrap();
        // terrain . scale -> 2.0
        // [0]    [1] [2]  [3] [4]
        assert_eq!(tokens[3].token, Token::Arrow);
    }
}
