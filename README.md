<p align="center">
  <strong>GAME</strong><br>
  <em>Generative Animation Matrix Engine</em>
</p>

<p align="center">
  A compiler that turns <code>.game</code> files into zero-dependency, GPU-accelerated Web Components.
</p>

<p align="center">
  <a href="#install">Install</a> &bull;
  <a href="#quick-start">Quick Start</a> &bull;
  <a href="#the-language">Language</a> &bull;
  <a href="#cli">CLI</a> &bull;
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

## Install

```bash
# Build from source (requires Rust)
cd game-compiler
cargo build --release

# Binary is at game-compiler/target/release/game
```

## Quick Start

**1. Write a `.game` file**

```game
cinematic "Hello" {
  layer {
    fn: circle(0.3 + sin(time) * 0.05) | glow(2.0)
  }
}
```

**2. Compile it**

```bash
# To a self-contained HTML file
game compile hello.game --html -o hello.html

# To a Web Component
game compile hello.game --component -o game-hello.js
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
# → http://localhost:3333
# → Watches for changes, live-reloads browser
# → Split view: HTML preview + component embed
```

---

## The Language

### Pipe chains

The `fn:` property defines a chain of operations piped left-to-right:

```game
fn: circle(0.3) | glow(2.0)           # glowing circle
fn: ring(0.3, 0.04) | rotate(time)    # spinning ring
fn: sphere(0.5) | shade(albedo: gold)  # golden sphere (3D)
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

### Signal sources

| Signal | Description |
|--------|-------------|
| `audio.bass`, `.mid`, `.treble`, `.energy` | FFT frequency bands |
| `mouse.x`, `mouse.y` | Cursor position (0..1) |
| `data.*` | Bound to component properties |
| `time` | Elapsed seconds |
| `sin(time)`, `cos(time)` | Any math expression |

### Rendering modes

```game
# 2D (default) — fragment shader on a fullscreen quad
cinematic { layer { fn: circle(0.3) | glow(2.0) } }

# 3D — SDF raymarching with camera and lighting
cinematic {
  layer { fn: fbm(p * 2.0, octaves: 6) | shade(albedo: gold) }
  lens { mode: raymarch  camera: orbit(radius: 4.0) }
}
```

See [LANGUAGE.md](LANGUAGE.md) for the full specification and [PRIMITIVES.md](PRIMITIVES.md) for all built-in functions.

---

## CLI

```
game compile <file>                      # WGSL shader output (default)
game compile <file> --html -o out.html   # Self-contained HTML file
game compile <file> --component          # ES module Web Component
game compile <file> --component --tag my-widget  # Custom element name
game dev <file> [--port 3333]            # Hot-reload dev server
game build <dir> [--outdir dist/]        # Batch compile directory
```

### Tag name derivation

The component tag name is derived from the filename:

| Filename | Tag |
|----------|-----|
| `loading-ring.game` | `<loading-ring>` |
| `spinner.game` | `<game-spinner>` |
| `001-hello.game` | `<game-hello>` |

Override with `--tag`:

```bash
game compile spinner.game --component --tag my-spinner
```

---

## Presets

Ready-to-use `.game` files in `presets/`:

| Preset | Tag | Data API | Description |
|--------|-----|----------|-------------|
| `loading-ring.game` | `<game-loading-ring>` | `progress` (0..1) | Arc loading indicator |
| `status-pulse.game` | `<game-status-pulse>` | `health` (0..1) | Glowing health indicator |
| `metric-ring.game` | `<game-metric-ring>` | `value` (0..1) | Circular metric gauge |
| `breathing-dot.game` | `<game-breathing-dot>` | none | Ambient breathing animation |
| `spinner.game` | `<game-spinner>` | none | Rotating ring spinner |

### Build all presets

```bash
game build presets/ --outdir dist/
```

### Use a preset

```html
<script type="module" src="dist/game-loading-ring.js"></script>

<game-loading-ring
  progress="0.75"
  style="width: 64px; height: 64px"
></game-loading-ring>

<script>
  // Live data binding
  const ring = document.querySelector('game-loading-ring');
  ring.progress = downloadProgress; // updates GPU shader in real-time
</script>
```

---

## Web Component API

Every compiled component follows the same pattern:

```js
// ES module import
import { GameLoadingRing } from './game-loading-ring.js';

// Or just use the tag (auto-registers via customElements.define)
```

**Properties:** Set via JS properties or HTML attributes. Each `data.*` signal in the `.game` source becomes a property.

```js
element.progress = 0.5;  // JS property (preferred, no string conversion)
```

```html
<game-loading-ring progress="0.5"></game-loading-ring>  <!-- HTML attribute -->
```

**Lifecycle:** Components initialize WebGPU on `connectedCallback` and clean up all GPU resources on `disconnectedCallback`. Safe to add/remove from the DOM.

**Shadow DOM:** Rendering is fully encapsulated. No style leakage in or out.

**Sizing:** Components fill their container. Set `width` and `height` on the element.

---

## Examples

The `examples/` directory contains `.game` files demonstrating language features:

| File | Features |
|------|----------|
| `001-hello.game` | Minimal — breathing circle |
| `002-audio-reactive.game` | Audio modulation, FFT bands |
| `003-interactive.game` | Mouse input, impulse signals |
| `004-resonance.game` | Cross-layer feedback |
| `005-audio-hello.game` | Audio + parameter binding |
| `006-spectrum.game` | Multi-band audio visualization |
| `007-showcase.game` | Multiple features combined |
| `008-mouse-follow.game` | Mouse tracking, translate |

---

## Architecture

```
.game source → [Lexer] → [Parser] → [Codegen] → [Runtime wrapper]
                                         ↓
                              ┌──────────┴──────────┐
                              ↓                     ↓
                        WGSL shader          CompileOutput
                              ↓                     ↓
                     ┌────────┴────────┐    ┌───────┴───────┐
                     ↓                 ↓    ↓               ↓
               HTML file      Web Component    WGSL only
              (standalone)    (ES module)     (raw shader)
```

The compiler is written in Rust. The output is pure JavaScript + WGSL — no Rust/WASM in the browser.

| Module | Purpose |
|--------|---------|
| `lexer.rs` | Tokenizer (logos) |
| `parser.rs` | Recursive descent parser |
| `ast.rs` | Abstract syntax tree types |
| `codegen.rs` | WGSL shader generation |
| `runtime.rs` | HTML + Web Component wrappers |
| `server.rs` | Dev server (axum + file watcher) |
| `main.rs` | CLI (clap) |

---

## Requirements

- **Rust** 1.70+ (to build the compiler)
- **WebGPU** (to run the output — Chrome 113+, Firefox 121+, Safari 18+)

The compiled output has **zero runtime dependencies**. No npm install. No bundler. No framework.

---

## License

[MIT](LICENSE)
