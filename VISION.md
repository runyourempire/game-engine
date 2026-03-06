# GAME v1.0 Vision — Revolutionizing Generative Animation

## The Thesis

Every generative animation tool today forces a choice: accessibility OR power. Shadertoy gives you raw GPU access but no abstraction. TouchDesigner gives you nodes but costs $2,200 and locks you in. Processing is accessible but not GPU-native. Hydra is magical but interpreted JavaScript that can't leave the browser tab.

GAME eliminates this tradeoff. A declarative DSL that reads like describing a painting, compiles to GPU-optimal WGSL + GLSL, and outputs zero-dependency Web Components that work anywhere. No runtime. No framework. No lock-in.

**The unique advantage no competitor has:** GAME's output is a self-contained `.js` file. Drop it into any web page. It just works. No npm install, no build step, no dependencies. This is the distribution moat.

---

## Current State: v0.3.0

- 13,690 lines of clean Rust (logos lexer, hand-written recursive descent parser)
- Full pipeline: `.game` source → Lex → Parse → Validate → Codegen → Runtime
- Dual-target: WGSL (WebGPU primary) + GLSL ES 3.0 (WebGL2 fallback)
- 43 built-in functions across 3 shader states (Position, SDF, Color)
- SDF algebra: boolean ops, spatial ops, shape modifiers, 9 new primitives
- 15+ synthesis techniques: temporal operators, audio DSP, voice synthesis, musical timelines, genetic composition, N-body gravity, reaction-diffusion, physarum stigmergy, vector field advection
- Live preview: `game dev` with HTTP server, file watcher, hot reload, parameter sliders
- Scaffolding: `game new` with 5 templates
- 34 example `.game` files, 234 passing tests
- 9 components deployed in production (4DA)

---

## Competitive Landscape

| Tool | Paradigm | Output | Audio | Compute | Composable | Free | Web-Native |
|------|----------|--------|-------|---------|------------|------|------------|
| **GAME** | DSL → compiled | Web Component (.js) | First-class | Yes | Planned | Yes | Yes |
| Shadertoy | Raw GLSL | Fragment shader only | Texture hack | No | No | Yes | Yes |
| TouchDesigner | Node graph | Project file | Yes | Yes | Yes | No ($2,200) | No |
| Cables.gl | Node graph | Web app | Partial | No | Yes | Freemium | Yes |
| Hydra | Live-code JS | Browser tab | Partial | No | Yes | Yes | Yes |
| ISF | JSON + GLSL | Fragment shader | Partial | No | No | Yes | Partial |
| Shader Park | JS API | Three.js mesh | Partial | No | Partial | Yes | Yes |
| KodeLife | GLSL editor | Shader file | Yes | No | No | Free tier | No |
| p5.js | JS framework | Canvas/WebGL | Partial | No | Partial | Yes | Yes |

**GAME's position:** The only tool that compiles a high-level DSL to self-contained, zero-dependency Web Components with first-class audio reactivity and GPU compute shaders.

---

## The Five Missing Layers

### 1. Shape Algebra — The Vocabulary

GAME can draw shapes. It cannot *compose* them. In SDF programming, the fundamental operations are boolean composition (union, subtract, intersect), spatial manipulation (repeat, mirror, symmetry), and morphing. Without these, GAME is a coloring book. With them, it's a sculptor's studio.

**Boolean Operations:**
```game
// Combine two shapes
union(circle(0.3), box(0.2, 0.4)) | glow(2.0)
subtract(circle(0.3), box(0.1, 0.1)) | glow(2.0)
intersect(ring(0.3, 0.05), star(5, 0.35, 0.15)) | glow(2.0)
smooth_union(circle(0.3), circle(0.3), 0.1) | glow(2.0)  // k = blend radius
```

**Spatial Operations:**
```game
// Infinite tiling
repeat(0.5, 0.5) | circle(0.1) | glow(2.0)

// Mirror symmetry
mirror() | star(5, 0.3, 0.15) | glow(2.0)

// Radial symmetry (kaleidoscope)
radial(6) | box(0.3, 0.05) | glow(2.0)
```

**Shape Modifiers:**
```game
// Round corners
round(0.05) | box(0.2, 0.3) | glow(2.0)

// Hollow out (shell)
shell(0.02) | circle(0.3) | glow(2.0)

// Concentric rings
onion(3, 0.02) | circle(0.3) | glow(2.0)
```

**New Primitives:** line, capsule, triangle, arc, cross, heart, egg, spiral, grid

