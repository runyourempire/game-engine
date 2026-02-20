# SWANS Analysis: GAME Engine
## Generative Animation Matrix Engine

**Date:** 2026-02-21
**Analyst:** Claude Opus 4.6 (SWANS methodology)
**Subject:** GAME compiler and runtime -- a Rust compiler for `.game` DSL files targeting WebGPU shaders, self-contained HTML, and Web Component ES modules.
**Version analyzed:** 0.2.0

---

## Phase 0: Assumption Testing

Before running the five lenses, every major claim in the VISION.md and LANGUAGE.md is tested against what the code actually does.

### Assumption 1: "Compiles to GPU shader programs"
**VERDICT: TRUE, with caveats.**

The compiler (`lib.rs:33-38`) does produce WGSL shaders via a `lex -> parse -> codegen` pipeline. The output is valid WebGPU shader code with vertex and fragment shaders. However, the "compilation" is more accurately described as **template-based code emission** than true compilation. The codegen (`codegen/mod.rs:189-224`) walks the AST and emits WGSL strings via `format!()` calls and `self.line()` -- there is no intermediate representation, no optimization passes, no type system beyond the ShaderState machine (Position/Sdf/Glow/Color), and no semantic validation phase (the "Resolver" described in `ARCHITECTURE.md:79-88` does not exist in code).

**Evidence:**
- `codegen/mod.rs:189-224`: The `generate()` method directly walks the AST and emits strings.
- `parser.rs:419-477`: `parse_react()` and `parse_resonance()` are stub implementations that skip their contents entirely.
- No file in the codebase implements the "Resolver" phase described in ARCHITECTURE.md and LANGUAGE.md Section 7.

### Assumption 2: "Cinematics as Code -- Git-versioned, diffable, AI-generated"
**VERDICT: TRUE.** The `.game` files are human-readable text. The examples and presets confirm this. No binary formats.

### Assumption 3: "Audio is Structural"
**VERDICT: PARTIALLY TRUE.** Audio-reactive parameters work via the `~` modulation operator (`parser.rs:508-526`). The runtime (`runtime.rs:204-274`) includes Web Audio API integration with FFT analysis. But audio is NOT "woven into the mathematics" as claimed -- it is an external signal that modulates uniforms on the CPU side. The shader itself has no awareness of audio; it just reads uniform floats. The beat detection is trivial (`bass > 0.5 ? 1.0 : 0.0` at `runtime.rs:273`). This is signal-driven parameter modulation, not structural audio integration.

### Assumption 4: "Resonance -- Cross-Layer Feedback"
**VERDICT: FALSE -- NOT IMPLEMENTED.** The `parse_resonance()` function (`parser.rs:450-477`) is a stub that skips all content inside the `resonate {}` block. It returns `ResonanceBlock { bindings: Vec::new(), damping: None }`. The codegen never reads resonance data. The feature described in LANGUAGE.md Section 4 does not exist.

### Assumption 5: "Arcs -- Temporal Structure"
**VERDICT: PARSED BUT NOT EXECUTED.** The parser (`parser.rs:277-415`) correctly parses arc blocks with timestamps, transitions, easing, and duration. But the codegen (`codegen/mod.rs`) never reads `cinematic.arc`. The runtime (`runtime.rs`) has no timeline interpolation logic. Arc data is parsed into the AST and then silently discarded.

### Assumption 6: "React -- Interaction"
**VERDICT: FALSE -- NOT IMPLEMENTED.** Same as resonance: `parse_react()` (`parser.rs:419-446`) is a stub that skips the block contents. Returns `ReactBlock { reactions: Vec::new() }`.

### Assumption 7: "Multiple lenses composited"
**VERDICT: FALSE.** The codegen (`codegen/mod.rs:194-212`) only reads `cinematic.layers.first()` for a single layer. Multiple layers are parsed but only the first is compiled. Lens composition is not implemented.

### Assumption 8: "Define blocks for reusable functions"
**VERDICT: PARSED BUT NOT USED.** `parse_define()` (`parser.rs:481-497`) parses the name, params, and body into a `DefineBlock`. But codegen never references `cinematic.defines`. Define calls are not inlined or expanded.

### Assumption 9: "Import system"
**VERDICT: FALSE.** The lexer has `Import` and `Expose` tokens (`token.rs:26-28`), but the parser has no `parse_import()` function. Imports are not recognized.

