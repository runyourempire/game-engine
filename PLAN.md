# GAME v1.0 — Complete Visual Experience Language

**Goal:** Every 4DA use case works flawlessly. GAME is publishable, documented, and the world's first AI-native visual experience language.

**Current state:** v0.8.0 — 25.3K LOC, 484 tests, 92 examples, dual WebGPU/WebGL2, props/dom/events/accessibility just landed. Framework wrappers (React/Vue/Svelte), VS Code extension (grammar+snippets+LSP binary), WASM browser API, npm package structure — all built but unpublished.

**Honest assessment:** GAME is 85% ready. The compiler is production-grade. The gaps are specific and bounded.

---

## Phase 1: Production Hardening (Ship What Exists)

**Why:** The biggest bang for the buck. Wire everything that's built but not connected. Zero new features — just finish the wiring.

### 1.1 Wire 4DA notification atmosphere end-to-end
- Verify GAME atmosphere components (`notif-card-*.js`) load in 4DA's notification WebView
- Fix `game-notification-loader.js` to complete the full path: Rust trigger → JS loader → GPU detect → GAME component → canvas render → CSS card becomes transparent container
- Test all 4 priority levels live through the actual notification window
- Verify fallback: CSS card is the full experience when GPU unavailable
- **Acceptance:** `trigger_notification_preview` shows GAME atmosphere at correct opacity (critical: 12%, high: 9%, medium: 6%, low: 4%) in all 4 priority levels

### 1.2 Polish DOM compositing (v0.8 feature)
- Add percentage-based positioning (`at: 50% 25%`) alongside pixel positioning
- Support `width` and `max-width` on dom text elements for text wrapping
- Add `align: left|center|right` for text alignment within bounds
- Verify string prop updates are instant (no frame delay)
- **Acceptance:** notification-card.game renders identically to CSS-only card at 440x160

### 1.3 VS Code extension: wire LSP client to server
- The LSP binary exists (`game lsp`), the extension scaffolding exists — connect them
- Extension activates on `.game` files, spawns `game lsp` subprocess via stdio
- Hover shows builtin signatures, completion suggests valid next pipeline stages
- Diagnostics highlight parse errors with line:column
- **Acceptance:** Open any .game file in VS Code, get syntax highlighting + hover + completion + error squiggles

### 1.4 Harden AI generation pipeline
- Update `prompts/generate-visual.md` to include v0.8 features (props, dom, on, role)
- Add `prompts/generate-component.md` for full component generation (not just visual effects)
- Create `scripts/validate-generation.py` that takes LLM output, compiles it, reports success/failure
- Test with 20 natural language prompts → verify 90%+ compile on first try
- **Acceptance:** "Build me a notification card with a pulsing indicator and dismiss animation" → valid .game code → compiles → works

### 1.5 Fix all known edge cases
- Verify all 92 examples produce correct visual output (not just compile)
- Test WebGL2 fallback path for all post-processing pass examples
- Verify memory/feedback ping-pong works at non-square aspect ratios (440x160 notification)
- Test component lifecycle: connectedCallback → disconnectedCallback → reconnect
- **Acceptance:** Zero known bugs. All examples visually correct on both WebGPU and WebGL2.

**Phase 1 output:** GAME is production-ready for 4DA launch. Notifications work. VS Code works. AI generation works. Zero rough edges.

---

## Phase 2: Layout & Interaction (Make GAME a Real Component System)

**Why:** DOM compositing (v0.8) proved the architecture works. Now give it proper layout and interaction so GAME components can replace any UI element, not just shader effects.

