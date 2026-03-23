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
        self.peek().map_or(false, |t| {
            std::mem::discriminant(t) == std::mem::discriminant(expected)
        })
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

    /// Convert a keyword token to its string name. Returns None for non-keyword tokens.
    /// Used to allow keywords as identifiers in contexts like layer names, param names, etc.
    fn token_to_name(tok: &Token) -> Option<String> {
        Some(
            match tok {
                Token::Opacity => "opacity",
                Token::Blend => "blend",
                Token::Memory => "memory",
                Token::Cast => "cast",
                Token::Score => "score",
                Token::Flow => "flow",
                Token::Particles => "particles",
                Token::React => "react",
                Token::Swarm => "swarm",
                Token::Gravity => "gravity",
                Token::Voice => "voice",
                Token::Listen => "listen",
                Token::Feedback => "feedback",
                Token::Play => "play",
                Token::Pass => "pass",
                Token::Matrix => "matrix",
                Token::Arc => "arc",
                Token::Resonate => "resonate",
                Token::Over => "over",
                Token::Breed => "breed",
                Token::From => "from",
                Token::Inherit => "inherit",
                Token::Mutate => "mutate",
                Token::Project => "project",
                Token::Scene => "scene",
                Token::Transition => "transition",
                Token::Props => "props",
                Token::Dom => "dom",
                _ => return None,
            }
            .into(),
        )
    }

    /// Check if the token at `pos` is a name-like token (Ident or keyword usable as name).
    fn is_name_token_at(&self, pos: usize) -> bool {
        match self.tokens.get(pos) {
            Some((Token::Ident(_), _, _)) => true,
            Some((tok, _, _)) => Self::token_to_name(tok).is_some(),
            None => false,
        }
    }

    // -- expect helpers ----------------------------------------------------

    fn expect(&mut self, expected: &Token) -> Result<Token, CompileError> {
        let (line, col) = self.current_pos();
        match self.advance() {
            Some(tok) if std::mem::discriminant(&tok) == std::mem::discriminant(expected) => {
                Ok(tok)
            }
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

    /// Accept an identifier OR a keyword that can appear as a name in certain contexts
    /// (e.g., layer names, field names after `.`, parameter names, stage names).
    /// AI-generated code frequently uses keywords as names (e.g., `layer flow { ... }`).
    fn expect_ident_or_keyword(&mut self) -> Result<String, CompileError> {
        let (line, col) = self.current_pos();
        match self.advance() {
            Some(Token::Ident(s)) => Ok(s),
            Some(tok) if Self::token_to_name(&tok).is_some() => {
                Ok(Self::token_to_name(&tok).unwrap())
            }
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

    /// Parse an easing function name that may contain hyphens (e.g., "ease-out", "ease-in-out").
    fn expect_easing(&mut self) -> Result<String, CompileError> {
        let mut name = self.expect_ident()?;
        // Consume hyphenated continuations: ease-out, ease-in-out
        while matches!(self.peek(), Some(Token::Minus)) {
            // Look ahead: is there an ident after the minus?
            if self
                .tokens
                .get(self.pos + 1)
                .map(|(t, _, _)| matches!(t, Token::Ident(_)))
                .unwrap_or(false)
            {
                self.advance(); // consume minus
                let part = self.expect_ident()?;
                name.push('-');
                name.push_str(&part);
            } else {
                break;
            }
        }
        Ok(name)
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
                Token::LBrace => {
                    depth += 1;
                    self.advance();
                }
                Token::RBrace if depth > 0 => {
                    depth -= 1;
                    self.advance();
                }
                Token::RBrace => {
                    self.advance();
                    return;
                }
                _ => {
                    self.advance();
                }
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
        let mut fns = Vec::new();
        let mut scenes = Vec::new();
        let mut ifs_blocks = Vec::new();
        let mut lsystem_blocks = Vec::new();
        let mut automaton_blocks = Vec::new();
        let mut matrix_blocks = Vec::new();

        while !self.at_end() {
            match self.peek() {
                Some(Token::Import) => match self.parse_import() {
                    Ok(imp) => imports.push(imp),
                    Err(e) => {
                        self.skip_to_recovery();
                        return Err(e);
                    }
                },
                Some(Token::Use) => match self.parse_use_import() {
                    Ok(imp) => imports.push(imp),
                    Err(e) => {
                        self.skip_to_recovery();
                        return Err(e);
                    }
                },
                Some(Token::Fn) => match self.parse_fn_def() {
                    Ok(f) => fns.push(f),
                    Err(e) => {
                        self.skip_to_recovery();
                        return Err(e);
                    }
                },
                Some(Token::Cinematic) => match self.parse_cinematic() {
                    Ok(cin) => cinematics.push(cin),
                    Err(e) => {
                        self.skip_to_recovery();
                        return Err(e);
                    }
                },
                Some(Token::Breed) => match self.parse_breed() {
                    Ok(b) => breeds.push(b),
                    Err(e) => {
                        self.skip_to_recovery();
                        return Err(e);
                    }
                },
                Some(Token::Project) => match self.parse_project() {
                    Ok(p) => projects.push(p),
                    Err(e) => {
                        self.skip_to_recovery();
                        return Err(e);
                    }
                },
                Some(Token::Scene) => match self.parse_scene() {
                    Ok(sc) => scenes.push(sc),
                    Err(e) => {
                        self.skip_to_recovery();
                        return Err(e);
                    }
                },
                Some(Token::Matrix) => match self.parse_matrix_block() {
                    Ok(m) => matrix_blocks.push(m),
                    Err(e) => {
                        self.skip_to_recovery();
                        return Err(e);
                    }
                },
                // Contextual keywords for v0.6 blocks
                Some(Token::Ident(s)) if s == "ifs" => match self.parse_ifs_block() {
                    Ok(b) => ifs_blocks.push(b),
                    Err(e) => {
                        self.skip_to_recovery();
                        return Err(e);
                    }
                },
                Some(Token::Ident(s)) if s == "lsystem" => match self.parse_lsystem_block() {
                    Ok(b) => lsystem_blocks.push(b),
                    Err(e) => {
                        self.skip_to_recovery();
                        return Err(e);
                    }
                },
                Some(Token::Ident(s)) if s == "automaton" => match self.parse_automaton_block() {
                    Ok(b) => automaton_blocks.push(b),
                    Err(e) => {
                        self.skip_to_recovery();
                        return Err(e);
                    }
                },
                Some(_) => {
                    let (line, col) = self.current_pos();
                    let tok = self.advance();
                    return Err(CompileError::ParseError {
                        message: format!(
                            "expected `import`, `use`, `fn`, `cinematic`, `breed`, `project`, `scene`, `matrix`, `ifs`, `lsystem`, or `automaton` at top level, found `{}`",
                            tok.map_or("EOF".into(), |t| t.to_string())
                        ),
                        line,
                        col,
                    });
                }
                None => break,
            }
        }

        Ok(Program {
            imports,
            cinematics,
            breeds,
            projects,
            fns,
            scenes,
            ifs_blocks,
            lsystem_blocks,
            automaton_blocks,
            matrix_blocks,
        })
    }

    // ======================================================================
    // fn name(params) { pipeline }
    // ======================================================================

    fn parse_fn_def(&mut self) -> Result<FnDef, CompileError> {
        self.expect(&Token::Fn)?;
        let name = self.expect_ident()?;
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        if !self.check(&Token::RParen) {
            params.push(self.expect_ident()?);
            while matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
                params.push(self.expect_ident()?);
            }
        }
        self.expect(&Token::RParen)?;
        self.expect(&Token::LBrace)?;
        let mut body = Vec::new();
        body.push(self.parse_stage()?);
        while matches!(self.peek(), Some(Token::Pipe)) {
            self.advance();
            body.push(self.parse_stage()?);
        }
        self.expect(&Token::RBrace)?;
        Ok(FnDef { name, params, body })
    }

    // ======================================================================
    // use "path.game"
    // ======================================================================

    fn parse_use_import(&mut self) -> Result<Import, CompileError> {
        self.expect(&Token::Use)?;
        let path = self.expect_string()?;
        // Derive alias from filename: "shapes.game" -> "shapes"
        let alias = path
            .rsplit('/')
            .next()
            .unwrap_or(&path)
            .rsplit('\\')
            .next()
            .unwrap_or(&path)
            .trim_end_matches(".game")
            .to_string();
        Ok(Import { path, alias })
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
    // pass name { pipeline } — inside cinematic
    // ======================================================================

    fn parse_pass_block(&mut self) -> Result<PassBlock, CompileError> {
        self.expect(&Token::Pass)?;
        let name = self.expect_ident()?;
        self.expect(&Token::LBrace)?;
        let mut body = Vec::new();
        body.push(self.parse_stage()?);
        while matches!(self.peek(), Some(Token::Pipe)) {
            self.advance();
            body.push(self.parse_stage()?);
        }
        self.expect(&Token::RBrace)?;
        Ok(PassBlock { name, body })
    }

    // ======================================================================
    // use "cinematic-name" as alias — inside cinematic
    // ======================================================================

    fn parse_cinematic_use(&mut self) -> Result<CinematicUse, CompileError> {
        self.expect(&Token::Use)?;
        let source = self.expect_string()?;
        self.expect(&Token::As)?;
        let alias = self.expect_ident()?;
        Ok(CinematicUse { source, alias })
    }

    // ======================================================================
    // scene "name" { play/transition entries }
    // ======================================================================

    fn parse_scene(&mut self) -> Result<SceneBlock, CompileError> {
        self.expect(&Token::Scene)?;
        let name = self.expect_string()?;
        self.expect(&Token::LBrace)?;

        let mut entries = Vec::new();
        while !self.at_end() && !self.check(&Token::RBrace) {
            match self.peek() {
                Some(Token::Play) => {
                    self.advance();
                    let cinematic = self.expect_string()?;
                    self.expect(&Token::For)?;
                    let duration = self.parse_duration()?;
                    entries.push(SceneEntry::Play {
                        cinematic,
                        duration,
                    });
                }
                Some(Token::Transition) => {
                    self.advance();
                    let kind_str = self.expect_ident()?;
                    let kind = match kind_str.as_str() {
                        "dissolve" => TransitionKind::Dissolve,
                        "fade" => TransitionKind::Fade,
                        "wipe" => TransitionKind::Wipe,
                        "morph" => TransitionKind::Morph,
                        _ => {
                            return Err(CompileError::validation(format!(
                                "unknown transition '{kind_str}', expected: dissolve, fade, wipe, morph"
                            )));
                        }
                    };
                    self.expect(&Token::Over)?;
                    let duration = self.parse_duration()?;
                    entries.push(SceneEntry::Transition { kind, duration });
                }
                Some(Token::Pipe) => {
                    self.advance(); // skip pipe separators
                }
                _ => {
                    let (line, col) = self.current_pos();
                    return Err(CompileError::ParseError {
                        message: format!(
                            "expected `play` or `transition` in scene, found `{}`",
                            self.peek().map_or("EOF".into(), |t| t.to_string())
                        ),
                        line,
                        col,
                    });
                }
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(SceneBlock { name, entries })
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
        let mut react = None;
        let mut swarm = None;
        let mut flow = None;
        let mut particles = None;
        let mut passes = Vec::new();
        let mut cinematic_uses = Vec::new();
        let mut matrix_coupling = None;
        let mut matrix_color = None;
        let mut props = None;
        let mut dom = None;
        let mut events = Vec::new();
        let mut role = None;
        let mut scene3d = None;
        let mut textures = Vec::new();

        while !self.at_end() && !self.check(&Token::RBrace) {
            match self.peek() {
                Some(Token::Layer) => layers.push(self.parse_layer()?),
                Some(Token::Arc) => arcs.push(self.parse_arc()?),
                Some(Token::Resonate) => resonates.push(self.parse_resonate()?),
                Some(Token::Listen) => listen = Some(self.parse_listen()?),
                Some(Token::Voice) => voice = Some(self.parse_voice()?),
                Some(Token::Score) => score = Some(self.parse_score()?),
                Some(Token::Gravity) => gravity = Some(self.parse_gravity()?),
                Some(Token::React) => react = Some(self.parse_react()?),
                Some(Token::Swarm) => swarm = Some(self.parse_swarm()?),
                Some(Token::Flow) => flow = Some(self.parse_flow()?),
                Some(Token::Particles) => particles = Some(self.parse_particles()?),
                Some(Token::Pass) => passes.push(self.parse_pass_block()?),
                Some(Token::Use) => cinematic_uses.push(self.parse_cinematic_use()?),
                Some(Token::Props) => props = Some(self.parse_props_block()?),
                Some(Token::Dom) => dom = Some(self.parse_dom_block()?),
                Some(Token::Matrix) => {
                    let matrix = self.parse_matrix_block()?;
                    match matrix {
                        MatrixBlock::Coupling(c) => matrix_coupling = Some(c),
                        MatrixBlock::Color(c) => matrix_color = Some(c),
                        MatrixBlock::Transitions(_) => {
                            let (line, col) = self.current_pos();
                            return Err(CompileError::ParseError {
                                message: "transition matrices belong at top level, not inside cinematic blocks".into(),
                                line,
                                col,
                            });
                        }
                    }
                }
                // `on "event" { ... }` and `role: "value"` are parsed from ident context
                Some(Token::Ident(s)) if s == "on" => {
                    events.push(self.parse_event_handler()?);
                }
                Some(Token::Ident(s)) if s == "role" => {
                    role = Some(self.parse_role()?);
                }
                Some(Token::Ident(s)) if s == "scene3d" => {
                    scene3d = Some(self.parse_scene3d_block()?);
                }
                Some(Token::Ident(s)) if s == "texture" => {
                    textures.push(self.parse_texture_decl()?);
                }
                _ => {
                    let (line, col) = self.current_pos();
                    return Err(CompileError::ParseError {
                        message: format!(
                            "expected `layer`, `arc`, `resonate`, `listen`, `voice`, `score`, `gravity`, `react`, `swarm`, `flow`, `particles`, `pass`, `use`, `matrix`, `props`, `dom`, `on`, `role`, `scene3d`, or `texture` inside cinematic, found `{}`",
                            self.peek().map_or("EOF".into(), |t| t.to_string())
                        ),
                        line,
                        col,
                    });
                }
            }
        }

        self.expect(&Token::RBrace)?;
        Ok(Cinematic {
            name,
            layers,
            arcs,
            resonates,
            listen,
            voice,
            score,
            gravity,
            react,
            swarm,
            flow,
            particles,
            passes,
            cinematic_uses,
            matrix_coupling,
            matrix_color,
            props,
            dom,
            events,
            role,
            scene3d,
            textures,
        })
    }

    // ======================================================================
    // layer ident [(opts)] { body }
    // ======================================================================

    fn parse_layer(&mut self) -> Result<Layer, CompileError> {
        self.expect(&Token::Layer)?;
        let name = self.expect_ident_or_keyword()?;

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

        // Optional `opacity : <float>`
        let opacity = if matches!(self.peek(), Some(Token::Opacity)) {
            self.advance(); // consume `opacity`
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

        // Optional `blend: <mode>` (add, screen, multiply, overlay)
        let blend = if matches!(self.peek(), Some(Token::Blend)) {
            self.advance(); // consume `blend`
            self.expect(&Token::Colon)?;
            let mode_str = self.expect_ident()?;
            match mode_str.as_str() {
                "add" => BlendMode::Add,
                "screen" => BlendMode::Screen,
                "multiply" => BlendMode::Multiply,
                "overlay" => BlendMode::Overlay,
                "occlude" => BlendMode::Occlude,
                _ => {
                    return Err(CompileError::validation(format!(
                        "unknown blend mode '{}', expected: add, screen, multiply, overlay, occlude",
                        mode_str
                    )));
                }
            }
        } else {
            BlendMode::Add
        };

        // Optional `feedback: true`
        let feedback = if matches!(self.peek(), Some(Token::Feedback)) {
            self.advance(); // consume `feedback`
            self.expect(&Token::Colon)?;
            let val = self.expect_ident()?;
            val == "true"
        } else {
            false
        };

        self.expect(&Token::LBrace)?;
        let body = self.parse_layer_body()?;
        self.expect(&Token::RBrace)?;

        Ok(Layer {
            name,
            opts,
            memory,
            opacity,
            cast,
            blend,
            feedback,
            body,
        })
    }

    fn parse_layer_opts(&mut self) -> Result<Vec<Param>, CompileError> {
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while !self.at_end() && !self.check(&Token::RParen) {
            let name = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let value = self.parse_expr()?;
            params.push(Param {
                name,
                value,
                modulation: None,
                temporal_ops: vec![],
            });
            if !self.check(&Token::RParen) {
                self.expect(&Token::Comma)?;
            }
        }
        self.expect(&Token::RParen)?;
        Ok(params)
    }

    // -- layer body: params OR pipe stages ---------------------------------

    fn parse_layer_body(&mut self) -> Result<LayerBody, CompileError> {
        // Decide by lookahead: IDENT COLON => params, IDENT LPAREN => stages, IF => conditional
        if self.at_end() || self.check(&Token::RBrace) {
            return Ok(LayerBody::Params(Vec::new()));
        }

        // Conditional: if expr { pipeline } else { pipeline }
        if matches!(self.peek(), Some(Token::If)) {
            return self.parse_conditional_body();
        }

        // Check if first token is a name-like token (ident or keyword usable as name)
        let first_is_name = self.is_name_token_at(self.pos);
        let second = self.tokens.get(self.pos + 1).map(|(t, _, _)| t.clone());

        if first_is_name {
            match second.as_ref() {
                Some(Token::Colon) => self.parse_param_list(),
                Some(Token::LParen) => self.parse_stage_pipeline(),
                Some(Token::Pipe) => {
                    // Pipeline starting with parameterless stage: `polar | simplex(6.0)`
                    self.parse_stage_pipeline()
                }
                Some(Token::RBrace) => {
                    // Single bare stage: `layer x { polar }`
                    self.parse_stage_pipeline()
                }
                _ => self.parse_param_list(),
            }
        } else {
            // Could be a single-token expression param or error --
            // try params first, fall back to error.
            self.parse_param_list()
        }
    }

    fn parse_conditional_body(&mut self) -> Result<LayerBody, CompileError> {
        self.expect(&Token::If)?;
        let condition = self.parse_expr()?;
        self.expect(&Token::LBrace)?;
        let mut then_branch = Vec::new();
        then_branch.push(self.parse_stage()?);
        while matches!(self.peek(), Some(Token::Pipe)) {
            self.advance();
            then_branch.push(self.parse_stage()?);
        }
        self.expect(&Token::RBrace)?;
        self.expect(&Token::Else)?;
        self.expect(&Token::LBrace)?;
        let mut else_branch = Vec::new();
        else_branch.push(self.parse_stage()?);
        while matches!(self.peek(), Some(Token::Pipe)) {
            self.advance();
            else_branch.push(self.parse_stage()?);
        }
        self.expect(&Token::RBrace)?;
        Ok(LayerBody::Conditional {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn parse_param_list(&mut self) -> Result<LayerBody, CompileError> {
        let mut params = Vec::new();
        while !self.at_end() && !self.check(&Token::RBrace) {
            let name = self.expect_ident_or_keyword()?;
            self.expect(&Token::Colon)?;
            let value = self.parse_expr()?;

            let modulation = if matches!(self.peek(), Some(Token::Tilde)) {
                self.advance();
                Some(self.parse_expr()?)
            } else {
                None
            };

            let temporal_ops = self.parse_temporal_ops()?;

            params.push(Param {
                name,
                value,
                modulation,
                temporal_ops,
            });
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
        let name = self.expect_ident_or_keyword()?;
        let args = if matches!(self.peek(), Some(Token::LParen)) {
            self.advance(); // consume (
            let a = self.parse_arg_list()?;
            self.expect(&Token::RParen)?;
            a
        } else {
            vec![]
        };
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
        // Named arg: NAME COLON expr  or  positional: expr
        // Lookahead for NAME ':' (where NAME is ident or keyword-as-name)
        let is_named = if let Some((Token::Colon, _, _)) = self.tokens.get(self.pos + 1) {
            self.is_name_token_at(self.pos)
        } else {
            false
        };
        if is_named {
            let name = self.expect_ident_or_keyword()?;
            self.expect(&Token::Colon)?;
            let value = self.parse_expr()?;
            Ok(Arg {
                name: Some(name),
                value,
            })
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

        // Check for optional lifecycle state: enter, exit, hover, idle
        let state = match self.peek() {
            Some(Token::Ident(s)) if matches!(s.as_str(), "enter" | "exit" | "hover" | "idle") => {
                let name = s.clone();
                self.advance();
                Some(match name.as_str() {
                    "enter" => ArcState::Enter,
                    "exit" => ArcState::Exit,
                    "hover" => ArcState::Hover,
                    "idle" => ArcState::Idle,
                    _ => unreachable!(),
                })
            }
            _ => None,
        };

        self.expect(&Token::LBrace)?;
        let mut entries = Vec::new();
        while !self.at_end() && !self.check(&Token::RBrace) {
            entries.push(self.parse_arc_entry()?);
        }
        self.expect(&Token::RBrace)?;
        Ok(ArcBlock { state, entries })
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
            Some(self.expect_easing()?)
        } else {
            None
        };
        Ok(ArcEntry {
            target,
            from,
            to,
            duration,
            easing,
        })
    }

    fn parse_dotted_ident(&mut self) -> Result<String, CompileError> {
        let mut s = self.expect_ident_or_keyword()?;
        while matches!(self.peek(), Some(Token::Dot)) {
            self.advance();
            let part = self.expect_ident_or_keyword()?;
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
            Some(Token::Float(v)) => Err(CompileError::ParseError {
                message: format!(
                    "expected duration (e.g. 2s, 500ms, 4bars), found bare number {v}"
                ),
                line,
                col,
            }),
            Some(Token::Integer(v)) => Err(CompileError::ParseError {
                message: format!(
                    "expected duration (e.g. 2s, 500ms, 4bars), found bare number {v}"
                ),
                line,
                col,
            }),
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
        let source = self.expect_ident_or_keyword()?;
        self.expect(&Token::Arrow)?;
        let target = self.expect_ident_or_keyword()?;
        self.expect(&Token::Dot)?;
        let field = self.expect_ident_or_keyword()?;
        self.expect(&Token::Star)?;
        let weight = self.parse_expr()?;
        Ok(ResonateEntry {
            source,
            target,
            field,
            weight,
        })
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
            signals.push(ListenSignal {
                name,
                algorithm,
                params,
            });
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
            params.push(Param {
                name,
                value,
                modulation: None,
                temporal_ops: vec![],
            });
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
                self.parse_listen_params()? // Reuse same param parser
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
                    sections.push(Section {
                        name,
                        phrases: refs,
                    });
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
                    });
                }
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(ScoreBlock {
            tempo_bpm,
            motifs,
            phrases,
            sections,
            arrange,
        })
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
                    inherit_rules.push(InheritRule {
                        target,
                        strategy,
                        weight,
                    });
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
                    });
                }
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(BreedBlock {
            name,
            parents,
            inherit_rules,
            mutations,
        })
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
                    });
                }
            }
            // optional comma separator
            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(GravityBlock {
            force_law,
            damping,
            bounds,
        })
    }

    // ======================================================================
    // react { feed, kill, diffuse_a, diffuse_b, seed }
    // ======================================================================

    fn parse_react(&mut self) -> Result<ReactBlock, CompileError> {
        self.expect(&Token::React)?;
        self.expect(&Token::LBrace)?;

        let mut feed = 0.055;
        let mut kill = 0.062;
        let mut diffuse_a = 1.0;
        let mut diffuse_b = 0.5;
        let mut seed = SeedMode::Center(0.1);

        while !self.at_end() && !self.check(&Token::RBrace) {
            let key = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            match key.as_str() {
                "feed" => feed = self.expect_number()?,
                "kill" => kill = self.expect_number()?,
                "diffuse_a" => diffuse_a = self.expect_number()?,
                "diffuse_b" => diffuse_b = self.expect_number()?,
                "seed" => {
                    let mode = self.expect_ident()?;
                    self.expect(&Token::LParen)?;
                    match mode.as_str() {
                        "center" => {
                            let radius = self.expect_number()?;
                            seed = SeedMode::Center(radius);
                        }
                        "scatter" => {
                            let count = self.expect_number()? as u32;
                            seed = SeedMode::Scatter(count);
                        }
                        "random" => {
                            let density = self.expect_number()?;
                            seed = SeedMode::Random(density);
                        }
                        _ => {
                            let (line, col) = self.current_pos();
                            return Err(CompileError::ParseError {
                                message: format!(
                                    "unknown seed mode `{mode}`, expected center/scatter/random"
                                ),
                                line,
                                col,
                            });
                        }
                    }
                    self.expect(&Token::RParen)?;
                }
                _ => {
                    let (line, col) = self.current_pos();
                    return Err(CompileError::ParseError {
                        message: format!("unknown react property `{key}`"),
                        line,
                        col,
                    });
                }
            }
            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(ReactBlock {
            feed,
            kill,
            diffuse_a,
            diffuse_b,
            seed,
        })
    }

    // ======================================================================
    // swarm { agents, sensor_angle, sensor_dist, turn_angle, step, ... }
    // ======================================================================

    fn parse_swarm(&mut self) -> Result<SwarmBlock, CompileError> {
        self.expect(&Token::Swarm)?;
        self.expect(&Token::LBrace)?;

        let mut agents = 100000u32;
        let mut sensor_angle = 45.0;
        let mut sensor_dist = 9.0;
        let mut turn_angle = 45.0;
        let mut step_size = 1.0;
        let mut deposit = 5.0;
        let mut decay = 0.95;
        let mut diffuse = 1u32;
        let mut bounds = BoundsMode::Wrap;

        while !self.at_end() && !self.check(&Token::RBrace) {
            let key = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            match key.as_str() {
                "agents" => agents = self.expect_number()? as u32,
                "sensor_angle" => sensor_angle = self.expect_number()?,
                "sensor_dist" => sensor_dist = self.expect_number()?,
                "turn_angle" => turn_angle = self.expect_number()?,
                "step" => step_size = self.expect_number()?,
                "deposit" => deposit = self.expect_number()?,
                "decay" => decay = self.expect_number()?,
                "diffuse" => diffuse = self.expect_number()? as u32,
                "bounds" => {
                    let mode_str = self.expect_ident()?;
                    bounds = match mode_str.as_str() {
                        "reflect" => BoundsMode::Reflect,
                        "wrap" => BoundsMode::Wrap,
                        "none" => BoundsMode::None,
                        _ => {
                            let (line, col) = self.current_pos();
                            return Err(CompileError::ParseError {
                                message: format!("unknown bounds mode `{mode_str}`"),
                                line,
                                col,
                            });
                        }
                    };
                }
                _ => {
                    let (line, col) = self.current_pos();
                    return Err(CompileError::ParseError {
                        message: format!("unknown swarm property `{key}`"),
                        line,
                        col,
                    });
                }
            }
            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(SwarmBlock {
            agents,
            sensor_angle,
            sensor_dist,
            turn_angle,
            step_size,
            deposit,
            decay,
            diffuse,
            bounds,
        })
    }

    // ======================================================================
    // flow { type, scale, speed, octaves, strength, bounds }
    // ======================================================================

    fn parse_flow(&mut self) -> Result<FlowBlock, CompileError> {
        self.expect(&Token::Flow)?;
        self.expect(&Token::LBrace)?;

        let mut flow_type = FlowType::Curl;
        let mut scale = 3.0;
        let mut speed = 0.5;
        let mut octaves = 4u32;
        let mut strength = 1.0;
        let mut bounds = BoundsMode::Wrap;

        while !self.at_end() && !self.check(&Token::RBrace) {
            let key = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            match key.as_str() {
                "type" => {
                    let t = self.expect_ident()?;
                    flow_type = match t.as_str() {
                        "curl" => FlowType::Curl,
                        "perlin" => FlowType::Perlin,
                        "simplex" => FlowType::Simplex,
                        "vortex" => FlowType::Vortex,
                        _ => {
                            let (line, col) = self.current_pos();
                            return Err(CompileError::ParseError {
                                message: format!("unknown flow type `{t}`"),
                                line,
                                col,
                            });
                        }
                    };
                }
                "scale" => scale = self.expect_number()?,
                "speed" => speed = self.expect_number()?,
                "octaves" => octaves = self.expect_number()? as u32,
                "strength" => strength = self.expect_number()?,
                "bounds" => {
                    let mode_str = self.expect_ident()?;
                    bounds = match mode_str.as_str() {
                        "reflect" => BoundsMode::Reflect,
                        "wrap" => BoundsMode::Wrap,
                        "none" => BoundsMode::None,
                        _ => {
                            let (line, col) = self.current_pos();
                            return Err(CompileError::ParseError {
                                message: format!("unknown bounds mode `{mode_str}`"),
                                line,
                                col,
                            });
                        }
                    };
                }
                _ => {
                    let (line, col) = self.current_pos();
                    return Err(CompileError::ParseError {
                        message: format!("unknown flow property `{key}`"),
                        line,
                        col,
                    });
                }
            }
            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(FlowBlock {
            flow_type,
            scale,
            speed,
            octaves,
            strength,
            bounds,
        })
    }

    // ======================================================================
    // particles { count, emit, lifetime, speed, spread, gravity, size, fade, color }
    // ======================================================================

    fn parse_particles(&mut self) -> Result<ParticlesBlock, CompileError> {
        self.expect(&Token::Particles)?;
        self.expect(&Token::LBrace)?;

        let mut count = 1000u32;
        let mut emit = EmitMode::Center;
        let mut lifetime = 2.0;
        let mut speed = 0.5;
        let mut spread = 360.0;
        let mut gravity = 0.0;
        let mut size = 2.0;
        let mut fade = true;
        let mut color = "white".to_string();

        while !self.at_end() && !self.check(&Token::RBrace) {
            // Use expect_ident_or_keyword because "gravity" is a keyword token
            let key = self.expect_ident_or_keyword()?;
            self.expect(&Token::Colon)?;
            match key.as_str() {
                "count" => count = self.expect_number()? as u32,
                "emit" => {
                    let mode = self.expect_ident()?;
                    emit = match mode.as_str() {
                        "center" => EmitMode::Center,
                        "random" => EmitMode::Random,
                        "ring" => {
                            // ring or ring(radius)
                            if self.check(&Token::LParen) {
                                self.advance();
                                let radius = self.expect_number()?;
                                self.expect(&Token::RParen)?;
                                EmitMode::Ring(radius)
                            } else {
                                EmitMode::Ring(0.3)
                            }
                        }
                        "point" => {
                            self.expect(&Token::LParen)?;
                            let x = self.expect_number()?;
                            self.expect(&Token::Comma)?;
                            let y = self.expect_number()?;
                            self.expect(&Token::RParen)?;
                            EmitMode::Point(x, y)
                        }
                        _ => {
                            let (line, col) = self.current_pos();
                            return Err(CompileError::ParseError {
                                message: format!(
                                    "unknown emit mode `{mode}`, expected center/random/ring/point"
                                ),
                                line,
                                col,
                            });
                        }
                    };
                }
                "lifetime" => lifetime = self.expect_number()?,
                "speed" => speed = self.expect_number()?,
                "spread" => spread = self.expect_number()?,
                "gravity" => gravity = self.parse_number_value()?,
                "size" => size = self.expect_number()?,
                "fade" => {
                    let val = self.expect_ident()?;
                    fade = val == "true";
                }
                "color" => color = self.expect_ident()?,
                _ => {
                    let (line, col) = self.current_pos();
                    return Err(CompileError::ParseError {
                        message: format!("unknown particles property `{key}`"),
                        line,
                        col,
                    });
                }
            }
            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(ParticlesBlock {
            count,
            emit,
            lifetime,
            speed,
            spread,
            gravity,
            size,
            fade,
            color,
        })
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

        Ok(ProjectBlock {
            mode,
            source,
            params,
        })
    }

    // ======================================================================
    // Expressions — precedence climbing
    // ======================================================================

    pub fn parse_expr(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.parse_additive()?;
        while matches!(
            self.peek(),
            Some(Token::Gt)
                | Some(Token::Lt)
                | Some(Token::Gte)
                | Some(Token::Lte)
                | Some(Token::EqEq)
                | Some(Token::NotEq)
        ) {
            let op = match self.advance() {
                Some(Token::Gt) => BinOp::Gt,
                Some(Token::Lt) => BinOp::Lt,
                Some(Token::Gte) => BinOp::Gte,
                Some(Token::Lte) => BinOp::Lte,
                Some(Token::EqEq) => BinOp::Eq,
                Some(Token::NotEq) => BinOp::NotEq,
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
            Some(Token::Float(v)) => {
                self.advance();
                Ok(Expr::Number(v))
            }
            Some(Token::Integer(v)) => {
                self.advance();
                Ok(Expr::Number(v as f64))
            }
            Some(Token::Seconds(v)) => {
                self.advance();
                Ok(Expr::Duration(Duration::Seconds(v)))
            }
            Some(Token::Millis(v)) => {
                self.advance();
                Ok(Expr::Duration(Duration::Millis(v)))
            }
            Some(Token::Bars(v)) => {
                self.advance();
                Ok(Expr::Duration(Duration::Bars(v)))
            }
            Some(Token::Degrees(v)) => {
                self.advance();
                Ok(Expr::Number(v))
            }
            Some(Token::StringLit(s)) => {
                self.advance();
                Ok(Expr::String(s))
            }
            Some(Token::HexColor(r, g, b)) => {
                self.advance();
                Ok(Expr::Color(r, g, b))
            }
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
                    Ok(Expr::DottedIdent {
                        object: name,
                        field,
                    })
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
            // Keywords used as identifiers in expressions (AI-generated code)
            Some(ref tok) if Self::token_to_name(tok).is_some() => {
                let name = Self::token_to_name(tok).unwrap();
                self.advance();
                if matches!(self.peek(), Some(Token::LParen)) {
                    self.advance();
                    let args = self.parse_arg_list()?;
                    self.expect(&Token::RParen)?;
                    Ok(Expr::Call { name, args })
                } else if matches!(self.peek(), Some(Token::Dot)) {
                    self.advance();
                    let field = self.expect_ident_or_keyword()?;
                    Ok(Expr::DottedIdent {
                        object: name,
                        field,
                    })
                } else {
                    Ok(Expr::Ident(name))
                }
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

    // ======================================================================
    // ifs "name" { transform "t1" [a,b,c,d,e,f] weight w | iterations N | color mode }
    // ======================================================================

    fn parse_ifs_block(&mut self) -> Result<IfsBlock, CompileError> {
        self.advance(); // consume "ifs" ident
        self.expect(&Token::LBrace)?;

        let mut transforms = Vec::new();
        let mut iterations = 100000u32;
        let mut color_mode = IfsColorMode::Transform;

        while !self.check(&Token::RBrace) {
            match self.peek() {
                Some(Token::Ident(s)) if s == "transform" => {
                    self.advance();
                    let name = self.expect_ident()?;
                    self.expect(&Token::LBracket)?;
                    let mut matrix = [0.0f64; 6];
                    for i in 0..6 {
                        if i > 0 {
                            self.expect(&Token::Comma)?;
                        }
                        matrix[i] = self.parse_number_value()?;
                    }
                    self.expect(&Token::RBracket)?;

                    // weight
                    let mut weight = 1.0;
                    if let Some(Token::Ident(s)) = self.peek() {
                        if s == "weight" {
                            self.advance();
                            weight = self.parse_number_value()?;
                        }
                    }
                    transforms.push(IfsTransform {
                        name,
                        matrix,
                        weight,
                    });
                }
                Some(Token::Ident(s)) if s == "iterations" => {
                    self.advance();
                    iterations = self.parse_number_value()? as u32;
                }
                Some(Token::Ident(s)) if s == "color" => {
                    self.advance();
                    let mode = self.expect_ident()?;
                    color_mode = match mode.as_str() {
                        "depth" => IfsColorMode::Depth,
                        "position" => IfsColorMode::Position,
                        _ => IfsColorMode::Transform,
                    };
                }
                Some(Token::Pipe) => {
                    self.advance();
                }
                _ => {
                    let (line, col) = self.current_pos();
                    return Err(CompileError::ParseError {
                        message: "expected `transform`, `iterations`, or `color` in ifs block"
                            .into(),
                        line,
                        col,
                    });
                }
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(IfsBlock {
            transforms,
            iterations,
            color_mode,
        })
    }

    // ======================================================================
    // lsystem "name" { axiom "F" | rule F -> "F+F" | angle 60 | iterations 4 | step 0.01 }
    // ======================================================================

    fn parse_lsystem_block(&mut self) -> Result<LsystemBlock, CompileError> {
        self.advance(); // consume "lsystem" ident
        self.expect(&Token::LBrace)?;

        let mut axiom = String::new();
        let mut rules = Vec::new();
        let mut angle = 90.0f64;
        let mut iterations = 4u32;
        let mut step = 0.01f64;

        while !self.check(&Token::RBrace) {
            match self.peek() {
                Some(Token::Ident(s)) if s == "axiom" => {
                    self.advance();
                    if let Some(Token::StringLit(s)) = self.peek().cloned() {
                        self.advance();
                        axiom = s;
                    } else {
                        axiom = self.expect_ident()?;
                    }
                }
                Some(Token::Ident(s)) if s == "rule" => {
                    self.advance();
                    let symbol_str = self.expect_ident()?;
                    let symbol = symbol_str.chars().next().unwrap_or('F');
                    self.expect(&Token::Arrow)?;
                    let replacement = if let Some(Token::StringLit(s)) = self.peek().cloned() {
                        self.advance();
                        s
                    } else {
                        self.expect_ident()?
                    };
                    rules.push(LsystemRule {
                        symbol,
                        replacement,
                    });
                }
                Some(Token::Ident(s)) if s == "angle" => {
                    self.advance();
                    angle = self.parse_number_value()?;
                }
                Some(Token::Ident(s)) if s == "iterations" => {
                    self.advance();
                    iterations = self.parse_number_value()? as u32;
                }
                Some(Token::Ident(s)) if s == "step" => {
                    self.advance();
                    step = self.parse_number_value()?;
                }
                Some(Token::Pipe) => {
                    self.advance();
                }
                _ => {
                    let (line, col) = self.current_pos();
                    return Err(CompileError::ParseError {
                        message:
                            "expected `axiom`, `rule`, `angle`, `iterations`, or `step` in lsystem block"
                                .into(),
                        line,
                        col,
                    });
                }
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(LsystemBlock {
            axiom,
            rules,
            angle,
            iterations,
            step,
        })
    }

    // ======================================================================
    // automaton { states 2 | neighborhood moore | rule "B3/S23" | seed random 0.5 | speed 10 }
    // ======================================================================

    fn parse_automaton_block(&mut self) -> Result<AutomatonBlock, CompileError> {
        self.advance(); // consume "automaton" ident
        self.expect(&Token::LBrace)?;

        let mut states = 2u32;
        let mut neighborhood = Neighborhood::Moore;
        let mut rule = "B3/S23".to_string();
        let mut seed = AutomatonSeed::Random(0.5);
        let mut speed = 10u32;

        while !self.check(&Token::RBrace) {
            match self.peek() {
                Some(Token::Ident(s)) if s == "states" => {
                    self.advance();
                    states = self.parse_number_value()? as u32;
                }
                Some(Token::Ident(s)) if s == "neighborhood" => {
                    self.advance();
                    let kind = self.expect_ident()?;
                    neighborhood = match kind.as_str() {
                        "vonneumann" | "von_neumann" => Neighborhood::VonNeumann,
                        _ => Neighborhood::Moore,
                    };
                }
                Some(Token::Ident(s)) if s == "rule" => {
                    self.advance();
                    if let Some(Token::StringLit(s)) = self.peek().cloned() {
                        self.advance();
                        rule = s;
                    } else {
                        rule = self.expect_ident()?;
                    }
                }
                Some(Token::Ident(s)) if s == "seed" => {
                    self.advance();
                    let kind = self.expect_ident()?;
                    seed = match kind.as_str() {
                        "center" => AutomatonSeed::Center,
                        "pattern" => {
                            if let Some(Token::StringLit(s)) = self.peek().cloned() {
                                self.advance();
                                AutomatonSeed::Pattern(s)
                            } else {
                                AutomatonSeed::Center
                            }
                        }
                        _ => {
                            // "random" with optional density
                            let density =
                                if let Some(Token::Float(_) | Token::Integer(_)) = self.peek() {
                                    self.parse_number_value()?
                                } else {
                                    0.5
                                };
                            AutomatonSeed::Random(density)
                        }
                    };
                }
                Some(Token::Ident(s)) if s == "speed" => {
                    self.advance();
                    speed = self.parse_number_value()? as u32;
                }
                Some(Token::Pipe) => {
                    self.advance();
                }
                _ => {
                    let (line, col) = self.current_pos();
                    return Err(CompileError::ParseError {
                        message:
                            "expected `states`, `neighborhood`, `rule`, `seed`, or `speed` in automaton block"
                                .into(),
                        line,
                        col,
                    });
                }
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(AutomatonBlock {
            states,
            neighborhood,
            rule,
            seed,
            speed,
        })
    }

    // ======================================================================
    // matrix <kind> { ... }
    // ======================================================================

    fn parse_matrix_block(&mut self) -> Result<MatrixBlock, CompileError> {
        self.expect(&Token::Matrix)?;
        let kind = self.expect_ident()?;
        match kind.as_str() {
            "coupling" => self.parse_matrix_coupling().map(MatrixBlock::Coupling),
            "color" => self.parse_matrix_color().map(MatrixBlock::Color),
            "transitions" => self
                .parse_matrix_transitions()
                .map(MatrixBlock::Transitions),
            _ => {
                let (line, col) = self.current_pos();
                Err(CompileError::ParseError {
                    message: format!(
                        "expected `coupling`, `color`, or `transitions` after `matrix`, found `{}`",
                        kind
                    ),
                    line,
                    col,
                })
            }
        }
    }

    /// Parse `matrix coupling { [sources] -> [targets] weights [...] damping N depth N }`
    fn parse_matrix_coupling(&mut self) -> Result<MatrixCoupling, CompileError> {
        self.expect(&Token::LBrace)?;

        // Parse [source1, source2, ...] -> [target1.field1, target2.field2, ...]
        self.expect(&Token::LBracket)?;
        let mut sources = Vec::new();
        loop {
            sources.push(self.expect_ident()?);
            if self.check(&Token::RBracket) {
                self.advance();
                break;
            }
            self.expect(&Token::Comma)?;
        }

        self.expect(&Token::Arrow)?;

        self.expect(&Token::LBracket)?;
        let mut targets = Vec::new();
        loop {
            let layer = self.expect_ident()?;
            self.expect(&Token::Dot)?;
            let field = self.expect_ident_or_keyword()?;
            targets.push(MatrixTarget { layer, field });
            if self.check(&Token::RBracket) {
                self.advance();
                break;
            }
            self.expect(&Token::Comma)?;
        }

        // Parse named fields
        let mut weights = Vec::new();
        let mut damping = 0.95;
        let mut depth = 4u32;

        while !self.check(&Token::RBrace) {
            match self.peek() {
                Some(Token::Ident(s)) if s == "weights" => {
                    self.advance();
                    self.expect(&Token::LBracket)?;
                    loop {
                        let v = self.parse_number_value()?;
                        weights.push(v);
                        if self.check(&Token::RBracket) {
                            self.advance();
                            break;
                        }
                        self.expect(&Token::Comma)?;
                    }
                }
                Some(Token::Ident(s)) if s == "damping" => {
                    self.advance();
                    damping = self.parse_number_value()?;
                }
                Some(Token::Ident(s)) if s == "depth" => {
                    self.advance();
                    depth = self.parse_number_value()? as u32;
                }
                _ => {
                    let (line, col) = self.current_pos();
                    return Err(CompileError::ParseError {
                        message: format!(
                            "expected `weights`, `damping`, or `depth` in matrix coupling, found `{}`",
                            self.peek().map_or("EOF".into(), |t| t.to_string())
                        ),
                        line,
                        col,
                    });
                }
            }
        }

        self.expect(&Token::RBrace)?;

        // Validate weight dimensions: must be sources * targets (or empty)
        let expected = sources.len() * targets.len();
        if !weights.is_empty() && weights.len() != expected {
            let (line, col) = self.current_pos();
            return Err(CompileError::ParseError {
                message: format!(
                    "matrix coupling weights: expected {} values ({}x{}), got {}",
                    expected,
                    sources.len(),
                    targets.len(),
                    weights.len()
                ),
                line,
                col,
            });
        }

        Ok(MatrixCoupling {
            sources,
            targets,
            weights,
            damping,
            depth,
        })
    }

    /// Parse `matrix color { [9 values] }`
    fn parse_matrix_color(&mut self) -> Result<MatrixColor, CompileError> {
        self.expect(&Token::LBrace)?;
        self.expect(&Token::LBracket)?;

        let mut values = [0.0f64; 9];
        for i in 0..9 {
            let v = self.parse_number_value()?;
            values[i] = v;
            if i < 8 {
                self.expect(&Token::Comma)?;
            }
        }

        self.expect(&Token::RBracket)?;
        self.expect(&Token::RBrace)?;
        Ok(MatrixColor { values })
    }

    /// Parse `matrix transitions "name" { states [...] weights [...] hold Ns }`
    fn parse_matrix_transitions(&mut self) -> Result<MatrixTransitions, CompileError> {
        let name = self.expect_string()?;
        self.expect(&Token::LBrace)?;

        let mut states = Vec::new();
        let mut weights = Vec::new();
        let mut hold = Duration::Seconds(5.0);

        while !self.check(&Token::RBrace) {
            match self.peek() {
                Some(Token::Ident(s)) if s == "states" => {
                    self.advance();
                    self.expect(&Token::LBracket)?;
                    loop {
                        if let Some(Token::StringLit(_)) = self.peek() {
                            let s = self.expect_string()?;
                            states.push(s);
                        } else {
                            states.push(self.expect_ident()?);
                        }
                        if self.check(&Token::RBracket) {
                            self.advance();
                            break;
                        }
                        self.expect(&Token::Comma)?;
                    }
                }
                Some(Token::Ident(s)) if s == "weights" => {
                    self.advance();
                    self.expect(&Token::LBracket)?;
                    loop {
                        let v = self.parse_number_value()?;
                        weights.push(v);
                        if self.check(&Token::RBracket) {
                            self.advance();
                            break;
                        }
                        self.expect(&Token::Comma)?;
                    }
                }
                Some(Token::Ident(s)) if s == "hold" => {
                    self.advance();
                    hold = self.parse_duration()?;
                }
                _ => {
                    let (line, col) = self.current_pos();
                    return Err(CompileError::ParseError {
                        message: format!(
                            "expected `states`, `weights`, or `hold` in matrix transitions, found `{}`",
                            self.peek().map_or("EOF".into(), |t| t.to_string())
                        ),
                        line,
                        col,
                    });
                }
            }
        }

        self.expect(&Token::RBrace)?;

        // Validate weight dimensions: must be states * states (or empty)
        let expected = states.len() * states.len();
        if !weights.is_empty() && weights.len() != expected {
            let (line, col) = self.current_pos();
            return Err(CompileError::ParseError {
                message: format!(
                    "matrix transitions weights: expected {} values ({}x{}), got {}",
                    expected,
                    states.len(),
                    states.len(),
                    weights.len()
                ),
                line,
                col,
            });
        }

        Ok(MatrixTransitions {
            name,
            states,
            weights,
            hold,
        })
    }

    /// Parse a numeric literal (float or int) and return as f64.
    fn parse_number_value(&mut self) -> Result<f64, CompileError> {
        let (line, col) = self.current_pos();
        match self.advance() {
            Some(Token::Float(v)) => Ok(v),
            Some(Token::Integer(v)) => Ok(v as f64),
            Some(Token::Minus) => {
                let val = self.parse_number_value()?;
                Ok(-val)
            }
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

    /// Parse a CSS value: either a bare number (becomes `"{n}px"`) or a string
    /// literal (passed through, e.g. `"50%"`, `"auto"`).
    fn parse_css_value(&mut self) -> Result<String, CompileError> {
        match self.peek() {
            Some(Token::StringLit(_)) => self.expect_string(),
            Some(Token::Float(_)) | Some(Token::Integer(_)) | Some(Token::Minus) => {
                let n = self.parse_number_value()?;
                Ok(format!("{n}px"))
            }
            _ => {
                let (line, col) = self.current_pos();
                Err(CompileError::ParseError {
                    message: "expected number or string literal for CSS value".into(),
                    line,
                    col,
                })
            }
        }
    }

    // ======================================================================
    // Phase v0.8: Component UI Layer
    // ======================================================================

    /// Parse `props { name: default, ... }`
    ///
    /// Properties with string defaults become string props (DOM-bound).
    /// Properties with number defaults become number props (shader uniforms).
    /// Properties ending with `: event` become event emitters.
    fn parse_props_block(&mut self) -> Result<PropsBlock, CompileError> {
        self.expect(&Token::Props)?;
        self.expect(&Token::LBrace)?;

        let mut props = Vec::new();
        while !self.check(&Token::RBrace) && !self.at_end() {
            let name = self.expect_ident_or_keyword()?;
            self.expect(&Token::Colon)?;

            // Check for `event` keyword (no default value)
            if let Some(Token::Ident(s)) = self.peek() {
                if s == "event" {
                    self.advance();
                    props.push(PropDef {
                        name,
                        default: Expr::String(String::new()),
                        is_event: true,
                    });
                    // Optional comma
                    if self.check(&Token::Comma) {
                        self.advance();
                    }
                    continue;
                }
            }

            // Parse default value (string or number expression)
            let default = self.parse_expr()?;
            let is_event = false;
            props.push(PropDef {
                name,
                default,
                is_event,
            });

            // Optional comma
            if self.check(&Token::Comma) {
                self.advance();
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(PropsBlock { props })
    }

    /// Parse `dom { text "name" { at: x y, style: "css", bind: "prop" } ... }`
    fn parse_dom_block(&mut self) -> Result<DomBlock, CompileError> {
        self.expect(&Token::Dom)?;
        self.expect(&Token::LBrace)?;

        let mut elements = Vec::new();
        while !self.check(&Token::RBrace) && !self.at_end() {
            let tag = self.expect_ident()?; // "text", "div", etc.
            let name = self.expect_string()?;
            self.expect(&Token::LBrace)?;

            let mut x = "0px".to_string();
            let mut y = "0px".to_string();
            let mut style = String::new();
            let mut bind = None;
            let mut width = None;
            let mut align = None;

            while !self.check(&Token::RBrace) && !self.at_end() {
                let key = self.expect_ident_or_keyword()?;
                self.expect(&Token::Colon)?;
                match key.as_str() {
                    "at" => {
                        x = self.parse_css_value()?;
                        y = self.parse_css_value()?;
                    }
                    "style" => {
                        style = self.expect_string()?;
                    }
                    "bind" => {
                        bind = Some(self.expect_string()?);
                    }
                    "width" => {
                        width = Some(self.parse_css_value()?);
                    }
                    "align" => {
                        align = Some(self.expect_string()?);
                    }
                    _ => {
                        let (line, col) = self.current_pos();
                        return Err(CompileError::ParseError {
                            message: format!(
                                "expected `at`, `style`, `bind`, `width`, or `align` in dom element, found `{key}`"
                            ),
                            line,
                            col,
                        });
                    }
                }
            }
            self.expect(&Token::RBrace)?;

            elements.push(DomElement {
                tag,
                name,
                x,
                y,
                style,
                bind,
                width,
                align,
            });
        }

        self.expect(&Token::RBrace)?;
        Ok(DomBlock { elements })
    }

    /// Parse `on "event" { emit: "name" }`
    fn parse_event_handler(&mut self) -> Result<EventHandler, CompileError> {
        // Consume the "on" identifier
        self.advance();
        let event = self.expect_string()?;
        self.expect(&Token::LBrace)?;

        let mut emit = None;
        while !self.check(&Token::RBrace) && !self.at_end() {
            let key = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            match key.as_str() {
                "emit" => {
                    emit = Some(self.expect_string()?);
                }
                _ => {
                    let (line, col) = self.current_pos();
                    return Err(CompileError::ParseError {
                        message: format!("expected `emit` in on block, found `{key}`"),
                        line,
                        col,
                    });
                }
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(EventHandler { event, emit })
    }

    /// Parse `role: "alert"` — accessibility role declaration.
    fn parse_role(&mut self) -> Result<String, CompileError> {
        // Consume the "role" identifier
        self.advance();
        self.expect(&Token::Colon)?;
        self.expect_string()
    }

    // ======================================================================
    // scene3d { camera: mode, fov: number, distance: number }
    // ======================================================================

    fn parse_scene3d_block(&mut self) -> Result<Scene3dBlock, CompileError> {
        // Consume the "scene3d" identifier
        self.advance();
        self.expect(&Token::LBrace)?;

        let mut camera = CameraMode::Orbit;
        let mut fov = 45.0;
        let mut distance = 3.0;

        while !self.at_end() && !self.check(&Token::RBrace) {
            let key = self.expect_ident_or_keyword()?;
            self.expect(&Token::Colon)?;
            match key.as_str() {
                "camera" => {
                    let mode_str = self.expect_ident()?;
                    camera = match mode_str.as_str() {
                        "orbit" => CameraMode::Orbit,
                        "static" => CameraMode::Static,
                        "fly" => CameraMode::Fly,
                        _ => {
                            let (line, col) = self.current_pos();
                            return Err(CompileError::ParseError {
                                message: format!(
                                    "unknown camera mode '{}', expected: orbit, static, fly",
                                    mode_str
                                ),
                                line,
                                col,
                            });
                        }
                    };
                }
                "fov" => {
                    fov = self.expect_number()?;
                }
                "distance" => {
                    distance = self.expect_number()?;
                }
                _ => {
                    let (line, col) = self.current_pos();
                    return Err(CompileError::ParseError {
                        message: format!(
                            "unknown scene3d property '{}', expected: camera, fov, distance",
                            key
                        ),
                        line,
                        col,
                    });
                }
            }
        }

        self.expect(&Token::RBrace)?;
        Ok(Scene3dBlock {
            camera,
            fov,
            distance,
        })
    }

    // ======================================================================
    // texture "name" [from "url"]
    // ======================================================================

    fn parse_texture_decl(&mut self) -> Result<TextureDecl, CompileError> {
        // Consume the "texture" identifier
        self.advance();
        let name = self.expect_string()?;

        // Optional `from "url"` clause
        let source = if matches!(self.peek(), Some(Token::From)) {
            self.advance(); // consume `from`
            Some(self.expect_string()?)
        } else {
            None
        };

        Ok(TextureDecl { name, source })
    }
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
