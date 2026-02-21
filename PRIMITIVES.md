# GAME Built-in Primitives

The `.game` language compiles these high-level primitives into WGSL shader code. Users write readable names; the compiler emits optimized GPU math.

> **Implemented primitives only.** This document reflects what the GAME compiler v0.2.0 actually compiles. Primitives listed here produce valid WGSL output.

---

## SDF Primitives

Signed distance fields: negative inside, positive outside, zero at surface. These produce `sdf_result` in the shader.

| Primitive | Syntax | Parameters |
|-----------|--------|------------|
| Circle | `circle(r)` | `r`: radius (default 0.5) |
| Sphere | `sphere(r)` | `r`: radius (default 0.5, 3D SDF projected to 2D) |
| Ring | `ring(radius, thickness)` | `radius`: center distance (default 0.3), `thickness`: wall width (default 0.04) |
| Box | `box(w, h)` | `w`: width (default 0.5), `h`: height (default 0.5) |
| Torus | `torus(R, r)` | `R`: major radius (default 0.3), `r`: minor radius (default 0.05) |
| Line | `line(x1, y1, x2, y2, thickness)` | Segment endpoints + `thickness` (default 0.02) |
| Polygon | `polygon(sides, radius)` | `sides`: number (default 6), `radius`: size (default 0.3) |
| Star | `star(points, outer, inner)` | `points`: number (default 5), `outer`: radius (default 0.4), `inner`: radius (default 0.2) |

### Example

```game
cinematic {
  layer { fn: circle(0.3) | glow(2.0) | tint(cyan) }
}
```

---

## Domain Operations

Transform position before SDF evaluation. Place *before* shapes in pipe chain.

| Operation | Syntax | Effect |
|-----------|--------|--------|
| Translate | `translate(x, y)` | Move in 2D space (default 0.0) |
| Rotate | `rotate(angle)` | 2D rotation in radians. Use `time` expressions for animation. |
| Scale | `scale(s)` | Uniform scale factor (default 1.0). SDF result auto-corrected. |
| Repeat | `repeat(spacing)` | Infinite spatial tiling (default 1.0) |
| Mirror | `mirror(axis)` | Reflect across axis: `"x"`, `"y"`, or `"xy"` (default `"xy"`) |
| Twist | `twist(amount)` | Twist along Y axis (default 1.0) |

### Example

```game
cinematic {
  layer {
    fn: translate(0.2, 0.0) | rotate(time * 0.5) | star(5, 0.3, 0.15) | glow(3.0)
  }
}
```

---

## SDF Modifiers

Modify an existing SDF. Place *after* a shape in the pipe chain.

| Modifier | Syntax | Effect |
|----------|--------|--------|
| Mask arc | `mask_arc(angle)` | Clip SDF to arc sector (0..6.283 radians) |
| Displace | `displace(strength)` | Noise-based surface displacement (default 0.1, uses simplex noise) |
| Round | `round(r)` | Round sharp edges by `r` (default 0.05) |
| Onion | `onion(thickness)` | Create concentric shells (default 0.02) |

### Example

```game
cinematic {
  layer {
    fn: polygon(6, 0.3) | onion(0.02) | glow(3.0)
  }
}
```

---

## Noise Functions

Procedural noise as SDF source. Produce `sdf_result`.

| Function | Syntax | Character |
|----------|--------|-----------|
| FBM | `fbm(pos, octaves:N, persistence:P, lacunarity:L)` | Fractal Brownian Motion — layered noise (default: pos=p, octaves=6, persistence=0.5, lacunarity=2.0) |
| Simplex | `simplex(frequency)` | Smooth organic noise (default frequency=1.0) |
| Voronoi | `voronoi(frequency)` | Cellular/crystal pattern (default frequency=1.0) |

### Example

```game
cinematic {
  layer {
    fn: fbm(p * 3.0, octaves: 4, persistence: 0.6) | shade(albedo: gold, emissive: ember)
  }
}
```

---

## Glow

Bridge from SDF to visual intensity.

| Function | Syntax | Effect |
|----------|--------|--------|
| Glow | `glow(intensity)` | Exponential distance falloff (default 2.0). Converts SDF → glow_result. |

---

## Shading & Color

Color stages. Follow SDF, glow, or other color stages.

| Function | Syntax | Effect |
|----------|--------|--------|
| Shade | `shade(albedo: color, emissive: color)` | PBR-style shading. Named params: `albedo` (default vec3f(0.8)), `emissive` (default vec3f(0.0)). |
| Emissive | `emissive()` | Quick self-illuminating gold glow |
| Colormap | `colormap()` | Map SDF distance to color gradient (dark blue → gold) |
| Spectrum | `spectrum(bass, mid, treble)` | Audio-reactive concentric rings per frequency band |
| Tint | `tint(color)` | Multiply glow/color by a named color or vec3f |
| Gradient | `gradient(color_a, color_b, direction)` | Spatial gradient: `"x"`, `"y"`, or `"radial"` (default `"y"`) |

