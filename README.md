<p align="center">
  <strong>GAME</strong><br>
  <em>Generative Animation Matrix Engine</em>
</p>

<p align="center">
  A compiler that turns <code>.game</code> files into zero-dependency, GPU-accelerated Web Components.
</p>

<p align="center">
  <a href="#quick-start">Quick Start</a> &bull;
  <a href="#features">Features</a> &bull;
  <a href="#the-language">Language</a> &bull;
  <a href="#signals">Signals</a> &bull;
  <a href="#cli">CLI</a> &bull;
  <a href="#web-component-output">Output</a> &bull;
  <a href="#presets">Presets</a> &bull;
  <a href="#examples">Examples</a> &bull;
  <a href="LANGUAGE.md">Spec</a>
</p>

---

## What Is This

GAME is a domain-specific language for generative visuals. You write a short text file describing mathematical fields, pipe them through transformations, bind them to live data — and the compiler produces a self-contained Web Component that renders on the GPU.

No runtime. No framework. No dependencies. One `.js` file. Works everywhere.

```game
cinematic "Loading Ring" {
  layer {
    fn: ring(0.3, 0.04) | mask_arc(angle) | glow(2.0)
    angle: 0.0 ~ data.progress * 6.283
  }
}
```

Compiles to:

```html
<script type="module" src="game-loading-ring.js"></script>
<game-loading-ring progress="0.75"></game-loading-ring>
```

That's a GPU-accelerated loading indicator in 5 lines of source. The output is a single ES module with zero dependencies.

---

## Quick Start

**1. Write a `.game` file**

```game
cinematic "Hello" {
  layer {
    fn: circle(0.3 + sin(time) * 0.05) | glow(2.0)
  }
}
```

This is the simplest possible `.game` file: a breathing circle with glow, driven by time.

**2. Compile it**

```bash
# Build from source (requires Rust)
cd game-compiler
cargo build --release

# Compile to a self-contained HTML file
./target/release/game compile hello.game --html > hello.html

# Compile to an ES module Web Component
./target/release/game compile hello.game --component > game-hello.js
```

**3. Use it**

```html
<!-- HTML: just open the file -->
open hello.html

<!-- Component: embed anywhere -->
<script type="module" src="game-hello.js"></script>
<game-hello style="width: 200px; height: 200px"></game-hello>
```

**4. Dev mode** (hot reload)

```bash
game dev hello.game
# -> http://localhost:3333
# -> Watches for changes, live-reloads browser
# -> Split view: preview + component + param sliders + WGSL viewer + inline editor
```

---

## Features

### Language Features

- **Pipe chains** — composable operations: `circle(0.3) | glow(2.0) | tint(1.0, 0.5, 0.0)`
- **Modulation (`~`)** — bind any parameter to a live signal: `radius: 0.3 ~ audio.bass * 0.2`
- **`define`** — reusable macros: `define glow_ring(r) { ring(r, 0.02) | glow(2.0) }`
- **`import`** — compose `.game` files: `import "stdlib/noise.game" expose fbm_field`
- **`memory`** — per-layer persistent state across frames (feedback effects, trails)
- **`cast`** — typed layer output (e.g. `cast point`, `cast field`, `cast color`)
- **`arc`** — timeline-driven parameter transitions with easing
- **`resonate`** — cross-layer modulation with weighted connections and damping
- **`listen`** — custom audio signal extraction (DSP algorithms on FFT data)
- **`voice`** — synthesis graph (oscillators, filters, output chains)
- **`score`** — musical composition (motifs, phrases, sections, tempo-synced arrangement)
- **`breed`** — genetic recombination of cinematics (inherit + mutate)
- **`gravity`** — particle physics (force laws, damping, boundary modes)
- **`project`** — spatial projection (flat, dome, cube, LED mapping)
- **`react`** — event-driven interactions (signal -> action bindings)
- **`lens`** — camera/post-processing configuration
- **Temporal operators** — `>>` (delay), `<>` (smooth), `!!` (trigger), `..` (range clamp)
- **Ternary expressions** — `cond ? a : b` for conditional logic

