# The GAME Language Specification

**Version:** 1.0
**Compiler:** game-compiler (Rust, ~29K lines, 534 tests)
**Targets:** WGSL (WebGPU) + GLSL ES 3.0 (WebGL2) + JavaScript runtime
**Output:** Self-contained Web Components (`<game-*>` custom elements)

---

## 1. Overview

GAME (Generative Animation Matrix Engine) is a domain-specific language for creating GPU-accelerated visual components. A `.game` file compiles to a standalone Web Component with dual-target shaders (WebGPU primary, WebGL2 fallback) and JavaScript runtime code for audio, animation, interaction, and physics.

### Design Principles

1. **Reads like intent** -- describe what you see, not how to render it
2. **Progressive disclosure** -- 3 lines for a glowing circle, 100 lines for a cinematic
3. **Mathematical native** -- expressions are first-class, not strings in quotes
4. **Composable** -- small pieces combine via pipes, imports, and nesting
5. **Temporal native** -- time, rhythm, and duration are built into the syntax
6. **Signal-driven** -- any parameter can react to any signal (audio, input, data)

---

## 2. Quick Start

The simplest `.game` file:

```game
cinematic "Hello" {
  layer {
    fn: circle(0.3) | glow(2.0)
  }
}
```

This compiles to a `<game-hello>` Web Component rendering a glowing circle. Three lines.

---

## 3. Program Structure

A GAME program consists of top-level declarations:

```game
import "path/to/module.game" expose name1, name2
import "path/to/lib.game" as mylib

cinematic "Name" {
  # layers, arcs, resonates, defines, listen, voice,
  # score, gravity, lenses, react blocks
}

breed "child" from "parent_a" + "parent_b" {
  # genetic composition rules
}

project dome(fisheye: 180) {
  source: layer_name
}
```

Top-level declarations: `import`, `cinematic`, `breed`, `project`.

### Comments

```game
# This is a comment (hash style)
// This is also a comment (C++ style)
```

Both `#` and `//` produce single-line comments. Everything after the comment marker to the end of the line is ignored.

---

## 4. Cinematic Block

The `cinematic` block is the primary container. It requires a string name and curly braces.

```game
cinematic "My Visual" {
  # ... contents ...
}
```

A cinematic can contain the following block types in any order:

| Block | Purpose | Multiplicity |
|-------|---------|-------------|
| `layer` | Visual layer (pipeline or params) | 0..N |
| `arc` | Timeline animation keyframes | 0..N |
| `resonate` | Cross-layer feedback | 0..N |
| `define` | Reusable macro function | 0..N |
| `lens` | Rendering mode / post-processing | 0..N |
| `react` | Input event handlers | 0..1 |
| `listen` | Audio analysis signals | 0..1 |
| `voice` | Audio synthesis graph | 0..1 |
| `score` | Musical sequencing / timeline | 0..1 |
| `gravity` | Particle physics (compute shader) | 0..1 |

---

## 5. Layers

Layers are the visual building blocks. Each layer compiles to shader code. Multiple layers are composited additively.

### 5.1 Pipeline Layers

A pipeline layer chains built-in stage functions with the pipe operator `|`:

```game
layer ring {
  fn: circle(0.3) | glow(2.0) | tint(0.831, 0.686, 0.216)
}
```

The `fn:` prefix indicates a pipeline body. Stages flow left to right through a typed state machine (see Section 7).

### 5.2 Param Layers

A param layer declares named parameters with optional modulation:

```game
layer config {
  intensity: 0.5
  scale: 2.0 ~ audio.bass * 0.3
  brightness: 1.0 ~ mouse.x
}
```

### 5.3 Mixed Layers (Pipeline + Params)

A layer can have both a pipeline and inline parameters. The pipeline is declared with `fn:` followed by parameters on subsequent lines:

```game
layer pulse {
  fn: circle(radius) | glow(intensity) | tint(gold)
  radius: 0.3 ~ audio.bass * 0.5
  intensity: 2.0 ~ audio.energy * 3.0
}
```

Parameters referenced by name in the pipeline (e.g., `radius`, `intensity`) become GPU uniforms that the runtime updates each frame.

### 5.4 Layer Name

The layer name is optional. If omitted, an auto-generated name is used:

```game
layer { fn: circle(0.3) | glow(2.0) }       # auto-named
layer my_layer { fn: circle(0.3) | glow(2.0) }  # named "my_layer"
```

### 5.5 Layer Options

Layers accept optional parenthesized parameters:

```game
layer ring (width: 0.02, segments: 8) {
  fn: ring(0.3, width) | glow(2.0)
}
```

### 5.6 Memory (Frame Feedback)

The `memory` property enables ping-pong frame buffer feedback. The value (0.0 to 1.0) controls decay -- how much of the previous frame persists:

```game
layer trails memory: 0.95 {
  fn: circle(0.1) | glow(3.0) | tint(cyan)
}
```

- `memory: 0.0` -- no persistence (default)
- `memory: 0.95` -- strong trails (95% of previous frame mixes in)
- `memory: 1.0` -- infinite persistence (never fades)

The compiler emits ping-pong texture bindings and mix operations in both WGSL and GLSL. The runtime manages two render targets swapped each frame.

### 5.7 Cast (Type Assertion)

The `cast` property asserts the pipeline's output type. The compiler validates that the pipeline's final stage matches the declared type:

```game
layer distance_field cast sdf {
  fn: circle(0.3)
}

layer colored cast color {
  fn: circle(0.3) | glow(2.0) | tint(gold)
}

layer warped cast position {
  fn: translate(0.1, 0.2) | rotate(0.5) | scale(2.0)
}
```

