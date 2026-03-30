use thiserror::Error;

/// Byte-offset span within source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

// ── Rich diagnostics ────────────────────────────────────────

/// Severity level for a diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

/// A structured diagnostic carrying rich error information.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Option<Span>,
    pub suggestion: Option<String>,
    pub help: Option<String>,
}

impl Diagnostic {
    /// Create a new error-level diagnostic.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            span: None,
            suggestion: None,
            help: None,
        }
    }

    /// Create a new warning-level diagnostic.
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            span: None,
            suggestion: None,
            help: None,
        }
    }

    /// Create a new note-level diagnostic.
    pub fn note(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Note,
            message: message.into(),
            span: None,
            suggestion: None,
            help: None,
        }
    }

    /// Attach a source span.
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Attach a suggestion (e.g., corrected code).
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Attach a help message.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

// ── Error codes ────────────────────────────────────────────────

/// Structured error codes for diagnostics.
///
/// Each variant maps to a specific class of compilation error, enabling
/// tooling (IDEs, CI) to match on codes programmatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// Unknown stage function (e.g., `circl` instead of `circle`).
    E001,
    /// Type mismatch in pipeline (e.g., `glow` receiving Position instead of Sdf).
    E002,
    /// Parse error — unexpected token or missing syntax element.
    E003,
    /// Expected identifier but found something else.
    E004,
    /// Circular import detected.
    E005,
    /// Unknown parameter name in a stage call.
    E006,
    /// Too many arguments supplied to a stage function.
    E007,
    /// Invalid define body (e.g., empty or malformed pipeline).
    E008,
    /// Unused parameter in a define block.
    E009,
    /// Duplicate layer name within a cinematic.
    E010,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::E001 => write!(f, "E001"),
            Self::E002 => write!(f, "E002"),
            Self::E003 => write!(f, "E003"),
            Self::E004 => write!(f, "E004"),
            Self::E005 => write!(f, "E005"),
            Self::E006 => write!(f, "E006"),
            Self::E007 => write!(f, "E007"),
            Self::E008 => write!(f, "E008"),
            Self::E009 => write!(f, "E009"),
            Self::E010 => write!(f, "E010"),
        }
    }
}

// ── Levenshtein distance & suggestions ──────────────────────

/// Compute Levenshtein edit distance between two strings.
pub(crate) fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut matrix = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in 0..=a.len() {
        matrix[i][0] = i;
    }
    for j in 0..=b.len() {
        matrix[0][j] = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }
    matrix[a.len()][b.len()]
}

/// Find the closest match to `typed` from `candidates` (max edit distance 2).
pub fn suggest_similar<'a>(typed: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let mut best: Option<(&str, usize)> = None;
    for &candidate in candidates {
        let dist = levenshtein(typed, candidate);
        if dist <= 2 && dist < typed.len() {
            match best {
                None => best = Some((candidate, dist)),
                Some((_, d)) if dist < d => best = Some((candidate, dist)),
                _ => {}
            }
        }
    }
    best.map(|(s, _)| s)
}

// ── CompileError ────────────────────────────────────────────

/// All errors produced during compilation.
#[derive(Debug, Error)]
pub enum CompileError {
    #[error("lex error at {span:?}: {message}")]
    LexError {
        span: Span,
        message: String,
        code: Option<ErrorCode>,
    },

    #[error("parse error at {line}:{col}: {message}")]
    ParseError {
        line: usize,
        col: usize,
        message: String,
        code: Option<ErrorCode>,
    },

    #[error("validation error: {message}")]
    ValidationError {
        message: String,
        code: Option<ErrorCode>,
    },

    #[error("codegen error: {message}")]
    CodegenError {
        message: String,
        code: Option<ErrorCode>,
    },

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    /// Wraps any variant with additional help text.
    #[error("{inner}\n  help: {help}")]
    WithHelp {
        inner: Box<CompileError>,
        help: String,
    },
}

impl CompileError {
    pub fn lex(start: usize, end: usize, msg: impl Into<String>) -> Self {
        Self::LexError {
            span: Span { start, end },
            message: msg.into(),
            code: None,
        }
    }

    pub fn parse(line: usize, col: usize, msg: impl Into<String>) -> Self {
        Self::ParseError {
            line,
            col,
            message: msg.into(),
            code: Some(ErrorCode::E003),
        }
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        Self::ValidationError {
            message: msg.into(),
            code: None,
        }
    }

    pub fn codegen(msg: impl Into<String>) -> Self {
        Self::CodegenError {
            message: msg.into(),
            code: None,
        }
    }

