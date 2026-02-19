# The `.game` Language Specification

**Version:** 0.1-draft
**Status:** Design phase

---

## Design Principles

1. **Reads like intent, not implementation** — describe what you want to see, not how to render it
2. **Progressive disclosure** — "hello world" is 3 lines. A 5-minute cinematic is 100 lines. Complexity is opt-in.
3. **Mathematical native** — math expressions are first-class, not strings in quotes
4. **Composable** — small pieces combine into larger ones via pipes, imports, and nesting
5. **Temporal native** — time, rhythm, and duration are built into the syntax
6. **Signal-driven** — any parameter can react to any signal (audio, input, other parameters)

---

## 1. The Simplest Cinematic

```game
cinematic {
  layer {
    fn: circle(0.3) | glow(2.0)
  }
}
```

This renders a glowing circle. Nothing else. Three lines.

The `fn` property defines a **generative function** — a chain of operations piped together. `circle(0.3)` produces an SDF circle with radius 0.3. `glow(2.0)` adds a bloom post-effect with intensity 2.0.

---

## 2. Core Concepts

### 2.1 Fields

A **field** is a continuous mathematical function that maps a position to a value. Fields are the atoms of GAME. Everything visual is a field or a composition of fields.

```game
layer terrain {
  fn: fbm(p * 2.0, octaves: 6, lacunarity: 2.1)
}
```

Fields are not objects. They have no edges, no polygon count, no bounding box. They exist everywhere in space simultaneously. The renderer samples them.

### 2.2 The Pipe Operator `|`

Pipes chain transformations left-to-right. Each step transforms the output of the previous step.

```game
fn: sphere(0.5)
  | twist(time * 0.5)
  | repeat(spacing: 2.0)
  | shade(albedo: gold, roughness: 0.3)
  | bloom(1.5)
```

Read: "A sphere, twisted over time, repeated in space, shaded gold, with bloom."

Pipe stages fall into categories:
- **Primitives** — `sphere`, `box`, `torus`, `plane`, `cylinder`
- **Domain ops** — `translate`, `rotate`, `scale`, `twist`, `bend`, `repeat`, `mirror`, `elongate`
- **Boolean ops** — `union`, `intersect`, `subtract`, `smooth_union(k)`
- **Noise** — `fbm`, `voronoi`, `simplex`, `curl`, `worley`
- **Shading** — `shade`, `emissive`, `fresnel`, `iridescent`
- **Post-processing** — `bloom`, `chromatic`, `vignette`, `grain`, `distort`

### 2.3 The Modulation Operator `~`

The `~` operator binds a parameter to a signal. The parameter's base value is modulated by the signal in real-time.

```game
layer pulse {
  fn: sphere(radius) | glow(intensity)
  radius: 0.3 ~ audio.bass          # radius reacts to bass frequency
  intensity: 2.0 ~ audio.energy     # glow reacts to audio energy
}
```

Modulation syntax:
```
param: base_value ~ signal                          # additive modulation
param: base_value ~ signal * scale                  # scaled modulation
param: base_value ~ signal * scale + offset         # scaled + offset
param: from ~ signal -> to                          # interpolation (signal 0..1 maps from..to)
param: base_value ~ signal_a * 0.5 + signal_b * 0.5 # multi-signal blend
```

Available signals:
- `audio.bass`, `audio.mid`, `audio.treble`, `audio.energy`, `audio.beat` — FFT bands
- `audio.waveform` — raw waveform data
- `audio.time` — playback position (0.0 to 1.0 over duration)
- `mouse.x`, `mouse.y` — normalized cursor position (0.0 to 1.0)
- `mouse.velocity` — cursor movement speed
- `mouse.click` — impulse on click (decays over ~200ms)
- `mouse.world` — cursor position in world space (for raymarched scenes)
- `key("space")` — key held state (0 or 1)
- `time` — elapsed time in seconds
- `beat` — pulse synchronized to detected BPM
- `random` — per-frame random value
- `layer_name.param_name` — another layer's parameter (cross-modulation)

### 2.4 Lenses

A **lens** is a rendering mode — how fields are observed and turned into pixels. Different lenses render the same fields differently.

