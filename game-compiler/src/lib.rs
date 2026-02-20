pub mod ast;
pub mod codegen;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod runtime;
pub mod server;
pub mod token;

use std::path::Path;

use error::Result;

/// Derive a custom element tag name from a file path.
///
/// Strips leading digits/dashes from the stem, ensures the name contains
/// a hyphen (prefixes with `game-` if needed).
pub fn derive_tag_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("component");
    let cleaned = stem.trim_start_matches(|c: char| c.is_ascii_digit() || c == '-');
    let name = if cleaned.is_empty() { stem } else { cleaned };
    if name.contains('-') {
        name.to_string()
    } else {
        format!("game-{name}")
    }
}

/// Compile a `.game` source string to WGSL shader code.
pub fn compile(source: &str) -> Result<String> {
    let tokens = lexer::lex(source)?;
    let mut parser = parser::Parser::new(tokens);
    let cinematic = parser.parse()?;
    codegen::generate_wgsl(&cinematic)
}

/// Compile a `.game` source string to a self-contained HTML file
/// with audio reactivity, parameter modulation, and WebGPU rendering.
pub fn compile_html(source: &str) -> Result<String> {
    let tokens = lexer::lex(source)?;
    let mut parser = parser::Parser::new(tokens);
    let cinematic = parser.parse()?;
    let output = codegen::generate_full(&cinematic)?;
    Ok(runtime::wrap_html_full(&output))
}

/// Full compilation producing structured output (for advanced use).
pub fn compile_full(source: &str) -> Result<codegen::CompileOutput> {
    let tokens = lexer::lex(source)?;
    let mut parser = parser::Parser::new(tokens);
    let cinematic = parser.parse()?;
    codegen::generate_full(&cinematic)
}

