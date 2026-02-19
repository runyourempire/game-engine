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
                    +-----------+-----------+
                    |  Compiled Artifacts   |
                    |  - WGSL shaders       |
                    |  - Uniform layouts    |
                    |  - Timeline data      |
                    |  - Audio config       |
                    |  - Resonance graph    |
                    +-----------+-----------+
                                |
                    +-----------+-----------+
                    |    GAME Runtime       |
                    |  (Rust/WASM + WebGPU) |
                    +-----------+-----------+
                          |    |    |
                   +------+    |    +------+
                   |           |           |
              Audio FFT   GPU Render   Input Events
              Analyzer    Pipeline     Handler
                   |           |           |
                   +-----+-----+-----------+
                         |
                    Uniform Buffer
                    (params + mods + arcs + resonance)
                         |
                      Screen
```

## The Compiler

### Phase 1: Lexer

Tokenizes `.game` source into:
- Keywords: `cinematic`, `layer`, `lens`, `arc`, `react`, `resonate`, `define`, `import`, `fn`
- Operators: `|`, `~`, `->`, `:`, `*`, `+`, `-`, `/`, `>`, `<`
- Literals: numbers, strings, identifiers, timestamps, colors
- Structural: `{`, `}`, `[`, `]`, `(`, `)`, `,`
- Comments: `#` to end of line

### Phase 2: Parser

Builds an AST:

```
Cinematic
  ├── Properties (resolution, audio, ...)
  ├── Defines (reusable functions)
  ├── Layers
  │     ├── name
  │     ├── fn: PipeChain [Op, Op, Op, ...]
  │     ├── params: [(name, base_value, modulation?), ...]
  │     └── depth: base | heart | top
  ├── Lenses
  │     ├── name
  │     ├── mode: raymarch | volume | flat | particles
  │     ├── fields: [layer_ref, ...]
  │     ├── camera: CameraConfig
  │     ├── lighting: LightingConfig
  │     └── post: [PostEffect, ...]
  ├── Arc
  │     └── moments: [(timestamp, name, transitions), ...]
  ├── Resonance
  │     └── bindings: [(target, source_expr, damping), ...]
  └── React
        └── reactions: [(signal, trigger?, action), ...]
```

### Phase 3: Resolver

Validates and enriches the AST:

1. **Reference resolution** — verify all `layer.param` references in arcs, resonance, and modulations point to real declarations
2. **Type checking** — ensure modulation sources are numeric, color operations get colors, vectors get vectors
3. **Resonance cycle detection** — find feedback loops, verify damping is applied, compute evaluation order (topological sort with cycle-breaking)
4. **Define inlining** — expand `define` calls into their pipe chains
5. **Import resolution** — load and merge imported `.game` files
6. **Depth ordering** — resolve layer composition order from `depth` annotations

### Phase 4: Code Generation

The heart of the compiler. For each lens, generates:

**A. WGSL Shader Code**

Merges all pipe chain operations for the lens's fields into a single WGSL fragment shader:

```
layer terrain:
  fn: fbm(p * scale, octaves: 6) | shade(albedo: gold)

  Compiles to WGSL:

  fn sdf_terrain(p: vec3f) -> f32 {
    return fbm(p * uniforms.terrain_scale, 6);
  }

  fn shade_terrain(p: vec3f, normal: vec3f) -> vec4f {
    let albedo = vec3f(0.831, 0.686, 0.216); // gold
    return pbr(albedo, normal, uniforms.light_dir, ...);
  }
```

SDF operations compose via mathematical operators:
- `union(a, b)` → `min(sdf_a(p), sdf_b(p))`
- `smooth_union(a, b, k)` → smooth min function
- `intersect(a, b)` → `max(sdf_a(p), sdf_b(p))`
- `subtract(a, b)` → `max(sdf_a(p), -sdf_b(p))`

