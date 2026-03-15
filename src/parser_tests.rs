use super::*;
use crate::token::Token;

/// Build a token triple with dummy span offsets.
fn s(tok: Token) -> (Token, usize, usize) {
    (tok, 0, 0)
}

// ===================================================================
// Empty program
// ===================================================================

#[test]
fn parse_empty_program() {
    let mut p = Parser::new(vec![]);
    let prog = p.parse().expect("should parse empty program");
    assert!(prog.imports.is_empty());
    assert!(prog.cinematics.is_empty());
}

// ===================================================================
// Import
// ===================================================================

#[test]
fn parse_import() {
    let tokens = vec![
        s(Token::Import),
        s(Token::StringLit("lib/base.game".into())),
        s(Token::As),
        s(Token::Ident("base".into())),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse import");
    assert_eq!(prog.imports.len(), 1);
    assert_eq!(prog.imports[0].path, "lib/base.game");
    assert_eq!(prog.imports[0].alias, "base");
}

// ===================================================================
// Cinematic with one layer
// ===================================================================

#[test]
fn parse_basic_cinematic_with_layer() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("intro".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("bg".into())),
        s(Token::LBrace),
        s(Token::Ident("color".into())),
        s(Token::Colon),
        s(Token::StringLit("red".into())),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse");
    assert_eq!(prog.cinematics.len(), 1);
    assert_eq!(prog.cinematics[0].name, "intro");
    assert_eq!(prog.cinematics[0].layers.len(), 1);
    assert_eq!(prog.cinematics[0].layers[0].name, "bg");
}

// ===================================================================
// Layer with pipe stages
// ===================================================================

#[test]
fn parse_layer_with_pipe_stages() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("test".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("fx".into())),
        s(Token::LBrace),
        s(Token::Ident("circle".into())),
        s(Token::LParen),
        s(Token::Float(0.2)),
        s(Token::RParen),
        s(Token::Pipe),
        s(Token::Ident("glow".into())),
        s(Token::LParen),
        s(Token::Float(1.5)),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse pipeline");
    let layer = &prog.cinematics[0].layers[0];
    match &layer.body {
        LayerBody::Pipeline(stages) => {
            assert_eq!(stages.len(), 2);
            assert_eq!(stages[0].name, "circle");
            assert_eq!(stages[1].name, "glow");
        }
        _ => panic!("expected pipeline body"),
    }
}

// ===================================================================
// Modulation (~)
// ===================================================================

#[test]
fn parse_modulation() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("m".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("bg".into())),
        s(Token::LBrace),
        s(Token::Ident("opacity".into())),
        s(Token::Colon),
        s(Token::Float(0.5)),
        s(Token::Tilde),
        s(Token::Ident("sin".into())),
        s(Token::LParen),
        s(Token::Ident("t".into())),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse modulation");
    let layer = &prog.cinematics[0].layers[0];
    match &layer.body {
        LayerBody::Params(params) => {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "opacity");
            assert!(params[0].modulation.is_some());
        }
        _ => panic!("expected params body"),
    }
}

// ===================================================================
// Arc block
// ===================================================================

#[test]
fn parse_arc_block() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("a".into())),
        s(Token::LBrace),
        s(Token::Arc),
        s(Token::LBrace),
        s(Token::Ident("bg".into())),
        s(Token::Dot),
        s(Token::Ident("opacity".into())),
        s(Token::Colon),
        s(Token::Integer(0)),
        s(Token::Arrow),
        s(Token::Integer(1)),
        s(Token::Over),
        s(Token::Seconds(2.0)),
        s(Token::Ident("ease_in".into())),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse arc");
    assert_eq!(prog.cinematics[0].arcs.len(), 1);
    let entry = &prog.cinematics[0].arcs[0].entries[0];
    assert_eq!(entry.target, "bg.opacity");
    assert_eq!(entry.easing, Some("ease_in".into()));
}

// ===================================================================
// Resonate block
// ===================================================================

#[test]
fn parse_resonate_block() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("r".into())),
        s(Token::LBrace),
        s(Token::Resonate),
        s(Token::LBrace),
        s(Token::Ident("kick".into())),
        s(Token::Arrow),
        s(Token::Ident("bg".into())),
        s(Token::Dot),
        s(Token::Ident("scale".into())),
        s(Token::Star),
        s(Token::Float(0.3)),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse resonate");
    assert_eq!(prog.cinematics[0].resonates.len(), 1);
    let entry = &prog.cinematics[0].resonates[0].entries[0];
    assert_eq!(entry.source, "kick");
    assert_eq!(entry.target, "bg");
    assert_eq!(entry.field, "scale");
}

// ===================================================================
// Expression precedence
// ===================================================================

#[test]
fn parse_expr_precedence() {
    // 1 + 2 * 3  =>  Add(1, Mul(2, 3))
    let tokens = vec![
        s(Token::Integer(1)),
        s(Token::Plus),
        s(Token::Integer(2)),
        s(Token::Star),
        s(Token::Integer(3)),
    ];
    let mut p = Parser::new(tokens);
    let expr = p.parse_expr().expect("should parse");
    match &expr {
        Expr::BinOp {
            op: BinOp::Add,
            left,
            right,
        } => {
            assert!(matches!(left.as_ref(), Expr::Number(n) if (*n - 1.0).abs() < f64::EPSILON));
            assert!(matches!(right.as_ref(), Expr::BinOp { op: BinOp::Mul, .. }));
        }
        other => panic!("unexpected expr: {other:?}"),
    }
}

#[test]
fn parse_expr_power_right_assoc() {
    // 2 ^ 3 ^ 4  =>  Pow(2, Pow(3, 4))
    let tokens = vec![
        s(Token::Integer(2)),
        s(Token::Caret),
        s(Token::Integer(3)),
        s(Token::Caret),
        s(Token::Integer(4)),
    ];
    let mut p = Parser::new(tokens);
    let expr = p.parse_expr().expect("should parse");
    match &expr {
        Expr::BinOp {
            op: BinOp::Pow,
            right,
            ..
        } => {
            assert!(matches!(right.as_ref(), Expr::BinOp { op: BinOp::Pow, .. }));
        }
        other => panic!("unexpected expr: {other:?}"),
    }
}

// ===================================================================
// Layer with memory
// ===================================================================

#[test]
fn parse_layer_with_memory() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("t".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("bg".into())),
        s(Token::Memory),
        s(Token::Colon),
        s(Token::Float(0.95)),
        s(Token::LBrace),
        s(Token::Ident("color".into())),
        s(Token::Colon),
        s(Token::StringLit("blue".into())),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse memory");
    assert_eq!(prog.cinematics[0].layers[0].memory, Some(0.95));
}

// ===================================================================
// Layer with cast
// ===================================================================

#[test]
fn parse_layer_with_cast() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("t".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("bg".into())),
        s(Token::Cast),
        s(Token::Ident("sdf".into())),
        s(Token::LBrace),
        s(Token::Ident("color".into())),
        s(Token::Colon),
        s(Token::StringLit("blue".into())),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse cast");
    assert_eq!(prog.cinematics[0].layers[0].cast, Some("sdf".into()));
}

