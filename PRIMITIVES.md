# GAME Language Primitives Reference

Complete reference for the GAME shader DSL. Every builtin, color, constant, function, and signal listed here is implemented in the compiler and produces valid WGSL/GLSL output.

---

## Type State Pipeline

The pipe operator `|` chains stages. Each builtin consumes an input type and produces an output type. The compiler enforces valid transitions at compile time.

```
Position --> [SDF Generator]  --> Sdf
Position --> [Transform]      --> Position
Position --> [Color Generator] --> Color
Sdf      --> [Bridge]         --> Color
Sdf      --> [SDF Modifier]   --> Sdf
Color    --> [Color Processor] --> Color
```

Position-input stages can appear anywhere in a chain (they reset the pipeline to Position state).

**Example valid pipeline:**
```game
fn: translate(0.1, 0) | circle(0.3) | mask_arc(pi) | glow(2.0) | tint(gold) | bloom(0.3, 1.5) | vignette(0.4)
```

---

## Builtins (37 total)

### SDF Generators (Position -> Sdf)

These evaluate a signed distance field from the current position. Negative inside, positive outside, zero at the surface.

| Builtin | Parameters | Description |
|---------|-----------|-------------|
| `circle` | `radius: 0.2` | Circular SDF |
| `ring` | `radius: 0.3, width: 0.02` | Ring / annulus SDF |
| `star` | `points: 5.0, radius: 0.3, inner: 0.15` | Star polygon SDF |
| `box` | `w: 0.2, h: 0.2` | Rectangle SDF |
| `polygon` | `sides: 6.0, radius: 0.3` | Regular polygon SDF |
| `fbm` | `scale: 1.0, octaves: 4.0, persistence: 0.5, lacunarity: 2.0` | Fractal Brownian Motion noise field |
| `simplex` | `scale: 1.0` | Simplex noise field |
| `voronoi` | `scale: 5.0` | Voronoi cellular noise field |
| `concentric_waves` | `amplitude: 1.0, width: 0.5, frequency: 3.0` | Concentric wave rings |

### SDF Bridges (Sdf -> Color)

Convert a distance field into visible color output.

| Builtin | Parameters | Description |
|---------|-----------|-------------|
| `glow` | `intensity: 1.5` | Exponential distance falloff glow |
| `shade` | `r: 1.0, g: 1.0, b: 1.0` | Flat-shaded color with anti-aliased edges |
| `emissive` | `intensity: 1.0` | Emissive / bright color |

### Color Processors (Color -> Color)

Post-process color output. Chain as many as needed.

| Builtin | Parameters | Description |
|---------|-----------|-------------|
| `tint` | `r: 1.0, g: 1.0, b: 1.0` | Multiply color by RGB tint |
| `bloom` | `threshold: 0.3, strength: 2.0` | Bloom / glow on bright areas |
| `grain` | `amount: 0.1` | Film grain noise overlay |
| `blend` | `factor: 0.5` | Blend with background by factor |
| `vignette` | `strength: 0.5, radius: 0.8` | Darken edges |
| `tonemap` | `exposure: 1.0` | HDR tonemapping (Reinhard) |
| `scanlines` | `frequency: 200.0, intensity: 0.3` | CRT scanline effect |
| `chromatic` | `offset: 0.005` | Chromatic aberration (RGB channel split) |
| `saturate_color` | `amount: 1.0` | Adjust color saturation |
| `glitch` | `intensity: 0.5` | Digital glitch distortion |

### Position Transforms (Position -> Position)

Transform the coordinate space before SDF evaluation. Place before shapes in the pipe chain.

| Builtin | Parameters | Description |
|---------|-----------|-------------|
| `translate` | `x: 0.0, y: 0.0` | Move in 2D space |
| `rotate` | `angle: 0.0` | Rotate (radians) |
| `scale` | `s: 1.0` | Uniform scale |
| `twist` | `amount: 0.0` | Twist distortion |
| `mirror` | `axis: 0.0` | Mirror across axis |
| `repeat` | `count: 4.0` | Tile / repeat pattern |
| `domain_warp` | `amount: 0.1, freq: 3.0` | Warp coordinate space with noise |
| `curl_noise` | `frequency: 1.0, amplitude: 0.1` | Curl noise distortion |
| `displace` | `strength: 0.1` | Displacement distortion |

### SDF Modifiers (Sdf -> Sdf)

Modify an existing SDF. Place after a shape in the pipe chain.

| Builtin | Parameters | Description |
|---------|-----------|-------------|
| `mask_arc` | `angle` (required, no default) | Mask SDF to arc sector |
| `threshold` | `cutoff: 0.5` | Hard threshold on SDF values |
| `onion` | `thickness: 0.02` | Hollow out SDF (shell) |
| `round` | `radius: 0.02` | Round SDF edges |

### Color Generators (Position -> Color)

Generate color directly from position, bypassing SDF evaluation.

| Builtin | Parameters | Description |
|---------|-----------|-------------|
| `gradient` | `color_a, color_b, mode` (all required, no defaults) | Linear or radial gradient between two colors |
| `spectrum` | `bass: 0.0, mid: 0.0, treble: 0.0` | Audio-reactive spectrum visualization |

---

## Named Colors (19)

Available anywhere a color value is accepted: `tint()`, `shade()`, `gradient()`, or as bare identifiers in expressions.

