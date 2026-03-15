# GAME Language Reference

**Version 0.7.0** | Generative Animation Matrix Engine

GAME compiles .game files to WebGPU/WebGL2 shaders packaged as zero-dependency Web Components. Every compiled component renders at 60fps on the GPU with automatic ACES tonemapping and dithering.

## Syntax Overview

```
cinematic "name" {
  layer config { param: default_value }
  layer name [memory: 0.0-1.0] [opacity: 0.0-1.0] [blend: add|screen|multiply|overlay] {
    stage1(args) | stage2(args) | ... | bridge(args) | color_op(args)
  }
  arc { param: start -> end over Ns easing }
  resonate { param -> layer.property * weight }
  matrix color { [r_r, r_g, r_b, g_r, g_g, g_b, b_r, b_g, b_b] }
  pass name { effect(args) }
}
```

## Pipeline State Machine

Stages chain with `|`. Three states: **Position -> Sdf -> Color**. Every layer MUST end in Color.

## Builtins (45)

### Position -> Position (transforms)
translate(x,y), rotate(speed), scale(s), warp(scale,oct,pers,lac,strength), distort(scale,speed,strength), polar, repeat(sx,sy), mirror, radial(count)

### Position -> Sdf (generators)
circle(r), ring(r,w), star(pts,r,inner), box(w,h), hex(r), triangle(size), line(x1,y1,x2,y2,w), capsule(len,r), arc_sdf(r,angle,w), cross(size,arm_w), heart(size), egg(r,k), spiral(turns,w), grid(spacing,w), fbm(scale,oct,pers,lac), simplex(scale), voronoi(scale), radial_fade(inner,outer)

### Sdf boolean ops
union(a,b), subtract(a,b), intersect(a,b), smooth_union(a,b,k), smooth_subtract(a,b,k), smooth_intersect(a,b,k), xor(a,b), morph(a,b,t)

### Sdf -> Sdf (modifiers)
round(r), shell(w), onion(count,w), mask_arc(angle)

### Sdf -> Color (bridges)
glow(intensity), shade(r,g,b), emissive(intensity), palette(name_or_coefficients)

### Color -> Color
tint(r,g,b), bloom(threshold,strength), grain(amount), outline(width)

## 30 Named Palettes
fire, ocean, neon, aurora, sunset, ice, ember, lava, magma, inferno, plasma, electric, cyber, matrix, forest, moss, earth, desert, blood, rose, candy, royal, deep_sea, coral, arctic, twilight, vapor, gold, silver, monochrome

## Post-Processing Passes (8 effects)
blur(radius), vignette(strength), chromatic(offset), sharpen(strength), film_grain(amount), bloom(threshold,strength), grain(amount), tint(r,g,b)

## Built-in Inputs

### Interaction (mouse/touch)
`mouse_x`, `mouse_y` (0.0-1.0 cursor position), `mouse_down` (0 or 1)

Use in any expression: `circle(0.1 + mouse_down * 0.2)`, `translate(mouse_x * 2.0 - 1.0, mouse_y * 2.0 - 1.0)`

### Audio
`bass`, `mid`, `treble` (frequency bands), `energy` (RMS), `beat` (detection signal)

## Expressions in Arguments

Stage arguments support full arithmetic expressions:

```
circle(0.1 + pulse * 0.15)
glow(2.0 + sin(pulse * 6.28) * 0.5)
translate(mouse_x * 2.0 - 1.0, mouse_y * 2.0 - 1.0)
warp(scale: 2.0, octaves: 4, strength: 0.1 + energy * 0.2)
```

**Math functions**: sin, cos, abs, min, max, pow, floor, ceil, fract, clamp, mix, step, smoothstep, length, dot, atan2

**Operators**: `+` `-` `*` `/` `^` (power)

## Key Features
- **Memory**: `layer x memory: 0.93` -- ping-pong framebuffer persistence
- **Arc**: `arc { param: 0 -> 1 over 5s ease-in-out }` -- timeline animations
- **Resonate**: `resonate { param -> layer.scale * 0.3 }` -- cross-layer coupling
- **Matrix**: `matrix color { [9 floats] }` -- 3x3 RGB color grading
- **Config**: `layer config { name: value }` -- declares uniform parameters
- **Interaction**: `mouse_x`, `mouse_y`, `mouse_down` -- live cursor + click/touch

## WASM API
```javascript
compileGame(source, target) // -> JSON [{name, js, wgsl, glsl, html, dts, uniforms}]
validateGame(source)        // -> "ok" or error string
getBuiltins()               // -> JSON [{name, signature, input, output}]
getPaletteNames()           // -> JSON ["fire", "ocean", ...]
```

See `prompts/generate-visual.md` for the complete AI generation system prompt.
