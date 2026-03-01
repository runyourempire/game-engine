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
        Expr::BinOp { op: BinOp::Add, left, right } => {
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
        Expr::BinOp { op: BinOp::Pow, right, .. } => {
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
        s(Token::Cinematic), s(Token::StringLit("t".into())), s(Token::LBrace),
        s(Token::Layer), s(Token::Ident("bg".into())), s(Token::LBrace),
        s(Token::Ident("bass".into())), s(Token::Colon), s(Token::Float(0.5)),
        s(Token::Tilde), s(Token::Ident("audio".into())), s(Token::Dot),
        s(Token::Ident("bass".into())),
        s(Token::ShiftRight), s(Token::Millis(200.0)),
        s(Token::RBrace), s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse temporal delay");
    match &prog.cinematics[0].layers[0].body {
        LayerBody::Params(params) => {
            assert_eq!(params[0].temporal_ops.len(), 1);
            assert!(matches!(&params[0].temporal_ops[0], TemporalOp::Delay(Duration::Millis(v)) if (*v - 200.0).abs() < f64::EPSILON));
        }
        _ => panic!("expected params"),
    }
}

#[test]
fn parse_temporal_smooth() {
    let tokens = vec![
        s(Token::Cinematic), s(Token::StringLit("t".into())), s(Token::LBrace),
        s(Token::Layer), s(Token::Ident("bg".into())), s(Token::LBrace),
        s(Token::Ident("val".into())), s(Token::Colon), s(Token::Float(0.5)),
        s(Token::Diamond), s(Token::Millis(50.0)),
        s(Token::RBrace), s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse temporal smooth");
    match &prog.cinematics[0].layers[0].body {
        LayerBody::Params(params) => {
            assert_eq!(params[0].temporal_ops.len(), 1);
            assert!(matches!(&params[0].temporal_ops[0], TemporalOp::Smooth(Duration::Millis(v)) if (*v - 50.0).abs() < f64::EPSILON));
        }
        _ => panic!("expected params"),
    }
}

#[test]
fn parse_temporal_trigger() {
    let tokens = vec![
        s(Token::Cinematic), s(Token::StringLit("t".into())), s(Token::LBrace),
        s(Token::Layer), s(Token::Ident("bg".into())), s(Token::LBrace),
        s(Token::Ident("beat".into())), s(Token::Colon), s(Token::Float(0.0)),
        s(Token::Tilde), s(Token::Ident("audio".into())), s(Token::Dot),
        s(Token::Ident("beat".into())),
        s(Token::BangBang), s(Token::Millis(300.0)),
        s(Token::RBrace), s(Token::RBrace),
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
        s(Token::Cinematic), s(Token::StringLit("t".into())), s(Token::LBrace),
        s(Token::Layer), s(Token::Ident("bg".into())), s(Token::LBrace),
        s(Token::Ident("energy".into())), s(Token::Colon), s(Token::Float(0.5)),
        s(Token::DotDot), s(Token::LBracket),
        s(Token::Float(0.1)), s(Token::Comma), s(Token::Float(0.9)),
        s(Token::RBracket),
        s(Token::RBrace), s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse temporal range");
    match &prog.cinematics[0].layers[0].body {
        LayerBody::Params(params) => {
            assert_eq!(params[0].temporal_ops.len(), 1);
            assert!(matches!(&params[0].temporal_ops[0], TemporalOp::Range(_, _)));
        }
        _ => panic!("expected params"),
    }
}

#[test]
fn parse_chained_temporal_ops() {
    // bass: 0.5 ~ audio.bass <> 50ms >> 200ms .. [0.0, 1.0]
    let tokens = vec![
        s(Token::Cinematic), s(Token::StringLit("t".into())), s(Token::LBrace),
        s(Token::Layer), s(Token::Ident("bg".into())), s(Token::LBrace),
        s(Token::Ident("bass".into())), s(Token::Colon), s(Token::Float(0.5)),
        s(Token::Tilde), s(Token::Ident("audio".into())), s(Token::Dot),
        s(Token::Ident("bass".into())),
        s(Token::Diamond), s(Token::Millis(50.0)),
        s(Token::ShiftRight), s(Token::Millis(200.0)),
        s(Token::DotDot), s(Token::LBracket),
        s(Token::Float(0.0)), s(Token::Comma), s(Token::Float(1.0)),
        s(Token::RBracket),
        s(Token::RBrace), s(Token::RBrace),
    ];
    let mut p = Parser::new(tokens);
    let prog = p.parse().expect("should parse chained temporal ops");
    match &prog.cinematics[0].layers[0].body {
        LayerBody::Params(params) => {
            assert_eq!(params[0].temporal_ops.len(), 3);
            assert!(matches!(&params[0].temporal_ops[0], TemporalOp::Smooth(_)));
            assert!(matches!(&params[0].temporal_ops[1], TemporalOp::Delay(_)));
            assert!(matches!(&params[0].temporal_ops[2], TemporalOp::Range(_, _)));
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
        s(Token::Cinematic), s(Token::StringLit("t".into())), s(Token::LBrace),
        s(Token::Listen), s(Token::LBrace),
        s(Token::Ident("onset".into())), s(Token::Colon),
        s(Token::Ident("attack".into())),
        s(Token::LParen),
        s(Token::Ident("threshold".into())), s(Token::Colon), s(Token::Float(0.7)),
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
        s(Token::Cinematic), s(Token::StringLit("t".into())), s(Token::LBrace),
        s(Token::Voice), s(Token::LBrace),
        s(Token::Ident("tone".into())), s(Token::Colon),
        s(Token::Ident("sine".into())),
        s(Token::LParen),
        s(Token::Ident("freq".into())), s(Token::Colon), s(Token::Integer(440)),
        s(Token::RParen),
        s(Token::Ident("filt".into())), s(Token::Colon),
        s(Token::Ident("lowpass".into())),
        s(Token::LParen),
        s(Token::Ident("cutoff".into())), s(Token::Colon), s(Token::Integer(2000)),
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
        s(Token::Cinematic), s(Token::StringLit("t".into())), s(Token::LBrace),
        s(Token::Score),
        s(Token::Ident("tempo".into())), s(Token::LParen), s(Token::Integer(120)), s(Token::RParen),
        s(Token::LBrace),
        // motif rise { scale: 0 -> 1 over 4bars }
        s(Token::Ident("motif".into())), s(Token::Ident("rise".into())), s(Token::LBrace),
        s(Token::Ident("scale".into())), s(Token::Colon),
        s(Token::Integer(0)), s(Token::Arrow), s(Token::Integer(1)),
        s(Token::Over), s(Token::Bars(4)),
        s(Token::RBrace),
        // phrase build = rise
        s(Token::Ident("phrase".into())), s(Token::Ident("build".into())), s(Token::Eq),
        s(Token::Ident("rise".into())),
        // arrange: build
        s(Token::Ident("arrange".into())), s(Token::Colon),
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
    let g = prog.cinematics[0].gravity.as_ref().expect("gravity should be Some");
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