### 37 Built-in Functions

| Category | Functions |
|----------|-----------|
| SDF generators | `circle`, `ring`, `star`, `box`, `polygon`, `fbm`, `simplex`, `voronoi`, `concentric_waves` |
| SDF -> Color bridges | `glow`, `shade`, `emissive` |
| Color processors | `tint`, `bloom`, `grain`, `blend`, `vignette`, `tonemap`, `scanlines`, `chromatic`, `saturate_color`, `glitch` |
| SDF modifiers | `mask_arc`, `threshold`, `onion`, `round` |
| Position transforms | `translate`, `rotate`, `scale`, `twist`, `mirror`, `repeat`, `domain_warp`, `curl_noise`, `displace` |
| Full-screen generators | `gradient`, `spectrum` |

### Type State Machine

The compiler tracks data types through the pipe chain:

```
Position -> [SDF generator] -> Sdf -> [bridge] -> Color -> [color processor] -> Color
Position -> [transform] -> Position -> [SDF generator] -> Sdf -> ...
Position -> [full-screen generator] -> Color -> ...
```

Invalid transitions are caught at compile time.

### Compilation Pipeline

```
.game source -> Lexer (logos) -> Parser (recursive descent) -> Resolver (imports)
             -> Optimizer (constant fold + noop elimination + dead uniform detection)
             -> Codegen (WGSL + GLSL + JS) -> Runtime (Web Component / HTML)
```

### Adapters

Import external signal sources:

- `import "shadertoy://[id]"` — Shadertoy shader adapter
- `import "midi://[channel]"` — MIDI controller input
- `import "osc://[host]:[port]/[path]"` — OSC protocol input
- `import "camera://[device]"` — Webcam video input

---

## Signals

| Signal | Description |
|--------|-------------|
| `audio.bass`, `.mid`, `.treble`, `.energy` | FFT frequency bands |
| `mouse.x`, `mouse.y` | Cursor position (0..1) |
| `data.*` | Bound to component properties |
| `time` | Elapsed seconds |
| `sin(time)`, `cos(time)` | Any math expression |

---

## The Language

### Pipe chains

The `fn:` property defines a chain of operations piped left-to-right:

```game
fn: circle(0.3) | glow(2.0)           # glowing circle
fn: ring(0.3, 0.04) | rotate(time)    # spinning ring
fn: fbm(2.0, octaves: 6) | shade(r: 0.831, g: 0.686, b: 0.216)  # golden noise
```

### Parameters + modulation (`~`)

Parameters have a base value and an optional signal binding:

```game
layer {
  fn: circle(radius) | glow(intensity)
  radius: 0.3 ~ audio.bass * 0.2      # reacts to music
  intensity: 2.0 ~ data.health * 3.0   # reacts to live data
}
```

### Rendering modes

```game
# 2D (default) — fragment shader on a fullscreen quad
cinematic { layer { fn: circle(0.3) | glow(2.0) } }

# 3D — lens block configures raymarching, camera, post-processing
cinematic {
  layer { fn: fbm(2.0, octaves: 6) | shade(r: 0.831, g: 0.686, b: 0.216) }
  lens { mode: raymarch  camera: orbit(radius: 4.0) }
}
```

See [LANGUAGE.md](LANGUAGE.md) for the full specification and [PRIMITIVES.md](PRIMITIVES.md) for all built-in functions.

---

## CLI

```
game build <files...> [-o dist/] [-f component|html|standalone] [-t webgpu|webgl2|both]
game compile <file> [--html] [--component] [--tag name]
game dev <files...> [--port 3333]
```

### `build` — Batch compile

Compiles one or more `.game` files to an output directory. Produces `.js` (Web Component), `.html` (if html/standalone format), `.wgsl`, and `.frag` (GLSL) files.