// ===================================================================
// Multiple layers
// ===================================================================

#[test]
fn parse_multiple_layers() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("multi".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("a".into())),
        s(Token::LBrace),
        s(Token::Ident("x".into())),
        s(Token::Colon),
        s(Token::Integer(1)),
        s(Token::RBrace),
        s(Token::Layer),
        s(Token::Ident("b".into())),
        s(Token::LBrace),
        s(Token::Ident("y".into())),
        s(Token::Colon),
        s(Token::Integer(2)),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse multiple layers");
    assert_eq!(prog.cinematics[0].layers.len(), 2);
    assert_eq!(prog.cinematics[0].layers[0].name, "a");
    assert_eq!(prog.cinematics[0].layers[1].name, "b");
}

// ===================================================================
// Error on unexpected token
// ===================================================================

#[test]
fn error_unexpected_token_at_top_level() {
    let tokens = vec![s(Token::Plus)];
    let mut p = Parser::new(tokens);
    let result = p.parse();
    assert!(result.is_err());
    match result.unwrap_err() {
        CompileError::ParseError { message, .. } => {
            assert!(message.contains("expected"));
        }
        other => panic!("expected ParseError, got {other:?}"),
    }
}

// ===================================================================
// Array expression
// ===================================================================

#[test]
fn parse_array_expr() {
    let tokens = vec![
        s(Token::LBracket),
        s(Token::Integer(1)),
        s(Token::Comma),
        s(Token::Integer(2)),
        s(Token::Comma),
        s(Token::Integer(3)),
        s(Token::RBracket),
    ];
    let mut p = Parser::new(tokens);
    let expr = p.parse_expr().expect("should parse array");
    match expr {
        Expr::Array(elems) => assert_eq!(elems.len(), 3),
        other => panic!("expected array, got {other:?}"),
    }
}

// ===================================================================
// Negative expression
// ===================================================================

#[test]
fn parse_negative_number() {
    let tokens = vec![s(Token::Minus), s(Token::Float(3.14))];
    let mut p = Parser::new(tokens);
    let expr = p.parse_expr().expect("should parse negative");
    assert!(matches!(expr, Expr::Neg(_)));
}

// ===================================================================
// Call expression
// ===================================================================

#[test]
fn parse_call_expr() {
    let tokens = vec![
        s(Token::Ident("sin".into())),
        s(Token::LParen),
        s(Token::Ident("t".into())),
        s(Token::RParen),
    ];
    let mut p = Parser::new(tokens);
    let expr = p.parse_expr().expect("should parse call");
    match expr {
        Expr::Call { name, args } => {
            assert_eq!(name, "sin");
            assert_eq!(args.len(), 1);
        }
        other => panic!("expected call, got {other:?}"),
    }
}

// ===================================================================
// Dotted ident expression
// ===================================================================

#[test]
fn parse_dotted_ident_expr() {
    let tokens = vec![
        s(Token::Ident("layer".into())),
        s(Token::Dot),
        s(Token::Ident("opacity".into())),
    ];
    let mut p = Parser::new(tokens);
    let expr = p.parse_expr().expect("should parse dotted ident");
    match expr {
        Expr::DottedIdent { object, field } => {
            assert_eq!(object, "layer");
            assert_eq!(field, "opacity");
        }
        other => panic!("expected dotted ident, got {other:?}"),
    }
}

// ===================================================================
// Named arg in stage
// ===================================================================

// ===================================================================
// Temporal operators
// ===================================================================

#[test]
fn parse_temporal_delay() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("t".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("bg".into())),
        s(Token::LBrace),
        s(Token::Ident("bass".into())),
        s(Token::Colon),
        s(Token::Float(0.5)),
        s(Token::Tilde),
        s(Token::Ident("audio".into())),
        s(Token::Dot),
        s(Token::Ident("bass".into())),
        s(Token::ShiftRight),
        s(Token::Millis(200.0)),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse temporal delay");
    match &prog.cinematics[0].layers[0].body {
        LayerBody::Params(params) => {
            assert_eq!(params[0].temporal_ops.len(), 1);
            assert!(
                matches!(&params[0].temporal_ops[0], TemporalOp::Delay(Duration::Millis(v)) if (*v - 200.0).abs() < f64::EPSILON)
            );
        }
        _ => panic!("expected params"),
    }
}

#[test]
fn parse_temporal_smooth() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("t".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("bg".into())),
        s(Token::LBrace),
        s(Token::Ident("val".into())),
        s(Token::Colon),
        s(Token::Float(0.5)),
        s(Token::Diamond),
        s(Token::Millis(50.0)),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse temporal smooth");
    match &prog.cinematics[0].layers[0].body {
        LayerBody::Params(params) => {
            assert_eq!(params[0].temporal_ops.len(), 1);
            assert!(
                matches!(&params[0].temporal_ops[0], TemporalOp::Smooth(Duration::Millis(v)) if (*v - 50.0).abs() < f64::EPSILON)
            );
        }
        _ => panic!("expected params"),
    }
}

#[test]
fn parse_temporal_trigger() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("t".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("bg".into())),
        s(Token::LBrace),
        s(Token::Ident("beat".into())),
        s(Token::Colon),
        s(Token::Float(0.0)),
        s(Token::Tilde),
        s(Token::Ident("audio".into())),
        s(Token::Dot),
        s(Token::Ident("beat".into())),
        s(Token::BangBang),
        s(Token::Millis(300.0)),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse temporal trigger");
    match &prog.cinematics[0].layers[0].body {
        LayerBody::Params(params) => {
            assert_eq!(params[0].temporal_ops.len(), 1);
            assert!(matches!(&params[0].temporal_ops[0], TemporalOp::Trigger(_)));
        }
        _ => panic!("expected params"),
    }
}

#[test]
fn parse_temporal_range() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("t".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("bg".into())),
        s(Token::LBrace),
        s(Token::Ident("energy".into())),
        s(Token::Colon),
        s(Token::Float(0.5)),
        s(Token::DotDot),
        s(Token::LBracket),
        s(Token::Float(0.1)),
        s(Token::Comma),
        s(Token::Float(0.9)),
        s(Token::RBracket),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse temporal range");
    match &prog.cinematics[0].layers[0].body {
        LayerBody::Params(params) => {
            assert_eq!(params[0].temporal_ops.len(), 1);
            assert!(matches!(
                &params[0].temporal_ops[0],
                TemporalOp::Range(_, _)
            ));
        }
        _ => panic!("expected params"),
    }
}

#[test]
fn parse_chained_temporal_ops() {
    // bass: 0.5 ~ audio.bass <> 50ms >> 200ms .. [0.0, 1.0]
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("t".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("bg".into())),
        s(Token::LBrace),
        s(Token::Ident("bass".into())),
        s(Token::Colon),
        s(Token::Float(0.5)),
        s(Token::Tilde),
        s(Token::Ident("audio".into())),
        s(Token::Dot),
        s(Token::Ident("bass".into())),
        s(Token::Diamond),
        s(Token::Millis(50.0)),
        s(Token::ShiftRight),
        s(Token::Millis(200.0)),
        s(Token::DotDot),
        s(Token::LBracket),
        s(Token::Float(0.0)),
        s(Token::Comma),
        s(Token::Float(1.0)),
        s(Token::RBracket),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse chained temporal ops");
    match &prog.cinematics[0].layers[0].body {
        LayerBody::Params(params) => {
            assert_eq!(params[0].temporal_ops.len(), 3);
            assert!(matches!(&params[0].temporal_ops[0], TemporalOp::Smooth(_)));
            assert!(matches!(&params[0].temporal_ops[1], TemporalOp::Delay(_)));
            assert!(matches!(
                &params[0].temporal_ops[2],
                TemporalOp::Range(_, _)
            ));
        }
        _ => panic!("expected params"),
    }
}

