//! Language Server Protocol implementation for the GAME language.
//!
//! Provides completion, hover, diagnostics, and go-to-definition
//! via stdio JSON-RPC transport using `lsp-server` + `lsp-types`.

use std::collections::HashMap;

use lsp_server::{Connection, ExtractError, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    notification::{DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument},
    request::{Completion, GotoDefinition, HoverRequest},
    CompletionItem, CompletionItemKind, CompletionOptions, CompletionParams, CompletionResponse,
    Diagnostic, DiagnosticSeverity, GotoDefinitionParams, GotoDefinitionResponse, Hover,
    HoverContents, HoverParams, HoverProviderCapability, InitializeParams, InsertTextFormat,
    Location, MarkupContent, MarkupKind, OneOf, Position, PublishDiagnosticsParams, Range,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
};

use crate::builtins::{self, ShaderState};

// ── Entry point ─────────────────────────────────────────────

/// Run the LSP server on stdin/stdout. Blocks until the client disconnects.
pub fn run_lsp() {
    eprintln!("[game-lsp] starting GAME language server");

    let (connection, io_threads) = Connection::stdio();

    let server_capabilities =
        serde_json::to_value(make_server_capabilities()).expect("serialize server capabilities");

    let init_params = match connection.initialize(server_capabilities) {
        Ok(params) => params,
        Err(e) => {
            eprintln!("[game-lsp] initialization failed: {e}");
            return;
        }
    };

    let _init_params: InitializeParams =
        serde_json::from_value(init_params).unwrap_or_else(|_| InitializeParams::default());

    eprintln!("[game-lsp] initialized, entering main loop");

    if let Err(e) = main_loop(&connection) {
        eprintln!("[game-lsp] main loop error: {e}");
    }

    io_threads.join().ok();
    eprintln!("[game-lsp] server shut down");
}

fn make_server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec!["|".to_string(), "(".to_string(), " ".to_string()]),
            resolve_provider: Some(false),
            ..Default::default()
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        ..Default::default()
    }
}

// ── Main loop ───────────────────────────────────────────────

/// Document store: maps URI -> full source text.
struct State {
    documents: HashMap<Uri, String>,
}

impl State {
    fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }
}

fn main_loop(connection: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = State::new();

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                handle_request(&connection, &state, req);
            }
            Message::Notification(not) => {
                handle_notification(&connection, &mut state, not);
            }
            Message::Response(_) => {
                // We don't send requests, so we don't expect responses.
            }
        }
    }

    Ok(())
}

// ── Request dispatch ────────────────────────────────────────

fn handle_request(connection: &Connection, state: &State, req: Request) {
    // Completion
    if let Some((id, params)) = cast_request::<Completion>(req.clone()) {
        let result = handle_completion(state, params);
        let resp = Response::new_ok(id, result);
        connection.sender.send(Message::Response(resp)).ok();
        return;
    }

    // Hover
    if let Some((id, params)) = cast_request::<HoverRequest>(req.clone()) {
        let result = handle_hover(state, params);
        let resp = Response::new_ok(id, result);
        connection.sender.send(Message::Response(resp)).ok();
        return;
    }

    // Go to definition
    if let Some((id, params)) = cast_request::<GotoDefinition>(req.clone()) {
        let result = handle_definition(state, params);
        let resp = Response::new_ok(id, result);
        connection.sender.send(Message::Response(resp)).ok();
        return;
    }

    // Unknown request — respond with method not found
    let resp = Response::new_err(
        req.id,
        lsp_server::ErrorCode::MethodNotFound as i32,
        format!("unhandled method: {}", req.method),
    );
    connection.sender.send(Message::Response(resp)).ok();
}

fn cast_request<R>(req: Request) -> Option<(RequestId, R::Params)>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    match req.extract::<R::Params>(R::METHOD) {
        Ok(pair) => Some(pair),
        Err(ExtractError::MethodMismatch(_)) => None,
        Err(ExtractError::JsonError { .. }) => {
            eprintln!("[game-lsp] failed to deserialize {} params", R::METHOD);
            None
        }
    }
}

