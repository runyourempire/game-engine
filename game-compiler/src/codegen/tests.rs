use super::*;
use crate::lexer;
use crate::parser::Parser;

fn compile(src: &str) -> String {
    let tokens = lexer::lex(src).expect("lex failed");
    let mut parser = Parser::new(tokens);
    let cin = parser.parse().expect("parse failed");
    generate_wgsl(&cin).expect("codegen failed")
}

fn compile_full_output(src: &str) -> CompileOutput {
    let tokens = lexer::lex(src).expect("lex failed");
    let mut parser = Parser::new(tokens);
    let cin = parser.parse().expect("parse failed");
    generate_full(&cin).expect("codegen failed")
}

#[test]
fn codegen_hello_game() {
    let wgsl = compile(
        r#"cinematic "Hello" {
            layer {
                fn: circle(0.3 + sin(time) * 0.05) | glow(2.0)
            }
        }"#,
    );

    assert!(wgsl.contains("struct Uniforms"));
    assert!(wgsl.contains("fn vs_main"));
    assert!(wgsl.contains("fn fs_main"));
    assert!(wgsl.contains("sdf_circle"));
    assert!(wgsl.contains("apply_glow"));
    assert!(wgsl.contains("(0.3 + (sin(time) * 0.05))"));
    assert!(wgsl.contains("2.0"));
}

#[test]
fn codegen_produces_valid_structure() {
    let wgsl = compile("cinematic { layer { fn: circle(0.5) | glow(1.0) } }");

    let uni_pos = wgsl.find("struct Uniforms").unwrap();
    let vs_pos = wgsl.find("fn vs_main").unwrap();
    let fs_pos = wgsl.find("fn fs_main").unwrap();
    assert!(uni_pos < vs_pos);
    assert!(vs_pos < fs_pos);
}

#[test]
fn codegen_audio_uniforms_present() {
    let wgsl = compile("cinematic { layer { fn: circle(0.5) } }");
    assert!(wgsl.contains("audio_bass: f32"));
    assert!(wgsl.contains("audio_energy: f32"));
    assert!(wgsl.contains("resolution: vec2f"));
    assert!(wgsl.contains("mouse: vec2f"));
}

#[test]
fn codegen_params_collected() {
    let out = compile_full_output(
        r#"cinematic {
            layer x {
                fn: circle(radius)
                radius: 0.3 ~ audio.bass * 0.2
                intensity: 2.0 ~ audio.energy * 3.0
            }
        }"#,
    );

    assert_eq!(out.params.len(), 2);
    assert_eq!(out.params[0].name, "radius");
    assert_eq!(out.params[0].base_value, 0.3);
    assert!(out.params[0].mod_js.is_some());
    assert_eq!(out.params[1].name, "intensity");
    assert!(out.uses_audio);
}

#[test]
fn codegen_param_uniform_emitted() {
    let wgsl = compile(
        r#"cinematic {
            layer x {
                fn: circle(radius)
                radius: 0.3 ~ audio.bass * 0.2
            }
        }"#,
    );

    // Param should appear in uniform struct
    assert!(wgsl.contains("p_radius: f32"));
    // Param should be bound as let in fragment
    assert!(wgsl.contains("let radius = u.p_radius;"));
}

#[test]
fn codegen_js_expression() {
    let expr = Expr::BinaryOp {
        left: Box::new(Expr::FieldAccess {
            object: Box::new(Expr::Ident("audio".to_string())),
            field: "bass".to_string(),
        }),
        op: BinOp::Mul,
        right: Box::new(Expr::Number(2.0)),
    };
    let js = compile_expr_js(&expr);
    assert_eq!(js, "(audioBass * 2.0)");
}

#[test]
fn codegen_fbm_with_persistence() {
    let wgsl = compile(
        r#"cinematic {
            layer { fn: fbm(p, octaves: 4, persistence: 0.6) }
        }"#,
    );
    assert!(wgsl.contains("fbm2(p, i32(4.0), 0.6, 2.0)"));
}

