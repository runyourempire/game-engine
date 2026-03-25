# GAME Compiler Edge Case Test Report

**Date:** 2026-03-25
**Compiler Version:** v1.0.0
**Existing Test Suite:** 567 tests, ALL PASSING
**Edge Tests Executed:** 75+
**Overall Result:** 1 BUG found, 3 WARNINGS, 2 OBSERVATIONS

---

## Phase 1: Advanced Pipeline Edge Cases

### Test 1: Every SDF Generator Alone (18 tests)

| Test | Pipeline | Result |
|------|----------|--------|
| 1a | `circle \| glow` | PASS |
| 1b | `ring \| glow` | PASS |
| 1c | `star \| glow` | PASS |
| 1d | `box \| glow` | PASS |
| 1e | `hex \| glow` | PASS |
| 1f | `fbm \| palette(fire)` | PASS |
| 1g | `simplex \| glow` | PASS |
| 1h | `voronoi \| glow` | PASS |
| 1i | `line \| glow` | PASS |
| 1j | `capsule \| glow` | PASS |
| 1k | `triangle \| glow` | PASS |
| 1l | `arc_sdf \| glow` | PASS |
| 1m | `cross \| glow` | PASS |
| 1n | `heart \| glow` | PASS |
| 1o | `egg \| glow` | PASS |
| 1p | `spiral \| glow` | PASS |
| 1q | `grid \| glow` | PASS |
| 1r | `radial_fade \| glow` | PASS |

**All 18 SDF generators compile successfully as standalone pipelines.**

### Test 2: Every Transform Before Circle (9 tests)

| Test | Pipeline | Result |
|------|----------|--------|
| 2a | `translate \| circle \| glow` | PASS |
| 2b | `rotate \| circle \| glow` | PASS |
| 2c | `scale \| circle \| glow` | PASS |
| 2d | `warp \| circle \| glow` | PASS |
| 2e | `distort \| circle \| glow` | PASS |
| 2f | `polar \| circle \| glow` | PASS |
| 2g | `repeat \| circle \| glow` | PASS |
| 2h | `mirror \| circle \| glow` | PASS |
| 2i | `radial \| circle \| glow` | PASS |

**All 9 transforms work correctly before SDF generators.**

### Test 3: Every Bridge Type (4 tests)

| Test | Pipeline | Result |
|------|----------|--------|
| 3a | `circle \| glow` | PASS |
| 3b | `circle \| shade(1.0, 0.5, 0.2)` | PASS |
| 3c | `circle \| palette(fire)` | PASS |
| 3d | `circle \| emissive(2.0)` | PASS |

**All 4 bridge types compile correctly.**

### Test 4: Every Color Processor (4 tests)

| Test | Pipeline | Result |
|------|----------|--------|
| 4a | `circle \| glow \| tint(...)` | PASS |
| 4b | `circle \| glow \| bloom(...)` | PASS |
| 4c | `circle \| glow \| grain(...)` | PASS |
| 4d | `circle \| glow \| outline(...)` | PASS |

**All 4 color processors compile correctly.**

### Test 5: Every SDF Modifier (4 tests)

| Test | Pipeline | Result |
|------|----------|--------|
| 5a | `circle \| mask_arc(1.5) \| glow` | PASS |
| 5b | `circle \| round(0.02) \| glow` | PASS |
| 5c | `circle \| shell(0.02) \| glow` | PASS |
| 5d | `circle \| onion(3, 0.02) \| glow` | PASS |

**All 4 SDF modifiers compile correctly.**

### Test 6: Chained Transforms (1 test)

| Test | Pipeline | Result |
|------|----------|--------|
| 6 | `translate \| rotate \| scale \| warp \| circle \| glow` | PASS |

### Test 7: Every SDF Boolean (7 tests)

| Test | Pipeline | Result |
|------|----------|--------|
| 7a | `union(circle, ring) \| glow` | PASS |
| 7b | `subtract(circle, box) \| glow` | PASS |
| 7c | `intersect(circle, star) \| glow` | PASS |
| 7d | `smooth_union(circle, circle, 0.1) \| glow` | PASS |
| 7e | `smooth_subtract(circle, box, 0.1) \| glow` | PASS |
| 7f | `smooth_intersect(circle, star, 0.1) \| glow` | PASS |
| 7g | `xor(circle, ring) \| glow` | PASS |
| 7h | `morph(circle, star, 0.5) \| glow` | PASS |

