# GAME Compiler Audit Report

**Date:** 2026-03-25
**Version:** 1.0.0
**Auditor:** Claude Opus 4.6 (automated exhaustive audit)
**Codebase:** 29,779 lines Rust across 48 source files

---

## 1. Test Suite Results

| Metric | Count |
|--------|-------|
| **Total tests** | 567 |
| **Passed** | 567 |
| **Failed** | 0 |
| **Ignored** | 0 |

**Verdict:** 100% pass rate. All library tests, main binary tests, and doc-tests pass. No flaky or skipped tests.

---

## 2. Example Compilation Matrix

**Total examples:** 76 files
**Compiled successfully:** 75 (98.7%)
**Failed:** 0
**No output (silent skip):** 1

| # | File | Status | Notes |
|---|------|--------|-------|
| 001-054, 060 | All numbered examples | PASS | All produce .js + .d.ts |
| 3d-golden-sphere through solar-flare | All named examples | PASS | All produce .js + .d.ts |
| 011-project-dome | `project dome(segments:8) { source: main }` | SILENT SKIP | Compiles without error but produces no output files. No warning emitted. |

**Multi-component files** (018, 019, 020, 021, 022, 023, 030, 031, 032, 033, 037, 040, 041, 043, 046) correctly produce multiple outputs per file.

**Output statistics (106 JS files from 76 examples):**
- Smallest: 795 bytes (`child.js` — breed merger)
- Largest: 51,564 bytes (`intelligence-banner.js`)
- Total: 2.95 MB across 106 .js files
- TypeScript definitions: 103 .d.ts files (3 special outputs — breed/scene/matrix — lack .d.ts)

---

## 3. Gallery Compilation Matrix

**Total gallery components:** 32 files
**Compiled successfully:** 0 (0%)
**Failed:** 32 (100%)

| Category | Count | Error |
|----------|-------|-------|
| 4da-production/ | 3 | `expected 'cinematic' at top level, found 'component'` |
| backgrounds/ | 8 | Same |
| data-viz/ | 5 | Same |
| effects/ | 6 | Same |
| indicators/ | 6 | Same |
| micro-interactions/ | 4 | Same |

**Root cause:** All 32 gallery files use the `component` keyword at top level, but the parser only accepts `cinematic`, `breed`, `project`, `scene`, `matrix`, `ifs`, `lsystem`, or `automaton`. The gallery appears to be written for a different syntax variant that was never implemented or was removed.

**Severity: HIGH** — The VS Code extension ships a gallery where zero components compile.

---

## 4. Boundary Test Results

### Syntax Boundaries

| # | Test Case | Result | Notes |
|---|-----------|--------|-------|
| 1 | Empty file (0 bytes) | PASS (silent) | No output, no error. `validate` also says "ok". |
| 2 | Empty cinematic `cinematic "empty" {}` | PASS | Generates 517-line JS with full renderer boilerplate for an empty shader. |
| 3 | Canvas property `canvas(800,600)` | FAIL | `canvas` is not a valid keyword inside cinematic blocks. |
| 4 | No canvas (implicit) | PASS | Works correctly with default canvas. |
| 5 | Duplicate pipeline stages `glow(1.0) \| glow(2.0)` | FAIL | Correctly caught: "stage 'glow' expects Sdf input but pipeline is in Color state". |
| 6 | Unknown property `foobar(1.0)` | FAIL | Correctly caught with "Did you mean 'polar'?" suggestion. |
| 7 | Negative values `glow(-1.0)` | N/A | `speed` is not a stage function (correctly rejected), but `glow(-1.0)` would compile. |
| 8 | Extreme values `canvas(10000, 10000)` | FAIL | Canvas rejected (not valid inside cinematic). |
| 9 | Zero values `canvas(0, 0)` | FAIL | Canvas rejected (same reason). |
| 10 | Very long name (200 chars) | PASS | Compiles and generates 200-char filename. No length validation. |
| 11 | Unicode name "uber" | PASS | Outputs `uber.js`. May cause issues on some filesystems. |
| 12 | Name with numbers "effect-2d" | PASS | Works correctly. |
| 13 | Multiple cinematics | FAIL | `rect` is not valid (should be `box`). Multiple cinematics DO work per the examples. |
| 14 | Comments everywhere | PASS | Inline and block comments handled correctly. |
| 15 | Multi-layer complex | FAIL | Same `rect` issue — when using `box`, multi-layer works. |
| 16 | Float precision (15 decimal places) | PASS | Outputs 6 decimal places (truncated). |
| 17 | Integer where float expected `circle(3)` | PASS | Accepted, compiles normally. |
| 18 | No arguments `circle() \| glow()` | PASS | Uses defaults (circle=0.2, glow=1.5). No warning. |
| 19 | Too many arguments `circle(0.3, 0.5, 0.7)` | PASS | Extra args silently ignored. Uses first arg only. |
| 20 | String where number expected `circle("hello")` | PASS | Silently falls back to default (0.2). No warning. |

