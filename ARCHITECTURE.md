# GAME Architecture

## System Overview

```
                            .game file
                                |
                    +-----------+-----------+
                    |    GAME Compiler      |
                    |  (Rust, runs once)    |
                    +-----------+-----------+
                                |
              +---------+-------+-------+---------+
              |         |               |         |
         WGSL shader  GLSL shader   JS module   HTML shell
         (.wgsl)      (.frag)     (Web Component) (standalone)
```

## Compilation Pipeline

Six phases, each operating on the output of the previous:

```
Source -> [1. Lexer] -> [2. Parser] -> [3. Resolver] -> [4. Optimizer] -> [5. Codegen] -> [6. Runtime]
```

### Phase 1: Lexer (`lexer.rs`)

Tokenizes `.game` source using the `logos` crate. Produces a flat token stream with spans.

Token categories:
- **Keywords:** `cinematic`, `layer`, `import`, `as`, `arc`, `resonate`, `memory`, `cast`, `over`, `listen`, `voice`, `score`, `breed`, `from`, `inherit`, `mutate`, `gravity`, `project`, `signals`, `route`, `hear`, `feel`, `lens`, `react`, `define`, `expose`, `ease`, `ALL`
- **Operators:** `|`, `~`, `->`, `>>`, `<>`, `!!`, `..`, `?`, `+`, `-`, `*`, `/`, `^`, `=`, `>`, `<`
- **Literals:** floats, integers, strings, identifiers
- **Units:** seconds (`3.5s`), milliseconds (`200ms`), bars (`4bars`), degrees (`45deg`), Hz, bpm
- **Structural:** `{`, `}`, `(`, `)`, `[`, `]`, `:`, `,`, `.`
- **Comments:** `#` or `//` to end of line

### Phase 2: Parser (`parser.rs`)

Hand-written recursive descent parser. Builds a typed AST:

```
Program
  ├── Imports: [Import]
  ├── Cinematics: [Cinematic]
  │     ├── name: String
  │     ├── Layers: [Layer]
  │     │     ├── name, opts, memory, cast
  │     │     └── body: Params [...] | Pipeline [Stage, Stage, ...]
  │     ├── Arcs: [ArcBlock]
  │     │     └── entries: [target, from, to, duration, easing]
  │     ├── Resonates: [ResonateBlock]
  │     │     └── entries: [source -> target.field * weight]
  │     ├── Listen: ListenBlock?
  │     │     └── signals: [name, algorithm, params]
  │     ├── Voice: VoiceBlock?
  │     │     └── nodes: [name, kind, params]
  │     ├── Score: ScoreBlock?
  │     │     └── tempo, motifs, phrases, sections, arrange
  │     ├── Gravity: GravityBlock?
  │     │     └── force_law, damping, bounds (reflect|wrap|none)
  │     ├── Lenses: [Lens]
  │     │     └── properties, post: [Stage]
  │     ├── React: ReactBlock?
  │     │     └── reactions: [signal -> action]
  │     └── Defines: [DefineBlock]
  │           └── name, params, body: [Stage]
  ├── Breeds: [BreedBlock]
  │     └── name, parents, inherit_rules, mutations
  └── Projects: [ProjectBlock]
        └── mode (flat|dome|cube|led), source, params
```

### AST Node Types

**Top-level:** `Program`, `Import`, `Cinematic`, `BreedBlock`, `ProjectBlock`

**Cinematic children:** `Layer`, `ArcBlock`, `ArcEntry`, `ResonateBlock`, `ResonateEntry`, `ListenBlock`, `ListenSignal`, `VoiceBlock`, `VoiceNode`, `ScoreBlock`, `Motif`, `Phrase`, `Section`, `GravityBlock`, `Lens`, `ReactBlock`, `Reaction`, `DefineBlock`

**Layer internals:** `LayerBody` (enum: `Params` | `Pipeline`), `Param`, `Stage`, `Arg`, `TemporalOp` (enum: `Delay` | `Smooth` | `Trigger` | `Range`)