**All 8 boolean/morph operations compile correctly.**

---

## Phase 2: Multi-layer Stress Tests

### Test 8: Layer Count Scaling (5 tests)

| Test | Layers | Result |
|------|--------|--------|
| 8a | 1 layer | PASS |
| 8b | 2 layers | PASS |
| 8c | 5 layers | PASS |
| 8d | 10 layers | PASS |
| 8e | 20 layers | PASS |

**Scales cleanly to 20 layers with no issues.**

### Test 9-10: Blend Modes (2 tests)

| Test | Config | Result |
|------|--------|--------|
| 9 | All layers `blend: add` | PASS |
| 10 | Mix of `blend: add` and `blend: occlude` | PASS |

**Note:** `blend:` must be in the layer header, not as a body statement.

### Test 11-12: Memory Extremes (2 tests)

| Test | Memory Value | Result |
|------|-------------|--------|
| 11 | `memory: 0.99` (near-permanent) | PASS |
| 12 | `memory: 0.01` (near-zero) | PASS |

**Note:** `memory:` must be in the layer header.

### Test 13: Config Parameter Scaling (3 tests)

| Test | Params | Result |
|------|--------|--------|
| 13a | 1 param | PASS |
| 13b | 5 params | PASS |
| 13c | 20 params | PASS |

**Config block scales to 20 parameters without issue.**

### Test 14: Resonate Coupling Scaling (2 tests)

| Test | Couplings | Result |
|------|-----------|--------|
| 14a | 1 coupling | PASS |
| 14b | 5 couplings | PASS |

### Test 15: Arc Animation Scaling (5 tests)

| Test | Animations | Easing | Result |
|------|-----------|--------|--------|
| 15a | 1 animation | ease-out | PASS |
| 15b | 2 animations | all with easing | PASS |
| 15c | 3 animations | all with easing | PASS |
| 15d | 4 animations | all with easing | PASS |
| 15e | 4 animations | no easing | **FAIL** |
| 15f | 2 animations | no easing | **FAIL** |
| 15g | 2 arcs, last has no easing | PASS |

**BUG FOUND (SEVERITY: MEDIUM):** When a non-last arc entry omits its easing
function, the parser greedily consumes the next entry's identifier as an easing
string, then fails on the subsequent colon. The easing check at parser.rs:1003
(`if matches!(self.peek(), Some(Token::Ident(_)))`) does not distinguish between
a valid easing name (e.g., "ease-out") and the start of the next arc entry.

**Workaround:** Always specify an easing function on every arc entry except the
last one, OR put the no-easing entries last.

**Error message:** `expected identifier, found ':'`

**Root cause:** `src/parser.rs` line 1003 -- after parsing `over Xs`, the parser
checks if the next token is an Ident and if so, calls `expect_easing()`. But the
next line's variable name is also an Ident. The parser needs to distinguish between
easing keywords (ease-out, ease-in, ease-in-out, linear) and general identifiers.

### Test 16: Full Combination (1 test)

| Test | Config | Result |
|------|--------|--------|
| 16 | config + resonate + arc + memory + blend | PASS |

---

## Phase 3: Output Format Coverage (5 tests)

| Test | Format | Files Produced | Result |
|------|--------|---------------|--------|
| 17 | `component` | .js, .d.ts, .wgsl, .frag | PASS |
| 18 | `split` | .js, .d.ts, .wgsl, .frag, game-runtime.js | PASS |
| 19 | `html` | .js, .d.ts, .wgsl, .frag, .html | PASS |
| 20 | `standalone` | .js, .d.ts, .wgsl, .frag, .html | PASS |
| 21 | `artblocks` | .js, .d.ts, .wgsl, .frag, .html | PASS |

**All 5 output formats produce valid files. Split format also generates a shared
runtime file. HTML/standalone/artblocks all include embeddable HTML files.**

---

## Phase 4: Multi-file Builds (4 tests)

| Test | Scenario | Result |
|------|----------|--------|
| 22 | 2 .game files in one command | PASS |
| 23 | Directory of .game files | FAIL (Expected -- not supported) |
| 24 | File with 2 cinematics | PASS (produces 2 output files) |
| 25 | Same file twice to same dir | PASS (clean overwrite) |

**Note:** Test 23 is not a bug -- the CLI expects explicit file paths, not directories.
The error message is clear: "The system cannot find the path specified."