### Named Colors

Available for `tint()`, `shade()`, `gradient()`:

| Name | RGB |
|------|-----|
| `black` | 0.0, 0.0, 0.0 |
| `white` | 1.0, 1.0, 1.0 |
| `red` | 1.0, 0.0, 0.0 |
| `green` | 0.0, 1.0, 0.0 |
| `blue` | 0.0, 0.0, 1.0 |
| `cyan` | 0.0, 1.0, 1.0 |
| `orange` | 1.0, 0.5, 0.0 |
| `gold` | 0.831, 0.686, 0.216 |
| `ember` | 0.8, 0.2, 0.05 |
| `frost` | 0.85, 0.92, 1.0 |
| `ivory` | 1.0, 0.97, 0.92 |
| `midnight` | 0.0, 0.0, 0.1 |
| `obsidian` | 0.04, 0.04, 0.06 |
| `deep_blue` | 0.0, 0.02, 0.15 |

---

## Post-Processing

Screen-space effects. Apply after color stages in the pipe chain.

| Effect | Syntax | Parameters |
|--------|--------|------------|
| Bloom | `bloom(threshold, intensity)` | threshold: luminance cutoff (default 0.6), intensity: glow (default 1.5) |
| Chromatic | `chromatic(strength)` | RGB channel separation (default 0.5) |
| Vignette | `vignette(strength)` | Edge darkening (default 0.3) |
| Grain | `grain(amount)` | Film grain noise (default 0.02) |
| Fog | `fog(density, color)` | Distance fog, color as vec3f (default black) |
| Glitch | `glitch(intensity)` | Digital artifact effect (default 0.5) |
| Scanlines | `scanlines(count, intensity)` | CRT effect, count: line frequency (default 100), intensity (default 0.3) |
| Tonemap | `tonemap(exposure)` | Reinhard-style HDR compression (default 1.0) |
| Invert | `invert()` | Invert all colors (1.0 - rgb) |
| Saturate | `saturate_color(amount)` | Saturation multiplier (default 1.5). >1 increases, <1 decreases. |

### Example

```game
cinematic {
  layer {
    fn: circle(0.3) | glow(3.0) | tint(gold) | bloom(0.5, 1.2) | vignette(0.4) | grain(0.01)
  }
}
```

---

## Signals

Real-time modulation via the `~` operator. Params react to signals each frame.

| Signal | Syntax | Range |
|--------|--------|-------|
| `audio.bass` | `~ audio.bass` | Low frequency FFT (0..1) |
| `audio.mid` | `~ audio.mid` | Mid frequency FFT (0..1) |
| `audio.treble` | `~ audio.treble` | High frequency FFT (0..1) |
| `audio.energy` | `~ audio.energy` | Overall energy (0..1) |
| `audio.beat` | `~ audio.beat` | Beat impulse (0 or 1) |
| `mouse.x` | `~ mouse.x` | Cursor X normalized (0..1) |
| `mouse.y` | `~ mouse.y` | Cursor Y normalized (0..1) |
| `data.*` | `~ data.value` | Web Component property. Any field name. |

### Modulation Syntax

```game
param_name: base_value ~ signal * scale
```

Example: `radius: 0.3 ~ audio.bass * 0.5` — base 0.3, adds up to 0.5 when bass peaks.

---

## Language Features

### Define (Reusable Macros)

```game
define glow_ring(r, t) {
  ring(r, t) | glow(2.0) | tint(cyan)
}

layer { fn: glow_ring(0.3, 0.04) }
```

Defines expand inline at compile time. Parameters are substituted by position.

### Multi-Layer Compositing

Multiple layers composite additively:

```game
layer bg   { fn: ring(0.4, 0.02) | glow(0.5) | tint(frost) }
layer main { fn: circle(0.2) | glow(3.0) | tint(gold) }
```

**Important:** Use unique param names across layers. Duplicate names produce a compiler warning.

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

### Lens Modes

```game
lens { mode: flat }        # Default: 2D SDF rendering
lens { mode: raymarch }    # 3D raymarching with orbit camera
```

### Constants

`pi` (3.14159), `tau` (6.28318), `e` (2.71828), `phi` (1.61803 golden ratio)

### Special Variables

| Variable | Available In | Meaning |
|----------|-------------|---------|
| `p` | `fn:` chains | Current sample position (vec2f) |
| `time` | Everywhere | Elapsed time in seconds (safe-wrapped at 120s) |
| `uv` | `flat` mode | Screen UV coordinates (0..1) |
| `height` | After SDF eval | Normalized distance (for shade/colormap) |
