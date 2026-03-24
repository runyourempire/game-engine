use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

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

    /// Compile and serve a live preview with hot reload.
    Dev {
        /// Input .game file.
        #[arg(required = true)]
        input: PathBuf,

        /// Port for the preview server.
        #[arg(short, long, default_value = "4200")]
        port: u16,
    },

    /// Scaffold a new .game file from a template.
    New {
        /// Template name.
        #[arg(short, long, default_value = "minimal")]
        template: TemplateArg,

        /// Output file path.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Validate a .game file without generating output.
    Validate {
        /// Input .game file.
        #[arg(required = true)]
        input: PathBuf,
    },

    /// List all available builtins, palettes, and templates.
    Info {
        /// Category: builtins, palettes, templates, or all.
        #[arg(default_value = "all")]
        category: String,
    },

    /// Start the Language Server Protocol server.
    #[cfg(feature = "lsp")]
    Lsp,
}

#[derive(Debug, Clone, ValueEnum)]
enum FormatArg {
    Component,
    Split,
    Html,
    Standalone,
    Artblocks,
}

#[derive(Debug, Clone, ValueEnum)]
enum TargetArg {
    Webgpu,
    Webgl2,
    Both,
}

#[derive(Debug, Clone, ValueEnum)]
enum TemplateArg {
    Minimal,
    Audio,
    Particles,
    Procedural,
    Composition,
    Reactive,
    Sdf,
    Scene,
}

// ── Template constants ──────────────────────────────────────

const TEMPLATE_MINIMAL: &str = r#"cinematic "untitled" {
  layer main {
    circle(0.3) | glow(2.0) | tint(0.83, 0.69, 0.22)
  }
}
"#;

const TEMPLATE_AUDIO: &str = r#"cinematic "audio-reactive" {
  listen {
    bass: energy(range: [20, 200])
    mid: energy(range: [200, 2000])
    treble: energy(range: [2000, 16000])
    onset: attack(threshold: 0.6)
  }

  layer pulse {
    circle(0.2) | glow(3.0) | tint(0.83, 0.69, 0.22)
  }

  layer ring {
    ring(0.4, 0.02) | glow(1.5) | tint(0.2, 0.6, 1.0)
  }
}
"#;

const TEMPLATE_PARTICLES: &str = r#"cinematic "particle-field" {
  layer bg {
    circle(0.01) | glow(4.0) | tint(0.1, 0.2, 0.4)
  }

  flow {
    type: curl
    scale: 3.0
    speed: 0.5
    octaves: 4
    strength: 1.0
    bounds: wrap
  }
}
"#;

const TEMPLATE_PROCEDURAL: &str = r#"cinematic "procedural" {
  layer terrain {
    warp(scale: 3.0, octaves: 4, strength: 0.3)
    | fbm(scale: 2.0, octaves: 6, persistence: 0.5)
    | palette(earth)
  }
}
"#;

const TEMPLATE_COMPOSITION: &str = r#"cinematic "composition" {
  layer base {
    circle(0.4) | glow(2.0) | tint(0.1, 0.1, 0.3)
  }

  layer mid blend: add {
    ring(0.3, 0.01) | glow(3.0) | tint(0.83, 0.69, 0.22)
  }

  layer top blend: add {
    circle(0.05) | glow(5.0) | tint(1.0, 1.0, 1.0)
  }
}
"#;

const TEMPLATE_REACTIVE: &str = r#"cinematic "audio-reactive" {
  listen {
    bass: energy(range: [20, 200])
    mid: energy(range: [200, 2000])
    treble: energy(range: [2000, 16000])
    onset: attack(threshold: 0.6)
  }

  layer core {
    warp(scale: 3.0, strength: 0.2)
    | circle(0.15) | glow(3.0) | tint(0.83, 0.69, 0.22)
  }

  layer ring1 blend: add {
    ring(0.25, 0.01) | glow(2.5) | tint(0.4, 0.7, 1.0)
  }

  resonate {
    bass -> core.radius * 0.15
    mid -> ring1.radius * 0.1
  }
}
"#;

const TEMPLATE_SDF: &str = r#"cinematic "sdf-showcase" {
  layer base {
    rotate(0.3)
    | smooth_union(circle(0.15), circle(0.1), 0.1)
    | glow(2.0) | tint(0.9, 0.3, 0.2)
  }

  layer rings blend: add {
    radial(6) | ring(0.3, 0.005) | glow(2.5) | tint(0.83, 0.69, 0.22)
  }
}
"#;

