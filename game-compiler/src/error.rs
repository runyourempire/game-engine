use std::fmt;
use std::ops::Range;

/// All errors produced by the GAME compiler.
#[derive(Debug)]
pub struct GameError {
    pub kind: ErrorKind,
    pub span: Option<Range<usize>>,
    pub source_text: Option<String>,
}

#[derive(Debug)]
pub enum ErrorKind {
    /// Lexer encountered an unrecognized character/sequence.
    UnrecognizedToken(String),
    /// Parser expected one thing, got another.
    UnexpectedToken {
        expected: String,
        got: String,
    },
    /// Parser reached end of input unexpectedly.
    UnexpectedEof {
        expected: String,
    },
    /// Codegen encountered an unknown primitive/function.
    UnknownFunction(String),
    /// General message.
    Message(String),
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::UnrecognizedToken(tok) => {
                write!(f, "unrecognized token: {tok}")
            }
            ErrorKind::UnexpectedToken { expected, got } => {
                write!(f, "expected {expected}, got {got}")
            }
            ErrorKind::UnexpectedEof { expected } => {
                write!(f, "unexpected end of file, expected {expected}")
            }
            ErrorKind::UnknownFunction(name) => {
                write!(f, "unknown built-in function: {name}")
            }
            ErrorKind::Message(msg) => write!(f, "{msg}"),
        }?;

        if let Some(span) = &self.span {
            write!(f, " (at byte {}..{})", span.start, span.end)?;
        }

        Ok(())
    }
}

impl std::error::Error for GameError {}

pub type Result<T> = std::result::Result<T, GameError>;

/// Shorthand constructors.
impl GameError {
    pub fn unexpected_token(expected: &str, got: &str, span: Range<usize>) -> Self {
        Self {
            kind: ErrorKind::UnexpectedToken {
                expected: expected.to_string(),
                got: got.to_string(),
            },
            span: Some(span),
            source_text: None,
        }
    }

    pub fn unexpected_eof(expected: &str) -> Self {
        Self {
            kind: ErrorKind::UnexpectedEof {
                expected: expected.to_string(),
            },
            span: None,
            source_text: None,
        }
    }

    pub fn unknown_function(name: &str) -> Self {
        Self {
            kind: ErrorKind::UnknownFunction(name.to_string()),
            span: None,
            source_text: None,
        }
    }

    /// General parse/resolve error with a message.
    pub fn parse(msg: &str) -> Self {
        Self {
            kind: ErrorKind::Message(msg.to_string()),
            span: None,
            source_text: None,
        }
    }
}