### `compile` — Single file to stdout

Compiles a single `.game` file and prints the result to stdout. Useful for piping and scripting.

### `dev` — Hot-reload dev server

Launches a dev server with live preview, WGSL inspector, inline editor, param sliders, and file watching with automatic recompilation.

### Tag name derivation

| Filename | Tag |
|----------|-----|
| `loading-ring.game` | `<game-loading-ring>` |
| `spinner.game` | `<game-spinner>` |
| `001-hello.game` | `<game-hello>` |

Override with `--tag`:

```bash
game compile spinner.game --component --tag my-spinner
```

---

## Web Component Output

Every compiled `.game` file produces a self-contained ES module that registers a custom element. No runtime dependencies.

### Usage

```html
<script type="module" src="game-loading-ring.js"></script>

<game-loading-ring
  progress="0.75"
  style="width: 64px; height: 64px"
></game-loading-ring>
```

### Data binding

Each `data.*` signal in the `.game` source becomes a property on the component:

```js
const ring = document.querySelector('game-loading-ring');

// JS property (preferred, no string conversion)
ring.progress = 0.75;

// HTML attribute
ring.setAttribute('progress', '0.75');

// Live updates drive the GPU shader in real-time
setInterval(() => { ring.progress = Math.random(); }, 100);
```

### Lifecycle

- **`connectedCallback`** — initializes WebGPU, creates pipeline, starts render loop
- **`disconnectedCallback`** — cleans up all GPU resources
- **Shadow DOM** — rendering is fully encapsulated, no style leakage
- **Sizing** — components fill their container; set `width` and `height` on the element

### Dual shader output

The compiler generates both WGSL (WebGPU) and GLSL (WebGL2) shaders. The runtime detects browser support and uses the appropriate backend.

### Framework wrappers

The `package/` directory includes wrappers for React, Vue, and Svelte:

```jsx
// React
import { GameComponent } from '@game/react';
<GameComponent src="./game-loading-ring.js" progress={0.75} />
```

---

## Presets

24 ready-to-use `.game` files in `presets/`:

| Preset | Description |
|--------|-------------|
| `loading-ring.game` | Arc loading indicator |
| `status-pulse.game` | Glowing health indicator |
| `metric-ring.game` | Circular metric gauge |
| `breathing-dot.game` | Ambient breathing animation |
| `spinner.game` | Rotating ring spinner |
| `dashboard-gauge.game` | Multi-layer data visualization |
| `achievement-ring.game` | Achievement progress ring |
| `boot-ring.game` | Boot sequence animation |
| `celebration.game` | Celebration burst effect |
| `engagement-bars.game` | Engagement level bars |
| `game-state-dashboard.game` | Game state overview |
| `level-up-burst.game` | Level-up burst effect |
| `loading-stages.game` | Multi-stage loading |
| `signal-dashboard.game` | Signal monitoring dashboard |
| `streak-flame.game` | Streak flame animation |
| `temporal-monitor.game` | Temporal data monitor |
| `void-heartbeat.game` | Ambient heartbeat pulse |
| `achievement-progress.game` | Achievement progress tracker |
| `arc-demo.game` | Arc system demonstration |
| `audio-layers.game` | Audio-reactive layer composition |
| `layered-scene.game` | Multi-layer scene composition |
| `route-matrix.game` | Route visualization matrix |
| `svg-badge.game` | Badge component |
| `synesthetic.game` | Synesthetic audio visualization |

### Build all presets

```bash
game build presets/*.game -o dist/
```

---

## Standard Library

6 stdlib modules in `stdlib/` for common patterns:

| Module | Contents |
|--------|----------|
| `primitives.game` | Reusable shape definitions |
| `noise.game` | Noise function compositions |
| `backgrounds.game` | Background patterns and gradients |
| `transitions.game` | Transition effect definitions |
| `post.game` | Post-processing chains |
| `ui.game` | UI component patterns |