Valid cast types: `sdf` (alias: `distance`), `color` (alias: `rgba`), `position` (alias: `uv`).

---

## 6. Expressions

GAME has a full expression system with precedence climbing.

### 6.1 Literals

```game
42          # integer (parsed as f64)
3.14        # float
-1.3        # negation
"hello"     # string
[1.0, 2.0, 3.0]  # array (compiles to vec3f in WGSL)
```

Arrays compile to WGSL vector types: `[a, b]` becomes `vec2f(a, b)`, `[a, b, c]` becomes `vec3f(a, b, c)`, `[a, b, c, d]` becomes `vec4f(a, b, c, d)`.

### 6.2 Identifiers and Dotted Names

```game
intensity       # plain identifier (param reference or uniform)
audio.bass      # dotted: object.field
mouse.x         # dotted: mouse signal
data.value      # dotted: external data binding
time            # built-in: elapsed seconds
```

### 6.3 Named Colors

The following color names resolve to `vec3f` values in WGSL and `[r, g, b]` arrays in JavaScript:

| Name | RGB | Name | RGB |
|------|-----|------|-----|
| `black` | (0, 0, 0) | `white` | (1, 1, 1) |
| `red` | (1, 0, 0) | `green` | (0, 1, 0) |
| `blue` | (0, 0, 1) | `cyan` | (0, 1, 1) |
| `gold` | (0.831, 0.686, 0.216) | `orange` | (1, 0.647, 0) |
| `midnight` | (0.039, 0.039, 0.118) | `deep_blue` | (0, 0.098, 0.392) |
| `obsidian` | (0.071, 0.059, 0.082) | `charcoal` | (0.212, 0.212, 0.212) |
| `ember` | (0.898, 0.318, 0.129) | `ash` | (0.467, 0.467, 0.467) |
| `ivory` | (1, 1, 0.941) | `frost` | (0.686, 0.878, 0.953) |
| `plasma` | (0.580, 0, 0.827) | `violet` | (0.541, 0.169, 0.886) |
| `magenta` | (1, 0, 1) | | |

### 6.4 Named Constants

| Name | Value |
|------|-------|
| `pi` | 3.14159265358979 |
| `tau` | 6.28318530717959 |
| `e` | 2.71828182845905 |
| `phi` | 1.61803398874989 |

### 6.5 Binary Operators

Precedence from lowest to highest:

| Precedence | Operators | Description |
|------------|-----------|-------------|
| 1 | `? :` | Ternary conditional |
| 2 | `>`, `<` | Comparison |
| 3 | `+`, `-` | Addition, subtraction |
| 4 | `*`, `/` | Multiplication, division |
| 5 | `^` | Power (right-associative, compiles to `pow()`) |
| 6 | `-expr` | Unary negation |
| 7 | `(expr)` | Parentheses |

Examples:

```game
x + y * 2.0           # multiplication binds tighter than addition
x ^ 2                 # compiles to pow(x, 2.0) in WGSL, (x ** 2.0) in JS
cond > 0.5 ? a : b    # ternary (compiles to select() in WGSL)
```

### 6.6 Function Calls

```game
sin(time)
mix(a, b, t)
clamp(x, 0.0, 1.0)
smoothstep(0.0, 1.0, x)
```

The compiler recognizes standard math functions and compiles them to the appropriate target:

**WGSL passthrough:** `abs`, `sin`, `cos`, `tan`, `sqrt`, `floor`, `ceil`, `fract`, `length`, `normalize`, `exp`, `log`, `sign`, `round`, `mix`, `clamp`, `smoothstep`, `step`, `min`, `max`, `pow`, `distance`, `dot`, `cross`, `reflect`, `atan2`

**JavaScript mapping:** These map to `Math.sin()`, `Math.cos()`, etc. Special cases:
- `mix(a, b, t)` becomes `(a + (b - a) * t)`
- `clamp(x, lo, hi)` becomes `Math.min(Math.max(x, lo), hi)`
- `smoothstep(lo, hi, x)` becomes an inline Hermite interpolation
- `fract(x)` becomes `(x % 1)`
- `mod(a, b)` becomes `(a % b)` in both targets

### 6.7 Duration Literals

Duration literals are numbers with a unit suffix:

```game
2s          # 2 seconds
0.5s        # 0.5 seconds
500ms       # 500 milliseconds
4bars       # 4 musical bars (at default 120 BPM = 8 seconds)
180deg      # 180 degrees (parsed as a number; sugar for readability)
```

Duration literals appear in temporal operators and arc transitions. In expression contexts, they compile to their value in seconds.

---

## 7. Pipeline Stages (Built-in Functions)

Pipeline stages are the atoms of visual composition. Each stage has a typed input and output, forming a state machine:

```
Position --> [SDF Generators] --> Sdf --> [Bridges] --> Color --> [Post-processing] --> Color
    |                               |
    +-- [Transforms] --+            +-- [SDF Modifiers] --+
    |                  |            |                      |
    +<-----------------+            +<---------------------+
```

### 7.1 State Machine

| State | Description |
|-------|-------------|
| **Position** | 2D coordinate (`vec2 p`) -- before SDF evaluation |
| **Sdf** | Signed distance field (`float sdf_result`) |
| **Color** | RGBA color (`vec4 color_result`) |

Stages must be connected in valid state transitions. The compiler validates the pipeline and rejects invalid chains (e.g., `glow()` without a preceding SDF generator).

### 7.2 SDF Generators (Position -> Sdf)

