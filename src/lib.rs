pub mod adapters;
pub mod ast;
pub mod builtins;
pub mod codegen;
pub mod error;
pub mod lexer;
#[cfg(feature = "lsp")]
pub mod lsp;
pub mod parser;
pub mod runtime;
pub mod token;
#[cfg(feature = "wasm")]
pub mod wasm;

use error::CompileError;

// ── Standard Library (embedded) ─────────────────────────

/// Resolve a stdlib import path like "std:shapes" to embedded source.
fn resolve_stdlib(path: &str) -> Option<String> {
    let name = path.strip_prefix("std:")?;
    match name {
        "shapes" => Some(include_str!("../stdlib/shapes.game").to_string()),
        "palettes" => Some(include_str!("../stdlib/palettes.game").to_string()),
        "patterns" => Some(include_str!("../stdlib/patterns.game").to_string()),
        "effects" => Some(include_str!("../stdlib/effects.game").to_string()),
        "motion" => Some(include_str!("../stdlib/motion.game").to_string()),
        "recipes" => Some(include_str!("../stdlib/recipes.game").to_string()),
        _ => None,
    }
}

// ── Configuration ────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Component,
    Html,
    Standalone,
    /// Art Blocks / fxhash compatible: deterministic, self-contained HTML.
    ArtBlocks,
}

#[derive(Debug, Clone)]
pub enum ShaderTarget {
    WebGpu,
    WebGl2,
    Both,
}

#[derive(Debug, Clone)]
pub struct CompileConfig {
    pub output_format: OutputFormat,
    pub target: ShaderTarget,
    /// Seed for deterministic output (Art Blocks mode).
    pub seed: Option<u64>,
}

impl Default for CompileConfig {
    fn default() -> Self {
        Self {
            output_format: OutputFormat::Component,
            target: ShaderTarget::Both,
            seed: None,
        }
    }
}

// ── Output ───────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CompileOutput {
    pub name: String,
    pub wgsl: Option<String>,
    pub glsl: Option<String>,
    pub js: String,
    pub html: Option<String>,
    /// TypeScript type definitions for the Web Component.
    pub dts: Option<String>,
}

// ── Public API ───────────────────────────────────────────

/// Parse a `.game` source string into an AST.
pub fn compile_to_ast(source: &str) -> Result<ast::Program, CompileError> {
    let tokens = lexer::lex(source)?;
    let mut parser = parser::Parser::new(tokens);
    parser.parse()
}

