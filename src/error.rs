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

/// LSP-compatible diagnostic representation.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub line: usize,
    pub col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub severity: DiagnosticSeverity,
    pub message: String,
}

/// Diagnostic severity levels matching LSP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl CompileError {
    /// Convert to LSP-compatible diagnostic format.
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            CompileError::LexError { span, message } => Diagnostic {
                line: 0,
                col: span.start,
                end_line: 0,
                end_col: span.end,
                severity: DiagnosticSeverity::Error,
                message: message.clone(),
            },
            CompileError::ParseError { line, col, message } => Diagnostic {
                line: *line,
                col: *col,
                end_line: *line,
                end_col: *col + 1,
                severity: DiagnosticSeverity::Error,
                message: message.clone(),
            },
            CompileError::ValidationError { message } => Diagnostic {
                line: 0,
                col: 0,
                end_line: 0,
                end_col: 0,
                severity: DiagnosticSeverity::Error,
                message: message.clone(),
            },
            CompileError::CodegenError { message } => Diagnostic {
                line: 0,
                col: 0,
                end_line: 0,
                end_col: 0,
                severity: DiagnosticSeverity::Error,
                message: message.clone(),
            },
            CompileError::IoError(e) => Diagnostic {
                line: 0,
                col: 0,
                end_line: 0,
                end_col: 0,
                severity: DiagnosticSeverity::Error,
                message: e.to_string(),
            },
        }
    }

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