These create signed distance fields from 2D positions.

| Stage | Parameters | Description |
|-------|-----------|-------------|
| `circle(radius)` | `radius`: 0.2 | Circle SDF |
| `ring(radius, width)` | `radius`: 0.3, `width`: 0.02 | Ring (annulus) SDF |
| `star(points, radius, inner)` | `points`: 5, `radius`: 0.3, `inner`: 0.15 | N-pointed star SDF |
| `box(w, h)` | `w`: 0.2, `h`: 0.2 | Rectangle SDF |
| `polygon(sides, radius)` | `sides`: 6, `radius`: 0.3 | Regular polygon SDF |
| `fbm(scale, octaves, persistence, lacunarity)` | `scale`: 1, `octaves`: 4, `persistence`: 0.5, `lacunarity`: 2 | Fractal Brownian motion noise field |
| `simplex(scale)` | `scale`: 1.0 | Simplex noise field |
| `voronoi(scale)` | `scale`: 5.0 | Voronoi cell pattern |
| `concentric_waves(amplitude, width, frequency)` | `amplitude`: 1, `width`: 0.5, `frequency`: 3 | Concentric wave rings |

### 7.3 SDF-to-Color Bridges (Sdf -> Color)

These convert distance field values to visible colors.

| Stage | Parameters | Description |
|-------|-----------|-------------|
| `glow(intensity)` | `intensity`: 1.5 | Exponential glow from SDF edge |
| `shade(r, g, b)` | `r`: 1, `g`: 1, `b`: 1 | Solid color based on distance (sharp edge) |
| `emissive(intensity)` | `intensity`: 1.0 | Glow with alpha = glow value (for compositing) |

### 7.4 SDF Modifiers (Sdf -> Sdf)

These modify the distance field before color conversion.

| Stage | Parameters | Description |
|-------|-----------|-------------|
| `mask_arc(angle)` | `angle`: (required) | Mask SDF to angular sector |
| `threshold(cutoff)` | `cutoff`: 0.5 | Hard threshold on SDF value |
| `onion(thickness)` | `thickness`: 0.02 | Convert filled SDF to shell (outline) |
| `round(radius)` | `radius`: 0.02 | Round the corners of an SDF |

### 7.5 Position Transforms (Position -> Position)

These warp the coordinate space before SDF evaluation.

| Stage | Parameters | Description |
|-------|-----------|-------------|
| `translate(x, y)` | `x`: 0, `y`: 0 | Translate position |
| `rotate(angle)` | `angle`: 0 | Rotate position (radians) |
| `scale(s)` | `s`: 1.0 | Uniform scale |
| `twist(amount)` | `amount`: 0 | Angular twist (distortion) |
| `mirror(axis)` | `axis`: 0 | Mirror across axis (0=X, 1=Y) |
| `repeat(count)` | `count`: 4 | Tile the space |
| `domain_warp(amount, freq)` | `amount`: 0.1, `freq`: 3 | Noise-based domain warping |
| `curl_noise(frequency, amplitude)` | `frequency`: 1, `amplitude`: 0.1 | Curl noise displacement |
| `displace(strength)` | `strength`: 0.1 | Noise displacement |

### 7.6 Color Post-processing (Color -> Color)

These modify the final color output.

| Stage | Parameters | Description |
|-------|-----------|-------------|
| `tint(r, g, b)` | `r`: 1, `g`: 1, `b`: 1 | Multiply color by RGB |
| `bloom(threshold, strength)` | `threshold`: 0.3, `strength`: 2 | Bloom/glow post-effect |
| `grain(amount)` | `amount`: 0.1 | Film grain noise |
| `vignette(strength, radius)` | `strength`: 0.5, `radius`: 0.8 | Edge darkening |
| `blend(factor)` | `factor`: 0.5 | Blend factor for mixing |
| `tonemap(exposure)` | `exposure`: 1.0 | Tone mapping |
| `scanlines(frequency, intensity)` | `frequency`: 200, `intensity`: 0.3 | CRT scanline effect |
| `chromatic(offset)` | `offset`: 0.005 | Chromatic aberration |
| `saturate_color(amount)` | `amount`: 1.0 | Color saturation adjustment |
| `glitch(intensity)` | `intensity`: 0.5 | Glitch distortion |

### 7.7 Full-Screen Generators (Position -> Color)

These produce color directly from position, bypassing the SDF stage.

| Stage | Parameters | Description |
|-------|-----------|-------------|
| `gradient(color_a, color_b, mode)` | (all required) | Two-color gradient. Mode: `"radial"` or `"linear"` |
| `spectrum(bass, mid, treble)` | `bass`: 0, `mid`: 0, `treble`: 0 | Audio-reactive spectrum visualization |

### 7.8 Pipeline Examples

```game
# Minimal: circle with glow
fn: circle(0.3) | glow(2.0)

# Transform chain: rotate, then draw
fn: rotate(time * 0.5) | circle(0.3) | glow(2.0) | tint(gold)

# SDF modifier: outlined polygon
fn: polygon(6, 0.3) | onion(0.02) | glow(3.0) | tint(cyan)

# Full chain: warp space, generate SDF, bridge to color, post-process
fn: domain_warp(0.2, 3.0) | fbm(2.0, 6, 0.5, 2.1) | glow(1.5) | tint(ember) | bloom(0.3, 2.0) | vignette(0.5, 0.8)

# Background gradient
fn: gradient(deep_blue, black, "radial")
```

### 7.9 Arguments

Stage arguments can be positional or named:

```game
circle(0.3)                    # positional: radius = 0.3
circle(radius: 0.3)           # named: same result
fbm(scale: 2.0, octaves: 8)   # named: skip defaults for others
ring(0.3, 0.02)               # positional: radius, width
```

When an argument is an identifier that is not a built-in function name, it becomes a user-defined uniform parameter:

```game
fn: circle(my_radius) | glow(my_intensity)
# "my_radius" and "my_intensity" become GPU uniforms
```

---

## 8. Modulation (`~` Operator)

The tilde operator `~` binds a parameter to a real-time signal. The base value is modulated by the signal expression each frame.

```game
radius: 0.3 ~ audio.bass * 0.5
intensity: 2.0 ~ audio.energy * 3.0
x_pos: 0.0 ~ mouse.x * 2.0 - 1.0
```

The expression after `~` is compiled to JavaScript and evaluated every frame. The result is added to (or replaces) the base value and uploaded to the GPU uniform buffer.

### Available Signals

| Signal | Description |
|--------|-------------|
| `audio.bass` | Low frequency energy (FFT) |
| `audio.mid` | Mid frequency energy (FFT) |
| `audio.treble` | High frequency energy (FFT) |
| `audio.energy` | Total audio energy |
| `audio.beat` | Beat detection pulse |
| `mouse.x` | Normalized cursor X (0.0 to 1.0) |
| `mouse.y` | Normalized cursor Y (0.0 to 1.0) |
| `data.*` | External data binding (arbitrary fields) |
| `time` | Elapsed time in seconds |

---

## 9. Temporal Operators

Temporal operators transform parameter values over time. They are applied after modulation and can be chained.

### 9.1 Delay (`>>`)

Ring buffer delay -- outputs the value from N duration ago:

```game
bass: 0.5 ~ audio.bass >> 200ms
```

Compiles to a `GameRingBuffer` JavaScript class with configurable buffer size.

### 9.2 Smooth (`<>`)

Exponential moving average -- smooths out rapid changes:

```game
energy: 0.0 ~ audio.energy <> 50ms
```

Compiles to a `GameEMAFilter` JavaScript class. The duration controls the smoothing window (longer = smoother).

### 9.3 Trigger (`!!`)

Edge detection with decay envelope -- fires on rising edges:

```game
beat: 0.0 ~ audio.beat !! 300ms
```

Compiles to a `GameEdgeDetector` JavaScript class. When the input rises above a threshold, the output snaps to 1.0 and decays over the specified duration.

### 9.4 Range Clamp (`..`)

Clamps the value to a range:

```game
volume: 0.5 ~ audio.energy .. [0.0, 1.0]
```

Compiles to inline `Math.min(Math.max(...))`.

### 9.5 Chaining

Temporal operators can be chained. They apply in left-to-right order:

```game
# Smooth, then delay, then clamp
bass: 0.5 ~ audio.bass <> 50ms >> 200ms .. [0.0, 1.0]
```

---

## 10. Define Blocks (Macros)

`define` creates reusable pipeline macros. Parameters are substituted at the call site during compilation.

```game
define hex_frame(size) {
  polygon(6.0, size) | onion(0.02) | glow(2.0)
}

layer frame {
  fn: rotate(time * 0.5) | hex_frame(0.3) | tint(cyan)
}
```

The compiler expands defines by replacing the call with the body stages, substituting formal parameters with actual arguments. Expansion supports nesting up to 16 levels deep.

Define blocks are scoped to the cinematic in which they are declared.

---

## 11. Import

Imports load definitions from other `.game` files.

### Named Imports

```game
import "stdlib/effects.game" expose bloom_pulse, shimmer
```

Imports specific named definitions from the file.

### Aliased Imports

```game
import "stdlib/effects.game" as fx
```

Imports the module under an alias.

---

## 12. Arc Blocks (Timeline Animation)

Arc blocks define keyframed parameter animation over time.

### 12.1 Simple Arc Entries

```game
arc {
  opacity: 0.0 -> 1.0 over 2s ease_in
  scale: 1.0 -> 3.0 over 4bars smooth
}
```

Format: `target: from -> to over duration [easing]`

### 12.2 Timestamp Moments

Arc entries can be grouped under labeled timestamps:

```game
arc {
  0:00 "void" {
    hex_size: 0.05
    hex_glow: 0.3
  }
  0:02 "ignite" {
    hex_size -> 0.25 ease(expo_out) over 2s
    hex_glow -> 3.0 ease(smooth) over 2s
  }
  0:06 "bloom" {
    hex_size -> 0.35 ease(cubic_in_out) over 3s
  }
}
```

Inside a timestamp block:
- `param: value` -- set static value at this moment
- `param -> value ease(fn) over duration` -- animate to target with easing

Timestamp format: `minutes:seconds` followed by an optional string label.

### 12.3 Easing Functions

The compiler emits a JavaScript easing library with 7 standard curves:

| Easing | Description |
|--------|-------------|
| `linear` | Constant speed (default) |
| `expo_in` | Exponential acceleration |
| `expo_out` | Exponential deceleration |
| `cubic_in_out` | Cubic ease in/out |
| `smooth` | Hermite smoothstep (`t*t*(3-2*t)`) |
| `elastic` | Elastic overshoot |
| `bounce` | Bounce at end |

### 12.4 Compilation

Arc blocks compile to a JavaScript `arcUpdate(time, params)` function containing a flat timeline array. Multiple arc blocks in a cinematic are concatenated sequentially. The runtime interpolates uniform values each frame using the easing functions.

---

## 13. Resonate Blocks (Cross-Layer Feedback)