Domain operations transform the input position:
- `translate(x, y, z)` → `p - vec3(x, y, z)`
- `rotate(angle, axis)` → rotation matrix applied to `p`
- `repeat(spacing)` → `fract(p / spacing) * spacing - spacing * 0.5`
- `twist(amount)` → rotate p.xz by `p.y * amount`

**B. Uniform Buffer Layout**

Every modulated parameter becomes a uniform:

```wgsl
struct Uniforms {
  time: f32,
  audio_bass: f32,
  audio_mid: f32,
  audio_treble: f32,
  audio_energy: f32,
  mouse_x: f32,
  mouse_y: f32,
  // ... per-layer params
  terrain_scale: f32,       // base: 2.0
  terrain_persistence: f32, // base: 0.5
  crystals_density: f32,    // base: 4.0
  sparks_freq: f32,         // base: 2.0
  sparks_count: f32,        // base: 5000
  // ... arc interpolation state
  exposure: f32,
  // ... resolution and aspect
  resolution: vec2f,
};
```

**C. Timeline Data**

Arc moments compile to interpolation keyframes:

```
[
  { time: 0.0,  param: "terrain.scale", value: 0.5, ease: null },
  { time: 20.0, param: "terrain.scale", value: 2.0, ease: "cubic_out", duration: 15.0 },
  ...
]
```

**D. Audio Analysis Config**

Which FFT bands to extract, beat detection parameters:

```
{
  fft_size: 2048,
  bands: {
    bass: { low: 20, high: 250 },
    mid: { low: 250, high: 4000 },
    treble: { low: 4000, high: 20000 },
  },
  beat_detection: true,
  smoothing: 0.8,
}
```

**E. Resonance Graph**

The cross-modulation topology:

```
{
  nodes: ["fire.freq", "ice.density", "fire.brightness", "ice.brightness"],
  edges: [
    { from: "ice.brightness", to: "fire.freq", weight: 2.0 },
    { from: "fire.brightness", to: "ice.density", weight: -1.0 },
  ],
  damping: 0.95,
  eval_order: ["fire.brightness", "ice.brightness", "fire.freq", "ice.density"],
}
```

## The Runtime

### Frame Loop (Audio-Clock Driven)

```
function frame() {
  // 1. Time source: audio clock, NOT requestAnimationFrame
  const t = audioContext.currentTime - startTime;

  // 2. Audio analysis
  analyser.getFloatFrequencyData(frequencyData);
  const bass = averageBand(frequencyData, 20, 250);
  const mid = averageBand(frequencyData, 250, 4000);
  const treble = averageBand(frequencyData, 4000, 20000);
  const energy = averageBand(frequencyData, 20, 20000);
  const beat = detectBeat(energy, beatHistory);

  // 3. Input state
  const mouseX = inputState.mouseX;
  const mouseY = inputState.mouseY;
  const mouseVelocity = inputState.mouseVelocity;

  // 4. Resonance evaluation (topological order)
  for (const node of resonanceGraph.evalOrder) {
    const incoming = resonanceGraph.edgesTo(node);
    let modulation = 0;
    for (const edge of incoming) {
      modulation += currentValues[edge.from] * edge.weight;
    }
    currentValues[node] = currentValues[node] * damping + modulation;
  }

  // 5. Arc interpolation
  for (const param of arcTimeline.params) {
    const arcValue = interpolateArc(param, t);
    if (arcValue !== null) baseValues[param] = arcValue;
  }

  // 6. Modulation evaluation
  for (const param of modulatedParams) {
    const base = baseValues[param.name];
    const modValue = evaluateModulation(param.modExpr, signals);
    uniforms[param.name] = base + modValue + resonanceValues[param.name];
  }

  // 7. Upload uniforms
  device.queue.writeBuffer(uniformBuffer, 0, uniformData);

  // 8. Render passes
  const commandEncoder = device.createCommandEncoder();
  for (const lens of lenses) {
    const pass = commandEncoder.beginRenderPass(lens.renderPassDescriptor);
    pass.setPipeline(lens.pipeline);
    pass.setBindGroup(0, lens.bindGroup);
    pass.draw(4, 1, 0, 0); // fullscreen quad
    pass.end();
  }

  // 9. Composite + global post-processing
  compositePass(commandEncoder, lensOutputs, globalPostEffects);

  // 10. Submit
  device.queue.submit([commandEncoder.finish()]);
  requestAnimationFrame(frame);
}
```