**Expressions:** `Expr` (enum: `Number`, `String`, `Ident`, `DottedIdent`, `Array`, `Paren`, `Neg`, `BinOp`, `Call`, `Duration`, `Ternary`)

**Supporting:** `BinOp` (Add, Sub, Mul, Div, Pow, Gt, Lt), `Duration` (Seconds, Millis, Bars), `BoundsMode` (Reflect, Wrap, None), `ProjectMode` (Flat, Dome, Cube, Led), `InheritRule`, `Mutation`

### Phase 3: Resolver (`resolver.rs`)

Resolves `import` declarations:

1. Locates files relative to source or in library directories
2. Parses imported files
3. Merges imported `define` blocks into the importing program
4. Detects circular imports via visited-set tracking
5. Resolves recursively for transitive imports

Define expansion happens separately in the codegen `analysis` module before shader generation.

### Phase 4: Optimizer (`optimize.rs`)

Three AST-level passes (no intermediate representation):

**Pass 1: Constant Folding**
- Folds binary operations on numeric literals: `2.0 + 3.0` -> `5.0`
- Folds identity operations: `x * 1` -> `x`, `x + 0` -> `x`, `x * 0` -> `0`
- Folds known math calls with constant args: `sin(0.0)` -> `0.0`, `cos(0.0)` -> `1.0`, `min(3, 7)` -> `3`
- Folds negation: `-7.0` literal
- Operates bottom-up recursively

**Pass 2: No-op Stage Elimination**
- Removes identity transform stages from pipelines:
  - `translate(0, 0)` — zero displacement
  - `scale(1)` — unit scale
  - `rotate(0)` / `twist(0)` — zero rotation

**Pass 3: Dead Uniform Detection**
- Scans all expression trees in layers, lenses, arcs, resonates, defines, and react blocks
- Reports uniforms declared in `Params`-body layers but never referenced anywhere
- Does not remove them (advisory only)

### Phase 5: Code Generation (`codegen/`)

Generates shader code and supporting JavaScript for each cinematic.

**Core modules:**
- `wgsl.rs` — WGSL fragment + vertex shader generation
- `glsl.rs` — GLSL fragment + vertex shader generation (WebGL2 fallback)
- `stages.rs` — pipe chain stage compilation (maps builtins to shader code)
- `expr.rs` — expression tree to shader expression compilation

**Feature modules** (each generates JavaScript classes injected into the component):
- `memory.rs` — frame persistence via feedback textures
- `resonance.rs` — cross-layer modulation graph evaluation
- `react.rs` — event-driven interaction handlers
- `listen.rs` — custom audio signal DSP
- `voice.rs` — synthesis graph (oscillators, filters)
- `score.rs` — musical timeline scheduling
- `breed.rs` — genetic recombination JS module
- `gravity.rs` — particle physics compute shader
- `project.rs` — vertex shader overrides for projection mapping
- `temporal.rs` — delay/smooth/trigger/range operators
- `cast.rs` — typed layer output handling
- `analysis.rs` — define expansion, uniform extraction

**Output per cinematic:**
- WGSL fragment + vertex shaders
- GLSL fragment + vertex shaders
- Uniform layout (names + defaults)
- JS module classes (listen, voice, score, breed, temporal, gravity)
- Compute shader (gravity, if applicable)

### Phase 6: Runtime (`runtime/`)

Wraps shader output into usable artifacts:

- `component.rs` — generates ES module Web Components (custom elements with Shadow DOM, WebGPU init, render loop, uniform binding, resize handling, cleanup)
- `html.rs` — generates standalone HTML files (component + minimal shell)
- `arc.rs` — arc interpolation JavaScript (keyframe evaluation, easing functions)
- `helpers.rs` — shared JS snippets (WebGPU detection, error handling)

## Type State Machine

The compiler enforces a type discipline on pipe chains via three shader states:

```
Position ──[SDF generator]──> Sdf ──[bridge]──> Color ──[color processor]──> Color
    |                           |
    | [transform]               | [SDF modifier]
    v                           v
Position                      Sdf
    |
    | [full-screen generator]
    v
  Color
```