// ===================================================================
// Listen block
// ===================================================================

#[test]
fn parse_listen_block() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("t".into())),
        s(Token::LBrace),
        s(Token::Listen),
        s(Token::LBrace),
        s(Token::Ident("onset".into())),
        s(Token::Colon),
        s(Token::Ident("attack".into())),
        s(Token::LParen),
        s(Token::Ident("threshold".into())),
        s(Token::Colon),
        s(Token::Float(0.7)),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse listen");
    let listen = prog.cinematics[0].listen.as_ref().expect("listen block");
    assert_eq!(listen.signals.len(), 1);
    assert_eq!(listen.signals[0].name, "onset");
    assert_eq!(listen.signals[0].algorithm, "attack");
    assert_eq!(listen.signals[0].params.len(), 1);
}

// ===================================================================
// Voice block
// ===================================================================

#[test]
fn parse_voice_block() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("t".into())),
        s(Token::LBrace),
        s(Token::Voice),
        s(Token::LBrace),
        s(Token::Ident("tone".into())),
        s(Token::Colon),
        s(Token::Ident("sine".into())),
        s(Token::LParen),
        s(Token::Ident("freq".into())),
        s(Token::Colon),
        s(Token::Integer(440)),
        s(Token::RParen),
        s(Token::Ident("filt".into())),
        s(Token::Colon),
        s(Token::Ident("lowpass".into())),
        s(Token::LParen),
        s(Token::Ident("cutoff".into())),
        s(Token::Colon),
        s(Token::Integer(2000)),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse voice");
    let voice = prog.cinematics[0].voice.as_ref().expect("voice block");
    assert_eq!(voice.nodes.len(), 2);
    assert_eq!(voice.nodes[0].name, "tone");
    assert_eq!(voice.nodes[0].kind, "sine");
    assert_eq!(voice.nodes[1].name, "filt");
    assert_eq!(voice.nodes[1].kind, "lowpass");
}

// ===================================================================
// Score block
// ===================================================================

#[test]
fn parse_score_block() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("t".into())),
        s(Token::LBrace),
        s(Token::Score),
        s(Token::Ident("tempo".into())),
        s(Token::LParen),
        s(Token::Integer(120)),
        s(Token::RParen),
        s(Token::LBrace),
        // motif rise { scale: 0 -> 1 over 4bars }
        s(Token::Ident("motif".into())),
        s(Token::Ident("rise".into())),
        s(Token::LBrace),
        s(Token::Ident("scale".into())),
        s(Token::Colon),
        s(Token::Integer(0)),
        s(Token::Arrow),
        s(Token::Integer(1)),
        s(Token::Over),
        s(Token::Bars(4)),
        s(Token::RBrace),
        // phrase build = rise
        s(Token::Ident("phrase".into())),
        s(Token::Ident("build".into())),
        s(Token::Eq),
        s(Token::Ident("rise".into())),
        // arrange: build
        s(Token::Ident("arrange".into())),
        s(Token::Colon),
        s(Token::Ident("build".into())),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse score");
    let score = prog.cinematics[0].score.as_ref().expect("score block");
    assert!((score.tempo_bpm - 120.0).abs() < f64::EPSILON);
    assert_eq!(score.motifs.len(), 1);
    assert_eq!(score.motifs[0].name, "rise");
    assert_eq!(score.phrases.len(), 1);
    assert_eq!(score.arrange, vec!["build"]);
}

// ===================================================================
// Named arg in stage
// ===================================================================

#[test]
fn parse_named_arg_in_stage() {
    let tokens = vec![
        s(Token::Ident("stage".into())),
        s(Token::LParen),
        s(Token::Ident("rate".into())),
        s(Token::Colon),
        s(Token::Float(0.5)),
        s(Token::RParen),
    ];
    let mut p = Parser::new(tokens);
    let stage = p.parse_stage().expect("should parse named arg");
    assert_eq!(stage.args.len(), 1);
    assert_eq!(stage.args[0].name, Some("rate".into()));
}

// ===================================================================
// Breed block
// ===================================================================

#[test]
fn parse_breed_block() {
    let tokens = vec![
        s(Token::Breed),
        s(Token::StringLit("child".into())),
        s(Token::From),
        s(Token::StringLit("fire".into())),
        s(Token::Plus),
        s(Token::StringLit("ice".into())),
        s(Token::LBrace),
        // inherit layers: mix(0.6)
        s(Token::Inherit),
        s(Token::Ident("layers".into())),
        s(Token::Colon),
        s(Token::Ident("mix".into())),
        s(Token::LParen),
        s(Token::Float(0.6)),
        s(Token::RParen),
        // mutate scale: 0.3
        s(Token::Mutate),
        s(Token::Ident("scale".into())),
        s(Token::Colon),
        s(Token::Float(0.3)),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse breed");
    assert_eq!(prog.breeds.len(), 1);
    let b = &prog.breeds[0];
    assert_eq!(b.name, "child");
    assert_eq!(b.parents, vec!["fire", "ice"]);
    assert_eq!(b.inherit_rules.len(), 1);
    assert_eq!(b.inherit_rules[0].target, "layers");
    assert_eq!(b.inherit_rules[0].strategy, "mix");
    assert!((b.inherit_rules[0].weight - 0.6).abs() < f64::EPSILON);
    assert_eq!(b.mutations.len(), 1);
    assert_eq!(b.mutations[0].target, "scale");
    assert!((b.mutations[0].range - 0.3).abs() < f64::EPSILON);
}

// ===================================================================
// Gravity block
// ===================================================================

#[test]
fn parse_gravity_block() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("particles".into())),
        s(Token::LBrace),
        // gravity { damping: 0.995, bounds: wrap }
        s(Token::Gravity),
        s(Token::LBrace),
        s(Token::Ident("damping".into())),
        s(Token::Colon),
        s(Token::Float(0.995)),
        s(Token::Comma),
        s(Token::Ident("bounds".into())),
        s(Token::Colon),
        s(Token::Ident("wrap".into())),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse gravity");
    assert_eq!(prog.cinematics.len(), 1);
    let g = prog.cinematics[0]
        .gravity
        .as_ref()
        .expect("gravity should be Some");
    assert!((g.damping - 0.995).abs() < f64::EPSILON);
    assert_eq!(g.bounds, crate::ast::BoundsMode::Wrap);
}

// ===================================================================
// Project block
// ===================================================================

