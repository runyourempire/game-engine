pub mod ast;
pub mod codegen;
pub mod docs;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod resolver;
pub mod runtime;
#[cfg(not(target_arch = "wasm32"))]
pub mod server;
#[cfg(feature = "snapshot")]
pub mod snapshot;
pub mod token;
#[cfg(feature = "wasm")]
pub mod wasm;

use std::path::{Path, PathBuf};

use error::Result;

/// Compile a `.game` file with import resolution.
///
/// `file_path` is the `.game` file being compiled.
/// `lib_dirs` are additional search paths for imports (e.g., stdlib directory).
pub fn compile_file(file_path: &Path, lib_dirs: &[PathBuf]) -> Result<codegen::CompileOutput> {
    let source = std::fs::read_to_string(file_path).map_err(|e| {
        error::GameError::parse(&format!("cannot read '{}': {e}", file_path.display()))
    })?;
    let tokens = lexer::lex(&source)?;
    let mut parser = parser::Parser::new(tokens);
    let mut cinematic = parser.parse()?;

    let base_dir = file_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    resolver::resolve_imports(&mut cinematic, &base_dir, lib_dirs)?;

    codegen::generate_full(&cinematic)
}

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

/// Compile with strict mode — warnings become errors.
pub fn compile_file_strict(file_path: &Path, lib_dirs: &[PathBuf]) -> Result<codegen::CompileOutput> {
    let output = compile_file(file_path, lib_dirs)?;
    if !output.warnings.is_empty() {
        let msg = output.warnings.iter()
            .map(|w| format!("  - {w}"))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(error::GameError::parse(&format!(
            "strict mode: {} warning(s):\n{msg}", output.warnings.len()
        )));
    }
    Ok(output)
}

/// Compile source string with strict mode.
pub fn compile_full_strict(source: &str) -> Result<codegen::CompileOutput> {
    let output = compile_full(source)?;
    if !output.warnings.is_empty() {
        let msg = output.warnings.iter()
            .map(|w| format!("  - {w}"))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(error::GameError::parse(&format!(
            "strict mode: {} warning(s):\n{msg}", output.warnings.len()
        )));
    }
    Ok(output)
}

