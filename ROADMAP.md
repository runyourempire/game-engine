# GAME Compiler Roadmap — v0.6.0 to v1.0

> ~18,500 lines of Rust. 338 tests. 34/34 examples compile. 9 production components.
> Phase 1 complete. Phase 2 complete. Scaffolding is now substance.

---

## Current Reality Check

| Feature | Codegen | Parser | Runtime | Verdict |
|---------|---------|--------|---------|---------|
| fn defs + conditionals | DONE | DONE | DONE | **Production** |
| morph + palettes | DONE | DONE | DONE | **Production** |
| stdlib imports | DONE | DONE | DONE | **Production** |
| Post-processing passes | WGSL generated | DONE | **WIRED** | **Production** |
| Layer feedback | Flag tracked | DONE | **WIRED** | **Production** |
| Scene sequencing | JS class generated | DONE | **NOT WIRED** | Scaffolding |
| Cinematic-use (RTT) | AST only | DONE | **NOT WIRED** | Scaffolding |
| IFS fractals | Compute WGSL + JS | DONE | Standalone only | Scaffolding |
| L-systems | SDF WGSL + JS | DONE | Standalone only | Scaffolding |
| Cellular automata | Compute WGSL + JS | DONE | Standalone only | Scaffolding |
| Compute dispatch | Shaders generated | N/A | **WIRED** | **Runtime dispatched** |
| Art Blocks output | HTML generated | N/A | DONE | **Production** |
| LSP | API surface only | N/A | No server | Placeholder |
| Video export | Not started | N/A | Not started | Future |
| npm/WASM | Not started | N/A | Not started | Future |

---

## Phase 1: Foundation Repair

**Goal:** 34/34 examples compile. Zero known bugs. 330+ tests.
**Effort:** 1 session
**Dependencies:** None

### 1.1 Parser: Keywords-as-identifiers

**Problem:** `opacity` is `Token::Opacity`, so `bass -> outer_ring.opacity` fails in resonate blocks because `expect_ident()` rejects keyword tokens.

**Fix:** Add `expect_ident_or_keyword()` helper that accepts any keyword token as a string. Use it in:
- `parse_resonate_entry()` for field names (line ~882)
- `parse_dotted_ident()` for dotted field access
- Any context where identifiers follow `.`

**Files:** `src/parser.rs`
**Fixes:** example 015

### 1.2 Parser: Hyphenated easing names

**Problem:** `ease-out` lexes as `Ident("ease") Minus Ident("out")`. Arc entry parser reads only the first ident.

**Fix:** Add `expect_easing()` helper that consumes `ident(-ident)*` pattern. Use it in `parse_arc_entry()` where easing is read (line ~803).

**Files:** `src/parser.rs`
**Fixes:** examples 016, 017

### 1.3 Examples: Missing Sdf-to-Color bridge

**Problem:** `circle() | tint()` skips the required Sdf→Color bridge stage. Pipeline state machine correctly rejects this.

**Fix:** Insert `glow()` or `shade()` between SDF generators and color stages in affected examples. This is an example fix, not a compiler fix — the state machine is correct.

**Files:** `examples/018-react-turing.game`, `examples/019-swarm-physarum.game`, `examples/020-flow-fields.game`
**Fixes:** examples 018, 019, 020

### 1.4 Parser: Parameterless stages in pipelines

**Problem:** `polar | simplex(6.0)` fails because parser lookahead requires `IDENT LPAREN` to detect a pipeline. Also `parse_stage()` always requires parentheses.

**Fix:** Two changes:
1. Extend `parse_layer_body()` lookahead to detect `IDENT PIPE` as pipeline start
2. Make `parse_stage()` accept stages without parentheses (empty args)

**Files:** `src/parser.rs`
**Fixes:** example 023

### 1.5 Runtime: Memory texture resize

**Problem:** `_resizeMemory()` exists but is never called on canvas resize. Stale texture dimensions after resize.

**Fix:** Call `this._renderer?._resizeMemory?.()` in the `_resize()` method.

**Files:** `src/runtime/component.rs`

### 1.6 Tests