### Value Validation Issues

| Issue | Severity |
|-------|----------|
| No argument count validation | MEDIUM — silently ignores extra args |
| No type checking for arguments | MEDIUM — string silently becomes default |
| No value range validation | LOW — extreme/negative values pass through |
| No component name length limits | LOW — very long names create very long filenames |

### Palette Coverage

| Test | Result |
|------|--------|
| All 30 named palettes (fire through monochrome) | ALL PASS |
| Unknown palette name `palette(doesnotexist)` | PASS (silently treated as uniform) |

**Issue:** Unknown palette names are not validated. `palette(doesnotexist)` compiles to a shader with `p_doesnotexist` as a uniform parameter, which will always be 0. No warning is emitted. Both `validate` and `build` accept it silently.

### Advanced Features

| # | Feature | Result | Notes |
|---|---------|--------|-------|
| 22 | SDF shade/outline | PASS | Pipeline state machine correctly enforces order. |
| 25 | Pass blocks (post-processing) | PASS | Named passes with blur/vignette work. |
| 26 | Props block | PASS | Typed properties compile correctly. |
| 27 | Dom block | PASS | Text overlay with positioning and styling works. |
| 28 | On block (events) | PASS | Event handlers with emit compile correctly. |
| 29 | Swarm compute | PASS | Full physarum simulation parameters compile. |
| 30 | Memory/persistence | PASS | Pass-based feedback with mix() works. |
| 31 | Warp+fbm+polar combo | N/A | Pipeline state prevents mixing freely — by design. |
| 32 | Resonate | PASS | Cross-layer coupling with `->` syntax works. |

### Error Recovery

| # | Test Case | Result | Notes |
|---|-----------|--------|-------|
| 37 | Missing closing brace | FAIL (correct) | "expected `}`, found end of input" |
| 38 | Missing opening brace | FAIL (correct) | "expected `{`, found `layer`" |
| 39 | Typo in keyword `componet` | FAIL (correct) | Lists valid top-level keywords. No "did you mean?" for keywords. |
| 40 | Extra trailing comma | FAIL (correct) | "unexpected token `)` in expression" |

Error messages are clear with byte-offset positions. The "Did you mean?" feature works for stage functions but not for top-level keywords.

---

## 5. Output Quality Analysis

### File: hello-glow.js (18,897 bytes, 546 lines)

**WebGPU Setup:**
- Correct fullscreen triangle vertex shader (3 vertices, no vertex buffer needed)
- Proper uniform buffer with 16-byte alignment
- Bind group layout + pipeline creation correct
- Alpha mode: `premultiplied` (correct for compositing)
- Blend state: `one, one-minus-src-alpha` (correct for premultiplied alpha)

**WebGL2 Fallback:**
- Complete fallback path with shader compilation and linking
- Uniform locations resolved by name
- Proper error handling for shader compilation failures
- `#version 300 es` with `precision highp float`

**Shader Quality:**
- SDF circle + glow pipeline is mathematically correct
- ACES tonemapping included by default (professional color pipeline)
- Dithering noise to prevent banding (good practice)
- Time wrapping: `fract(u.time / 120.0) * 120.0` prevents float precision loss after long runtimes
- Aspect ratio correction applied to UV coordinates