Resonate blocks create controlled feedback loops between layers -- one layer's output modulates another layer's parameters.

```game
resonate {
  kick -> bg.scale * 0.3
  snare -> fg.intensity * 0.7
}
```

Format: `source -> target.field * weight`

- `source` -- a named signal (e.g., from a `listen` block)
- `target.field` -- a uniform parameter to modulate
- `weight` -- scaling factor (any expression)

The compiler emits a `resonanceUpdate(params, signals, dt)` JavaScript function that applies cross-layer modulation each frame with delta-time scaling. It includes cycle detection: multiple writes to the same target generate warnings.

### Example

```game
layer fire {
  fn: fbm(3.0) | glow(brightness) | tint(orange)
  brightness: 0.8
}

layer ice {
  fn: voronoi(density) | glow(1.0) | tint(cyan)
  density: 2.0
}

resonate {
  fire_output -> ice.density * 0.2
  ice_output -> fire.brightness * -0.1
}
```

---

## 14. React Blocks (Interaction)

React blocks map user input signals to runtime actions.

```game
react {
  mouse.click -> pulse(1.0)
  mouse.x -> intensity
  key("space") -> toggle(0)
  audio.bass > 0.8 -> pulse(2.0)
}
```

Format: `signal -> action`

### Recognized Signal Patterns

| Signal | Compiles to |
|--------|-------------|
| `mouse.click` | `canvas.addEventListener('click', ...)` |
| `mouse.x` | `canvas.addEventListener('mousemove', ...)` with normalized X |
| `mouse.y` | `canvas.addEventListener('mousemove', ...)` with inverted normalized Y |
| `key("x")` | `document.addEventListener('keydown', ...)` for key "x" |
| `audio.field > threshold` | Audio threshold check in animation loop |

### Recognized Action Patterns

| Action | Effect |
|--------|--------|
| `uniform_name` | Set the named uniform to the signal value |
| `pulse(magnitude)` | Impulse with decay |
| `toggle(index)` | Toggle a uniform between 0 and 1 |

The compiler emits a self-contained IIFE that attaches event listeners to the canvas element.

---

## 15. Listen Blocks (Audio Analysis)

Listen blocks define named audio analysis signals using Web Audio API DSP algorithms.

```game
listen {
  onset: attack(threshold: 0.7, decay: 300.0)
  melody: pitch(min: 200.0, max: 4000.0)
  rhythm: phase(subdivide: 16.0)
  drop: delta(window: 2.0)
}
```

Format: `signal_name: algorithm(params)`

### Algorithms

| Algorithm | Parameters | Description |
|-----------|-----------|-------------|
| `attack` | `threshold`: 0.7, `decay`: 300 (ms) | Onset detection via spectral energy flux |
| `pitch` | `min`: 200 (Hz), `max`: 4000 (Hz) | Pitch detection via autocorrelation |
| `phase` | `subdivide`: 16 | Beat subdivision / rhythm phase |
| `delta` | `window`: 2.0 (seconds) | Energy derivative (rate of change) |

Compiles to a `GameListenPipeline` JavaScript class that reads from a Web Audio `AnalyserNode` and exposes signals as `pipeline.signals['name']`.

---

## 16. Voice Blocks (Audio Synthesis)

Voice blocks define a Web Audio synthesis graph -- oscillators, filters, and effects chained together.

```game
voice {
  osc: sine(freq: 440.0)
  filter: lowpass(cutoff: 2000.0, q: 1.0)
  vol: gain(level: 0.5)
  verb: reverb(room: 0.4)
}
```

Format: `node_name: type(params)`

### Node Types

| Type | Parameters | Web Audio Node |
|------|-----------|---------------|
| `sine` | `freq`: 440 | `OscillatorNode` (type: sine) |
| `square` | `freq`: 440 | `OscillatorNode` (type: square) |
| `sawtooth` | `freq`: 440 | `OscillatorNode` (type: sawtooth) |
| `triangle` | `freq`: 440 | `OscillatorNode` (type: triangle) |
| `noise` | (none) | `AudioBufferSourceNode` (white noise buffer, looped) |
| `lowpass` | `cutoff`: 1000, `q`: 1.0 | `BiquadFilterNode` (type: lowpass) |
| `highpass` | `cutoff`: 1000, `q`: 1.0 | `BiquadFilterNode` (type: highpass) |
| `bandpass` | `cutoff`: 1000, `q`: 1.0 | `BiquadFilterNode` (type: bandpass) |
| `notch` | `cutoff`: 1000, `q`: 1.0 | `BiquadFilterNode` (type: notch) |
| `gain` | `level`: 0.5 | `GainNode` |
| `reverb` | `room`: 0.4 | `ConvolverNode` (generated impulse response) |

Nodes are connected in declaration order (first to last, last to destination). Compiles to a `GameVoiceSynth` JavaScript class with `connect(destination)`, `start()`, `stop()`, and `setParam(nodeName, paramName, value)` methods.

---

## 17. Score Blocks (Musical Sequencing)

Score blocks organize timeline animation into a musical structure of motifs, phrases, sections, and arrangements.

```game
score tempo(120) {
  motif rise {
    scale: 0.5 -> 2.0 over 4bars ease_in
  }
  motif fall {
    scale: 2.0 -> 0.5 over 2bars
  }

  phrase build = rise | fall
  section verse = build
  arrange: verse
}
```

### Structure

| Element | Syntax | Description |
|---------|--------|-------------|
| `tempo(BPM)` | Before `{` | Beats per minute (default: 120) |
| `motif name { entries }` | Arc entries | Named animation snippet |
| `phrase name = motif1 \| motif2` | Pipe-separated | Sequence of motifs |
| `section name = phrase1 phrase2` | Space-separated | Sequence of phrases |
| `arrange: section1 section2` | Space-separated | Final playback order |