// ── Notification dispatch ───────────────────────────────────

fn handle_notification(connection: &Connection, state: &mut State, not: Notification) {
    // didOpen
    if let Some(params) = cast_notification::<DidOpenTextDocument>(not.clone()) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        state.documents.insert(uri.clone(), text);
        publish_diagnostics(connection, &uri, state);
        return;
    }

    // didChange (full sync)
    if let Some(params) = cast_notification::<DidChangeTextDocument>(not.clone()) {
        let uri = params.text_document.uri.clone();
        if let Some(change) = params.content_changes.into_iter().last() {
            state.documents.insert(uri.clone(), change.text);
            publish_diagnostics(connection, &uri, state);
        }
        return;
    }

    // didClose
    if let Some(params) = cast_notification::<DidCloseTextDocument>(not.clone()) {
        state.documents.remove(&params.text_document.uri);
    }
}

fn cast_notification<N>(not: Notification) -> Option<N::Params>
where
    N: lsp_types::notification::Notification,
    N::Params: serde::de::DeserializeOwned,
{
    if not.method == N::METHOD {
        serde_json::from_value(not.params).ok()
    } else {
        None
    }
}

// ── Diagnostics ─────────────────────────────────────────────

fn publish_diagnostics(connection: &Connection, uri: &Uri, state: &State) {
    let diagnostics = match state.documents.get(uri) {
        Some(source) => compute_diagnostics(source),
        None => vec![],
    };

    let params = PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics,
        version: None,
    };

    let not = Notification::new(
        "textDocument/publishDiagnostics".to_string(),
        serde_json::to_value(params).unwrap(),
    );

    connection.sender.send(Message::Notification(not)).ok();
}

fn compute_diagnostics(source: &str) -> Vec<Diagnostic> {
    match crate::compile_to_ast(source) {
        Ok(_) => vec![], // No errors — clear diagnostics
        Err(e) => {
            let d = e.to_diagnostic();
            vec![Diagnostic {
                range: Range {
                    start: Position {
                        line: d.line as u32,
                        character: d.col as u32,
                    },
                    end: Position {
                        line: d.end_line as u32,
                        character: d.end_col as u32,
                    },
                },
                severity: Some(match d.severity {
                    crate::error::DiagnosticSeverity::Error => DiagnosticSeverity::ERROR,
                    crate::error::DiagnosticSeverity::Warning => DiagnosticSeverity::WARNING,
                    crate::error::DiagnosticSeverity::Info => DiagnosticSeverity::INFORMATION,
                    crate::error::DiagnosticSeverity::Hint => DiagnosticSeverity::HINT,
                }),
                source: Some("game".to_string()),
                message: d.message,
                ..Default::default()
            }]
        }
    }
}

// ── Completion ──────────────────────────────────────────────

fn handle_completion(state: &State, params: CompletionParams) -> Option<CompletionResponse> {
    let uri = &params.text_document_position.text_document.uri;
    let pos = params.text_document_position.position;
    let source = state.documents.get(uri)?;

    let pipeline_state = detect_pipeline_state(source, pos);
    let mut items = Vec::new();

    // 1. Context-aware builtin completions based on pipeline state
    let valid_names = builtins::valid_next_stages(pipeline_state);
    let all_completions = builtins::completions();

    for comp in &all_completions {
        if !valid_names.contains(&comp.name.as_str()) {
            continue;
        }

        // Build snippet insert text: name(${1:param1}, ${2:param2})
        let insert_text = build_snippet(&comp.name);

        items.push(CompletionItem {
            label: comp.name.clone(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(comp.signature.clone()),
            documentation: Some(lsp_types::Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!(
                    "**{}**\n\n`{} -> {}`",
                    comp.signature, comp.input, comp.output
                ),
            })),
            insert_text: Some(insert_text),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }

    // 2. Palette name completions when inside palette(...)
    if is_inside_palette_call(source, pos) {
        for (name, description) in palette_completions() {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                detail: Some(description.to_string()),
                documentation: Some(lsp_types::Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("**palette({name})**\n\n{description}"),
                })),
                insert_text: Some(name.to_string()),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }
    }

    // 3. User-defined function completions from the current file
    for (name, params) in find_user_functions(source) {
        let param_list = params.join(", ");
        let snippet = if params.is_empty() {
            format!("{name}()")
        } else {
            let snippet_params: Vec<String> = params
                .iter()
                .enumerate()
                .map(|(i, p)| format!("${{{}: {}}}", i + 1, p))
                .collect();
            format!("{}({})", name, snippet_params.join(", "))
        };

        items.push(CompletionItem {
            label: name.clone(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(format!("fn {}({})", name, param_list)),
            documentation: Some(lsp_types::Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("User-defined function `{}`", name),
            })),
            insert_text: Some(snippet),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }

    Some(CompletionResponse::Array(items))
}