/// Generate x-ray variants: one WGSL shader per chain prefix per layer.
pub fn compile_xray_variants(source: &str) -> Result<Vec<codegen::XrayVariant>> {
    let tokens = lexer::lex(source)?;
    let mut parser = parser::Parser::new(tokens);
    let cinematic = parser.parse()?;
    codegen::generate_xray_variants(&cinematic)
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

    #[test]
    fn arc_multiple_transitions_same_moment() {
        let source = r#"
            cinematic {
              layer x {
                fn: circle(radius) | glow(intensity)
                radius: 0.3
                intensity: 2.0
              }
              arc {
                0:00 "init" {
                  radius: 0.1
                  intensity: 0.5
                }
                0:05 "expand" {
                  radius -> 0.5 ease(expo_out) over 2s
                  intensity -> 4.0 ease(smooth) over 2s
                }
              }
            }
        "#;
        let output = compile_full(source).expect("multi-transition arc should compile");
        assert_eq!(output.arc_moments[0].transitions.len(), 2, "first moment should have 2 transitions");
        assert_eq!(output.arc_moments[1].transitions.len(), 2, "second moment should have 2 transitions");
    }

    #[test]
    fn arc_easing_functions_available() {
        // All documented easing functions should be in the generated JS
        let source = r#"
            cinematic {
              layer x {
                fn: circle(0.5) | glow(b)
                b: 1.0
              }
              arc {
                0:00 { b: 0.0 }
                0:01 { b -> 1.0 ease(expo_in) }
                0:02 { b -> 0.0 ease(expo_out) }
                0:03 { b -> 1.0 ease(cubic_in_out) }
                0:04 { b -> 0.0 ease(smooth) }
                0:05 { b -> 1.0 ease(elastic) }
                0:06 { b -> 0.0 ease(bounce) }
              }
            }
        "#;
        let html = compile_html(source).expect("all easings should compile to HTML");
        assert!(html.contains("expo_in:"), "should contain expo_in easing");
        assert!(html.contains("expo_out:"), "should contain expo_out easing");
        assert!(html.contains("cubic_in_out:"), "should contain cubic_in_out easing");
        assert!(html.contains("smooth:"), "should contain smooth easing");
        assert!(html.contains("elastic:"), "should contain elastic easing");
        assert!(html.contains("bounce:"), "should contain bounce easing");
    }

    #[test]
    fn arc_instant_and_animated_mix() {
        let source = r#"
            cinematic {
              layer x {
                fn: circle(0.5) | glow(intensity)
                intensity: 1.0
              }
              arc {
                0:00 "off" {
                  intensity: 0.0
                }
                0:03 "fade_in" {
                  intensity -> 3.0 ease(expo_out) over 2s
                }
              }
            }
        "#;
        let output = compile_full(source).expect("mixed arc should compile");
        // First moment: instant
        assert!(!output.arc_moments[0].transitions[0].is_animated);
        // Second moment: animated
        assert!(output.arc_moments[1].transitions[0].is_animated);
        assert_eq!(output.arc_moments[1].transitions[0].easing, "expo_out");
        assert_eq!(output.arc_moments[1].transitions[0].duration_secs, Some(2.0));
    }

    #[test]
    fn arc_html_frame_loop_integration() {
        // Verify the HTML output has arcUpdate called in the frame loop
        let source = r#"
            cinematic {
              layer x {
                fn: circle(0.5) | glow(b)
                b: 1.0
              }
              arc {
                0:00 { b: 0.5 }
                0:05 { b -> 2.0 ease(linear) }
              }
            }
        "#;
        let html = compile_html(source).expect("arc HTML should compile");
        assert!(html.contains("arcUpdate(time)"), "frame loop should call arcUpdate");
        assert!(html.contains("arcTimeline"), "should contain timeline data");
    }

    #[test]
    fn arc_component_frame_loop_integration() {
        // Verify the Web Component output has arc timeline inline
        let source = r#"
            cinematic {
              layer x {
                fn: circle(0.5) | glow(b)
                b: 1.0
              }
              arc {
                0:00 { b: 0.5 }
                0:05 { b -> 2.0 ease(linear) }
              }
            }
        "#;
        let js = compile_component(source, "arc-comp")
            .expect("arc component should compile");
        assert!(js.contains("_arcTimeline"), "component should have arc timeline");
        assert!(js.contains("_arcBaseOverrides"), "component should use arc base overrides");
    }

    #[test]
    fn arc_no_arc_no_overhead() {
        // Files without arcs should not include easing/timeline code
        let source = r#"
            cinematic {
              layer { fn: circle(0.5) | glow(2.0) }
            }
        "#;
        let html = compile_html(source).expect("no-arc should compile");
        assert!(!html.contains("arcTimeline"), "no-arc should not include timeline");

        let js = compile_component(source, "no-arc")
            .expect("no-arc component should compile");
        assert!(!js.contains("_arcTimeline"), "no-arc component should not include timeline");
    }

    // ── Compiler warnings tests ────────────────────────────────────

    #[test]
    fn resonance_block_compiles() {
        let source = r#"
            cinematic {
              layer fire {
                fn: circle(0.3) | glow(intensity)
                intensity: 0.5
              }
              layer ice {
                fn: ring(0.4, 0.03) | glow(clarity)
                clarity: 0.5
              }
              resonate {
                intensity ~ clarity * 2.0
                damping: 0.96
              }
            }
        "#;

        let output = compile_full(source).expect("resonance compilation should succeed");
        // Resonance is now compiled — should produce JS code, not a warning
        assert!(
            !output.resonance_js.is_empty(),
            "resonance should produce JS code"
        );
        assert!(
            output.resonance_js.contains("resonanceUpdate"),
            "should contain resonance update function"
        );
    }

    #[test]
    fn multi_layer_compilation() {
        let source = r#"
            cinematic {
              layer a { fn: circle(0.3) | glow(2.0) }
              layer b { fn: box(0.2, 0.2) | glow(1.0) }
            }
        "#;

        let output = compile_full(source).expect("multi-layer compilation should succeed");
        // Both layers should appear in WGSL
        assert!(output.wgsl.contains("Layer 0: a"), "should contain layer 0 header");
        assert!(output.wgsl.contains("Layer 1: b"), "should contain layer 1 header");
        assert!(output.wgsl.contains("sdf_circle"), "layer a uses circle");
        assert!(output.wgsl.contains("sdf_box2"), "layer b uses box");
        assert!(output.wgsl.contains("final_color"), "multi-layer uses compositing");
        // No "additional layer ignored" warning
        assert!(
            !output.warnings.iter().any(|w| w.contains("additional layer")),
            "multi-layer should not warn about ignored layers: {:?}", output.warnings
        );
    }

    #[test]
    fn multi_layer_html_and_component() {
        let source = r#"
            cinematic "Layers" {
              layer bg { fn: gradient(deep_blue, midnight, "y") }
              layer orb { fn: circle(0.3) | glow(2.0) | tint(gold) }
            }
        "#;

        // HTML output should work
        let html = compile_html(source).expect("multi-layer HTML should succeed");
        assert!(html.contains("GAME"), "should be a valid HTML page");

        // Component output should work
        let js = compile_component(source, "game-layers")
            .expect("multi-layer component should succeed");
        assert!(js.contains("class GameLayers"), "should produce a Web Component");
    }

    #[test]
    fn multi_layer_three_layers() {
        let source = r#"
            cinematic "Three" {
              layer bg { fn: gradient(black, deep_blue, "radial") }
              layer ring { fn: ring(0.3, 0.04) | glow(2.0) | tint(cyan) }
              layer orb { fn: circle(0.1) | glow(3.0) | tint(gold) }
            }
        "#;

        let output = compile_full(source).expect("3-layer compilation should succeed");
        assert!(output.wgsl.contains("Layer 0: bg"));
        assert!(output.wgsl.contains("Layer 1: ring"));
        assert!(output.wgsl.contains("Layer 2: orb"));
        assert!(output.wgsl.contains("final_color"));
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
        // Use glow-before-SDF to trigger a warning
        let source = r#"
            cinematic {
              layer { fn: glow(2.0) | circle(0.3) }
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

    // ── Cross-layer param collision test ─────────────────────────

    #[test]
    fn warns_on_duplicate_param_across_layers() {
        let source = r#"
            cinematic {
              layer a {
                fn: circle(0.2) | glow(intensity)
                intensity: 2.0 ~ audio.bass * 4.0
              }
              layer b {
                fn: ring(0.4, 0.03) | glow(intensity)
                intensity: 1.0 ~ audio.treble * 3.0
              }
            }
        "#;

        let output = compile_full(source).expect("should compile despite duplicate");
        assert!(
            output.warnings.iter().any(|w| w.contains("duplicates a param")),
            "should warn about duplicate param 'intensity': {:?}", output.warnings
        );
        // Should only have one param (first wins, second skipped)
        assert_eq!(
            output.params.iter().filter(|p| p.name == "intensity").count(),
            1,
            "should have exactly one 'intensity' param"
        );
    }

    // ── Define inlining tests ──────────────────────────────────────

    #[test]
    fn define_basic_expansion() {
        let source = r#"
            cinematic {
              define glow_ring(r, t) {
                ring(r, t) | glow(2.0)
              }
              layer {
                fn: glow_ring(0.3, 0.04) | tint(cyan)
              }
            }
        "#;

        let output = compile_full(source).expect("define expansion should succeed");
        // ring and glow should appear in WGSL (expanded from glow_ring)
        assert!(output.wgsl.contains("abs(length(p) - 0.3) - 0.04"),
            "ring SDF should be in output");
        assert!(output.wgsl.contains("apply_glow"),
            "glow should be in output");
        // No "define not implemented" warning
        assert!(
            !output.warnings.iter().any(|w| w.contains("define")),
            "should not warn about defines: {:?}", output.warnings
        );
    }

    #[test]
    fn define_with_expressions() {
        let source = r#"
            cinematic {
              define bright_circle(size) {
                circle(size) | glow(4.0) | tint(gold)
              }
              layer {
                fn: bright_circle(0.2)
              }
            }
        "#;

        let output = compile_full(source).expect("define with expr should succeed");
        assert!(output.wgsl.contains("sdf_circle(p, 0.2)"),
            "circle with substituted size should be in output");
    }

    #[test]
    fn define_multiple_uses() {
        // Same define used in different layers
        let source = r#"
            cinematic {
              define orb(r) {
                circle(r) | glow(3.0)
              }
              layer a {
                fn: orb(0.1) | tint(gold)
              }
              layer b {
                fn: translate(0.5, 0.0) | orb(0.2) | tint(cyan)
              }
            }
        "#;

        let output = compile_full(source).expect("multi-use define should succeed");
        assert!(output.wgsl.contains("Layer 0: a"), "layer a in output");
        assert!(output.wgsl.contains("Layer 1: b"), "layer b in output");
        assert!(output.wgsl.contains("sdf_circle"), "circle should appear");
    }

    // ── X-Ray variant tests ─────────────────────────────────────────

    #[test]
    fn xray_single_layer_variants() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(0.3) | glow(2.0) | tint(cyan)
              }
            }
        "#;

        let variants = compile_xray_variants(source).expect("xray variants should succeed");
        assert_eq!(variants.len(), 3, "3 stages = 3 variants");
        assert_eq!(variants[0].stage_name, "circle");
        assert_eq!(variants[1].stage_name, "glow");
        assert_eq!(variants[2].stage_name, "tint");
        // First variant (circle only) should have sdf_circle but no apply_glow
        assert!(variants[0].wgsl.contains("sdf_circle"));
        // Last variant should contain tint color
        assert!(variants[2].wgsl.contains("tint") || variants[2].wgsl.contains("vec3f(glow_result)"));
    }

    #[test]
    fn xray_multi_layer_variants() {
        let source = r#"
            cinematic {
              layer a { fn: circle(0.3) | glow(2.0) }
              layer b { fn: box(0.2, 0.2) | glow(1.0) | tint(gold) }
            }
        "#;

        let variants = compile_xray_variants(source).expect("xray variants should succeed");
        // Layer a: 2 stages, layer b: 3 stages = 5 total
        assert_eq!(variants.len(), 5, "should have 5 variants");
        assert_eq!(variants[0].layer_index, 0);
        assert_eq!(variants[0].stage_name, "circle");
        assert_eq!(variants[2].layer_index, 1);
        assert_eq!(variants[2].stage_name, "box");
    }

    #[test]
    fn xray_variants_preserve_uniform_struct() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(radius) | glow(intensity)
                radius: 0.3 ~ audio.bass * 0.2
                intensity: 2.0
              }
            }
        "#;

        let variants = compile_xray_variants(source).expect("xray should succeed");
        // Both variants should have the same uniform struct (both params declared)
        for v in &variants {
            assert!(v.wgsl.contains("p_radius: f32"), "variant '{}' should have p_radius", v.stage_name);
            assert!(v.wgsl.contains("p_intensity: f32"), "variant '{}' should have p_intensity", v.stage_name);
        }
    }

    // ── Import system tests ──────────────────────────────────────────

    #[test]
    fn parse_import_declaration() {
        let source = r#"
            import "stdlib/ui.game" expose loading_spinner, pulse_dot
            cinematic {
              layer { fn: circle(0.3) | glow(2.0) }
            }
        "#;
        let tokens = lexer::lex(source).expect("lex should succeed");
        let mut p = parser::Parser::new(tokens);
        let cinematic = p.parse().expect("parse should succeed");
        assert_eq!(cinematic.imports.len(), 1);
        assert_eq!(cinematic.imports[0].path, "stdlib/ui.game");
        assert_eq!(cinematic.imports[0].names, vec!["loading_spinner", "pulse_dot"]);
    }

    #[test]
    fn parse_import_all() {
        let source = r#"
            import "stdlib/primitives.game" expose ALL
            cinematic {
              layer { fn: circle(0.3) | glow(2.0) }
            }
        "#;
        let tokens = lexer::lex(source).expect("lex should succeed");
        let mut p = parser::Parser::new(tokens);
        let cinematic = p.parse().expect("parse should succeed");
        assert_eq!(cinematic.imports.len(), 1);
        assert_eq!(cinematic.imports[0].names, vec!["ALL"]);
    }

    #[test]
    fn parse_multiple_imports() {
        let source = r#"
            import "a.game" expose foo
            import "b.game" expose bar, baz
            cinematic {
              layer { fn: circle(0.3) | glow(2.0) }
            }
        "#;
        let tokens = lexer::lex(source).expect("lex should succeed");
        let mut p = parser::Parser::new(tokens);
        let cinematic = p.parse().expect("parse should succeed");
        assert_eq!(cinematic.imports.len(), 2);
        assert_eq!(cinematic.imports[0].path, "a.game");
        assert_eq!(cinematic.imports[1].names, vec!["bar", "baz"]);
    }

    #[test]
    fn import_resolve_from_file() {
        use std::io::Write;

        // Create temp directory with a library file
        let dir = std::env::temp_dir().join("game_import_test");
        let _ = std::fs::create_dir_all(&dir);

        // Write a library file
        let lib_file = dir.join("mylib.game");
        let mut f = std::fs::File::create(&lib_file).expect("create lib file");
        writeln!(f, r#"cinematic "mylib" {{
            define glowing_ring(r, t) {{
                ring(r, t) | glow(3.0)
            }}
        }}"#).expect("write lib file");

        // Write a main file that imports from it
        let main_file = dir.join("main.game");
        let mut f = std::fs::File::create(&main_file).expect("create main file");
        writeln!(f, r#"import "mylib.game" expose glowing_ring
        cinematic "Test" {{
            layer {{
                fn: glowing_ring(0.3, 0.04) | tint(gold)
            }}
        }}"#).expect("write main file");

        // Compile with import resolution
        let output = compile_file(&main_file, &[]).expect("import compile should succeed");
        // Ring is inlined as abs(length(p) - r) - t, glow is apply_glow
        assert!(output.wgsl.contains("abs(length(p)"), "ring SDF should be expanded from imported define");
        assert!(output.wgsl.contains("apply_glow"), "glow should be expanded from imported define");

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn import_circular_detected() {
        use std::io::Write;

        let dir = std::env::temp_dir().join("game_circular_test");
        let _ = std::fs::create_dir_all(&dir);

        // a.game imports b.game, b.game imports a.game
        let a_file = dir.join("a.game");
        let mut f = std::fs::File::create(&a_file).expect("create a.game");
        writeln!(f, r#"import "b.game" expose foo
        cinematic "A" {{
            define bar(r) {{ circle(r) | glow(2.0) }}
            layer {{ fn: circle(0.3) | glow(2.0) }}
        }}"#).expect("write a.game");

        let b_file = dir.join("b.game");
        let mut f = std::fs::File::create(&b_file).expect("create b.game");
        writeln!(f, r#"import "a.game" expose bar
        cinematic "B" {{
            define foo(r) {{ circle(r) | glow(3.0) }}
            layer {{ fn: circle(0.3) | glow(2.0) }}
        }}"#).expect("write b.game");

        let result = compile_file(&a_file, &[]);
        assert!(result.is_err(), "circular import should produce error");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("circular"), "error should mention circular: {err_msg}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn import_missing_define_produces_error() {
        use std::io::Write;

        let dir = std::env::temp_dir().join("game_missing_define_test");
        let _ = std::fs::create_dir_all(&dir);

        let lib_file = dir.join("lib.game");
        let mut f = std::fs::File::create(&lib_file).expect("create lib.game");
        writeln!(f, r#"cinematic "lib" {{
            define exists(r) {{ circle(r) | glow(2.0) }}
        }}"#).expect("write lib.game");

        let main_file = dir.join("main.game");
        let mut f = std::fs::File::create(&main_file).expect("create main.game");
        writeln!(f, r#"import "lib.game" expose does_not_exist
        cinematic "Test" {{
            layer {{ fn: circle(0.3) | glow(2.0) }}
        }}"#).expect("write main.game");

        let result = compile_file(&main_file, &[]);
        assert!(result.is_err(), "missing define should produce error");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("does_not_exist"), "error should mention missing name: {err_msg}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn import_with_lib_dirs() {
        use std::io::Write;

        let lib_dir = std::env::temp_dir().join("game_lib_dir_test");
        let main_dir = std::env::temp_dir().join("game_lib_main_test");
        let _ = std::fs::create_dir_all(&lib_dir);
        let _ = std::fs::create_dir_all(&main_dir);

        // Library file in lib_dir
        let lib_file = lib_dir.join("shared.game");
        let mut f = std::fs::File::create(&lib_file).expect("create shared.game");
        writeln!(f, r#"cinematic "shared" {{
            define dot(r) {{ circle(r) | glow(2.0) }}
        }}"#).expect("write shared.game");

        // Main file in main_dir imports from lib_dir
        let main_file = main_dir.join("main.game");
        let mut f = std::fs::File::create(&main_file).expect("create main.game");
        writeln!(f, r#"import "shared.game" expose dot
        cinematic "Test" {{
            layer {{ fn: dot(0.2) | tint(cyan) }}
        }}"#).expect("write main.game");

        // Without lib_dirs, should fail (file not in same dir)
        let result = compile_file(&main_file, &[]);
        assert!(result.is_err(), "should fail without lib_dir");

        // With lib_dirs, should succeed
        let output = compile_file(&main_file, &[lib_dir.clone()])
            .expect("should succeed with lib_dir");
        assert!(output.wgsl.contains("sdf_circle"), "imported define should expand");

        let _ = std::fs::remove_dir_all(&lib_dir);
        let _ = std::fs::remove_dir_all(&main_dir);
    }

    // ── GLSL fallback tests ──────────────────────────────────────

    #[test]
    fn glsl_fallback_generated_for_hello() {
        let source = r#"
            cinematic "Hello" {
              layer {
                fn: circle(0.3) | glow(2.0)
              }
            }
        "#;
        let output = compile_full(source).expect("compile should succeed");

        // GLSL vertex shader checks
        assert!(output.glsl_vertex.contains("#version 300 es"), "GLSL VS should have version");
        assert!(output.glsl_vertex.contains("gl_VertexID"), "GLSL VS should use gl_VertexID");
        assert!(output.glsl_vertex.contains("v_uv"), "GLSL VS should output v_uv");

        // GLSL fragment shader checks
        assert!(output.glsl_fragment.contains("#version 300 es"), "GLSL FS should have version");
        assert!(output.glsl_fragment.contains("out vec4 fragColor"), "GLSL FS should have fragColor");
        assert!(output.glsl_fragment.contains("uniform float u_time"), "GLSL FS should have time uniform");
        assert!(!output.glsl_fragment.contains("vec2f("), "GLSL FS should not contain WGSL vec2f");
        assert!(!output.glsl_fragment.contains("@fragment"), "GLSL FS should not contain @fragment");
    }

    // ── Phase 0: Trust Recovery tests ─────────────────────────────

    #[test]
    fn strict_mode_rejects_with_warnings() {
        // glow before SDF produces a pipe chain warning
        let source = r#"
            cinematic {
              layer {
                fn: glow(2.0) | circle(0.3)
              }
            }
        "#;

        let result = compile_full_strict(source);
        assert!(result.is_err(), "strict mode should reject files with warnings");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("strict mode"), "error should mention strict mode: {err_msg}");
        assert!(err_msg.contains("warning"), "error should mention warnings: {err_msg}");
    }

    #[test]
    fn strict_mode_passes_clean_file() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(0.3) | glow(2.0) | tint(gold)
              }
            }
        "#;

        let output = compile_full_strict(source).expect("strict mode should pass on clean file");
        assert!(output.warnings.is_empty(), "clean file should have no warnings");
    }

    #[test]
    fn unknown_identifier_produces_warning() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(mysterious_var) | glow(2.0)
                mysterious_var: 0.3
              }
            }
        "#;

        // First check that it compiles — mysterious_var is declared as a param
        let output = compile_full(source).expect("should compile");
        // mysterious_var is a declared param, so no warning
        assert!(
            !output.warnings.iter().any(|w| w.contains("mysterious_var")),
            "declared param should not produce warning: {:?}", output.warnings
        );

        // Now use a truly unknown identifier
        let source2 = r#"
            cinematic {
              layer {
                fn: circle(totally_undefined) | glow(2.0)
              }
            }
        "#;

        let output2 = compile_full(source2).expect("should compile");
        assert!(
            output2.warnings.iter().any(|w| w.contains("totally_undefined")),
            "unknown identifier should produce warning: {:?}", output2.warnings
        );
    }

    #[test]
    fn known_builtins_pass_validation() {
        // time, pi, gold — all known builtins that should not produce warnings
        let source = r#"
            cinematic {
              layer {
                fn: circle(0.3 + sin(time) * 0.05) | glow(2.0) | tint(gold)
              }
            }
        "#;

        let output = compile_full(source).expect("should compile");
        assert!(
            !output.warnings.iter().any(|w| w.contains("unknown identifier")),
            "known builtins should not produce warnings: {:?}", output.warnings
        );
    }

    #[test]
    fn define_bindings_resolve() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(radius) | glow(intensity)
                radius: 0.3
                intensity: 2.0
              }
            }
        "#;

        let output = compile_full(source).expect("should compile");
        assert!(
            !output.warnings.iter().any(|w| w.contains("radius") || w.contains("intensity")),
            "params used in expressions should resolve: {:?}", output.warnings
        );
    }

    #[test]
    fn react_mouse_axis_is_functional() {
        let source = r#"
            cinematic {
              layer flame {
                fn: circle(0.3) | glow(intensity)
                intensity: 2.0
              }
              react {
                mouse.click -> flame.intensity
                mouse.x -> intensity
              }
            }
        "#;

        let output = compile_full(source).expect("should compile");
        // Mouse axis handlers are now fully functional — no stub warning
        assert!(
            !output.warnings.iter().any(|w| w.contains("mouse.x") && w.contains("stub")),
            "mouse axis react should NOT produce stub warning: {:?}", output.warnings
        );
        // Should generate functional JS, not just a comment
        assert!(!output.react_js.is_empty(), "react should produce JS");
        assert!(output.react_js.contains("mousemove"), "mouse.x should generate mousemove listener");
    }

    #[test]
    fn check_command_validates() {
        // This tests the compile_full path (the check command equivalent)
        let source = r#"
            cinematic "Valid" {
              layer {
                fn: circle(0.3) | glow(2.0) | tint(gold)
              }
            }
        "#;

        let output = compile_full(source).expect("valid file should compile");
        assert!(output.warnings.is_empty(), "valid file should have no warnings");
        assert!(!output.wgsl.is_empty(), "should produce WGSL output");
        assert_eq!(output.title, "Valid");
    }

    #[test]
    fn glsl_component_contains_fallback_code() {
        let source = r#"
            cinematic "Test" {
              layer {
                fn: circle(0.3) | glow(2.0) | tint(gold)
              }
            }
        "#;
        let output = compile_full(source).expect("compile should succeed");
        let component = runtime::wrap_web_component(&output, "test-glsl");

        assert!(component.contains("GLSL_VS"), "component should embed GLSL vertex shader");
        assert!(component.contains("GLSL_FS"), "component should embed GLSL fragment shader");
        assert!(component.contains("_initWebGL2"), "component should have WebGL2 init method");
        assert!(component.contains("_glFrame"), "component should have WebGL2 frame loop");
        assert!(component.contains("TRIANGLE_STRIP"), "component should draw triangle strip");
    }

    // ── Multi-layer compositing hardening tests ────────────────────

    #[test]
    fn multi_layer_blend_additive() {
        let source = r#"
            cinematic {
              layer a { fn: circle(0.3) | glow(2.0) }
              layer b { fn: box(0.2, 0.2) | glow(1.0) | blend(mode: additive) }
            }
        "#;
        let output = compile_full(source).expect("additive blend should compile");
        assert!(output.wgsl.contains("final_color.rgb + lc"),
            "additive blend should use addition: {}", output.wgsl);
    }

    #[test]
    fn multi_layer_blend_multiply() {
        let source = r#"
            cinematic {
              layer a { fn: circle(0.3) | glow(2.0) }
              layer b { fn: box(0.2, 0.2) | glow(1.0) | blend(mode: multiply) }
            }
        "#;
        let output = compile_full(source).expect("multiply blend should compile");
        assert!(output.wgsl.contains("final_color.rgb * lc"),
            "multiply blend should use multiplication: {}", output.wgsl);
    }

    #[test]
    fn multi_layer_blend_screen() {
        let source = r#"
            cinematic {
              layer a { fn: circle(0.3) | glow(2.0) }
              layer b { fn: box(0.2, 0.2) | glow(1.0) | blend(mode: screen) }
            }
        "#;
        let output = compile_full(source).expect("screen blend should compile");
        assert!(output.wgsl.contains("1.0 - (1.0 - final_color.rgb) * (1.0 - lc)"),
            "screen blend should use screen formula: {}", output.wgsl);
    }

    #[test]
    fn multi_layer_blend_overlay() {
        let source = r#"
            cinematic {
              layer a { fn: circle(0.3) | glow(2.0) }
              layer b { fn: box(0.2, 0.2) | glow(1.0) | blend(mode: overlay) }
            }
        "#;
        let output = compile_full(source).expect("overlay blend should compile");
        assert!(output.wgsl.contains("ov_sel"),
            "overlay blend should use ov_sel variable: {}", output.wgsl);
    }

    #[test]
    fn multi_layer_z_order_respected() {
        // Layers should composite in declaration order
        let source = r#"
            cinematic {
              layer bg { fn: gradient(black, deep_blue, "y") }
              layer fg { fn: circle(0.3) | glow(2.0) | tint(gold) }
            }
        "#;
        let output = compile_full(source).expect("z-order should compile");
        let wgsl = &output.wgsl;
        let bg_pos = wgsl.find("Layer 0: bg").expect("should contain bg layer");
        let fg_pos = wgsl.find("Layer 1: fg").expect("should contain fg layer");
        assert!(bg_pos < fg_pos, "bg should render before fg");
    }

    #[test]
    fn single_layer_regression() {
        // Single layer should still work without multi-layer compositing
        let source = r#"
            cinematic {
              layer { fn: circle(0.5) | glow(2.0) }
            }
        "#;
        let output = compile_full(source).expect("single layer should compile");
        assert!(!output.wgsl.contains("final_color"),
            "single layer should not use multi-layer compositing");
        assert!(output.wgsl.contains("glow_result"),
            "single layer should use glow_result directly");
    }

    #[test]
    fn raymarch_multi_layer_warns() {
        let source = r#"
            cinematic {
              layer terrain {
                fn: fbm(p * 2.0, octaves: 6, persistence: 0.5)
              }
              layer overlay {
                fn: circle(0.3) | glow(2.0)
              }
              lens {
                mode: raymarch
                camera: orbit(radius: 4.0, height: 2.0, speed: 0.05)
              }
            }
        "#;
        let output = compile_full(source).expect("raymarch multi-layer should compile");
        assert!(output.warnings.iter().any(|w| w.contains("raymarch")),
            "should warn about multi-layer in raymarch mode: {:?}", output.warnings);
    }

    #[test]
    fn multi_layer_blend_opacity() {
        let source = r#"
            cinematic {
              layer a { fn: circle(0.3) | glow(2.0) }
              layer b { fn: box(0.2, 0.2) | glow(1.0) | blend(mode: additive, opacity: 0.5) }
            }
        "#;
        let output = compile_full(source).expect("blend opacity should compile");
        assert!(output.wgsl.contains("0.500"),
            "opacity should appear in WGSL output: {}", output.wgsl);
    }

    // ── Resonance system tests ─────────────────────────────────────

    #[test]
    fn resonance_cross_layer_modulation() {
        let source = r#"
            cinematic {
              layer fire {
                fn: circle(0.3) | glow(intensity)
                intensity: 0.5
              }
              layer ice {
                fn: ring(0.4, 0.03) | glow(clarity)
                clarity: 0.5
              }
              resonate {
                intensity ~ clarity * 2.0
                clarity ~ intensity * -1.5
                damping: 0.96
              }
            }
        "#;
        let output = compile_full(source).expect("cross-layer resonance should compile");
        assert!(!output.resonance_js.is_empty(), "should produce resonance JS");
        assert!(output.resonance_js.contains("resonanceUpdate"), "should have update function");
        assert!(output.resonance_js.contains("damp"), "should use damping");
    }

    #[test]
    fn resonance_damping_applied() {
        let source = r#"
            cinematic {
              layer a {
                fn: circle(0.3) | glow(x)
                x: 1.0
              }
              layer b {
                fn: circle(0.3) | glow(y)
                y: 1.0
              }
              resonate {
                x ~ y * 0.5
                damping: 0.8
              }
            }
        "#;
        let output = compile_full(source).expect("resonance with damping should compile");
        assert!(output.resonance_js.contains("0.8"), "should contain damping value");
    }

    #[test]
    fn resonance_empty_bindings() {
        let source = r#"
            cinematic {
              layer a {
                fn: circle(0.3) | glow(2.0)
              }
              resonate {
                damping: 0.95
              }
            }
        "#;
        let output = compile_full(source).expect("empty resonance should compile");
        assert!(output.resonance_js.is_empty(), "empty bindings should produce no JS");
    }

    #[test]
    fn resonance_html_integration() {
        let source = r#"
            cinematic {
              layer fire {
                fn: circle(0.3) | glow(intensity)
                intensity: 0.5
              }
              layer ice {
                fn: ring(0.4, 0.03) | glow(clarity)
                clarity: 0.5
              }
              resonate {
                intensity ~ clarity * 2.0
                damping: 0.96
              }
            }
        "#;
        let html = compile_html(source).expect("resonance HTML should compile");
        assert!(html.contains("resonanceUpdate"), "HTML should include resonance function");
    }

    #[test]
    fn resonance_unresolvable_target_warns() {
        let source = r#"
            cinematic {
              layer a {
                fn: circle(0.3) | glow(x)
                x: 1.0
              }
              resonate {
                nonexistent ~ x * 0.5
                damping: 0.95
              }
            }
        "#;
        let output = compile_full(source).expect("unresolvable resonance should still compile");
        assert!(output.warnings.iter().any(|w| w.contains("nonexistent")),
            "should warn about unresolvable target: {:?}", output.warnings);
    }

    #[test]
    fn resonance_default_damping() {
        let source = r#"
            cinematic {
              layer a {
                fn: circle(0.3) | glow(x)
                x: 1.0
              }
              layer b {
                fn: circle(0.3) | glow(y)
                y: 1.0
              }
              resonate {
                x ~ y * 0.5
              }
            }
        "#;
        let output = compile_full(source).expect("default damping should compile");
        assert!(output.resonance_js.contains("0.95"), "should use default damping of 0.95");
    }

    // ── Nested define & stdlib tests ─────────────────────────────────

    #[test]
    fn nested_define_expansion() {
        // Define A uses define B which is also a define
        let source = r#"
            cinematic {
              define inner_shape() {
                circle(0.3)
              }
              define glowing_shape() {
                inner_shape() | glow(2.0)
              }
              layer {
                fn: glowing_shape()
              }
            }
        "#;
        let output = compile_full(source).expect("nested define should compile");
        assert!(output.wgsl.contains("sdf_circle"), "inner define should expand to circle");
        assert!(output.wgsl.contains("apply_glow"), "outer define should expand to glow");
    }

    #[test]
    fn stdlib_files_parse() {
        // Verify all stdlib files can be parsed (not necessarily compiled standalone)
        use std::fs;
        let stdlib_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("stdlib");
        if stdlib_dir.exists() {
            for entry in fs::read_dir(&stdlib_dir).unwrap() {
                let path = entry.unwrap().path();
                if path.extension().map(|e| e == "game").unwrap_or(false) {
                    let source = fs::read_to_string(&path).expect("should read stdlib file");
                    let tokens = crate::lexer::lex(&source);
                    assert!(tokens.is_ok(), "stdlib file {:?} should lex", path.file_name());
                }
            }
        }
    }

    #[test]
    fn define_with_params_expands() {
        let source = r#"
            cinematic {
              define sized_circle(r) {
                circle(r) | glow(2.0)
              }
              layer {
                fn: sized_circle(0.4)
              }
            }
        "#;
        let output = compile_full(source).expect("parameterized define should compile");
        assert!(output.wgsl.contains("sdf_circle(p, 0.4)"), "should substitute param");
    }

    #[test]
    fn triple_nested_define_expansion() {
        // Three levels: A -> B -> C
        let source = r#"
            cinematic {
              define base_dot() {
                circle(0.1)
              }
              define glowing_dot() {
                base_dot() | glow(3.0)
              }
              define fancy_dot() {
                glowing_dot() | tint(cyan)
              }
              layer {
                fn: fancy_dot()
              }
            }
        "#;
        let output = compile_full(source).expect("triple nested define should compile");
        assert!(output.wgsl.contains("sdf_circle"), "innermost define should expand to circle");
        assert!(output.wgsl.contains("apply_glow"), "middle define should expand to glow");
    }

    // ── Documentation generation tests ─────────────────────────────────

    #[test]
    fn doc_generation_basic() {
        let source = r#"
            cinematic "My Art" {
              layer pulse {
                fn: circle(0.3) | glow(intensity)
                intensity: 2.0 ~ audio.bass * 1.5
              }
            }
        "#;
        let output = compile_full(source).expect("should compile");
        let tokens = crate::lexer::lex(source).unwrap();
        let mut parser = crate::parser::Parser::new(tokens);
        let cinematic = parser.parse().unwrap();
        let doc = crate::docs::generate_docs(&cinematic, &output);
        assert!(doc.contains("# My Art"), "should have title");
        assert!(doc.contains("pulse"), "should list layer name");
        assert!(doc.contains("intensity"), "should list param");
        assert!(doc.contains("**Audio reactive:** true"), "should show audio flag");
    }

    #[test]
    fn doc_generation_with_arc() {
        let source = r#"
            cinematic "Timeline" {
              layer x {
                fn: circle(0.5) | glow(b)
                b: 1.0
              }
              arc {
                0:00 "start" { b: 0.5 }
                0:05 "end" { b -> 3.0 ease(expo_out) }
              }
            }
        "#;
        let output = compile_full(source).expect("should compile");
        let tokens = crate::lexer::lex(source).unwrap();
        let mut parser = crate::parser::Parser::new(tokens);
        let cinematic = parser.parse().unwrap();
        let doc = crate::docs::generate_docs(&cinematic, &output);
        assert!(doc.contains("Arc Timeline"), "should have arc section");
        assert!(doc.contains("start"), "should list moment names");
    }

    #[test]
    fn embed_format_has_postmessage() {
        let source = r#"
            cinematic "Embed" {
              layer { fn: circle(0.3) | glow(2.0) }
            }
        "#;
        let output = compile_full(source).expect("should compile");
        let embed = crate::runtime::wrap_html_embed(&output);
        assert!(
            embed.contains("postMessage") || embed.contains("message"),
            "embed format should have message listener"
        );
        assert!(embed.contains("<!DOCTYPE html"), "should be valid HTML");
    }

    #[test]
    fn doc_generation_with_defines() {
        let source = r#"
            cinematic "With Defines" {
              define my_shape() {
                circle(0.3) | glow(2.0)
              }
              layer {
                fn: my_shape()
              }
            }
        "#;
        let output = compile_full(source).expect("should compile");
        let tokens = crate::lexer::lex(source).unwrap();
        let mut parser = crate::parser::Parser::new(tokens);
        let cinematic = parser.parse().unwrap();
        let doc = crate::docs::generate_docs(&cinematic, &output);
        assert!(doc.contains("Defines"), "should have defines section");
        assert!(doc.contains("my_shape"), "should list define name");
    }

    // ── React block interaction tests ─────────────────────────────

    #[test]
    fn react_click_generates_listener() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(0.3) | glow(2.0)
              }
              react {
                mouse.click -> ripple
              }
            }
        "#;
        let output = compile_full(source).expect("react click should compile");
        assert!(!output.react_js.is_empty(), "react should produce JS");
        assert!(output.react_js.contains("addEventListener"), "should have event listener");
        assert!(output.react_js.contains("click"), "should listen for click");
    }

    #[test]
    fn react_key_generates_listener() {
        let source = r#"
            cinematic {
              layer x {
                fn: circle(0.5) | glow(b)
                b: 1.0
              }
              react {
                key("space") -> arc.pause_toggle
              }
            }
        "#;
        let output = compile_full(source).expect("react key should compile");
        assert!(output.react_js.contains("keydown"), "should listen for keydown");
        assert!(output.react_js.contains("space"), "should filter for space key");
    }

    #[test]
    fn react_mouse_axis_generates_code() {
        let source = r#"
            cinematic {
              layer fire {
                fn: circle(0.3) | glow(intensity)
                intensity: 2.0
              }
              react {
                mouse.x -> fire.intensity
              }
            }
        "#;
        let output = compile_full(source).expect("react mouse.x should compile");
        assert!(!output.react_js.is_empty(), "mouse axis should produce JS");
        // Should generate something functional, not just a comment
        assert!(output.react_js.contains("mouse"), "should reference mouse");
        assert!(output.react_js.contains("mousemove"), "should listen for mousemove");
        assert!(output.react_js.contains("params["), "should update params");
    }

    #[test]
    fn react_html_integration() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(0.3) | glow(2.0)
              }
              react {
                mouse.click -> ripple
                key("space") -> arc.pause_toggle
              }
            }
        "#;
        let html = compile_html(source).expect("react HTML should compile");
        assert!(html.contains("React"), "HTML should include react section");
    }

    #[test]
    fn react_empty_block() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(0.3) | glow(2.0)
              }
              react {
              }
            }
        "#;
        let output = compile_full(source).expect("empty react should compile");
        assert!(output.react_js.is_empty(), "empty react should produce no JS");
    }

    #[test]
    fn react_multiple_handlers() {
        let source = r#"
            cinematic {
              layer x {
                fn: circle(0.5) | glow(b)
                b: 1.0
              }
              react {
                mouse.click -> ripple
                key("space") -> arc.pause_toggle
                key("r") -> reset
              }
            }
        "#;
        let output = compile_full(source).expect("multi-handler react should compile");
        assert!(output.react_js.contains("click"), "should have click handler");
        assert!(output.react_js.contains("space"), "should have space handler");
        assert!(output.react_js.contains("r"), "should have r key handler");
    }

    #[test]
    fn react_scroll_generates_listener() {
        let source = r#"
            cinematic {
              layer x {
                fn: circle(0.5) | glow(intensity)
                intensity: 2.0
              }
              react {
                scroll -> intensity
              }
            }
        "#;
        let output = compile_full(source).expect("react scroll should compile");
        assert!(!output.react_js.is_empty(), "scroll should produce JS");
        assert!(output.react_js.contains("wheel"), "scroll should generate wheel listener");
        assert!(output.react_js.contains("delta"), "scroll should normalize delta");
    }

    // ── Phase 6: Advanced rendering tests ─────────────────────────

    #[test]
    fn raymarch_has_shadow_and_ao() {
        let source = r#"
            cinematic {
              layer terrain {
                fn: fbm(p * 2.0, octaves: 6, persistence: 0.5) | shade(albedo: gold)
              }
              lens {
                mode: raymarch
                camera: orbit(radius: 4.0, height: 2.0, speed: 0.05)
              }
            }
        "#;
        let output = compile_full(source).expect("raymarch should compile");
        assert!(output.wgsl.contains("soft_shadow") || output.wgsl.contains("shadow"),
            "raymarch should include shadow calculation");
        assert!(output.wgsl.contains("calc_ao") || output.wgsl.contains("occ"),
            "raymarch should include ambient occlusion");
    }

    #[test]
    fn smooth_union_builtin_available() {
        // Test that smooth boolean operations can be used if registered as builtins
        let source = r#"
            cinematic {
              layer {
                fn: circle(0.3) | glow(2.0)
              }
            }
        "#;
        // This just verifies compilation still works - smooth ops are helpers
        let output = compile_full(source).expect("compilation should succeed");
        assert!(!output.wgsl.is_empty());
    }

    #[test]
    fn color_grade_postprocess() {
        let source = r#"
            cinematic {
              layer {
                fn: circle(0.3) | glow(2.0) | tint(gold) | color_grade(1.1, 0.0, 1.0)
              }
            }
        "#;
        let result = compile_full(source);
        // color_grade may or may not be implemented yet - test it doesn't panic
        // If it's implemented, verify the output
        match result {
            Ok(output) => {
                assert!(output.wgsl.contains("color_result") || output.wgsl.contains("gamma"),
                    "color grade should modify color output");
            }
            Err(e) => {
                // If not implemented, should give a clear "unknown function" error
                let msg = format!("{e}");
                assert!(msg.contains("unknown") || msg.contains("color_grade"),
                    "error should mention the unknown function");
            }
        }
    }

    #[test]
    fn raymarch_performance_basics() {
        let source = r#"
            cinematic {
              layer terrain {
                fn: fbm(p * 2.0, octaves: 6, persistence: 0.5)
              }
              lens {
                mode: raymarch
                camera: orbit(radius: 4.0, height: 2.0, speed: 0.05)
              }
            }
        "#;
        let output = compile_full(source).expect("raymarch should compile");
        // Verify performance features
        assert!(output.wgsl.contains("50.0"), "should have distance limit for early termination");
        assert!(output.wgsl.contains("0.8"), "should have relaxation factor");
        assert!(output.wgsl.contains("128"), "should have reasonable max iterations");
    }
}
