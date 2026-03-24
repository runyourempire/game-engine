# Changelog

All notable changes to the GAME compiler will be documented in this file.

## [1.0.0] - 2026-03-25

### Added
- **VS Code Extension v0.5.0** — live preview, parameter tuner, component gallery (32 components), AI generation (Claude API), one-click export (5 formats)
- **30 named palettes** — fire, ocean, neon, aurora, sunset, ice, ember, lava, magma, inferno, plasma, electric, cyber, matrix, forest, moss, earth, desert, blood, rose, candy, royal, deep_sea, coral, arctic, twilight, vapor, gold, silver, monochrome
- **8 project templates** — minimal, audio, particles, procedural, composition, reactive, sdf, scene
- **5 output formats** — component, split, html, standalone, artblocks
- **Dual-target rendering** — WebGPU primary with WebGL2 fallback, or single-target via `-t` flag
- **Component UI layer** — props, dom overlay, event handlers, ARIA roles
- **Compute shaders** — swarm (physarum), react (Turing patterns), flow fields, gravity
- **SDF boolean operations** — union, subtract, intersect, smooth_union, smooth_subtract, smooth_intersect, xor
- **Post-processing passes** — FBO chain, ping-pong textures, multi-pass rendering
- **Arc animations** — keyframe timeline system with easing functions
- **Resonate coupling** — cross-layer parameter linking
- **Memory persistence** — per-layer frame feedback with configurable decay
- **Listen blocks** — audio-reactive parameters (bass, mid, treble, energy, beat)
- **Framework wrappers** — React, Vue, Svelte (unpublished)
- **WASM target** — browser compilation API (build pending)
- **LSP server** — completions, hover, diagnostics, go-to-definition
- **49 builtins** across 5 categories (SDF generators, transforms, bridges, color processors, SDF modifiers)
- **Palette name validation** — unknown palette names produce clear errors
- **"Did you mean?" suggestions** — for both builtins and top-level keywords
- **BOM handling** — UTF-8 BOM stripped automatically
- **Empty file warnings** — clear feedback instead of silent success
- **Directory path detection** — helpful error instead of OS-level message
- **File extension warnings** — hints when non-.game files are compiled
- **Arc parser fix** — multi-entry arc blocks without easing no longer misparse
- **Duplicate layer detection** — same-name layers produce validation errors
- **FSL-1.1-Apache-2.0 license** — source-available, converts to Apache 2.0 after 2 years

### Performance
- Sub-20ms compilation for any complexity
- 42-45% output size reduction with `-t webgpu` or `-t webgl2` (single renderer)
- IntersectionObserver visibility culling in generated components
- DPR-aware canvas sizing

### Output Quality
- ACES tonemapping by default
- Dithering noise (anti-banding)
- Premultiplied alpha pipeline
- Time precision wrapping (prevents float degradation after long runtimes)
- Full cleanup in disconnectedCallback (renderer, observers, listeners)
- Shadow DOM isolation
- Touch event support with passive listeners

### Testing
- 567 Rust tests (100% pass rate)
- 78 example files (all compile)
- 32 gallery components (all compile)
- 8 templates (all compile)
- E2E test ensures no example regressions