Add tests for each fix:
- keyword-as-field-name in resonate
- hyphenated easing parsing
- parameterless stage parsing
- memory resize method call in component output

**Verification gate:**
- [x] `cargo test` → 331 passing
- [x] All 34 examples compile with `game build`
- [x] All 9 production 4DA components compile clean

---

## Phase 2: Runtime Wiring

**Goal:** Features that generate code actually render. Visual proof of every v0.5/v0.6 feature.
**Effort:** 2-3 sessions
**Dependencies:** Phase 1

### 2.1 Feedback texture binding (TRIVIAL → HIGH IMPACT)

The memory ping-pong system already works. Feedback just needs an explicit shader binding.

**Tasks:**
1. In `codegen/wgsl.rs`: when `uses_feedback`, emit `@group(0) @binding(N) var u_prev_frame: texture_2d<f32>;` and `var u_prev_sampler: sampler;`
2. In `runtime/component.rs`: when `uses_feedback`, reuse the existing memory texture infrastructure to bind previous frame
3. In fragment shader: expose `textureSample(u_prev_frame, u_prev_sampler, uv)` for layers with `feedback: true`

**Files:** `src/codegen/wgsl.rs`, `src/runtime/component.rs`
**~30-50 lines**

### 2.2 Post-processing pass pipeline (MODERATE → HIGH IMPACT)

The pass WGSL is generated. The runtime needs to execute it.

**Tasks:**
1. In `runtime/component.rs`: when `pass_count > 0`:
   - Create N framebuffer textures (same size as canvas)
   - Create N render pipelines (one per pass shader)
   - Create samplers and bind groups for each pass
2. In render loop: after main render pass:
   - For pass 0: bind main output as `pass_tex`, render to FBO[0]
   - For pass i > 0: bind FBO[i-1] as `pass_tex`, render to FBO[i]
   - Copy FBO[N-1] to canvas (or render final pass directly to canvas)
3. Handle WebGL2 path: use framebuffer objects with color attachments

**Files:** `src/runtime/component.rs`
**~150-200 lines**

### 2.3 Compute shader dispatch (MODERATE → UNLOCKS 4 FEATURES)

Gravity, reaction-diffusion, swarm, and flow all generate compute shaders that are never dispatched.

**Tasks:**
1. In `runtime/component.rs`: when any compute shader exists:
   - Create compute pipeline with `createComputePipeline()`
   - Create storage buffers/textures for compute I/O
   - In render loop: dispatch compute before fragment render
2. Handle each compute type:
   - **Gravity:** dispatch N-body sim, read particle positions into vertex buffer
   - **React:** dispatch reaction-diffusion step, sample result texture in fragment
   - **Swarm:** dispatch agent step + trail decay, sample trail texture
   - **Flow:** dispatch curl noise field, advect particles

**Files:** `src/runtime/component.rs`
**~200-300 lines total across all compute types**

### 2.4 Verification

Create visual test examples:
- `examples/035-feedback-trails.game` — circles with feedback creating persistence trails
- `examples/036-blur-pass.game` — sharp geometry with blur post-processing
- `examples/037-compute-react.game` — reaction-diffusion with proper compute dispatch

**Verification gate:**
- [x] Feedback texture binding wired (memory + feedback share ping-pong)
- [x] Post-processing pass pipeline wired (FBO chain, N passes)
- [x] Compute dispatch wired (gravity, react, swarm, flow)
- [ ] Visual verification needed (feedback trails, blur pass, compute render)
- [x] 338 tests

---

## Phase 3: Showcase Examples

**Goal:** 5 jaw-dropping examples that prove GAME's composition thesis. Demo reel material.
**Effort:** 1 session
**Dependencies:** Phase 2

### 3.1 "Genesis" — L-system growth with feedback trails

```game
// L-system that expands over time, leaves glowing trails via feedback
cinematic "genesis" {
    layer branches feedback: true {
        // L-system Koch curve with glow
    }
    layer glow {
        // Soft ambient glow from the trail accumulation
    }
    pass bloom { blur(3.0) | blend_add() }
}
```

