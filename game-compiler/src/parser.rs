// GAME Compiler — Recursive Descent Parser
//
// Transforms a token stream into an AST. Hand-written for precise error
// messages and straightforward recovery.

use crate::ast::*;
use crate::error::{CompileError, ErrorCode};
use crate::token::Token;

// ---------------------------------------------------------------------------
// Parser core
// ---------------------------------------------------------------------------

pub struct Parser {
    tokens: Vec<(Token, usize, usize)>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<(Token, usize, usize)>) -> Self {
        Self { tokens, pos: 0 }
    }

    // -- navigation helpers ------------------------------------------------

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|(t, _, _)| t)
    }

    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].0.clone();
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn current_pos(&self) -> (usize, usize) {
        if self.pos < self.tokens.len() {
            let (_, s, e) = &self.tokens[self.pos];
            (*s, *e)
        } else if let Some((_, s, e)) = self.tokens.last() {
            (*s, *e)
        } else {
            (0, 0)
        }
    }

    fn check(&self, expected: &Token) -> bool {
        self.peek().map_or(false, |t| std::mem::discriminant(t) == std::mem::discriminant(expected))
    }

    /// Returns `true` if the next token is an identifier that is NOT a
    /// score-block keyword (`motif`, `phrase`, `section`, `arrange`).
    /// Used to stop greedy reference consumption inside score parsing.
    fn is_score_ref_ident(&self) -> bool {
        match self.peek() {
            Some(Token::Ident(s)) => {
                !matches!(s.as_str(), "motif" | "phrase" | "section" | "arrange")
            }
            _ => false,
        }
    }

    // -- expect helpers ----------------------------------------------------

    fn expect(&mut self, expected: &Token) -> Result<Token, CompileError> {
        let (line, col) = self.current_pos();
        match self.advance() {
            Some(tok) if std::mem::discriminant(&tok) == std::mem::discriminant(expected) => Ok(tok),
            Some(tok) => Err(CompileError::ParseError {
                message: format!("expected `{expected}`, found `{tok}`"),
                line,
                col,
                code: Some(ErrorCode::E003),
            }),
            None => Err(CompileError::ParseError {
                message: format!("expected `{expected}`, found end of input"),
                line,
                col,
                code: Some(ErrorCode::E003),
            }),
        }
    }

    fn expect_ident(&mut self) -> Result<String, CompileError> {
        let (line, col) = self.current_pos();
        match self.advance() {
            Some(Token::Ident(s)) => Ok(s),
            Some(tok) => Err(CompileError::ParseError {
                message: format!("expected identifier, found `{tok}`"),
                line,
                col,
                code: Some(ErrorCode::E003),
            }),
            None => Err(CompileError::ParseError {
                message: "expected identifier, found end of input".into(),
                line,
                col,
                code: Some(ErrorCode::E003),
            }),
        }
    }

    /// Like expect_ident but also accepts the `ALL` keyword.
    fn expect_ident_or_all(&mut self) -> Result<String, CompileError> {
        let (line, col) = self.current_pos();
        match self.advance() {
            Some(Token::Ident(s)) => Ok(s),
            Some(Token::All) => Ok("ALL".into()),
            Some(tok) => Err(CompileError::ParseError {
                message: format!("expected identifier, found `{tok}`"),
                line,
                col,
                code: Some(ErrorCode::E003),
            }),
            None => Err(CompileError::ParseError {
                message: "expected identifier, found end of input".into(),
                line,
                col,
                code: Some(ErrorCode::E003),
            }),
        }
    }

    fn expect_string(&mut self) -> Result<String, CompileError> {
        let (line, col) = self.current_pos();
        match self.advance() {
            Some(Token::StringLit(s)) => Ok(s),
            Some(tok) => Err(CompileError::ParseError {
                message: format!("expected string literal, found `{tok}`"),
                line,
                col,
                code: Some(ErrorCode::E003),
            }),
            None => Err(CompileError::ParseError {
                message: "expected string literal, found end of input".into(),
                line,
                col,
                code: Some(ErrorCode::E003),
            }),
        }
    }

    fn expect_number(&mut self) -> Result<f64, CompileError> {
        let (line, col) = self.current_pos();
        match self.advance() {
            Some(Token::Float(v)) => Ok(v),
            Some(Token::Integer(v)) => Ok(v as f64),
            Some(tok) => Err(CompileError::ParseError {
                message: format!("expected number, found `{tok}`"),
                line,
                col,
                code: Some(ErrorCode::E003),
            }),
            None => Err(CompileError::ParseError {
                message: "expected number, found end of input".into(),
                line,
                col,
                code: Some(ErrorCode::E003),
            }),
        }
    }

    // -- error recovery ----------------------------------------------------

    fn skip_to_recovery(&mut self) {
        let mut depth = 0i32;
        while let Some(tok) = self.peek() {
            match tok {
                Token::LBrace => { depth += 1; self.advance(); }
                Token::RBrace if depth > 0 => { depth -= 1; self.advance(); }
                Token::RBrace => { self.advance(); return; }
                _ => { self.advance(); }
            }
        }
    }

    /// Skip a keyword followed by a `{ ... }` block (including nested braces).
    /// Consumes the keyword token, the opening `{`, all contents, and the closing `}`.
    fn skip_brace_block(&mut self) -> Result<(), CompileError> {
        self.advance(); // consume the keyword (e.g. `signals`)
        self.expect(&Token::LBrace)?;
        let mut depth = 1u32;
        while depth > 0 {
            match self.advance() {
                Some(Token::LBrace) => depth += 1,
                Some(Token::RBrace) => depth -= 1,
                Some(_) => {}
                None => {
                    let (line, col) = self.current_pos();
                    return Err(CompileError::ParseError {
                        message: "unexpected end of input while skipping block".into(),
                        line,
                        col,
                        code: Some(ErrorCode::E003),
                    });
                }
            }
        }
        Ok(())
    }

    /// Skip to the matching closing `}` without requiring a leading keyword
    /// or opening brace. Assumes we are already inside a `{ ... }` block
    /// (depth starts at 1). Used for error recovery within cinematic blocks.
    fn skip_brace_block_safe(&mut self) {
        let mut depth = 1i32;
        while depth > 0 {
            match self.peek() {
                Some(Token::LBrace) => { depth += 1; self.advance(); }
                Some(Token::RBrace) => { depth -= 1; self.advance(); }
                None => break,
                _ => { self.advance(); }
            }
        }
    }

    // ======================================================================
    // Top-level: program
    // ======================================================================

    pub fn parse(&mut self) -> Result<Program, CompileError> {
        let mut imports = Vec::new();
        let mut cinematics = Vec::new();
        let mut breeds = Vec::new();
        let mut projects = Vec::new();
        let mut errors: Vec<CompileError> = Vec::new();

        while !self.at_end() {
            match self.peek() {
                Some(Token::Import) => match self.parse_import() {
                    Ok(imp) => imports.push(imp),
                    Err(e) => {
                        errors.push(e);
                        self.skip_to_recovery();
                    }
                },
                Some(Token::Cinematic) => match self.parse_cinematic() {
                    Ok(cin) => cinematics.push(cin),
                    Err(e) => {
                        errors.push(e);
                        self.skip_to_recovery();
                    }
                },
                Some(Token::Breed) => match self.parse_breed() {
                    Ok(b) => breeds.push(b),
                    Err(e) => {
                        errors.push(e);
                        self.skip_to_recovery();
                    }
                },
                Some(Token::Project) => match self.parse_project() {
                    Ok(p) => projects.push(p),
                    Err(e) => {
                        errors.push(e);
                        self.skip_to_recovery();
                    }
                },
                Some(_) => {
                    let (line, col) = self.current_pos();
                    let tok = self.advance();
                    errors.push(CompileError::ParseError {
                        message: format!(
                            "expected `import`, `cinematic`, `breed`, or `project` at top level, found `{}`",
                            tok.map_or("EOF".into(), |t| t.to_string())
                        ),
                        line,
                        col,
                        code: Some(ErrorCode::E003),
                    });
                    self.skip_to_recovery();
                }
                None => break,
            }
        }

        if errors.is_empty() {
            Ok(Program { imports, cinematics, breeds, projects })
        } else {
            // Return first error for backward compatibility
            // (future: return all errors via a batch mechanism)
            Err(errors.remove(0))
        }
    }

    // ======================================================================
    // import "path" as name
    // ======================================================================

    fn parse_import(&mut self) -> Result<Import, CompileError> {
        self.expect(&Token::Import)?;
        let path = self.expect_string()?;

        // Two styles: `import "path" as alias` or `import "path" expose a, b`
        if matches!(self.peek(), Some(Token::Expose)) {
            self.advance();
            let mut names = vec![self.expect_ident_or_all()?];
            while matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
                names.push(self.expect_ident_or_all()?);
            }
            Ok(Import { path, alias: String::new(), exposed: names })
        } else {
            self.expect(&Token::As)?;
            let alias = self.expect_ident()?;
            Ok(Import { path, alias, exposed: Vec::new() })
        }
    }

    // ======================================================================
    // cinematic "name" { ... }
    // ======================================================================

    fn parse_cinematic(&mut self) -> Result<Cinematic, CompileError> {
        self.expect(&Token::Cinematic)?;
        let name = self.expect_string()?;
        self.expect(&Token::LBrace)?;

        let mut layers = Vec::new();
        let mut arcs = Vec::new();
        let mut resonates = Vec::new();
        let mut listen = None;
        let mut voice = None;
        let mut score = None;
        let mut gravity = None;
        let mut lenses = Vec::new();
        let mut react = None;
        let mut defines = Vec::new();
        let mut errors: Vec<CompileError> = Vec::new();

        while !self.at_end() && !self.check(&Token::RBrace) {
            match self.peek() {
                Some(Token::Layer) => match self.parse_layer() {
                    Ok(layer) => layers.push(layer),
                    Err(e) => {
                        errors.push(e);
                        self.skip_brace_block_safe();
                    }
                },
                Some(Token::Arc) => match self.parse_arc() {
                    Ok(a) => arcs.push(a),
                    Err(e) => {
                        errors.push(e);
                        self.skip_brace_block_safe();
                    }
                },
                Some(Token::Resonate) => match self.parse_resonate() {
                    Ok(r) => resonates.push(r),
                    Err(e) => {
                        errors.push(e);
                        self.skip_brace_block_safe();
                    }
                },
                Some(Token::Listen) => match self.parse_listen() {
                    Ok(l) => listen = Some(l),
                    Err(e) => {
                        errors.push(e);
                        self.skip_brace_block_safe();
                    }
                },
                Some(Token::Voice) => match self.parse_voice() {
                    Ok(v) => voice = Some(v),
                    Err(e) => {
                        errors.push(e);
                        self.skip_brace_block_safe();
                    }
                },
                Some(Token::Score) => match self.parse_score() {
                    Ok(s) => score = Some(s),
                    Err(e) => {
                        errors.push(e);
                        self.skip_brace_block_safe();
                    }
                },
                Some(Token::Gravity) => match self.parse_gravity() {
                    Ok(g) => gravity = Some(g),
                    Err(e) => {
                        errors.push(e);
                        self.skip_brace_block_safe();
                    }
                },
                Some(Token::Lens) => match self.parse_lens() {
                    Ok(l) => lenses.push(l),
                    Err(e) => {
                        errors.push(e);
                        self.skip_brace_block_safe();
                    }
                },
                Some(Token::React) => match self.parse_react() {
                    Ok(r) => react = Some(r),
                    Err(e) => {
                        errors.push(e);
                        self.skip_brace_block_safe();
                    }
                },
                Some(Token::Define) => match self.parse_define() {
                    Ok(d) => defines.push(d),
                    Err(e) => {
                        errors.push(e);
                        self.skip_brace_block_safe();
                    }
                },
                Some(Token::Signals) | Some(Token::Route) | Some(Token::Hear) | Some(Token::Feel) => {
                    if let Err(e) = self.skip_brace_block() {
                        errors.push(e);
                    }
                }
                _ => {
                    let (line, col) = self.current_pos();
                    let tok_str = self.peek().map_or("EOF".into(), |t| t.to_string());
                    errors.push(CompileError::ParseError {
                        message: format!(
                            "unexpected `{}` inside cinematic block",
                            tok_str
                        ),
                        line,
                        col,
                        code: Some(ErrorCode::E003),
                    });
                    // Advance past the unexpected token to avoid infinite loop
                    self.advance();
                }
            }
        }

        self.expect(&Token::RBrace)?;

        if !errors.is_empty() {
            return Err(errors.remove(0));
        }

        Ok(Cinematic {
            name, layers, arcs, resonates, listen, voice, score, gravity,
            lenses, react, defines,
        })
    }

    // ======================================================================
    // layer ident [(opts)] { body }
    // ======================================================================

    fn parse_layer(&mut self) -> Result<Layer, CompileError> {
        self.expect(&Token::Layer)?;

        // Optional layer name: `layer myname { ... }` or `layer { ... }`
        let name = if matches!(self.peek(), Some(Token::Ident(_))) {
            self.expect_ident()?
        } else {
            format!("_layer_{}", self.pos)
        };

        // optional layer-level params: (key: val, ...)
        let mut opts = if self.check(&Token::LParen) {
            self.parse_layer_opts()?
        } else {
            Vec::new()
        };

        // Phase-1: optional `memory : <float>`
        let memory = if matches!(self.peek(), Some(Token::Memory)) {
            self.advance(); // consume `memory`
            self.expect(&Token::Colon)?;
            Some(self.expect_number()?)
        } else {
            None
        };

        // Phase-1: optional `cast <ident>`
        let cast = if matches!(self.peek(), Some(Token::Cast)) {
            self.advance(); // consume `cast`
            Some(self.expect_ident()?)
        } else {
            None
        };

        self.expect(&Token::LBrace)?;
        let (body, inline_params) = self.parse_layer_body()?;
        opts.extend(inline_params);
        self.expect(&Token::RBrace)?;

        Ok(Layer { name, opts, memory, cast, body })
    }

    fn parse_layer_opts(&mut self) -> Result<Vec<Param>, CompileError> {
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while !self.at_end() && !self.check(&Token::RParen) {
            let name = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let value = self.parse_expr()?;
            params.push(Param { name, value, modulation: None, temporal_ops: vec![] });
            if !self.check(&Token::RParen) {
                self.expect(&Token::Comma)?;
            }
        }
        self.expect(&Token::RParen)?;
        Ok(params)
    }

    // -- layer body: params, pipe stages, or fn: mixed ----------------------

    fn parse_layer_body(&mut self) -> Result<(LayerBody, Vec<Param>), CompileError> {
        // Empty body
        if self.at_end() || self.check(&Token::RBrace) {
            return Ok((LayerBody::Params(Vec::new()), Vec::new()));
        }

        // Check for `fn:` mixed body (pipeline + inline params)
        if let Some((Token::Ident(name), _, _)) = self.tokens.get(self.pos) {
            if name == "fn" {
                if let Some((Token::Colon, _, _)) = self.tokens.get(self.pos + 1) {
                    return self.parse_fn_mixed_body();
                }
            }
        }

        match (self.tokens.get(self.pos), self.tokens.get(self.pos + 1)) {
            (Some((Token::Ident(_), _, _)), Some((Token::Colon, _, _))) => {
                let body = self.parse_param_list()?;
                Ok((body, Vec::new()))
            }
            (Some((Token::Ident(_), _, _)), Some((Token::LParen, _, _))) => {
                let body = self.parse_stage_pipeline()?;
                Ok((body, Vec::new()))
            }
            _ => {
                let body = self.parse_param_list()?;
                Ok((body, Vec::new()))
            }
        }
    }

    /// Parse `fn: stage() | stage() \n param: value ~ mod` mixed body.
    /// Returns pipeline as LayerBody and inline params separately.
    fn parse_fn_mixed_body(&mut self) -> Result<(LayerBody, Vec<Param>), CompileError> {
        // Consume `fn` and `:`
        self.advance(); // fn
        self.advance(); // :

        // Parse the pipeline stages
        let body = self.parse_stage_pipeline()?;

        // Parse remaining entries as inline params
        let mut params = Vec::new();
        while !self.at_end() && !self.check(&Token::RBrace) {
            let name = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let value = self.parse_expr()?;

            let modulation = if matches!(self.peek(), Some(Token::Tilde)) {
                self.advance();
                Some(self.parse_expr()?)
            } else {
                None
            };

            let temporal_ops = self.parse_temporal_ops()?;

            params.push(Param { name, value, modulation, temporal_ops });
        }

        Ok((body, params))
    }

    fn parse_param_list(&mut self) -> Result<LayerBody, CompileError> {
        let mut params = Vec::new();
        while !self.at_end() && !self.check(&Token::RBrace) {
            let name = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let value = self.parse_expr()?;

            let modulation = if matches!(self.peek(), Some(Token::Tilde)) {
                self.advance();
                Some(self.parse_expr()?)
            } else {
                None
            };

            let temporal_ops = self.parse_temporal_ops()?;

            params.push(Param { name, value, modulation, temporal_ops });
        }
        Ok(LayerBody::Params(params))
    }

    /// Parse a chain of temporal operators: `>> 0.5s <> 100ms .. [0.0, 1.0]`
    fn parse_temporal_ops(&mut self) -> Result<Vec<TemporalOp>, CompileError> {
        let mut ops = Vec::new();
        loop {
            match self.peek() {
                Some(Token::ShiftRight) => {
                    self.advance();
                    let dur = self.parse_duration()?;
                    ops.push(TemporalOp::Delay(dur));
                }
                Some(Token::Diamond) => {
                    self.advance();
                    let dur = self.parse_duration()?;
                    ops.push(TemporalOp::Smooth(dur));
                }
                Some(Token::BangBang) => {
                    self.advance();
                    let dur = self.parse_duration()?;
                    ops.push(TemporalOp::Trigger(dur));
                }
                Some(Token::DotDot) => {
                    self.advance();
                    self.expect(&Token::LBracket)?;
                    let min = self.parse_expr()?;
                    self.expect(&Token::Comma)?;
                    let max = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    ops.push(TemporalOp::Range(min, max));
                }
                _ => break,
            }
        }
        Ok(ops)
    }

    fn parse_stage_pipeline(&mut self) -> Result<LayerBody, CompileError> {
        let mut stages = Vec::new();
        stages.push(self.parse_stage()?);
        while matches!(self.peek(), Some(Token::Pipe)) {
            self.advance();
            stages.push(self.parse_stage()?);
        }
        Ok(LayerBody::Pipeline(stages))
    }

    pub fn parse_stage(&mut self) -> Result<Stage, CompileError> {
        let name = self.expect_ident()?;
        self.expect(&Token::LParen)?;
        let args = self.parse_arg_list()?;
        self.expect(&Token::RParen)?;
        Ok(Stage { name, args })
    }

    fn parse_arg_list(&mut self) -> Result<Vec<Arg>, CompileError> {
        let mut args = Vec::new();
        if self.check(&Token::RParen) {
            return Ok(args);
        }
        args.push(self.parse_arg()?);
        while matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            args.push(self.parse_arg()?);
        }
        Ok(args)
    }

    fn parse_arg(&mut self) -> Result<Arg, CompileError> {
        // Named arg: IDENT COLON expr  or  positional: expr
        // Lookahead for IDENT ':'
        if let (Some((Token::Ident(_), _, _)), Some((Token::Colon, _, _))) =
            (self.tokens.get(self.pos), self.tokens.get(self.pos + 1))
        {
            let name = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let value = self.parse_expr()?;
            Ok(Arg { name: Some(name), value })
        } else {
            let value = self.parse_expr()?;
            Ok(Arg { name: None, value })
        }
    }

    // ======================================================================
    // arc { entries }
    // ======================================================================

    fn parse_arc(&mut self) -> Result<ArcBlock, CompileError> {
        self.expect(&Token::Arc)?;
        self.expect(&Token::LBrace)?;
        let mut entries = Vec::new();
        while !self.at_end() && !self.check(&Token::RBrace) {
            // Check for timestamp format: Number:Number "label" { ... }
            if self.is_timestamp_start() {
                let mut ts_entries = self.parse_arc_timestamp_entry()?;
                entries.append(&mut ts_entries);
            } else {
                entries.push(self.parse_arc_entry()?);
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(ArcBlock { entries })
    }

    /// Check if the next tokens form a timestamp pattern: Number Colon Number
    fn is_timestamp_start(&self) -> bool {
        let p = self.pos;
        matches!(
            (self.tokens.get(p), self.tokens.get(p + 1), self.tokens.get(p + 2)),
            (
                Some((Token::Integer(_) | Token::Float(_), _, _)),
                Some((Token::Colon, _, _)),
                Some((Token::Integer(_) | Token::Float(_), _, _)),
            )
        )
    }

    /// Parse a timestamp arc entry: `0:02 "label" { param: val | param -> val ease(e) over dur }`
    /// Returns one ArcEntry per parameter in the block.
    fn parse_arc_timestamp_entry(&mut self) -> Result<Vec<ArcEntry>, CompileError> {
        // Parse timestamp: minutes:seconds
        let minutes = self.expect_number()?;
        self.expect(&Token::Colon)?;
        let seconds = self.expect_number()?;
        let _time_seconds = minutes * 60.0 + seconds;

        // Parse optional label string
        let _label = if matches!(self.peek(), Some(Token::StringLit(_))) {
            Some(self.expect_string()?)
        } else {
            None
        };

        // Parse the block: { ... }
        self.expect(&Token::LBrace)?;
        let mut entries = Vec::new();
        while !self.at_end() && !self.check(&Token::RBrace) {
            let target = self.expect_ident()?;

            if matches!(self.peek(), Some(Token::Colon)) {
                // Static assignment: `param: value`
                self.advance(); // consume ':'
                let value = self.parse_expr()?;
                entries.push(ArcEntry {
                    target,
                    from: value.clone(),
                    to: value,
                    duration: Duration::Seconds(0.0),
                    easing: None,
                });
            } else if matches!(self.peek(), Some(Token::Arrow)) {
                // Transition: `param -> value [ease(name)] over duration`
                self.advance(); // consume '->'
                let to = self.parse_expr()?;

                // Optional ease(name)
                let easing = if matches!(self.peek(), Some(Token::Ease)) {
                    self.advance(); // consume 'ease'
                    self.expect(&Token::LParen)?;
                    let name = self.expect_ident()?;
                    self.expect(&Token::RParen)?;
                    Some(name)
                } else {
                    None
                };

                // `over duration`
                let duration = if matches!(self.peek(), Some(Token::Over)) {
                    self.advance(); // consume 'over'
                    self.parse_duration()?
                } else {
                    Duration::Seconds(0.0)
                };

                entries.push(ArcEntry {
                    target,
                    from: Expr::Number(0.0), // implicit from previous value
                    to,
                    duration,
                    easing,
                });
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(entries)
    }

    fn parse_arc_entry(&mut self) -> Result<ArcEntry, CompileError> {
        // dotted_ident : from_expr -> to_expr over duration [easing]
        let target = self.parse_dotted_ident()?;
        self.expect(&Token::Colon)?;
        let from = self.parse_expr()?;
        self.expect(&Token::Arrow)?;
        let to = self.parse_expr()?;
        self.expect(&Token::Over)?;
        let duration = self.parse_duration()?;
        let easing = if matches!(self.peek(), Some(Token::Ident(_))) {
            Some(self.expect_ident()?)
        } else {
            None
        };
        Ok(ArcEntry { target, from, to, duration, easing })
    }

    fn parse_dotted_ident(&mut self) -> Result<String, CompileError> {
        let mut s = self.expect_ident()?;
        while matches!(self.peek(), Some(Token::Dot)) {
            self.advance();
            let part = self.expect_ident()?;
            s.push('.');
            s.push_str(&part);
        }
        Ok(s)
    }

    fn parse_duration(&mut self) -> Result<Duration, CompileError> {
        let (line, col) = self.current_pos();
        match self.advance() {
            Some(Token::Seconds(v)) => Ok(Duration::Seconds(v)),
            Some(Token::Millis(v)) => Ok(Duration::Millis(v)),
            Some(Token::Bars(v)) => Ok(Duration::Bars(v)),
            Some(Token::Float(v)) => {
                Err(CompileError::ParseError {
                    message: format!("expected duration (e.g. 2s, 500ms, 4bars), found bare number {v}"),
                    line,
                    col,
                    code: Some(ErrorCode::E003),
                })
            }
            Some(Token::Integer(v)) => {
                Err(CompileError::ParseError {
                    message: format!("expected duration (e.g. 2s, 500ms, 4bars), found bare number {v}"),
                    line,
                    col,
                    code: Some(ErrorCode::E003),
                })
            }
            Some(tok) => Err(CompileError::ParseError {
                message: format!("expected duration, found `{tok}`"),
                line,
                col,
                code: Some(ErrorCode::E003),
            }),
            None => Err(CompileError::ParseError {
                message: "expected duration, found end of input".into(),
                line,
                col,
                code: Some(ErrorCode::E003),
            }),
        }
    }

    // ======================================================================
    // resonate { entries }
    // ======================================================================

    fn parse_resonate(&mut self) -> Result<ResonateBlock, CompileError> {
        self.expect(&Token::Resonate)?;
        self.expect(&Token::LBrace)?;
        let mut entries = Vec::new();
        while !self.at_end() && !self.check(&Token::RBrace) {
            entries.push(self.parse_resonate_entry()?);
        }
        self.expect(&Token::RBrace)?;
        Ok(ResonateBlock { entries })
    }

    fn parse_resonate_entry(&mut self) -> Result<ResonateEntry, CompileError> {
        // source -> target.field * weight
        let source = self.expect_ident()?;
        self.expect(&Token::Arrow)?;
        let target = self.expect_ident()?;
        self.expect(&Token::Dot)?;
        let field = self.expect_ident()?;
        self.expect(&Token::Star)?;
        let weight = self.parse_expr()?;
        Ok(ResonateEntry { source, target, field, weight })
    }

    // ======================================================================
    // listen { signal: algorithm(params) }
    // ======================================================================

    fn parse_listen(&mut self) -> Result<ListenBlock, CompileError> {
        self.expect(&Token::Listen)?;
        self.expect(&Token::LBrace)?;
        let mut signals = Vec::new();
        while !self.at_end() && !self.check(&Token::RBrace) {
            let name = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let algorithm = self.expect_ident()?;
            let params = if self.check(&Token::LParen) {
                self.parse_listen_params()?
            } else {
                vec![]
            };
            signals.push(ListenSignal { name, algorithm, params });
        }
        self.expect(&Token::RBrace)?;
        Ok(ListenBlock { signals })
    }

    fn parse_listen_params(&mut self) -> Result<Vec<Param>, CompileError> {
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while !self.at_end() && !self.check(&Token::RParen) {
            let name = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let value = self.parse_expr()?;
            params.push(Param { name, value, modulation: None, temporal_ops: vec![] });
            if !self.check(&Token::RParen) {
                self.expect(&Token::Comma)?;
            }
        }
        self.expect(&Token::RParen)?;
        Ok(params)
    }

    // ======================================================================
    // voice { node: kind(params) }
    // ======================================================================

    fn parse_voice(&mut self) -> Result<VoiceBlock, CompileError> {
        self.expect(&Token::Voice)?;
        self.expect(&Token::LBrace)?;
        let mut nodes = Vec::new();
        while !self.at_end() && !self.check(&Token::RBrace) {
            let name = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let kind = self.expect_ident()?;
            let params = if self.check(&Token::LParen) {
                self.parse_listen_params()?  // Reuse same param parser
            } else {
                vec![]
            };
            nodes.push(VoiceNode { name, kind, params });
        }
        self.expect(&Token::RBrace)?;
        Ok(VoiceBlock { nodes })
    }

    // ======================================================================
    // score tempo(BPM) { motifs, phrases, sections, arrange }
    // ======================================================================

    fn parse_score(&mut self) -> Result<ScoreBlock, CompileError> {
        self.expect(&Token::Score)?;

        // Parse optional tempo: `tempo(120)`
        let tempo_bpm = if matches!(self.peek(), Some(Token::Ident(s)) if s == "tempo") {
            self.advance();
            self.expect(&Token::LParen)?;
            let bpm = self.expect_number()?;
            self.expect(&Token::RParen)?;
            bpm
        } else {
            120.0
        };

        self.expect(&Token::LBrace)?;

        let mut motifs = Vec::new();
        let mut phrases = Vec::new();
        let mut sections = Vec::new();
        let mut arrange = Vec::new();

        while !self.at_end() && !self.check(&Token::RBrace) {
            match self.peek() {
                Some(Token::Ident(s)) if s == "motif" => {
                    self.advance();
                    let name = self.expect_ident()?;
                    self.expect(&Token::LBrace)?;
                    let mut entries = Vec::new();
                    while !self.at_end() && !self.check(&Token::RBrace) {
                        entries.push(self.parse_arc_entry()?);
                    }
                    self.expect(&Token::RBrace)?;
                    motifs.push(Motif { name, entries });
                }
                Some(Token::Ident(s)) if s == "phrase" => {
                    self.advance();
                    let name = self.expect_ident()?;
                    self.expect(&Token::Eq)?;
                    let mut refs = Vec::new();
                    while self.is_score_ref_ident() {
                        refs.push(self.expect_ident()?);
                        if matches!(self.peek(), Some(Token::Pipe)) {
                            self.advance();
                        }
                    }
                    phrases.push(Phrase { name, motifs: refs });
                }
                Some(Token::Ident(s)) if s == "section" => {
                    self.advance();
                    let name = self.expect_ident()?;
                    self.expect(&Token::Eq)?;
                    let mut refs = Vec::new();
                    while self.is_score_ref_ident() {
                        refs.push(self.expect_ident()?);
                    }
                    sections.push(Section { name, phrases: refs });
                }
                Some(Token::Ident(s)) if s == "arrange" => {
                    self.advance();
                    self.expect(&Token::Colon)?;
                    while self.is_score_ref_ident() {
                        arrange.push(self.expect_ident()?);
                    }
                }
                _ => {
                    let (line, col) = self.current_pos();
                    return Err(CompileError::ParseError {
                        message: format!(
                            "expected `motif`, `phrase`, `section`, or `arrange` in score, found `{}`",
                            self.peek().map_or("EOF".into(), |t| t.to_string())
                        ),
                        line,
                        col,
                        code: Some(ErrorCode::E003),
                    });
                }
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(ScoreBlock { tempo_bpm, motifs, phrases, sections, arrange })
    }

    // ======================================================================
    // breed "name" from "parent1" + "parent2" { inherit ..., mutate ... }
    // ======================================================================

    fn parse_breed(&mut self) -> Result<BreedBlock, CompileError> {
        self.expect(&Token::Breed)?;
        let name = self.expect_string()?;
        self.expect(&Token::From)?;

        // Parse parent list: "parent1" + "parent2" [+ ...]
        let mut parents = vec![self.expect_string()?];
        while matches!(self.peek(), Some(Token::Plus)) {
            self.advance(); // consume `+`
            parents.push(self.expect_string()?);
        }

        self.expect(&Token::LBrace)?;
        let mut inherit_rules = Vec::new();
        let mut mutations = Vec::new();

        while !self.at_end() && !self.check(&Token::RBrace) {
            match self.peek() {
                Some(Token::Inherit) => {
                    self.advance();
                    let target = self.expect_ident()?;
                    self.expect(&Token::Colon)?;
                    // strategy(weight) — e.g. mix(0.6) or pick(0.5)
                    let strategy = self.expect_ident()?;
                    self.expect(&Token::LParen)?;
                    let weight = self.expect_number()?;
                    self.expect(&Token::RParen)?;
                    inherit_rules.push(InheritRule { target, strategy, weight });
                }
                Some(Token::Mutate) => {
                    self.advance();
                    let target = self.expect_ident()?;
                    self.expect(&Token::Colon)?;
                    // +/-range — parse as a number (can be negative from unary minus)
                    let range = self.expect_number()?.abs();
                    mutations.push(Mutation { target, range });
                }
                _ => {
                    let (line, col) = self.current_pos();
                    return Err(CompileError::ParseError {
                        message: format!(
                            "expected `inherit` or `mutate` in breed, found `{}`",
                            self.peek().map_or("EOF".into(), |t| t.to_string())
                        ),
                        line,
                        col,
                        code: Some(ErrorCode::E003),
                    });
                }
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(BreedBlock { name, parents, inherit_rules, mutations })
    }

    // ======================================================================
    // gravity { rule: expr, damping: f, bounds: mode }
    // ======================================================================

    fn parse_gravity(&mut self) -> Result<GravityBlock, CompileError> {
        self.expect(&Token::Gravity)?;
        self.expect(&Token::LBrace)?;

        let mut force_law = Expr::Number(1.0); // default: constant attraction
        let mut damping = 0.99;
        let mut bounds = BoundsMode::Reflect;

        while !self.at_end() && !self.check(&Token::RBrace) {
            let key = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            match key.as_str() {
                "rule" => force_law = self.parse_expr()?,
                "damping" => damping = self.expect_number()?,
                "bounds" => {
                    let mode_str = self.expect_ident()?;
                    bounds = match mode_str.as_str() {
                        "reflect" => BoundsMode::Reflect,
                        "wrap" => BoundsMode::Wrap,
                        "none" => BoundsMode::None,
                        _ => {
                            let (line, col) = self.current_pos();
                            return Err(CompileError::ParseError {
                                message: format!(
                                    "unknown bounds mode `{mode_str}`, expected reflect/wrap/none"
                                ),
                                line,
                                col,
                                code: Some(ErrorCode::E003),
                            });
                        }
                    };
                }
                _ => {
                    let (line, col) = self.current_pos();
                    return Err(CompileError::ParseError {
                        message: format!(
                            "unknown gravity property `{key}`, expected rule/damping/bounds"
                        ),
                        line,
                        col,
                        code: Some(ErrorCode::E003),
                    });
                }
            }
            // optional comma separator
            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(GravityBlock { force_law, damping, bounds })
    }

    // ======================================================================
    // project mode(params) { source: name, ... }
    // ======================================================================

    fn parse_project(&mut self) -> Result<ProjectBlock, CompileError> {
        self.expect(&Token::Project)?;

        // Mode identifier: flat, dome, cube, led
        let mode_str = self.expect_ident()?;
        let mode = match mode_str.as_str() {
            "flat" => ProjectMode::Flat,
            "dome" => ProjectMode::Dome,
            "cube" => ProjectMode::Cube,
            "led" => ProjectMode::Led,
            _ => {
                let (line, col) = self.current_pos();
                return Err(CompileError::ParseError {
                    message: format!(
                        "unknown projection mode `{mode_str}`, expected flat/dome/cube/led"
                    ),
                    line,
                    col,
                    code: Some(ErrorCode::E003),
                });
            }
        };

        // Optional params in parens: (segments: 8, fisheye: 180)
        let params = if self.check(&Token::LParen) {
            self.parse_layer_opts()?
        } else {
            Vec::new()
        };

        self.expect(&Token::LBrace)?;

        // Required: source: name
        let mut source = String::new();
        while !self.at_end() && !self.check(&Token::RBrace) {
            let key = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            match key.as_str() {
                "source" => source = self.expect_ident()?,
                _ => {
                    // skip unknown keys — parse and discard the value
                    let _val = self.expect_ident()?;
                }
            }
            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            }
        }
        self.expect(&Token::RBrace)?;

        Ok(ProjectBlock { mode, source, params })
    }

    // ======================================================================
    // lens [name] { properties, post: pipeline }
    // ======================================================================

    fn parse_lens(&mut self) -> Result<Lens, CompileError> {
        self.expect(&Token::Lens)?;
        let name = if matches!(self.peek(), Some(Token::Ident(_))) {
            Some(self.expect_ident()?)
        } else {
            None
        };
        self.expect(&Token::LBrace)?;

        let mut properties = Vec::new();
        let mut post = Vec::new();

        while !self.at_end() && !self.check(&Token::RBrace) {
            // Check for `post:` which starts a pipeline
            if let Some((Token::Ident(s), _, _)) = self.tokens.get(self.pos) {
                if s == "post" {
                    if let Some((Token::Colon, _, _)) = self.tokens.get(self.pos + 1) {
                        self.advance(); // consume "post"
                        self.advance(); // consume ":"
                        // Parse pipeline stages
                        post.push(self.parse_stage()?);
                        while matches!(self.peek(), Some(Token::Pipe)) {
                            self.advance();
                            post.push(self.parse_stage()?);
                        }
                        continue;
                    }
                }
            }
            // Otherwise parse as param
            let name = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let value = self.parse_expr()?;
            properties.push(Param { name, value, modulation: None, temporal_ops: vec![] });
        }
        self.expect(&Token::RBrace)?;
        Ok(Lens { name, properties, post })
    }

    // ======================================================================
    // react { signal -> action, ... }
    // ======================================================================

    fn parse_react(&mut self) -> Result<ReactBlock, CompileError> {
        self.expect(&Token::React)?;
        self.expect(&Token::LBrace)?;
        let mut reactions = Vec::new();
        while !self.at_end() && !self.check(&Token::RBrace) {
            let signal = self.parse_expr()?;
            self.expect(&Token::Arrow)?;
            let action = self.parse_expr()?;
            reactions.push(Reaction { signal, action });
        }
        self.expect(&Token::RBrace)?;
        Ok(ReactBlock { reactions })
    }

    // ======================================================================
    // define name(params) { pipeline }
    // ======================================================================

    fn parse_define(&mut self) -> Result<DefineBlock, CompileError> {
        self.expect(&Token::Define)?;
        let name = self.expect_ident()?;

        // Parse parameter list
        let mut params = Vec::new();
        if self.check(&Token::LParen) {
            self.advance();
            while !self.at_end() && !self.check(&Token::RParen) {
                params.push(self.expect_ident()?);
                if !self.check(&Token::RParen) {
                    self.expect(&Token::Comma)?;
                }
            }
            self.expect(&Token::RParen)?;
        }

        self.expect(&Token::LBrace)?;
        let mut body = vec![self.parse_stage()?];
        while matches!(self.peek(), Some(Token::Pipe)) {
            self.advance();
            body.push(self.parse_stage()?);
        }
        self.expect(&Token::RBrace)?;

        Ok(DefineBlock { name, params, body })
    }

    // ======================================================================
    // Expressions — precedence climbing
    // ======================================================================

    pub fn parse_expr(&mut self) -> Result<Expr, CompileError> {
        let expr = self.parse_comparison()?;
        // Ternary: expr ? if_true : if_false
        if matches!(self.peek(), Some(Token::Question)) {
            self.advance();
            let if_true = self.parse_expr()?;
            self.expect(&Token::Colon)?;
            let if_false = self.parse_expr()?;
            Ok(Expr::Ternary {
                condition: Box::new(expr),
                if_true: Box::new(if_true),
                if_false: Box::new(if_false),
            })
        } else {
            Ok(expr)
        }
    }

    fn parse_comparison(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.parse_additive()?;
        while matches!(self.peek(), Some(Token::Greater) | Some(Token::Less)) {
            let op = match self.advance() {
                Some(Token::Greater) => BinOp::Gt,
                Some(Token::Less) => BinOp::Lt,
                _ => unreachable!(),
            };
            let right = self.parse_additive()?;
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.parse_term()?;
        while matches!(self.peek(), Some(Token::Plus) | Some(Token::Minus)) {
            let op = match self.advance() {
                Some(Token::Plus) => BinOp::Add,
                Some(Token::Minus) => BinOp::Sub,
                _ => unreachable!(),
            };
            let right = self.parse_term()?;
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.parse_factor()?;
        while matches!(self.peek(), Some(Token::Star) | Some(Token::Slash)) {
            let op = match self.advance() {
                Some(Token::Star) => BinOp::Mul,
                Some(Token::Slash) => BinOp::Div,
                _ => unreachable!(),
            };
            let right = self.parse_factor()?;
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr, CompileError> {
        let base = self.parse_atom()?;
        if matches!(self.peek(), Some(Token::Caret)) {
            self.advance();
            let exp = self.parse_factor()?; // right-associative
            Ok(Expr::BinOp {
                op: BinOp::Pow,
                left: Box::new(base),
                right: Box::new(exp),
            })
        } else {
            Ok(base)
        }
    }

    fn parse_atom(&mut self) -> Result<Expr, CompileError> {
        let (line, col) = self.current_pos();
        match self.peek().cloned() {
            Some(Token::Float(v)) => { self.advance(); Ok(Expr::Number(v)) }
            Some(Token::Integer(v)) => { self.advance(); Ok(Expr::Number(v as f64)) }
            Some(Token::Seconds(v)) => { self.advance(); Ok(Expr::Duration(Duration::Seconds(v))) }
            Some(Token::Millis(v)) => { self.advance(); Ok(Expr::Duration(Duration::Millis(v))) }
            Some(Token::Bars(v)) => { self.advance(); Ok(Expr::Duration(Duration::Bars(v))) }
            Some(Token::Degrees(v)) => { self.advance(); Ok(Expr::Number(v)) }
            Some(Token::StringLit(s)) => { self.advance(); Ok(Expr::String(s)) }
            Some(Token::Ident(name)) => {
                self.advance();
                // call: IDENT '(' args ')'
                if matches!(self.peek(), Some(Token::LParen)) {
                    self.advance();
                    let args = self.parse_arg_list()?;
                    self.expect(&Token::RParen)?;
                    Ok(Expr::Call { name, args })
                }
                // dotted: IDENT '.' IDENT
                else if matches!(self.peek(), Some(Token::Dot)) {
                    self.advance();
                    let field = self.expect_ident()?;
                    Ok(Expr::DottedIdent { object: name, field })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            Some(Token::LParen) => {
                self.advance();
                let inner = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(Expr::Paren(Box::new(inner)))
            }
            Some(Token::LBracket) => {
                self.advance();
                let mut elems = Vec::new();
                if !self.check(&Token::RBracket) {
                    elems.push(self.parse_expr()?);
                    while matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                        elems.push(self.parse_expr()?);
                    }
                }
                self.expect(&Token::RBracket)?;
                Ok(Expr::Array(elems))
            }
            Some(Token::Minus) => {
                self.advance();
                let inner = self.parse_factor()?;
                Ok(Expr::Neg(Box::new(inner)))
            }
            Some(tok) => Err(CompileError::ParseError {
                message: format!("unexpected token `{tok}` in expression"),
                line,
                col,
                code: Some(ErrorCode::E003),
            }),
            None => Err(CompileError::ParseError {
                message: "unexpected end of input in expression".into(),
                line,
                col,
                code: Some(ErrorCode::E003),
            }),
        }
    }
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;