The compiler flattens the hierarchy into a flat timeline with absolute timestamps, converting `bars` to seconds using the tempo. Compiles to a `GameScorePlayer` JavaScript class with `start(time)` and `evaluate(time)` methods, plus built-in easing support.

Motif entries use the same format as arc entries: `target: from -> to over duration [easing]`.

---

## 18. Gravity Blocks (Particle Physics)

Gravity blocks define N-body particle simulations that run on the GPU via compute shaders.

```game
gravity {
  rule: 1.0
  damping: 0.995
  bounds: reflect
}
```

### Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `rule` | expression | `1.0` | Force law expression |
| `damping` | float | `0.99` | Velocity damping per frame (0.0 = instant stop, 1.0 = no damping) |
| `bounds` | keyword | `reflect` | Boundary behavior: `reflect`, `wrap`, or `none` |

### Compilation

The gravity block compiles to:
1. **WGSL compute shader** with storage buffers for particle positions and velocities, N-body force accumulation, velocity integration, and boundary handling
2. **GameGravitySim** JavaScript class for GPU compute dispatch with ping-pong buffer swapping

Default particle count: 1024. The compute shader uses workgroup size 64.

---

## 19. Lens Blocks (Rendering Configuration)

Lens blocks configure rendering mode and post-processing.

```game
lens main {
  mode: flat
  camera: orbit(5.0, 2.0, 0.1)
  post: bloom(0.3, 2.0) | vignette(0.5, 0.8) | grain(0.02)
}
```

### Properties

Lens properties are parsed as named parameters. The `post:` key is special -- it starts a pipeline of post-processing stages using the same stage syntax as layers.

```game
lens {
  mode: flat
  post: tonemap(1.2) | chromatic(0.003) | scanlines(200, 0.2) | grain(0.05)
}
```

Lens names are optional. Multiple lenses can be declared per cinematic.

---

## 20. Breed Blocks (Genetic Composition)

Breed blocks merge parameters from two parent cinematics using inheritance rules and random mutations.

```game
breed "child" from "fire" + "ice" {
  inherit layers: mix(0.6)
  inherit params: pick(0.5)
  mutate scale: 0.3
  mutate intensity: 0.1
}
```

### Syntax

- Declared at **top level** (outside cinematic blocks)
- `from "parent_a" + "parent_b" [+ ...]` -- two or more parents
- Body contains `inherit` and `mutate` rules

### Inherit Strategies

| Strategy | Syntax | Description |
|----------|--------|-------------|
| `mix(weight)` | `inherit target: mix(0.6)` | Weighted blend: `a * weight + b * (1 - weight)` |
| `pick(probability)` | `inherit target: pick(0.5)` | Random selection: choose A with given probability |

### Mutate

```game
mutate scale: 0.3    # adds random value in [-0.3, +0.3]
```

Compiles to a `GameBreedMerger` JavaScript class with a `merge(parentA, parentB)` method.

---

## 21. Project Blocks (Projection Mapping)

Project blocks configure output for non-standard display surfaces.

```game
project dome(fisheye: 180, segments: 8) {
  source: my_layer
}
```

### Syntax

- Declared at **top level**
- Mode keyword followed by optional parameters in parentheses
- Body contains `source:` pointing to a layer name

### Projection Modes

| Mode | Description | Extra Parameters |
|------|-------------|-----------------|
| `flat` | Standard fullscreen (default vertex shader) | None |
| `dome` | Fisheye/equirectangular for planetarium domes | `fov_deg`, `segments` |
| `cube` | 6-face cubemap for immersive displays | `face_index` |
| `led` | LED strip sampling (horizontal 1D) | `count`, `aspect` |

Each mode generates a specialized WGSL vertex shader with appropriate UV mapping.

---

## 22. Built-in Uniforms

Every compiled shader receives these uniforms automatically:

| Uniform | Type | Description |
|---------|------|-------------|
| `time` | `f32` | Elapsed time in seconds (wraps at 120s) |
| `audio_bass` | `f32` | Low frequency energy |
| `audio_mid` | `f32` | Mid frequency energy |
| `audio_treble` | `f32` | High frequency energy |
| `audio_energy` | `f32` | Total audio energy |
| `audio_beat` | `f32` | Beat detection pulse |
| `resolution` | `vec2<f32>` | Canvas size in pixels |
| `mouse` | `vec2<f32>` | Normalized mouse position |
| `p_*` | `f32` | User-defined parameters (prefixed with `p_`) |

---

## 23. Compilation Model

```
.game source
    |
    v
 [Lexer]     -----> Token stream (logos crate, zero-allocation)
    |
    v
 [Parser]    -----> AST (recursive descent, hand-written)
    |
    v
 [Analysis]  -----> Define expansion, signal detection
    |
    v
 [Validation] ----> Pipeline state machine, cast type checking
    |
    v
 [Codegen]   -----> WGSL fragment + GLSL fragment + WGSL vertex + GLSL vertex
    |                + JS modules (temporal, listen, voice, score, gravity,
    |                  react, resonance, arc)
    |                + Optional compute shader (gravity)
    v
 [Runtime]   -----> Web Component JS (custom element, WebGPU + WebGL2 fallback)
                    OR standalone HTML page
                    OR TypeScript definitions
```

The pipe chain compiles to a single monolithic fragment shader per cinematic. The user writes modular, composable functions; the compiler merges them into optimal GPU code.

