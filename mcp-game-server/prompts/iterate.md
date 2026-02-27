# GAME Iteration Prompt

You are an expert in the GAME language (Generative Animation Matrix Engine). Your task is to modify existing `.game` source code based on user feedback.

## Current Source

```game
{{source}}
```

## Requested Changes

{{feedback}}

---

## Instructions

1. **Preserve working parts** -- only change what the feedback specifically requests
2. **Maintain structure** -- keep the cinematic block, layer names, and organization unless restructuring is explicitly requested
3. **Validate stage ordering** -- pipe chains must follow: domain ops -> SDF -> modifiers -> glow -> shading -> post-processing
4. **Use valid primitives** -- only use primitives listed in the reference below
5. **Test mentally** -- consider whether the modified code would compile and produce the intended visual result

## Common Refinement Patterns

### Adding glow
Append `| glow(intensity)` after an SDF primitive or modifier:
```game
fn: circle(0.3) | glow(2.0)
```

### Changing color
Add or modify a `tint()` or `shade()` stage:
```game
fn: circle(0.3) | glow(2.0) | tint(cyan)
fn: fbm(p * 3.0) | shade(albedo: gold, emissive: ember)
```

### Adding animation
Use `time` in expressions or add modulation:
```game
fn: rotate(time * 0.5) | circle(0.3) | glow(2.0)
radius: 0.3 ~ audio.bass * 0.2
```

### Adding post-processing
Append effects at the end of a pipe chain:
```game
fn: circle(0.3) | glow(2.0) | tint(gold) | bloom(0.5, 1.2) | vignette(0.3) | grain(0.01)
```

### Making it audio-reactive
Add `~` modulation to parameters:
```game
layer pulse {
  fn: circle(radius) | glow(intensity)
  radius: 0.3 ~ audio.bass * 0.2
  intensity: 2.0 ~ audio.energy * 3.0
}
```

### Adding layers
Create additional layer blocks for composite effects:
```game
layer bg   { fn: gradient(deep_blue, black, "radial") }
layer main { fn: circle(0.2) | glow(3.0) | tint(gold) }
```

### Adding a timeline
Use an `arc` block with named moments:
```game
arc {
  0:00 "start" { radius: 0.1 }
  0:05 "expand" { radius -> 0.5 ease(expo_out) over 3s }
}
```

### Adding interaction
Use a `react` block:
```game
react {
  mouse.click -> particles.burst(at: mouse.world, count: 200)
  key("space") -> arc.pause_toggle
}
```

### Adding cross-layer feedback
Use a `resonate` block:
```game
resonate {
  fire.freq ~ ice.clarity * 2.0
  ice.density ~ fire.intensity * -1.5
  damping: 0.96
}
```

### Adding iridescence
Append `| iridescent(strength)` after a shading stage:
```game
fn: voronoi(3.0) | glow(2.0) | tint(frost) | iridescent(0.3)
```

### Adding particles
Use the `particles()` function in a shading context:
```game
fn: curl_noise(p, frequency: 2.0, amplitude: 1.0) | particles(count: 5000, size: 1.5, color: gold, trail: 0.3)
```

---

## Quick Reference: Available Primitives

**SDF:** circle, sphere, ring, box, torus, line, polygon, star
**Domain:** translate, rotate, scale, repeat, mirror, twist
**Modifiers:** mask_arc, displace, round, onion, threshold
**Noise:** fbm, simplex, voronoi, curl_noise, concentric_waves
**Glow:** glow
**Color:** shade, emissive, colormap, spectrum, tint, gradient, particles
**Post:** bloom, chromatic, vignette, grain, fog, glitch, scanlines, tonemap, invert, saturate_color, iridescent
**Colors:** black, white, red, green, blue, cyan, orange, gold, ember, frost, ivory, midnight, obsidian, deep_blue
**Easing:** linear, smooth, expo_in, expo_out, cubic_in_out, elastic, bounce
**Signals:** audio.bass, audio.mid, audio.treble, audio.energy, audio.beat, mouse.x, mouse.y, mouse.click, time, data.*

---

## Output Rules

1. Return ONLY the modified `.game` source code in a single fenced code block
2. No explanation before or after the code block
3. Include comments only if the original source had comments
4. Preserve the cinematic title if one existed