/// Compile a `.game` source string to a self-contained ES module Web Component.
/// The `tag_name` must be a valid custom element name (contain a hyphen).
pub fn compile_component(source: &str, tag_name: &str) -> Result<String> {
    let output = compile_full(source)?;
    Ok(runtime::wrap_web_component(&output, tag_name))
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn end_to_end_hello_game() {
        let source = r#"
            cinematic "Hello" {
              layer {
                fn: circle(0.3 + sin(time) * 0.05) | glow(2.0)
              }
            }
        "#;

        let wgsl = compile(source).expect("compilation should succeed");
        assert!(wgsl.contains("struct Uniforms"));
        assert!(wgsl.contains("fn vs_main"));
        assert!(wgsl.contains("fn fs_main"));
        assert!(wgsl.contains("sdf_circle"));
        assert!(wgsl.contains("apply_glow"));
    }

    #[test]
    fn end_to_end_minimal() {
        let source = "cinematic { layer { fn: circle(0.5) } }";
        let wgsl = compile(source).expect("compilation should succeed");
        assert!(wgsl.contains("sdf_circle(p, 0.5)"));
    }

    #[test]
    fn end_to_end_named_layer() {
        let source = r#"cinematic "Test" {
            layer orb {
                fn: sphere(0.3) | glow(3.0)
                depth: base
            }
        }"#;
        let wgsl = compile(source).expect("compilation should succeed");
        assert!(wgsl.contains("sdf_sphere"));
        assert!(wgsl.contains("apply_glow"));
    }

    #[test]
    fn end_to_end_audio_hello() {
        let source = r#"
            cinematic "Audio Hello" {
              layer {
                fn: circle(radius) | glow(intensity)
                radius: 0.3 ~ audio.bass * 0.2
                intensity: 2.0 ~ audio.energy * 3.0
              }
            }
        "#;

        let output = compile_full(source).expect("compilation should succeed");
        assert_eq!(output.params.len(), 2);
        assert!(output.uses_audio);
        assert!(output.wgsl.contains("p_radius: f32"));
        assert!(output.wgsl.contains("p_intensity: f32"));

        // HTML should compile
        let html = compile_html(source).expect("html compilation should succeed");
        assert!(html.contains("getAudioBands"));
        assert!(html.contains("audioBass"));
    }

    #[test]
    fn end_to_end_raymarch() {
        let source = r#"
            cinematic "Terrain" {
              layer terrain {
                fn: fbm(p * 2.0, octaves: 6, persistence: 0.5)
                  | shade(albedo: gold)
              }
              lens {
                mode: raymarch
                camera: orbit(radius: 4.0, height: 2.0, speed: 0.05)
              }
            }
        "#;

        let output = compile_full(source).expect("compilation should succeed");
        assert!(matches!(output.render_mode, codegen::RenderMode::Raymarch { .. }));
        assert!(output.wgsl.contains("fn map_scene"));
        assert!(output.wgsl.contains("fn calc_normal"));
        assert!(output.wgsl.contains("fn field_at"));
    }

    #[test]
    fn end_to_end_spectrum() {
        let source = r#"
            cinematic "Spectrum" {
              layer {
                fn: spectrum(bass, mid, treble)
                bass: 0.0 ~ audio.bass * 1.5
                mid: 0.0 ~ audio.mid * 1.5
                treble: 0.0 ~ audio.treble * 1.5
              }
            }
        "#;

        let output = compile_full(source).expect("compilation should succeed");
        assert_eq!(output.params.len(), 3);
        assert!(output.uses_audio);
        assert!(output.wgsl.contains("d_bass"));
        assert!(output.wgsl.contains("d_mid"));
        assert!(output.wgsl.contains("d_treble"));
        assert!(output.wgsl.contains("c_bass"));

        let html = compile_html(source).expect("html compilation should succeed");
        assert!(html.contains("audioBass"));
        assert!(html.contains("audioMid"));
        assert!(html.contains("audioTreble"));
    }

    #[test]
    fn end_to_end_postprocess() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(0.3) | glow(3.0) | bloom(0.5, 1.5) | vignette(0.3) | grain(0.02)
              }
            }
        "#;

        let output = compile_full(source).expect("compilation should succeed");
        // Glow→Color bridge + post-processing
        assert!(output.wgsl.contains("var color_result"));
        assert!(output.wgsl.contains("pp_lum"));
        assert!(output.wgsl.contains("gr_n"));
    }

    #[test]
    fn end_to_end_mouse_translate() {
        let source = r#"
            cinematic {
              layer {
                fn: translate(mx, my) | circle(0.2) | glow(3.0)
                mx: 0.0 ~ mouse.x * 2.0 - 1.0
                my: 0.0 ~ mouse.y * 2.0 - 1.0
              }
            }
        "#;

        let output = compile_full(source).expect("compilation should succeed");
        assert!(output.uses_mouse);
        assert!(output.wgsl.contains("p = p - vec2f(mx, my)"));
        assert!(output.wgsl.contains("var p"));

        let html = compile_html(source).expect("html compilation should succeed");
        assert!(html.contains("mouseX"));
    }

    #[test]
    fn compile_component_produces_valid_es_module() {
        let source = r#"
            cinematic "Hello" {
              layer {
                fn: circle(0.3) | glow(2.0)
              }
            }
        "#;

        let js = compile_component(source, "game-hello").expect("component compilation should succeed");
        assert!(js.contains("class GameHello extends HTMLElement"));
        assert!(js.contains("customElements.define('game-hello'"));
        assert!(js.contains("export { GameHello }"));
        assert!(js.contains("export default GameHello"));
        assert!(js.contains("connectedCallback"));
        assert!(js.contains("disconnectedCallback"));
        assert!(js.contains("shadowRoot"));
    }

    #[test]
    fn compile_component_with_data_signals() {
        let source = r#"
            cinematic "Data Test" {
              layer {
                fn: circle(radius) | glow(intensity)
                radius: 0.3 ~ data.progress * 0.5
                intensity: 2.0 ~ data.health * 3.0
              }
            }
        "#;

        let output = compile_full(source).expect("compilation should succeed");
        assert!(output.uses_data);
        assert!(output.data_fields.contains(&"progress".to_string()));
        assert!(output.data_fields.contains(&"health".to_string()));

        let js = compile_component(source, "game-data-test").expect("component compilation should succeed");
        assert!(js.contains("'progress'"));
        assert!(js.contains("'health'"));
        assert!(js.contains("set progress("));
        assert!(js.contains("set health("));
        assert!(js.contains("observedAttributes"));
    }

    #[test]
    fn data_signals_in_html_output() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(r) | glow(2.0)
                r: 0.3 ~ data.value * 0.5
              }
            }
        "#;

        let output = compile_full(source).expect("compilation should succeed");
        assert!(output.uses_data);
        assert!(output.data_fields.contains(&"value".to_string()));

        let html = compile_html(source).expect("html compilation should succeed");
        assert!(html.contains("data_value"));
    }

    #[test]
    fn end_to_end_ring_primitive() {
        let source = r#"
            cinematic {
              layer {
                fn: ring(0.3, 0.04) | glow(2.0)
              }
            }
        "#;

        let output = compile_full(source).expect("compilation should succeed");
        assert!(output.wgsl.contains("abs(length(p) - 0.3) - 0.04"));
        assert!(output.wgsl.contains("apply_glow"));
    }

    #[test]
    fn end_to_end_mask_arc() {
        let source = r#"
            cinematic {
              layer {
                fn: ring(0.3, 0.04) | mask_arc(angle) | glow(2.0)
                angle: 0.0 ~ data.progress * 6.283
              }
            }
        "#;

        let output = compile_full(source).expect("compilation should succeed");
        assert!(output.wgsl.contains("arc_theta"));
        assert!(output.wgsl.contains("select(999.0, sdf_result"));
        assert!(output.uses_data);
    }

    #[test]
    fn end_to_end_rotate() {
        let source = r#"
            cinematic {
              layer {
                fn: rotate(1.57) | circle(0.3) | glow(2.0)
              }
            }
        "#;

        let output = compile_full(source).expect("compilation should succeed");
        assert!(output.wgsl.contains("let rc = cos("));
        assert!(output.wgsl.contains("let rs = sin("));
    }

    #[test]
    fn preset_loading_ring_compiles() {
        let source = r#"
            cinematic "Loading Ring" {
              layer {
                fn: ring(0.3, 0.04) | mask_arc(angle) | glow(2.0)
                angle: 0.0 ~ data.progress * 6.283
              }
            }
        "#;
        compile_component(source, "game-loading-ring").expect("loading ring should compile");
    }

    #[test]
    fn preset_status_pulse_compiles() {
        let source = r#"
            cinematic "Status Pulse" {
              layer {
                fn: circle(0.2) | glow(intensity)
                intensity: 1.0 ~ data.health * 4.0
              }
            }
        "#;
        compile_component(source, "game-status-pulse").expect("status pulse should compile");
    }

    #[test]
    fn preset_metric_ring_compiles() {
        let source = r#"
            cinematic "Metric Ring" {
              layer {
                fn: ring(0.35, 0.06) | mask_arc(fill) | glow(3.0)
                fill: 0.0 ~ data.value * 6.283
              }
            }
        "#;
        compile_component(source, "game-metric-ring").expect("metric ring should compile");
    }

    #[test]
    fn component_cleanup_and_lifecycle() {
        let source = "cinematic { layer { fn: circle(0.5) | glow(2.0) } }";
        let js = compile_component(source, "game-lifecycle").expect("compilation should succeed");

        // Verify cleanup in disconnectedCallback
        assert!(js.contains("cancelAnimationFrame"));
        assert!(js.contains("_resizeObserver"));
        assert!(js.contains("ResizeObserver"));
        // Verify WebGPU setup
        assert!(js.contains("navigator.gpu"));
        assert!(js.contains("createShaderModule"));
        assert!(js.contains("createRenderPipeline"));
    }

    // ── Error path tests ──────────────────────────────────────────────

    #[test]
    fn error_empty_source() {
        let result = compile("");
        assert!(result.is_err());
    }

    #[test]
    fn error_missing_cinematic_keyword() {
        let result = compile("layer { fn: circle(0.5) }");
        assert!(result.is_err());
    }

    #[test]
    fn error_unclosed_brace() {
        let result = compile("cinematic { layer { fn: circle(0.5) }");
        assert!(result.is_err());
    }

    #[test]
    fn error_missing_fn_in_layer() {
        // Layer with only a property, no fn: chain — should still parse
        // but codegen should handle it gracefully (not panic)
        let result = compile("cinematic { layer { depth: base } }");
        // This may succeed (empty layer) or error — either is fine, just no panic
        let _ = result;
    }

    #[test]
    fn error_unknown_function_in_pipe() {
        let result = compile("cinematic { layer { fn: totally_fake(0.5) } }");
        assert!(result.is_err(), "unknown function should produce an error");
        let err = result.unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("unknown") || msg.contains("Unknown") || msg.contains("totally_fake"),
            "error should mention the unknown function, got: {msg}");
    }

    #[test]
    fn error_invalid_number() {
        let result = compile("cinematic { layer { fn: circle(abc) } }");
        // 'abc' is an identifier, not a number — should either compile as
        // a parameter reference or produce an error, not panic
        let _ = result;
    }

    #[test]
    fn derive_tag_name_various_inputs() {
        use std::path::Path;

        assert_eq!(derive_tag_name(Path::new("loading-ring.game")), "loading-ring");
        assert_eq!(derive_tag_name(Path::new("spinner.game")), "game-spinner");
        assert_eq!(derive_tag_name(Path::new("001-hello.game")), "game-hello");
        assert_eq!(derive_tag_name(Path::new("my-component.game")), "my-component");
        // Edge case: all digits
        assert_eq!(derive_tag_name(Path::new("123.game")), "game-123");
    }

    #[test]
    fn error_garbage_input() {
        let result = compile("!@#$%^&*()");
        assert!(result.is_err());
    }

    #[test]
    fn compile_html_error_propagates() {
        let result = compile_html("not a valid game file at all");
        assert!(result.is_err());
    }

    #[test]
    fn compile_component_error_propagates() {
        let result = compile_component("not valid", "game-bad");
        assert!(result.is_err());
    }

    // ── New SDF primitives ────────────────────────────────────────────

    #[test]
    fn end_to_end_box_sdf() {
        let source = r#"
            cinematic {
              layer {
                fn: box(0.3, 0.2) | glow(2.0)
              }
            }
        "#;

        let output = compile_full(source).expect("box compilation should succeed");
        assert!(output.wgsl.contains("sdf_box2(p, vec2f(0.3, 0.2))"));
        assert!(output.wgsl.contains("fn sdf_box2"));
    }

    #[test]
    fn end_to_end_polygon_sdf() {
        let source = r#"
            cinematic {
              layer {
                fn: polygon(6.0, 0.3) | glow(2.0)
              }
            }
        "#;

        let output = compile_full(source).expect("polygon compilation should succeed");
        assert!(output.wgsl.contains("sdf_polygon(p, 6.0, 0.3)"));
        assert!(output.wgsl.contains("fn sdf_polygon"));
    }

    #[test]
    fn end_to_end_star_sdf() {
        let source = r#"
            cinematic {
              layer {
                fn: star(5.0, 0.4, 0.2) | glow(3.0)
              }
            }
        "#;

        let output = compile_full(source).expect("star compilation should succeed");
        assert!(output.wgsl.contains("sdf_star(p, 5.0, 0.4, 0.2)"));
        assert!(output.wgsl.contains("fn sdf_star"));
    }

    #[test]
    fn end_to_end_line_sdf() {
        let source = r#"
            cinematic {
              layer {
                fn: line(-0.5, 0.0, 0.5, 0.0, 0.02) | glow(2.0)
              }
            }
        "#;

        let output = compile_full(source).expect("line compilation should succeed");
        assert!(output.wgsl.contains("sdf_line"));
        assert!(output.wgsl.contains("fn sdf_line"));
    }

    #[test]
    fn end_to_end_torus_sdf() {
        let source = r#"
            cinematic {
              layer {
                fn: torus(0.3, 0.05) | glow(2.0)
              }
            }
        "#;

        let output = compile_full(source).expect("torus compilation should succeed");
        assert!(output.wgsl.contains("abs(length(p) - 0.3) - 0.05"));
    }

    // ── Domain operations ─────────────────────────────────────────────

    #[test]
    fn end_to_end_repeat() {
        let source = r#"
            cinematic {
              layer {
                fn: repeat(0.5) | circle(0.1) | glow(2.0)
              }
            }
        "#;

        let output = compile_full(source).expect("repeat compilation should succeed");
        assert!(output.wgsl.contains("round(p / 0.5)"));
    }

    #[test]
    fn end_to_end_mirror() {
        let source = r#"
            cinematic {
              layer {
                fn: mirror("x") | circle(0.3) | glow(2.0)
              }
            }
        "#;

        let output = compile_full(source).expect("mirror compilation should succeed");
        assert!(output.wgsl.contains("abs(p.x)"));
    }

    #[test]
    fn end_to_end_mirror_xy() {
        let source = r#"
            cinematic {
              layer {
                fn: mirror("xy") | circle(0.3) | glow(2.0)
              }
            }
        "#;

        let output = compile_full(source).expect("mirror xy compilation should succeed");
        assert!(output.wgsl.contains("p = abs(p)"));
    }

    #[test]
    fn end_to_end_scale() {
        let source = r#"
            cinematic {
              layer {
                fn: scale(2.0) | circle(0.3) | glow(2.0)
              }
            }
        "#;

        let output = compile_full(source).expect("scale compilation should succeed");
        assert!(output.wgsl.contains("p = p / 2.0"));
        assert!(output.wgsl.contains("scale_factor"));
        assert!(output.wgsl.contains("sdf_result *= scale_factor"));
    }

    #[test]
    fn end_to_end_twist() {
        let source = r#"
            cinematic {
              layer {
                fn: twist(2.0) | circle(0.3) | glow(2.0)
              }
            }
        "#;

        let output = compile_full(source).expect("twist compilation should succeed");
        assert!(output.wgsl.contains("tw_a"));
        assert!(output.wgsl.contains("tw_c"));
    }

    #[test]
    fn end_to_end_onion() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(0.3) | onion(0.02) | glow(2.0)
              }
            }
        "#;

        let output = compile_full(source).expect("onion compilation should succeed");
        assert!(output.wgsl.contains("abs(sdf_result) - 0.02"));
    }

    #[test]
    fn end_to_end_round_sdf() {
        let source = r#"
            cinematic {
              layer {
                fn: box(0.3, 0.2) | round(0.05) | glow(2.0)
              }
            }
        "#;

        let output = compile_full(source).expect("round compilation should succeed");
        assert!(output.wgsl.contains("sdf_result -= 0.05"));
    }

    // ── Noise stages ──────────────────────────────────────────────────

    #[test]
    fn end_to_end_simplex() {
        let source = r#"
            cinematic {
              layer {
                fn: simplex(3.0) | glow(2.0)
              }
            }
        "#;

        let output = compile_full(source).expect("simplex compilation should succeed");
        assert!(output.wgsl.contains("simplex2(p * 3.0)"));
        assert!(output.wgsl.contains("fn simplex2"));
    }

    #[test]
    fn end_to_end_voronoi() {
        let source = r#"
            cinematic {
              layer {
                fn: voronoi(5.0) | glow(2.0)
              }
            }
        "#;

        let output = compile_full(source).expect("voronoi compilation should succeed");
        assert!(output.wgsl.contains("voronoi2(p * 5.0)"));
        assert!(output.wgsl.contains("fn voronoi2"));
    }

    // ── Post-processing stages ────────────────────────────────────────

    #[test]
    fn end_to_end_fog() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(0.3) | glow(2.0) | tint(gold) | fog(1.5)
              }
            }
        "#;

        let output = compile_full(source).expect("fog compilation should succeed");
        assert!(output.wgsl.contains("exp(-length(uv)"));
    }

    #[test]
    fn end_to_end_scanlines() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(0.3) | glow(2.0) | tint(gold) | scanlines(100.0, 0.3)
              }
            }
        "#;

        let output = compile_full(source).expect("scanlines compilation should succeed");
        assert!(output.wgsl.contains("sin(input.uv.y"));
    }

    #[test]
    fn end_to_end_tonemap() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(0.3) | glow(2.0) | tint(gold) | tonemap(1.5)
              }
            }
        "#;

        let output = compile_full(source).expect("tonemap compilation should succeed");
        assert!(output.wgsl.contains("1.0 + color_result.rgb * 1.5"));
    }

    #[test]
    fn end_to_end_invert() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(0.3) | glow(2.0) | tint(gold) | invert()
              }
            }
        "#;

        let output = compile_full(source).expect("invert compilation should succeed");
        assert!(output.wgsl.contains("1.0 - color_result.rgb"));
    }

    #[test]
    fn end_to_end_saturate_color() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(0.3) | glow(2.0) | tint(gold) | saturate_color(1.5)
              }
            }
        "#;

        let output = compile_full(source).expect("saturate_color compilation should succeed");
        assert!(output.wgsl.contains("sat_lum"));
    }

    #[test]
    fn end_to_end_gradient() {
        let source = r#"
            cinematic {
              layer {
                fn: gradient(red, blue, "y")
              }
            }
        "#;

        let output = compile_full(source).expect("gradient compilation should succeed");
        assert!(output.wgsl.contains("mix("));
        assert!(output.wgsl.contains("input.uv.y"));
    }

    // ── Combined stages ───────────────────────────────────────────────

    #[test]
    fn end_to_end_polygon_with_onion_and_glow() {
        let source = r#"
            cinematic {
              layer {
                fn: polygon(6.0, 0.3) | onion(0.02) | glow(3.0) | tint(cyan)
              }
            }
        "#;

        let output = compile_full(source).expect("combined compilation should succeed");
        assert!(output.wgsl.contains("sdf_polygon"));
        assert!(output.wgsl.contains("abs(sdf_result)"));
        assert!(output.wgsl.contains("apply_glow"));
    }

    #[test]
    fn end_to_end_repeat_star() {
        let source = r#"
            cinematic {
              layer {
                fn: repeat(1.0) | star(5.0, 0.3, 0.15) | glow(2.0)
              }
            }
        "#;

        let output = compile_full(source).expect("repeat + star should compile");
        assert!(output.wgsl.contains("round(p /"));
        assert!(output.wgsl.contains("sdf_star"));
    }

    #[test]
    fn end_to_end_displace() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(0.3) | displace(0.1) | glow(2.0)
              }
            }
        "#;

        let output = compile_full(source).expect("displace compilation should succeed");
        assert!(output.wgsl.contains("simplex2(p * 3.0)"));
    }

    // ── Arc timeline tests ──────────────────────────────────────────

    #[test]
    fn end_to_end_arc_basic() {
        let source = r#"
            cinematic "Arc Test" {
              layer pulse {
                fn: circle(0.3) | glow(intensity)
                intensity: 2.0
              }

              arc {
                0:00 "start" {
                  intensity: 0.5
                }
                0:05 "build" {
                  intensity -> 4.0 ease(expo_out) over 3s
                }
              }
            }
        "#;

        let output = compile_full(source).expect("arc compilation should succeed");
        assert_eq!(output.arc_moments.len(), 2, "should have 2 moments");
        assert_eq!(output.arc_moments[0].time_seconds, 0.0);
        assert_eq!(output.arc_moments[0].name.as_deref(), Some("start"));
        assert_eq!(output.arc_moments[1].time_seconds, 5.0);
        assert_eq!(output.arc_moments[1].name.as_deref(), Some("build"));

        // First moment: instant set
        assert_eq!(output.arc_moments[0].transitions.len(), 1);
        assert!(!output.arc_moments[0].transitions[0].is_animated);
        assert!((output.arc_moments[0].transitions[0].target_value - 0.5).abs() < 1e-10);

        // Second moment: animated transition
        assert_eq!(output.arc_moments[1].transitions.len(), 1);
        assert!(output.arc_moments[1].transitions[0].is_animated);
        assert!((output.arc_moments[1].transitions[0].target_value - 4.0).abs() < 1e-10);
        assert_eq!(output.arc_moments[1].transitions[0].easing, "expo_out");
        assert_eq!(output.arc_moments[1].transitions[0].duration_secs, Some(3.0));
    }

    #[test]
    fn arc_html_contains_easing_and_timeline() {
        let source = r#"
            cinematic {
              layer x {
                fn: circle(0.5) | glow(brightness)
                brightness: 1.0
              }

              arc {
                0:00 "off" {
                  brightness: 0.0
                }
                0:03 "on" {
                  brightness -> 1.0 ease(smooth) over 2s
                }
              }
            }
        "#;

        let html = compile_html(source).expect("arc HTML compilation should succeed");
        assert!(html.contains("smooth: t =>"), "HTML should contain easing functions");
        assert!(html.contains("arcTimeline"), "HTML should contain arc timeline data");
        assert!(html.contains("arcUpdate"), "HTML should contain arcUpdate function");
    }

    #[test]
    fn arc_component_contains_timeline() {
        let source = r#"
            cinematic {
              layer x {
                fn: circle(0.5) | glow(brightness)
                brightness: 1.0
              }

              arc {
                0:00 "dim" {
                  brightness: 0.2
                }
                0:02 "bright" {
                  brightness -> 3.0 ease(expo_out)
                }
              }
            }
        "#;

        let component = compile_component(source, "arc-test")
            .expect("arc component compilation should succeed");
        assert!(
            component.contains("_arcTimeline"),
            "component should contain arc timeline"
        );
        assert!(
            component.contains("_arcEase"),
            "component should contain easing reference"
        );
    }

    #[test]
    fn arc_no_moments_produces_empty() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(0.3) | glow(2.0)
              }
            }
        "#;

        let output = compile_full(source).expect("no-arc compilation should succeed");
        assert!(output.arc_moments.is_empty());
    }

    #[test]
    fn arc_unresolvable_target_skipped() {
        // "unknown_param" doesn't match any declared param — transition should be skipped
        let source = r#"
            cinematic {
              layer x {
                fn: circle(0.5) | glow(brightness)
                brightness: 1.0
              }

              arc {
                0:00 "start" {
                  unknown_param: 5.0
                }
              }
            }
        "#;

        let output = compile_full(source).expect("compilation should succeed");
        assert_eq!(output.arc_moments.len(), 1);
        assert!(
            output.arc_moments[0].transitions.is_empty(),
            "unresolvable target should be skipped"
        );
    }

    // ── Compiler warnings tests ────────────────────────────────────

    #[test]
    fn warnings_for_unimplemented_resonance() {
        let source = r#"
            cinematic {
              layer { fn: circle(0.3) | glow(2.0) }
              resonate { something: 1.0 }
            }
        "#;

        let output = compile_full(source).expect("compilation should succeed");
        assert!(
            output.warnings.iter().any(|w| w.contains("resonance")),
            "should warn about resonance: {:?}", output.warnings
        );
    }

    #[test]
    fn warnings_for_multiple_layers() {
        let source = r#"
            cinematic {
              layer a { fn: circle(0.3) | glow(2.0) }
              layer b { fn: box(0.2, 0.2) | glow(1.0) }
            }
        "#;

        let output = compile_full(source).expect("compilation should succeed");
        assert!(
            output.warnings.iter().any(|w| w.contains("additional layer")),
            "should warn about ignored layers: {:?}", output.warnings
        );
    }

    #[test]
    fn warnings_for_pipe_chain_glow_first() {
        let source = r#"
            cinematic {
              layer {
                fn: glow(2.0) | circle(0.3)
              }
            }
        "#;

        let output = compile_full(source).expect("compilation should succeed");
        assert!(
            output.warnings.iter().any(|w| w.contains("glow") && w.contains("SDF")),
            "should warn about glow before SDF: {:?}", output.warnings
        );
    }

    #[test]
    fn no_warnings_for_clean_chain() {
        let source = r#"
            cinematic {
              layer { fn: circle(0.3) | glow(2.0) | tint(gold) }
            }
        "#;

        let output = compile_full(source).expect("compilation should succeed");
        assert!(
            output.warnings.is_empty(),
            "clean chain should have no warnings: {:?}", output.warnings
        );
    }

    #[test]
    fn warnings_appear_in_html_output() {
        let source = r#"
            cinematic {
              layer a { fn: circle(0.3) | glow(2.0) }
              layer b { fn: box(0.2, 0.2) | glow(1.0) }
            }
        "#;

        let html = compile_html(source).expect("compilation should succeed");
        assert!(
            html.contains("console.warn"),
            "HTML should include console.warn for compiler warnings"
        );
    }

    // ── JS expression completeness test ────────────────────────────

    #[test]
    fn js_expr_ternary_compiles_correctly() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(radius) | glow(2.0)
                radius: 0.3 ~ audio.beat > 0.5 ? audio.bass * 0.5 : 0.1
              }
            }
        "#;

        let output = compile_full(source).expect("ternary modulation should compile");
        assert_eq!(output.params.len(), 1);
        let mod_js = output.params[0].mod_js.as_ref().expect("should have mod_js");
        assert!(mod_js.contains("?"), "JS should contain ternary operator: {mod_js}");
        assert!(mod_js.contains("audioBeat"), "JS should reference audioBeat: {mod_js}");
        assert!(mod_js.contains("audioBass"), "JS should reference audioBass: {mod_js}");
    }
}