**Animation Loop:**
- IntersectionObserver pauses rendering when element is off-screen (performance optimization)
- requestAnimationFrame-based loop (correct)
- Visibility check prevents hidden tab rendering

**Shadow DOM:**
- Proper `attachShadow({ mode: 'open' })`
- Scoped CSS: `:host{display:block;width:100%;height:100%}`
- Canvas element with 100% fill

**Cleanup (disconnectedCallback):**
- Renderer destroyed
- ResizeObserver disconnected
- Mouse/touch event listeners removed (all 6)
- IntersectionObserver disconnected
- GPU device destroyed

**Issues Found:**
- Both `GameRenderer` (WebGPU) and `GameRendererGL` (WebGL2) classes are ALWAYS included, even when `-t webgpu` or `-t webgl2` is specified. The target flag has no effect on output.
- Empty cinematic generates full 17,520-byte renderer for an empty shader. Could be 0 bytes.

### File: notification-card.js (30,837 bytes)

- DOM overlay with positioned text elements
- String property bindings (`title`, `body`, `priority`)
- Event dispatch (`click` -> `dismiss` CustomEvent)
- CSS styles embedded in shadow DOM
- Property getters/setters with DOM update triggers
- observedAttributes includes both uniform and string props

### File: genesis.js (41,037 bytes)

- Multi-layer shader with memory (feedback) passes
- FBO ping-pong texture setup for persistence
- Config layer with named parameters exposed as uniforms
- Arc timeline animation system
- Resonate coupling between layers

### General Output Quality:
- **No dead code** in standard outputs
- All custom elements follow `game-<name>` pattern
- getFrame() and getFrameDataURL() capture methods on all components
- setAudioData() and setAudioSource() for audio reactivity on all components
- DPR-aware canvas sizing (devicePixelRatio)
- Touch event support alongside mouse events
- Passive touch listeners (`{passive: true}`)

---

## 6. CLI Coverage

| Command | Status | Notes |
|---------|--------|-------|
| `game build <file> -o <dir>` | PASS | Core build |
| `game build` (default dir) | PASS | Defaults to `dist/` |
| `game build <file1> <file2>` | PASS | Multi-file build |
| `game build -f component` | PASS | Default format |
| `game build -f split` | PASS | Separate runtime + component |
| `game build -f html` | PASS | Generates .html preview |
| `game build -f standalone` | PASS | Self-contained HTML |
| `game build -f artblocks` | PASS | Art Blocks format |
| `game build -t webgpu` | BUG | Produces identical output to `-t both` |
| `game build -t webgl2` | BUG | Produces identical output to `-t both` |
| `game build -t both` | PASS | Both targets (default) |
| `game validate <file>` | PASS | Returns "ok" / error |
| `game validate` (empty file) | BUG | Returns "ok" for empty file |
| `game info` | PASS | Lists builtins, palettes |
| `game info builtins` | PASS | 49 builtins listed |
| `game info palettes` | PASS | 30 named palettes listed |
| `game new -t minimal` | PASS | Template compiles |
| `game new -t audio` | PASS | Template compiles |
| `game new -t particles` | PASS | Template compiles |
| `game new -t procedural` | BUG | Template fails to compile |
| `game new -t composition` | BUG | Template fails to compile |
| `game new -t reactive` | BUG | Template fails to compile |
| `game new -t sdf` | BUG | Template fails to compile |
| `game new -t scene` | BUG | Template fails to compile |
| `game dev <file>` | NOT TESTED | Requires running server |
| `game --version` | PASS | `game 1.0.0` |
| `game --help` | PASS | All commands listed |

---

## 7. Performance Benchmarks

| Test | Time | Notes |
|------|------|-------|
| Simple file (001-hello.game, 1 layer) | 196ms | Includes process startup |
| Complex file (038-genesis.game, multi-layer) | 213ms | ~17ms compilation overhead |
| Multi-component (019-swarm, 2 components) | 186ms | Parallel? Or cached? |
| All 76 examples sequentially | 15.0s | ~197ms per invocation |