    /// Set the error code on this error variant.
    pub fn with_code(mut self, error_code: ErrorCode) -> Self {
        match &mut self {
            Self::LexError { code, .. } => *code = Some(error_code),
            Self::ParseError { code, .. } => *code = Some(error_code),
            Self::ValidationError { code, .. } => *code = Some(error_code),
            Self::CodegenError { code, .. } => *code = Some(error_code),
            Self::IoError(_) | Self::WithHelp { .. } => {} // no code slot
        }
        self
    }

    /// Retrieve the error code, if any.
    pub fn code(&self) -> Option<ErrorCode> {
        match self {
            Self::LexError { code, .. } => *code,
            Self::ParseError { code, .. } => *code,
            Self::ValidationError { code, .. } => *code,
            Self::CodegenError { code, .. } => *code,
            Self::WithHelp { inner, .. } => inner.code(),
            Self::IoError(_) => None,
        }
    }

    /// Attach a help message to any error variant.
    pub fn with_help(self, help: impl Into<String>) -> Self {
        Self::WithHelp {
            inner: Box::new(self),
            help: help.into(),
        }
    }

    /// Retrieve the help text, if any.
    pub fn help(&self) -> Option<&str> {
        match self {
            Self::WithHelp { help, .. } => Some(help.as_str()),
            _ => None,
        }
    }

    /// Unwrap through `WithHelp` to get the underlying error variant.
    pub fn inner(&self) -> &CompileError {
        match self {
            Self::WithHelp { inner, .. } => inner.inner(),
            other => other,
        }
    }
}

/// Render an error with source context: source line and caret underline.
/// Also renders help text and suggestions when attached via `with_help`.
pub fn render_with_source(error: &CompileError, source: &str) -> String {
    let mut out = if let Some(code) = error.code() {
        format!("error[{code}]: {error}")
    } else {
        format!("error: {error}")
    };

    // Unwrap through WithHelp to get the underlying error for span rendering
    let inner = error.inner();

    // For LexError, use the span to find the source line
    if let CompileError::LexError { span, .. } = inner {
        if span.start <= source.len() {
            let line_num = source[..span.start].chars().filter(|c| *c == '\n').count() + 1;
            let line_start = source[..span.start].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = source[span.start..]
                .find('\n')
                .map(|i| span.start + i)
                .unwrap_or(source.len());
            let line = &source[line_start..line_end];
            let col = span.start - line_start;
            let underline_len = (span.end - span.start).max(1).min(line.len().saturating_sub(col));

            out.push_str(&format!("\n --> line {line_num}:{col}"));
            out.push_str(&format!("\n  {line_num} | {line}"));
            out.push_str(&format!(
                "\n  {} | {}{}",
                " ".repeat(line_num.to_string().len()),
                " ".repeat(col),
                "^".repeat(underline_len),
            ));
        }
    }

    // Append help text if present
    if let Some(help) = error.help() {
        out.push_str(&format!("\n  help: {help}"));
    }

    out
}

