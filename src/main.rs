use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};

use game_compiler::{CompileConfig, OutputFormat, ShaderTarget};

/// GAME compiler — compiles .game DSL to WebGPU shaders + Web Components.
#[derive(Parser, Debug)]
#[command(name = "game", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Compile .game file(s) to output.
    Build {
        /// Input .game file(s).
        #[arg(required = true)]
        input: Vec<PathBuf>,

        /// Output directory.
        #[arg(short, long, default_value = "dist")]
        output_dir: PathBuf,

        /// Output format.
        #[arg(short, long, default_value = "component")]
        format: FormatArg,

        /// Shader target.
        #[arg(short, long, default_value = "both")]
        target: TargetArg,
    },

    /// Compile and watch for changes.
    Dev {
        /// Input .game file.
        #[arg(required = true)]
        input: Vec<PathBuf>,

        /// Output directory.
        #[arg(short, long, default_value = "dist")]
        output_dir: PathBuf,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum FormatArg {
    Component,
    Html,
    Standalone,
}

#[derive(Debug, Clone, ValueEnum)]
enum TargetArg {
    Webgpu,
    Webgl2,
    Both,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Build {
            input,
            output_dir,
            format,
            target,
        } => {
            let config = CompileConfig {
                output_format: match format {
                    FormatArg::Component => OutputFormat::Component,
                    FormatArg::Html => OutputFormat::Html,
                    FormatArg::Standalone => OutputFormat::Standalone,
                },
                target: match target {
                    TargetArg::Webgpu => ShaderTarget::WebGpu,
                    TargetArg::Webgl2 => ShaderTarget::WebGl2,
                    TargetArg::Both => ShaderTarget::Both,
                },
            };

            std::fs::create_dir_all(&output_dir)
                .with_context(|| format!("create output dir: {}", output_dir.display()))?;

            for path in &input {
                eprintln!("[game] compiling {}", path.display());
                let source = std::fs::read_to_string(path)
                    .with_context(|| format!("read: {}", path.display()))?;

                let results = game_compiler::compile(&source, &config)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;

                for output in &results {
                    let stem = &output.name;

                    // Write JS component
                    let js_path = output_dir.join(format!("{stem}.js"));
                    std::fs::write(&js_path, &output.js)
                        .with_context(|| format!("write: {}", js_path.display()))?;
                    eprintln!("[game] wrote {}", js_path.display());

                    // Write HTML if generated
                    if let Some(html) = &output.html {
                        let html_path = output_dir.join(format!("{stem}.html"));
                        std::fs::write(&html_path, html)
                            .with_context(|| format!("write: {}", html_path.display()))?;
                        eprintln!("[game] wrote {}", html_path.display());
                    }

                    // Write shader files
                    if let Some(wgsl) = &output.wgsl {
                        let wgsl_path = output_dir.join(format!("{stem}.wgsl"));
                        std::fs::write(&wgsl_path, wgsl)
                            .with_context(|| format!("write: {}", wgsl_path.display()))?;
                    }
                    if let Some(glsl) = &output.glsl {
                        let glsl_path = output_dir.join(format!("{stem}.frag"));
                        std::fs::write(&glsl_path, glsl)
                            .with_context(|| format!("write: {}", glsl_path.display()))?;
                    }
                }
            }
        }
        Command::Dev { input, output_dir } => {
            eprintln!("[game dev] initial build...");
            // Do a build, then placeholder for watch
            for path in &input {
                let source = std::fs::read_to_string(path)
                    .with_context(|| format!("read: {}", path.display()))?;
                let _program = game_compiler::compile_to_ast(&source)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                eprintln!("[game dev] parsed {} successfully", path.display());
            }
            eprintln!(
                "[game dev] watching {} (Ctrl+C to stop)",
                output_dir.display()
            );
            eprintln!("[game dev] file watcher not yet implemented");
        }
    }

    Ok(())
}
