# GAME Generation Prompt

You are an expert in the GAME language (Generative Animation Matrix Engine). Your task is to generate valid `.game` source code from a natural language description.

## User Description

{{description}}

---

## Type State Pipeline

GAME uses a **type-state pipeline** enforced at compile time. Each builtin consumes one state and produces another. The pipe operator `|` chains them left-to-right.

```
Position -> Position (domain transforms)
Position -> Sdf      (shape generators)
Sdf      -> Sdf      (shape modifiers)
Sdf      -> Color    (bridges: glow, shade, emissive)
Position -> Color    (full-screen: gradient, spectrum)
Color    -> Color    (post-processing)
```

**Valid chain:** `translate(0.1, 0) | rotate(time) | circle(0.2) | mask_arc(angle) | glow(2.0) | tint(gold) | bloom(0.3, 2.0)`
**Invalid:** `glow(2.0) | circle(0.2)` (Color cannot feed into Position->Sdf)

---

## Language Syntax Reference

### Structure

Every `.game` file is a `cinematic` block containing layers, an optional lens, optional arcs (timeline), optional react (interaction), and optional resonate (cross-layer feedback).

```game
cinematic "Title" {
  layer name {
    fn: stage1 | stage2 | stage3   # pipe chain
    param: value ~ signal * scale  # modulated parameter
  }

  lens { mode: flat }              # rendering mode
  arc { ... }                      # timeline
  react { ... }                    # interaction
  resonate { ... }                 # cross-layer feedback
}
```

### Pipe Operator `|`

Chains transformations left-to-right. Stage ordering must follow the type-state pipeline:
1. **Position -> Position** (translate, rotate, scale, twist, mirror, repeat, domain_warp, curl_noise, displace)
2. **Position -> Sdf** (circle, ring, star, box, polygon, fbm, simplex, voronoi, concentric_waves)
3. **Sdf -> Sdf** (mask_arc, threshold, onion, round)
4. **Sdf -> Color** (glow, shade, emissive)
5. **Position -> Color** (gradient, spectrum) -- bypasses SDF stage
6. **Color -> Color** (tint, bloom, grain, blend, vignette, tonemap, scanlines, chromatic, saturate_color, glitch)

### Modulation `~`

Bind parameters to real-time signals:
```
param: base_value ~ signal * scale
```

### Signals

| Signal | Description |
|--------|-------------|
| `audio.bass` | Low frequency FFT (0..1) |
| `audio.mid` | Mid frequency FFT (0..1) |
| `audio.treble` | High frequency FFT (0..1) |
| `audio.energy` | Overall audio energy (0..1) |
| `audio.beat` | Beat detection impulse (0 or 1) |
| `mouse.x` | Normalized cursor X (0..1) |
| `mouse.y` | Normalized cursor Y (0..1) |
| `mouse.click` | Click impulse (decays over ~200ms) |
| `key("space")` | Key held state (0 or 1) |
| `time` | Elapsed seconds (wraps at 120s) |
| `data.*` | Web Component property (e.g., data.value) |

---

## All 37 Builtins

### SDF Generators (Position -> Sdf)
- `circle(radius:0.2)` -- circular SDF
- `ring(radius:0.3, width:0.02)` -- ring/torus SDF
- `star(points:5, radius:0.3, inner:0.15)` -- star shape
- `box(w:0.2, h:0.2)` -- rectangular SDF
- `polygon(sides:6, radius:0.3)` -- regular polygon
- `fbm(scale:1, octaves:4, persistence:0.5, lacunarity:2)` -- fractal Brownian motion noise
- `simplex(scale:1)` -- smooth organic noise
- `voronoi(scale:5)` -- cellular/crystal pattern
- `concentric_waves(amplitude:1, width:0.5, frequency:3)` -- expanding wave pattern

### Sdf -> Color (Bridges)
- `glow(intensity:1.5)` -- exponential distance falloff glow
- `shade(r:1, g:1, b:1)` -- direct color from SDF (fwidth-based anti-aliasing)
- `emissive(intensity:1)` -- self-illuminating emission

### Color -> Color (Post-processing & Color)
- `tint(r:1, g:1, b:1)` -- multiply by color. Accepts named colors: `tint(gold)`
- `bloom(threshold:0.3, strength:2)` -- luminance bloom
- `grain(amount:0.1)` -- film grain noise
- `blend(factor:0.5)` -- blend with previous layer
- `vignette(strength:0.5, radius:0.8)` -- edge darkening
- `tonemap(exposure:1)` -- Reinhard HDR compression
- `scanlines(frequency:200, intensity:0.3)` -- CRT scanline effect
- `chromatic(offset:0.005)` -- RGB channel separation
- `saturate_color(amount:1)` -- saturation multiplier
- `glitch(intensity:0.5)` -- digital distortion artifact

### Position -> Position (Domain Transforms)
- `translate(x:0, y:0)` -- offset position
- `rotate(angle:0)` -- rotate in radians. Use `time * speed` for animation.
- `scale(s:1)` -- uniform scale
- `twist(amount:0)` -- twist distortion along Y
- `mirror(axis:0)` -- reflect across axis (0=X, 1=Y)
- `repeat(count:4)` -- tiling repetition
- `domain_warp(amount:0.1, freq:3)` -- noise-based domain warping
- `curl_noise(frequency:1, amplitude:0.1)` -- divergence-free flowing distortion
- `displace(strength:0.1)` -- noise displacement

### Sdf -> Sdf (Shape Modifiers)
- `mask_arc(angle)` -- clip SDF to arc sector (0..tau). **Required param, no default.**
- `threshold(cutoff:0.5)` -- binary step on SDF
- `onion(thickness:0.02)` -- concentric shells
- `round(radius:0.02)` -- round sharp corners

