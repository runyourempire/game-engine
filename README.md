# GAME — Generative Animation Matrix Engine

A shader DSL that compiles to zero-dependency Web Components.

Write this:
```game
cinematic "golden-orb" {
  layer main {
    circle(0.3) | glow(2.0) | tint(0.83, 0.69, 0.22)
  }
}
```

Get a self-contained `<game-golden-orb>` Web Component. No npm install. No build step. No framework. Drop the `.js` file into any web page and it works.

## Install

```bash
cargo install --path .
```

## Quick Start

```bash
# Create a new .game file from a template
game new --template minimal --output my-first.game

# Live preview with hot reload
game dev my-first.game

# Build to Web Component
game build my-first.game -o dist/
```

Open `http://localhost:4200` — you'll see your shader with parameter sliders, audio input, and FPS counter. Edit the `.game` file, save, and watch it update live.

## The Language

### Cinematics

A cinematic is a named visual composition. It compiles to a custom HTML element.

```game
cinematic "name" {
  layer layer_name {
    shape() | effect() | color()
  }
}
```

### Pipeline Syntax

Each layer contains a pipeline of stages separated by `|`. Data flows left to right:

```
Position → [SDF Primitive] → [SDF Modifier] → [SDF→Color Bridge] → [Color Effect]
            circle, ring       round, shell      glow, shade          tint, bloom
            star, box, hex     mask_arc           emissive, palette    grain
            triangle, line     ...                                     ...
            capsule, arc
```

### SDF Primitives

```game
circle(0.3)                      // Circle with radius 0.3
ring(0.3, 0.02)                  // Ring with radius 0.3, width 0.02
star(5, 0.3, 0.15)              // 5-pointed star, outer 0.3, inner 0.15
box(0.3, 0.2)                   // Rectangle 0.3 x 0.2
hex(0.3)                        // Hexagon with radius 0.3
triangle(0.3)                   // Equilateral triangle, size 0.3
line(x1, y1, x2, y2)           // Line segment between two points
capsule(0.3, 0.05)             // Capsule (rounded line), length 0.3, radius 0.05
arc(0.3, 3.14, 0.02)           // Arc, radius 0.3, angle 3.14, width 0.02
cross(0.3, 0.05)               // Cross shape, size 0.3, arm width 0.05
heart(0.3)                      // Heart shape, size 0.3
egg(0.3, 0.1)                  // Egg shape, radius 0.3, asymmetry 0.1
spiral(3.0, 0.02)              // Spiral, 3 turns, width 0.02
grid(0.1, 0.005)               // Grid lattice, spacing 0.1, line width 0.005
```

### SDF Boolean Operations

Combine shapes using constructive solid geometry:

```game
union(circle(0.3), box(0.2, 0.4))              // Union (OR)
subtract(circle(0.3), box(0.1, 0.1))           // Subtraction (A minus B)
intersect(ring(0.3, 0.05), star(5, 0.35, 0.15)) // Intersection (AND)
smooth_union(circle(0.3), circle(0.3), 0.1)     // Smooth blend, k=0.1
smooth_subtract(circle(0.3), box(0.1, 0.1), 0.05)
smooth_intersect(circle(0.3), box(0.2, 0.2), 0.08)
xor(circle(0.3), box(0.25, 0.25))              // Symmetric difference
```

### Spatial Operations

Transform the coordinate space before shape evaluation:

```game
repeat(0.5, 0.5) | circle(0.1) | glow(2.0)   // Infinite tiling
mirror() | star(5, 0.3, 0.15) | glow(2.0)     // Bilateral symmetry
radial(6) | box(0.3, 0.05) | glow(2.0)        // 6-fold radial symmetry
```

### Shape Modifiers

```game
circle(0.3) | round(0.05) | glow(2.0)          // Round corners
circle(0.3) | shell(0.02) | glow(2.0)          // Hollow out
circle(0.3) | onion(3, 0.02) | glow(2.0)       // Concentric shells
```

### SDF → Color Bridges

```game
glow(2.0)                       // Exponential falloff glow
shade(0.8, 0.6, 0.2)           // Direct SDF-to-color mapping
emissive(1.5)                   // Glow with alpha transparency
palette(...)                    // Cosine palette (12 params: a,b,c,d RGB)
```

### Color Effects

```game
tint(0.83, 0.69, 0.22)         // Multiply by RGB color
bloom(0.3, 2.0)                // High-pass bloom, threshold + strength
grain(0.05)                    // Film grain noise
outline(0.01)                  // Edge outline, width
```

