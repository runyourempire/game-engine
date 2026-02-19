# GAME Roadmap

## Milestone 0: "Proof of Concept" (The Compiler Exists)
**Goal:** Parse a `.game` file and produce a working WebGPU shader.
**Deliverable:** CLI tool that reads `001-hello.game` and renders a glowing circle in the browser.

### Tasks
- [ ] Lexer: tokenize `.game` syntax (Rust, `logos` crate)
- [ ] Parser: build AST for `cinematic`, `layer`, `fn`, pipe chains
- [ ] Codegen: compile a pipe chain into a WGSL fragment shader
- [ ] Runtime: minimal WebGPU setup — fullscreen quad, uniform buffer, render loop
- [ ] Glue: compile Rust to WASM, serve with a tiny HTML shell
- [ ] Validate: `001-hello.game` renders correctly

**Success:** A `.game` file produces pixels. The compiler works.

---

## Milestone 1: "It Breathes" (Audio + Modulation)
**Goal:** Audio drives the visuals. The `~` operator works.
**Deliverable:** `002-audio-reactive.game` plays synced to music.

### Tasks
- [ ] Audio: Web Audio API integration, FFT analysis, band extraction
- [ ] Modulation: implement `~` operator — parse, resolve, compile to uniform bindings
- [ ] Signals: `audio.bass`, `audio.mid`, `audio.treble`, `audio.energy`, `time`
- [ ] Uniforms: dynamic uniform buffer with all modulated parameters
- [ ] Audio clock: drive frame timing from `audioContext.currentTime`, not rAF
- [ ] Time safety: implement `safe_time()` wrapping for shader time values
- [ ] SDF library: compile `fbm`, `shade`, `mix` to WGSL builtins
- [ ] Lens: basic `raymarch` mode with hardcoded camera and lighting
- [ ] Validate: `002-audio-reactive.game` renders a music-reactive terrain

**Success:** Music and math produce synchronized cinema.

---

## Milestone 2: "It Responds" (Interaction + Arcs)
**Goal:** The viewer affects the experience. Time has structure.
**Deliverable:** `003-interactive.game` responds to mouse input.

### Tasks
- [ ] Input: mouse position, velocity, click events → uniform signals
- [ ] React block: parse and execute `react` declarations
- [ ] Impulse signals: `mouse.click` as decaying impulse (spike + exponential decay)
- [ ] Flat lens: 2D fragment shader mode (no raymarching)
- [ ] Arc system: parse arc moments, interpolate parameters over time
- [ ] Easing: implement easing functions (expo, cubic, smooth, elastic, bounce)
- [ ] Transition syntax: `param -> value ease(fn) over Ns`
- [ ] `ALL` keyword: apply arc transitions to all layers simultaneously
- [ ] Post-processing: implement `bloom`, `chromatic`, `vignette` as composable passes
- [ ] Validate: `003-interactive.game` responds to mouse, `004-resonance.game` evolves over time

**Success:** Interactive generative cinema with temporal arc. The core product works.

---

## Milestone 3: "It Lives" (Resonance + Polish)
**Goal:** Layers influence each other. Emergent behavior. The full language works.
**Deliverable:** `004-resonance.game` produces emergent visual dynamics.

### Tasks
- [ ] Resonance: parse `resonate` block, build dependency graph
- [ ] Cycle detection: identify feedback loops, enforce damping
- [ ] Topological evaluation: evaluate resonance in correct order each frame
- [ ] Cross-layer modulation: layer A's output affects layer B's parameters
- [ ] Particle lens: GPU particle system driven by curl noise fields
- [ ] Camera system: `orbit`, `dolly`, `closeup`, `pullback` with smooth transitions
- [ ] Lighting: `sun`, `ambient`, `emissive_pass` composition
- [ ] Multiple lenses: render multiple lenses, composite with blend modes
- [ ] `define` blocks: user-defined reusable generative functions
- [ ] Adaptive quality: detect framerate drops, reduce resolution/complexity
- [ ] Validate: `004-resonance.game` produces emergent fire/ice dynamics

**Success:** The full language works. Emergent cinematics exist.

---

## Milestone 4: "It Spreads" (Distribution + Community)
**Goal:** Anyone can create, share, and experience GAME cinematics.

### Tasks
- [ ] `game build` CLI: compile `.game` file into a self-contained web bundle
- [ ] Hosting: static file hosting (any CDN) serves playable cinematics
- [ ] `game dev` CLI: hot-reload mode — edit `.game` files, see changes live
- [ ] `game export` CLI: offline frame-perfect rendering to video (mp4/webm)
- [ ] Standard library: ship `stdlib/` with primitives, noise, shading, transitions
- [ ] `import` system: load and compose `.game` files from other `.game` files
- [ ] Documentation site: language reference, tutorials, example gallery
- [ ] Gallery: curated collection of community `.game` cinematics, playable in browser
- [ ] npm package: `npx game init` scaffolds a new cinematic project
- [ ] Shader import: ability to reference Shadertoy shaders by ID/URL (adapter layer)

**Success:** A creative coding community forms around the `.game` format.

---

## Milestone 5: "It Evolves" (Advanced Features)
**Goal:** GAME becomes a platform.

### Tasks
- [ ] Branch arcs: conditional narrative paths based on interaction
- [ ] Loop sections: repeating segments until interaction advances
- [ ] MIDI input: live performance / VJ controller support
- [ ] Microphone input: voice/sound reactive cinematics
- [ ] Webcam input: face/motion detection as signals
- [ ] Multi-output: render to multiple windows/screens (installation art)
- [ ] Collaborative editing: multiple people editing the same `.game` file live
- [ ] AI generation: natural language → `.game` file pipeline
- [ ] Visual editor: optional GUI for those who prefer it (built on the `.game` format, not replacing it)
- [ ] Plugin system: custom WASM functions as field generators

---

## Technology Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Compiler language | **Rust** | Performance, WASM compilation, wgpu ecosystem, type safety |
| Shader language | **WGSL** | WebGPU native, no translation needed for web target |
| Web runtime | **WASM + WebGPU** | Cross-browser, native performance, no JS framework dependency |
| Native runtime | **wgpu (Rust)** | Same shaders, same pipeline, Vulkan/Metal/DX12 backends |
| Audio | **Web Audio API** (web) / **cpal** (native) | Standard, no dependencies |
| Parser | **Custom recursive descent** | Full control over error messages, no grammar tool dependency |
| CLI | **clap** (Rust) | Standard Rust CLI framework |
| Packaging | **wasm-pack** + npm | Standard web distribution |

## Non-Goals (Explicit)

- Not a game engine (no physics, no entity system, no collision)
- Not a 3D modeling tool (no polygon meshes, no UV mapping)
- Not a video editor (no timeline-based clip editing)
- Not a shader IDE (no text editor, no debugger — use your own)
- Not competing with Unreal/Unity/Godot (different paradigm entirely)
- Not an AI video generator (real-time interactive, not pre-rendered)
