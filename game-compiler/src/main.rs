use std::fs;
use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "game", version = "0.2.0")]
#[command(about = "GAME — Generative Animation Matrix Engine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a .game file to WGSL, HTML, or Web Component
    Compile {
        /// Input .game file
        file: PathBuf,

        /// Output a self-contained ES module Web Component
        #[arg(long)]
        component: bool,

        /// Output a self-contained HTML file
        #[arg(long)]
        html: bool,

        /// Custom element tag name (default: derived from filename)
        #[arg(long)]
        tag: Option<String>,

        /// Write output to file instead of stdout
        #[arg(short)]
        o: Option<PathBuf>,
    },

    /// Start a hot-reload dev server for a .game file
    Dev {
        /// Input .game file
        file: PathBuf,

        /// Server port
        #[arg(long, default_value_t = 3333)]
        port: u16,
    },

    /// Batch compile all .game files in a directory
    Build {
        /// Input directory containing .game files
        dir: PathBuf,

        /// Output directory for compiled files
        #[arg(long, default_value = "dist")]
        outdir: PathBuf,
    },

    /// Run visual snapshot tests against .game files
    #[cfg(feature = "snapshot")]
    Test {
        /// .game files to test
        files: Vec<PathBuf>,

        /// Similarity threshold (0-100, default 99)
        #[arg(long, default_value_t = 99.0)]
        threshold: f64,

        /// Update reference snapshots
        #[arg(long)]
        update: bool,

        /// Render size in pixels
        #[arg(long, default_value_t = 256)]
        size: u32,

        /// Time value for rendering (seconds)
        #[arg(long, default_value_t = 0.5)]
        time: f32,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile {
            file,
            component,
            html,
            tag,
            o,
        } => {
            let source = match fs::read_to_string(&file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: cannot read '{}': {e}", file.display());
                    process::exit(1);
                }
            };

            // Use compile_full to get warnings, then wrap for the target format
            let full_output = match game_compiler::compile_full(&source) {
                Ok(o) => o,
                Err(e) => {
                    print_error(&e, &source);
                    process::exit(1);
                }
            };

            // Print warnings to stderr
            for w in &full_output.warnings {
                eprintln!("warning: {w}");
            }

            let (output_str, kind) = if component {
                let tag_name = tag.unwrap_or_else(|| game_compiler::derive_tag_name(&file));
                (game_compiler::runtime::wrap_web_component(&full_output, &tag_name), "component")
            } else if html {
                (game_compiler::runtime::wrap_html_full(&full_output), "HTML")
            } else {
                (full_output.wgsl.clone(), "WGSL")
            };

            if let Some(out_path) = o {
                match fs::write(&out_path, &output_str) {
                    Ok(()) => {
                        eprintln!(
                            "wrote {kind} to {} ({} bytes)",
                            out_path.display(),
                            output_str.len()
                        );
                    }
                    Err(e) => {
                        eprintln!("error: cannot write '{}': {e}", out_path.display());
                        process::exit(1);
                    }
                }
            } else {
                print!("{output_str}");
            }
        }

        Commands::Dev { file, port } => {
            let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
            rt.block_on(async {
                if let Err(e) = game_compiler::server::run_dev_server(file, port).await {
                    eprintln!("error: dev server failed: {e}");
                    process::exit(1);
                }
            });
        }

        #[cfg(feature = "snapshot")]
        Commands::Test {
            files,
            threshold,
            update,
            size,
            time,
        } => {
            let renderer = match game_compiler::snapshot::SnapshotRenderer::new() {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("error: failed to initialize GPU: {e}");
                    process::exit(1);
                }
            };

            let mut passed = 0;
            let mut failed = 0;
            let mut updated = 0;

            for file in &files {
                let source = match fs::read_to_string(file) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("  {} ... READ ERROR: {e}", file.display());
                        failed += 1;
                        continue;
                    }
                };

                let output = match game_compiler::compile_full(&source) {
                    Ok(o) => o,
                    Err(e) => {
                        eprintln!("  {} ... COMPILE ERROR: {e}", file.display());
                        failed += 1;
                        continue;
                    }
                };

                let pixels = match renderer.render_frame(&output, size, size, time) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("  {} ... RENDER ERROR: {e}", file.display());
                        failed += 1;
                        continue;
                    }
                };

                let snap_path = file.with_extension("game.snap.png");
                let diff_path = file.with_extension("game.diff.png");

                if update {
                    if let Err(e) =
                        game_compiler::snapshot::save_png(&pixels, size, size, &snap_path)
                    {
                        eprintln!("  {} ... SAVE ERROR: {e}", file.display());
                        failed += 1;
                    } else {
                        eprintln!(
                            "  {} ... UPDATED ({}x{})",
                            file.display(),
                            size,
                            size
                        );
                        updated += 1;
                    }
                    continue;
                }

                if !snap_path.exists() {
                    eprintln!(
                        "  {} ... NO REFERENCE (run with --update)",
                        file.display()
                    );
                    failed += 1;
                    continue;
                }

                let (ref_pixels, ref_w, ref_h) =
                    match game_compiler::snapshot::load_png(&snap_path) {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("  {} ... REF ERROR: {e}", file.display());
                            failed += 1;
                            continue;
                        }
                    };

                if ref_w != size || ref_h != size {
                    eprintln!(
                        "  {} ... SIZE MISMATCH (ref {}x{}, actual {}x{})",
                        file.display(),
                        ref_w,
                        ref_h,
                        size,
                        size
                    );
                    failed += 1;
                    continue;
                }

                let similarity =
                    game_compiler::snapshot::compare_pixels(&pixels, &ref_pixels, 2);

                if similarity >= threshold {
                    eprintln!(
                        "  {} ... PASS ({:.1}%)",
                        file.display(),
                        similarity
                    );
                    passed += 1;
                    // Clean up any old diff
                    let _ = fs::remove_file(&diff_path);
                } else {
                    eprintln!(
                        "  {} ... FAIL ({:.1}%) -- diff: {}",
                        file.display(),
                        similarity,
                        diff_path.display()
                    );
                    let diff_pixels =
                        game_compiler::snapshot::generate_diff(&pixels, &ref_pixels, size, size);
                    let _ =
                        game_compiler::snapshot::save_png(&diff_pixels, size, size, &diff_path);
                    failed += 1;
                }
            }

            eprintln!();
            if update {
                eprintln!("{updated} snapshots updated, {failed} errors");
            } else {
                eprintln!(
                    "{passed} passed, {failed} failed (threshold: {threshold}%)"
                );
            }
            if failed > 0 {
                process::exit(1);
            }
        }

        Commands::Build { dir, outdir } => {
            if !dir.is_dir() {
                eprintln!("error: '{}' is not a directory", dir.display());
                process::exit(1);
            }

            fs::create_dir_all(&outdir).unwrap_or_else(|e| {
                eprintln!("error: cannot create output dir '{}': {e}", outdir.display());
                process::exit(1);
            });

            let mut compiled = 0;
            let mut errors = 0;

            let entries: Vec<_> = match fs::read_dir(&dir) {
                Ok(rd) => rd,
                Err(e) => {
                    eprintln!("error: cannot read '{}': {e}", dir.display());
                    process::exit(1);
                }
            }
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "game")
                        .unwrap_or(false)
                })
                .collect();

            for entry in entries {
                let path = entry.path();
                let tag = game_compiler::derive_tag_name(&path);
                let out_file = outdir.join(format!("{tag}.js"));

                let source = match fs::read_to_string(&path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("  error: {}: {e}", path.display());
                        errors += 1;
                        continue;
                    }
                };

                match game_compiler::compile_full(&source) {
                    Ok(full) => {
                        for w in &full.warnings {
                            eprintln!("  warning: {}: {w}", path.display());
                        }
                        let js = game_compiler::runtime::wrap_web_component(&full, &tag);
                        fs::write(&out_file, &js).unwrap_or_else(|e| {
                            eprintln!("  error: cannot write {}: {e}", out_file.display());
                        });
                        eprintln!("  {} -> {} ({} bytes)", path.display(), out_file.display(), js.len());
                        compiled += 1;
                    }
                    Err(e) => {
                        eprintln!("  error: {}: {e}", path.display());
                        errors += 1;
                    }
                }
            }

            eprintln!("built {compiled} components ({errors} errors)");
            if errors > 0 {
                process::exit(1);
            }
        }
    }
}

fn print_error(e: &game_compiler::error::GameError, source: &str) {
    eprintln!("error: {e}");

    if let Some(span) = &e.span {
        if span.start <= source.len() {
            let line_num =
                source[..span.start].chars().filter(|c| *c == '\n').count() + 1;
            let line_start =
                source[..span.start].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = source[span.start..]
                .find('\n')
                .map(|i| span.start + i)
                .unwrap_or(source.len());
            let line = &source[line_start..line_end];
            let col = span.start - line_start;

            eprintln!();
            eprintln!("  {line_num} | {line}");
            eprintln!(
                "  {} | {}^",
                " ".repeat(line_num.to_string().len()),
                " ".repeat(col)
            );
        }
    }
}
