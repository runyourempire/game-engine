# GAME — Generative Animation Matrix Engine

A compiler for the `.game` visual component language. Write declarative visual specifications, get production WebGPU shaders and Web Components.

```
cinematic "Hello" {
  layer {
    fn: circle(0.3) | glow(2.0)
  }
}
```

Three lines. A breathing circle of light. Zero dependencies. Ships as a `<game-hello>` custom element.

## What It Does

`.game` files compile to:

- **WGSL shaders** — raw GPU code for any WebGPU renderer
- **Web Components** — self-contained ES modules with `<custom-element>` API
- **HTML files** — standalone pages with full audio/mouse interactivity

One language produces shader art, data visualization components, and audio-reactive visuals. The dev server lets you edit, debug, and export — all in one tool.

## Install

```bash
cargo install game-compiler
```

Or build from source:

```bash
git clone https://github.com/runyourempire/game-engine.git
cd game-engine/game-compiler
cargo build --release
```

## Quick Start

```bash
# Compile to WGSL shader
game compile hello.game

# Compile to Web Component
game compile hello.game --component -o hello.js

# Compile to standalone HTML
game compile hello.game --html -o hello.html

# Launch dev server with hot reload
game dev hello.game

# Batch compile a directory
game build examples/ --outdir dist/
```

## Dev Server

`game dev <file>` launches a full development environment at `http://localhost:3333`:

| Feature | Key | Description |
|---------|-----|-------------|
| **Preview** | `1` | Live-rendered component with size controls (S/M/L) |
| **WGSL Viewer** | `2` | Syntax-highlighted generated shader code |
| **API Docs** | `3` | Auto-generated component API with copy-paste imports |
| **Live Editor** | `4` | Edit `.game` source in-browser, instant recompilation |
| **X-Ray Mode** | `X` | Click pipe stages to isolate each visual contribution |
| **Pixel Autopsy** | `P` | Click any pixel for UV, RGBA, hex, distance breakdown |
| **Export** | toolbar | PNG screenshot, 5s WebM video, React/Vue wrappers, CSS fallback |
| **Timeline** | `Space` | Scrub arc animations with play/pause and arrow keys |
| **Shortcuts** | `?` | Full keyboard shortcut overlay |

The editor supports `Ctrl+S` to save changes back to disk, triggering hot reload.

## Snapshot Testing

Visual regression testing with headless GPU rendering (opt-in):

```bash
# Install with snapshot support
cargo install game-compiler --features snapshot

# Generate reference snapshots
game test examples/*.game --update

# Run regression tests (99% pixel similarity threshold)
game test examples/*.game

# Custom threshold and render size
game test examples/*.game --threshold 95 --size 512
```

Failed tests produce `.game.diff.png` files highlighting changed pixels in red.

## Language Reference

### Structure

Every `.game` file is a `cinematic` block containing layers:

```
cinematic "Name" {
  layer name {
    fn: shape(...) | modifier(...) | effect(...)
    param: base_value ~ modulation_expression
  }
}
```

### Primitives

| Shape | Syntax | Description |
|-------|--------|-------------|
| `circle` | `circle(radius)` | Circular disk |
| `ring` | `ring(radius, thickness)` | Annulus/donut |
| `box` | `box(width, height)` | Rectangle |
| `polygon` | `polygon(sides, radius)` | Regular polygon |
| `star` | `star(points, outer_r, inner_r)` | Star shape |
| `line` | `line(x1, y1, x2, y2, thickness)` | Line segment |
| `torus` | `torus(major_r, minor_r)` | Torus (ring alias) |
| `fbm` | `fbm(pos, octaves: N, persistence: P)` | Fractal noise |
| `simplex` | `simplex(frequency)` | Simplex noise |
| `voronoi` | `voronoi(frequency)` | Voronoi cells |

### Transforms

| Transform | Syntax | Effect |
|-----------|--------|--------|
| `translate` | `translate(x, y)` | Move in space |
| `rotate` | `rotate(radians)` | Rotate |
| `scale` | `scale(factor)` | Zoom |
| `repeat` | `repeat(spacing)` | Tile infinitely |
| `mirror` | `mirror("x"/"y"/"xy")` | Mirror across axis |
| `twist` | `twist(amount)` | Spiral distortion |

### SDF Modifiers

| Modifier | Syntax | Effect |
|----------|--------|--------|
| `onion` | `onion(thickness)` | Convert solid to outline |
| `round` | `round(radius)` | Round corners |
| `displace` | `displace(strength)` | Noise displacement |
| `mask_arc` | `mask_arc(angle)` | Circular arc mask (0–2pi) |

