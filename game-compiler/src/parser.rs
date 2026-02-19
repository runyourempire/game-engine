use crate::ast::*;
use crate::error::{GameError, Result};
use crate::token::{Spanned, Token};

/// Recursive descent parser for the `.game` language.
///
/// Grammar is LL(1) with Pratt-style precedence climbing for expressions.
pub struct Parser {
    tokens: Vec<Spanned>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Spanned>) -> Self {
        Self { tokens, pos: 0 }
    }

    // ── Helpers ────────────────────────────────────────────────────────

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|s| &s.token)
    }

    fn peek_spanned(&self) -> Option<&Spanned> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Spanned> {
        let tok = self.tokens.get(self.pos);
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<&Spanned> {
        match self.peek_spanned() {
            Some(s) if &s.token == expected => {
                self.pos += 1;
                Ok(&self.tokens[self.pos - 1])
            }
            Some(s) => Err(GameError::unexpected_token(
                expected.describe(),
                s.token.describe(),
                s.span.clone(),
            )),
            None => Err(GameError::unexpected_eof(expected.describe())),
        }
    }

    fn expect_ident(&mut self) -> Result<String> {
        match self.peek() {
            Some(Token::Ident(_)) => {
                let s = self.advance().unwrap();
                if let Token::Ident(name) = &s.token {
                    Ok(name.clone())
                } else {
                    unreachable!()
                }
            }
            Some(_) => {
                let s = self.peek_spanned().unwrap();
                Err(GameError::unexpected_token(
                    "identifier",
                    s.token.describe(),
                    s.span.clone(),
                ))
            }
            None => Err(GameError::unexpected_eof("identifier")),
        }
    }

    fn at(&self, token: &Token) -> bool {
        self.peek() == Some(token)
    }

    fn at_ident(&self) -> bool {
        matches!(self.peek(), Some(Token::Ident(_)))
    }

    fn last_span(&self) -> std::ops::Range<usize> {
        if self.pos > 0 {
            self.tokens[self.pos - 1].span.clone()
        } else {
            0..0
        }
    }

    // ── Top-level ──────────────────────────────────────────────────────

    /// Parse a complete `.game` file.
    pub fn parse(&mut self) -> Result<Cinematic> {
        self.parse_cinematic()
    }

    fn parse_cinematic(&mut self) -> Result<Cinematic> {
        self.expect(&Token::Cinematic)?;

        let name = if let Some(Token::String(_)) = self.peek() {
            let s = self.advance().unwrap();
            if let Token::String(n) = &s.token {
                Some(n.clone())
            } else {
                None
            }
        } else {
            None
        };

        self.expect(&Token::LBrace)?;

        let mut cinematic = Cinematic {
            name,
            properties: Vec::new(),
            layers: Vec::new(),
            lenses: Vec::new(),
            arc: None,
            react: None,
            resonance: None,
            defines: Vec::new(),
        };

        while !self.at(&Token::RBrace) {
            if self.peek().is_none() {
                return Err(GameError::unexpected_eof("'}'"));
            }
            match self.peek() {
                Some(Token::Layer) => {
                    cinematic.layers.push(self.parse_layer()?);
                }
                Some(Token::Lens) => {
                    cinematic.lenses.push(self.parse_lens()?);
                }
                Some(Token::Arc) => {
                    cinematic.arc = Some(self.parse_arc()?);
                }
                Some(Token::React) => {
                    cinematic.react = Some(self.parse_react()?);
                }
                Some(Token::Resonate) => {
                    cinematic.resonance = Some(self.parse_resonance()?);
                }
                Some(Token::Define) => {
                    cinematic.defines.push(self.parse_define()?);
                }
                Some(Token::Ident(_)) => {
                    cinematic.properties.push(self.parse_property()?);
                }
                _ => {
                    let s = self.peek_spanned().unwrap();
                    return Err(GameError::unexpected_token(
                        "layer, lens, arc, react, resonate, define, or property",
                        s.token.describe(),
                        s.span.clone(),
                    ));
                }
            }
        }

        self.expect(&Token::RBrace)?;
        Ok(cinematic)
    }

    // ── Layer ──────────────────────────────────────────────────────────

    fn parse_layer(&mut self) -> Result<Layer> {
        self.expect(&Token::Layer)?;

        // Optional name
        let name = if self.at_ident() {
            Some(self.expect_ident()?)
        } else {
            None
        };

        self.expect(&Token::LBrace)?;

        let mut layer = Layer {
            name,
            fn_chain: None,
            params: Vec::new(),
            properties: Vec::new(),
        };

        while !self.at(&Token::RBrace) {
            if self.peek().is_none() {
                return Err(GameError::unexpected_eof("'}'"));
            }

            // Peek at the identifier name to decide what to parse
            if self.at_ident() {
                // Look ahead: is this `fn:` (pipe chain) or `name: value` (property/param)?
                let name_str = match self.peek() {
                    Some(Token::Ident(s)) => s.clone(),
                    _ => unreachable!(),
                };

                if name_str == "fn" {
                    // fn: pipe_chain
                    self.advance(); // consume 'fn'
                    self.expect(&Token::Colon)?;
                    layer.fn_chain = Some(self.parse_pipe_chain()?);
                } else {
                    // Could be a param (with ~) or a plain property
                    let prop = self.parse_property_or_param()?;
                    match prop {
                        PropertyOrParam::Property(p) => layer.properties.push(p),
                        PropertyOrParam::Param(p) => layer.params.push(p),
                    }
                }
            } else {
                let s = self.peek_spanned().unwrap();
                return Err(GameError::unexpected_token(
                    "property or 'fn:'",
                    s.token.describe(),
                    s.span.clone(),
                ));
            }
        }

        self.expect(&Token::RBrace)?;
        Ok(layer)
    }

    // ── Lens ───────────────────────────────────────────────────────────

    fn parse_lens(&mut self) -> Result<Lens> {
        self.expect(&Token::Lens)?;

        let name = if self.at_ident() {
            Some(self.expect_ident()?)
        } else {
            None
        };

        self.expect(&Token::LBrace)?;

        let mut lens = Lens {
            name,
            properties: Vec::new(),
            post: Vec::new(),
        };

        while !self.at(&Token::RBrace) {
            if self.peek().is_none() {
                return Err(GameError::unexpected_eof("'}'"));
            }

            if self.at_ident() {
                let name_str = match self.peek() {
                    Some(Token::Ident(s)) => s.clone(),
                    _ => unreachable!(),
                };

                if name_str == "post" {
                    self.advance(); // consume 'post'
                    self.expect(&Token::Colon)?;
                    self.expect(&Token::LBracket)?;
                    while !self.at(&Token::RBracket) {
                        lens.post.push(self.parse_fn_call()?);
                        if self.at(&Token::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(&Token::RBracket)?;
                } else {
                    lens.properties.push(self.parse_property()?);
                }
            } else {
                let s = self.peek_spanned().unwrap();
                return Err(GameError::unexpected_token(
                    "property",
                    s.token.describe(),
                    s.span.clone(),
                ));
            }
        }

        self.expect(&Token::RBrace)?;
        Ok(lens)
    }

    // ── Arc ────────────────────────────────────────────────────────────

    fn parse_arc(&mut self) -> Result<ArcBlock> {
        self.expect(&Token::Arc)?;
        self.expect(&Token::LBrace)?;

        let mut moments = Vec::new();
        while !self.at(&Token::RBrace) {
            if self.peek().is_none() {
                return Err(GameError::unexpected_eof("'}'"));
            }
            moments.push(self.parse_moment()?);
        }

        self.expect(&Token::RBrace)?;
        Ok(ArcBlock { moments })
    }

    fn parse_moment(&mut self) -> Result<Moment> {
        // Timestamp: INT : INT  (minutes:seconds)
        let minutes = match self.peek() {
            Some(Token::Int(n)) => {
                let v = *n;
                self.advance();
                v
            }
            Some(_) => {
                let s = self.peek_spanned().unwrap();
                return Err(GameError::unexpected_token(
                    "timestamp (e.g. 0:00)",
                    s.token.describe(),
                    s.span.clone(),
                ));
            }
            None => return Err(GameError::unexpected_eof("timestamp")),
        };
        self.expect(&Token::Colon)?;
        let seconds = match self.peek() {
            Some(Token::Int(n)) => {
                let v = *n;
                self.advance();
                v
            }
            Some(_) => {
                let s = self.peek_spanned().unwrap();
                return Err(GameError::unexpected_token(
                    "seconds",
                    s.token.describe(),
                    s.span.clone(),
                ));
            }
            None => return Err(GameError::unexpected_eof("seconds")),
        };

        let time_seconds = (minutes * 60 + seconds) as f64;

        // Optional name string
        let name = if let Some(Token::String(_)) = self.peek() {
            let s = self.advance().unwrap();
            if let Token::String(n) = &s.token {
                Some(n.clone())
            } else {
                None
            }
        } else {
            None
        };

        self.expect(&Token::LBrace)?;

        let mut transitions = Vec::new();
        while !self.at(&Token::RBrace) {
            if self.peek().is_none() {
                return Err(GameError::unexpected_eof("'}'"));
            }
            transitions.push(self.parse_transition()?);
        }

        self.expect(&Token::RBrace)?;

        Ok(Moment {
            time_seconds,
            name,
            transitions,
        })
    }

    fn parse_transition(&mut self) -> Result<Transition> {
        // target.param : value  OR  target.param -> value ease(fn) over Ns
        let target = self.expect_ident()?;
        let full_target = if self.at(&Token::Dot) {
            self.advance();
            let field = self.expect_ident()?;
            format!("{target}.{field}")
        } else {
            target
        };

        let is_animated = self.at(&Token::Arrow);
        if is_animated {
            self.advance(); // consume ->
        } else {
            self.expect(&Token::Colon)?;
        }

        let value = self.parse_expr(0)?;

        // Optional: ease(fn)
        let easing = if self.at(&Token::Ease) {
            self.advance();
            self.expect(&Token::LParen)?;
            let name = self.expect_ident()?;
            self.expect(&Token::RParen)?;
            Some(name)
        } else {
            None
        };

        // Optional: over Ns
        let duration_secs = if self.at(&Token::Over) {
            self.advance();
            let num = self.parse_number()?;
            // Consume 's' suffix if present
            if let Some(Token::Ident(s)) = self.peek() {
                if s == "s" {
                    self.advance();
                }
            }
            Some(num)
        } else {
            None
        };

        Ok(Transition {
            target: full_target,
            value,
            is_animated,
            easing,
            duration_secs,
        })
    }

    // ── React (stub for M0) ───────────────────────────────────────────

    fn parse_react(&mut self) -> Result<ReactBlock> {
        self.expect(&Token::React)?;
        self.expect(&Token::LBrace)?;
        // For M0, skip contents
        let mut depth = 1u32;
        while depth > 0 {
            match self.peek() {
                Some(Token::LBrace) => {
                    depth += 1;
                    self.advance();
                }
                Some(Token::RBrace) => {
                    depth -= 1;
                    if depth > 0 {
                        self.advance();
                    }
                }
                Some(_) => {
                    self.advance();
                }
                None => return Err(GameError::unexpected_eof("'}'")),
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(ReactBlock {
            reactions: Vec::new(),
        })
    }

    // ── Resonate (stub for M0) ────────────────────────────────────────

    fn parse_resonance(&mut self) -> Result<ResonanceBlock> {
        self.expect(&Token::Resonate)?;
        self.expect(&Token::LBrace)?;
        let mut depth = 1u32;
        while depth > 0 {
            match self.peek() {
                Some(Token::LBrace) => {
                    depth += 1;
                    self.advance();
                }
                Some(Token::RBrace) => {
                    depth -= 1;
                    if depth > 0 {
                        self.advance();
                    }
                }
                Some(_) => {
                    self.advance();
                }
                None => return Err(GameError::unexpected_eof("'}'")),
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(ResonanceBlock {
            bindings: Vec::new(),
            damping: None,
        })
    }

    // ── Define (stub for M0) ──────────────────────────────────────────

    fn parse_define(&mut self) -> Result<DefineBlock> {
        self.expect(&Token::Define)?;
        let name = self.expect_ident()?;
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while !self.at(&Token::RParen) {
            params.push(self.expect_ident()?);
            if self.at(&Token::Comma) {
                self.advance();
            }
        }
        self.expect(&Token::RParen)?;
        self.expect(&Token::LBrace)?;
        let body = self.parse_pipe_chain()?;
        self.expect(&Token::RBrace)?;
        Ok(DefineBlock { name, params, body })
    }

    // ── Properties & Params ───────────────────────────────────────────

    fn parse_property(&mut self) -> Result<Property> {
        let name = self.expect_ident()?;
        self.expect(&Token::Colon)?;
        let value = self.parse_expr(0)?;
        Ok(Property { name, value })
    }

    fn parse_property_or_param(&mut self) -> Result<PropertyOrParam> {
        let name = self.expect_ident()?;
        self.expect(&Token::Colon)?;
        let value = self.parse_expr(0)?;

        if self.at(&Token::Tilde) {
            self.advance(); // consume ~
            let signal = self.parse_expr(0)?;
            Ok(PropertyOrParam::Param(ParamDecl {
                name,
                base_value: value,
                modulation: Some(Modulation { signal }),
            }))
        } else {
            // Check if this looks like a param (has a numeric value) or a property
            // For now, treat everything without ~ as a property
            Ok(PropertyOrParam::Property(Property { name, value }))
        }
    }

    // ── Pipe Chains ───────────────────────────────────────────────────

    fn parse_pipe_chain(&mut self) -> Result<PipeChain> {
        let mut stages = vec![self.parse_fn_call()?];

        while self.at(&Token::Pipe) {
            self.advance(); // consume |
            stages.push(self.parse_fn_call()?);
        }

        Ok(PipeChain { stages })
    }

    fn parse_fn_call(&mut self) -> Result<FnCall> {
        let name = self.expect_ident()?;
        self.expect(&Token::LParen)?;

        let mut args = Vec::new();
        while !self.at(&Token::RParen) {
            if self.peek().is_none() {
                return Err(GameError::unexpected_eof("')'"));
            }

            // Check for named argument: `ident:`
            if self.at_ident() {
                // Look ahead for colon (named arg)
                if self.pos + 1 < self.tokens.len()
                    && self.tokens[self.pos + 1].token == Token::Colon
                {
                    let arg_name = self.expect_ident()?;
                    self.expect(&Token::Colon)?;
                    let value = self.parse_expr(0)?;
                    args.push(Arg::Named {
                        name: arg_name,
                        value,
                    });
                } else {
                    args.push(Arg::Positional(self.parse_expr(0)?));
                }
            } else {
                args.push(Arg::Positional(self.parse_expr(0)?));
            }

            if self.at(&Token::Comma) {
                self.advance();
            }
        }

        self.expect(&Token::RParen)?;
        Ok(FnCall { name, args })
    }

    // ── Expressions (Pratt precedence climbing) ───────────────────────

    fn parse_expr(&mut self, min_prec: u8) -> Result<Expr> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match self.peek() {
                Some(Token::Plus) => BinOp::Add,
                Some(Token::Minus) => BinOp::Sub,
                Some(Token::Star) => BinOp::Mul,
                Some(Token::Slash) => BinOp::Div,
                Some(Token::Greater) => BinOp::Gt,
                Some(Token::Less) => BinOp::Lt,
                _ => break,
            };

            if op.precedence() <= min_prec {
                break;
            }

            self.advance(); // consume operator
            let right = self.parse_expr(op.precedence())?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        // Ternary: expr ? expr : expr (lowest precedence — only at top level)
        if min_prec == 0 && self.at(&Token::Question) {
            self.advance();
            let if_true = self.parse_expr(0)?;
            self.expect(&Token::Colon)?;
            let if_false = self.parse_expr(0)?;
            left = Expr::Ternary {
                condition: Box::new(left),
                if_true: Box::new(if_true),
                if_false: Box::new(if_false),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        if self.at(&Token::Minus) {
            self.advance();
            let expr = self.parse_primary()?;
            return Ok(Expr::Negate(Box::new(expr)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        let expr = match self.peek() {
            Some(Token::Float(_)) => {
                let s = self.advance().unwrap();
                if let Token::Float(v) = s.token {
                    Expr::Number(v)
                } else {
                    unreachable!()
                }
            }
            Some(Token::Int(_)) => {
                let s = self.advance().unwrap();
                if let Token::Int(v) = s.token {
                    Expr::Number(v as f64)
                } else {
                    unreachable!()
                }
            }
            Some(Token::String(_)) => {
                let s = self.advance().unwrap();
                if let Token::String(v) = &s.token {
                    Expr::String(v.clone())
                } else {
                    unreachable!()
                }
            }
            Some(Token::Ident(_)) => {
                let name = self.expect_ident()?;

                // Function call: ident(...)
                if self.at(&Token::LParen) {
                    let call = self.parse_fn_call_with_name(name)?;
                    Expr::Call(call)
                }
                // Field access: ident.ident.ident
                else if self.at(&Token::Dot) {
                    let mut expr = Expr::Ident(name);
                    while self.at(&Token::Dot) {
                        self.advance();
                        let field = self.expect_ident()?;
                        expr = Expr::FieldAccess {
                            object: Box::new(expr),
                            field,
                        };
                    }
                    expr
                } else {
                    Expr::Ident(name)
                }
            }
            Some(Token::LParen) => {
                self.advance();
                let expr = self.parse_expr(0)?;
                self.expect(&Token::RParen)?;
                expr
            }
            Some(Token::LBracket) => {
                self.advance();
                let mut elements = Vec::new();
                while !self.at(&Token::RBracket) {
                    if self.peek().is_none() {
                        return Err(GameError::unexpected_eof("']'"));
                    }
                    elements.push(self.parse_expr(0)?);
                    if self.at(&Token::Comma) {
                        self.advance();
                    }
                }
                self.expect(&Token::RBracket)?;
                Expr::Array(elements)
            }
            Some(_) => {
                let s = self.peek_spanned().unwrap();
                return Err(GameError::unexpected_token(
                    "expression",
                    s.token.describe(),
                    s.span.clone(),
                ));
            }
            None => return Err(GameError::unexpected_eof("expression")),
        };

        Ok(expr)
    }

    /// Parse fn call when we already consumed the name.
    fn parse_fn_call_with_name(&mut self, name: String) -> Result<FnCall> {
        self.expect(&Token::LParen)?;
        let mut args = Vec::new();
        while !self.at(&Token::RParen) {
            if self.peek().is_none() {
                return Err(GameError::unexpected_eof("')'"));
            }
            if self.at_ident()
                && self.pos + 1 < self.tokens.len()
                && self.tokens[self.pos + 1].token == Token::Colon
            {
                let arg_name = self.expect_ident()?;
                self.expect(&Token::Colon)?;
                let value = self.parse_expr(0)?;
                args.push(Arg::Named {
                    name: arg_name,
                    value,
                });
            } else {
                args.push(Arg::Positional(self.parse_expr(0)?));
            }
            if self.at(&Token::Comma) {
                self.advance();
            }
        }
        self.expect(&Token::RParen)?;
        Ok(FnCall { name, args })
    }

    fn parse_number(&mut self) -> Result<f64> {
        match self.peek() {
            Some(Token::Float(_)) => {
                let s = self.advance().unwrap();
                if let Token::Float(v) = s.token {
                    Ok(v)
                } else {
                    unreachable!()
                }
            }
            Some(Token::Int(_)) => {
                let s = self.advance().unwrap();
                if let Token::Int(v) = s.token {
                    Ok(v as f64)
                } else {
                    unreachable!()
                }
            }
            Some(_) => {
                let s = self.peek_spanned().unwrap();
                Err(GameError::unexpected_token(
                    "number",
                    s.token.describe(),
                    s.span.clone(),
                ))
            }
            None => Err(GameError::unexpected_eof("number")),
        }
    }
}

enum PropertyOrParam {
    Property(Property),
    Param(ParamDecl),
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;

    fn parse_source(src: &str) -> Cinematic {
        let tokens = lexer::lex(src).expect("lex failed");
        let mut parser = Parser::new(tokens);
        parser.parse().expect("parse failed")
    }

    #[test]
    fn parse_hello_game() {
        let cin = parse_source(
            r#"cinematic "Hello" {
                layer {
                    fn: circle(0.3 + sin(time) * 0.05) | glow(2.0)
                }
            }"#,
        );

        assert_eq!(cin.name.as_deref(), Some("Hello"));
        assert_eq!(cin.layers.len(), 1);

        let layer = &cin.layers[0];
        assert!(layer.name.is_none());

        let chain = layer.fn_chain.as_ref().expect("fn chain should exist");
        assert_eq!(chain.stages.len(), 2);
        assert_eq!(chain.stages[0].name, "circle");
        assert_eq!(chain.stages[1].name, "glow");

        // circle has 1 positional arg: 0.3 + sin(time) * 0.05
        assert_eq!(chain.stages[0].args.len(), 1);

        // glow has 1 positional arg: 2.0
        assert_eq!(chain.stages[1].args.len(), 1);
        if let Arg::Positional(Expr::Number(v)) = &chain.stages[1].args[0] {
            assert!((v - 2.0).abs() < 1e-10);
        } else {
            panic!("expected glow(2.0)");
        }
    }

    #[test]
    fn parse_named_layer() {
        let cin = parse_source(
            r#"cinematic {
                layer terrain {
                    fn: fbm(p)
                    scale: 2.0
                }
            }"#,
        );

        let layer = &cin.layers[0];
        assert_eq!(layer.name.as_deref(), Some("terrain"));
        assert_eq!(layer.properties.len(), 1);
        assert_eq!(layer.properties[0].name, "scale");
    }

    #[test]
    fn parse_modulation() {
        let cin = parse_source(
            r#"cinematic {
                layer x {
                    fn: circle(0.5)
                    scale: 2.0 ~ audio.bass * 1.5
                }
            }"#,
        );

        let layer = &cin.layers[0];
        assert_eq!(layer.params.len(), 1);
        assert_eq!(layer.params[0].name, "scale");
        assert!(layer.params[0].modulation.is_some());
    }

    #[test]
    fn parse_operator_precedence() {
        // 0.3 + sin(time) * 0.05  should parse as  0.3 + (sin(time) * 0.05)
        let cin = parse_source(
            r#"cinematic {
                layer { fn: f(0.3 + sin(time) * 0.05) }
            }"#,
        );
        let chain = cin.layers[0].fn_chain.as_ref().unwrap();
        let arg = &chain.stages[0].args[0];
        if let Arg::Positional(Expr::BinaryOp { op, right, .. }) = arg {
            assert_eq!(*op, BinOp::Add);
            // right should be a multiplication
            if let Expr::BinaryOp { op: inner_op, .. } = right.as_ref() {
                assert_eq!(*inner_op, BinOp::Mul);
            } else {
                panic!("expected multiplication on right side of addition");
            }
        } else {
            panic!("expected binary op");
        }
    }
}