/// Render a `Diagnostic` with optional source context.
pub fn render_diagnostic(diag: &Diagnostic, source: &str) -> String {
    let prefix = match diag.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Note => "note",
    };
    let mut out = format!("{prefix}: {}", diag.message);

    // If we have a span, render the source context
    if let Some(ref span) = diag.span {
        if span.start <= source.len() {
            let line_num = source[..span.start].chars().filter(|c| *c == '\n').count() + 1;
            let line_start = source[..span.start].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = source[span.start..]
                .find('\n')
                .map(|i| span.start + i)
                .unwrap_or(source.len());
            let line = &source[line_start..line_end];
            let col = span.start - line_start;
            let underline_len = (span.end - span.start).max(1).min(line.len().saturating_sub(col));

            out.push_str(&format!("\n --> line {line_num}:{col}"));
            out.push_str(&format!("\n  {line_num} | {line}"));
            out.push_str(&format!(
                "\n  {} | {}{}",
                " ".repeat(line_num.to_string().len()),
                " ".repeat(col),
                "^".repeat(underline_len),
            ));
        }
    }

    // Append suggestion
    if let Some(ref suggestion) = diag.suggestion {
        out.push_str(&format!("\n  suggestion: {suggestion}"));
    }

    // Append help
    if let Some(ref help) = diag.help {
        out.push_str(&format!("\n  help: {help}"));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_lex_error_with_source() {
        let err = CompileError::lex(6, 7, "unexpected character: '@'");
        let source = "layer @bad {}";
        let rendered = render_with_source(&err, source);
        assert!(rendered.contains("line 1:6"));
        assert!(rendered.contains("^"));
    }

    #[test]
    fn levenshtein_basic() {
        assert_eq!(levenshtein("circle", "circle"), 0);
        assert_eq!(levenshtein("cicle", "circle"), 1);
        assert_eq!(levenshtein("cirle", "circle"), 1);
        assert_eq!(levenshtein("glow", "blow"), 1);
        assert_eq!(levenshtein("abc", "xyz"), 3);
    }

    #[test]
    fn suggest_similar_finds_close_match() {
        let candidates = &["circle", "ring", "star", "glow", "tint"];
        assert_eq!(suggest_similar("cicle", candidates), Some("circle"));
        assert_eq!(suggest_similar("circl", candidates), Some("circle"));
        assert_eq!(suggest_similar("glo", candidates), Some("glow"));
        assert_eq!(suggest_similar("tnt", candidates), Some("tint"));
    }

    #[test]
    fn suggest_similar_returns_none_for_distant() {
        let candidates = &["circle", "ring", "star"];
        assert_eq!(suggest_similar("xxxxxxxxx", candidates), None);
    }

    #[test]
    fn with_help_preserves_message() {
        let err = CompileError::validation("unknown function 'cicle'")
            .with_help("did you mean 'circle'?");
        let rendered = format!("{err}");
        assert!(rendered.contains("unknown function 'cicle'"));
    }

    #[test]
    fn with_help_accessible() {
        let err = CompileError::validation("bad")
            .with_help("try this instead");
        assert_eq!(err.help(), Some("try this instead"));
    }

    #[test]
    fn inner_unwraps_through_with_help() {
        let err = CompileError::validation("inner msg")
            .with_help("some help");
        match err.inner() {
            CompileError::ValidationError { message, .. } => {
                assert_eq!(message, "inner msg");
            }
            _ => panic!("expected ValidationError"),
        }
    }

    #[test]
    fn render_with_source_includes_help() {
        let err = CompileError::lex(0, 1, "bad char")
            .with_help("did you mean something else?");
        let source = "x";
        let rendered = render_with_source(&err, source);
        assert!(rendered.contains("help: did you mean something else?"));
    }

    #[test]
    fn diagnostic_error_renders() {
        let diag = Diagnostic::error("something went wrong")
            .with_span(Span { start: 0, end: 3 })
            .with_help("check your syntax");
        let source = "abc def";
        let rendered = render_diagnostic(&diag, source);
        assert!(rendered.contains("error: something went wrong"));
        assert!(rendered.contains("^^^"));
        assert!(rendered.contains("help: check your syntax"));
    }

    #[test]
    fn diagnostic_warning_renders() {
        let diag = Diagnostic::warning("unused variable")
            .with_suggestion("remove it or prefix with _");
        let rendered = render_diagnostic(&diag, "");
        assert!(rendered.contains("warning: unused variable"));
        assert!(rendered.contains("suggestion: remove it or prefix with _"));
    }

    #[test]
    fn error_code_display() {
        assert_eq!(format!("{}", ErrorCode::E001), "E001");
        assert_eq!(format!("{}", ErrorCode::E010), "E010");
    }

    #[test]
    fn with_code_sets_code() {
        let err = CompileError::validation("bad function")
            .with_code(ErrorCode::E001);
        assert_eq!(err.code(), Some(ErrorCode::E001));
    }

    #[test]
    fn parse_error_has_default_code() {
        let err = CompileError::parse(1, 0, "unexpected token");
        assert_eq!(err.code(), Some(ErrorCode::E003));
    }

    #[test]
    fn code_survives_with_help() {
        let err = CompileError::validation("unknown function 'cicle'")
            .with_code(ErrorCode::E001)
            .with_help("did you mean 'circle'?");
        assert_eq!(err.code(), Some(ErrorCode::E001));
    }

    #[test]
    fn render_with_source_includes_error_code() {
        let err = CompileError::validation("unknown stage function: 'cicle'")
            .with_code(ErrorCode::E001);
        let rendered = render_with_source(&err, "");
        assert!(rendered.starts_with("error[E001]:"), "should include error code: {rendered}");
    }

    #[test]
    fn render_without_code_has_no_brackets() {
        let err = CompileError::validation("something bad");
        let rendered = render_with_source(&err, "");
        assert!(rendered.starts_with("error: "), "should not have brackets: {rendered}");
    }
}