**Dual target:** Every `.game` file produces both WGSL (WebGPU) and GLSL ES 3.0 (WebGL2) shaders. The runtime auto-detects the available API and selects the appropriate shader at component initialization.

---

## 24. Formal Grammar (EBNF)

```ebnf
(* ===================== Top Level ===================== *)

program          = { import_decl | cinematic_decl | breed_decl | project_decl } ;

(* ===================== Import ===================== *)

import_decl      = "import" STRING ( "as" IDENT | "expose" ident_list ) ;
ident_list       = IDENT { "," IDENT } ;

(* ===================== Cinematic ===================== *)

cinematic_decl   = "cinematic" STRING "{" cinematic_body "}" ;
cinematic_body   = { layer_decl | arc_decl | resonate_decl | listen_decl
                   | voice_decl | score_decl | gravity_decl | lens_decl
                   | react_decl | define_decl
                   | signals_skip | route_skip | hear_skip | feel_skip } ;

(* Parsed but skipped blocks *)
signals_skip     = "signals" "{" skip_body "}" ;
route_skip       = "route" "{" skip_body "}" ;
hear_skip        = "hear" "{" skip_body "}" ;
feel_skip        = "feel" "{" skip_body "}" ;
skip_body        = { any_token_except_matching_brace } ;

(* ===================== Layer ===================== *)

layer_decl       = "layer" [ IDENT ] [ layer_opts ] [ memory_decl ] [ cast_decl ]
                   "{" layer_body "}" ;
layer_opts       = "(" param { "," param } ")" ;
memory_decl      = "memory" ":" NUMBER ;
cast_decl        = "cast" IDENT ;
layer_body       = fn_mixed_body | param_list | stage_pipeline ;

fn_mixed_body    = "fn" ":" stage_pipeline { param } ;
param_list       = { param } ;
param            = IDENT ":" expr [ "~" expr ] { temporal_op } ;

stage_pipeline   = stage { "|" stage } ;
stage            = IDENT "(" [ arg_list ] ")" ;
arg_list         = arg { "," arg } ;
arg              = [ IDENT ":" ] expr ;

temporal_op      = ">>" duration              (* delay *)
                 | "<>" duration              (* smooth *)
                 | "!!" duration              (* trigger *)
                 | ".." "[" expr "," expr "]"  (* range clamp *) ;

duration         = NUMBER "s" | NUMBER "ms" | NUMBER "bars" ;

(* ===================== Arc ===================== *)

arc_decl         = "arc" "{" { arc_item } "}" ;
arc_item         = timestamp_entry | arc_entry ;

timestamp_entry  = NUMBER ":" NUMBER [ STRING ] "{" { ts_body_entry } "}" ;
ts_body_entry    = IDENT ":" expr                          (* static set *)
                 | IDENT "->" expr [ ease ] [ over_dur ]   (* transition *) ;

arc_entry        = dotted_ident ":" expr "->" expr "over" duration [ IDENT ] ;

ease             = "ease" "(" IDENT ")" ;
over_dur         = "over" duration ;
dotted_ident     = IDENT { "." IDENT } ;

(* ===================== Resonate ===================== *)

resonate_decl    = "resonate" "{" { resonate_entry } "}" ;
resonate_entry   = IDENT "->" IDENT "." IDENT "*" expr ;

(* ===================== Listen ===================== *)

listen_decl      = "listen" "{" { listen_signal } "}" ;
listen_signal    = IDENT ":" IDENT [ "(" listen_params ")" ] ;
listen_params    = param { "," param } ;

(* ===================== Voice ===================== *)

voice_decl       = "voice" "{" { voice_node } "}" ;
voice_node       = IDENT ":" IDENT [ "(" listen_params ")" ] ;

(* ===================== Score ===================== *)

score_decl       = "score" [ "tempo" "(" NUMBER ")" ] "{" score_body "}" ;
score_body       = { motif_decl | phrase_decl | section_decl | arrange_decl } ;

motif_decl       = "motif" IDENT "{" { arc_entry } "}" ;
phrase_decl      = "phrase" IDENT "=" IDENT { "|" IDENT } ;
section_decl     = "section" IDENT "=" IDENT { IDENT } ;
arrange_decl     = "arrange" ":" IDENT { IDENT } ;

(* ===================== Gravity ===================== *)

gravity_decl     = "gravity" "{" { gravity_prop } "}" ;
gravity_prop     = "rule" ":" expr
                 | "damping" ":" NUMBER
                 | "bounds" ":" ( "reflect" | "wrap" | "none" ) ;

(* ===================== Lens ===================== *)

lens_decl        = "lens" [ IDENT ] "{" { lens_item } "}" ;
lens_item        = "post" ":" stage { "|" stage }         (* post-processing pipeline *)
                 | IDENT ":" expr                          (* property *) ;

(* ===================== React ===================== *)

react_decl       = "react" "{" { reaction } "}" ;
reaction         = expr "->" expr ;

(* ===================== Define ===================== *)

define_decl      = "define" IDENT [ "(" ident_list ")" ] "{" stage_pipeline "}" ;

(* ===================== Breed ===================== *)

breed_decl       = "breed" STRING "from" STRING { "+" STRING }
                   "{" { inherit_rule | mutate_rule } "}" ;
inherit_rule     = "inherit" IDENT ":" IDENT "(" NUMBER ")" ;
mutate_rule      = "mutate" IDENT ":" NUMBER ;

(* ===================== Project ===================== *)

project_decl     = "project" IDENT [ "(" param { "," param } ")" ]
                   "{" { project_prop } "}" ;
project_prop     = IDENT ":" IDENT [ "," ] ;

(* ===================== Expressions ===================== *)

expr             = ternary ;
ternary          = comparison [ "?" expr ":" expr ] ;
comparison       = additive { ( ">" | "<" ) additive } ;
additive         = term { ( "+" | "-" ) term } ;
term             = factor { ( "*" | "/" ) factor } ;
factor           = atom [ "^" factor ] ;                   (* right-associative *)
atom             = NUMBER | STRING | IDENT [ call_or_dot ]
                 | "(" expr ")" | "[" expr { "," expr } "]"
                 | "-" factor | duration ;
call_or_dot      = "(" [ arg_list ] ")"                    (* function call *)
                 | "." IDENT ;                              (* dotted access *)

(* ===================== Terminals ===================== *)

NUMBER           = INTEGER | FLOAT ;
INTEGER          = digit { digit } ;
FLOAT            = digit { digit } "." digit { digit } ;
STRING           = '"' { any_char_except_quote } '"' ;
IDENT            = ( letter | "_" ) { letter | digit | "_" } ;
comment          = ( "#" | "//" ) { any_char_except_newline } ;
```