/// Full compile pipeline: lex → parse → validate → codegen → runtime output.
///
/// Returns one `CompileOutput` per cinematic in the program.
pub fn compile(source: &str, config: &CompileConfig) -> Result<Vec<CompileOutput>, CompileError> {
    let program = compile_to_ast(source)?;
    let mut outputs = Vec::new();

    // Resolve URI-schemed imports into adapter JS modules
    let mut import_modules = Vec::new();
    for import in &program.imports {
        match adapters::parse_uri(&import.path) {
            adapters::ImportScheme::Shadertoy(id) => {
                import_modules.push(adapters::shadertoy::generate_shadertoy_adapter(&id));
            }
            adapters::ImportScheme::Midi { channel } => {
                import_modules.push(adapters::midi::generate_midi_adapter(channel));
            }
            adapters::ImportScheme::Osc { host, port, path } => {
                import_modules.push(adapters::osc::generate_osc_adapter(&host, port, &path));
            }
            adapters::ImportScheme::Camera { device_index } => {
                import_modules.push(adapters::camera::generate_camera_adapter(device_index));
            }
            adapters::ImportScheme::File => {} // standard file import — future work
        }
    }

    // Handle file imports: resolve `use "path.game"` into fn definitions
    let mut all_fns = program.fns.clone();
    for import in &program.imports {
        if let Some(stdlib_source) = resolve_stdlib(&import.path) {
            // Standard library import
            if let Ok(imported) = compile_to_ast(&stdlib_source) {
                all_fns.extend(imported.fns);
            }
        } else if import.path.ends_with(".game") {
            // File import — try to read and parse for fn definitions
            if let Ok(source) = std::fs::read_to_string(&import.path) {
                if let Ok(imported) = compile_to_ast(&source) {
                    all_fns.extend(imported.fns);
                }
            }
        }
    }

    for cinematic in &program.cinematics {
        let mut shader = codegen::generate_with_fns(cinematic, &all_fns)?;

        // Prepend import adapter modules so they're available to all cinematic JS
        let mut all_js = import_modules.clone();
        all_js.append(&mut shader.js_modules);
        shader.js_modules = all_js;

        let js = match config.output_format {
            OutputFormat::Component | OutputFormat::Standalone => {
                runtime::component::generate_component(&shader)
            }
            OutputFormat::Html | OutputFormat::ArtBlocks => {
                runtime::component::generate_component(&shader)
            }
        };

        let html = match config.output_format {
            OutputFormat::Html | OutputFormat::Standalone => {
                Some(runtime::html::generate_html(&shader))
            }
            OutputFormat::ArtBlocks => {
                Some(runtime::html::generate_artblocks_html(&shader, config.seed))
            }
            OutputFormat::Component => None,
        };

        let dts = Some(runtime::typescript::generate_typescript_defs(&shader));

        outputs.push(CompileOutput {
            name: shader.name.clone(),
            wgsl: Some(shader.wgsl_fragment),
            glsl: Some(shader.glsl_fragment),
            js,
            html,
            dts,
        });
    }

    // Apply project vertex overrides
    for proj in &program.projects {
        let custom_vert = codegen::project::generate_vertex_wgsl(&proj.mode);
        if let Some(out) = outputs.iter_mut().find(|o| o.name == proj.source) {
            out.wgsl = Some(custom_vert);
        }
    }

    // Breed blocks produce standalone JS modules (not shader output)
    for breed_block in &program.breeds {
        let js = codegen::breed::generate_breed_js(breed_block);
        outputs.push(CompileOutput {
            name: breed_block.name.clone(),
            wgsl: None,
            glsl: None,
            js,
            html: None,
            dts: None,
        });
    }

    // Scene blocks produce full Web Component orchestrators
    for scene_block in &program.scenes {
        let js = codegen::scene::generate_scene_component(scene_block);
        if !js.is_empty() {
            outputs.push(CompileOutput {
                name: scene_block.name.clone(),
                wgsl: None,
                glsl: None,
                js,
                html: None,
                dts: None,
            });
        }
    }

    // IFS blocks produce compute WGSL + JS runtime
    for ifs_block in &program.ifs_blocks {
        let (w, h) = (512u32, 512u32);
        let wgsl = codegen::ifs::generate_compute_wgsl(ifs_block);
        let js = codegen::ifs::generate_runtime_js(ifs_block, w, h);
        outputs.push(CompileOutput {
            name: format!("ifs_{}", outputs.len()),
            wgsl: Some(wgsl),
            glsl: None,
            js,
            html: None,
            dts: None,
        });
    }

    // L-system blocks produce SDF WGSL + JS runtime
    for lsystem_block in &program.lsystem_blocks {
        let wgsl = codegen::lsystem::generate_lsystem_wgsl(lsystem_block);
        let js = codegen::lsystem::generate_runtime_js(lsystem_block);
        outputs.push(CompileOutput {
            name: format!("lsystem_{}", outputs.len()),
            wgsl: Some(wgsl),
            glsl: None,
            js,
            html: None,
            dts: None,
        });
    }

    // Automaton blocks produce compute WGSL + JS runtime
    for automaton_block in &program.automaton_blocks {
        let (w, h) = (256u32, 256u32);
        let wgsl = codegen::automaton::generate_compute_wgsl(automaton_block);
        let js = codegen::automaton::generate_runtime_js(automaton_block, w, h);
        outputs.push(CompileOutput {
            name: format!("automaton_{}", outputs.len()),
            wgsl: Some(wgsl),
            glsl: None,
            js,
            html: None,
            dts: None,
        });
    }

    // Transition matrix blocks produce JS controller classes
    for matrix_block in &program.matrix_blocks {
        if let crate::ast::MatrixBlock::Transitions(ref tm) = matrix_block {
            let js = codegen::matrix::generate_transition_js(tm);
            if !js.is_empty() {
                outputs.push(CompileOutput {
                    name: format!("matrix_transitions_{}", tm.name),
                    wgsl: None,
                    glsl: None,
                    js,
                    html: None,
                    dts: None,
                });
            }
        }
    }

    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> CompileConfig {
        CompileConfig::default()
    }

    #[test]
    fn e2e_minimal_cinematic() {
        let source = r#"
            cinematic "test" {
                layer bg {
                    circle(0.3) | glow(1.5) | tint(1.0, 0.5, 0.2)
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].name, "test");
        assert!(outputs[0].wgsl.is_some());
        assert!(outputs[0].glsl.is_some());
    }

    #[test]
    fn e2e_multiple_cinematics() {
        let source = r#"
            cinematic "a" {
                layer bg { circle(0.1) | glow(1.0) }
            }
            cinematic "b" {
                layer bg { circle(0.2) | glow(2.0) }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].name, "a");
        assert_eq!(outputs[1].name, "b");
    }

    #[test]
    fn e2e_fn_and_cinematic() {
        let source = r#"
            fn dot(r) {
                circle(r) | glow(1.0) | tint(1.0, 1.0, 1.0)
            }
            cinematic "test" {
                layer bg { dot(0.1) }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].wgsl.as_ref().unwrap().contains("fn fs_main"));
    }

    #[test]
    fn e2e_conditional_layer() {
        let source = r#"
            cinematic "test" {
                layer bg {
                    if time > 5.0 {
                        circle(0.5) | glow(2.0) | tint(1.0, 0.0, 0.0)
                    } else {
                        circle(0.2) | glow(1.0) | tint(0.0, 1.0, 0.0)
                    }
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].wgsl.as_ref().unwrap().contains("select"));
    }

    #[test]
    fn e2e_scene_block() {
        let source = r#"
            cinematic "intro" {
                layer bg { circle(0.3) | glow(1.0) }
            }
            scene "show" {
                play "intro" for 5s
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 2); // cinematic + scene
        assert!(outputs[1].js.contains("GameSceneTimeline"));
        // Scene now produces a full Web Component
        assert!(outputs[1].js.contains("customElements.define"));
        assert!(outputs[1].js.contains("game-scene-show"));
    }

    #[test]
    fn e2e_breed_block() {
        let source = r#"
            breed "child" from "parent_a" + "parent_b" {
                inherit layers: mix(0.5)
                mutate radius: 0.1
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].name, "child");
    }

    #[test]
    fn e2e_ifs_block() {
        let source = r#"
            ifs {
                transform t1 [0.5, 0.0, 0.0, 0.5, 0.0, 0.0] weight 0.33
                | transform t2 [0.5, 0.0, 0.0, 0.5, 0.25, 0.5] weight 0.33
                | iterations 50000
                | color transform
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].wgsl.as_ref().unwrap().contains("@compute"));
        assert!(outputs[0].js.contains("GameIfsFractal"));
    }

    #[test]
    fn e2e_lsystem_block() {
        let source = r#"
            lsystem {
                axiom "F"
                | rule F -> "F+F-F-F+F"
                | angle 90
                | iterations 3
                | step 0.01
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].wgsl.as_ref().unwrap().contains("lsystem_sdf"));
        assert!(outputs[0].js.contains("GameLsystem"));
    }

    #[test]
    fn e2e_automaton_block() {
        let source = r#"
            automaton {
                states 2
                | neighborhood moore
                | rule "B3/S23"
                | seed random 0.3
                | speed 10
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].wgsl.as_ref().unwrap().contains("@compute"));
        assert!(outputs[0].js.contains("GameAutomaton"));
    }

    #[test]
    fn e2e_stdlib_import() {
        let source = r#"
            use "std:shapes"
            cinematic "test" {
                layer bg { dot(0.1) }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
        // dot is defined in std:shapes, so codegen should succeed
        assert!(outputs[0].wgsl.is_some());
    }

    #[test]
    fn e2e_artblocks_format() {
        let source = r#"
            cinematic "gen" {
                layer bg { circle(0.3) | glow(1.0) }
            }
        "#;
        let config = CompileConfig {
            output_format: OutputFormat::ArtBlocks,
            target: ShaderTarget::Both,
            seed: Some(42),
        };
        let outputs = compile(source, &config).unwrap();
        assert!(outputs[0].html.is_some());
        let html = outputs[0].html.as_ref().unwrap();
        assert!(html.contains("fxhash"));
        assert!(html.contains("splitmix32"));
    }

    #[test]
    fn e2e_html_format() {
        let source = r#"
            cinematic "gen" {
                layer bg { circle(0.3) | glow(1.0) }
            }
        "#;
        let config = CompileConfig {
            output_format: OutputFormat::Html,
            target: ShaderTarget::Both,
            seed: None,
        };
        let outputs = compile(source, &config).unwrap();
        assert!(outputs[0].html.is_some());
        assert!(outputs[0].html.as_ref().unwrap().contains("<html"));
    }

    #[test]
    fn e2e_pass_block() {
        let source = r#"
            cinematic "fx" {
                layer bg { circle(0.3) | glow(1.0) }
                pass blur_pass { blur(2.0) }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
        // The pass doesn't add a separate output, it's part of the cinematic
        assert!(outputs[0].wgsl.is_some());
    }

    #[test]
    fn e2e_feedback_layer() {
        let source = r#"
            cinematic "trail" {
                layer bg feedback: true {
                    circle(0.3) | glow(1.0)
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn e2e_morph_stage() {
        let source = r#"
            cinematic "test" {
                layer bg {
                    morph(circle(0.3), star(5, 0.3, 0.15), 0.5) | glow(1.0) | tint(1.0, 1.0, 1.0)
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].wgsl.as_ref().unwrap().contains("mix"));
    }

    #[test]
    fn e2e_named_palette() {
        let source = r#"
            cinematic "test" {
                layer bg {
                    circle(0.3) | palette(fire)
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn e2e_compile_error_on_bad_pipeline() {
        let source = r#"
            cinematic "test" {
                layer bg {
                    glow(1.0)
                }
            }
        "#;
        let result = compile(source, &default_config());
        assert!(result.is_err());
    }

    #[test]
    fn e2e_diagnostics_from_error() {
        let source = r#"
            cinematic "test" {
                layer bg {
                    glow(1.0)
                }
            }
        "#;
        let result = compile(source, &default_config());
        let err = result.unwrap_err();
        let diag = err.to_diagnostic();
        assert_eq!(diag.severity, error::DiagnosticSeverity::Error);
        assert!(!diag.message.is_empty());
    }

    #[test]
    fn e2e_mixed_all_block_types() {
        let source = r#"
            use "std:shapes"
            fn custom(r) {
                circle(r) | glow(2.0) | tint(0.5, 0.5, 1.0)
            }
            cinematic "main" {
                layer bg { custom(0.3) }
            }
            scene "timeline" {
                play "main" for 10s
            }
            ifs {
                transform a [0.5, 0.0, 0.0, 0.5, 0.0, 0.0]
                | iterations 10000
            }
            lsystem {
                axiom "F"
                | rule F -> "FF"
                | angle 90
                | iterations 2
            }
            automaton {
                states 2
                | rule "B3/S23"
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        // cinematic + scene + ifs + lsystem + automaton = 5
        assert_eq!(outputs.len(), 5);
    }

    #[test]
    fn e2e_empty_program() {
        let source = "";
        let outputs = compile(source, &default_config()).unwrap();
        assert!(outputs.is_empty());
    }

    #[test]
    fn e2e_comment_only_program() {
        let source = "// nothing here";
        let outputs = compile(source, &default_config()).unwrap();
        assert!(outputs.is_empty());
    }

    #[test]
    fn ast_program_has_v06_fields() {
        let source = "";
        let prog = compile_to_ast(source).unwrap();
        assert!(prog.ifs_blocks.is_empty());
        assert!(prog.lsystem_blocks.is_empty());
        assert!(prog.automaton_blocks.is_empty());
    }

    #[test]
    fn e2e_resonate_keyword_field() {
        let source = r#"
            cinematic "test" {
                layer bg {
                    circle(0.3) | glow(1.5)
                }
                resonate {
                    bass -> bg.opacity * 0.7
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn e2e_arc_with_easing() {
        let source = r#"
            cinematic "test" {
                layer bg {
                    circle(0.3) | glow(1.5)
                }
                arc {
                    scale: 0.1 -> 1.0 over 5s ease-out
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn e2e_arc_ease_in_out() {
        let source = r#"
            cinematic "test" {
                layer bg {
                    circle(0.3) | glow(1.5)
                }
                arc {
                    growth: 0.0 -> 1.0 over 8s ease-in-out
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn e2e_parameterless_stage() {
        let source = r#"
            cinematic "test" {
                layer main {
                    polar | circle(0.3) | glow(2.0) | tint(0.6, 0.8, 1.0)
                }
            }
        "#;
        // This should parse but may fail validation (polar is not a registered builtin)
        // The important thing is it parses without error
        let ast = compile_to_ast(source);
        assert!(ast.is_ok());
    }

    #[test]
    fn e2e_example_015_resonate_network() {
        let source = std::fs::read_to_string("examples/015-resonate-network.game").unwrap();
        let result = compile(&source, &default_config());
        assert!(
            result.is_ok(),
            "example 015 should compile: {:?}",
            result.err()
        );
    }

    #[test]
    fn e2e_example_016_arc_evolution() {
        let source = std::fs::read_to_string("examples/016-arc-evolution.game").unwrap();
        let result = compile(&source, &default_config());
        assert!(
            result.is_ok(),
            "example 016 should compile: {:?}",
            result.err()
        );
    }

    #[test]
    fn e2e_example_018_react_turing() {
        let source = std::fs::read_to_string("examples/018-react-turing.game").unwrap();
        let result = compile(&source, &default_config());
        assert!(
            result.is_ok(),
            "example 018 should compile: {:?}",
            result.err()
        );
    }

    #[test]
    fn e2e_example_023_polar_distort() {
        let source = std::fs::read_to_string("examples/023-polar-distort.game").unwrap();
        let result = compile(&source, &default_config());
        assert!(
            result.is_ok(),
            "example 023 should compile: {:?}",
            result.err()
        );
    }

    #[test]
    fn e2e_example_017_living_organism() {
        let source = std::fs::read_to_string("examples/017-living-organism.game").unwrap();
        let result = compile(&source, &default_config());
        assert!(
            result.is_ok(),
            "example 017 should compile: {:?}",
            result.err()
        );
    }

    #[test]
    fn e2e_example_019_swarm() {
        let source = std::fs::read_to_string("examples/019-swarm-physarum.game").unwrap();
        let result = compile(&source, &default_config());
        assert!(
            result.is_ok(),
            "example 019 should compile: {:?}",
            result.err()
        );
    }

    #[test]
    fn e2e_example_020_flow() {
        let source = std::fs::read_to_string("examples/020-flow-fields.game").unwrap();
        let result = compile(&source, &default_config());
        assert!(
            result.is_ok(),
            "example 020 should compile: {:?}",
            result.err()
        );
    }

    #[test]
    fn e2e_all_examples_compile() {
        let mut failures = Vec::new();
        for entry in std::fs::read_dir("examples").unwrap() {
            let path = entry.unwrap().path();
            if path.extension().map_or(false, |e| e == "game") {
                let source = std::fs::read_to_string(&path).unwrap();
                if compile(&source, &default_config()).is_err() {
                    failures.push(path.display().to_string());
                }
            }
        }
        assert!(
            failures.is_empty(),
            "These examples failed to compile: {:?}",
            failures
        );
    }

    #[test]
    fn e2e_memory_plus_passes_combined() {
        let source = r#"
            cinematic "test" {
                layer trail memory: 0.95 {
                    circle(0.1) | glow(2.0) | tint(1.0, 0.5, 0.2)
                }
                pass soften { blur(2.0) }
                pass frame { vignette(0.5) }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
        // Should have memory bindings in fragment shader
        let wgsl = outputs[0].wgsl.as_ref().unwrap();
        assert!(wgsl.contains("prev_frame"));
        // JS should have pass shaders and memory init
        let js = &outputs[0].js;
        assert!(js.contains("PASS_WGSL_0"));
        assert!(js.contains("PASS_WGSL_1"));
        assert!(js.contains("_initMemory"));
    }

    #[test]
    fn e2e_multi_layer_memory_decay() {
        let source = r#"
            cinematic "test" {
                layer fast memory: 0.97 {
                    circle(0.05) | glow(3.0) | tint(1.0, 0.3, 0.1)
                }
                layer slow memory: 0.85 {
                    ring(0.3, 0.02) | glow(1.5) | tint(0.3, 0.7, 1.0)
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        let wgsl = outputs[0].wgsl.as_ref().unwrap();
        // Both decay values should appear in the shader
        assert!(wgsl.contains("0.970000"));
        assert!(wgsl.contains("0.850000"));
    }

    #[test]
    fn e2e_pass_blur_has_struct_defs() {
        let source = r#"
            cinematic "test" {
                layer bg { circle(0.3) | glow(1.0) }
                pass blur_pass { blur(3.0) }
            }
        "#;
        let config = CompileConfig {
            output_format: OutputFormat::Component,
            target: ShaderTarget::Both,
            seed: None,
        };
        let outputs = compile(source, &config).unwrap();
        let js = &outputs[0].js;
        // Pass WGSL should contain self-contained struct definitions
        assert!(js.contains("struct Uniforms"));
        assert!(js.contains("PASS_WGSL_0"));
    }

    #[test]
    fn e2e_pass_vignette_generates() {
        let source = r#"
            cinematic "test" {
                layer bg { circle(0.3) | glow(1.0) }
                pass v { vignette(0.6) }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        let js = &outputs[0].js;
        assert!(js.contains("PASS_WGSL_0"));
        assert!(js.contains("vign"));
    }

    #[test]
    fn e2e_pass_chain_three_stages() {
        let source = r#"
            cinematic "test" {
                layer bg { circle(0.3) | glow(1.0) }
                pass a { blur(2.0) }
                pass b { threshold(0.5) }
                pass c { vignette(0.4) }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        let js = &outputs[0].js;
        assert!(js.contains("PASS_WGSL_0"));
        assert!(js.contains("PASS_WGSL_1"));
        assert!(js.contains("PASS_WGSL_2"));
        assert!(js.contains("PASS_SHADERS"));
    }

    #[test]
    fn e2e_react_compute_output() {
        let source = r#"
            cinematic "test" {
                layer bg {
                    circle(0.5) | glow(1.5) | tint(0.01, 0.01, 0.03, 1.0)
                }
                react {
                    feed: 0.055
                    kill: 0.062
                    diffuse_a: 1.0
                    diffuse_b: 0.5
                    seed: center(0.15)
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        let js = &outputs[0].js;
        assert!(js.contains("REACT_WGSL"));
        assert!(js.contains("GameReactionField"));
    }

    #[test]
    fn e2e_swarm_compute_output() {
        let source = r#"
            cinematic "test" {
                layer bg {
                    circle(0.5) | glow(1.5) | tint(0.0, 0.0, 0.0, 1.0)
                }
                swarm {
                    agents: 100000
                    sensor_angle: 45
                    sensor_dist: 9.0
                    turn_angle: 45
                    step: 1.0
                    deposit: 5.0
                    decay: 0.95
                    diffuse: 1
                    bounds: wrap
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        let js = &outputs[0].js;
        assert!(js.contains("SWARM_AGENT_WGSL"));
        assert!(js.contains("SWARM_TRAIL_WGSL"));
        assert!(js.contains("GameSwarmSim"));
    }

    #[test]
    fn e2e_flow_compute_output() {
        let source = r#"
            cinematic "test" {
                layer bg {
                    circle(0.5) | glow(1.5) | tint(0.01, 0.0, 0.02, 1.0)
                }
                flow {
                    type: curl
                    scale: 3.0
                    speed: 0.5
                    octaves: 4
                    strength: 1.0
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        let js = &outputs[0].js;
        assert!(js.contains("FLOW_WGSL"));
        assert!(js.contains("GameFlowField"));
    }

    #[test]
    fn e2e_gravity_compute_output() {
        let source = r#"
            cinematic "test" {
                gravity {
                    damping: 0.995,
                    bounds: reflect
                }
                layer stars {
                    circle(0.005) | glow(1.0) | tint(0.8, 0.9, 1.0)
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        let js = &outputs[0].js;
        assert!(js.contains("COMPUTE_WGSL"));
        assert!(js.contains("GameGravitySim"));
    }

    #[test]
    fn e2e_domain_warp_with_palette() {
        let source = r#"
            cinematic "test" {
                layer main {
                    warp(scale: 3.0, octaves: 4, strength: 0.3)
                    | voronoi(5.0)
                    | palette(
                        a_r: 0.5, a_g: 0.5, a_b: 0.5,
                        b_r: 0.5, b_g: 0.5, b_b: 0.5,
                        c_r: 1.0, c_g: 1.0, c_b: 1.0,
                        d_r: 0.0, d_g: 0.33, d_b: 0.67
                    )
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        let wgsl = outputs[0].wgsl.as_ref().unwrap();
        assert!(wgsl.contains("warp"));
        assert!(wgsl.contains("voronoi"));
    }

    #[test]
    fn e2e_sdf_boolean_smooth_union() {
        let source = r#"
            cinematic "test" {
                layer main {
                    smooth_union(
                        circle(0.1),
                        ring(0.18, 0.03),
                        0.06
                    ) | glow(2.0) | tint(1.0, 0.85, 0.5)
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        let wgsl = outputs[0].wgsl.as_ref().unwrap();
        assert!(wgsl.contains("smin"));
    }

    #[test]
    fn e2e_example_035_feedback_trails() {
        let source = std::fs::read_to_string("examples/035-feedback-trails.game").unwrap();
        let result = compile(&source, &default_config());
        assert!(
            result.is_ok(),
            "example 035 should compile: {:?}",
            result.err()
        );
    }

    #[test]
    fn e2e_example_036_blur_vignette() {
        let source = std::fs::read_to_string("examples/036-blur-vignette.game").unwrap();
        let result = compile(&source, &default_config());
        assert!(
            result.is_ok(),
            "example 036 should compile: {:?}",
            result.err()
        );
        let outputs = result.unwrap();
        // Should have pass shaders
        assert!(outputs[0].js.contains("PASS_WGSL_0"));
    }

    #[test]
    fn e2e_example_038_genesis() {
        let source = std::fs::read_to_string("examples/038-genesis.game").unwrap();
        let result = compile(&source, &default_config());
        assert!(
            result.is_ok(),
            "example 038 should compile: {:?}",
            result.err()
        );
        let outputs = result.unwrap();
        // Should have memory (3 layers use it) and vignette pass
        assert!(outputs[0].js.contains("_initMemory"));
        assert!(outputs[0].js.contains("PASS_WGSL_0"));
    }

    #[test]
    fn e2e_example_039_cosmos() {
        let source = std::fs::read_to_string("examples/039-cosmos.game").unwrap();
        let result = compile(&source, &default_config());
        assert!(
            result.is_ok(),
            "example 039 should compile: {:?}",
            result.err()
        );
        let outputs = result.unwrap();
        // Memory + passes + SDF boolean
        assert!(outputs[0].js.contains("_initMemory"));
        assert!(outputs[0].js.contains("PASS_WGSL_0"));
        assert!(outputs[0].js.contains("PASS_WGSL_1"));
    }

    #[test]
    fn e2e_example_042_mandala_bloom() {
        let source = std::fs::read_to_string("examples/042-mandala-bloom.game").unwrap();
        let result = compile(&source, &default_config());
        assert!(
            result.is_ok(),
            "example 042 should compile: {:?}",
            result.err()
        );
        let outputs = result.unwrap();
        // SDF boolean + memory + passes
        assert!(outputs[0].js.contains("_initMemory"));
        assert!(outputs[0].js.contains("PASS_WGSL_0"));
    }

    #[test]
    fn e2e_scene_component_has_child_elements() {
        let source = r#"
            cinematic "a" {
                layer bg { circle(0.3) | glow(1.0) }
            }
            cinematic "b" {
                layer bg { circle(0.2) | glow(2.0) }
            }
            scene "demo" {
                play "a" for 5s
                transition dissolve over 2s
                play "b" for 5s
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 3); // 2 cinematics + 1 scene
        let scene_js = &outputs[2].js;
        assert!(scene_js.contains("SCENE_CINEMATICS"));
        assert!(scene_js.contains("game-scene-demo"));
        assert!(scene_js.contains("GameSceneTimeline"));
        assert!(scene_js.contains("style.opacity"));
    }

    #[test]
    fn e2e_scene_with_transitions() {
        let source = r#"
            cinematic "x" {
                layer bg { circle(0.3) | glow(1.0) }
            }
            cinematic "y" {
                layer bg { circle(0.2) | glow(2.0) }
            }
            scene "show" {
                play "x" for 10s
                transition fade over 3s
                play "y" for 10s
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        let scene_js = &outputs[2].js;
        assert!(scene_js.contains("kind: 'fade'"));
        assert!(scene_js.contains("blend"));
    }

    #[test]
    fn e2e_stdlib_effects_import() {
        let source = r#"
            use "std:effects"
            cinematic "test" {
                layer bg { ember_orb(0.2) }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].wgsl.is_some());
    }

    #[test]
    fn e2e_stdlib_motion_import() {
        let source = r#"
            use "std:motion"
            cinematic "test" {
                layer bg { wobble(4.0, 0.5, 0.3) | circle(0.2) | glow(2.0) | tint(1.0, 0.5, 0.2) }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn e2e_error_suggests_typo() {
        let source = r#"
            cinematic "test" {
                layer bg { circl(0.3) | glow(1.0) }
            }
        "#;
        let result = compile(source, &default_config());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Did you mean 'circle'?"), "got: {err}");
    }

    #[test]
    fn e2e_error_suggests_bridge() {
        let source = r#"
            cinematic "test" {
                layer bg { circle(0.3) | tint(1.0, 0.5, 0.2) }
            }
        "#;
        let result = compile(source, &default_config());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("glow") || err.contains("shade"), "got: {err}");
    }

    #[test]
    fn e2e_example_043_scene_sequence() {
        let source = std::fs::read_to_string("examples/043-scene-sequence.game").unwrap();
        let result = compile(&source, &default_config());
        assert!(
            result.is_ok(),
            "example 043 should compile: {:?}",
            result.err()
        );
        let outputs = result.unwrap();
        // 3 cinematics + 1 scene component
        assert_eq!(outputs.len(), 4);
        assert!(outputs[3].js.contains("game-scene-day-cycle"));
    }

    #[test]
    fn e2e_multiple_scenes() {
        let source = r#"
            cinematic "a" { layer bg { circle(0.1) | glow(1.0) } }
            cinematic "b" { layer bg { circle(0.2) | glow(2.0) } }
            scene "s1" { play "a" for 5s }
            scene "s2" { play "b" for 5s }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 4); // 2 cinematics + 2 scenes
    }

    #[test]
    fn e2e_arc_resonate_memory_pass_combined() {
        let source = r#"
            cinematic "full" {
                layer config { growth: 0.0 }
                layer core memory: 0.95 {
                    circle(0.1) | glow(3.0) | tint(1.0, 0.5, 0.2)
                }
                layer ring_layer {
                    ring(0.3, 0.02) | glow(1.5) | tint(0.5, 0.5, 0.8)
                }
                arc { growth: 0.0 -> 1.0 over 5s ease-out }
                resonate { growth -> core.scale * 0.3 }
                pass v { vignette(0.5) }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
        let js = &outputs[0].js;
        assert!(js.contains("GameArcTimeline"));
        assert!(js.contains("GameResonanceNetwork"));
        assert!(js.contains("_initMemory"));
        assert!(js.contains("PASS_WGSL_0"));
    }

    #[test]
    fn e2e_feedback_layer_compiles() {
        let source = r#"
            cinematic "fb" {
                layer trail feedback: true {
                    circle(0.05) | glow(2.0) | tint(1.0, 0.3, 0.1)
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert!(outputs[0].js.contains("_initMemory"));
    }

    #[test]
    fn e2e_warp_fbm_palette_pipeline() {
        let source = r#"
            cinematic "test" {
                layer main {
                    warp(scale: 2.0, octaves: 4, strength: 0.3)
                    | fbm(scale: 3.0, octaves: 5)
                    | palette(
                        a_r: 0.5, a_g: 0.5, a_b: 0.5,
                        b_r: 0.5, b_g: 0.5, b_b: 0.5,
                        c_r: 1.0, c_g: 1.0, c_b: 1.0,
                        d_r: 0.0, d_g: 0.33, d_b: 0.67
                    )
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert!(outputs[0].wgsl.as_ref().unwrap().contains("warp"));
    }

    #[test]
    fn e2e_polar_distort_pipeline() {
        let source = r#"
            cinematic "test" {
                layer main {
                    polar | simplex(6.0) | glow(1.5) | tint(0.6, 0.8, 1.0)
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn e2e_subtract_boolean() {
        let source = r#"
            cinematic "test" {
                layer main {
                    subtract(circle(0.3), box(0.12, 0.12))
                    | glow(2.5) | tint(1.0, 0.4, 0.2)
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn e2e_intersect_boolean() {
        let source = r#"
            cinematic "test" {
                layer main {
                    intersect(ring(0.3, 0.08), star(5, 0.35, 0.15))
                    | glow(2.0) | tint(0.9, 0.8, 0.3)
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn e2e_blend_mode_layers() {
        let source = r#"
            cinematic "test" {
                layer a {
                    circle(0.2) | glow(2.0) | tint(1.0, 0.0, 0.0)
                }
                layer b blend: screen {
                    circle(0.15) | glow(1.5) | tint(0.0, 0.0, 1.0)
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn e2e_opacity_layer() {
        let source = r#"
            cinematic "test" {
                layer bg opacity: 0.5 {
                    circle(0.3) | glow(2.0) | tint(1.0, 1.0, 1.0)
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn e2e_new_primitives() {
        for prim in [
            "line", "capsule", "triangle", "arc_sdf", "cross", "heart", "egg", "spiral", "grid",
        ] {
            let source = format!("cinematic \"test\" {{ layer bg {{ {prim}() | glow(1.0) }} }}");
            let result = compile(&source, &default_config());
            assert!(
                result.is_ok(),
                "primitive '{}' should compile: {:?}",
                prim,
                result.err()
            );
        }
    }

    #[test]
    fn e2e_shape_modifiers() {
        for modifier in ["round", "shell", "onion"] {
            let source = format!(
                "cinematic \"test\" {{ layer bg {{ circle(0.2) | {modifier}(0.01) | glow(1.0) }} }}"
            );
            let result = compile(&source, &default_config());
            assert!(
                result.is_ok(),
                "modifier '{}' should compile: {:?}",
                modifier,
                result.err()
            );
        }
    }

    #[test]
    fn e2e_spatial_ops() {
        for op in ["repeat", "mirror", "radial"] {
            let source = format!(
                "cinematic \"test\" {{ layer bg {{ {op}(4) | circle(0.05) | glow(1.0) }} }}"
            );
            let result = compile(&source, &default_config());
            assert!(
                result.is_ok(),
                "spatial op '{}' should compile: {:?}",
                op,
                result.err()
            );
        }
    }

    #[test]
    fn e2e_all_pass_types() {
        for pass_type in [
            "blur(2.0)",
            "threshold(0.5)",
            "invert",
            "blend_add",
            "vignette(0.5)",
        ] {
            let source = format!(
                "cinematic \"test\" {{ layer bg {{ circle(0.3) | glow(1.0) }} pass p {{ {pass_type} }} }}"
            );
            let result = compile(&source, &default_config());
            assert!(
                result.is_ok(),
                "pass type '{}' should compile: {:?}",
                pass_type,
                result.err()
            );
        }
    }

    #[test]
    fn e2e_named_palette_variants() {
        for name in ["fire", "ocean", "neon", "aurora", "sunset", "ice"] {
            let source =
                format!("cinematic \"test\" {{ layer bg {{ circle(0.3) | palette({name}) }} }}");
            let result = compile(&source, &default_config());
            assert!(
                result.is_ok(),
                "palette '{}' should compile: {:?}",
                name,
                result.err()
            );
        }
    }

    #[test]
    fn e2e_multiple_stdlib_imports() {
        let source = r#"
            use "std:shapes"
            use "std:patterns"
            cinematic "test" {
                layer a { dot(0.1) }
                layer b { soft_circle(0.2, 1.0, 0.5, 0.2) }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn e2e_multi_layer_blend_modes() {
        let source = r#"
            cinematic "test" {
                layer a { circle(0.2) | glow(2.0) | tint(1.0, 0.0, 0.0) }
                layer b blend: screen { circle(0.15) | glow(1.5) | tint(0.0, 1.0, 0.0) }
                layer c blend: multiply { circle(0.1) | glow(1.0) | tint(0.0, 0.0, 1.0) }
                layer d blend: overlay { ring(0.3, 0.01) | glow(1.0) | tint(1.0, 1.0, 1.0) }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn e2e_conditional_with_audio() {
        let source = r#"
            cinematic "test" {
                layer bg {
                    if audio.beat > 0.5 {
                        circle(0.3) | glow(3.0) | tint(1.0, 0.5, 0.2)
                    } else {
                        circle(0.15) | glow(1.0) | tint(0.5, 0.5, 0.8)
                    }
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].wgsl.as_ref().unwrap().contains("select"));
    }

    #[test]
    fn e2e_html_output_with_passes() {
        let source = r#"
            cinematic "test" {
                layer bg { circle(0.3) | glow(1.0) }
                pass blur { blur(2.0) }
            }
        "#;
        let config = CompileConfig {
            output_format: OutputFormat::Html,
            target: ShaderTarget::Both,
            seed: None,
        };
        let outputs = compile(source, &config).unwrap();
        assert!(outputs[0].html.is_some());
        let html = outputs[0].html.as_ref().unwrap();
        assert!(html.contains("PASS_WGSL_0"));
    }

    #[test]
    fn e2e_artblocks_with_memory() {
        let source = r#"
            cinematic "gen" {
                layer trail memory: 0.95 {
                    circle(0.1) | glow(2.0) | tint(1.0, 0.5, 0.2)
                }
            }
        "#;
        let config = CompileConfig {
            output_format: OutputFormat::ArtBlocks,
            target: ShaderTarget::Both,
            seed: Some(42),
        };
        let outputs = compile(source, &config).unwrap();
        assert!(outputs[0].html.is_some());
        let html = outputs[0].html.as_ref().unwrap();
        assert!(html.contains("fxhash"));
        assert!(html.contains("_initMemory"));
    }

    #[test]
    fn e2e_component_output_format() {
        let source = r#"
            cinematic "test" {
                layer bg { circle(0.3) | glow(1.5) }
            }
        "#;
        let config = CompileConfig {
            output_format: OutputFormat::Component,
            target: ShaderTarget::Both,
            seed: None,
        };
        let outputs = compile(source, &config).unwrap();
        assert!(outputs[0].html.is_none()); // Component mode has no HTML
        assert!(!outputs[0].js.is_empty());
    }

    // ======================================================================
    // Matrix keyword E2E tests
    // ======================================================================

    #[test]
    fn e2e_matrix_coupling() {
        let source = r#"
            cinematic "test" {
                layer config { bass: 0.0  treble: 0.0 }
                layer core { circle(0.3) | glow(1.5) | tint(1.0, 0.5, 0.2) }
                layer ring { ring(0.4, 0.02) | glow(0.8) | tint(1.0, 1.0, 1.0) }
                matrix coupling {
                    [bass, treble] -> [core.scale, ring.opacity]
                    weights [0.3, 0.1, 0.0, 0.5]
                    damping 0.9
                    depth 2
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
        let js = &outputs[0].js;
        assert!(
            js.contains("GameCouplingMatrix"),
            "should generate coupling matrix class"
        );
        assert!(
            js.contains("propagate(uniforms)"),
            "should have propagate method"
        );
        assert!(js.contains("'bass'"), "should have bass source");
        assert!(js.contains("core"), "should have core target");
    }

    #[test]
    fn e2e_matrix_color() {
        let source = r#"
            cinematic "test" {
                layer bg {
                    warp(scale: 2.0, octaves: 3, strength: 0.2)
                    | fbm(scale: 3.0, octaves: 3)
                    | palette(a_r: 0.5, a_g: 0.5, a_b: 0.5, b_r: 0.5, b_g: 0.5, b_b: 0.5, c_r: 1.0, c_g: 1.0, c_b: 1.0, d_r: 0.0, d_g: 0.33, d_b: 0.67)
                }
                layer stars {
                    simplex(10.0) | glow(0.5) | tint(1.0, 1.0, 1.0)
                }
                matrix color {
                    [1.2, 0.1, -0.05,
                     -0.1, 1.05, 0.0,
                     0.0, -0.05, 0.8]
                }
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        assert_eq!(outputs.len(), 1);
        let wgsl = outputs[0].wgsl.as_ref().unwrap();
        assert!(
            wgsl.contains("apply_color_matrix"),
            "WGSL should have color matrix function"
        );
        assert!(wgsl.contains("mat3x3f"), "WGSL should have mat3x3f");
        let glsl = outputs[0].glsl.as_ref().unwrap();
        assert!(
            glsl.contains("apply_color_matrix"),
            "GLSL should have color matrix function"
        );
        assert!(glsl.contains("mat3"), "GLSL should have mat3");
    }

    #[test]
    fn e2e_matrix_transitions() {
        let source = r#"
            cinematic "a" { layer bg { circle(0.3) | glow(1.0) | tint(0.5, 0.5, 0.8) } }
            cinematic "b" { layer bg { circle(0.5) | glow(2.0) | tint(1.0, 0.8, 0.3) } }
            matrix transitions "flow" {
                states ["a", "b"]
                weights [0.0, 1.0, 0.5, 0.5]
                hold 3s
            }
        "#;
        let outputs = compile(source, &default_config()).unwrap();
        // 2 cinematics + 1 transition matrix
        assert_eq!(outputs.len(), 3);
        let tm_js = &outputs[2].js;
        assert!(
            tm_js.contains("GameTransitionMatrix_flow"),
            "should generate transition matrix class"
        );
        assert!(
            tm_js.contains("evaluate(elapsed)"),
            "should have evaluate method"
        );
        assert!(tm_js.contains("next()"), "should have next method");
        assert!(tm_js.contains("'a'"), "should have state 'a'");
        assert!(tm_js.contains("'b'"), "should have state 'b'");
    }

    #[test]
    fn e2e_matrix_examples_compile() {
        for example in [
            "044-matrix-coupling",
            "045-matrix-color",
            "046-matrix-transitions",
        ] {
            let path = format!("examples/{}.game", example);
            let source =
                std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("should read {}", path));
            let result = compile(&source, &default_config());
            assert!(
                result.is_ok(),
                "{} should compile: {:?}",
                example,
                result.err()
            );
        }
    }

    // =================================================================
    // AI-resilience tests — patterns that AI models commonly generate
    // =================================================================

    #[test]
    fn ai_semicolons_in_config() {
        let source = r#"cinematic "t" {
            layer config { pulse: 0.5; intensity: 1.0; speed: 0.3; }
            layer main { circle(0.2) | glow(2.0) | tint(1.0, 0.5, 0.2) }
        }"#;
        assert!(compile(source, &default_config()).is_ok());
    }

    #[test]
    fn ai_keyword_as_layer_name() {
        for keyword in ["flow", "react", "score", "matrix", "feedback", "play"] {
            let source = format!(
                r#"cinematic "t" {{
                    layer {} memory: 0.92 {{
                        circle(0.3) | glow(2.0) | tint(1.0, 0.5, 0.2)
                    }}
                }}"#,
                keyword
            );
            assert!(
                compile(&source, &default_config()).is_ok(),
                "'{}' as layer name should compile",
                keyword
            );
        }
    }

    #[test]
    fn ai_complex_expressions() {
        let source = r#"cinematic "t" {
            layer config { pulse: 0.0 }
            layer main {
                circle(0.1 + sin(pulse * 6.28) * 0.05)
                | glow(2.0 + pulse * 1.5)
                | tint(1.0, 0.5 + cos(pulse * 3.14) * 0.3, 0.2)
            }
        }"#;
        assert!(compile(source, &default_config()).is_ok());
    }

    #[test]
    fn ai_negative_numbers() {
        let source = r#"cinematic "t" {
            layer main {
                translate(-0.5, -0.3) | circle(0.3) | glow(2.0) | tint(1.0, 0.5, 0.2)
            }
        }"#;
        assert!(compile(source, &default_config()).is_ok());
    }

    #[test]
    fn ai_comments_inline() {
        let source = r#"// A beautiful scene
        cinematic "t" {
            // Background layer
            layer bg memory: 0.92 {
                warp(scale: 2.0, octaves: 4, strength: 0.2) // organic
                | fbm(scale: 3.0, octaves: 5) | palette(aurora)
            }
        }"#;
        assert!(compile(source, &default_config()).is_ok());
    }

    #[test]
    fn ai_multiple_cinematics() {
        let source = r#"
        cinematic "a" { layer x { circle(0.3) | glow(2.0) | tint(1.0, 0.5, 0.2) } }
        cinematic "b" { layer y { ring(0.4, 0.02) | glow(1.5) | tint(0.5, 0.8, 1.0) } }
        "#;
        let results = compile(source, &default_config()).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn ai_missing_bridge_auto_glow() {
        // AI forgets bridge — compiler auto-inserts glow(1.5)
        let source = r#"cinematic "t" {
            layer main { circle(0.3) }
        }"#;
        assert!(compile(source, &default_config()).is_ok());
    }

    #[test]
    fn ai_unknown_builtin_error() {
        let source = r#"cinematic "t" {
            layer main { ripple(0.3) | glow(2.0) | tint(1.0, 0.5, 0.2) }
        }"#;
        let err = compile(source, &default_config()).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unknown stage function"), "error: {}", msg);
        assert!(
            msg.contains("ripple"),
            "error should mention 'ripple': {}",
            msg
        );
    }

    #[test]
    fn ai_wrong_pipeline_order_error() {
        let source = r#"cinematic "t" {
            layer main { tint(1.0, 0.5, 0.2) | circle(0.3) | glow(2.0) }
        }"#;
        assert!(compile(source, &default_config()).is_err());
    }

    #[test]
    fn ai_extra_args_tolerated() {
        // AI passes extra args — should not crash
        let source = r#"cinematic "t" {
            layer main { circle(0.3, 0.5) | glow(2.0, 1.0) | tint(1.0, 0.5, 0.2) }
        }"#;
        assert!(compile(source, &default_config()).is_ok());
    }

    #[test]
    fn ai_full_composition() {
        // Realistic AI-generated multi-layer composition
        let source = r#"cinematic "floating-rings" {
            layer config { pulse: 0.0  intensity: 0.5 }
            layer field memory: 0.92 {
                warp(scale: 2.5, octaves: 4, strength: 0.15)
                | fbm(scale: 3.0, octaves: 5) | palette(aurora)
            }
            layer ring_1 memory: 0.88 {
                distort(scale: 2.0, speed: 0.5, strength: 0.03)
                | ring(0.3, 0.015) | glow(2.0) | tint(0.8, 0.7, 0.3)
            }
            layer core memory: 0.95 {
                circle(0.1) | glow(3.5) | tint(1.0, 0.6, 0.2)
            }
            layer edge { ring(0.45, 0.003) | glow(0.8) | tint(0.3, 0.3, 0.35) }
            arc { pulse: 0.0 -> 1.0 over 5s ease-in-out  intensity: 0.5 -> 1.0 over 8s ease-in-out }
            resonate { pulse -> ring_1.scale * 0.2  intensity -> core.brightness * 0.5 }
            pass glow_pass { blur(1.5) }
            pass frame { vignette(0.4) }
        }"#;
        assert!(compile(source, &default_config()).is_ok());
    }

    #[test]
    fn ai_mouse_interaction() {
        let source = r#"cinematic "interactive" {
            layer core memory: 0.95 {
                translate(mouse_x * 2.0 - 1.0, mouse_y * 2.0 - 1.0)
                | circle(0.08 + mouse_down * 0.12)
                | glow(2.0 + mouse_down * 2.5)
                | tint(1.0, 0.6 + mouse_down * 0.3, 0.2)
            }
        }"#;
        assert!(compile(source, &default_config()).is_ok());
    }

    #[test]
    fn ai_keyword_in_resonate_target() {
        let source = r#"cinematic "t" {
            layer config { pulse: 0.0 }
            layer flow memory: 0.92 {
                warp(scale: 2.0, octaves: 4, strength: 0.2)
                | fbm(scale: 3.0, octaves: 5) | palette(ocean)
            }
            resonate { pulse -> flow.scale * 0.3 }
        }"#;
        assert!(compile(source, &default_config()).is_ok());
    }

    #[test]
    fn ai_palette_not_extracted_as_uniform() {
        // palette(aurora) should NOT create an "aurora" uniform
        let source = r#"cinematic "t" {
            layer config { pulse: 0.5 }
            layer bg {
                warp(scale: 2.0, octaves: 4, strength: 0.2)
                | fbm(scale: 3.0, octaves: 5) | palette(aurora)
            }
            layer main {
                circle(0.1 + pulse * 0.2) | glow(2.0) | tint(1.0, 0.5, 0.2)
            }
        }"#;
        let results = compile(source, &default_config()).unwrap();
        let js = &results[0].js;
        // Should have pulse but NOT aurora in UNIFORMS
        assert!(js.contains("name:'pulse'"), "should have pulse uniform");
        assert!(
            !js.contains("name:'aurora'"),
            "aurora should not be a uniform"
        );
    }

    #[test]
    fn ai_time_variable_in_expressions() {
        let source = r#"cinematic "t" {
            layer main {
                translate(sin(time * 0.7) * 0.2, cos(time * 0.5) * 0.15)
                | ring(0.25, 0.012) | glow(2.0) | tint(0.8, 0.6, 1.0)
            }
        }"#;
        let results = compile(source, &default_config()).unwrap();
        let js = &results[0].js;
        // time should NOT be extracted as a uniform
        assert!(!js.contains("name:'time'"), "time should not be a uniform");
        // Should reference time in shader
        assert!(js.contains("time"), "shader should reference time");
    }

    #[test]
    fn uniforms_extracted_from_nested_expressions() {
        // Idents inside binary ops should be extracted as uniforms
        let source = r#"cinematic "t" {
            layer config { pulse: 0.5 speed: 1.0 }
            layer main {
                circle(0.1 + pulse * 0.2) | glow(2.0 + speed * 1.5) | tint(1.0, 0.5, 0.2)
            }
        }"#;
        let results = compile(source, &default_config()).unwrap();
        let js = &results[0].js;
        assert!(js.contains("name:'pulse'"), "pulse should be a uniform");
        assert!(js.contains("name:'speed'"), "speed should be a uniform");
        // WGSL should use them
        assert!(js.contains("let pulse = u.p_pulse;"), "WGSL should read pulse from uniform");
        assert!(js.contains("let speed = u.p_speed;"), "WGSL should read speed from uniform");
    }

    #[test]
    fn uniforms_without_config_layer_extracted_from_expressions() {
        // Variables used in expressions but NOT declared in config should still be uniforms
        let source = r#"cinematic "t" {
            layer main {
                circle(0.1 + intensity * 0.2) | glow(2.0) | tint(1.0, 0.5, 0.2)
            }
        }"#;
        let results = compile(source, &default_config()).unwrap();
        let js = &results[0].js;
        assert!(js.contains("name:'intensity'"), "intensity should be extracted as uniform from expression");
        // WGSL should declare and use it
        assert!(js.contains("let intensity = u.p_intensity;"), "WGSL should read intensity from uniform");
    }

    #[test]
    fn component_has_pending_params_buffer() {
        // Compiled components should buffer params for async renderer init
        let source = r#"cinematic "t" {
            layer config { size: 0.3 }
            layer main {
                circle(size) | glow(2.0) | tint(1.0, 0.5, 0.2)
            }
        }"#;
        let results = compile(source, &default_config()).unwrap();
        let js = &results[0].js;
        assert!(js.contains("_pendingParams"), "should have pending params buffer");
        assert!(js.contains("this._pendingParams[name] = value"), "setParam should buffer");
        assert!(js.contains("Object.entries(this._pendingParams)"), "should replay pending params");
    }
}
