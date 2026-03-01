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