#[test]
fn parse_project_block() {
    let tokens = vec![
        s(Token::Project),
        s(Token::Ident("dome".into())),
        s(Token::LParen),
        s(Token::Ident("segments".into())),
        s(Token::Colon),
        s(Token::Float(8.0)),
        s(Token::RParen),
        s(Token::LBrace),
        s(Token::Ident("source".into())),
        s(Token::Colon),
        s(Token::Ident("main".into())),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse project");
    assert_eq!(prog.projects.len(), 1);
    let pr = &prog.projects[0];
    assert_eq!(pr.mode, crate::ast::ProjectMode::Dome);
    assert_eq!(pr.source, "main");
    assert_eq!(pr.params.len(), 1);
    assert_eq!(pr.params[0].name, "segments");
}

// ===================================================================
// React block
// ===================================================================

#[test]
fn parse_react_block() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("turing".into())),
        s(Token::LBrace),
        // react { feed: 0.04, kill: 0.06, seed: scatter(50) }
        s(Token::React),
        s(Token::LBrace),
        s(Token::Ident("feed".into())),
        s(Token::Colon),
        s(Token::Float(0.04)),
        s(Token::Comma),
        s(Token::Ident("kill".into())),
        s(Token::Colon),
        s(Token::Float(0.06)),
        s(Token::Comma),
        s(Token::Ident("seed".into())),
        s(Token::Colon),
        s(Token::Ident("scatter".into())),
        s(Token::LParen),
        s(Token::Float(50.0)),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse react");
    assert_eq!(prog.cinematics.len(), 1);
    let r = prog.cinematics[0]
        .react
        .as_ref()
        .expect("react should be Some");
    assert!((r.feed - 0.04).abs() < f64::EPSILON);
    assert!((r.kill - 0.06).abs() < f64::EPSILON);
    assert!(matches!(r.seed, crate::ast::SeedMode::Scatter(50)));
}

// ===================================================================
// Swarm block
// ===================================================================

#[test]
fn parse_swarm_block() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("physarum".into())),
        s(Token::LBrace),
        // swarm { agents: 50000, sensor_angle: 30, decay: 0.9, bounds: reflect }
        s(Token::Swarm),
        s(Token::LBrace),
        s(Token::Ident("agents".into())),
        s(Token::Colon),
        s(Token::Float(50000.0)),
        s(Token::Comma),
        s(Token::Ident("sensor_angle".into())),
        s(Token::Colon),
        s(Token::Float(30.0)),
        s(Token::Comma),
        s(Token::Ident("decay".into())),
        s(Token::Colon),
        s(Token::Float(0.9)),
        s(Token::Comma),
        s(Token::Ident("bounds".into())),
        s(Token::Colon),
        s(Token::Ident("reflect".into())),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse swarm");
    assert_eq!(prog.cinematics.len(), 1);
    let sw = prog.cinematics[0]
        .swarm
        .as_ref()
        .expect("swarm should be Some");
    assert_eq!(sw.agents, 50000);
    assert!((sw.sensor_angle - 30.0).abs() < f64::EPSILON);
    assert!((sw.decay - 0.9).abs() < f64::EPSILON);
    assert_eq!(sw.bounds, crate::ast::BoundsMode::Reflect);
}

// ===================================================================
// Flow block
// ===================================================================

#[test]
fn parse_flow_block() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("smoke".into())),
        s(Token::LBrace),
        // flow { type: vortex, scale: 5.0, octaves: 6, strength: 2.0 }
        s(Token::Flow),
        s(Token::LBrace),
        s(Token::Ident("type".into())),
        s(Token::Colon),
        s(Token::Ident("vortex".into())),
        s(Token::Comma),
        s(Token::Ident("scale".into())),
        s(Token::Colon),
        s(Token::Float(5.0)),
        s(Token::Comma),
        s(Token::Ident("octaves".into())),
        s(Token::Colon),
        s(Token::Float(6.0)),
        s(Token::Comma),
        s(Token::Ident("strength".into())),
        s(Token::Colon),
        s(Token::Float(2.0)),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse flow");
    assert_eq!(prog.cinematics.len(), 1);
    let f = prog.cinematics[0]
        .flow
        .as_ref()
        .expect("flow should be Some");
    assert_eq!(f.flow_type, crate::ast::FlowType::Vortex);
    assert!((f.scale - 5.0).abs() < f64::EPSILON);
    assert_eq!(f.octaves, 6);
    assert!((f.strength - 2.0).abs() < f64::EPSILON);
}

// ===================================================================
// Layer with blend mode
// ===================================================================

#[test]
fn parse_layer_with_blend_screen() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("t".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("fx".into())),
        s(Token::Blend),
        s(Token::Colon),
        s(Token::Ident("screen".into())),
        s(Token::LBrace),
        s(Token::Ident("color".into())),
        s(Token::Colon),
        s(Token::StringLit("red".into())),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse blend");
    assert_eq!(
        prog.cinematics[0].layers[0].blend,
        crate::ast::BlendMode::Screen
    );
}

#[test]
fn parse_layer_default_blend_is_add() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("t".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("bg".into())),
        s(Token::LBrace),
        s(Token::Ident("color".into())),
        s(Token::Colon),
        s(Token::StringLit("blue".into())),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse");
    assert_eq!(
        prog.cinematics[0].layers[0].blend,
        crate::ast::BlendMode::Add
    );
}

// ===================================================================
// Layer with opacity
// ===================================================================