/// Build a snippet string with tabstops for a builtin's default parameters.
fn build_snippet(name: &str) -> String {
    match builtins::lookup(name) {
        Some(builtin) => {
            if builtin.params.is_empty() {
                return format!("{name}()");
            }
            let params: Vec<String> = builtin
                .params
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let default = p.default.map(|d| format!("{d}")).unwrap_or_default();
                    format!("${{{}:{}}}", i + 1, default)
                })
                .collect();
            format!("{}({})", name, params.join(", "))
        }
        None => format!("{name}()"),
    }
}

/// Detect the current pipeline state by scanning backwards from the cursor.
///
/// Walks backwards through `|`-separated stages, resolving each stage name
/// via the builtins registry to determine the output state. The output state
/// of the last resolved stage becomes the completion context.
///
/// Defaults to `Position` (start of pipeline) when nothing is found.
fn detect_pipeline_state(source: &str, pos: Position) -> ShaderState {
    let lines: Vec<&str> = source.lines().collect();
    let line_idx = pos.line as usize;
    if line_idx >= lines.len() {
        return ShaderState::Position;
    }

    // Collect text from the current line up to the cursor position
    let col = pos.character as usize;
    let current_line = lines[line_idx];
    let text_before_cursor = if col < current_line.len() {
        &current_line[..col]
    } else {
        current_line
    };

    // Gather text from earlier lines in the same block (walk up until we find
    // a line with '{' or reach a blank / non-pipeline line)
    let mut pipeline_text = String::new();
    for i in (0..line_idx).rev() {
        let l = lines[i].trim();
        if l.is_empty()
            || l.ends_with('{')
            || l.starts_with("layer ")
            || l.starts_with("cinematic ")
            || l.starts_with("fn ")
            || l.starts_with("pass ")
            || l == "}"
        {
            break;
        }
        pipeline_text = format!("{}\n{}", l, pipeline_text);
    }
    pipeline_text.push_str(text_before_cursor);

    // Split by pipe and resolve each stage
    let mut state = ShaderState::Position;
    for segment in pipeline_text.split('|') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        // Extract function name (everything before '(' or end of segment)
        let name = segment
            .split(|c: char| c == '(' || c.is_whitespace())
            .next()
            .unwrap_or("")
            .trim();

        if name.is_empty() {
            continue;
        }

        if let Some(builtin) = builtins::lookup(name) {
            state = builtin.output;
        }
        // If it's a user-defined fn, we can't know its output state,
        // so keep the current state (best-effort).
    }

    state
}

/// Check whether the cursor is inside the parentheses of a `palette(...)` call.
///
/// Scans backwards from the cursor to find the nearest unmatched `(`, then
/// checks if the identifier immediately preceding it is `palette`.
fn is_inside_palette_call(source: &str, pos: Position) -> bool {
    let lines: Vec<&str> = source.lines().collect();
    let line_idx = pos.line as usize;
    if line_idx >= lines.len() {
        return false;
    }

    let line = lines[line_idx];
    let col = (pos.character as usize).min(line.len());
    let before = &line[..col];

    // Walk backwards counting parens to find the nearest unmatched '('
    let mut depth: i32 = 0;
    for (i, ch) in before.char_indices().rev() {
        match ch {
            ')' => depth += 1,
            '(' => {
                if depth == 0 {
                    // Found unmatched open-paren — check if preceded by "palette"
                    let prefix = before[..i].trim_end();
                    return prefix.ends_with("palette");
                }
                depth -= 1;
            }
            _ => {}
        }
    }

    false
}