### 3.2 "Turing" — Reaction-diffusion with palette morphing

```game
// Gray-Scott reaction-diffusion colored by palette that shifts over time
cinematic "turing" {
    react { feed: 0.055, kill: 0.062, ... }
    layer viz {
        circle(0.5) | palette(ocean)
    }
    pass vignette { vignette(0.8) }
}
```

### 3.3 "Emergence" — Cellular automaton with scene transitions

```game
// Game of Life → Highlife → Brian's Brain via scene sequencing
scene "emergence" {
    play "game-of-life" for 10s
    transition dissolve over 2s
    play "highlife" for 10s
    transition fade over 3s
    play "brians-brain" for 10s
}
```

### 3.4 "Fractal Cathedral" — IFS with parametric animation

```game
// Barnsley fern with arcs that morph transform weights over time
ifs {
    transform stem [0.0, 0.0, 0.0, 0.16, 0.0, 0.0] weight 0.01
    transform left [0.85, 0.04, -0.04, 0.85, 0.0, 1.6] weight 0.85
    transform right [0.2, -0.26, 0.23, 0.22, 0.0, 1.6] weight 0.07
    transform tip [-0.15, 0.28, 0.26, 0.24, 0.0, 0.44] weight 0.07
    iterations 200000
    color position
}
```

### 3.5 "Cosmos" — Everything together

```game
// Multi-layer: swarm agents + flow field + bloom + palette cycling
cinematic "cosmos" {
    flow { type: curl, scale: 4.0, speed: 0.3, octaves: 6, ... }
    layer field {
        circle(0.3) | glow(2.0) | palette(aurora)
    }
    layer stars opacity: 0.3 {
        star(5, 0.02, 0.008) | glow(4.0) | tint(1.0, 0.95, 0.8)
    }
    pass bloom { blur(2.0) | blend_add() }
    pass color_grade { threshold(0.1) }
}
```

**Verification gate:**
- [ ] Each example compiles and renders
- [ ] Visual output is genuinely impressive (not just "technically works")
- [ ] Saved as examples/035-039

---

## Phase 4: 4DA Production Recompile

**Goal:** Ship updated components to 4DA.
**Effort:** Trivial (< 30 minutes)
**Dependencies:** Phase 2 (runtime must be solid)

