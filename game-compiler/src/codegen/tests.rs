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
    assert!(wgsl.contains("fbm2(p, 4, 0.6, 2.0)"));
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