---

## 25. Complete Example

```game
# "Audio Spectrum" — multi-layer audio-reactive rings
cinematic "Audio Spectrum" {
  layer bg {
    fn: gradient(deep_blue, black, "radial")
  }

  layer bass_ring {
    fn: ring(0.15, 0.04) | glow(bass_g) | tint(ember)
    bass_g: 1.0 ~ audio.bass * 6.0
  }

  layer mid_ring {
    fn: ring(0.25, 0.03) | glow(mid_g) | tint(cyan)
    mid_g: 1.0 ~ audio.mid * 5.0
  }

  layer treble_ring {
    fn: ring(0.35, 0.02) | glow(treble_g) | tint(frost)
    treble_g: 1.0 ~ audio.treble * 4.0
  }

  layer core {
    fn: circle(0.06) | glow(energy_g) | tint(gold)
    energy_g: 2.0 ~ audio.energy * 8.0
  }
}
```

---

## 26. Advanced Example

```game
# "Cinematic Arc" — timeline-driven animation with macros
cinematic "Cinematic Arc" {
  define hex_frame(size) {
    polygon(6.0, size) | onion(0.02) | glow(hex_glow)
  }

  layer bg {
    fn: gradient(black, deep_blue, "radial")
  }

  layer frame {
    fn: rotate(time * 0.5) | hex_frame(hex_size) | tint(cyan)
    hex_size: 0.3
    hex_glow: 2.0
  }

  layer core {
    fn: circle(core_r) | glow(core_g) | tint(gold)
    core_r: 0.05
    core_g: 1.0
  }

  arc {
    0:00 "void" {
      hex_size: 0.05
      hex_glow: 0.3
      core_r: 0.01
      core_g: 0.5
    }
    0:02 "ignite" {
      hex_size -> 0.25 ease(expo_out) over 2s
      hex_glow -> 3.0 ease(smooth) over 2s
      core_r -> 0.06 ease(expo_out) over 1s
      core_g -> 4.0 ease(expo_out) over 2s
    }
    0:06 "bloom" {
      hex_size -> 0.35 ease(cubic_in_out) over 3s
      hex_glow -> 5.0 ease(expo_out) over 2s
      core_r -> 0.08 ease(smooth) over 2s
      core_g -> 6.0 ease(expo_out) over 2s
    }
    0:12 "dissolve" {
      hex_size -> 0.5 ease(smooth) over 4s
      hex_glow -> 0.3 ease(expo_in) over 3s
      core_r -> 0.02 ease(expo_in) over 3s
      core_g -> 0.5 ease(smooth) over 3s
    }
  }
}
```

---

## 27. Parsed but Not Yet Implemented

The following blocks are **recognized by the parser** (they will not cause parse errors) but their contents are **discarded** during compilation. They produce no output:

| Block | Intended Purpose |
|-------|-----------------|
| `signals { }` | Named signal routing |
| `route { }` | Signal routing graph |
| `hear { }` | Alternative audio input |
| `feel { }` | Haptic/tactile feedback |

These blocks exist as forward-compatible syntax reservations. You can include them in `.game` files without breaking compilation, but they have no effect.

---

## 28. Future / Not Implemented

The following features are **not implemented** in the compiler. Do not use them -- they will produce parse errors or be silently ignored:

### Planned SDF Operations
- Boolean operations: `union`, `intersect`, `subtract`, `smooth_union(k)`
- Domain operations: `bend`, `elongate`

### Planned Noise Types
- `worley` noise, `value` noise

### Planned Shading
- `fresnel`, `iridescent`, `subsurface` shading models

### Planned Lens Modes
- `volume` (volumetric rendering)
- `particles` (GPU particle system)
- `trace` (path tracing)

### Planned Camera Types
- `dolly`, `crane`, `handheld`, `track`

### Planned Audio Analysis
- `beat_detect`, `onset`, `pitch_track`, `spectral_flux` (as listen algorithms -- note that `attack`, `pitch`, `phase`, and `delta` ARE implemented)

### Planned Particle Types
- `burst`, `stream`, `swarm`, `fireflies`, `rain`, `sparks`

### Planned Features
- `branch` -- conditional arcs (interactive narrative)
- `loop` -- repeating sections
- `export` -- offline frame rendering to video
- `midi` -- MIDI input as signals
- `osc` -- Open Sound Control
- `wasm_fn` -- custom WASM escape hatch