const TEMPLATE_SCENE: &str = r#"cinematic "intro" {
  layer glow_center {
    circle(0.2) | glow(3.0) | tint(0.83, 0.69, 0.22)
  }

  arc {
    glow_center.radius: 0.05 -> 0.3 over 3s ease_out
  }
}

cinematic "main" {
  layer bg {
    warp(3.0, 4, 0.5, 2.0, 0.3) | fbm(2.0, 4) | palette(ocean)
  }

  layer indicator blend: add {
    ring(0.2, 0.01) | glow(2.5) | tint(0.2, 0.8, 0.4)
  }
}

scene "demo" {
  play "intro" for 5s
  transition dissolve over 1s
  play "main" for 10s
}
"#;

// ── Shared state for dev server ─────────────────────────────

struct CompiledState {
    html: String,
    version: u64,
}

// ── Preview HTML generation ─────────────────────────────────

fn generate_slider_html(uniforms: &[game_compiler::codegen::UniformInfo]) -> String {
    let mut html = String::new();
    for u in uniforms {
        let max = if u.default > 1.0 {
            u.default * 3.0
        } else if u.default == 0.0 {
            1.0
        } else {
            f64::max(1.0, u.default * 5.0)
        };
        html.push_str(&format!(
            r#"<div class="param-group">
  <label>{name} <span class="value" id="val-{name}">{default:.3}</span></label>
  <input type="range" class="param-slider" data-param="{name}"
         min="0" max="{max}" step="0.001" value="{default}">
</div>
"#,
            name = u.name,
            default = u.default,
            max = max,
        ));
    }
    html
}