---

## Phase 5: Validate vs Build Consistency (10 tests)

| Test | Scenario | Validate | Build | Agree? |
|------|----------|----------|-------|--------|
| 26 | Valid simple file | ok | ok | YES |
| 27 | Invalid pipeline order | Error: expects Sdf input but pipeline is in Position state | Same error | YES |
| 28 | Unknown builtin | Error: unknown stage 'foobar'. Did you mean 'polar'? | Same error | YES |
| 29 | Missing closing brace | Error: expected `}`, found end of input | Same error | YES |
| 30 | Only comments | Warning: no components, ok | ok (no output) | YES |
| 31 | Only whitespace | Warning: no components, ok | ok (no output) | YES |
| 32 | Unknown palette | Error: unknown palette + lists all 30 valid palettes | Same error | YES |
| 33 | Valid multi-layer | ok | ok | YES |
| 34 | Config + palette | ok | ok | YES |
| 35 | Valid arc | ok | ok | YES |

**100% agreement between validate and build on all test cases. Error messages are
excellent -- they include "did you mean" suggestions and list valid alternatives.**

---

## Phase 6: LSP Functionality (5 tests)

| Test | Scenario | Result |
|------|----------|--------|
| 36 | Compiles with `--features lsp` | PASS |
| 37 | Completions offered | Context-aware: builtins filtered by pipeline state (Position/Sdf/Color) + user-defined `fn` functions. Snippet insertion with tabstops for parameters. |
| 38 | Malformed JSON-RPC | Graceful: `eprintln` + returns `None`, no crash. Unknown requests get `MethodNotFound` error response. |
| 39 | Go-to-definition | YES: supports `fn <name>` definitions within the file. Builtins return None (no source location). Handles prefix collisions correctly (myEffect vs myEffectExtra). |
| 40 | Hover | YES: shows builtin name, input/output types, signature with defaults, parameter documentation in Markdown. |

**LSP implementation is solid. 725 lines with 14 unit tests covering state detection,
word extraction, function finding, and snippet building.**

---

## Phase 7: WASM Target (4 tests)

| Test | Scenario | Result |
|------|----------|--------|
| 41 | Exported functions | 4 functions: `compileGame(source, target)`, `validateGame(source)`, `getBuiltins()`, `getPaletteNames()` |
| 42 | unwrap/panic calls | NONE in production code. All error paths use `map_err(e.to_string())` or `unwrap_or_else`. |
| 43 | compileGame error handling | Returns `Err(String)` -- no panics. |
| 44 | validateGame error handling | Returns error string -- no panics. |

**WASM module is clean. No panics possible from WASM entry points.**

---

## Phase 8: Dev Server (4 tests)

| Test | Scenario | Result |
|------|----------|--------|
| 45 | Routes | 2 routes: `/version` (returns version number), `*` (returns compiled HTML). No filesystem access from URL. |
| 46 | Compilation errors in watch | Handled gracefully: `eprintln` error, keeps serving last successful build. |
| 47 | File types served | Only `text/html` and `text/plain`. No static file serving. |
| 48 | Path traversal | NOT VULNERABLE: no filesystem access based on URL. All content served from memory. |

**Dev server is minimal and secure. Uses `tiny_http`, watches parent directory
for changes, debounces at 50ms, and uses `unwrap_or_else(e.into_inner())` for
poisoned mutex recovery.**

---

## Phase 9: Generated Output Analysis (7 tests)

| Test | Scenario | Result |
|------|----------|--------|
| 49 | WebGPU availability check | PASS: `if (!navigator.gpu) return false` + adapter null check |
| 50 | WebGL2 availability check | PASS: `getContext('webgl2')` with null check in `init()` |
| 51 | Both unavailable fallback | PASS: `console.warn('no WebGPU or WebGL2 support'); return;` -- graceful degradation |
| 52 | eval() calls | NONE found. Clean. |
| 53 | innerHTML assignments | NONE found. Uses `document.createElement` + `shadowRoot.appendChild`. Clean. |
| 54 | requestAnimationFrame cancelable | PARTIAL: `this.running = false` stops the loop. But when `!this._visible`, rAF continues scheduling (skips render only). |
| 55 | ResizeObserver zero-size | **WARNING**: `_resize()` does `Math.round(rect.width * dpr)` which can be 0. WebGL `_resizeMemory` protects with `|| 1`, but WebGPU `render()` uses `this.canvas.width` and `this.canvas.height` without `|| 1` -- zero-size textures would be invalid. |

