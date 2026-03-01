use thiserror::Error;

/// Byte-offset span within source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// All errors produced during compilation.
#[derive(Debug, Error)]
pub enum CompileError {
    #[error("lex error at {span:?}: {message}")]
    LexError { span: Span, message: String },

    #[error("parse error at {line}:{col}: {message}")]
    ParseError {
        line: usize,
        col: usize,
        message: String,
    },

    #[error("validation error: {message}")]
    ValidationError { message: String },

    #[error("codegen error: {message}")]
    CodegenError { message: String },

    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

impl CompileError {
    pub fn lex(start: usize, end: usize, msg: impl Into<String>) -> Self {
        Self::LexError {
            span: Span { start, end },
            message: msg.into(),
        }
    }

    pub fn parse(line: usize, col: usize, msg: impl Into<String>) -> Self {
        Self::ParseError {
            line,
            col,
            message: msg.into(),
        }
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        Self::ValidationError {
            message: msg.into(),
        }
    }

    pub fn codegen(msg: impl Into<String>) -> Self {
        Self::CodegenError {
            message: msg.into(),
        }
    }
}
