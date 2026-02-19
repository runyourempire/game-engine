pub mod ast;
pub mod codegen;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod runtime;
pub mod server;
pub mod token;

use error::Result;

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
}
