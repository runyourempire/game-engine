use std::collections::HashMap;
use std::sync::Mutex;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use super::completion;
use super::docs;

/// The LSP backend for .game files.
pub struct GameBackend {
    client: Client,
    /// In-memory document store: URI -> source text.
    documents: Mutex<HashMap<Url, String>>,
}

impl GameBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Mutex::new(HashMap::new()),
        }
    }

    /// Run diagnostics on the document and publish them.
    async fn publish_diagnostics(&self, uri: Url, source: &str) {
        let diagnostics = compute_diagnostics(source);
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

/// Compute LSP diagnostics from a .game source string.
///
/// This is a free function so it can be tested without a Client/transport.
pub fn compute_diagnostics(source: &str) -> Vec<Diagnostic> {
    let (output, errors) = crate::compile_with_diagnostics(source);

    let mut diagnostics = Vec::new();

    // Convert errors to diagnostics
    for err in &errors {
        diagnostics.push(game_error_to_diagnostic(
            err,
            source,
            DiagnosticSeverity::ERROR,
        ));
    }

    // Convert warnings to diagnostics
    if let Some(out) = &output {
        for warning in &out.warnings {
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 0,
                    },
                },
                severity: Some(DiagnosticSeverity::WARNING),
                source: Some("game".to_string()),
                message: warning.to_string(),
                ..Default::default()
            });
        }
    }

    diagnostics
}

/// Convert a GameError to an LSP Diagnostic.
fn game_error_to_diagnostic(
    err: &crate::error::GameError,
    source: &str,
    severity: DiagnosticSeverity,
) -> Diagnostic {
    let range = match &err.span {
        Some(span) => byte_span_to_range(source, span.start, span.end),
        None => Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        },
    };

    Diagnostic {
        range,
        severity: Some(severity),
        source: Some("game".to_string()),
        message: format!("{}", err),
        ..Default::default()
    }
}

/// Convert a byte offset span to an LSP Range (line/character).
fn byte_span_to_range(source: &str, start: usize, end: usize) -> Range {
    let start_pos = byte_offset_to_position(source, start);
    let end_pos = byte_offset_to_position(source, end);
    Range {
        start: start_pos,
        end: end_pos,
    }
}

/// Convert a byte offset to an LSP Position (line/character).
pub fn byte_offset_to_position(source: &str, offset: usize) -> Position {
    let offset = offset.min(source.len());
    let before = &source[..offset];
    let line = before.chars().filter(|c| *c == '\n').count();
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let character = before[line_start..].chars().count();
    Position {
        line: line as u32,
        character: character as u32,
    }
}

