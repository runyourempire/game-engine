pub mod adapters;
pub mod ast;
pub mod builtins;
pub mod codegen;
pub mod error;
pub mod lexer;
pub mod optimize;
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

use error::CompileError;

// ── Configuration ────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Component,
    Html,
    Standalone,
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
}

impl Default for CompileConfig {
    fn default() -> Self {
        Self {
            output_format: OutputFormat::Component,
            target: ShaderTarget::Both,
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
    let mut program = compile_to_ast(source)?;

    // Expand defines before codegen
    for cinematic in &mut program.cinematics {
        let _ = codegen::analysis::expand_defines(cinematic);
    }

    // Optimize each cinematic (constant folding, strength reduction, no-op elimination,
    // dead define elimination, semantic analysis)
    for cinematic in &mut program.cinematics {
        optimize::optimize_cinematic(cinematic);
    }

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

    for cinematic in &program.cinematics {
        let mut shader = codegen::generate(cinematic)?;

        // Prepend import adapter modules so they're available to all cinematic JS
        let mut all_js = import_modules.clone();
        all_js.append(&mut shader.js_modules);
        shader.js_modules = all_js;

        let js = match config.output_format {
            OutputFormat::Component | OutputFormat::Standalone => {
                runtime::component::generate_component(&shader)
            }
            OutputFormat::Html => {
                runtime::component::generate_component(&shader)
            }
        };

        let html = match config.output_format {
            OutputFormat::Html | OutputFormat::Standalone => {
                Some(runtime::html::generate_html(&shader))
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

    Ok(outputs)
}