#[test]
fn codegen_shade_with_albedo() {
    let wgsl = compile(
        r#"cinematic {
            layer { fn: circle(0.5) | shade(albedo: gold) }
        }"#,
    );
    assert!(wgsl.contains("shade_albedo"));
    assert!(wgsl.contains("vec3f(0.831, 0.686, 0.216)"));
}

#[test]
fn codegen_unresolved_ident_error() {
    let src = r#"cinematic { layer { fn: circle(bad_var) | glow(2.0) } }"#;
    let tokens = lexer::lex(src).expect("lex failed");
    let mut parser = Parser::new(tokens);
    let cin = parser.parse().expect("parse failed");
    let err = generate_full(&cin).expect_err("should fail on unknown ident");
    assert!(format!("{err}").contains("bad_var"));
}

#[test]
fn codegen_named_colors_are_valid() {
    let src = r#"cinematic { layer { fn: circle(0.3) | glow(2.0) | tint(gold) } }"#;
    let wgsl = compile(src);
    assert!(wgsl.contains("vec3f(0.831, 0.686, 0.216)"));
}

#[test]
fn codegen_math_constants() {
    let src = r#"cinematic { layer { fn: circle(pi * 0.1) | glow(tau) } }"#;
    let wgsl = compile(src);
    assert!(wgsl.contains("3.14159265359"));
    assert!(wgsl.contains("6.28318530718"));
}

#[test]
fn codegen_pipe_chain_glow_first_errors() {
    let src = r#"cinematic { layer { fn: glow(2.0) | circle(0.3) } }"#;
    let tokens = lexer::lex(src).expect("lex failed");
    let mut parser = Parser::new(tokens);
    let cin = parser.parse().expect("parse failed");
    let err = generate_full(&cin).expect_err("glow-first should error");
    assert!(format!("{err}").contains("glow"));
}

#[test]
fn codegen_ternary_select() {
    let src = r#"cinematic {
        layer { fn: circle(0.3) | glow(height > 0.5 ? 3.0 : 1.0) }
    }"#;
    let wgsl = compile(src);
    assert!(wgsl.contains("select("));
}

#[test]
fn codegen_negate_expression() {
    let src = r#"cinematic { layer { fn: translate(-0.5, 0.0) | circle(0.3) | glow(1.0) } }"#;
    let wgsl = compile(src);
    assert!(wgsl.contains("-0.5") || wgsl.contains("(-0.5)"));
}

#[test]
fn codegen_array_to_vec3() {
    let src = r#"cinematic { layer { fn: circle(0.3) | glow(2.0) | tint([0.5, 0.8, 1.0]) } }"#;
    let wgsl = compile(src);
    assert!(wgsl.contains("vec3f(0.5, 0.8, 1.0)"));
}

#[test]
fn codegen_multi_layer_has_blend_logic() {
    let src = r#"cinematic {
        layer a { fn: circle(0.3) | glow(1.0) }
        layer b { fn: ring(0.4, 0.02) | glow(2.0) | blend(mode: additive) }
    }"#;
    let output = compile_full_output(src);
    assert_eq!(output.layer_count, 2);
}

#[test]
fn codegen_raymarch_mode() {
    let out = compile_full_output(
        r#"cinematic {
            layer terrain {
                fn: fbm(p * 2.0, octaves: 6, persistence: 0.5)
            }
            lens {
                mode: raymarch
                camera: orbit(radius: 4.0, height: 2.0, speed: 0.05)
            }
        }"#,
    );

    assert!(matches!(out.render_mode, RenderMode::Raymarch { .. }));
    assert!(out.wgsl.contains("fn field_at"));
    assert!(out.wgsl.contains("fn map_scene"));
    assert!(out.wgsl.contains("fn calc_normal"));
    assert!(out.wgsl.contains("cam_pos"));
}