/// All 30 named palettes with short descriptions.
fn palette_completions() -> &'static [(&'static str, &'static str)] {
    &[
        ("fire", "warm reds/oranges"),
        ("ocean", "cool blues"),
        ("neon", "vibrant rainbow"),
        ("aurora", "green/purple northern lights"),
        ("sunset", "warm orange/pink horizon"),
        ("ice", "cool blue/white"),
        ("ember", "deep red/orange coals"),
        ("lava", "volcanic orange/red"),
        ("magma", "bright orange to deep red"),
        ("inferno", "yellow-white to deep red"),
        ("plasma", "purple/pink energy"),
        ("electric", "bright cyan/blue"),
        ("cyber", "neon green/cyan"),
        ("matrix", "green-on-black terminal"),
        ("forest", "deep greens/warm browns"),
        ("moss", "muted greens/earth"),
        ("earth", "brown/tan/olive"),
        ("desert", "warm sand/terracotta"),
        ("blood", "dark to bright red"),
        ("rose", "pink/rose/magenta"),
        ("candy", "bright pink/purple/blue"),
        ("royal", "deep purple/gold"),
        ("deep_sea", "dark blue/cyan"),
        ("coral", "warm coral/orange/pink"),
        ("arctic", "white/light blue cold"),
        ("twilight", "purple/orange horizon"),
        ("vapor", "vaporwave purple/pink/teal"),
        ("gold", "warm gold/amber"),
        ("silver", "cool gray/white"),
        ("monochrome", "grayscale"),
    ]
}

// ── Hover ───────────────────────────────────────────────────

fn handle_hover(state: &State, params: HoverParams) -> Option<Hover> {
    let uri = &params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let source = state.documents.get(uri)?;

    let word = word_at_position(source, pos)?;

    // Look up in builtins
    let builtin = builtins::lookup(&word)?;

    let params_doc: Vec<String> = builtin
        .params
        .iter()
        .map(|p| {
            if let Some(d) = p.default {
                format!("- `{}` (default: `{}`)", p.name, d)
            } else {
                format!("- `{}` (required)", p.name)
            }
        })
        .collect();

    let params_section = if params_doc.is_empty() {
        String::from("*No parameters*")
    } else {
        format!("**Parameters:**\n{}", params_doc.join("\n"))
    };

    // Build signature
    let param_names: Vec<String> = builtin
        .params
        .iter()
        .map(|p| {
            if let Some(d) = p.default {
                format!("{}={}", p.name, d)
            } else {
                p.name.to_string()
            }
        })
        .collect();
    let signature = format!("{}({})", builtin.name, param_names.join(", "));

    let markdown = format!(
        "### `{}`\n\n```\n{:?} -> {:?}\n```\n\n```game\n{}\n```\n\n{}",
        builtin.name, builtin.input, builtin.output, signature, params_section
    );

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: None,
    })
}

/// Extract the word under the cursor position.
fn word_at_position(source: &str, pos: Position) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    let line_idx = pos.line as usize;
    if line_idx >= lines.len() {
        return None;
    }

    let line = lines[line_idx];
    let col = pos.character as usize;
    if col > line.len() {
        return None;
    }

    let bytes = line.as_bytes();

    // Walk backwards to find word start
    let mut start = col;
    while start > 0 && is_ident_char(bytes[start - 1]) {
        start -= 1;
    }

    // Walk forwards to find word end
    let mut end = col;
    while end < bytes.len() && is_ident_char(bytes[end]) {
        end += 1;
    }

    if start == end {
        return None;
    }

    Some(line[start..end].to_string())
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

// ── Go to definition ────────────────────────────────────────