### 2.1 Layout engine integration
- Integrate [Taffy](https://github.com/DioxusLabs/taffy) (pure Rust flexbox/grid, ~5K lines) as an optional dependency
- New `layout` block syntax:
  ```game
  layout row(padding: 16, gap: 12, align: center) {
    slot "indicator" size: 48 48
    layout column(gap: 4, flex: 1) {
      slot "title"
      slot "body"
    }
  }
  ```
- Compiler resolves layout at compile time, emits CSS flexbox for the DOM overlay
- Slots map to dom text elements by name
- Backward compatible: pixel `at:` positioning still works, layout is additive
- **Acceptance:** notification-card.game uses `layout` instead of pixel positions, renders correctly at any width

### 2.2 Interaction regions (SDF hit testing)
- Add `hover` and `press` blocks that reference SDF shapes:
  ```game
  hover "indicator" {
    glow_intensity: 2.5    // uniform override on hover
    cursor: "pointer"
  }
  ```
- Implementation: evaluate SDF at mouse position in JS, apply uniform overrides
- SDF shapes already define exact boundaries — hit testing is `sdf_value < 0`
- Emit CSS `cursor` changes via pointer region mapping
- **Acceptance:** Hovering over the indicator orb in notification-card.game intensifies its glow

### 2.3 Transition/animation system
- Extend `arc` blocks with states and triggers:
  ```game
  arc enter {
    opacity: 0.0 -> 1.0 over 200ms ease-out
    scale: 0.97 -> 1.0 over 200ms ease-out
  }
  arc exit {
    opacity: 1.0 -> 0.0 over 400ms ease-in
  }
  ```
- States: `enter`, `exit`, `hover`, `press`, `idle` — triggered by lifecycle/interaction
- Component exposes `playArc("enter")` and `playArc("exit")` methods
- **Acceptance:** notification-card.game has enter/exit animations that match 4DA's CSS transitions

### 2.4 Aspect ratio uniform
- Auto-inject `aspect_ratio` uniform from canvas dimensions
- Update all SDF functions to use `p.x * aspect` for correct proportions
- Fix non-square rendering (440x160 notification is 2.75:1)
- **Acceptance:** circle() renders as a circle (not ellipse) at any aspect ratio

**Phase 2 output:** GAME components have proper layout, hover effects, lifecycle animations, and correct aspect ratios. The notification card is pixel-perfect at 440x160.

---

## Phase 3: GPU Text Rendering (The Transformative Leap)

**Why:** DOM text overlay works but feels bolted on. GPU text rendering makes GAME a true visual language — text is just another pipeline stage, subject to all GAME effects (glow, warp, animate).

### 3.1 SDF font atlas generator
- Build `game font-gen <font.ttf> -o atlas.png` CLI command
- Uses msdfgen algorithm (multi-channel signed distance field) for crisp text at any size
- Generates: atlas PNG (1024x1024) + glyph metrics JSON (positions, advances, kerning)
- Ship Inter and JetBrains Mono as built-in atlases (embedded in binary like stdlib)
- **Acceptance:** `game font-gen Inter-Regular.ttf` produces atlas usable by shader

### 3.2 `text()` builtin
- New pipeline stage: `text("Hello", size=14, font="inter")` — Position → Color
- Shader samples SDF atlas texture, applies smoothstep for anti-aliased edges
- Text is a first-class GAME element: you can `tint()`, `glow()`, `warp()` it
- Automatic line breaking and alignment
- **Acceptance:** `text("4DA") | glow(1.5) | tint(0.83, 0.69, 0.22)` renders glowing gold text

### 3.3 Texture input system
- New uniform type: `texture` — loads external image as GPU texture
- Syntax: `sample("atlas")` in pipeline — samples bound texture at UV coordinates
- Enables: icon rendering, image backgrounds, atlas-based graphics
- Foundation for SDF font atlas sampling
- **Acceptance:** A .game file can load and display an external PNG image

### 3.4 Deprecate DOM overlay (optional)
- Once GPU text works, the DOM overlay becomes optional
- Components can choose: `mode: "gpu"` (pure canvas) or `mode: "hybrid"` (canvas + DOM)
- GPU mode: smaller output, no DOM manipulation, all rendering on GPU
- Hybrid mode: GPU effects + DOM text (best accessibility, current behavior)
- **Acceptance:** notification-card.game works in both modes, identical visual output

**Phase 3 output:** GAME can render text on the GPU. Text is a pipeline stage like any other — it can glow, warp, animate. Components can be pure GPU or hybrid. This is the leap from "shader DSL" to "visual experience language."

---

## Phase 4: Distribution & Documentation (The World Uses It)

**Why:** Nothing matters if people can't install it, learn it, and ship with it.

### 4.1 Publish npm packages
- `game-compiler` — WASM package for browser compilation
- `@game/react` — React wrapper
- `@game/vue` — Vue wrapper
- `@game/svelte` — Svelte wrapper
- Verify each package installs cleanly, has correct types, works in fresh project
- **Acceptance:** `npm install game-compiler && npx game-compile hello.game` works

### 4.2 Publish VS Code extension
- Marketplace listing: `4da-systems.game-language`
- Features: syntax highlighting, 17 snippets, LSP (hover, completion, diagnostics)
- Icon, README, screenshots, changelog
- **Acceptance:** Install from VS Code marketplace, open .game file, everything works

### 4.3 CLI distribution
- `cargo install game-compiler` — Rust users
- Prebuilt binaries for Windows/macOS/Linux via GitHub Releases
- `npx game-compiler` — Node users (wraps WASM)
- **Acceptance:** Fresh machine, install, compile, works

### 4.4 Documentation site
- LANGUAGE.md already exists (400+ lines) — expand to full reference
- Quick Start guide (5 minutes to first component)
- Cookbook: 10 common patterns (notification, status indicator, loading animation, data viz, etc.)
- API reference: all 45 builtins with visual examples
- Integration guides: React, Vue, Svelte, Tauri, Electron, plain HTML
- **Acceptance:** A developer who's never seen GAME can build a component in 10 minutes

### 4.5 WASM playground
- Web page with Monaco editor + live preview
- User types .game code, sees compiled output in real-time
- Powered by WASM build (already functional)
- Shareable URLs (encode source in URL hash)
- **Acceptance:** playground.game-lang.dev (or similar) works, loads fast, compiles live

**Phase 4 output:** GAME is installable everywhere (npm, cargo, binary, VS Code). Documentation is complete. Playground lets people try it instantly. The world can use it.

---

## Phase 5: Advanced Capabilities (100x Quality)

**Why:** These are the features that make GAME extraordinary — not just functional, but best-in-class.

### 5.1 3D SDF ray marching
- New `scene3d` mode that uses sphere-tracing instead of flat UV projection
- Same SDF builtins work in 3D with minimal syntax changes
- Lighting: Phong/PBR from SDF normals (gradient estimation)
- Camera: orbit, fly, static — declared in `view` block
- **Acceptance:** `sphere(0.3) | shade(1.0, 0.5, 0.2)` renders a lit 3D sphere

### 5.2 Particle systems
- First-class `particles` block with emitters, forces, constraints
- Builds on existing compute dispatch (gravity, flow already work)
- Visual: point sprites, textured quads, trail ribbons
- Physics: gravity, wind, turbulence, collision, bounds
- **Acceptance:** `particles { count: 10000, emit: "center", force: gravity(0.5) }` renders live

### 5.3 Audio synthesis verification
- Voice/listen/score blocks exist in parser + codegen but are untested end-to-end
- Verify Web Audio API chime synthesis works (critical/high notification sounds)
- Test AudioContext initialization in various contexts (WebView, browser, first-interaction)
- **Acceptance:** Critical notification plays ascending two-tone chime

### 5.4 Scene composition
- Scene blocks and transitions exist but aren't fully tested
- Verify: `scene { play "intro" for 5s, transition dissolve 1s, play "main" }` works
- Test all transition types: dissolve, fade, wipe, morph
- **Acceptance:** Multi-cinematic scene with smooth transitions renders correctly

### 5.5 Shader caching & performance
- Profile compile times for complex components
- Implement shader module caching in WebGPU renderer (avoid recompilation on reconnect)
- Lazy initialization: don't create GPU resources until component is visible
- RequestAnimationFrame optimization: pause rendering when component is offscreen
- **Acceptance:** 60fps on integrated GPU with 3 simultaneous GAME components

**Phase 5 output:** GAME handles 3D, particles, audio, scene sequencing, and performance optimization. It's not just a component system — it's a creative platform.

---

## Phase Priorities for 4DA Launch

| Phase | Priority | Effort | Impact | Blocks Launch? |
|-------|----------|--------|--------|---------------|
| **Phase 1** | CRITICAL | 2-3 sessions | High — ships what exists | YES |
| **Phase 2** | HIGH | 3-4 sessions | High — real component system | Partial (2.4 yes) |
| **Phase 3** | MEDIUM | 4-5 sessions | Transformative — GPU text | No |
| **Phase 4** | HIGH | 2-3 sessions | High — distribution | Partial (4.1-4.3 yes) |
| **Phase 5** | LOW | 5+ sessions | Extraordinary — creative platform | No |

**Minimum for launch:** Phase 1 complete + Phase 2.4 (aspect ratio fix) + Phase 4.1-4.3 (distribution)

**Ideal for launch:** Phase 1 + Phase 2 + Phase 4.1-4.3

**Post-launch trajectory:** Phase 3 → Phase 5 → GAME becomes the standard for AI-generated visuals

---

## Architecture Principles

1. **Every phase is independently valuable.** Phase 1 alone makes 4DA notifications work. Phase 2 alone makes GAME a real component system. No phase depends on a later phase.

2. **Backward compatibility is sacred.** Every existing .game file must continue to compile and produce identical output. New features are additive, never breaking.

3. **Subtlety over spectacle.** The 4DA aesthetic is whisper-quiet. GAME's role in notifications is 6-12% opacity atmosphere, not dominating effects. Production components should feel alive, not loud.

4. **CSS first, GPU optional.** For 4DA's notification system: the CSS card is the product. GAME is the atmosphere. Progressive enhancement, never degradation.

5. **Test everything.** Every new feature gets tests. Every existing test must pass. The test suite is the contract.

---

## Success Metrics

| Metric | Current | Phase 1 Target | v1.0 Target |
|--------|---------|----------------|-------------|
| Tests passing | 484 | 520+ | 600+ |
| Examples compiling | 92 | 95+ | 110+ |
| 4DA components in production | 9 | 13+ | 20+ |
| Notification atmosphere | CSS only | GAME wired | GAME polished |
| npm packages published | 0 | 0 | 4+ |
| VS Code extension | Scaffolded | Working | Published |
| AI generation success rate | Unknown | 90%+ | 95%+ |
| WebGL2 visual parity | Untested | Verified | Guaranteed |

---

*Plan authored: 2026-03-24*
*GAME compiler: D:\runyourempire\game-engine\game-compiler\*
*4DA app: D:\4DA\*