```game
import "stdlib/noise.game" expose turbulent_field
```

---

## Examples

21 example files in `examples/` demonstrating language features:

| File | Features |
|------|----------|
| `001-hello.game` | Minimal breathing circle |
| `002-audio-reactive.game` | Audio-driven terrain |
| `002-multi-layer.game` | Multi-layer composition |
| `003-interactive.game` | Mouse interaction |
| `003-memory-trails.game` | Memory-based trail effects |
| `004-cast-types.game` | Cast type system |
| `004-resonance.game` | Cross-layer resonance |
| `005-audio-hello.game` | Basic audio reactivity |
| `005-temporal-ops.game` | Delay, smooth, trigger, range |
| `006-listen-signals.game` | Custom audio signal extraction |
| `006-spectrum.game` | Audio spectrum visualization |
| `007-showcase.game` | Feature showcase |
| `007-voice-synth.game` | Voice synthesis graph |
| `008-mouse-follow.game` | Cursor tracking with voronoi |
| `008-score-timeline.game` | Musical score composition |
| `009-breed-genetics.game` | Genetic recombination |
| `010-gravity-particles.game` | Particle physics |
| `011-project-dome.game` | Dome projection mapping |
| `012-ambient-intelligence.game` | Ambient data visualization |
| `013-score-fingerprint.game` | Score-driven visual fingerprint |
| `014-decision-countdown.game` | Temporal countdown effect |

---

## Dev Server

```bash
game dev my-component.game
# -> http://localhost:3333
```

Features:
- **Live reload** — edit the source file, browser updates instantly (via LiveReload)
- **Split view** — preview pane + component embed at configurable sizes (SM/MD/LG)
- **WGSL viewer** — inspect generated shader code with copy button
- **Inline editor** — edit `.game` source directly in the browser, compile and save
- **Param sliders** — auto-generated sliders for all uniform parameters

---

## Architecture

```
.game source -> [Lexer] -> [Parser] -> [Resolver] -> [Optimizer] -> [Codegen] -> [Runtime]
                                                                        |
                                                            +-----------+-----------+
                                                            |           |           |
                                                       WGSL+GLSL    JS Module    HTML shell
                                                        shaders   (Web Component) (standalone)
```

The compiler is written in Rust. The output is pure JavaScript + WGSL/GLSL — no Rust/WASM in the browser.

| Module | Purpose |
|--------|---------|
| `lexer.rs` | Tokenizer (logos) |
| `parser.rs` | Recursive descent parser |
| `resolver.rs` | Import resolution + define merging |
| `optimize.rs` | Constant folding, noop elimination, dead uniform detection |
| `ast.rs` | Abstract syntax tree types |
| `codegen/` | WGSL + GLSL shader generation (wgsl, glsl, stages, expr, + per-feature modules) |
| `runtime/` | Web Component + HTML wrappers + arc interpolation |
| `server/` | Dev server (axum + livereload + preview UI) |
| `adapters/` | External signal adapters (Shadertoy, MIDI, OSC, camera) |
| `wasm.rs` | WASM bindings (optional, `--features wasm`) |
| `main.rs` | CLI (clap) |

---

## Ecosystem

- **VS Code extension** (`vscode-game/`) — syntax highlighting, snippets, language configuration
- **npm package** (`package/`) — `create-game` scaffolding + React/Vue/Svelte wrappers
- **WASM module** — compile `.game` files in the browser (`--features wasm`)
- **Playground** (`playground/`) — browser-based WASM playground
- **Showcase** (`showcase.html`) — all components rendering live with interactive sliders

---

## Requirements

- **Rust** 1.70+ (to build the compiler)
- **WebGPU** (to run the output — Chrome 113+, Firefox 121+, Safari 18+)

The compiled output has **zero runtime dependencies**. No npm install. No bundler. No framework.

---

## License

[MIT](LICENSE)