fn handle_definition(
    state: &State,
    params: GotoDefinitionParams,
) -> Option<GotoDefinitionResponse> {
    let uri = &params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let source = state.documents.get(uri)?;

    let word = word_at_position(source, pos)?;

    // Skip builtins — they have no source location
    if builtins::lookup(&word).is_some() {
        return None;
    }

    // Search for `fn <word>` definition in the source
    let fn_pos = find_fn_definition(source, &word)?;

    Some(GotoDefinitionResponse::Scalar(Location {
        uri: uri.clone(),
        range: Range {
            start: fn_pos,
            end: Position {
                line: fn_pos.line,
                character: fn_pos.character + word.len() as u32,
            },
        },
    }))
}

/// Find the position of `fn <name>` in the source text.
fn find_fn_definition(source: &str, name: &str) -> Option<Position> {
    let target = format!("fn {name}");
    for (line_idx, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(&target) {
            // Verify it's actually `fn name(` and not `fn name_extended(`
            let rest = &trimmed[target.len()..];
            if rest.starts_with('(') || rest.starts_with(' ') || rest.is_empty() {
                // Calculate the column of the name (after "fn ")
                let indent = line.len() - trimmed.len();
                let col = indent + 3; // "fn " is 3 chars
                return Some(Position {
                    line: line_idx as u32,
                    character: col as u32,
                });
            }
        }
    }
    None
}

// ── User function discovery ─────────────────────────────────