**Tasks:**
1. `game build` all 9 production .game files
2. Copy output .js files to `D:\4DA\src\lib\game-components\`
3. Verify in 4DA dev server
4. Commit to 4DA repo

**Verification gate:**
- [ ] All 9 components render identically or better in 4DA
- [ ] No console errors in browser
- [ ] Void Engine Heartbeat still works

---

## Phase 5: Scene Composition

**Goal:** Cinematics can reference each other. Scenes play in sequence. IFS/L-systems usable as sources.
**Effort:** 2 sessions
**Dependencies:** Phase 2

### 5.1 Scene timeline runtime integration

**Tasks:**
1. Extend Web Component with `_timeline` property
2. When a scene JS class is present, instantiate it in `connectedCallback()`
3. Each frame: query timeline for current cinematic + blend factor
4. Manage canvas/pipeline switching based on active cinematic
5. Implement crossfade: render both cinematics, blend with factor

**Files:** `src/runtime/component.rs`, `src/codegen/scene.rs`
**~100-150 lines**

### 5.2 IFS/L-system/automaton as cinematic sources

**Tasks:**
1. Allow `use "ifs_0" as fractal` syntax to reference IFS blocks in cinematics
2. In render loop: dispatch IFS compute first, then bind result texture as layer input
3. Same pattern for L-system (render SDF to texture) and automaton (compute to texture)

**Files:** `src/codegen/mod.rs`, `src/runtime/component.rs`
**~100-120 lines**

### 5.3 Basic cinematic-use (render-to-texture)

**Tasks:**
1. Extend codegen to emit texture binding when `cinematic_uses` is non-empty
2. Manage render ordering: dependencies render first
3. Bind dependent cinematic output as texture input
4. Support `alias.r`, `alias.g`, `alias.b` in expressions

**Files:** `src/codegen/wgsl.rs`, `src/codegen/mod.rs`, `src/runtime/component.rs`
**~200-300 lines**

**Verification gate:**
- [ ] Scene example plays two cinematics in sequence with dissolve transition
- [ ] IFS fractal renders as a layer source inside a cinematic
- [ ] Cinematic-use example renders foreground over background texture
- [ ] 380+ tests

---

## Phase 6: Polish + Hardening

**Goal:** Professional quality. Comprehensive tests. Clean docs.
**Effort:** 1-2 sessions
**Dependencies:** Phase 5

### 6.1 `game dev` live preview

Ensure all new features render correctly in the dev server:
- Post-processing passes visible in preview
- Compute shader results visible
- Feedback trails visible
- Scene playback works (auto-advance or manual controls)

### 6.2 Stdlib expansion

Add more reusable functions:
- `stdlib/effects.game` — bloom, chromatic aberration, CRT scanlines
- `stdlib/motion.game` — easing functions, oscillators, noise generators
- `stdlib/fractals.game` — IFS presets (Sierpinski, Barnsley, Dragon)

### 6.3 Error messages

Improve error quality:
- "Did you mean `glow` before `tint`?" when Sdf→Color bridge is missing
- "Unknown stage 'blurr'. Did you mean 'blur'?" for typos
- Span-accurate error reporting (line:col, not byte offsets)

### 6.4 Test hardening

Target: 400+ tests
- E2e tests for every showcase example
- Fuzz testing for parser (random token sequences)
- Regression tests for all 7 fixed examples
- Property tests for state machine (all valid stage sequences)

### 6.5 Documentation

- README.md with quick start, examples, language reference
- `game --help` improvements
- Inline doc comments on all public APIs

**Verification gate:**
- [ ] `game dev` renders all showcase examples correctly
- [ ] 400+ tests passing
- [ ] All 34+ examples compile and render
- [ ] README exists with working quick start

---

## Phase 7: Distribution (Future — Only When Visual Quality Demands It)

These are NOT blocked by any current work. They're future multipliers.

### 7.1 WASM Playground
- `wasm-pack` compile of game_compiler lib
- Browser-based editor with live preview
- Zero-install "try GAME in 10 seconds" experience

### 7.2 Landing Page
- 5 embedded showcase demos running live
- "View Source" button showing the .game code
- Single CTA: download CLI or try playground

### 7.3 npm Package
- `npm install game-compiler`
- Programmatic API: `compile(source) → { js, wgsl, html }`
- Framework adapters: React, Svelte, Vue

### 7.4 VS Code Extension
- Syntax highlighting (TextMate grammar from token definitions)
- LSP integration (wire existing Diagnostic/CompletionItem API)
- Inline preview panel

### 7.5 Video Export
- `wgpu` headless rendering at specified resolution
- Frame-by-frame capture → ffmpeg pipe → MP4/WebM
- Deterministic output for Art Blocks verification

---

## Execution Order

```
Phase 1 ──→ Phase 2 ──→ Phase 3 ──→ Phase 4
              │                        │
              └────→ Phase 5 ────→ Phase 6
                                       │
                                       └────→ Phase 7 (future)
```

Phase 1 is the foundation. Everything else builds on it.
Phase 2 is the unlock — turns generated code into rendered visuals.
Phase 3 is the proof — visual evidence that composition works.
Phase 4 is the payoff — updated 4DA components.
Phase 5 is the ambition — multi-cinematic composition.
Phase 6 is the polish — professional quality.
Phase 7 is the reach — when the work speaks for itself.

---

## Metrics

| Milestone | Tests | Examples | LOC | Status |
|-----------|-------|----------|-----|--------|
| v0.3.0 | 234 | 27/34 | 13,690 | Shipped |
| v0.6.0 | 311 | 27/34 | 17,683 | Shipped |
| Phase 1 | 331 | 34/34 | ~18,000 | **Done** |
| Phase 2 | 338 | 34/34 | ~18,500 | **Done** |
| Phase 3 | 360+ | 42/42 | ~19,000 | Planned |
| Phase 6 | 400+ | 42/42 | ~20,000 | Target |