**Morphing:**
```game
// Interpolate between shapes over time
morph(circle(0.3), star(5, 0.3, 0.15), time * 0.2) | glow(2.0)
```

### 2. Live Studio — The Feedback Loop

Creation without feedback isn't creation — it's writing letters into a void. The `game dev` command must become an instrument, not a build tool.

**Architecture:**
```
game dev example.game
  → File watcher (notify crate, debounced)
  → Compile on change (< 10ms for typical .game files)
  → HTTP server on localhost:4200
  → Server-Sent Events push reload signal
  → Browser hot-swaps shader strings (preserves time, params)
  → Preview HTML with auto-generated parameter sliders
  → Audio input toggle (microphone for reactive testing)
  → FPS counter, resolution picker, fullscreen toggle
```

**The key insight:** Don't reload the page. Hot-swap the shader module. The WebGPU renderer can `createShaderModule()` with new WGSL and rebuild just the render pipeline, preserving the uniform buffer, animation time, and parameter values. The visual updates in a single frame. This is Hydra-level responsiveness with compiled shader quality.

**Scaffolding:**
```
game new --template minimal    → circle | glow | tint
game new --template particles  → flow + gravity setup
game new --template audio      → listen + voice + reactive layers
game new --template procedural → fbm + warp + voronoi
game new --template composition → multi-layer with blend modes
```

### 3. Composition — The Architecture

Simple shapes become complex art through composition. GAME needs:

**Cinematic-as-Texture:**
```game
cinematic "background" {
  layer noise { fbm(3.0, 4, 0.5, 2.0) | palette(...) }
}

cinematic "foreground" {
  use "background" as bg_tex
  layer main { circle(0.3) | glow(2.0) | tint(bg_tex.r, bg_tex.g, bg_tex.b) }
}
```

**Frame Feedback:**
```game
cinematic "trails" {
  layer feedback memory: 0.97 {
    // Previous frame feeds into current frame
    // Already supported via memory buffers — extend to full feedback
  }
}
```

**Multi-Pass:**
```game
cinematic "bloom-pass" {
  pass blur_h { /* horizontal gaussian blur */ }
  pass blur_v { /* vertical gaussian blur */ }
  pass composite { /* combine original + blurred */ }
}
```

### 4. Generative Grammar — The Evolution

The `breed` system is the seed of something unprecedented. Extend it:

**Visual Genomes:**
```game
genome "particle-species" {
  shape: [circle, ring, star, hex]
  radius: range(0.05, 0.3)
  glow_strength: range(1.0, 5.0)
  color: palette(random)
  rotation_speed: range(0.0, 3.0)
}
```

**Procedural Population:**
```game
populate "field" from "particle-species" {
  count: 200
  seed: 42
  layout: grid(10, 20)
  variation: 0.4
}
```

**Interactive Evolution:**
```game
evolve "species-a" {
  parents: ["fire-particle", "ice-particle"]
  generations: 10
  fitness: visual_complexity  // built-in fitness functions
  survivors: 4
}
```

### 5. Distribution — The Reach

```
game build input.game                    → Web Component (.js)
game build input.game --html             → Standalone HTML demo
game build input.game --artblocks        → Deterministic HTML (fxhash compatible)
game build input.game --embed            → <iframe> snippet
game build input.game --gif --duration 5 → Animated GIF (5 seconds)
game build input.game --mp4 --duration 5 → MP4 video
```

---

## Technical Architecture

### SDF Boolean Operations — The Key Challenge

GAME's pipeline is linear: each stage takes one input, produces one output. Boolean SDF operations take TWO inputs. Solution: boolean operators receive sub-expressions as `Expr::Call` arguments (already supported in the AST).

**Codegen pattern:**
```wgsl
// union(circle(0.3), box(0.2, 0.4))
var sdf_a = sdf_circle(p, 0.3);
var sdf_b = sdf_box(p, 0.2, 0.4);
var sdf_result = min(sdf_a, sdf_b);

// smooth_union(circle(0.3), box(0.2, 0.4), 0.1)
var sdf_a = sdf_circle(p, 0.3);
var sdf_b = sdf_box(p, 0.2, 0.4);
var sdf_result = smin(sdf_a, sdf_b, 0.1);
```

**Required helper function (smooth min):**
```wgsl
fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = max(k - abs(a - b), 0.0) / k;
    return min(a, b) - h * h * k * 0.25;
}
```

### The WGSL mod() Gotcha