### Color & Lighting

| Stage | Syntax | Effect |
|-------|--------|--------|
| `glow` | `glow(intensity)` | SDF to light falloff |
| `tint` | `tint(color)` | Multiply by color |
| `shade` | `shade(albedo: color, emissive: color)` | Material shading |
| `gradient` | `gradient(color_a, color_b, "direction")` | Color gradient |
| `spectrum` | `spectrum(bass, mid, treble)` | Audio-reactive rings |

Named colors: `black`, `white`, `red`, `green`, `blue`, `gold`, `cyan`, `orange`, `ember`, `midnight`, `frost`, `ivory`, `deep_blue`

### Post-Processing

| Effect | Syntax |
|--------|--------|
| `bloom` | `bloom(threshold, intensity)` |
| `vignette` | `vignette(strength)` |
| `chromatic` | `chromatic(amount)` |
| `grain` | `grain(amount)` |
| `glitch` | `glitch(intensity)` |
| `scanlines` | `scanlines(count, intensity)` |
| `fog` | `fog(density)` |
| `tonemap` | `tonemap(exposure)` |
| `invert` | `invert()` |
| `saturate_color` | `saturate_color(amount)` |

### Parameters & Modulation

Parameters have a base value and optional modulation:

```
radius: 0.3 ~ audio.bass * 0.2
```

Modulation sources:
- `audio.bass`, `audio.mid`, `audio.treble`, `audio.energy`, `audio.beat`
- `mouse.x`, `mouse.y` (0–1 normalized)
- `data.fieldname` (Web Component attributes)
- `time` (seconds), math functions (`sin`, `cos`, `sqrt`, etc.)
- Constants: `pi`, `tau`, `e`, `phi`

### Data Fields

Parameters modulated by `data.*` become Web Component properties:

```
cinematic "Gauge" {
  layer {
    fn: ring(0.3, 0.04) | mask_arc(angle) | glow(2.0)
    angle: 0.0 ~ data.progress * 6.283
  }
}
```

```html
<game-gauge progress="0.75"></game-gauge>
<script>
  document.querySelector('game-gauge').progress = 0.82;
</script>
```

### Arc Timeline

Keyframe animations with easing:

```
arc {
  0:00 "intro" {
    brightness: 0.0
  }
  0:02 "ramp" {
    brightness -> 3.0 ease(expo_out) over 2s
  }
}
```

Easing functions: `linear`, `smooth`, `cubic_in_out`, `expo_in`, `expo_out`, `elastic`, `bounce`

### Define Blocks

Reusable macros:

```
cinematic {
  define glow_ring(r, t) {
    ring(r, t) | glow(2.0)
  }
  layer {
    fn: glow_ring(0.3, 0.04) | tint(cyan)
  }
}
```

## Examples

See [`examples/`](examples/) for compilable demos:

| File | Description |
|------|-------------|
| `hello.game` | Minimal — a glowing circle |
| `neon-ring.game` | Ring with glow, tint, bloom, and vignette |
| `galaxy.game` | Multi-layer: gradient background, tiled rings, golden core |
| `loading-ring.game` | Data-driven progress ring component |
| `starfield.game` | Tiled star outlines with onion modifier |
| `dashboard-gauge.game` | Multi-layer gauge with value + target arcs |

## Architecture

```
src/
  lexer.rs        Tokenizer (logos-based)
  parser.rs       Recursive descent parser → AST
  ast.rs          AST node definitions
  codegen/        WGSL code generation
    mod.rs        Main codegen orchestration
    stages.rs     Pipe stage processing
    builtins.rs   Built-in SDF/effect functions
    expr.rs       Expression compilation
    raymarch.rs   Raymarch mode codegen
  runtime.rs      Web Component JS runtime
  server/         Dev server (axum-based)
    mod.rs        Routes, handlers, state
    inline_js.rs  Client-side JS (editor, autopsy, export, x-ray)
    css.rs        All styles
    panels.rs     Tab panel content
    page.rs       HTML assembly
    toolbar.rs    Toolbar with tabs and actions
    timeline.rs   Arc timeline scrubber
    export.rs     React/Vue/CSS export generators
    util.rs       Shared helpers
  snapshot.rs     Headless GPU renderer (feature-gated)
  main.rs         CLI (clap-based)
  lib.rs          Library root + tests
```

## License

MIT