/// Extract the word at a given position in the source text.
/// Returns the word string, or None if the cursor is not on a word character.
pub fn word_at_position(source: &str, position: Position) -> Option<String> {
    // Find the byte offset of the start of the target line
    let mut line_start_byte = 0usize;
    let mut current_line = 0u32;
    for (i, ch) in source.char_indices() {
        if current_line == position.line {
            line_start_byte = i;
            break;
        }
        if ch == '\n' {
            current_line += 1;
        }
    }
    // Handle: if the source ended before reaching the target line
    if current_line < position.line {
        return None;
    }
    // Special case: target line is 0 and source is non-empty
    if position.line == 0 {
        line_start_byte = 0;
    }

    // Walk along the line to find the byte offset at the given character column
    let line_bytes = &source[line_start_byte..];
    let mut char_col = 0u32;
    let mut cursor_byte = line_start_byte;
    let mut found_cursor = false;
    for (i, ch) in line_bytes.char_indices() {
        if ch == '\n' {
            break;
        }
        if char_col == position.character {
            cursor_byte = line_start_byte + i;
            found_cursor = true;
            break;
        }
        char_col += 1;
    }
    // If character is at the end of line
    if !found_cursor {
        return None;
    }

    // Ensure the cursor is on a word character
    if cursor_byte >= source.len() {
        return None;
    }
    let cursor_char = source[cursor_byte..].chars().next()?;
    if !is_word_char(cursor_char) {
        return None;
    }

    // Expand backwards to find the start of the word
    let mut word_start = cursor_byte;
    for ch in source[..cursor_byte].chars().rev() {
        if is_word_char(ch) {
            word_start -= ch.len_utf8();
        } else {
            break;
        }
    }

    // Expand forwards to find the end of the word
    let mut word_end = cursor_byte;
    for (i, ch) in source[cursor_byte..].char_indices() {
        if is_word_char(ch) {
            word_end = cursor_byte + i + ch.len_utf8();
        } else {
            break;
        }
    }

    if word_start < word_end {
        Some(source[word_start..word_end].to_string())
    } else {
        None
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[tower_lsp::async_trait]
impl LanguageServer for GameBackend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec!["|".to_string(), ".".to_string()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "GAME LSP server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        {
            let mut docs = self.documents.lock().unwrap_or_else(|e| e.into_inner());
            docs.insert(uri.clone(), text.clone());
        }
        self.publish_diagnostics(uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        // We use FULL sync, so the last content change is the entire document.
        if let Some(change) = params.content_changes.into_iter().last() {
            let text = change.text;
            {
                let mut docs = self.documents.lock().unwrap_or_else(|e| e.into_inner());
                docs.insert(uri.clone(), text.clone());
            }
            self.publish_diagnostics(uri, &text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        {
            let mut docs = self.documents.lock().unwrap_or_else(|e| e.into_inner());
            docs.remove(&uri);
        }
        // Clear diagnostics for the closed document
        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let source = {
            let docs = self.documents.lock().unwrap_or_else(|e| e.into_inner());
            match docs.get(uri) {
                Some(s) => s.clone(),
                None => return Ok(None),
            }
        };

        let word = match word_at_position(&source, position) {
            Some(w) => w,
            None => return Ok(None),
        };

        // Try stage docs first, then builtins, then colors, then keywords
        let doc = docs::get_stage_docs(&word)
            .or_else(|| docs::get_builtin_docs(&word))
            .or_else(|| docs::get_color_docs(&word))
            .or_else(|| docs::get_keyword_docs(&word));

        Ok(doc.map(|content| Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: content.to_string(),
            }),
            range: None,
        }))
    }

    async fn completion(&self, _params: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::Array(
            completion::all_completions(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── byte_offset_to_position tests ───────────────────────────────

    #[test]
    fn position_first_line_start() {
        let source = "cinematic \"Hello\" {\n  layer {\n  }\n}";
        let pos = byte_offset_to_position(source, 0);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn position_first_line_middle() {
        let source = "cinematic \"Hello\" {\n  layer {\n  }\n}";
        let pos = byte_offset_to_position(source, 10);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 10);
    }

    #[test]
    fn position_second_line() {
        // "cinematic \"Hello\" {\n" is 20 bytes. "  layer" starts at 20.
        let source = "cinematic \"Hello\" {\n  layer {\n  }\n}";
        let pos = byte_offset_to_position(source, 22);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 2);
    }

    #[test]
    fn position_clamps_beyond_eof() {
        let source = "hello";
        let pos = byte_offset_to_position(source, 999);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 5);
    }

    // ── word_at_position tests ──────────────────────────────────────

    #[test]
    fn word_at_circle() {
        let source = "fn: circle(0.3) | glow(2.0)";
        let word = word_at_position(source, Position { line: 0, character: 6 });
        assert_eq!(word, Some("circle".to_string()));
    }

    #[test]
    fn word_at_glow() {
        let source = "fn: circle(0.3) | glow(2.0)";
        let word = word_at_position(source, Position { line: 0, character: 19 });
        assert_eq!(word, Some("glow".to_string()));
    }

    #[test]
    fn word_at_operator_returns_none() {
        let source = "fn: circle(0.3) | glow(2.0)";
        let word = word_at_position(source, Position { line: 0, character: 16 });
        assert_eq!(word, None);
    }

    #[test]
    fn word_at_multiline() {
        let source = "cinematic \"Hello\" {\n  layer {\n    fn: box(0.5)\n  }\n}";
        let word = word_at_position(source, Position { line: 2, character: 8 });
        assert_eq!(word, Some("box".to_string()));
    }

    // ── compute_diagnostics tests ───────────────────────────────────

    #[test]
    fn diagnostics_for_unknown_stage() {
        let source = "cinematic { layer { fn: unknown_thing(1.0) } }";
        let diags = compute_diagnostics(source);
        assert!(
            !diags.is_empty(),
            "should produce diagnostics for unknown stage"
        );
        assert!(
            diags
                .iter()
                .any(|d| d.severity == Some(DiagnosticSeverity::ERROR)),
            "should have at least one error"
        );
    }

    #[test]
    fn diagnostics_for_valid_source() {
        let source = r#"cinematic "Test" { layer { fn: circle(0.3) | glow(2.0) } }"#;
        let diags = compute_diagnostics(source);
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
            .collect();
        assert!(
            errors.is_empty(),
            "valid source should not produce errors: {:?}",
            errors
        );
    }

    #[test]
    fn diagnostics_for_warnings() {
        // glow before any SDF shape should produce a warning
        let source = r#"cinematic "Test" { layer { fn: glow(2.0) } }"#;
        let diags = compute_diagnostics(source);
        let warnings: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::WARNING))
            .collect();
        assert!(
            !warnings.is_empty(),
            "should produce a warning for glow before SDF"
        );
    }
}