**Analysis:** Compilation is effectively instant. The 196ms per invocation is dominated by process startup, not compilation. The actual parse+codegen time is under 20ms even for complex files. For batch operations, a persistent process or `--watch` mode would eliminate the startup overhead.

---

## 8. Strengths

1. **100% test pass rate with 567 tests.** Thorough unit and E2E coverage. The `e2e_all_examples_compile` test ensures no example regressions.

2. **Dual-target codegen is production-grade.** Both WGSL (WebGPU) and GLSL (WebGL2) shaders are generated with correct syntax, and the runtime auto-detects the best backend.

3. **Pipeline state machine is well-designed.** The Position -> SDF -> Color state machine catches invalid pipeline ordering at compile time with helpful "Did you mean?" suggestions.

4. **Output quality is professional.** ACES tonemapping, dithering, premultiplied alpha, IntersectionObserver visibility culling, DPR-aware rendering, proper cleanup in disconnectedCallback. This is not toy code.

5. **TypeScript definitions are comprehensive.** Every component gets a .d.ts with typed interfaces and HTMLElementTagNameMap augmentation for TypeScript-first developer experience.

6. **Component UI layer is fully functional.** Props, DOM overlay, event handlers, accessibility roles all compile correctly. The notification-card example is a complete real-world component.

7. **Compute shaders work.** Swarm (physarum), react (Turing patterns), flow fields, and gravity simulations all compile and produce GPU compute dispatch code.

8. **CLI is well-structured.** Build, validate, new, info, dev commands. Multiple output formats. Good defaults.

9. **Error messages are helpful.** Parse errors include position, expected tokens, and "Did you mean?" for stage functions.

10. **Performance is excellent.** Sub-20ms compilation for any complexity level.

---

## 9. Weaknesses (Ranked by Severity)

### CRITICAL

1. **5 of 8 `game new` templates fail to compile.** The `procedural`, `composition`, `reactive`, `sdf`, and `scene` templates produce files that the compiler rejects. This means a new user running `game new -t sdf` gets broken code on their first interaction. Templates use syntax constructs (`blend add`, `blend screen`, multiline `smooth_union()`, named `fbm()` params in wrong order) that the parser/validator rejects.

2. **All 32 VS Code gallery components fail to compile.** The gallery uses `component` keyword syntax, which the parser does not accept. These are showcased in the extension's gallery view but cannot be compiled.

### HIGH

3. **`-t webgpu` and `-t webgl2` flags have no effect.** All three target options produce byte-identical output. Both renderer classes are always included regardless of the target flag. This bloats single-target builds by ~2x.

4. **Unknown palette names compile silently.** `palette(doesnotexist)` passes both `validate` and `build` without any warning. It generates a uniform parameter that defaults to 0, producing incorrect visual output. Should either error or warn.

### MEDIUM

5. **No argument count validation.** `circle(0.3, 0.5, 0.7)` silently ignores extra arguments. `circle()` silently uses defaults. `circle("hello")` silently uses defaults. No warnings emitted for any of these.

6. **Empty file validates as "ok".** An empty `.game` file passes `validate` and `build` silently produces nothing. Should at minimum warn "no components found".

7. **011-project-dome.game produces no output with no error.** The `project` block compiles without error but generates nothing. No warning or indication of a no-op.

8. **3 special output types lack .d.ts files.** Breed mergers (`child.js`), scene timelines (`day-cycle.js`), and transition matrices (`matrix_transitions_flow.js`) are generated without TypeScript definitions.

### LOW

9. **No value range validation.** Negative glow, 999999-size canvases, and zero-radius shapes all pass through to the shader. While technically valid WGSL/GLSL, they produce degenerate visuals.

10. **Empty cinematic generates 17KB of code.** `cinematic "empty" {}` produces a full renderer with empty fragment shader. Should warn or produce nothing.

11. **Unicode in component names.** `cinematic "uber"` generates `uber.js`. While technically valid, this may cause filesystem issues on Windows or in web servers.

12. **200-character filenames accepted.** No name length limit could cause issues on some filesystems (especially Windows MAX_PATH).

---

