# GAME — Generative Animation Matrix Engine

## The One-Line Pitch

A language and runtime for creating interactive generative cinematics — where mathematical fields, not pre-authored assets, produce cinematic experiences in real-time.

## The Problem

The creative coding world is split into two irreconcilable camps:

**Heavy engines** (Unreal, Unity, Godot) — 50GB installs, polygon-based rendering, steep learning curves, designed for games not cinema. You can make cinematics in them, but you're using a bulldozer to paint a portrait.

**Shader sandboxes** (Shadertoy, GLSL Sandbox, p5.js) — beautiful isolated effects that can never become a sequenced, interactive, audio-synced experience. You write a single shader that loops forever. No narrative arc, no transitions, no composition.

**Animation tools** (Theatre.js, Motion Canvas, After Effects) — keyframe-based, asset-dependent, manual. Every frame is hand-authored. Nothing is generated. Nothing surprises you.

**Between these three categories: nothing.**

No tool exists where you can write a text file that describes a generative cinematic — fields of mathematics that evolve over time, react to music, respond to interaction — and have it render in real-time in a browser.

GAME is that tool.

## The Core Innovation

GAME introduces a new paradigm for animation: **generative composition**.

| Paradigm | Model | Example Tools |
|----------|-------|---------------|
| Imperative timeline | "At t=2, move X to position Y" | After Effects, Theatre.js, Motion Canvas |
| Declarative state | "When condition, show this" | CSS transitions, React Spring |
| **Generative composition** | "These fields interact by these rules, observed through these lenses, synchronized to this signal" | **GAME** |

You don't tell GAME what to render. You describe mathematical fields, how they interact, how they're observed, and what external signals modulate them. The visuals **emerge**.

## Five Revolutionary Properties

### 1. Zero Assets
The entire cinematic is mathematics. No textures, no 3D models, no sprite sheets. A `.game` file + an audio file = a complete cinematic experience. The whole thing can fit in under 100KB. The demoscene promise, productized.

### 2. Cinematics as Code
A `.game` file is human-readable text. It can be:
- Git-versioned (collaborate on cinematics like code)
- Diffed (see exactly what changed between versions)
- AI-generated (describe a cinematic in English, get a `.game` file)
- Composed (import one cinematic into another)
- Hot-reloaded (edit while it runs, see changes instantly)

### 3. Emergent, Not Authored
No two viewings are identical. Because the visuals emerge from mathematical interactions rather than pre-authored keyframes, each rendering has organic variation. Like watching waves — governed by the same physics, never exactly repeated.

### 4. Audio Is Structural
Audio isn't layered on top — it's woven into the mathematics. Audio frequencies modulate visual parameters directly. The bass makes terrain rise. The treble scatters particles. The cinematic doesn't just play alongside music — it IS the music, rendered as light.

### 5. Interactive by Nature
Every parameter can be bound to an input signal. Mouse position shifts gravity. Microphone input adds turbulence. Touch creates ripples. The viewer isn't watching a film — they're playing an instrument whose output is cinema.

## Who Is This For?

**Primary:** Creative coders, generative artists, shader programmers, demoscene practitioners, creative technologists — people who think in math and render in pixels.

**Secondary:** VJs and live performers, music visualizer creators, interactive installation artists, portfolio builders, AI artists exploring visual generation.

**Not for:** Game developers (use Bevy/Godot), film editors (use DaVinci), motion graphics designers who need vector precision (use Motion Canvas).

## Technology

- **Language:** Custom DSL (`.game` files) — compiles to GPU shader programs
- **Rendering:** WebGPU (primary) with WebGL2 fallback. SDF raymarching, not polygon rasterization.
- **Audio:** Web Audio API with FFT analysis feeding shader uniforms
- **Runtime:** Rust + wgpu, compiled to WASM for browser distribution
- **Distribution:** Share a URL. The cinematic runs in the browser. No install.

## The Name

**G**enerative — visuals emerge from mathematics, not pre-authored assets
**A**nimation — temporal, evolving, alive
**M**atrix — the composition grid: layers x time. Every cinematic is a matrix of generative functions mapped across temporal arcs
**E**ngine — compiles, optimizes, and executes the generative composition at 60fps
