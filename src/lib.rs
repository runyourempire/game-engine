pub mod adapters;
pub mod ast;
pub mod builtins;
pub mod codegen;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod runtime;
pub mod token;

use error::CompileError;

// ── Standard Library (embedded) ─────────────────────────

/// Resolve a stdlib import path like "std:shapes" to embedded source.
fn resolve_stdlib(path: &str) -> Option<String> {
    let name = path.strip_prefix("std:")?;
    match name {
        "shapes" => Some(include_str!("../stdlib/shapes.game").to_string()),
        "palettes" => Some(include_str!("../stdlib/palettes.game").to_string()),
        "patterns" => Some(include_str!("../stdlib/patterns.game").to_string()),
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

        outputs.push(CompileOutput {
            name: shader.name.clone(),
            wgsl: Some(shader.wgsl_fragment),
            glsl: Some(shader.glsl_fragment),
            js,
            html,
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
        });
    }

    // Scene blocks produce JS timeline controllers
    for scene_block in &program.scenes {
        let js = codegen::scene::generate_scene_js(scene_block);
        if !js.is_empty() {
            outputs.push(CompileOutput {
                name: scene_block.name.clone(),
                wgsl: None,
                glsl: None,
                js,
                html: None,
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
        });
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
}
