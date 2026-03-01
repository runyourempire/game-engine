// GAME Compiler — Recursive Descent Parser
//
// Transforms a token stream into an AST. Hand-written for precise error
// messages and straightforward recovery.

use crate::ast::*;
use crate::error::CompileError;
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

    // -- expect helpers ----------------------------------------------------

    fn expect(&mut self, expected: &Token) -> Result<Token, CompileError> {
        let (line, col) = self.current_pos();
        match self.advance() {
            Some(tok) if std::mem::discriminant(&tok) == std::mem::discriminant(expected) => Ok(tok),
            Some(tok) => Err(CompileError::ParseError {
                message: format!("expected `{expected}`, found `{tok}`"),
                line,
                col,
            }),
            None => Err(CompileError::ParseError {
                message: format!("expected `{expected}`, found end of input"),
                line,
                col,
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
            }),
            None => Err(CompileError::ParseError {
                message: "expected identifier, found end of input".into(),
                line,
                col,
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
            }),
            None => Err(CompileError::ParseError {
                message: "expected string literal, found end of input".into(),
                line,
                col,
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
            }),
            None => Err(CompileError::ParseError {
                message: "expected number, found end of input".into(),
                line,
                col,
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

    // ======================================================================
    // Top-level: program
    // ======================================================================

    pub fn parse(&mut self) -> Result<Program, CompileError> {
        let mut imports = Vec::new();
        let mut cinematics = Vec::new();

        while !self.at_end() {
            match self.peek() {
                Some(Token::Import) => match self.parse_import() {
                    Ok(imp) => imports.push(imp),
                    Err(e) => { self.skip_to_recovery(); return Err(e); }
                },
                Some(Token::Cinematic) => match self.parse_cinematic() {
                    Ok(cin) => cinematics.push(cin),
                    Err(e) => { self.skip_to_recovery(); return Err(e); }
                },
                Some(_) => {
                    let (line, col) = self.current_pos();
                    let tok = self.advance();
                    return Err(CompileError::ParseError {
                        message: format!(
                            "expected `import` or `cinematic` at top level, found `{}`",
                            tok.map_or("EOF".into(), |t| t.to_string())
                        ),
                        line,
                        col,
                    });
                }
                None => break,
            }
        }

        Ok(Program { imports, cinematics })
    }

    // ======================================================================
    // import "path" as name
    // ======================================================================

    fn parse_import(&mut self) -> Result<Import, CompileError> {
        self.expect(&Token::Import)?;
        let path = self.expect_string()?;
        self.expect(&Token::As)?;
        let alias = self.expect_ident()?;
        Ok(Import { path, alias })
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

        while !self.at_end() && !self.check(&Token::RBrace) {
            match self.peek() {
                Some(Token::Layer) => layers.push(self.parse_layer()?),
                Some(Token::Arc) => arcs.push(self.parse_arc()?),
                Some(Token::Resonate) => resonates.push(self.parse_resonate()?),
                _ => {
                    let (line, col) = self.current_pos();
                    return Err(CompileError::ParseError {
                        message: format!(
                            "expected `layer`, `arc`, or `resonate` inside cinematic, found `{}`",
                            self.peek().map_or("EOF".into(), |t| t.to_string())
                        ),
                        line,
                        col,
                    });
                }
            }
        }

        self.expect(&Token::RBrace)?;
        Ok(Cinematic { name, layers, arcs, resonates })
    }

    // ======================================================================
    // layer ident [(opts)] { body }
    // ======================================================================

    fn parse_layer(&mut self) -> Result<Layer, CompileError> {
        self.expect(&Token::Layer)?;
        let name = self.expect_ident()?;

        // optional layer-level params: (key: val, ...)
        let opts = if self.check(&Token::LParen) {
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
        let body = self.parse_layer_body()?;
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
            params.push(Param { name, value, modulation: None });
            if !self.check(&Token::RParen) {
                self.expect(&Token::Comma)?;
            }
        }
        self.expect(&Token::RParen)?;
        Ok(params)
    }

    // -- layer body: params OR pipe stages ---------------------------------

    fn parse_layer_body(&mut self) -> Result<LayerBody, CompileError> {
        // Decide by lookahead: IDENT COLON => params, IDENT LPAREN => stages
        if self.at_end() || self.check(&Token::RBrace) {
            return Ok(LayerBody::Params(Vec::new()));
        }

        match (self.tokens.get(self.pos), self.tokens.get(self.pos + 1)) {
            (Some((Token::Ident(_), _, _)), Some((Token::Colon, _, _))) => {
                self.parse_param_list()
            }
            (Some((Token::Ident(_), _, _)), Some((Token::LParen, _, _))) => {
                self.parse_stage_pipeline()
            }
            _ => {
                // Could be a single-token expression param or error --
                // try params first, fall back to error.
                self.parse_param_list()
            }
        }
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

            params.push(Param { name, value, modulation });
        }
        Ok(LayerBody::Params(params))
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
            entries.push(self.parse_arc_entry()?);
        }
        self.expect(&Token::RBrace)?;
        Ok(ArcBlock { entries })
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
                })
            }
            Some(Token::Integer(v)) => {
                Err(CompileError::ParseError {
                    message: format!("expected duration (e.g. 2s, 500ms, 4bars), found bare number {v}"),
                    line,
                    col,
                })
            }
            Some(tok) => Err(CompileError::ParseError {
                message: format!("expected duration, found `{tok}`"),
                line,
                col,
            }),
            None => Err(CompileError::ParseError {
                message: "expected duration, found end of input".into(),
                line,
                col,
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
    // Expressions — precedence climbing
    // ======================================================================

    pub fn parse_expr(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.parse_term()?;
        while matches!(self.peek(), Some(Token::Plus) | Some(Token::Minus)) {
            let op = match self.advance() {
                Some(Token::Plus) => BinOp::Add,
                Some(Token::Minus) => BinOp::Sub,
                _ => unreachable!(), // guarded by matches! above
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
            }),
            None => Err(CompileError::ParseError {
                message: "unexpected end of input in expression".into(),
                line,
                col,
            }),
        }
    }
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;