**Position -> Position:** `translate`, `rotate`, `scale`, `twist`, `mirror`, `repeat`, `domain_warp`, `curl_noise`, `displace`
**Position -> Sdf:** `circle`, `ring`, `star`, `box`, `polygon`, `fbm`, `simplex`, `voronoi`, `concentric_waves`
**Position -> Color:** `gradient`, `spectrum`
**Sdf -> Sdf:** `mask_arc`, `threshold`, `onion`, `round`
**Sdf -> Color:** `glow`, `shade`, `emissive`
**Color -> Color:** `tint`, `bloom`, `grain`, `blend`, `vignette`, `tonemap`, `scanlines`, `chromatic`, `saturate_color`, `glitch`

Invalid transitions (e.g., piping a Color result into an SDF generator) are caught during codegen.

## Import Adapters (`adapters/`)

URI-schemed imports resolve to generated JavaScript adapter modules:

| Adapter | URI scheme | Generated code |
|---------|-----------|----------------|
| `shadertoy.rs` | `shadertoy://[id]` | Fetches and wraps a Shadertoy shader |
| `midi.rs` | `midi://[channel]` | Web MIDI API input binding |
| `osc.rs` | `osc://[host]:[port]/[path]` | OSC protocol via WebSocket |
| `camera.rs` | `camera://[device]` | Webcam video texture input |

## Dev Server (`server/`)

Built on axum with `tower-livereload`:

- `mod.rs` — server setup, file watcher (notify crate), recompilation on change
- `page.rs` — HTML page builder (split-pane layout, tab bar, param sliders from uniform extraction)
- `css.rs` — dark theme CSS for the dev UI
- `export.rs` — framework export helpers (React wrapper, Vue SFC, CSS fallback generation)
- `util.rs` — HTML/JSON escaping

Tabs: Preview (iframe), WGSL (syntax display + copy), Editor (textarea + compile/save).
Right panel: component embed at selectable sizes + auto-generated param sliders.

## WASM Target (`wasm.rs`)

Behind the `wasm` feature flag. Exposes compiler functions to JavaScript via `wasm-bindgen`:

- `compile_to_wgsl(source)` — returns WGSL shader string
- `compile_to_html(source)` — returns standalone HTML string
- `compile_to_component(source)` — returns ES module JS string

Build with: `wasm-pack build --target web --features wasm`

## Performance Model

The compiled output runs a `requestAnimationFrame` loop that:

1. Evaluates signal sources (time, audio FFT bands, mouse position, data properties)
2. Evaluates resonance graph (topological order, weighted connections, damping)
3. Interpolates arc keyframes (easing functions)
4. Computes modulated uniform values (base + signal modulation + resonance)
5. Uploads uniform buffer to GPU
6. Executes render passes (fullscreen quad with fragment shader)

The shader does the real work. The JavaScript overhead per frame is minimal — uniform buffer writes and signal evaluation.

## Technology Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Compiler language | **Rust** | Performance, WASM compilation, wgpu ecosystem, type safety |
| Shader targets | **WGSL + GLSL** | WebGPU primary, WebGL2 fallback for broader support |
| Lexer | **logos** | Fast zero-allocation tokenization |
| Parser | **Hand-written recursive descent** | Full control over error messages, no grammar tool dependency |
| CLI | **clap** | Standard Rust CLI framework |
| Dev server | **axum + tower-livereload + notify** | Async Rust web framework with file watching |
| Browser runtime | **Pure JS + WebGPU/WebGL2** | No WASM in browser output — zero runtime dependencies |

## Non-Goals (Explicit)

- Not a game engine (no physics simulation, no entity system, no collision)
- Not a 3D modeling tool (no polygon meshes, no UV mapping)
- Not a video editor (no timeline-based clip editing)
- Not a shader IDE (no text editor, no debugger — use your own)
- Not competing with Unreal/Unity/Godot (different paradigm entirely)