### Shader Compilation Strategy

To avoid startup stalls:

1. **On load:** Parse `.game`, compile ALL shaders in parallel using `device.createShaderModuleAsync()`
2. **Show progress:** Render a simple loading animation (itself a minimal shader) while compilation proceeds
3. **Cache:** Store compiled shader modules in IndexedDB keyed by shader source hash. On reload, skip compilation for unchanged shaders.
4. **Hot reload:** When a `.game` file changes (dev mode), diff the AST. Only recompile shaders for changed layers. Swap pipeline objects without interrupting playback.

### Floating-Point Time Safety

Time values are kept small to avoid precision loss:

```wgsl
// In the runtime, not the user's .game file
fn safe_time(raw_time: f32, period: f32) -> f32 {
  return fract(raw_time / period) * period;
}

// Expose multiple time scales to shaders
uniforms.time_fast = safe_time(t, 10.0);   // wraps every 10s
uniforms.time_slow = safe_time(t, 120.0);  // wraps every 2min
uniforms.time_raw = t;                      // for arc interpolation only
```

Users never think about this. The compiler maps `time` references to the appropriate safe time scale based on context.

## Distribution Model

### Browser (Primary)

```
your-cinematic/
  index.html          # Minimal HTML shell (~200 bytes)
  game_runtime.wasm   # Compiled Rust runtime (~500KB gzipped)
  cinematic.game      # The scene description (~2-50KB)
  audio.ogg           # The audio track
```

Total: under 2MB for a complete interactive cinematic experience. Share a URL, it runs.

### Native (Secondary)

Same Rust runtime compiled as a native binary instead of WASM. Uses wgpu's Vulkan/Metal/DX12 backends directly. Higher performance ceiling for complex scenes.

```
game-player cinematic.game           # Play a cinematic
game-player cinematic.game --export  # Export to video
game-player cinematic.game --dev     # Hot-reload mode
```

### Embeddable (Tertiary)

The runtime as a library, embeddable in other applications:

```rust
use game_engine::{Cinematic, Runtime};

let cinematic = Cinematic::load("scene.game")?;
let runtime = Runtime::new(gpu_device, audio_context);
runtime.play(cinematic);
```

This is the path for eventual integration with other tools.

## Performance Budget

### Target: 60fps at 1080p on integrated GPU

| System | Budget | Rationale |
|--------|--------|-----------|
| Audio FFT | <0.5ms/frame | 2048-point FFT is ~0.1ms on modern CPUs |
| Resonance eval | <0.1ms/frame | Typically <20 nodes, simple arithmetic |
| Arc interpolation | <0.1ms/frame | Binary search + lerp, trivially fast |
| Modulation eval | <0.1ms/frame | Simple arithmetic per parameter |
| Uniform upload | <0.2ms/frame | Single buffer write, <4KB typical |
| Shader execution | <14ms/frame | The real budget. At 1080p, ~480 ALU ops/fragment |
| Composition | <1ms/frame | Fullscreen blit + post-processing |
| **Total** | **<16.6ms** | **= 60fps** |

The shader execution is 85% of the frame budget. This is correct — the GPU should be doing the work.

### Adaptive Quality

When framerate drops below target:
1. **First:** Reduce render resolution (render at 0.75x, upscale)
2. **Second:** Reduce noise octaves (6 -> 4 -> 2)
3. **Third:** Reduce raymarching steps (64 -> 32 -> 16)
4. **Fourth:** Disable post-processing (bloom, chromatic, grain)
5. **Never:** Skip frames or drop audio sync