```game
cinematic {
  layer terrain {
    fn: fbm(p * scale, octaves: 6)
    scale: 2.0
  }

  # Lens 1: render as 3D raymarched landscape
  lens main {
    mode: raymarch
    fields: [terrain]
    camera: orbit(radius: 5.0, height: 2.0, speed: 0.1)
    lighting: sun(direction: [0.8, 0.6, 1.0], intensity: 0.9) + ambient(0.1)
    post: [fog(density: 0.02, color: midnight), bloom(1.0)]
  }

  # Lens 2: render as 2D heightmap overlay
  lens minimap {
    mode: flat
    fields: [terrain]
    region: bottom_right(size: 0.2)
    colormap: viridis
    opacity: 0.6
  }
}
```

Lens modes:
- `raymarch` — SDF raymarching (3D generative geometry)
- `volume` — volumetric rendering (clouds, fog, nebulae)
- `flat` — 2D fragment shader (classic Shadertoy-style)
- `particles` — GPU particle system driven by fields
- `trace` — path tracing (offline/high-quality mode)

### 2.5 Arcs

An **arc** defines the temporal structure — how parameters evolve over the cinematic's duration. Arcs use named moments, not mechanical timestamps.

```game
arc {
  0:00 "void" {
    terrain.scale: 0.1
    main.exposure: 0.0
  }

  0:10 "awakening" {
    terrain.scale -> 2.0 ease(expo_out) over 8s
    main.exposure -> 1.0 ease(linear) over 3s
  }

  0:45 "growth" {
    crystals.density -> 5.0 ease(cubic_in_out)
    particles.opacity -> 1.0
  }

  1:20 "crescendo" {
    ALL.intensity * 2.0 ease(expo_in) over 5s
    camera -> closeup ease(smooth)
  }

  1:45 "dissolution" {
    ALL.opacity -> 0.0 ease(expo_out) over 10s
  }
}
```

Arc properties:
- Moments are timestamps + names. Names are for readability and can be jumped to via interaction.
- `->` transitions a parameter from its current value to the target
- `ease(fn)` specifies easing: `linear`, `expo_in`, `expo_out`, `cubic_in_out`, `smooth`, `elastic`, `bounce`
- `over Ns` specifies transition duration (default: until next moment)
- `ALL` references all layers simultaneously
- Between moments, modulation (`~`) continues to operate on top of arc values

### 2.6 Interaction

The `react` block defines how user input affects the cinematic beyond parameter modulation.

```game
react {
  mouse.click -> particles.burst(
    at: mouse.world,
    count: 200,
    color: white,
    decay: 2.0
  )

  key("space") -> arc.pause_toggle
  key("r") -> arc.restart

  mouse.drag -> camera.orbit_manual
  scroll -> camera.zoom(speed: 0.1)

  mic.amplitude > 0.5 -> terrain.crack(
    at: random_surface_point,
    intensity: mic.amplitude
  )
}
```

---

## 3. Composition and Reuse

### 3.1 Named Functions

Define reusable generative functions:

```game
define organic_sphere(r, roughness) {
  sphere(r)
    | displace(simplex(p * 3.0) * roughness)
    | shade(albedo: ivory, roughness: 0.4, subsurface: 0.2)
}

layer planet {
  fn: organic_sphere(1.0, 0.15)
    | rotate(0, time * 0.1, 0)
}
```

### 3.2 Imports

Import functions, layers, or entire cinematics from other `.game` files:

```game
import "stdlib/nebula.game" expose nebula_field, cosmic_dust
import "stdlib/transitions.game" expose *

layer bg {
  fn: nebula_field(density: 3.0, color_palette: fire)
}

layer dust {
  fn: cosmic_dust(count: 10000) ~ audio.energy
}
```

### 3.3 The Standard Library

GAME ships with a standard library of generative primitives:

```
stdlib/
  primitives.game    # SDF primitives, boolean ops, domain ops
  noise.game         # fbm, voronoi, curl, simplex, worley, value
  shading.game       # PBR, toon, emissive, volumetric, iridescent
  camera.game        # orbit, dolly, crane, handheld, track
  transitions.game   # fade, morph, shatter, dissolve, ripple, bloom_wipe
  particles.game     # burst, stream, swarm, fireflies, rain, sparks
  post.game          # bloom, chromatic, vignette, grain, scanlines, glitch
  colormaps.game     # viridis, magma, inferno, plasma, turbo, custom
  audio.game         # beat_detect, onset, pitch_track, spectral_flux
```

---

## 4. Resonance — Cross-Layer Feedback

The most revolutionary feature: layers can modulate each other, creating visual feedback loops.