/// Find all user-defined `fn` blocks in the source, returning (name, params).
fn find_user_functions(source: &str) -> Vec<(String, Vec<String>)> {
    let mut results = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("fn ") {
            continue;
        }

        let after_fn = trimmed[3..].trim();

        // Extract name (up to '(' or whitespace)
        let name_end = after_fn
            .find(|c: char| c == '(' || c.is_whitespace())
            .unwrap_or(after_fn.len());
        let name = &after_fn[..name_end];

        if name.is_empty() {
            continue;
        }

        // Extract params if we have parentheses
        let params = if let Some(paren_start) = after_fn.find('(') {
            if let Some(paren_end) = after_fn[paren_start..].find(')') {
                let param_str = &after_fn[paren_start + 1..paren_start + paren_end];
                param_str
                    .split(',')
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        results.push((name.to_string(), params));
    }

    results
}

// ── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_state_empty_returns_position() {
        let state = detect_pipeline_state(
            "",
            Position {
                line: 0,
                character: 0,
            },
        );
        assert_eq!(state, ShaderState::Position);
    }

    #[test]
    fn detect_state_after_circle() {
        let source = "circle(0.3) | ";
        let state = detect_pipeline_state(
            source,
            Position {
                line: 0,
                character: 14,
            },
        );
        assert_eq!(state, ShaderState::Sdf);
    }

    #[test]
    fn detect_state_after_glow() {
        let source = "circle(0.3) | glow(2.0) | ";
        let state = detect_pipeline_state(
            source,
            Position {
                line: 0,
                character: 26,
            },
        );
        assert_eq!(state, ShaderState::Color);
    }

    #[test]
    fn detect_state_after_transform() {
        let source = "rotate(1.0) | ";
        let state = detect_pipeline_state(
            source,
            Position {
                line: 0,
                character: 14,
            },
        );
        assert_eq!(state, ShaderState::Position);
    }

    #[test]
    fn word_at_position_finds_word() {
        let source = "circle(0.3) | glow(2.0)";
        let word = word_at_position(
            source,
            Position {
                line: 0,
                character: 3,
            },
        );
        assert_eq!(word, Some("circle".to_string()));
    }

    #[test]
    fn word_at_position_finds_second_word() {
        let source = "circle(0.3) | glow(2.0)";
        let word = word_at_position(
            source,
            Position {
                line: 0,
                character: 16,
            },
        );
        assert_eq!(word, Some("glow".to_string()));
    }

    #[test]
    fn find_fn_def_basic() {
        let source = "fn myEffect(intensity) {\n  circle(0.3) | glow(intensity)\n}";
        let pos = find_fn_definition(source, "myEffect");
        assert_eq!(
            pos,
            Some(Position {
                line: 0,
                character: 3
            })
        );
    }

    #[test]
    fn find_fn_def_not_found() {
        let source = "fn myEffect(intensity) {\n  circle(0.3)\n}";
        let pos = find_fn_definition(source, "nonexistent");
        assert!(pos.is_none());
    }

    #[test]
    fn find_fn_def_no_false_prefix() {
        let source = "fn myEffect(a) {}\nfn myEffectExtra(b) {}";
        let pos = find_fn_definition(source, "myEffect");
        // Should find the first one, not confuse with myEffectExtra
        assert_eq!(
            pos,
            Some(Position {
                line: 0,
                character: 3
            })
        );
    }

    #[test]
    fn find_user_functions_basic() {
        let source = "fn glow_pulse(speed, intensity) {\n  circle(0.3) | glow(2.0)\n}\n\nfn shimmer() {\n  ring(0.3) | glow(1.0)\n}";
        let fns = find_user_functions(source);
        assert_eq!(fns.len(), 2);
        assert_eq!(fns[0].0, "glow_pulse");
        assert_eq!(fns[0].1, vec!["speed", "intensity"]);
        assert_eq!(fns[1].0, "shimmer");
        assert!(fns[1].1.is_empty());
    }

    #[test]
    fn build_snippet_with_defaults() {
        let snippet = build_snippet("circle");
        assert_eq!(snippet, "circle(${1:0.2})");
    }

    #[test]
    fn build_snippet_no_params() {
        let snippet = build_snippet("polar");
        assert_eq!(snippet, "polar()");
    }

    #[test]
    fn build_snippet_multi_params() {
        let snippet = build_snippet("tint");
        assert_eq!(snippet, "tint(${1:1}, ${2:1}, ${3:1})");
    }

    #[test]
    fn diagnostics_valid_source() {
        let source = r#"cinematic "test" {
  layer main {
    circle(0.3) | glow(2.0)
  }
}"#;
        let diags = compute_diagnostics(source);
        assert!(diags.is_empty());
    }

    #[test]
    fn diagnostics_invalid_source() {
        let diags = compute_diagnostics("this is not valid game code {{{{");
        assert!(!diags.is_empty());
        assert_eq!(diags[0].severity, Some(DiagnosticSeverity::ERROR));
    }

    #[test]
    fn inside_palette_call_basic() {
        let source = "circle(0.3) | palette(";
        assert!(is_inside_palette_call(
            source,
            Position {
                line: 0,
                character: 22
            }
        ));
    }

    #[test]
    fn inside_palette_call_partial_name() {
        let source = "circle(0.3) | palette(fi";
        assert!(is_inside_palette_call(
            source,
            Position {
                line: 0,
                character: 24
            }
        ));
    }

    #[test]
    fn not_inside_palette_call_other_fn() {
        let source = "circle(0.3) | glow(";
        assert!(!is_inside_palette_call(
            source,
            Position {
                line: 0,
                character: 19
            }
        ));
    }

    #[test]
    fn not_inside_palette_call_outside_parens() {
        let source = "circle(0.3) | palette";
        assert!(!is_inside_palette_call(
            source,
            Position {
                line: 0,
                character: 21
            }
        ));
    }

    #[test]
    fn inside_palette_call_nested_parens() {
        // Cursor after a closed inner paren should not match
        let source = "circle(0.3) | palette(fire) | glow(";
        assert!(!is_inside_palette_call(
            source,
            Position {
                line: 0,
                character: 35
            }
        ));
    }

    #[test]
    fn palette_completions_has_30_entries() {
        assert_eq!(palette_completions().len(), 30);
    }

    #[test]
    fn palette_completions_contains_fire() {
        let palettes = palette_completions();
        assert!(palettes.iter().any(|(name, _)| *name == "fire"));
    }

    #[test]
    fn palette_completions_contains_monochrome() {
        let palettes = palette_completions();
        assert!(palettes.iter().any(|(name, _)| *name == "monochrome"));
    }
}
