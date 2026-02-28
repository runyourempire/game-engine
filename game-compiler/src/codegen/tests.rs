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

#[test]
fn codegen_raymarch_no_post_clean_output() {
    // Raymarch with no lens post stages should NOT contain hardcoded bloom/vignette
    let wgsl = compile(
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

    // Should NOT have the old hardcoded bloom/vignette lines
    assert!(!wgsl.contains("max(lum - 0.7, 0.0) * 1.2"), "hardcoded bloom should be removed");
    assert!(!wgsl.contains("1.0 - length(uv) * 0.3"), "hardcoded vignette should be removed");
    // Should have the clean output comment
    assert!(wgsl.contains("No post-processing stages"));
    // Should still produce valid shader structure
    assert!(wgsl.contains("return vec4f(color, 1.0)"));
}

#[test]
fn codegen_raymarch_with_bloom_post() {
    // Raymarch with bloom in lens post should produce configurable bloom
    let wgsl = compile(
        r#"cinematic {
            layer terrain {
                fn: fbm(p * 2.0, octaves: 6, persistence: 0.5)
            }
            lens {
                mode: raymarch
                camera: orbit(radius: 4.0, height: 2.0, speed: 0.05)
                post: [bloom(0.5, 1.2)]
            }
        }"#,
    );

    // Should have configurable bloom with user-specified threshold and intensity
    assert!(wgsl.contains("pp_lum"), "bloom should compute luminance");
    assert!(wgsl.contains("0.5"), "bloom threshold should be 0.5");
    assert!(wgsl.contains("1.2"), "bloom intensity should be 1.2");
    assert!(wgsl.contains("pp_bloom_color"), "bloom should compute bloom color");
    // Should have var color_result for post-processing
    assert!(wgsl.contains("var color_result = vec4f(color, 1.0)"));
}

#[test]
fn codegen_raymarch_multiple_post_stages_in_order() {
    // Raymarch with multiple post stages should apply them sequentially
    let wgsl = compile(
        r#"cinematic {
            layer terrain {
                fn: fbm(p * 2.0, octaves: 6, persistence: 0.5)
            }
            lens {
                mode: raymarch
                camera: orbit(radius: 4.0, height: 2.0, speed: 0.05)
                post: [bloom(0.5, 1.2), vignette(0.4)]
            }
        }"#,
    );

    // Both post stages should be present
    assert!(wgsl.contains("post 0: bloom(...)"), "bloom should be stage 0");
    assert!(wgsl.contains("post 1: vignette(...)"), "vignette should be stage 1");
    // Bloom should come before vignette
    let bloom_pos = wgsl.find("post 0: bloom").unwrap();
    let vignette_pos = wgsl.find("post 1: vignette").unwrap();
    assert!(bloom_pos < vignette_pos, "bloom must come before vignette");
    // Vignette should use user-specified strength
    assert!(wgsl.contains("0.4"), "vignette strength should be 0.4");
}

#[test]
fn codegen_raymarch_vignette_color_grade_compose() {
    // Raymarch with vignette + color_grade should compose correctly
    let wgsl = compile(
        r#"cinematic {
            layer terrain {
                fn: fbm(p * 2.0, octaves: 6, persistence: 0.5)
            }
            lens {
                mode: raymarch
                camera: orbit(radius: 4.0, height: 2.0, speed: 0.05)
                post: [vignette(0.4), color_grade(1.2, 0.0, 1.0)]
            }
        }"#,
    );

    // Vignette
    assert!(wgsl.contains("post 0: vignette(...)"), "vignette should be stage 0");
    assert!(wgsl.contains("length(uv) * 0.4"), "vignette strength 0.4");
    // Color grade
    assert!(wgsl.contains("post 1: color_grade(...)"), "color_grade should be stage 1");
    assert!(wgsl.contains("1.2"), "contrast should be 1.2");
    assert!(wgsl.contains("pow(max(color_result.rgb"), "gamma correction should be present");
}

#[test]
fn codegen_flat_mode_post_still_works() {
    // Flat mode post-processing should be unaffected by the raymarch changes
    let wgsl = compile(
        r#"cinematic {
            layer {
                fn: circle(0.5) | glow(2.0) | tint(gold) | bloom(0.6, 1.5) | vignette(0.3)
            }
        }"#,
    );

    // Flat mode bloom and vignette should still work
    assert!(wgsl.contains("pp_lum"), "flat bloom should compute luminance");
    assert!(wgsl.contains("pp_bloom"), "flat bloom should be present");
    assert!(wgsl.contains("length(uv) * 0.3"), "flat vignette should be present");
    // Should NOT contain raymarch-specific code
    assert!(!wgsl.contains("fn field_at"), "flat mode should not have raymarch helpers");
}

#[test]
fn codegen_raymarch_chromatic_post() {
    // Verify chromatic aberration works in raymarch post-processing
    let wgsl = compile(
        r#"cinematic {
            layer terrain {
                fn: fbm(p * 2.0, octaves: 6, persistence: 0.5)
            }
            lens {
                mode: raymarch
                camera: orbit(radius: 4.0, height: 2.0, speed: 0.05)
                post: [chromatic(0.8)]
            }
        }"#,
    );

    assert!(wgsl.contains("ca_d"), "chromatic aberration should compute distance");
    assert!(wgsl.contains("0.8"), "chromatic strength should be 0.8");
    assert!(wgsl.contains("var color_result = vec4f(color, 1.0)"), "should bridge to color_result");
}
