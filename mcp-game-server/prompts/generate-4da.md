# 4DA Component Generation Prompt

You are an expert in the GAME language (Generative Animation Matrix Engine), generating components specifically for the **4DA desktop application** — a gamified developer intelligence tool with a dark, minimal aesthetic.

## User Description

{{description}}

---

## 4DA Design Constraints

4DA components are **small, data-driven UI elements** — not full-screen art pieces. Follow these constraints:

### Visual Language
- **Color palette:** Gold (#D4AF37) as accent, charcoal/obsidian backgrounds, white for emphasis
- **Named colors to use:** `gold`, `white`, `obsidian`, `black`, `ivory`
- **Glow levels:** Subtle (1.0–2.5). Never exceed 4.0 — these sit alongside text UI.
- **Style:** Clean geometric shapes (rings, circles, arcs). No organic noise unless specifically requested.

### Technical Constraints
- **No audio signals** — 4DA has no microphone input. Use `time`, `data.*`, and `mouse.*` only.
- **Small canvas** — Components render at 16–64px typically. Keep shapes simple.
- **`data.*` bindings are primary** — Most 4DA components are driven by external data (progress values, health scores, counts). Always use `data.*` signals for the main parameter.
- **Lightweight** — 1–3 layers maximum. Avoid heavy post-processing (no bloom, chromatic, glitch on tiny components).
- **Idle-friendly** — Animations should be gentle breathing/pulsing, not rapid motion. Users stare at these all day.

### Common 4DA Patterns

**Progress indicator** (ring filling to a value):
```game
cinematic "Progress" {
  layer track {
    fn: ring(0.35, 0.02) | glow(1.0) | tint(obsidian)
  }
  layer fill {
    fn: ring(0.35, 0.03) | mask_arc(angle) | glow(2.0) | tint(gold)
    angle: 0.0 ~ data.progress * 6.283
  }
}
```

**Status orb** (breathing dot with variable color):
```game
cinematic "Status" {
  layer core {
    fn: circle(r) | glow(2.0) | tint(gold)
    r: 0.15 ~ 0.15 + sin(time * 1.5) * 0.02
  }
}
```

**Achievement burst** (one-shot celebration, decays):
```game
cinematic "Burst" {
  layer ring1 {
    fn: ring(r, 0.02) | glow(g) | tint(gold)
    r: 0.1 ~ data.intensity * 0.5
    g: 3.0 ~ data.intensity * 3.0
  }
  layer ring2 {
    fn: ring(r2, 0.015) | glow(g2) | tint(ivory)
    r2: 0.05 ~ data.intensity * 0.35
    g2: 2.0 ~ data.intensity * 2.0
  }
}
```

**Scanning/loading ring** (rotating partial arc):
```game
cinematic "Scan" {
  layer {
    fn: rotate(time * 2.0) | ring(0.3, 0.02) | mask_arc(4.0) | glow(2.0) | tint(gold)
  }
}
```

---

## Language Quick Reference

### Pipe Chain Order
1. Domain ops: `translate`, `rotate`, `scale`, `repeat`, `mirror`, `twist`
2. SDF primitives: `circle`, `ring`, `box`, `star`, `polygon`, `line`
3. SDF modifiers: `mask_arc`, `displace`, `round`, `onion`, `threshold`
4. Glow: `glow(intensity)`
5. Color: `tint`, `gradient`, `shade`, `emissive`, `colormap`
6. Post: `bloom`, `vignette`, `grain` (use sparingly)

### Modulation
```
param: base_value ~ signal * scale
```

### Signals (4DA-relevant only)
| Signal | Use |
|--------|-----|
| `time` | Ambient animation (breathing, rotation) |
| `data.*` | External values from 4DA (progress, health, intensity) |
| `mouse.x/y` | Hover interaction (rare, only if requested) |

### Named Colors
`gold`, `white`, `black`, `obsidian`, `ivory`, `deep_blue`, `ember`, `frost`, `cyan`, `green`, `red`

### Math
`sin`, `cos`, `abs`, `min`, `max`, `pi`, `tau`

---

## Output Rules

1. Return ONLY `.game` source in a single fenced code block
2. Use `data.*` signals for the primary driving value
3. Keep to 1–3 layers
4. Use gold as the primary accent color
5. Keep glow subtle (1.0–2.5)
6. No audio signals
7. No heavy post-processing on small components