---

## Additional Edge Case Tests

### Boundary Value Tests (7 tests)

| Test | Scenario | Result |
|------|----------|--------|
| 40 | Empty cinematic (no layers) | PASS (produces valid but empty component) |
| 41 | Duplicate layer names | PASS (no validation -- potential issue) |
| 42 | Long pipeline (14 stages chained) | PASS |
| 43 | Two cinematics in one file | PASS (produces 2 separate output files) |
| 44 | Zero parameter values | PASS (circle(0.0), glow(0.0)) |
| 45 | Negative parameter values | PASS (no bounds checking) |
| 46 | Huge parameter values (999999) | PASS (no bounds checking) |

### Error Quality Tests (5 tests)

| Test | Scenario | Error Quality |
|------|----------|---------------|
| 50 | Empty layer body | PASS (produces valid output -- empty layer) |
| 51 | Unclosed parenthesis | EXCELLENT: "expected `)`, found `\|`" |
| 52 | Double bridge (glow then shade) | EXCELLENT: "stage 'shade' expects Sdf input but pipeline is in Color state" |
| 53 | SDF modifier after bridge | EXCELLENT: "stage 'round' expects Sdf input but pipeline is in Color state" |
| 54 | Transform after bridge | EXCELLENT: "stage 'rotate' expects Position input but pipeline is in Color state" |
| 55 | Layer without cinematic | EXCELLENT: lists all valid top-level keywords |

### Shader Target Tests (2 tests)

| Test | Target | Result |
|------|--------|--------|
| -- | `-t webgpu` only | PASS |
| -- | `-t webgl2` only | PASS |

---

## Summary of Findings

### BUG (1 found)

**ARC-EASING-GREEDY (Severity: MEDIUM)**
- **Location:** `src/parser.rs` line 1003
- **Description:** The arc entry parser greedily consumes the next entry's identifier
  as an easing function name when the current entry omits easing. This breaks multi-arc
  blocks where any non-last entry lacks an easing specifier.
- **Reproduction:** Any arc block with 2+ entries where a non-last entry omits easing.
- **Workaround:** Always specify easing on all arc entries except the last.
- **Fix:** The easing parser at line 1003 should check if the next Ident is a known
  easing function (ease-out, ease-in, ease-in-out, linear, etc.) before consuming it.
  Alternatively, use a lookahead: if the Ident is followed by a colon, it's the next
  entry, not an easing.

### WARNINGS (3 found)

1. **ZERO-SIZE-CANVAS (Severity: LOW)**
   - Generated WebGPU renderer does not guard against zero-size canvas dimensions in
     `render()`. Could cause WebGPU validation errors if ResizeObserver fires with 0x0.
   - WebGL path partially guards with `|| 1` in `_resizeMemory` but not in `render()`.

2. **DUPLICATE-LAYER-NAMES (Severity: LOW)**
   - No validation prevents duplicate layer names in the same cinematic. The second
     layer silently overwrites/coexists with the first. Could cause confusing behavior.

3. **EMPTY-CINEMATIC (Severity: LOW)**
   - An empty cinematic (no layers) produces a valid but functionally empty Web
     Component. Consider warning the user, like the "contains no components" warning
     for empty files.

### OBSERVATIONS (2)

1. **rAF-WHEN-INVISIBLE:** Both WebGPU and WebGL renderers continue scheduling
   `requestAnimationFrame` when the component is off-screen (`!this._visible`).
   They skip the render call but still schedule the next frame. This wastes a tiny
   amount of CPU. Could be improved by stopping the loop when invisible and restarting
   via IntersectionObserver callback.

2. **NO-BOUNDS-CHECKING:** The compiler accepts any numeric value for parameters
   (negative radius, zero glow, huge values). This is probably intentional -- the
   shader will handle the math -- but could lead to confusing visual results. No
   severity assigned as this is a design choice.

---

## Test Infrastructure

- All test files: `D:/runyourempire/game-engine/game-compiler/tmp-edge-tests/`
- All build outputs: `tmp-edge-tests/out/` and format-specific subdirectories
- Existing test suite: 567 tests, all passing
- Compiler build: clean, no warnings
- LSP feature build: clean, no warnings