Critical for spatial repetition: WGSL's `%` operator is NOT equivalent to GLSL's `mod()`.
- GLSL: `mod(x, y) = x - y * floor(x/y)` — result always in [0, y)
- WGSL: `x % y = x - y * trunc(x/y)` — result can be NEGATIVE

For SDF repetition with negative coordinates, GAME must emit a floor-based modulo helper:
```wgsl
fn game_mod(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}
```

### Live Preview — Minimal Dependencies

The dev server should add minimal weight to the compiler binary:
- `notify` v7 — cross-platform file watcher (~150KB)
- `tiny-http` — embedded HTTP server (~100KB, zero async runtime)
- No WebSocket needed — use Server-Sent Events (SSE) over HTTP (one-way, simpler)
- Preview HTML embedded via `include_str!` at compile time

**Hot-swap architecture:**
1. File change detected → recompile `.game` → extract new WGSL/GLSL strings
2. Send SSE event with new shader source as JSON payload
3. Browser-side JS: `device.createShaderModule({ code: newWGSL })` → rebuild pipeline
4. Uniform buffer preserved → animation continues seamlessly
5. Parameter sliders remain in sync

### New SDF Primitives — The Math

**Line segment:** `distance_to_segment = length(p - clamp(dot(p-a,b-a)/dot(b-a,b-a), 0, 1) * (b-a) - a)`
**Capsule:** `line_segment_sdf - radius`
**Triangle:** Three-edge minimum distance with cross-product sign
**Arc:** Angle-constrained ring using atan2 + clamp
**Cross:** Union of two perpendicular boxes
**Heart:** Parametric heart curve via `(x^2 + y^2 - 1)^3 = x^2 * y^3`
**Egg:** Modified circle with asymmetric y-axis
**Spiral:** Angular + radial distance from Archimedean spiral
**Grid:** Repeated line segments at regular intervals

---

## Release Plan

### v0.3 — Shape Algebra + Live Studio (SHIPPED)
- [x] SDF boolean operations: union, subtract, intersect, smooth_union, smooth_subtract, smooth_intersect, xor
- [x] Spatial operations: repeat, mirror, radial
- [x] Shape modifiers: round, shell, onion
- [x] 9 new SDF primitives: line, capsule, triangle, arc_sdf, cross, heart, egg, spiral, grid
- [x] Layer opacity (alpha compositing)
- [x] Outline/stroke effect
- [x] `game dev` with file watcher + HTTP server + hot reload
- [x] Preview HTML with parameter sliders + audio input
- [x] `game new` scaffolding with 5 templates
- [x] README.md with language reference
- [ ] Named color presets (fire, ocean, neon, aurora, sunset, ice) — deferred to v0.4
- [ ] Morph (SDF interpolation) — deferred to v0.4

### v0.4 — Functions + Composition
- [ ] User-defined functions: `fn name(params) { pipeline }`
- [ ] File imports: `use "library.game"`
- [ ] Conditional pipelines: `if param > threshold { a } else { b }`
- [ ] Render-to-texture: cinematic as input to another cinematic
- [ ] Frame feedback loops
- [ ] Multi-pass rendering

### v0.5 — Scenes + Evolution
- [ ] Scene system with cinematic sequencing
- [ ] Transition library (fade, dissolve, morph, wipe)
- [ ] Visual genomes + procedural population
- [ ] Evolutionary exploration
- [ ] Export targets: artblocks, npm, gif, mp4
- [ ] `game playground` web editor

### v1.0 — Community
- [ ] Language Server Protocol (LSP) for editor support
- [ ] VS Code syntax highlighting extension
- [ ] Community gallery
- [ ] Standard library of common patterns
- [ ] Performance optimization guide
- [ ] npm package: `npx game build`

---

## Design Principles

1. **Every visual concept has a word.** Don't make artists write math. `smooth_union(circle, box, 0.1)` reads like English.

2. **The pipeline is the program.** Visual thought flows linearly: shape → effect → color. Binary operations branch and merge back immediately.

3. **Time is a first-class dimension.** Every parameter can be temporal. `>>` `<>` `!!` `..` make time manipulation as natural as arithmetic.

4. **The compiler is the artist's assistant.** It handles dual-target shaders, uniform management, GPU fallback, resize, audio plumbing, memory buffers. The artist handles shapes, colors, motion, rhythm.

5. **Zero-dependency output.** A GAME component is a single `.js` file. No npm install, no build step, no framework. This is the distribution advantage no competitor has.

6. **Generative means evolutionary.** Creation isn't just drawing — it's exploring possibility space. Breed, mutate, evolve, select. The language should make discovery as natural as description.
