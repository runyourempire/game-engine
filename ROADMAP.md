# GAME Roadmap

## Milestone 0: "Proof of Concept" -- COMPLETE
**Goal:** Parse a `.game` file and produce a working WebGPU shader.
**Deliverable:** CLI tool that reads `001-hello.game` and renders a glowing circle in the browser.

### Completed
- [x] Lexer: tokenize `.game` syntax (Rust, `logos` crate)
- [x] Parser: build AST for `cinematic`, `layer`, `fn`, pipe chains
- [x] Codegen: compile a pipe chain into WGSL + GLSL fragment shaders
- [x] Runtime: Web Component with WebGPU setup -- fullscreen quad, uniform buffer, render loop
- [x] Output: self-contained HTML and ES module Web Component (no WASM in browser)
- [x] Validate: `001-hello.game` renders correctly

**Result:** A `.game` file produces pixels. The compiler works.

---

## Milestone 1: "It Breathes" -- COMPLETE
**Goal:** Audio drives the visuals. The `~` operator works.
**Deliverable:** `002-audio-reactive.game` plays synced to music.

### Completed
- [x] Modulation: `~` operator -- parse, resolve, compile to uniform bindings
- [x] Signals: `audio.bass`, `audio.mid`, `audio.treble`, `audio.energy`, `time`
- [x] Mouse: `mouse.x`, `mouse.y` cursor position signals
- [x] Data binding: `data.*` signals map to component properties
- [x] Uniforms: dynamic uniform buffer with all modulated parameters
- [x] SDF library: 37 builtins (circle, ring, star, box, polygon, fbm, simplex, voronoi, etc.)
- [x] Dual shader output: WGSL (WebGPU) + GLSL (WebGL2 fallback)
- [x] Type state machine: Position -> Sdf -> Color with compile-time validation
- [x] Optimizer: constant folding, no-op stage elimination, dead uniform detection
- [x] Validate: audio-reactive examples render correctly

**Result:** Music and math produce synchronized visuals.

---

## Milestone 2: "It Responds" -- COMPLETE
**Goal:** The viewer affects the experience. Time has structure.
**Deliverable:** Interactive cinematics with arcs, reactions, and event handling.

### Completed
- [x] React block: parse and execute `react` declarations (signal -> action bindings)
- [x] Arc system: timeline-driven parameter transitions with easing
- [x] Listen block: custom audio signal extraction (DSP algorithms on FFT data)
- [x] Voice block: synthesis graph (oscillators, filters, output chains)
- [x] Score block: musical composition (motifs, phrases, sections, tempo-synced arrangement)
- [x] Temporal operators: `>>` (delay), `<>` (smooth), `!!` (trigger), `..` (range clamp)
- [x] Ternary expressions: `cond ? a : b` for conditional logic in expressions
- [x] Post-processing builtins: bloom, chromatic, vignette, tonemap, scanlines, grain, glitch
- [x] Define blocks: user-defined reusable pipeline macros
- [x] Validate: interactive and timeline-driven examples work

**Result:** Interactive generative cinema with temporal structure.

---

## Milestone 3: "It Lives" -- COMPLETE
**Goal:** Layers influence each other. Genetic recombination. Physics. Spatial projection.
**Deliverable:** Emergent visual dynamics from cross-layer interactions.

### Completed
- [x] Resonate block: cross-layer modulation with weighted connections
- [x] Breed block: genetic recombination of cinematics (inherit rules + mutations)
- [x] Gravity block: particle physics (force laws, damping, boundary modes: reflect/wrap/none)
- [x] Project block: spatial projection mapping (flat, dome, cube, LED)
- [x] Memory: per-layer persistent state across frames (feedback effects, trails)
- [x] Cast: typed layer output (point, field, color)
- [x] Import system: file imports with define merging + circular import detection
- [x] Import adapters: Shadertoy, MIDI, OSC, camera URI schemes
- [x] Standard library: 6 stdlib modules (primitives, noise, backgrounds, transitions, post, ui)
- [x] Validate: resonance, breed, gravity, and projection examples work

**Result:** The full language works. Emergent cinematics exist.

---

## Milestone 4: "It Spreads" -- PARTIAL
**Goal:** Anyone can create, share, and experience GAME cinematics.

### Completed
- [x] `game build` CLI: batch compile `.game` files to output directory
- [x] `game compile` CLI: single file compilation to stdout (HTML, component, or WGSL)
- [x] `game dev` CLI: hot-reload dev server with preview, WGSL viewer, editor, param sliders
- [x] Web Component output: zero-dependency ES modules that work everywhere
- [x] Standard library: `stdlib/` with 6 reusable modules
- [x] Import system: compose `.game` files from other `.game` files
- [x] npm package structure: `package/` with create-game scaffolding
- [x] Framework wrappers: React, Vue, Svelte wrappers in `package/`
- [x] VS Code extension: syntax highlighting, snippets, language configuration
- [x] WASM target: compile `.game` files in the browser (behind feature flag)
- [x] Playground: browser-based WASM compilation playground
- [x] Showcase: `showcase.html` with all components rendering live
- [x] 24 presets: ready-to-use `.game` files covering common use cases
- [x] 21 examples: progressive feature demonstrations

### Not yet implemented
- [ ] Video export: offline frame-perfect rendering to video (mp4/webm)
- [ ] Native binary player: `game-player` command for desktop playback
- [ ] Documentation site: language reference, tutorials, example gallery
- [ ] Gallery: curated collection of community `.game` cinematics
- [ ] npm publish: `npx create-game` scaffolding live on npm

### Notes
The distribution story is functional for developers who build from source. The compiler produces production-quality Web Components. What's missing is the polish layer: video export for non-interactive contexts, a native player, and public distribution via npm/documentation site.

---

## Milestone 5: "It Evolves" -- NOT STARTED
**Goal:** GAME becomes a platform.

### Planned
- [ ] Branch arcs: conditional narrative paths based on interaction
- [ ] Loop sections: repeating segments until interaction advances
- [ ] Visual editor: optional GUI for `.game` file creation
- [ ] Plugin system: custom WASM functions as field generators
- [ ] Collaborative editing: multiple people editing the same `.game` file live
- [ ] AI generation: natural language -> `.game` file pipeline (prompts exist, validation pipeline exists)
- [ ] Multi-output: render to multiple windows/screens (installation art)

### Notes
AI generation has groundwork laid (see `prompts/generate-visual.md` and `validate.py`) but is not integrated into the CLI or runtime. The visual editor and plugin system are design-phase only.

---

## Technology Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Compiler language | **Rust** | Performance, WASM compilation, wgpu ecosystem, type safety |
| Shader languages | **WGSL + GLSL** | WebGPU primary, WebGL2 fallback for broader browser support |
| Browser output | **Pure JS + WebGPU/WebGL2** | Zero runtime dependencies, no WASM in browser |
| Parser | **Custom recursive descent** | Full control over error messages, no grammar tool dependency |
| CLI | **clap** (Rust) | Standard Rust CLI framework |
| Dev server | **axum + tower-livereload** | Async Rust, file watching via notify |
| Packaging | **wasm-pack** (optional) | WASM target for browser-based compilation |

## Non-Goals (Explicit)

- Not a game engine (no physics, no entity system, no collision)
- Not a 3D modeling tool (no polygon meshes, no UV mapping)
- Not a video editor (no timeline-based clip editing)
- Not a shader IDE (no text editor, no debugger -- use your own)
- Not competing with Unreal/Unity/Godot (different paradigm entirely)
- Not an AI video generator (real-time interactive, not pre-rendered)
