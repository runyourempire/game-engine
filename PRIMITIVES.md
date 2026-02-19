# GAME Built-in Primitives

The `.game` language compiles these high-level primitives into WGSL shader code. Users write readable names; the compiler emits optimized GPU math.

---

## SDF Primitives

Every primitive returns a signed distance — negative inside, positive outside, zero at the surface.

| Primitive | Syntax | Parameters |
|-----------|--------|------------|
| Sphere | `sphere(r)` | `r`: radius |
| Box | `box(w, h, d)` | width, height, depth (or `box(size)` for cube) |
| Torus | `torus(R, r)` | `R`: major radius, `r`: minor radius |
| Cylinder | `cylinder(r, h)` | `r`: radius, `h`: height |
| Plane | `plane(normal, offset)` | `normal`: direction vector, `offset`: distance from origin |
| Capsule | `capsule(a, b, r)` | `a`, `b`: endpoints, `r`: radius |
| Cone | `cone(angle, h)` | `angle`: half-angle, `h`: height |
| Line | `line(a, b, r)` | segment from `a` to `b`, thickness `r` |
| Circle (2D) | `circle(r)` | For flat/2D mode: 2D circle SDF |

### WGSL Example: sphere(0.5) compiles to

```wgsl
fn sdf_sphere(p: vec3f, radius: f32) -> f32 {
    return length(p) - radius;
}
```

---

## Boolean Operations

Combine SDFs to build complex shapes.

| Operation | Syntax | Effect |
|-----------|--------|--------|
| Union | `union(a, b)` | Merge shapes |
| Smooth union | `smooth_union(a, b, k)` | Merge with rounded blend (`k` = smoothness) |
| Intersection | `intersect(a, b)` | Only where both shapes overlap |
| Smooth intersect | `smooth_intersect(a, b, k)` | Intersection with rounded blend |
| Subtraction | `subtract(a, b)` | Cut shape `b` from shape `a` |
| Smooth subtract | `smooth_subtract(a, b, k)` | Subtraction with rounded edge |

### Inline Usage via Pipes

```game
# Two spheres smoothly merged
fn: sphere(0.5) | translate(-0.3, 0, 0)
  | smooth_union(
      sphere(0.4) | translate(0.3, 0, 0),
      k: 0.2
    )
```

---

## Domain Operations

Transform the input position before evaluating the SDF. These are the tools of spatial choreography.

| Operation | Syntax | Effect |
|-----------|--------|--------|
| Translate | `translate(x, y, z)` | Move in space |
| Rotate | `rotate(angle_x, angle_y, angle_z)` | Euler rotation (radians) |
| Rotate axis | `rotate_axis(axis, angle)` | Rotate around arbitrary axis |
| Scale | `scale(s)` or `scale(x, y, z)` | Uniform or non-uniform scale |
| Repeat | `repeat(spacing)` or `repeat(x, y, z)` | Infinite repetition |
| Repeat limited | `repeat_n(spacing, count)` | Finite repetition |
| Mirror | `mirror(axis)` | Mirror across axis (`x`, `y`, `z`, `xy`, etc.) |
| Twist | `twist(amount)` | Twist around Y axis |
| Bend | `bend(amount)` | Bend around Y axis |
| Elongate | `elongate(x, y, z)` | Stretch the SDF |
| Displace | `displace(noise_fn)` | Add noise to the surface |
| Round | `round(r)` | Round edges by `r` |
| Shell | `shell(thickness)` | Hollow out with wall thickness |
| Onion | `onion(thickness)` | Concentric shell layers |
| Symmetry | `symmetry(axes)` | Force symmetry across axes |

### WGSL Example: repeat(2.0) compiles to

```wgsl
fn op_repeat(p: vec3f, spacing: f32) -> vec3f {
    return p - spacing * round(p / spacing);
}
```

### Composition

```game
# A twisted lattice of spheres
fn: sphere(0.2)
  | repeat(1.5)
  | twist(time * 0.3)
  | round(0.05)
```

---

## Noise Functions

Procedural noise — the soul of generative art. All noise functions accept a position and return a float in [-1, 1] (or [0, 1] for some).

| Function | Syntax | Character |
|----------|--------|-----------|
| Simplex | `simplex(p)` | Smooth, organic, gradient-based |
| Perlin | `perlin(p)` | Classic, slightly gridded |
| Value | `value_noise(p)` | Simple interpolated random |
| Worley | `worley(p, jitter)` | Cellular, crystal-like |
| Voronoi | `voronoi(p, jitter)` | Cell boundaries (returns distance to nearest edge) |
| FBM | `fbm(p, octaves, lacunarity, persistence)` | Fractal Brownian Motion — layered noise |
| Turbulence | `turbulence(p, octaves)` | Like FBM but absolute value (sharper features) |
| Ridged | `ridged(p, octaves)` | Inverted turbulence (mountain ridges) |
| Curl | `curl_noise(p, frequency)` | Divergence-free 3D noise (fluid-like motion) |
| Domain warp | `warp(p, noise_fn, strength)` | Feed noise back into position (organic distortion) |

### FBM Detail

```game
# Terrain with 8 octaves of detail
fn: fbm(p * 2.0, octaves: 8, lacunarity: 2.1, persistence: 0.5)
```

Compiles to a loop:

```wgsl
fn fbm(p: vec3f, octaves: i32, lacunarity: f32, persistence: f32) -> f32 {
    var value: f32 = 0.0;
    var amplitude: f32 = 1.0;
    var frequency: f32 = 1.0;
    var max_value: f32 = 0.0;

    for (var i: i32 = 0; i < octaves; i++) {
        value += simplex(p * frequency) * amplitude;
        max_value += amplitude;
        amplitude *= persistence;
        frequency *= lacunarity;
    }

    return value / max_value;
}
```

---

## Shading

Transform SDF distance/position into color.

| Function | Syntax | Effect |
|----------|--------|--------|
| Shade (PBR) | `shade(albedo, roughness, metallic)` | Physically-based rendering |
| Emissive | `emissive(color, intensity)` | Self-illuminating glow |
| Fresnel | `fresnel(color, power)` | Edge glow effect |
| Iridescent | `iridescent(strength)` | Angle-dependent color shift |
| Toon | `toon(colors, steps)` | Cel-shaded look |
| Matcap | `matcap(texture_fn)` | Material capture using a function instead of a texture |
| Colormap | `colormap(palette)` | Map scalar value to color gradient |

### Built-in Color Palettes

```
midnight, obsidian, charcoal, ash, smoke,    # Darks
ivory, frost, pearl, bone, cloud,            # Lights
ember, flame, fire, lava, magma,             # Warms
ocean, deep_blue, cyan, ice, arctic,         # Cools
gold, amber, honey, brass, copper,           # Metals
emerald, jade, moss, forest, lime,           # Greens
crimson, scarlet, blood, rose, coral,        # Reds
violet, amethyst, indigo, lavender, plum,    # Purples

# Scientific colormaps (continuous)
viridis, magma, inferno, plasma, turbo, cividis,
hot, cool, rainbow, grayscale
```

---

## Post-Processing

Applied after rendering, per-lens or globally.

| Effect | Syntax | Parameters |
|--------|--------|------------|
| Bloom | `bloom(intensity, threshold?)` | Glow on bright areas |
| Chromatic | `chromatic(strength)` | RGB channel separation |
| Vignette | `vignette(strength)` | Darkened edges |
| Grain | `grain(intensity)` | Film grain noise |
| Fog | `fog(density, color)` | Distance-based atmospheric fog |
| Distort | `distort(noise_fn, strength)` | Screen-space distortion |
| Glitch | `glitch(intensity, speed)` | Digital artifact effect |
| Scanlines | `scanlines(count, intensity)` | CRT monitor effect |
| Sharpen | `sharpen(strength)` | Edge enhancement |
| Blur | `blur(radius)` | Gaussian blur |
| Depth of field | `dof(focus_dist, aperture)` | Focus effect for raymarched scenes |
| Color grade | `grade(lift, gamma, gain)` | Film-style color correction |
| Tonemap | `tonemap(method)` | HDR to SDR: `aces`, `reinhard`, `filmic` |

### Chaining

Post-effects chain in order:

```game
post: [
  fog(0.02, midnight),
  bloom(1.2, threshold: 0.7),
  chromatic(0.001),
  grade(lift: [0.02, 0.01, 0.05]),
  tonemap(aces),
  vignette(0.3),
  grain(0.02)
]
```

---

## Camera

For `raymarch` and `volume` lens modes.

| Camera | Syntax | Behavior |
|--------|--------|----------|
| Orbit | `orbit(radius, height, speed)` | Circles around origin |
| Static | `static(position, target)` | Fixed position and look-at |
| Dolly | `dolly(from, to, ease)` | Linear movement |
| Crane | `crane(height_from, height_to, radius)` | Vertical arc movement |
| Handheld | `handheld(position, shake)` | Subtle noise-based shake |
| Track | `track(path, speed)` | Follow a defined path |
| First person | `fps(position, look_dir)` | Mouse-controlled look |

### Camera in Arcs

```game
arc {
  0:00 "wide" {
    camera: orbit(radius: 8.0, height: 4.0)
  }

  0:30 "approach" {
    camera -> orbit(radius: 2.0, height: 0.5) ease(smooth) over 10s
  }

  1:00 "intimate" {
    camera -> static([0.5, 0.3, 0.5], target: [0, 0, 0]) ease(cubic_in_out)
  }
}
```

---

## Math Built-ins

Available everywhere in expressions.

### Functions
`sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`,
`sqrt`, `pow`, `exp`, `log`, `log2`,
`abs`, `sign`, `floor`, `ceil`, `round`, `fract`,
`min`, `max`, `clamp`, `saturate`,
`mix` (lerp), `smoothstep`, `step`,
`length`, `distance`, `dot`, `cross`, `normalize`,
`mod`, `fmod`

### Constants
`pi`, `tau` (2*pi), `e`, `phi` (golden ratio)

### Helpers
`remap(value, in_low, in_high, out_low, out_high)` — linear remap
`pulse(time, frequency, width)` — periodic pulse wave
`ease_in(t)`, `ease_out(t)`, `ease_in_out(t)` — easing as functions
`hash(p)` — deterministic pseudo-random from position
`rotate2d(angle)` — 2D rotation matrix

---

## Special Values

Available in expressions based on context:

| Value | Type | Available In | Meaning |
|-------|------|-------------|---------|
| `p` | vec3 | `fn:` chains | Current sample position |
| `time` | float | Everywhere | Elapsed time (safe-wrapped) |
| `height` | float | After SDF eval | Normalized height of surface point |
| `normal` | vec3 | In `shade` | Surface normal at current point |
| `uv` | vec2 | `flat` mode | Screen UV coordinates (0..1) |
| `depth` | float | After raymarch | Ray travel distance |
| `hit` | bool | After raymarch | Whether ray hit a surface |