#[test]
fn parse_layer_with_opacity() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("test".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("main".into())),
        s(Token::Opacity),
        s(Token::Colon),
        s(Token::Float(0.75)),
        s(Token::LBrace),
        s(Token::Ident("circle".into())),
        s(Token::LParen),
        s(Token::Float(0.2)),
        s(Token::RParen),
        s(Token::Pipe),
        s(Token::Ident("glow".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse opacity");
    assert_eq!(prog.cinematics[0].layers[0].opacity, Some(0.75));
}

#[test]
fn parse_layer_opacity_and_memory() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("test".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("main".into())),
        s(Token::Memory),
        s(Token::Colon),
        s(Token::Float(0.95)),
        s(Token::Opacity),
        s(Token::Colon),
        s(Token::Float(0.5)),
        s(Token::LBrace),
        s(Token::Ident("circle".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::Pipe),
        s(Token::Ident("glow".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse memory + opacity");
    assert_eq!(prog.cinematics[0].layers[0].memory, Some(0.95));
    assert_eq!(prog.cinematics[0].layers[0].opacity, Some(0.5));
}

#[test]
fn parse_layer_no_opacity_default() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("test".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("main".into())),
        s(Token::LBrace),
        s(Token::Ident("circle".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::Pipe),
        s(Token::Ident("glow".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse without opacity");
    assert_eq!(prog.cinematics[0].layers[0].opacity, None);
}

// ===================================================================
// v0.4 — fn definitions
// ===================================================================

#[test]
fn parse_fn_def_simple() {
    // fn petal(size) { circle(size) | glow(2.0) }
    let tokens = vec![
        s(Token::Fn),
        s(Token::Ident("petal".into())),
        s(Token::LParen),
        s(Token::Ident("size".into())),
        s(Token::RParen),
        s(Token::LBrace),
        s(Token::Ident("circle".into())),
        s(Token::LParen),
        s(Token::Ident("size".into())),
        s(Token::RParen),
        s(Token::Pipe),
        s(Token::Ident("glow".into())),
        s(Token::LParen),
        s(Token::Float(2.0)),
        s(Token::RParen),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse fn def");
    assert_eq!(prog.fns.len(), 1);
    assert_eq!(prog.fns[0].name, "petal");
    assert_eq!(prog.fns[0].params, vec!["size"]);
    assert_eq!(prog.fns[0].body.len(), 2);
    assert_eq!(prog.fns[0].body[0].name, "circle");
    assert_eq!(prog.fns[0].body[1].name, "glow");
}

#[test]
fn parse_fn_def_multi_params() {
    // fn tinted(r, g, b) { circle(0.3) | tint(r, g, b) }
    let tokens = vec![
        s(Token::Fn),
        s(Token::Ident("tinted".into())),
        s(Token::LParen),
        s(Token::Ident("r".into())),
        s(Token::Comma),
        s(Token::Ident("g".into())),
        s(Token::Comma),
        s(Token::Ident("b".into())),
        s(Token::RParen),
        s(Token::LBrace),
        s(Token::Ident("circle".into())),
        s(Token::LParen),
        s(Token::Float(0.3)),
        s(Token::RParen),
        s(Token::Pipe),
        s(Token::Ident("tint".into())),
        s(Token::LParen),
        s(Token::Ident("r".into())),
        s(Token::Comma),
        s(Token::Ident("g".into())),
        s(Token::Comma),
        s(Token::Ident("b".into())),
        s(Token::RParen),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse fn with multi params");
    assert_eq!(prog.fns[0].params, vec!["r", "g", "b"]);
    assert_eq!(prog.fns[0].body[1].args.len(), 3);
}

#[test]
fn parse_fn_no_params() {
    // fn dot() { circle(0.1) }
    let tokens = vec![
        s(Token::Fn),
        s(Token::Ident("dot".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::LBrace),
        s(Token::Ident("circle".into())),
        s(Token::LParen),
        s(Token::Float(0.1)),
        s(Token::RParen),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse fn with no params");
    assert_eq!(prog.fns[0].params.len(), 0);
    assert_eq!(prog.fns[0].body.len(), 1);
}

// ===================================================================
// v0.4 — conditional layers
// ===================================================================

#[test]
fn parse_conditional_layer() {
    // cinematic "t" { layer x { if bass > 0.5 { circle() } else { ring() } } }
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("t".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("x".into())),
        s(Token::LBrace),
        s(Token::If),
        s(Token::Ident("bass".into())),
        s(Token::Gt),
        s(Token::Float(0.5)),
        s(Token::LBrace),
        s(Token::Ident("circle".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::Else),
        s(Token::LBrace),
        s(Token::Ident("ring".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse conditional layer");
    let layer = &prog.cinematics[0].layers[0];
    match &layer.body {
        LayerBody::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            assert_eq!(then_branch.len(), 1);
            assert_eq!(then_branch[0].name, "circle");
            assert_eq!(else_branch.len(), 1);
            assert_eq!(else_branch[0].name, "ring");
            // Check condition is a BinOp::Gt
            match condition {
                Expr::BinOp { op, .. } => assert_eq!(*op, BinOp::Gt),
                _ => panic!("expected BinOp condition"),
            }
        }
        _ => panic!("expected Conditional body"),
    }
}

#[test]
fn parse_conditional_multi_stage_branches() {
    // if energy > 0.8 { circle() | glow() | tint() } else { ring() | glow() | tint() }
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("t".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("a".into())),
        s(Token::LBrace),
        s(Token::If),
        s(Token::Ident("energy".into())),
        s(Token::Gt),
        s(Token::Float(0.8)),
        s(Token::LBrace),
        s(Token::Ident("circle".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::Pipe),
        s(Token::Ident("glow".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::Pipe),
        s(Token::Ident("tint".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::Else),
        s(Token::LBrace),
        s(Token::Ident("ring".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::Pipe),
        s(Token::Ident("glow".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::Pipe),
        s(Token::Ident("tint".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse multi-stage conditional");
    match &prog.cinematics[0].layers[0].body {
        LayerBody::Conditional {
            then_branch,
            else_branch,
            ..
        } => {
            assert_eq!(then_branch.len(), 3);
            assert_eq!(else_branch.len(), 3);
        }
        _ => panic!("expected Conditional body"),
    }
}

// ===================================================================
// v0.4 — use imports
// ===================================================================

#[test]
fn parse_use_import_basic() {
    // use "shapes.game"
    let tokens = vec![s(Token::Use), s(Token::StringLit("shapes.game".into()))];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse use import");
    assert_eq!(prog.imports.len(), 1);
    assert_eq!(prog.imports[0].path, "shapes.game");
    assert_eq!(prog.imports[0].alias, "shapes");
}

#[test]
fn parse_use_import_with_path() {
    // use "lib/palettes.game"
    let tokens = vec![
        s(Token::Use),
        s(Token::StringLit("lib/palettes.game".into())),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse use import with path");
    assert_eq!(prog.imports[0].alias, "palettes");
}

// ===================================================================
// v0.4 — comparison operators in expressions
// ===================================================================

#[test]
fn parse_comparison_operators() {
    // Expression: x > 0.5
    let tokens = vec![
        s(Token::Fn),
        s(Token::Ident("test".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::LBrace),
        s(Token::Ident("circle".into())),
        s(Token::LParen),
        s(Token::Ident("x".into())),
        s(Token::Gt),
        s(Token::Float(0.5)),
        s(Token::RParen),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse comparison in arg");
    let arg = &prog.fns[0].body[0].args[0].value;
    match arg {
        Expr::BinOp { op, .. } => assert_eq!(*op, BinOp::Gt),
        _ => panic!("expected BinOp"),
    }
}

#[test]
fn parse_all_comparison_ops() {
    // Test each operator parses correctly
    for (tok, expected_op) in [
        (Token::Gt, BinOp::Gt),
        (Token::Lt, BinOp::Lt),
        (Token::Gte, BinOp::Gte),
        (Token::Lte, BinOp::Lte),
        (Token::EqEq, BinOp::Eq),
        (Token::NotEq, BinOp::NotEq),
    ] {
        let tokens = vec![
            s(Token::Fn),
            s(Token::Ident("t".into())),
            s(Token::LParen),
            s(Token::RParen),
            s(Token::LBrace),
            s(Token::Ident("circle".into())),
            s(Token::LParen),
            s(Token::Ident("a".into())),
            s(tok),
            s(Token::Float(1.0)),
            s(Token::RParen),
            s(Token::RBrace),
        ];
        let mut p = Parser::new(tokens);
        let prog = p.parse().expect("should parse comparison");
        match &prog.fns[0].body[0].args[0].value {
            Expr::BinOp { op, .. } => assert_eq!(*op, expected_op),
            _ => panic!("expected BinOp"),
        }
    }
}

// ===================================================================
// v0.4 — fn + cinematic together
// ===================================================================

#[test]
fn parse_fn_and_cinematic() {
    // fn dot() { circle(0.1) }
    // cinematic "t" { layer main { dot() } }
    let tokens = vec![
        s(Token::Fn),
        s(Token::Ident("dot".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::LBrace),
        s(Token::Ident("circle".into())),
        s(Token::LParen),
        s(Token::Float(0.1)),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::Cinematic),
        s(Token::StringLit("t".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("main".into())),
        s(Token::LBrace),
        s(Token::Ident("dot".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse fn + cinematic");
    assert_eq!(prog.fns.len(), 1);
    assert_eq!(prog.cinematics.len(), 1);
    assert_eq!(prog.cinematics[0].layers[0].name, "main");
    match &prog.cinematics[0].layers[0].body {
        LayerBody::Pipeline(stages) => {
            assert_eq!(stages[0].name, "dot");
        }
        _ => panic!("expected Pipeline"),
    }
}

// ===================================================================
// IFS block parsing
// ===================================================================

#[test]
fn parse_ifs_block_basic() {
    let tokens = vec![
        s(Token::Ident("ifs".into())),
        s(Token::LBrace),
        s(Token::Ident("transform".into())),
        s(Token::Ident("t1".into())),
        s(Token::LBracket),
        s(Token::Float(0.5)),
        s(Token::Comma),
        s(Token::Float(0.0)),
        s(Token::Comma),
        s(Token::Float(0.0)),
        s(Token::Comma),
        s(Token::Float(0.5)),
        s(Token::Comma),
        s(Token::Float(0.0)),
        s(Token::Comma),
        s(Token::Float(0.0)),
        s(Token::RBracket),
        s(Token::Ident("weight".into())),
        s(Token::Float(0.33)),
        s(Token::Pipe),
        s(Token::Ident("iterations".into())),
        s(Token::Integer(50000)),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse ifs block");
    assert_eq!(prog.ifs_blocks.len(), 1);
    assert_eq!(prog.ifs_blocks[0].transforms.len(), 1);
    assert_eq!(prog.ifs_blocks[0].transforms[0].name, "t1");
    assert!((prog.ifs_blocks[0].transforms[0].weight - 0.33).abs() < 0.01);
    assert_eq!(prog.ifs_blocks[0].iterations, 50000);
}

#[test]
fn parse_ifs_color_mode() {
    let tokens = vec![
        s(Token::Ident("ifs".into())),
        s(Token::LBrace),
        s(Token::Ident("color".into())),
        s(Token::Ident("depth".into())),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse ifs color mode");
    assert_eq!(prog.ifs_blocks[0].color_mode, IfsColorMode::Depth);
}

#[test]
fn parse_ifs_multiple_transforms() {
    let tokens = vec![
        s(Token::Ident("ifs".into())),
        s(Token::LBrace),
        s(Token::Ident("transform".into())),
        s(Token::Ident("a".into())),
        s(Token::LBracket),
        s(Token::Float(0.5)),
        s(Token::Comma),
        s(Token::Float(0.0)),
        s(Token::Comma),
        s(Token::Float(0.0)),
        s(Token::Comma),
        s(Token::Float(0.5)),
        s(Token::Comma),
        s(Token::Float(0.0)),
        s(Token::Comma),
        s(Token::Float(0.0)),
        s(Token::RBracket),
        s(Token::Pipe),
        s(Token::Ident("transform".into())),
        s(Token::Ident("b".into())),
        s(Token::LBracket),
        s(Token::Float(0.5)),
        s(Token::Comma),
        s(Token::Float(0.0)),
        s(Token::Comma),
        s(Token::Float(0.0)),
        s(Token::Comma),
        s(Token::Float(0.5)),
        s(Token::Comma),
        s(Token::Float(0.25)),
        s(Token::Comma),
        s(Token::Float(0.5)),
        s(Token::RBracket),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse multiple transforms");
    assert_eq!(prog.ifs_blocks[0].transforms.len(), 2);
    assert_eq!(prog.ifs_blocks[0].transforms[1].name, "b");
}

// ===================================================================
// L-system block parsing
// ===================================================================

#[test]
fn parse_lsystem_block_basic() {
    let tokens = vec![
        s(Token::Ident("lsystem".into())),
        s(Token::LBrace),
        s(Token::Ident("axiom".into())),
        s(Token::StringLit("F".into())),
        s(Token::Pipe),
        s(Token::Ident("rule".into())),
        s(Token::Ident("F".into())),
        s(Token::Arrow),
        s(Token::StringLit("F+F-F-F+F".into())),
        s(Token::Pipe),
        s(Token::Ident("angle".into())),
        s(Token::Integer(90)),
        s(Token::Pipe),
        s(Token::Ident("iterations".into())),
        s(Token::Integer(4)),
        s(Token::Pipe),
        s(Token::Ident("step".into())),
        s(Token::Float(0.01)),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse lsystem block");
    assert_eq!(prog.lsystem_blocks.len(), 1);
    assert_eq!(prog.lsystem_blocks[0].axiom, "F");
    assert_eq!(prog.lsystem_blocks[0].rules.len(), 1);
    assert_eq!(prog.lsystem_blocks[0].rules[0].symbol, 'F');
    assert_eq!(prog.lsystem_blocks[0].rules[0].replacement, "F+F-F-F+F");
    assert!((prog.lsystem_blocks[0].angle - 90.0).abs() < 0.01);
    assert_eq!(prog.lsystem_blocks[0].iterations, 4);
}

#[test]
fn parse_lsystem_multiple_rules() {
    let tokens = vec![
        s(Token::Ident("lsystem".into())),
        s(Token::LBrace),
        s(Token::Ident("axiom".into())),
        s(Token::StringLit("A".into())),
        s(Token::Pipe),
        s(Token::Ident("rule".into())),
        s(Token::Ident("A".into())),
        s(Token::Arrow),
        s(Token::StringLit("AB".into())),
        s(Token::Pipe),
        s(Token::Ident("rule".into())),
        s(Token::Ident("B".into())),
        s(Token::Arrow),
        s(Token::StringLit("A".into())),
        s(Token::Pipe),
        s(Token::Ident("angle".into())),
        s(Token::Integer(60)),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse multiple rules");
    assert_eq!(prog.lsystem_blocks[0].rules.len(), 2);
    assert_eq!(prog.lsystem_blocks[0].rules[0].symbol, 'A');
    assert_eq!(prog.lsystem_blocks[0].rules[1].symbol, 'B');
}

// ===================================================================
// Automaton block parsing
// ===================================================================

#[test]
fn parse_automaton_block_basic() {
    let tokens = vec![
        s(Token::Ident("automaton".into())),
        s(Token::LBrace),
        s(Token::Ident("states".into())),
        s(Token::Integer(2)),
        s(Token::Pipe),
        s(Token::Ident("neighborhood".into())),
        s(Token::Ident("moore".into())),
        s(Token::Pipe),
        s(Token::Ident("rule".into())),
        s(Token::StringLit("B3/S23".into())),
        s(Token::Pipe),
        s(Token::Ident("seed".into())),
        s(Token::Ident("random".into())),
        s(Token::Float(0.3)),
        s(Token::Pipe),
        s(Token::Ident("speed".into())),
        s(Token::Integer(10)),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse automaton block");
    assert_eq!(prog.automaton_blocks.len(), 1);
    assert_eq!(prog.automaton_blocks[0].states, 2);
    assert_eq!(prog.automaton_blocks[0].neighborhood, Neighborhood::Moore);
    assert_eq!(prog.automaton_blocks[0].rule, "B3/S23");
    assert_eq!(prog.automaton_blocks[0].speed, 10);
}

#[test]
fn parse_automaton_vonneumann() {
    let tokens = vec![
        s(Token::Ident("automaton".into())),
        s(Token::LBrace),
        s(Token::Ident("neighborhood".into())),
        s(Token::Ident("vonneumann".into())),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse vonneumann");
    assert_eq!(
        prog.automaton_blocks[0].neighborhood,
        Neighborhood::VonNeumann
    );
}

#[test]
fn parse_automaton_seed_center() {
    let tokens = vec![
        s(Token::Ident("automaton".into())),
        s(Token::LBrace),
        s(Token::Ident("seed".into())),
        s(Token::Ident("center".into())),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse center seed");
    assert_eq!(prog.automaton_blocks[0].seed, AutomatonSeed::Center);
}

#[test]
fn parse_automaton_seed_pattern() {
    let tokens = vec![
        s(Token::Ident("automaton".into())),
        s(Token::LBrace),
        s(Token::Ident("seed".into())),
        s(Token::Ident("pattern".into())),
        s(Token::StringLit("glider".into())),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse pattern seed");
    assert_eq!(
        prog.automaton_blocks[0].seed,
        AutomatonSeed::Pattern("glider".into())
    );
}

// ===================================================================
// End-to-end: mixed v0.6 blocks with cinematics
// ===================================================================

#[test]
fn parse_mixed_cinematic_and_ifs() {
    let tokens = vec![
        // cinematic
        s(Token::Cinematic),
        s(Token::StringLit("test".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("bg".into())),
        s(Token::LBrace),
        s(Token::Ident("circle".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::RBrace),
        // ifs
        s(Token::Ident("ifs".into())),
        s(Token::LBrace),
        s(Token::Ident("iterations".into())),
        s(Token::Integer(10000)),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse cinematic + ifs");
    assert_eq!(prog.cinematics.len(), 1);
    assert_eq!(prog.ifs_blocks.len(), 1);
}

// ===================================================================
// Scene block with pipe separators
// ===================================================================

#[test]
fn parse_scene_with_pipe_separators() {
    let tokens = vec![
        s(Token::Scene),
        s(Token::StringLit("show".into())),
        s(Token::LBrace),
        s(Token::Play),
        s(Token::StringLit("intro".into())),
        s(Token::For),
        s(Token::Seconds(5.0)),
        s(Token::Pipe),
        s(Token::Transition),
        s(Token::Ident("dissolve".into())),
        s(Token::Over),
        s(Token::Seconds(2.0)),
        s(Token::Pipe),
        s(Token::Play),
        s(Token::StringLit("main".into())),
        s(Token::For),
        s(Token::Seconds(10.0)),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse scene with pipes");
    assert_eq!(prog.scenes.len(), 1);
    assert_eq!(prog.scenes[0].entries.len(), 3);
}

// ===================================================================
// Pass block in cinematic
// ===================================================================

#[test]
fn parse_pass_block_in_cinematic() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("fx".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("bg".into())),
        s(Token::LBrace),
        s(Token::Ident("circle".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::Pass),
        s(Token::Ident("blur_pass".into())),
        s(Token::LBrace),
        s(Token::Ident("blur".into())),
        s(Token::LParen),
        s(Token::Float(2.0)),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse pass block");
    assert_eq!(prog.cinematics[0].passes.len(), 1);
    assert_eq!(prog.cinematics[0].passes[0].name, "blur_pass");
    assert_eq!(prog.cinematics[0].passes[0].body.len(), 1);
}

// ===================================================================
// Feedback layer
// ===================================================================

#[test]
fn parse_feedback_layer() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("trail".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("bg".into())),
        s(Token::Feedback),
        s(Token::Colon),
        s(Token::Ident("true".into())),
        s(Token::LBrace),
        s(Token::Ident("circle".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse feedback layer");
    assert!(prog.cinematics[0].layers[0].feedback);
}

// ===================================================================
// Phase 1 fixes: keyword-as-field-name, easing, parameterless stages
// ===================================================================

#[test]
fn parse_keyword_as_field_name_in_resonate() {
    // bass -> outer_ring.opacity * 0.7
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("test".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("bg".into())),
        s(Token::LBrace),
        s(Token::Ident("circle".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::Pipe),
        s(Token::Ident("glow".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::Resonate),
        s(Token::LBrace),
        s(Token::Ident("bass".into())),
        s(Token::Arrow),
        s(Token::Ident("outer_ring".into())),
        s(Token::Dot),
        s(Token::Opacity), // keyword used as field name
        s(Token::Star),
        s(Token::Float(0.7)),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse keyword as field name");
    assert_eq!(prog.cinematics[0].resonates[0].entries[0].field, "opacity");
}

#[test]
fn parse_keyword_blend_as_field_name() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("test".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("bg".into())),
        s(Token::LBrace),
        s(Token::Ident("circle".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::Pipe),
        s(Token::Ident("glow".into())),
        s(Token::LParen),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::Resonate),
        s(Token::LBrace),
        s(Token::Ident("src".into())),
        s(Token::Arrow),
        s(Token::Ident("tgt".into())),
        s(Token::Dot),
        s(Token::Blend), // keyword used as field name
        s(Token::Star),
        s(Token::Float(0.5)),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse blend as field name");
    assert_eq!(prog.cinematics[0].resonates[0].entries[0].field, "blend");
}

#[test]
fn parse_hyphenated_easing_ease_out() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("test".into())),
        s(Token::LBrace),
        s(Token::Arc),
        s(Token::LBrace),
        s(Token::Ident("scale".into())),
        s(Token::Colon),
        s(Token::Float(0.1)),
        s(Token::Arrow),
        s(Token::Float(1.0)),
        s(Token::Over),
        s(Token::Seconds(5.0)),
        s(Token::Ident("ease".into())),
        s(Token::Minus),
        s(Token::Ident("out".into())),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse ease-out");
    assert_eq!(
        prog.cinematics[0].arcs[0].entries[0].easing.as_deref(),
        Some("ease-out")
    );
}

#[test]
fn parse_hyphenated_easing_ease_in_out() {
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("test".into())),
        s(Token::LBrace),
        s(Token::Arc),
        s(Token::LBrace),
        s(Token::Ident("scale".into())),
        s(Token::Colon),
        s(Token::Float(0.0)),
        s(Token::Arrow),
        s(Token::Float(1.0)),
        s(Token::Over),
        s(Token::Seconds(8.0)),
        s(Token::Ident("ease".into())),
        s(Token::Minus),
        s(Token::Ident("in".into())),
        s(Token::Minus),
        s(Token::Ident("out".into())),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse ease-in-out");
    assert_eq!(
        prog.cinematics[0].arcs[0].entries[0].easing.as_deref(),
        Some("ease-in-out")
    );
}

#[test]
fn parse_parameterless_stage_in_pipeline() {
    // layer main { polar | glow(1.5) }
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("test".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("main".into())),
        s(Token::LBrace),
        s(Token::Ident("polar".into())),
        s(Token::Pipe),
        s(Token::Ident("glow".into())),
        s(Token::LParen),
        s(Token::Float(1.5)),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse parameterless stage");
    match &prog.cinematics[0].layers[0].body {
        LayerBody::Pipeline(stages) => {
            assert_eq!(stages.len(), 2);
            assert_eq!(stages[0].name, "polar");
            assert!(stages[0].args.is_empty());
            assert_eq!(stages[1].name, "glow");
        }
        _ => panic!("expected Pipeline"),
    }
}

#[test]
fn parse_single_bare_stage() {
    // layer main { polar }
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("test".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("main".into())),
        s(Token::LBrace),
        s(Token::Ident("polar".into())),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse single bare stage");
    match &prog.cinematics[0].layers[0].body {
        LayerBody::Pipeline(stages) => {
            assert_eq!(stages.len(), 1);
            assert_eq!(stages[0].name, "polar");
            assert!(stages[0].args.is_empty());
        }
        _ => panic!("expected Pipeline"),
    }
}

#[test]
fn parse_mixed_parameterless_and_normal_stages() {
    // layer main { distort | circle(0.3) | glow(2.0) }
    let tokens = vec![
        s(Token::Cinematic),
        s(Token::StringLit("test".into())),
        s(Token::LBrace),
        s(Token::Layer),
        s(Token::Ident("main".into())),
        s(Token::LBrace),
        s(Token::Ident("distort".into())),
        s(Token::Pipe),
        s(Token::Ident("circle".into())),
        s(Token::LParen),
        s(Token::Float(0.3)),
        s(Token::RParen),
        s(Token::Pipe),
        s(Token::Ident("glow".into())),
        s(Token::LParen),
        s(Token::Float(2.0)),
        s(Token::RParen),
        s(Token::RBrace),
        s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse mixed stages");
    match &prog.cinematics[0].layers[0].body {
        LayerBody::Pipeline(stages) => {
            assert_eq!(stages.len(), 3);
            assert_eq!(stages[0].name, "distort");
            assert!(stages[0].args.is_empty());
            assert_eq!(stages[1].name, "circle");
            assert_eq!(stages[1].args.len(), 1);
        }
        _ => panic!("expected Pipeline"),
    }
}

// ===================================================================
// AI-resilient parsing — semicolons, keywords as names, expressions
// ===================================================================

#[test]
fn parse_semicolons_in_config() {
    let source = r#"
        cinematic "test" {
            layer config {
                pulse: 0.5;
                drift: 0.3;
            }
            layer bg { circle(0.3) | glow(2.0) }
        }
    "#;
    let prog = crate::compile_to_ast(source).expect("semicolons in config should parse");
    assert_eq!(prog.cinematics.len(), 1);
    assert_eq!(prog.cinematics[0].layers.len(), 2);
    assert_eq!(prog.cinematics[0].layers[0].name, "config");
    match &prog.cinematics[0].layers[0].body {
        LayerBody::Params(params) => {
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].name, "pulse");
            assert_eq!(params[1].name, "drift");
        }
        _ => panic!("expected Params body for config layer"),
    }
}

#[test]
fn parse_keyword_as_layer_name() {
    let source = r#"
        cinematic "test" {
            layer flow memory: 0.92 { circle(0.3) | glow(2.0) }
            layer matrix { ring(0.4, 0.02) | glow(1.5) }
            layer blend opacity: 0.5 { fbm(scale: 3.0, octaves: 5) | palette(fire) }
        }
    "#;
    let prog = crate::compile_to_ast(source).expect("keywords as layer names should parse");
    assert_eq!(prog.cinematics[0].layers.len(), 3);
    assert_eq!(prog.cinematics[0].layers[0].name, "flow");
    assert_eq!(prog.cinematics[0].layers[1].name, "matrix");
    assert_eq!(prog.cinematics[0].layers[2].name, "blend");
}

#[test]
fn parse_keyword_as_config_param() {
    let source = r#"
        cinematic "test" {
            layer config {
                flow: 1.0
                gravity: 0.5
                score: 0.8
            }
            layer bg { circle(0.3) | glow(2.0) }
        }
    "#;
    let prog = crate::compile_to_ast(source).expect("keywords as config params should parse");
    let config = &prog.cinematics[0].layers[0];
    assert_eq!(config.name, "config");
    match &config.body {
        LayerBody::Params(params) => {
            assert_eq!(params.len(), 3);
            assert_eq!(params[0].name, "flow");
            assert_eq!(params[1].name, "gravity");
            assert_eq!(params[2].name, "score");
        }
        _ => panic!("expected Params body for config layer"),
    }
}

#[test]
fn parse_expressions_in_stage_args() {
    let source = r#"
        cinematic "test" {
            layer config { pulse: 0.5 }
            layer core {
                translate(mouse_x * 2.0 - 1.0, mouse_y * 2.0 - 1.0)
                | circle(0.08 + mouse_down * 0.12)
                | glow(3.2 + pulse * 1.5)
                | tint(1.0, 0.6, 0.2)
            }
        }
    "#;
    let prog = crate::compile_to_ast(source).expect("expressions in stage args should parse");
    assert_eq!(prog.cinematics[0].layers.len(), 2);
    let core = &prog.cinematics[0].layers[1];
    assert_eq!(core.name, "core");
    match &core.body {
        LayerBody::Pipeline(stages) => {
            assert_eq!(stages.len(), 4);
            assert_eq!(stages[0].name, "translate");
            assert_eq!(stages[0].args.len(), 2);
            assert_eq!(stages[1].name, "circle");
            assert_eq!(stages[1].args.len(), 1);
            assert_eq!(stages[2].name, "glow");
            assert_eq!(stages[2].args.len(), 1);
            assert_eq!(stages[3].name, "tint");
            assert_eq!(stages[3].args.len(), 3);
        }
        _ => panic!("expected Pipeline body for core layer"),
    }
}

#[test]
fn parse_keyword_in_expression() {
    let source = r#"
        cinematic "test" {
            layer config { flow: 1.0 }
            layer core { circle(flow * 0.1) | glow(2.0) }
        }
    "#;
    let prog = crate::compile_to_ast(source).expect("keyword in expression should parse");
    assert_eq!(prog.cinematics[0].layers.len(), 2);
    let core = &prog.cinematics[0].layers[1];
    assert_eq!(core.name, "core");
    match &core.body {
        LayerBody::Pipeline(stages) => {
            assert_eq!(stages.len(), 2);
            assert_eq!(stages[0].name, "circle");
            assert_eq!(stages[0].args.len(), 1);
            // The first arg should contain a BinOp with 'flow' as an ident
            match &stages[0].args[0].value {
                Expr::BinOp { op, left, .. } => {
                    assert_eq!(*op, BinOp::Mul);
                    assert!(matches!(left.as_ref(), Expr::Ident(name) if name == "flow"));
                }
                other => panic!("expected BinOp, got {:?}", other),
            }
        }
        _ => panic!("expected Pipeline body for core layer"),
    }
}
