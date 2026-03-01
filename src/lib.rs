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
    let program = compile_to_ast(source)?;
    let mut outputs = Vec::new();

    for cinematic in &program.cinematics {
        let shader = codegen::generate(cinematic)?;

        let js = match config.output_format {
            OutputFormat::Component | OutputFormat::Standalone => {
                runtime::component::generate_component(&shader)
            }
            OutputFormat::Html => {
                // For HTML format, the JS is still the component (embedded in HTML below)
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

    Ok(outputs)
}