| Name | R | G | B |
|------|---|---|---|
| `black` | 0.000 | 0.000 | 0.000 |
| `white` | 1.000 | 1.000 | 1.000 |
| `red` | 1.000 | 0.000 | 0.000 |
| `green` | 0.000 | 1.000 | 0.000 |
| `blue` | 0.000 | 0.000 | 1.000 |
| `cyan` | 0.000 | 1.000 | 1.000 |
| `magenta` | 1.000 | 0.000 | 1.000 |
| `orange` | 1.000 | 0.647 | 0.000 |
| `gold` | 0.831 | 0.686 | 0.216 |
| `ember` | 0.898 | 0.318 | 0.129 |
| `ivory` | 1.000 | 1.000 | 0.941 |
| `frost` | 0.686 | 0.878 | 0.953 |
| `ash` | 0.467 | 0.467 | 0.467 |
| `charcoal` | 0.212 | 0.212 | 0.212 |
| `midnight` | 0.039 | 0.039 | 0.118 |
| `obsidian` | 0.071 | 0.059 | 0.082 |
| `deep_blue` | 0.000 | 0.098 | 0.392 |
| `plasma` | 0.580 | 0.000 | 0.827 |
| `violet` | 0.541 | 0.169 | 0.886 |

---

## Math Constants (4)

Available as bare identifiers in any expression.

| Name | Value |
|------|-------|
| `pi` | 3.14159265358979 |
| `tau` | 6.28318530717959 |
| `e` | 2.71828182845905 |
| `phi` | 1.61803398874989 |

---

## Expression Functions

Math functions usable in parameter expressions and modulation. These compile to native WGSL builtins (GPU) and `Math.*` equivalents (JS runtime).

### Single-argument

`sin` `cos` `tan` `abs` `floor` `ceil` `fract` `sqrt` `exp` `log` `sign` `round` `length` `normalize`

### Multi-argument

| Function | Arguments | Description |
|----------|-----------|-------------|
| `min(a, b)` | 2 | Minimum of two values |
| `max(a, b)` | 2 | Maximum of two values |
| `mix(a, b, t)` | 3 | Linear interpolation: `a + (b - a) * t` |
| `clamp(x, lo, hi)` | 3 | Clamp value to range |
| `smoothstep(edge0, edge1, x)` | 3 | Hermite interpolation |
| `step(edge, x)` | 2 | 0.0 if `x < edge`, else 1.0 |
| `pow(base, exp)` | 2 | Exponentiation (also available as `^` operator) |
| `mod(x, y)` | 2 | Modulo (compiles to `%` operator) |
| `distance(a, b)` | 2 | Euclidean distance |
| `dot(a, b)` | 2 | Dot product |
| `cross(a, b)` | 2 | Cross product |
| `reflect(i, n)` | 2 | Reflection vector |
| `atan2(y, x)` | 2 | Two-argument arctangent |

---

## Signals

Runtime inputs that update every frame. Use directly in expressions or with the `~` modulation operator.

| Signal | Description | Range |
|--------|-------------|-------|
| `time` | Elapsed time in seconds | 0+ |
| `audio.bass` | Low frequency energy | 0 - 1 |
| `audio.mid` | Mid frequency energy | 0 - 1 |
| `audio.treble` | High frequency energy | 0 - 1 |
| `audio.energy` | Total audio energy | 0 - 1 |
| `audio.beat` | Beat detection pulse | 0 - 1 |
| `mouse.x` | Normalized mouse X position | 0 - 1 |
| `mouse.y` | Normalized mouse Y position | 0 - 1 |
| `data.*` | Web Component property binding (any field name) | any |

### Modulation syntax

```game
param_name: base_value ~ signal * scale
```

Example: `radius: 0.3 ~ audio.bass * 0.5` sets base radius to 0.3, adds up to 0.5 when bass peaks.

---

## Language Features

### Layers

Multiple layers composite additively:

```game
cinematic {
  layer bg   { fn: ring(0.4, 0.02) | glow(0.5) | tint(frost) }
  layer main { fn: circle(0.2) | glow(3.0) | tint(gold) }
}
```

### Define (Reusable Macros)

Defines expand inline at compile time. Parameters are substituted by position.

```game
define glow_ring(r, t) {
  ring(r, t) | glow(2.0) | tint(cyan)
}

layer { fn: glow_ring(0.3, 0.04) }
```

### Arc Timeline

Temporal parameter evolution with named moments:

```game
arc {
  0:00 "idle" {
    radius: 0.1
    intensity: 1.0
  }
  0:03 "expand" {
    radius -> 0.5 ease(expo_out) over 2s
    intensity -> 4.0 ease(smooth) over 2s
  }
}
```

### Easing Functions

For arc transitions: `linear`, `smooth`, `expo_in`, `expo_out`, `cubic_in_out`, `elastic`, `bounce`.

### Duration Literals

- `2s` = 2 seconds
- `500ms` = 500 milliseconds (compiles to 0.5 seconds)
- `4bars` = 4 bars at assumed 120 BPM (compiles to 8.0 seconds)

### Operators

| Operator | Description |
|----------|-------------|
| `+` `-` `*` `/` | Arithmetic |
| `^` | Exponentiation (compiles to `pow()`) |
| `>` `<` | Comparison |
| `\|` | Pipe (chain pipeline stages) |
| `~` | Modulation (signal binding) |
| `? :` | Ternary conditional (compiles to WGSL `select()`) |

### Special Variables

| Variable | Available In | Meaning |
|----------|-------------|---------|
| `p` | `fn:` chains | Current sample position (vec2f) |
| `time` | Everywhere | Elapsed time in seconds |
| `uv` | `flat` mode | Screen UV coordinates (0 - 1) |

### Array Literals

Arrays compile to WGSL vector types:

```game
[1.0, 0.5, 0.0]    // compiles to vec3f(1.0, 0.5, 0.0)
[0.5, 0.5]          // compiles to vec2f(0.5, 0.5)
```