### Assumption 10: "Self-contained HTML and Web Component output"
**VERDICT: TRUE.** This works well. `runtime.rs:28-567` generates complete HTML with WebGPU setup, audio reactivity, mouse tracking, and audio controls. `runtime.rs:620-858` generates proper ES module Web Components with Shadow DOM, ResizeObserver, lifecycle callbacks, and data-driven properties. The 68 integration tests in `lib.rs:65-804` thoroughly validate this.

### Assumption 11: "Progressive disclosure -- hello world is 3 lines"
**VERDICT: TRUE.** `examples/001-hello.game` is 3 substantive lines and compiles to a working glowing circle.

### Assumption 12: "Tree-shaken WGSL builtins"
**VERDICT: TRUE.** `codegen/builtins.rs` uses a `HashSet<&'static str>` (`used_builtins`) to track which builtin functions are referenced during codegen, and only emits the WGSL for those that are actually used. This is genuine dead-code elimination at the function level.

### Summary: Reality vs. Claims

| Feature | Documented | Implemented | Status |
|---------|-----------|-------------|--------|
| Lexer | Yes | Yes | COMPLETE |
| Parser (core) | Yes | Yes | COMPLETE |
| Pipe chains | Yes | Yes | COMPLETE |
| SDF primitives | 10 types | 10 types | COMPLETE |
| Domain operations | 14 types | 8 types | PARTIAL |
| Noise functions | 10 types | 4 types | PARTIAL |
| Modulation (~) | Yes | Yes | COMPLETE |
| Audio signals | Yes | Yes | COMPLETE |
| Mouse signals | Yes | Yes | COMPLETE |
| Data signals | Yes | Yes | COMPLETE |
| Post-processing | 13 types | 10 types | MOSTLY |
| Shading | 7 types | 4 types | PARTIAL |
| Raymarch mode | Yes | Yes | BASIC |
| Arcs | Yes | Parsed only | STUB |
| Resonance | Yes | Parsed only | STUB |
| React | Yes | Parsed only | STUB |
| Define | Yes | Parsed only | STUB |
| Import | Documented | Not parsed | MISSING |
| Multiple layers | Yes | First only | INCOMPLETE |
| Multiple lenses | Yes | First only | INCOMPLETE |
| Web Component output | Yes | Yes | COMPLETE |
| HTML output | Yes | Yes | COMPLETE |
| Dev server | Yes | Yes | COMPLETE |
| Batch build | Yes | Yes | COMPLETE |
| MCP server | Yes | Yes | COMPLETE |
| React wrappers | Yes | Yes | COMPLETE |

**Honest assessment:** The compiler is a working v0.2 that handles 2D flat-mode shaders and basic 3D raymarching excellently, with solid audio/mouse/data reactivity. The temporal (arcs), compositional (resonance, multiple layers), interactive (react), and modular (define, import) features are all parsed but not compiled. The codebase is about 40% of the way to implementing the full language spec.

---

## Phase 1: Five Perspectives

---

### 1. SHERLOCK (The Detective)
*What does the evidence actually say? What's the real story?*

#### 1.1 The Code Tells a Different Story Than the Docs

The most striking finding is the **specification-implementation gap**. The LANGUAGE.md describes an enormously ambitious language with resonance, arcs, lenses, particles, volume rendering, path tracing, branching narratives, and MIDI. The actual compiler implements a subset that could be summarized as: **"compile a single pipe chain of SDF/noise/shading/postprocess operations into a WGSL fragment shader, wrapped in HTML or a Web Component."**

This is not a criticism -- it is an excellent v0.2. But the documents create expectations the code cannot meet. Someone reading LANGUAGE.md would expect resonance to work. It does not.

#### 1.2 The Architecture Is Monolithic

Every stage emitter lives in `codegen/stages.rs:1-393` as one giant match statement. The `emit_flat_stage()` function dispatches on `stage.name.as_str()` with 35+ match arms. This works at the current scale (~35 primitives) but will not scale to 100+. Adding a new primitive requires editing one large file.

The ShaderState machine (`codegen/mod.rs:339-345`) has 4 states: Position, Sdf, Glow, Color. This is the compiler's type system. It determines when to bridge between states (e.g., SDF-to-Color requires emitting a `height` variable). But there is no formal state transition table -- the transitions are scattered across `emit_flat_fragment()` in `stages.rs:30-48`. Missing transitions cause silent incorrect output rather than compile errors.

#### 1.3 The Expression Compiler Is Naive

`codegen/expr.rs:39-114` compiles expressions by recursive string concatenation. WGSL identifiers are resolved by a hardcoded match on name strings (`compile_ident` at `expr.rs:119-146`). Unknown identifiers pass through unchanged -- meaning a typo like `tme` instead of `time` generates WGSL that references an undeclared variable. The WGSL compiler will catch this, but the error message will be about WGSL, not about `.game`, destroying the user's debugging experience.