### Transforms

```game
rotate(1.0) | circle(0.3) | glow(2.0)         // Time-driven rotation
translate(0.2, 0.1) | circle(0.3) | glow(2.0) // Offset position
scale(2.0) | circle(0.3) | glow(2.0)          // Scale up 2x
```

### Procedural Noise

```game
fbm(3.0, 4, 0.5, 2.0)         // Fractional Brownian motion
simplex(5.0)                   // Simplex noise
voronoi(5.0)                   // Voronoi cells
warp(3.0, 4, 0.5, 2.0, 0.3)   // Domain warping (fbm-based)
distort(3.0, 1.0, 0.2)        // Sinusoidal distortion
polar()                        // Polar coordinate transform
```

### Parameters & Uniforms

Declare named parameters that become GPU uniforms:

```game
cinematic "reactive-orb" {
  layer config {
    intensity: 1.0
    radius: 0.3
    hue: 0.5
  }

  layer orb {
    circle(radius) | glow(intensity) | tint(hue, 0.7, 0.3)
  }
}
```

Access from JavaScript: `element.intensity = 0.8` or `<game-reactive-orb intensity="0.8">`.

### Temporal Operators

Modulate parameters with smooth time-domain processing:

```game
layer config {
  bass: 0.5 ~ audio.bass <> 50ms >> 200ms .. [0.0, 1.0]
}
```

| Operator | Meaning | Example |
|----------|---------|---------|
| `~` | Modulate by signal | `value ~ audio.bass` |
| `<>` | Smooth (EMA filter) | `<> 50ms` |
| `>>` | Delay (ring buffer) | `>> 200ms` |
| `!!` | Trigger (edge detect) | `!! 300ms` |
| `..` | Range clamp | `.. [0.0, 1.0]` |

### Multi-Layer Composition

```game
cinematic "layered" {
  layer background blend screen {
    fbm(2.0, 4, 0.5, 2.0) | glow(1.0) | tint(0.1, 0.1, 0.3)
  }

  layer foreground blend add {
    circle(0.3) | glow(2.0) | tint(0.83, 0.69, 0.22)
  }
}
```

Blend modes: `add` (default), `screen`, `multiply`, `overlay`.

### Scene Composition

Sequence cinematics with timed transitions:

```game
cinematic "dawn" { ... }
cinematic "noon" { ... }
cinematic "dusk" { ... }

scene "day-cycle" {
  play "dawn" for 10s
  transition dissolve over 2s
  play "noon" for 15s
  transition fade over 3s
  play "dusk" for 10s
}
```

Compiles to a `<game-scene-day-cycle>` Web Component that creates child cinematic elements and crossfades between them via CSS opacity transitions.

### User-Defined Functions

```game
fn neon_dot(r, g, b) {
  circle(0.1) | glow(4.0) | tint(r, g, b)
}

cinematic "dots" {
  layer main {
    neon_dot(1.0, 0.3, 0.8)
  }
}
```

### Standard Library

Import reusable functions from the stdlib:

```game
import "std:shapes"    // dot, petal, gear, bullseye, halo, badge, blob, pill
import "std:palettes"  // ocean, fire, forest, neon, sunset, aurora, cyber
import "std:patterns"  // checker, dots, stripes
import "std:effects"   // neon_dot, ember_orb, ice_orb, gold_ring, soft_star, warped_noise
import "std:motion"    // orbit, pulse_ring, spin, wobble
```

### Conditionals

```game
layer main {
  if energy > 0.5 {
    star(5, 0.3, 0.15) | glow(3.0) | tint(1.0, 0.5, 0.1)
  } else {
    circle(0.2) | glow(2.0) | tint(0.3, 0.5, 1.0)
  }
}
```

### Memory (Visual Persistence)

Layers with `memory` retain a fraction of the previous frame:

```game
layer trails memory: 0.95 {
  circle(0.2) | glow(3.0) | tint(1.0, 0.5, 0.2)
}
```

### Animation Curves (Arc)

```game
arc {
  scale: 0.1 -> 1.0 over 3s ease_in_out
  brightness: 0.0 -> 1.0 over 2s ease_out
}
```

### Parameter Coupling (Resonate)

```game
resonate {
  bass -> core.scale * 0.3
  pulse -> ring.brightness * 0.5
}
```

### Audio Analysis (Listen)