```game
layer fire {
  fn: fbm(p * freq, octaves: 4) | emissive(color: orange)
  freq: 3.0
  brightness: 0.8
}

layer ice {
  fn: voronoi(p * density) | shade(albedo: cyan, metallic: 0.9)
  density: 2.0
  brightness: 0.8
}

# Resonance: fire and ice react to each other
resonate {
  fire.freq ~ ice.brightness * 2.0       # brighter ice = more turbulent fire
  ice.density ~ fire.brightness * -1.0   # brighter fire = sparser ice
}
```

When fire gets brighter, ice retreats. When ice gets denser, fire becomes more turbulent. The interaction evolves organically — never the same twice. This is **emergent visual behavior** from simple rules.

Resonance chains can create complex emergent systems:

```game
resonate {
  a.intensity ~ b.brightness * 0.3
  b.intensity ~ c.brightness * 0.3
  c.intensity ~ a.brightness * 0.3    # circular feedback
  damping: 0.95                       # prevents runaway feedback
}
```

---

## 5. Complete Example

A full cinematic in `.game` format:

```game
# "Deep Signal" — a 2-minute generative cinematic
cinematic "Deep Signal" {
  resolution: 1920x1080
  audio: "deep_signal.ogg"

  # --- Layers ---

  layer void {
    fn: simplex(p * 0.5 + time * 0.02) * 0.5 + 0.5
      | colormap(midnight_to_indigo)
    depth: base
  }

  layer terrain {
    fn: fbm(p * scale, octaves: 8, persistence: persistence)
      | shade(
          albedo: mix(obsidian, gold, height),
          roughness: 0.6,
          emissive: height > 0.7 ? gold * 0.3 : black
        )
    scale: 2.0 ~ audio.bass * 1.5
    persistence: 0.5 ~ audio.energy * 0.3
    depth: heart
  }

  layer crystals {
    fn: voronoi(p * density, seed: 42)
      | threshold(0.05)
      | extrude(height: audio.mid * 0.5)
      | shade(albedo: white, metallic: 1.0, roughness: 0.05)
      | iridescent(strength: 0.4)
    density: 4.0
    depth: heart
  }

  layer sparks {
    fn: curl_noise(p, frequency: freq)
      | particles(
          count: count,
          size: 1.5,
          color: gold,
          trail: 0.3
        )
    freq: 2.0 ~ audio.treble
    count: 5000 ~ audio.energy * 3000
    depth: top
  }

  # --- Resonance ---

  resonate {
    terrain.scale ~ crystals.density * 0.1
    crystals.density ~ sparks.freq * 0.2
    damping: 0.98
  }

  # --- Lens ---

  lens main {
    mode: raymarch
    fields: [void, terrain, crystals]
    overlay: [sparks]
    camera: orbit(radius: 4.0, height: 1.5, speed: 0.05)
    lighting: sun([0.5, 0.8, 1.0], 0.8) + ambient(0.15) + emissive_pass
    post: [
      fog(density: 0.03, color: deep_blue),
      bloom(intensity: 1.2, threshold: 0.7),
      chromatic(strength: 0.001),
      vignette(0.3),
      grain(0.02)
    ]
  }

  # --- Arc ---

  arc {
    0:00 "silence" {
      void.opacity: 0.3
      terrain.scale: 0.5
      crystals.density: 0.0
      sparks.count: 0
      main.exposure: 0.1
    }

    0:08 "first light" {
      main.exposure -> 0.8 ease(expo_out) over 6s
      void.opacity -> 1.0 ease(linear)
    }

    0:20 "terrain rises" {
      terrain.scale -> 2.0 ease(cubic_out) over 15s
    }

    0:45 "crystallization" {
      crystals.density -> 4.0 ease(elastic) over 10s
    }

    1:00 "ignition" {
      sparks.count -> 5000 ease(expo_in) over 5s
      main.camera -> closeup(height: 0.3, radius: 1.5) ease(smooth) over 8s
    }

    1:30 "crescendo" {
      ALL.intensity * 2.5 ease(expo_in) over 10s
      terrain.scale -> 8.0 ease(expo_in)
    }

    1:50 "dissolution" {
      ALL.opacity -> 0.0 ease(expo_out) over 15s
      main.camera -> pullback(radius: 20.0) ease(linear)
    }
  }

  # --- Interaction ---

  react {
    mouse.click -> sparks.burst(at: mouse.world, count: 500, color: white)
    mouse.move -> camera.offset(mouse.x * 0.1, mouse.y * 0.05)
    key("f") -> fullscreen.toggle
  }
}
```

---

## 6. Grammar (Formal)