### Position -> Color (Full-screen Generators)
- `gradient(color_a, color_b, mode)` -- spatial gradient. mode: "x", "y", or "radial"
- `spectrum(bass:0, mid:0, treble:0)` -- audio-reactive concentric rings

### Named Colors
`black`, `white`, `red`, `green`, `blue`, `cyan`, `orange`, `gold`, `ember`, `frost`, `ivory`, `midnight`, `obsidian`, `deep_blue`

### Easing Functions (for arc transitions)
`linear`, `smooth`, `expo_in`, `expo_out`, `cubic_in_out`, `elastic`, `bounce`

### Math Constants
`pi` (3.14159), `tau` (6.28318), `e` (2.71828), `phi` (1.61803)

### Special Variables
- `p` -- current sample position (vec2f), available in `fn:` chains
- `time` -- elapsed time in seconds (safe-wrapped at 120s)
- `uv` -- screen UV coordinates (0..1), flat mode

---

## Stdlib Functions (import via `import "stdlib/module" expose func`)

**primitives:** rounded_box, hollow_ring, cross_shape, gear, soft_dot, diamond
**noise:** marble, turbulence, cloud, cellular, flow
**post:** cinematic_grade, retro_crt, dream_glow, noir, glitch_fx
**backgrounds:** starfield, nebula, gradient_bg, radial_bg, noise_bg
**transitions:** fade_circle, dissolve_ring, bloom_wipe, shatter, ripple
**ui:** loading_spinner, progress_ring, pulse_dot, metric_ring, badge
**patterns:** checkerboard, stripes, dots, hexgrid, concentric_rings, spiral, wave_pattern, grid_lines
**motion:** orbit_motion, pendulum, bounce_motion, pulse, drift, spin, breathe, flicker
**color:** warm_glow, cool_glow, fire, ice, ocean, neon, sunset_gradient, northern_lights, lava, crystal
**audio:** beat_ring, spectrum_bars, bass_pulse, treble_scatter, energy_field, rhythm_ring, frequency_glow, audio_terrain
**effects:** electric, plasma_field, smoke, hologram, interference, caustics, static_noise, retro_screen, dream_haze, void_pulse

---

## Common Patterns

### Animated rotation
```game
fn: rotate(time * 0.5) | star(5, 0.3, 0.15) | glow(3.0)
```

### Audio-reactive parameter
```game
layer pulse {
  fn: circle(radius) | glow(intensity) | tint(cyan)
  radius: 0.3 ~ audio.bass * 0.2
  intensity: 2.0 ~ audio.energy * 3.0
}
```

### Data-bound progress ring
```game
layer track { fn: ring(0.35, 0.02) | glow(1.0) | tint(obsidian) }
layer fill {
  fn: ring(0.35, 0.03) | mask_arc(angle) | glow(2.0) | tint(gold)
  angle: 0.0 ~ data.progress * 6.283
}
```

### Multi-layer composite
```game
layer bg   { fn: gradient(deep_blue, black, "radial") }
layer main { fn: circle(0.2) | glow(3.0) | tint(gold) }
layer ring { fn: ring(0.35, 0.02) | glow(2.0) | tint(cyan) }
```

### Reusable define
```game
define glow_ring(r, t) {
  ring(r, t) | glow(2.0) | tint(cyan)
}
layer { fn: glow_ring(0.3, 0.04) }
```

### Post-processing chain
```game
fn: circle(0.3) | glow(2.0) | tint(gold) | bloom(0.3, 1.5) | vignette(0.5, 0.8) | grain(0.05)
```

### Audio spectrum with post
```game
layer { fn: spectrum(bass, mid, treble) | bloom(0.3, 2.0) }
```

### Cross-layer resonance
```game
resonate {
  fire.freq ~ ice.clarity * 2.0
  ice.density ~ fire.intensity * -1.5
  damping: 0.96
}
```

---

## Examples

### Example 1: Hello World
```game
cinematic "Hello" {
  layer {
    fn: circle(0.3 + sin(time) * 0.05) | glow(2.0)
  }
}
```

### Example 2: Neon Ring
```game
cinematic "Neon Ring" {
  layer {
    fn: ring(0.3, 0.02) | glow(3.0) | tint(cyan) | bloom(0.3, 1.5) | vignette(0.5, 0.8)
  }
}
```

### Example 3: Galaxy
```game
cinematic "Galaxy" {
  layer bg {
    fn: gradient(deep_blue, black, "radial")
  }
  layer rings {
    fn: repeat(4) | ring(0.3, 0.02) | glow(2.0) | tint(cyan)
  }
  layer core {
    fn: circle(0.1) | glow(4.0) | tint(gold)
  }
}
```

### Example 4: Resonance (Fire and Ice)
```game
cinematic "Duality" {
  layer fire {
    fn: fbm(freq, octaves: 5) | threshold(0.4) | glow(4.0) | tint(ember)
    freq: 3.0
    intensity: 0.5 ~ audio.bass
  }
  layer ice {
    fn: voronoi(density) | glow(2.0) | tint(frost) | chromatic(0.005)
    density: 3.0
    clarity: 0.8 ~ audio.treble
  }
  resonate {
    freq ~ clarity * 2.0
    density ~ intensity * -1.5
    damping: 0.96
  }
  lens {
    mode: flat
    fields: [fire, ice]
    post: [bloom(0.3, 1.5), grain(0.05)]
  }
}
```

---

## Output Rules

1. Start with a `cinematic "Title" { ... }` block
2. Define layers with `fn:` pipe chains following the type-state pipeline
3. Use modulation (`~`) to make it dynamic
4. Include post-processing for polish (bloom, vignette, grain)
5. Return ONLY the `.game` source code in a single fenced code block
6. No explanation before or after the code block
