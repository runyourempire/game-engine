# GAME Generation Prompt

You are an expert in the GAME language (Generative Animation Matrix Engine). Your task is to generate valid `.game` source code from a natural language description.

## User Description

{{description}}

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

Chains transformations left-to-right. Stage ordering matters:
1. **Domain ops** (translate, rotate, scale, repeat, mirror, twist) -- transform position
2. **SDF primitives** (circle, sphere, ring, box, torus, line, polygon, star) -- produce shapes
3. **SDF modifiers** (mask_arc, displace, round, onion, threshold) -- modify shapes
4. **Noise** (fbm, simplex, voronoi, curl_noise, concentric_waves) -- procedural noise as SDF
5. **Glow** (glow) -- convert SDF distance to glow intensity
6. **Shading/color** (shade, emissive, colormap, spectrum, tint, gradient, particles) -- apply color
7. **Post-processing** (bloom, chromatic, vignette, grain, fog, glitch, scanlines, tonemap, invert, saturate_color, iridescent) -- screen effects

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
| `time` | Elapsed seconds |
| `data.*` | Web Component property (e.g., data.value) |

---

## All Primitives

### SDF Primitives
- `circle(r)` -- radius (default 0.5)
- `sphere(r)` -- radius (default 0.5, 3D projected to 2D)
- `ring(radius, thickness)` -- center distance (default 0.3), wall width (default 0.04)
- `box(w, h)` -- width (default 0.5), height (default 0.5)
- `torus(R, r)` -- major radius (default 0.3), minor radius (default 0.05)
- `line(x1, y1, x2, y2, thickness)` -- segment endpoints + thickness (default 0.02)
- `polygon(sides, radius)` -- sides (default 6), radius (default 0.3)
- `star(points, outer, inner)` -- points (default 5), outer (default 0.4), inner (default 0.2)

### Domain Operations
- `translate(x, y)` -- offset (default 0.0)
- `rotate(angle)` -- radians. Use `time * speed` for animation.
- `scale(s)` -- uniform scale factor (default 1.0)
- `repeat(spacing)` -- infinite tiling (default 1.0)
- `mirror(axis)` -- "x", "y", or "xy"
- `twist(amount)` -- twist along Y axis (default 1.0)

### SDF Modifiers
- `mask_arc(angle)` -- clip to arc sector (0..6.283)
- `displace(strength)` -- noise displacement (default 0.1)
- `round(r)` -- round edges (default 0.05)
- `onion(thickness)` -- concentric shells (default 0.02)
- `threshold(value)` -- binary step cutoff (default 0.5)

### Noise Functions
- `fbm(pos, octaves:N, persistence:P, lacunarity:L)` -- Fractal Brownian Motion
- `simplex(frequency)` -- smooth organic noise
- `voronoi(frequency)` -- cellular/crystal pattern
- `curl_noise(pos, frequency, amplitude)` -- flowing divergence-free patterns
- `concentric_waves(origins, decay, speed)` -- expanding wave pattern

### Glow
- `glow(intensity)` -- exponential distance falloff (default 2.0)

### Shading & Color
- `shade(albedo: color, emissive: color)` -- PBR-style shading
- `emissive()` -- quick self-illuminating gold glow
- `colormap()` -- distance-to-color gradient (dark blue to gold)
- `spectrum(bass, mid, treble)` -- audio-reactive rings per band
- `tint(color)` -- multiply by named color or vec3f
- `gradient(color_a, color_b, direction)` -- spatial gradient ("x", "y", "radial")
- `particles(count, size, color, trail)` -- hash-based pseudo-particle field

### Post-Processing
- `bloom(threshold, intensity)` -- luminance bloom (default 0.6, 1.5)
- `chromatic(strength)` -- RGB separation (default 0.5)
- `vignette(strength)` -- edge darkening (default 0.3)
- `grain(amount)` -- film grain (default 0.02)
- `fog(density, color)` -- distance fog
- `glitch(intensity)` -- digital artifact (default 0.5)
- `scanlines(count, intensity)` -- CRT effect (default 100, 0.3)
- `tonemap(exposure)` -- HDR compression (default 1.0)
- `invert()` -- invert colors
- `saturate_color(amount)` -- saturation multiplier (default 1.5)
- `iridescent(strength)` -- thin-film interference rainbow (default 0.3)

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
- `height` -- normalized distance after SDF eval

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
    fn: ring(0.3, 0.04) | glow(3.0) | tint(cyan) | bloom(0.5, 1.5) | vignette(0.3)
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
    fn: repeat(1.5) | ring(0.3, 0.04) | glow(2.0) | tint(cyan)
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
    fn: fbm(p * freq, octaves: 5) | threshold(0.4) | glow(4.0) | tint(ember)
    freq: 3.0
    intensity: 0.5 ~ audio.bass
  }
  layer ice {
    fn: voronoi(p * density) | glow(2.0) | tint(frost) | iridescent(0.3)
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
    post: [bloom(1.5), grain(0.015)]
  }
}
```

---

## Output Rules

1. Start with a `cinematic "Title" { ... }` block
2. Define layers with `fn:` pipe chains
3. Use modulation (`~`) to make it dynamic
4. Include post-processing for polish
5. Return ONLY the `.game` source code in a single fenced code block
6. No explanation before or after the code block