```game
listen {
  onset: attack(threshold: 0.7, decay: 300ms)
  melody: pitch(min: 200, max: 4000)
  rhythm: phase(subdivide: 16)
  delta: delta(window: 2.0, direction: "negative")
}
```

### Audio Synthesis (Voice)

```game
voice {
  osc: sine(freq: 440)
  filter: lowpass(cutoff: 2000, q: 1.5)
  gain: gain(level: 0.5)
  reverb: reverb(room: 0.4)
}
```

### Musical Timeline (Score)

```game
score tempo(120) {
  motif rise { scale: 0.5 -> 2.0 over 4bars }
  motif fall { scale: 2.0 -> 0.5 over 2bars }
  phrase build = rise | fall
  section verse = build
  arrange: verse verse verse
}
```

### Compute Shaders

#### N-Body Gravity
```game
gravity {
  force_law: 1.0 / (distance * distance)
  damping: 0.995
  bounds: wrap
}
```

#### Reaction-Diffusion
```game
react {
  feed: 0.055
  kill: 0.062
  diffuse_a: 1.0
  diffuse_b: 0.5
  seed: center(0.1)
}
```

#### Physarum Stigmergy
```game
swarm {
  agents: 500000
  sensor_angle: 45
  sensor_dist: 9.0
  turn_angle: 45
  step: 1.0
  deposit: 5.0
  decay: 0.95
  diffuse: 1
  bounds: wrap
}
```

#### Vector Field Flow
```game
flow {
  type: curl
  scale: 3.0
  speed: 0.5
  octaves: 4
  strength: 1.5
}
```

### Genetic Composition (Breed)

```game
breed "child" from "fire" + "ice" {
  inherit layers: mix(0.6)
  inherit params: pick(0.5)
  mutate scale: 0.3
  mutate speed: 0.1
}
```

### Projection Mapping

```game
project dome(theta: 0.5) {
  source: "my-cinematic"
}
```

Modes: `flat`, `dome`, `cube`, `led`.

### External Imports

```game
import "shadertoy://XsXXDn" as shader
import "midi://channel/1" as midi
import "osc://localhost:9000/params" as osc
import "camera://0" as webcam
```

## Output

### Web Component (.js)
```bash
game build input.game -o dist/
```
Produces `dist/name.js` — a self-registering custom element:
```html
<script src="name.js"></script>
<game-name intensity="0.8"></game-name>
```

### Standalone HTML
```bash
game build input.game -o dist/ -f html
```
Produces a complete HTML page with embedded shaders.

### Shader Files
Every build also produces `name.wgsl` and `name.frag` (GLSL ES 3.0) alongside the component.

## Using Components

### HTML
```html
<script type="module" src="./golden-orb.js"></script>
<game-golden-orb style="width:200px;height:200px"></game-golden-orb>
```

### React/TypeScript
```tsx
import './lib/game-components/golden-orb.js';

function App() {
  const ref = useRef<HTMLElement>(null);

  useEffect(() => {
    if (ref.current) {
      (ref.current as any).intensity = 0.8;
    }
  }, []);

  return <game-golden-orb ref={ref} />;
}
```

### JavaScript API
```js
const el = document.querySelector('game-golden-orb');
el.intensity = 0.8;           // Property setter
el.setParam('radius', 0.3);   // Generic setter
el.setAudioData({ bass: 0.5, mid: 0.3, treble: 0.1, energy: 0.3, beat: 0 });
```

## Architecture

```
.game source → Lexer (logos) → Parser (recursive descent) → AST
  → Validate (pipeline state machine with typed stages)
  → Codegen (WGSL + GLSL + JS modules)
  → Runtime (Web Component wrapper)
  → Output (.js + .wgsl + .frag + .html)
```

- **WebGPU** primary renderer (90%+ modern browser support)
- **WebGL2** automatic fallback (older browsers)
- **ResizeObserver** for responsive canvas
- Shaders use periodic time `fract(t/120)*120` to avoid float precision issues
- Pipeline state machine: Position → Sdf → Color with typed transitions
- "Did you mean?" error suggestions for typos and missing bridge stages
- ~29,000 lines of Rust, 49 builtins, 567 tests

## Examples

See the `examples/` directory for 78 reference `.game` files covering every feature.

## License

[FSL-1.1-Apache-2.0](LICENSE) — Functional Source License, converting to Apache 2.0 after 2 years.

Copyright (c) 2026 4DA Systems Pty Ltd.