fn generate_preview_html(
    tag: &str,
    name: &str,
    compiled_js: &str,
    uniforms: &[game_compiler::codegen::UniformInfo],
    build_version: u64,
) -> String {
    let sliders_html = generate_slider_html(uniforms);

    format!(
        r##"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>GAME Preview — {name}</title>
  <style>
    * {{ margin: 0; padding: 0; box-sizing: border-box; }}
    body {{ background: #0a0a0a; color: #a0a0a0; font-family: 'Inter', system-ui, sans-serif; overflow: hidden; }}
    #viewport {{ position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; }}
    game-{tag} {{ display: block; width: 100%; height: 100%; }}
    #panel {{ position: fixed; right: 0; top: 0; width: 280px; height: 100vh;
             background: rgba(20,20,20,0.9); backdrop-filter: blur(10px);
             padding: 16px; overflow-y: auto; z-index: 10;
             border-left: 1px solid #2a2a2a; }}
    #panel h2 {{ color: #fff; font-size: 14px; margin-bottom: 12px; }}
    .param-group {{ margin-bottom: 12px; }}
    .param-group label {{ display: block; font-size: 11px; color: #666; margin-bottom: 4px; }}
    .param-group input[type=range] {{ width: 100%; accent-color: #d4af37; }}
    .param-group .value {{ font-size: 11px; color: #d4af37; float: right; }}
    #fps {{ position: fixed; left: 12px; bottom: 12px; font-size: 11px; color: #666; z-index: 10; }}
    #controls {{ position: fixed; left: 12px; top: 12px; z-index: 10; }}
    #controls button {{ background: #1f1f1f; border: 1px solid #2a2a2a; color: #a0a0a0;
                        padding: 6px 12px; margin-right: 8px; cursor: pointer; font-size: 11px; }}
    #controls button:hover {{ background: #2a2a2a; color: #fff; }}
    #controls button.active {{ border-color: #d4af37; color: #d4af37; }}
  </style>
</head>
<body>
  <div id="viewport">
    <game-{tag} id="component"></game-{tag}>
  </div>

  <div id="panel">
    <h2>GAME // {name}</h2>
    {sliders_html}
  </div>

  <div id="controls">
    <button onclick="togglePanel()">Params</button>
    <button id="audioBtn" onclick="toggleAudio()">Audio</button>
    <button onclick="toggleFullscreen()">Fullscreen</button>
  </div>

  <div id="fps">-- fps</div>

  <script>{compiled_js}</script>
  <script>
    // Parameter sliders
    const comp = document.getElementById('component');
    document.querySelectorAll('.param-slider').forEach(slider => {{
      slider.addEventListener('input', e => {{
        const name = e.target.dataset.param;
        const val = parseFloat(e.target.value);
        if (comp.setParam) comp.setParam(name, val);
        document.getElementById('val-' + name).textContent = val.toFixed(3);
      }});
    }});

    // FPS counter
    let frames = 0, lastTime = performance.now();
    function countFPS() {{
      frames++;
      const now = performance.now();
      if (now - lastTime >= 1000) {{
        document.getElementById('fps').textContent = frames + ' fps';
        frames = 0;
        lastTime = now;
      }}
      requestAnimationFrame(countFPS);
    }}
    countFPS();

    // Audio toggle
    let audioCtx = null, analyser = null, audioStream = null;
    async function toggleAudio() {{
      const btn = document.getElementById('audioBtn');
      if (audioCtx) {{
        audioCtx.close();
        audioCtx = null;
        if (audioStream) audioStream.getTracks().forEach(t => t.stop());
        btn.classList.remove('active');
        return;
      }}
      try {{
        audioStream = await navigator.mediaDevices.getUserMedia({{ audio: true }});
        audioCtx = new AudioContext();
        analyser = audioCtx.createAnalyser();
        analyser.fftSize = 256;
        audioCtx.createMediaStreamSource(audioStream).connect(analyser);
        const data = new Uint8Array(analyser.frequencyBinCount);
        btn.classList.add('active');

        function pumpAudio() {{
          if (!audioCtx) return;
          analyser.getByteFrequencyData(data);
          const n = data.length;
          const bass = Array.from(data.slice(0, n/4)).reduce((a,b) => a+b, 0) / (n/4) / 255;
          const mid = Array.from(data.slice(n/4, n/2)).reduce((a,b) => a+b, 0) / (n/4) / 255;
          const treble = Array.from(data.slice(n/2)).reduce((a,b) => a+b, 0) / (n/2) / 255;
          const energy = (bass + mid + treble) / 3;
          if (comp.setAudioData) comp.setAudioData({{ bass, mid, treble, energy, beat: bass > 0.6 ? 1 : 0 }});
          requestAnimationFrame(pumpAudio);
        }}
        pumpAudio();
      }} catch(e) {{ console.warn('Audio access denied:', e); }}
    }}

    // Panel toggle
    let panelVisible = true;
    function togglePanel() {{
      panelVisible = !panelVisible;
      document.getElementById('panel').style.display = panelVisible ? 'block' : 'none';
    }}

    // Fullscreen
    function toggleFullscreen() {{
      if (!document.fullscreenElement) document.documentElement.requestFullscreen();
      else document.exitFullscreen();
    }}

    // Hot reload polling
    let currentVersion = '{build_version}';
    setInterval(async () => {{
      try {{
        const resp = await fetch('/version');
        const ver = await resp.text();
        if (ver !== currentVersion) location.reload();
      }} catch(e) {{}}
    }}, 500);
  </script>
</body>
</html>"##,
        tag = tag,
        name = name,
        compiled_js = compiled_js,
        sliders_html = sliders_html,
        build_version = build_version,
    )
}

// ── Compile helper ──────────────────────────────────────────

struct CompileResult {
    js: String,
    tag: String,
    name: String,
    uniforms: Vec<game_compiler::codegen::UniformInfo>,
    cinematic_count: usize,
    uniform_count: usize,
}

fn compile_game_file(path: &std::path::Path) -> Result<CompileResult> {
    let source =
        std::fs::read_to_string(path).with_context(|| format!("read: {}", path.display()))?;

    let config = CompileConfig {
        output_format: OutputFormat::Component,
        target: ShaderTarget::Both,
        seed: None,
    };

    let program = game_compiler::compile_to_ast(&source).map_err(|e| anyhow::anyhow!("{e}"))?;

    let cinematic_count = program.cinematics.len();

    // Get uniforms from the first cinematic for the preview panel
    let uniforms = if let Some(cin) = program.cinematics.first() {
        game_compiler::codegen::extract_uniforms_public(cin)
    } else {
        vec![]
    };
    let uniform_count = uniforms.len();

    let results = game_compiler::compile(&source, &config).map_err(|e| anyhow::anyhow!("{e}"))?;

    let first = results
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("no cinematics found in file"))?;

    let tag = first.name.clone();
    let display_name = tag.replace('-', " ");

    Ok(CompileResult {
        js: first.js,
        tag,
        name: display_name,
        uniforms,
        cinematic_count,
        uniform_count,
    })
}

// ── Dev server ──────────────────────────────────────────────

fn run_dev_server(input: PathBuf, port: u16) -> Result<()> {
    let input = std::fs::canonicalize(&input)
        .with_context(|| format!("resolve path: {}", input.display()))?;

    // Initial compile
    eprintln!("[game dev] compiling {}...", input.display());
    let start = Instant::now();
    let result = compile_game_file(&input)?;
    let elapsed = start.elapsed();
    eprintln!(
        "[game dev] compiled ({:.0}ms) — {} cinematic(s), {} uniform(s)",
        elapsed.as_secs_f64() * 1000.0,
        result.cinematic_count,
        result.uniform_count,
    );

    let initial_version: u64 = 1;
    let html = generate_preview_html(
        &result.tag,
        &result.name,
        &result.js,
        &result.uniforms,
        initial_version,
    );

    let state = Arc::new(Mutex::new(CompiledState {
        html,
        version: initial_version,
    }));

    // Start HTTP server
    let addr = format!("127.0.0.1:{}", port);
    let server = tiny_http::Server::http(&addr)
        .map_err(|e| anyhow::anyhow!("failed to start HTTP server on {}: {}", addr, e))?;
    let server = Arc::new(server);

    eprintln!("[game dev] serving preview at http://localhost:{}", port);
    eprintln!("[game dev] watching for changes (Ctrl+C to stop)");

    // Spawn file watcher thread
    let state_watcher = Arc::clone(&state);
    let input_watcher = input.clone();
    let watcher_thread = std::thread::spawn(move || {
        use notify::{Event, EventKind, RecursiveMode, Watcher};
        use std::sync::mpsc;

        let (tx, rx) = mpsc::channel();
        let mut watcher = match notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_)) {
                    let _ = tx.send(());
                }
            }
        }) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("[game dev] failed to start file watcher: {}", e);
                return;
            }
        };

        let watch_path = input_watcher.parent().unwrap_or(&input_watcher);
        if let Err(e) = watcher.watch(watch_path, RecursiveMode::NonRecursive) {
            eprintln!("[game dev] failed to watch {}: {}", watch_path.display(), e);
            return;
        }

        // Debounce: wait 50ms after last event before recompiling
        loop {
            match rx.recv() {
                Ok(()) => {
                    // Drain any additional events within 50ms
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    while rx.try_recv().is_ok() {}

                    eprintln!("[game dev] change detected, recompiling...");
                    let start = Instant::now();
                    match compile_game_file(&input_watcher) {
                        Ok(result) => {
                            let elapsed = start.elapsed();
                            let mut locked = state_watcher.lock().unwrap_or_else(|e| e.into_inner());
                            locked.version += 1;
                            locked.html = generate_preview_html(
                                &result.tag,
                                &result.name,
                                &result.js,
                                &result.uniforms,
                                locked.version,
                            );
                            eprintln!(
                                "[game dev] recompiled ({:.0}ms) — {} cinematic(s), {} uniform(s)",
                                elapsed.as_secs_f64() * 1000.0,
                                result.cinematic_count,
                                result.uniform_count,
                            );
                        }
                        Err(e) => {
                            eprintln!("[game dev] compile error: {}", e);
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Main thread: handle HTTP requests
    let state_server = Arc::clone(&state);
    loop {
        match server.recv() {
            Ok(request) => {
                let url = request.url().to_string();
                let locked = state_server.lock().unwrap_or_else(|e| e.into_inner());

                match url.as_str() {
                    "/version" => {
                        let version_str = locked.version.to_string();
                        let response = tiny_http::Response::from_string(version_str)
                            .with_header(
                                tiny_http::Header::from_bytes(
                                    &b"Content-Type"[..],
                                    &b"text/plain; charset=utf-8"[..],
                                )
                                .unwrap(),
                            )
                            .with_header(
                                tiny_http::Header::from_bytes(
                                    &b"Cache-Control"[..],
                                    &b"no-cache"[..],
                                )
                                .unwrap(),
                            );
                        let _ = request.respond(response);
                    }
                    _ => {
                        let response = tiny_http::Response::from_string(&locked.html)
                            .with_header(
                                tiny_http::Header::from_bytes(
                                    &b"Content-Type"[..],
                                    &b"text/html; charset=utf-8"[..],
                                )
                                .unwrap(),
                            )
                            .with_header(
                                tiny_http::Header::from_bytes(
                                    &b"Cache-Control"[..],
                                    &b"no-cache"[..],
                                )
                                .unwrap(),
                            );
                        let _ = request.respond(response);
                    }
                }
            }
            Err(e) => {
                eprintln!("[game dev] server error: {}", e);
                break;
            }
        }
    }

    let _ = watcher_thread.join();
    Ok(())
}

// ── Main ────────────────────────────────────────────────────

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
                    FormatArg::Split => OutputFormat::Split,
                    FormatArg::Html => OutputFormat::Html,
                    FormatArg::Standalone => OutputFormat::Standalone,
                    FormatArg::Artblocks => OutputFormat::ArtBlocks,
                },
                target: match target {
                    TargetArg::Webgpu => ShaderTarget::WebGpu,
                    TargetArg::Webgl2 => ShaderTarget::WebGl2,
                    TargetArg::Both => ShaderTarget::Both,
                },
                seed: None,
            };

            std::fs::create_dir_all(&output_dir)
                .with_context(|| format!("create output dir: {}", output_dir.display()))?;

            for path in &input {
                eprintln!("[game] compiling {}", path.display());
                let source = std::fs::read_to_string(path)
                    .with_context(|| format!("read: {}", path.display()))?;

                let results =
                    game_compiler::compile(&source, &config).map_err(|e| anyhow::anyhow!("{e}"))?;

                for output in &results {
                    let stem = &output.name;

                    // Write JS component
                    let js_path = output_dir.join(format!("{stem}.js"));
                    std::fs::write(&js_path, &output.js)
                        .with_context(|| format!("write: {}", js_path.display()))?;
                    eprintln!("[game] wrote {}", js_path.display());

                    // Write TypeScript definitions
                    if let Some(dts) = &output.dts {
                        let dts_path = output_dir.join(format!("{stem}.d.ts"));
                        std::fs::write(&dts_path, dts)
                            .with_context(|| format!("write: {}", dts_path.display()))?;
                        eprintln!("[game] wrote {}", dts_path.display());
                    }

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

        Command::Dev { input, port } => {
            run_dev_server(input, port)?;
        }

        Command::New { template, output } => {
            let content = match template {
                TemplateArg::Minimal => TEMPLATE_MINIMAL,
                TemplateArg::Audio => TEMPLATE_AUDIO,
                TemplateArg::Particles => TEMPLATE_PARTICLES,
                TemplateArg::Procedural => TEMPLATE_PROCEDURAL,
                TemplateArg::Composition => TEMPLATE_COMPOSITION,
                TemplateArg::Reactive => TEMPLATE_REACTIVE,
                TemplateArg::Sdf => TEMPLATE_SDF,
                TemplateArg::Scene => TEMPLATE_SCENE,
            };

            let template_name = match template {
                TemplateArg::Minimal => "minimal",
                TemplateArg::Audio => "audio",
                TemplateArg::Particles => "particles",
                TemplateArg::Procedural => "procedural",
                TemplateArg::Composition => "composition",
                TemplateArg::Reactive => "reactive",
                TemplateArg::Sdf => "sdf",
                TemplateArg::Scene => "scene",
            };

            let out_path =
                output.unwrap_or_else(|| PathBuf::from(format!("{}.game", template_name)));

            if out_path.exists() {
                anyhow::bail!("file already exists: {}", out_path.display());
            }

            // Create parent directories if they don't exist
            if let Some(parent) = out_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("create directory: {}", parent.display()))?;
                }
            }

            std::fs::write(&out_path, content)
                .with_context(|| format!("write: {}", out_path.display()))?;
            eprintln!(
                "[game new] created {} (template: {})",
                out_path.display(),
                template_name
            );
        }

        Command::Validate { input } => {
            let source = std::fs::read_to_string(&input)
                .map_err(|e| anyhow::anyhow!("read: {}: {}", input.display(), e))?;
            match game_compiler::compile_to_ast(&source) {
                Ok(program) => {
                    if program.cinematics.is_empty()
                        && program.breeds.is_empty()
                        && program.scenes.is_empty()
                    {
                        eprintln!(
                            "warning: {} contains no components",
                            input.display()
                        );
                    }
                    // Also validate each cinematic's pipeline
                    for cinematic in &program.cinematics {
                        game_compiler::codegen::validate(cinematic, &program.fns)?;
                    }
                    println!("ok");
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        }

        #[cfg(feature = "lsp")]
        Command::Lsp => {
            game_compiler::lsp::run_lsp();
        }

        Command::Info { category } => match category.as_str() {
            "builtins" | "b" => print_builtins(),
            "palettes" | "p" => print_palettes(),
            "templates" | "t" => print_templates(),
            _ => {
                print_builtins();
                println!();
                print_palettes();
                println!();
                print_templates();
            }
        },
    }

    Ok(())
}

fn print_builtins() {
    println!("GAME Builtins");
    println!("=============");
    let items = game_compiler::builtins::completions();
    let mut sdf_gen = Vec::new();
    let mut transforms = Vec::new();
    let mut bridges = Vec::new();
    let mut color_proc = Vec::new();
    let mut sdf_mod = Vec::new();

    for item in &items {
        match (item.input.as_str(), item.output.as_str()) {
            ("Position", "Sdf") => sdf_gen.push(&item.signature),
            ("Position", "Position") => transforms.push(&item.signature),
            ("Sdf", "Color") => bridges.push(&item.signature),
            ("Color", "Color") => color_proc.push(&item.signature),
            ("Sdf", "Sdf") => sdf_mod.push(&item.signature),
            _ => {}
        }
    }

    println!("\n  SDF Generators (Position -> SDF):");
    for sig in &sdf_gen {
        println!("    {sig}");
    }
    println!("\n  Transforms (Position -> Position):");
    for sig in &transforms {
        println!("    {sig}");
    }
    println!("\n  Bridges (SDF -> Color):");
    for sig in &bridges {
        println!("    {sig}");
    }
    println!("\n  Color Processors (Color -> Color):");
    for sig in &color_proc {
        println!("    {sig}");
    }
    println!("\n  Shape Modifiers (SDF -> SDF):");
    for sig in &sdf_mod {
        println!("    {sig}");
    }
}

fn print_palettes() {
    println!("Named Palettes (30)");
    println!("===================");
    let palettes = [
        ("fire", "warm reds/oranges"),
        ("ocean", "cool blues"),
        ("neon", "vibrant rainbow"),
        ("aurora", "green/purple northern lights"),
        ("sunset", "warm orange/pink horizon"),
        ("ice", "cool blue/white"),
        ("ember", "deep red/orange coals"),
        ("lava", "volcanic orange/red"),
        ("magma", "bright orange to deep red"),
        ("inferno", "yellow-white to deep red"),
        ("plasma", "purple/pink energy"),
        ("electric", "bright cyan/blue"),
        ("cyber", "neon green/cyan"),
        ("matrix", "green-on-black terminal"),
        ("forest", "deep greens/warm browns"),
        ("moss", "muted greens/earth"),
        ("earth", "brown/tan/olive"),
        ("desert", "warm sand/terracotta"),
        ("blood", "dark to bright red"),
        ("rose", "pink/rose/magenta"),
        ("candy", "bright pink/purple/blue"),
        ("royal", "deep purple/gold"),
        ("deep_sea", "dark blue/cyan"),
        ("coral", "warm coral/orange/pink"),
        ("arctic", "white/light blue cold"),
        ("twilight", "purple/orange horizon"),
        ("vapor", "vaporwave purple/pink/teal"),
        ("gold", "warm gold/amber"),
        ("silver", "cool gray/white"),
        ("monochrome", "grayscale"),
    ];
    for (name, desc) in &palettes {
        println!("  {name:15} — {desc}");
    }
}

fn print_templates() {
    println!("Templates (8)");
    println!("=============");
    println!("  minimal      — single layer, basic circle + glow");
    println!("  audio        — audio-reactive with listen block");
    println!("  particles    — particle field with curl noise flow");
    println!("  procedural   — terrain-like FBM + voronoi + palette");
    println!("  composition  — multi-layer blend modes");
    println!("  reactive     — audio-reactive with domain warping");
    println!("  sdf          — SDF boolean operations showcase");
    println!("  scene        — scene sequencing with transitions");
    println!();
    println!("  Usage: game new --template <name> [-o output.game]");
}