The JS expression compiler (`compile_expr_js` at `expr.rs:149-204`) has a fallback of `_ => "0".to_string()` for unrecognized expression types (line 203). This means `Expr::String`, `Expr::Array`, and `Expr::Ternary` in modulation expressions silently compile to `0`, producing incorrect behavior with no error.

#### 1.4 The Test Suite Is Strong but Narrow

68 integration tests in `lib.rs` cover the happy path thoroughly. Each primitive has at least one end-to-end test. Error paths are tested (empty source, missing keywords, garbage input). But:
- No test validates the WGSL output is actually valid WGSL (no WGSL parser or validator)
- No test covers multiple layers (because the feature doesn't work)
- No test exercises arcs at runtime (because arcs aren't compiled)
- No fuzz testing of the parser
- No snapshot tests to catch regression in generated output

#### 1.5 The Uniform Buffer Layout Is Fragile

`codegen/mod.rs:56-68` hardcodes 10 system floats at fixed indices (time at 0, audio bands at 1-5, resolution at 6-7, mouse at 8-9). Dynamic params start at index 10. If the system uniform layout ever changes, every piece of generated JS and WGSL must be manually updated in sync. There is no single source of truth for the layout.

The buffer size calculation (`runtime.rs:32-33`) uses `div_ceil(16) * 16` for 16-byte alignment. This is correct for WebGPU but relies on the programmer remembering to update it if the uniform struct changes.

#### 1.6 The Dev Server Is Well-Designed

`server.rs:1-258` implements a genuinely useful development workflow: split-panel view with HTML preview on the left and Web Component on the right, live reload via `tower-livereload`, and file watching via the `notify` crate. The JSON encoding function (`serde_json_inline` at `server.rs:239-257`) avoids adding `serde_json` as a dependency for a single use case -- a pragmatic choice.

#### 1.7 Dependency Audit

```
logos 0.14       -- Lexer generator. Solid, maintained. Correct choice.
clap 4           -- CLI framework. Industry standard. Correct.
axum 0.8         -- HTTP framework for dev server. Correct.
tokio 1 [full]   -- Async runtime. full features is overkill for a dev
                    server; could use just "rt-multi-thread", "macros",
                    "net" to reduce compile time.
notify 7         -- File watcher. Correct.
tower-livereload 0.9 -- Live reload middleware. Niche but functional.
tower 0.5        -- Middleware. Required by axum.
```

No unnecessary dependencies. No security concerns. Compile time could be improved by trimming tokio features.

---

### 2. WIZARD (The Paradigm Breaker)
*What paradigm-shifting opportunities exist? What would a 10x version look like?*

#### 2.1 The Killer Insight: GAME Is an Assembly Language for Visual Components

The current framing positions GAME as a "generative cinematic tool" aimed at creative coders and demoscene practitioners. This is the *smallest* possible audience for this technology.

What GAME actually is, based on what the code produces: **a compiler that turns a concise DSL into zero-dependency, GPU-accelerated Web Components**. The `data.*` signal system (`analysis.rs:20-60`, `runtime.rs:620-932`) already enables arbitrary programmatic control of shader parameters via standard DOM properties and attributes. The loading-ring preset (`presets/loading-ring.game`) is a 4-line file that compiles to a production-quality GPU loading indicator.

**The paradigm shift:** GAME is not a cinematic tool. GAME is a **visual component compiler**. The cinematic use case is one application. The universal use case is: **any visual element that benefits from GPU acceleration, expressed in 3-10 lines of DSL, compiled to a standard Web Component.**

Use cases that become trivially possible:
- Dashboard gauge components (metric-ring already exists)
- Ambient status indicators (breathing-dot already exists)
- Data visualization primitives (bind data.* to visual params)
- Background effects for web apps
- Loading states with visual fidelity
- Interactive data exploration widgets
- Generative art NFT renderers (fully deterministic, seed-based)

#### 2.2 The AI Compiler Opportunity Is Real but Requires Architecture Changes

The MCP server (`mcp-game-server/src/index.ts:1-763`) already enables AI-driven compilation. But the current architecture has a critical limitation: the MCP server shells out to the compiled binary (`runCompiler` at `index.ts:116-123`). This means:

1. Every compilation requires writing a temp file, spawning a process, and reading stdout
2. Error messages are string-formatted, not structured
3. No incremental compilation -- changing one parameter recompiles everything
4. No AST-level feedback to the AI (e.g., "these are the available parameters")

A paradigm-breaking move: compile GAME to WASM and expose a `compile(source) -> { wgsl, errors[], params[], signals[] }` API callable from JS. The MCP server (and any AI tool) gets structured output without process spawning. Errors include span information the AI can act on. The AI can query available primitives from the same WASM module.

#### 2.3 The Language Could Be Bidirectional

Right now, GAME compiles `.game -> WGSL`. What if it could also decompile `WGSL -> .game`? Even partial decompilation (recognizing common patterns like SDF primitives, FBM noise, glow functions) would enable:

- Importing Shadertoy shaders as `.game` components
- Visual debugging (show which `.game` line produced which WGSL section)
- Round-trip editing in a visual editor

This is hard but not impossible for the specific subset of WGSL patterns GAME generates.

#### 2.4 The "Performance Instrument" Use Case Is Underexplored

The VISION mentions "code metrics -> real-time visual feedback" as a moonshot application. The `data.*` signal system already supports this -- you can bind any numeric value to any visual parameter. What's missing is the **connection layer**: how do metrics get into the component?

The answer is already in the code: Web Component properties. A performance dashboard could do:
```html
<game-cpu-flame cpu={cpuPercent} memory={memPercent} errors={errorCount} />
```

The `.game` source would map those data signals to visual parameters. This requires zero additional compiler work -- it works today.

#### 2.5 Time-Travel Debugging for Shaders

Since GAME controls the uniform buffer, it knows every input to the shader at every frame. Recording the uniform buffer history (a few KB per frame) would enable:
- Scrubbing back and forth through the animation timeline
- Comparing "what the shader saw" at any two points in time
- Exporting reproducible visual bug reports

No existing shader tool offers this. It would be a genuine first.

---

### 3. ALIEN (The Outsider)
*Looking at this as someone who has never seen a shader before. What is confusing, surprising, or unnecessarily complex?*

#### 3.1 The Naming Is Inconsistent and Confusing

- `cinematic` is the root keyword, but most presets are UI components, not cinematics
- `layer` is used for what is essentially a "visual element" -- but it implies compositing layers that don't actually composite
- `lens` controls rendering mode -- but "lens" implies a camera, and the lens also controls post-processing
- `fn:` prefix for the pipe chain is borrowed from Rust, but `.game` is not a programming language -- something like `draw:` or `visual:` would be more intuitive
- `glow` is a specific visual effect, but in the ShaderState machine it is a distinct state (`ShaderState::Glow`) rather than a post-process

The most confusing naming: `shade(albedo: gold)` applies PBR shading, but `tint(gold)` applies a color multiply. Both involve color. The distinction is not obvious from the names.

#### 3.2 The Error Messages Are Excellent for a v0.2

`main.rs:192-217` formats errors with source location, line content, and a caret pointing to the error position. This is better error reporting than many mature languages. Example:

```
error: unknown built-in function: totally_fake (at byte 32..44)

  1 | cinematic { layer { fn: totally_fake(0.5) } }
    |                         ^
```

The error types (`error.rs:13-29`) are well-structured with `UnrecognizedToken`, `UnexpectedToken`, `UnexpectedEof`, `UnknownFunction`, and `Message` variants. Each carries an optional span for location reporting.

#### 3.3 The Learning Curve Has a Cliff

A user can go from zero to `circle(0.3) | glow(2.0)` in seconds. But the moment they want to do anything beyond the built-in stages, they hit a wall:

1. No way to combine two SDFs (boolean operations are documented but not implemented in the pipe chain syntax)
2. No way to use a custom function (`define` is parsed but not compiled)
3. No way to layer multiple elements (only the first layer is compiled)
4. Identifier typos silently pass through to WGSL where the error is incomprehensible

The progressive disclosure promise works for the first step but breaks at the second.

#### 3.4 The Pipe Semantics Are Ambiguous

In `circle(0.3) | glow(2.0) | tint(gold) | bloom(0.5)`, the pipe operator means four different things:

1. `circle(0.3)` -- generates an SDF value (writes `sdf_result`)
2. `glow(2.0)` -- transforms SDF to glow intensity (reads `sdf_result`, writes `glow_result`)
3. `tint(gold)` -- transforms glow to colored output (reads `glow_result`, writes `color_result`)
4. `bloom(0.5)` -- post-processes the color (reads and writes `color_result`)

The pipe looks uniform but the semantics change at each state transition. This is fine for simple chains but will cause confusion with complex ones. A user might try `circle(0.3) | tint(gold) | glow(2.0)` and get different (possibly broken) results because the state machine transitions differently.

#### 3.5 The Documentation-Implementation Gap Is the Biggest Usability Issue

A new user reads LANGUAGE.md, gets excited about resonance and arcs, writes a `.game` file using those features, and... it silently compiles with those features ignored. The parser accepts the syntax. The codegen discards the data. There is no warning, no error, no "not yet implemented" message. This is the most dangerous kind of UX failure: **silent data loss**.

---

### 4. NERD (The Metrician)
*Numbers, performance, code quality metrics, and quantitative analysis.*

#### 4.1 Codebase Metrics

| File | Lines | Role | Complexity |
|------|-------|------|------------|
| `lib.rs` | 804 | Public API + 68 integration tests | Medium |
| `main.rs` | 218 | CLI entry point | Low |
| `parser.rs` | 888 | Recursive descent parser | High |
| `lexer.rs` | 114 | Tokenizer (logos-based) | Low |
| `ast.rs` | 194 | AST type definitions | Low |
| `token.rs` | 145 | Token types | Low |
| `error.rs` | 92 | Error types | Low |
| `codegen/mod.rs` | 345 | WGSL generation core | High |
| `codegen/stages.rs` | 395 | Stage emitters (flat mode) | High |
| `codegen/builtins.rs` | 286 | Tree-shaken WGSL builtins | Medium |
| `codegen/expr.rs` | 205 | Expression compilation | Medium |
| `codegen/analysis.rs` | 144 | Signal detection | Low |
| `codegen/raymarch.rs` | 202 | 3D raymarching mode | Medium |
| `codegen/tests.rs` | 150 | Unit tests | Low |
| `runtime.rs` | 932 | HTML + Web Component output | High |
| `server.rs` | 258 | Dev server | Medium |
| **TOTAL** | **~5,372** | | |

The Rust compiler is ~5,400 lines. The MCP server is ~763 lines of TypeScript. The React wrapper is 58 lines. Total codebase: ~6,200 lines producing a working compiler, dev server, MCP server, and React integration.

**Lines per feature ratio:** ~170 lines per implemented primitive/stage. This is reasonable for a code generator.

#### 4.2 Test Coverage Analysis

- **Integration tests:** 68 tests covering 42 distinct compilation scenarios
- **Parser unit tests:** 4 tests covering basic parsing, named layers, modulation, and precedence
- **Lexer unit tests:** 3 tests covering tokenization, comments, and operators
- **Codegen unit tests:** 8 tests covering output structure, params, expressions, and modes

**Total: 83 tests.**

Coverage gaps:
- No tests for arc parsing correctness (timestamps, transitions, easing)
- No tests for error recovery (parser continuing after errors)
- No tests for the dev server endpoints
- No tests for the Web Component JS lifecycle
- No tests for the MCP server tools
- No negative tests for invalid pipe chain orderings

#### 4.3 Output Size Analysis

Based on the test assertions and runtime structure:

| Output Type | Approximate Size | Content |
|-------------|-----------------|---------|
| WGSL (simple) | ~1-2 KB | Uniforms + vertex shader + builtins + fragment shader |
| WGSL (complex) | ~3-5 KB | Above + noise functions + raymarch |
| HTML wrapper | ~8-12 KB | Full WebGPU runtime + audio + controls + shader |
| Web Component | ~6-10 KB | Self-contained ES module + shader |

These are small. A loading ring component is ~7KB total. This is excellent for a GPU-accelerated component.

#### 4.4 Compilation Performance

The compiler has no benchmark tests, but analysis of the pipeline reveals:

1. **Lexing:** O(n) via logos DFA -- effectively instant for any `.game` file
2. **Parsing:** O(n) recursive descent with no backtracking -- fast
3. **Codegen:** O(n * s) where n = AST nodes and s = stages -- dominated by string concatenation
4. **Bottleneck:** String concatenation in `runtime.rs` which uses `format!()` with large template strings. This allocates on every compilation.

For the MCP server use case, the dominant cost is process spawning, not compilation. A WASM-based compiler would eliminate this entirely.

#### 4.5 WebGPU Shader Performance Characteristics

The generated shaders have predictable performance:

- **Flat mode:** Single fullscreen quad, fragment shader evaluates SDF + effects. Cost: O(pixels * stages). At 1080p with 5 stages: ~10M evaluations/frame. Well within GPU capability.
- **Raymarch mode:** 128 march steps per pixel (hardcoded at `raymarch.rs:125`). At 1080p: ~265M SDF evaluations/frame. This is the performance cliff -- complex SDFs (FBM with 6 octaves) will struggle on integrated GPUs.

The relaxation factor (`t += d * 0.8` at `raymarch.rs:131`) is conservative. A factor of 0.9-1.0 would be faster but could cause artifacts. The choice of 0.8 is safe.

#### 4.6 Dependency Compilation Cost

```
logos 0.14         -- Proc macro. Slow to compile initially.
clap 4 [derive]    -- Proc macro. Slow to compile initially.
axum 0.8           -- Large dependency tree (hyper, http, etc.)
tokio 1 [full]     -- Large. "full" feature flag pulls in everything.
```

First build from clean: estimated 60-120 seconds on a modern machine. Incremental builds: 2-5 seconds. The `tokio [full]` flag is the biggest contributor to compile time -- trimming features could save 15-20 seconds on clean builds.

---

### 5. SURGEON (Health Assessment)
*Where is the patient healthy? Where is it sick? What needs immediate attention?*

#### 5.1 Vital Signs: HEALTHY

- **Core pipeline works end-to-end.** `.game` source -> tokens -> AST -> WGSL -> HTML/Component. All 68 integration tests pass.
- **Error handling is principled.** No `unwrap()` on user-facing paths. Errors carry source spans. CLI formats errors with source context.
- **The output is production-quality.** Generated Web Components handle lifecycle (connectedCallback/disconnectedCallback), resource cleanup (cancelAnimationFrame, ResizeObserver disconnect), WebGPU device loss, and fallback for missing WebGPU support.
- **The DSL design is genuinely good.** The pipe operator, modulation operator, and signal system are intuitive and composable.
- **The dev server is a real tool.** Split-panel preview, hot reload, slider controls for data parameters.

#### 5.2 Chronic Conditions: NEEDS ATTENTION

**A. Silent Feature Degradation**
The parser accepts resonance, react, arcs, and define blocks but the codegen silently ignores them. This will cause user confusion and support burden. **Severity: HIGH.** Fix: Either emit a compiler warning ("resonate block parsed but not yet supported") or return an error.

**B. Single-Layer Limitation**
`codegen/mod.rs:195-196` and `:205-206` both use `cinematic.layers.first()`. Multiple layers silently reduce to one. **Severity: HIGH** for the "universal visual language" goal.

**C. Missing IR (Intermediate Representation)**
The compiler goes directly from AST to string output. There is no IR where optimizations, validations, or transformations can be applied. Every new feature requires editing the string-emitting codegen. **Severity: MEDIUM.** This will become critical as the language grows.

**D. Expression Type Unsafety**
Unknown identifiers in expressions pass through to WGSL without validation (`expr.rs:144`). A typo in a `.game` file produces a WGSL error about an undeclared variable, not a `.game` error about an unknown name. **Severity: MEDIUM.**

**E. Hardcoded Constants**
Pi is repeated as `3.14159265359` in 4 locations across the codebase (`builtins.rs:77,98-99,233-234`, `stages.rs:125`). The system uniform count (10) is hardcoded in `mod.rs:68` and implicitly assumed in `runtime.rs:486-495`. **Severity: LOW** but increases maintenance risk.

#### 5.3 Acute Issues: NEEDS IMMEDIATE FIX

**A. The `unreachable!()` calls in parser.rs**
`parser.rs:58,187,648-649,653` contain `unreachable!()` macros in pattern matches that could theoretically be reached if the token stream is corrupted. In practice these are safe because the outer `match` already checked the token type, but they violate the Rust `never unwrap/panic in production` principle.

**B. JS Expression Fallback to "0"**
`expr.rs:203` silently compiles unrecognized expression types to `"0"` in JS. If someone uses a ternary expression in a modulation (e.g., `health: 0.0 ~ data.status > 0.5 ? 1.0 : 0.0`), the ternary compiles to `0` instead of the conditional. **Severity: HIGH** -- this is a data-corrupting silent failure.

**C. No Validation of Pipe Chain Ordering**
A user can write `glow(2.0) | circle(0.3)` (glow before any SDF). The codegen will reference `sdf_result` before it is declared, producing invalid WGSL. The WGSL compiler catches this, but the error is incomprehensible. **Severity: MEDIUM.**

#### 5.4 Prognosis

The patient is a healthy young codebase with strong fundamentals (good DSL design, clean Rust, excellent output quality) but with a documentation problem that overpromises relative to current capability. The immediate risks are all related to **silent failures** -- features accepted but ignored, expressions compiled incorrectly, pipe orderings not validated.

The architecture is sound for the current scope but will need an IR layer and a proper validation pass before the language can grow to its full specification.

---

## Phase 2: Convergence Synthesis

### Cross-Lens Agreement

All five perspectives converge on these findings:

1. **The core compiler is solid.** Sherlock verified the pipeline works. Nerd confirmed good metrics. Surgeon found no critical bugs in the happy path. The foundation is trustworthy.

2. **The specification-implementation gap is the primary risk.** Sherlock found it. Alien identified it as the biggest usability issue. Surgeon classified it as the highest-severity chronic condition. This gap undermines trust and wastes user time.

3. **The "cinematic" framing undersells the technology.** Wizard identified the paradigm shift: GAME is a visual component compiler, not just a cinematic tool. Alien noted the naming inconsistency (presets are UI components, not cinematics). This reframing opens a much larger market.

4. **Silent failures must become loud failures.** Sherlock documented the JS expression fallback to "0". Surgeon flagged the unvalidated pipe orderings. Alien noted the resonance-accepted-but-ignored problem. The pattern is clear: the compiler needs to either implement features or reject them loudly.

5. **The architecture needs one structural investment: an IR.** Nerd showed the codegen is O(n*s) string concatenation. Surgeon identified the missing IR as the growth bottleneck. Wizard's proposed optimizations (incremental compilation, WASM target, bidirectional compilation) all require an IR.

### Cross-Lens Disagreement

**Wizard vs. Surgeon on scope.** Wizard wants to expand (bidirectional compilation, time-travel debugging, WASM compilation). Surgeon wants to stabilize (fix silent failures, warn on unimplemented features, add validation). Both are right -- the question is sequencing.

**Resolution:** Stabilize first, expand second. The current codebase has a trust deficit from the documentation gap. Fixing that takes less work than any expansion and immediately improves every user's experience.

---

## Phase 3: Action Plan

### Tier 1: Immediate (1-2 weeks)

#### 1.1 Emit Warnings for Unimplemented Features
**Files:** `codegen/mod.rs`, `error.rs`
**Action:** When the codegen encounters a non-empty `arc`, `react`, `resonate`, or `defines` on the Cinematic, emit a compiler warning. Do NOT silently discard the data.
**Impact:** Eliminates the most common source of user confusion.
**Effort:** ~50 lines.

#### 1.2 Fix JS Expression Fallback
**File:** `codegen/expr.rs:203`
**Action:** Replace `_ => "0".to_string()` with proper compilation for `Expr::String` (pass as string literal), `Expr::Array` (compile elements), and `Expr::Ternary` (compile as JS ternary). Unknown types should return an error, not `"0"`.
**Impact:** Fixes silent data corruption in modulation expressions.
**Effort:** ~30 lines.

#### 1.3 Validate Pipe Chain Ordering
**File:** `codegen/stages.rs`
**Action:** Before emitting the fragment shader, walk the pipe chain and verify the ShaderState transitions are legal. Reject chains where a stage reads a variable that hasn't been defined yet (e.g., `glow` before any SDF).
**Impact:** Converts incomprehensible WGSL errors into clear `.game` errors.
**Effort:** ~60 lines.

#### 1.4 Validate Identifiers in Expressions
**File:** `codegen/expr.rs:119-146`
**Action:** Instead of passing unknown identifiers through as-is, check against a whitelist of valid names (builtins + param names). Unknown names produce a compiler error.
**Impact:** Catches typos at compile time instead of WGSL time.
**Effort:** ~40 lines.

### Tier 2: Short-term (2-6 weeks)

#### 2.1 Implement Multiple Layers
**Files:** `codegen/mod.rs`, `codegen/stages.rs`
**Action:** Iterate over all layers, compile each to a separate SDF/color computation, and composite them in the fragment shader using depth ordering or additive blending.
**Impact:** Unlocks the most basic compositional capability described in the spec.
**Effort:** ~200 lines.

#### 2.2 Implement Arc Compilation
**Files:** `codegen/mod.rs`, `runtime.rs`
**Action:** Compile arc moments into a JS timeline data structure. Add interpolation logic to the runtime frame loop.
**Impact:** Enables temporal evolution -- transforms GAME from "looping effect" to "cinematic with narrative arc."
**Effort:** ~300 lines.

#### 2.3 Implement Define Inlining
**Files:** `codegen/mod.rs`, `codegen/stages.rs`
**Action:** Before codegen, resolve `define` references in pipe chains by inlining their body. This is purely an AST transformation.
**Impact:** Enables reusable functions -- the first step toward a standard library.
**Effort:** ~150 lines.

#### 2.4 Introduce Structured Compiler Output
**Files:** `lib.rs`, `codegen/mod.rs`
**Action:** Add a `compile_structured()` API that returns `CompileOutput` plus `warnings: Vec<String>` and `diagnostics: Vec<Diagnostic>`. The MCP server can consume this directly.
**Impact:** Enables AI tools to get structured feedback without parsing stderr.
**Effort:** ~100 lines.

### Tier 3: Medium-term (1-3 months)

#### 3.1 Introduce an Intermediate Representation
**New file:** `codegen/ir.rs`
**Action:** Define an IR that represents the shader program as a graph of typed operations (SDF nodes, color nodes, post-process nodes, uniform references). Codegen compiles AST -> IR -> WGSL instead of AST -> WGSL.
**Impact:** Enables optimization passes, proper type checking, multiple backends.
**Effort:** ~500-800 lines.

#### 3.2 Compile to WASM
**Files:** `lib.rs`, new `wasm.rs`
**Action:** Add a WASM compilation target using `wasm-bindgen`. Expose `compile(source: &str) -> JsValue` that returns structured output.
**Impact:** Eliminates process spawning for MCP server and web-based editors. Enables client-side compilation.
**Effort:** ~200 lines + build configuration.

#### 3.3 Implement Resonance
**Files:** `codegen/mod.rs`, `runtime.rs`, `codegen/analysis.rs`
**Action:** Build resonance graph from parsed bindings. Detect cycles, enforce damping. Add resonance evaluation to the JS runtime frame loop.
**Impact:** The single most differentiating feature of the language.
**Effort:** ~400 lines.

#### 3.4 Reframe as "Visual Component Compiler"
**Files:** Documentation, README, website
**Action:** Position GAME as a compiler for GPU-accelerated Web Components, not primarily a cinematic engine. The cinematic use case becomes the showcase, not the product definition.
**Impact:** Opens the technology to a 100x larger audience (every web developer vs. demoscene practitioners).
**Effort:** Documentation work, not code.

### Tier 4: Long-term (3-6 months)

#### 4.1 Boolean SDF Operations in Pipe Chains
Support `smooth_union`, `intersect`, `subtract` as pipe stages that reference sub-chains.

#### 4.2 Volume and Particle Lens Modes
Extend beyond flat and raymarch to volumetric rendering and GPU particle systems.

#### 4.3 Import System
Load and compose `.game` files, enabling a standard library and community sharing.

#### 4.4 Visual Editor
A web-based editor that reads/writes `.game` format, providing GUI controls while keeping the text format as the source of truth.

---

## Final Assessment

### Strengths
1. **DSL design is genuinely excellent.** The pipe operator, modulation operator, and progressive disclosure model are well-conceived and rare in shader tooling.
2. **Output quality is production-grade.** The generated Web Components handle every edge case (lifecycle, cleanup, fallback, resize).
3. **The compiler is clean Rust.** Small, readable, well-organized. No unsafe code. Good error handling on user-facing paths.
4. **The `data.*` signal system is a hidden gem.** It enables GAME components to be driven by arbitrary application data, which is the bridge to "universal visual language."
5. **Test suite is thorough for the implemented features.** 68 integration tests, each validating specific output patterns.

### Weaknesses
1. **Documentation promises features the code cannot deliver.** Resonance, arcs, react, define, import, multiple layers, multiple lenses -- all parsed, none compiled.
2. **Silent failures in expression compilation.** The JS fallback to "0" and the pass-through of unknown identifiers can corrupt visual output without any error.
3. **No intermediate representation.** The direct AST-to-string codegen limits optimization, validation, and extensibility.
4. **Single-layer limitation.** Only the first layer in a cinematic is compiled, silently discarding the rest.
5. **The "cinematic" framing limits the market.** The technology is more universally useful than the naming suggests.

### Overall Health Score: 7/10

The patient is young, vigorous, and well-structured, with a clear growth path. The core compiler works correctly for its implemented scope. The primary disease is the documentation-implementation gap, which is treatable by either implementing the missing features or adding honest "not yet supported" warnings. The secondary concern is the missing IR, which is the structural investment needed before the language can grow to full specification.

**Prognosis:** Excellent, if the Tier 1 actions (warnings for unimplemented features, fix silent failures, validate pipe ordering and identifiers) are executed first. These are small changes with outsized impact on user trust.

### The One Sentence

GAME is a well-architected DSL compiler that genuinely delivers on its "zero to GPU pixels in 3 lines" promise, but its documentation describes a language roughly 2.5x larger than what the compiler currently implements, and the most important next step is making that gap visible to users rather than silently swallowing their intent.