```ebnf
program         = cinematic_decl | define_decl* cinematic_decl
cinematic_decl  = "cinematic" string? "{" cinematic_body "}"
cinematic_body  = (property | layer_decl | lens_decl | arc_decl | react_decl | resonate_decl | import_decl)*

property        = ident ":" expression
layer_decl      = "layer" ident? "{" layer_body "}"
layer_body      = (fn_decl | param_decl | property)*
fn_decl         = "fn:" pipe_expr
param_decl      = ident ":" expression modulation?
modulation      = "~" mod_expr

pipe_expr       = call_expr ("|" call_expr)*
call_expr       = ident "(" arg_list? ")"
arg_list        = arg ("," arg)*
arg             = (ident ":")? expression

mod_expr        = signal_expr (("*" | "+" | "-") expression)*
                | signal_expr "->" expression
signal_expr     = ident ("." ident)*

lens_decl       = "lens" ident "{" lens_body "}"
lens_body       = (property | post_decl)*
post_decl       = "post:" "[" call_expr ("," call_expr)* "]"

arc_decl        = "arc" "{" moment* "}"
moment          = timestamp string? "{" transition* "}"
timestamp       = number ":" number
transition      = target "->" expression ease? duration?
                | target ":" expression
                | "ALL" "." ident op expression ease? duration?
target          = ident "." ident
ease            = "ease(" ident ")"
duration        = "over" number "s"

react_decl      = "react" "{" reaction* "}"
reaction        = signal_expr trigger? "->" action
trigger         = (">" | "<" | "==" ) expression
action          = ident "." call_expr | ident "." ident

resonate_decl   = "resonate" "{" resonance* "}"
resonance       = target "~" mod_expr

define_decl     = "define" ident "(" param_list? ")" "{" pipe_expr "}"
import_decl     = "import" string "expose" (ident ("," ident)* | "*")

expression      = number | string | ident | call_expr | vec_expr
                | expression op expression
                | expression "?" expression ":" expression
vec_expr        = "[" expression ("," expression)* "]"
op              = "+" | "-" | "*" | "/" | ">" | "<" | "==" | "!="

string          = '"' [^"]* '"'
number          = [0-9]+ ("." [0-9]+)?
ident           = [a-zA-Z_] [a-zA-Z0-9_]*
comment         = "#" [^\n]*
```

---

## 7. Compilation Model

```
.game source
    |
    v
 [Lexer] -----> Token stream
    |
    v
 [Parser] ----> AST (cinematic, layers, lenses, arcs, resonance)
    |
    v
 [Resolver] --> Validated AST
    |            - Resolve cross-references (layer.param in arcs/resonance)
    |            - Type-check modulation chains
    |            - Flatten pipe chains into shader operation sequences
    |            - Detect resonance cycles and insert damping
    |
    v
 [Compiler] --> Compilation artifacts:
    |            - WGSL shader source per lens (merged pipe chains)
    |            - Uniform buffer layouts (all params + modulations)
    |            - Audio analysis config (which FFT bands, beat detection params)
    |            - Arc timeline (interpolation keyframes)
    |            - Interaction bindings (input -> uniform mappings)
    |            - Resonance graph (feedback topology + damping coefficients)
    |
    v
 [Runtime] ---> Execution:
                 - Create WebGPU device, pipelines, bind groups
                 - Load + decode audio, start FFT analyzer
                 - Each frame (driven by audio clock):
                   1. Sample audio FFT -> band values
                   2. Read interaction state (mouse, keyboard, mic)
                   3. Evaluate resonance graph (topological order)
                   4. Interpolate arc parameters at current audio time
                   5. Compute final uniform values (base + arc + modulation + resonance)
                   6. Upload uniforms to GPU
                   7. Execute render pass per lens
                   8. Composite lens outputs
                   9. Apply global post-processing
                  10. Present to display
```

The key insight: **the pipe chain compiles to a single monolithic shader per lens.** The user writes modular, composable functions. The compiler merges them into optimal GPU code. Like how a C compiler inlines functions — you write for clarity, the machine executes for speed.

---

## 8. Reserved for Future

- `branch` — conditional arcs based on interaction state (interactive narrative)
- `loop` — repeating sections until interaction advances
- `export` — offline frame-perfect rendering to video
- `midi` — MIDI input as signals (for live performance / VJ use)
- `osc` — Open Sound Control for external hardware integration
- `wasm_fn` — custom WASM functions as field generators (escape hatch)