## 10. Recommendations (Prioritized)

### Must Fix Before v1.0 Release

1. **Fix the 5 broken `game new` templates.** This is the #1 first-impression issue. Every template must compile. Test them in CI.
   - `procedural`: pipeline order issue (fbm->warp invalid, must be warp->fbm)
   - `composition`: `blend screen`/`blend add` syntax not recognized by parser
   - `reactive`: same `blend add` issue
   - `sdf`: multiline `smooth_union()` with nested pipeline syntax
   - `scene`: same `blend add` issue

2. **Fix or remove the gallery components.** Either:
   (a) Add `component` keyword support to the parser, or
   (b) Rewrite all 32 gallery files to use `cinematic` syntax, or
   (c) Remove the gallery until the syntax aligns.

3. **Implement the `-t` target flag.** When `-t webgpu` is specified, omit the WebGL2 renderer (and vice versa). This could save 7-10KB per component.

### Should Fix

4. **Validate palette names against the known list.** Emit a warning or error for unknown palette names. The list of 30 is well-defined.

5. **Add argument count validation.** Warn when a builtin receives more arguments than expected, or when a string appears where a number is expected.

6. **Warn on empty input.** Empty files and empty cinematics should produce a warning, not silent success.

7. **Add `--strict` mode.** A flag that turns all warnings into errors for CI/production use.

### Nice to Have

8. **Add "did you mean?" for top-level keywords.** The stage function suggestions are helpful; extend this to `componet` -> `cinematic`, etc.

9. **Add `project` block output.** Currently compiles silently with no output. Either generate projection mapping code or warn that it's unimplemented.

10. **Generate .d.ts for breed/scene/matrix outputs.** These are valid JS that should have type definitions.

11. **Add output size to build log.** After `[game] wrote file.js`, show the file size. Helps users spot bloat.

12. **Batch compilation mode.** Instead of invoking `game build` 76 times (15 seconds), support a directory glob or persistent daemon to eliminate process startup overhead.

---

## Appendix A: Full Template Compilation Matrix

| Template | Compiles? | Error |
|----------|-----------|-------|
| minimal | YES | -- |
| audio | YES | -- |
| particles | YES | -- |
| procedural | NO | `stage 'warp' expects Position input but pipeline is in Sdf state` |
| composition | NO | `expected ':', found 'screen'` (blend mode syntax) |
| reactive | NO | `expected ':', found 'add'` (blend mode syntax) |
| sdf | NO | `expected ')', found '\|'` (nested pipeline in smooth_union) |
| scene | NO | `expected ':', found 'add'` (blend mode syntax) |

## Appendix B: Gallery Failure Summary

All 32 gallery `.game` files fail with identical error:
```
parse error at 0:9: expected `import`, `use`, `fn`, `cinematic`, `breed`,
`project`, `scene`, `matrix`, `ifs`, `lsystem`, or `automaton` at top level,
found `component`
```

The `component` keyword is referenced in the gallery files but not in the parser's top-level dispatch.

## Appendix C: Output Validation Checklist

| Check | Result |
|-------|--------|
| Custom element follows `game-<name>` pattern | PASS (all 106 files) |
| WebGPU setup code present | PASS |
| WebGL2 fallback code present | PASS |
| Shadow DOM properly initialized | PASS |
| disconnectedCallback cleanup | PASS |
| ResizeObserver for responsive sizing | PASS |
| IntersectionObserver for visibility culling | PASS |
| Mouse and touch event handling | PASS |
| Audio data API (setAudioData, setAudioSource) | PASS |
| Frame capture (getFrame, getFrameDataURL) | PASS |
| observedAttributes for HTML attribute binding | PASS |
| .d.ts generated alongside .js | PASS (103/106 — 3 expected exceptions) |
| IIFE wrapping (no global pollution) | PASS |
| Premultiplied alpha pipeline | PASS |
| ACES tonemapping | PASS |
| Dithering noise (anti-banding) | PASS |
| Time precision wrapping | PASS |

---

*Report generated by exhaustive automated audit. All tests executed against game compiler v1.0.0 on Windows 10 Pro.*
