//! Particles block codegen — general-purpose GPU-accelerated particle system.
//!
//! Each particle has position, velocity, life, and max_life. On compute dispatch:
//! update position += velocity*dt, apply gravity, age life, respawn when life <= 0.
//! Renders into a storage buffer that the fragment shader samples as a density texture.
//!
//! ```game
//! particles {
//!   count: 5000
//!   emit: center
//!   lifetime: 3.0
//!   speed: 0.5
//!   spread: 360
//!   gravity: -0.2
//!   size: 2.0
//!   fade: true
//!   color: fire
//! }
//! ```
//!
//! Generates:
//! - WGSL compute shader for particle simulation (update + respawn)
//! - WGSL compute shader for rasterizing particles into a density texture
//! - `GameParticleSim` JS class for GPU dispatch

use crate::ast::{EmitMode, ParticlesBlock};

/// Generate WGSL compute shader for particle simulation.
///
/// Each particle: pos(vec2), vel(vec2), life(f32), max_life(f32).
/// On dispatch: age particles, apply gravity, move, respawn dead particles.
pub fn generate_sim_wgsl(particles: &ParticlesBlock) -> String {
    let mut s = String::with_capacity(4096);

    // Structs
    s.push_str("struct Particle {\n");
    s.push_str("    pos: vec2<f32>,\n");
    s.push_str("    vel: vec2<f32>,\n");
    s.push_str("    life: f32,\n");
    s.push_str("    max_life: f32,\n");
    s.push_str("    size: f32,\n");
    s.push_str("    _pad: f32,\n");
    s.push_str("};\n\n");

    s.push_str("struct ParticleParams {\n");
    s.push_str("    dt: f32,\n");
    s.push_str("    gravity: f32,\n");
    s.push_str("    speed: f32,\n");
    s.push_str("    spread: f32,\n");
    s.push_str("    lifetime: f32,\n");
    s.push_str("    count: u32,\n");
    s.push_str("    time: f32,\n");
    s.push_str("    size: f32,\n");
    s.push_str("    emit_mode: u32,\n");
    s.push_str("    emit_x: f32,\n");
    s.push_str("    emit_y: f32,\n");
    s.push_str("    emit_radius: f32,\n");
    s.push_str("};\n\n");

    // Bindings
    s.push_str("@group(0) @binding(0) var<uniform> params: ParticleParams;\n");
    s.push_str(
        "@group(0) @binding(1) var<storage, read_write> particles: array<Particle>;\n\n",
    );

    // Hash for pseudo-random (deterministic per-particle per-frame)
    s.push_str("fn hash(seed: u32) -> f32 {\n");
    s.push_str("    var x = seed;\n");
    s.push_str("    x = x ^ (x >> 16u);\n");
    s.push_str("    x = x * 0x45d9f3bu;\n");
    s.push_str("    x = x ^ (x >> 16u);\n");
    s.push_str("    x = x * 0x45d9f3bu;\n");
    s.push_str("    x = x ^ (x >> 16u);\n");
    s.push_str("    return f32(x) / 4294967295.0;\n");
    s.push_str("}\n\n");

    // Emit position based on mode
    s.push_str("fn emit_position(idx: u32, t: f32) -> vec2<f32> {\n");
    s.push_str("    let seed = idx * 7919u + u32(t * 1000.0);\n");
    s.push_str("    // emit_mode: 0=center, 1=random, 2=ring, 3=point\n");
    s.push_str("    if (params.emit_mode == 1u) {\n");
    s.push_str("        // Random position in [-1, 1]\n");
    s.push_str("        return vec2<f32>(hash(seed) * 2.0 - 1.0, hash(seed + 1u) * 2.0 - 1.0);\n");
    s.push_str("    } else if (params.emit_mode == 2u) {\n");
    s.push_str("        // Ring: emit on circle of radius emit_radius\n");
    s.push_str("        let angle = hash(seed) * 6.28318;\n");
    s.push_str(
        "        return vec2<f32>(cos(angle) * params.emit_radius, sin(angle) * params.emit_radius);\n",
    );
    s.push_str("    } else if (params.emit_mode == 3u) {\n");
    s.push_str("        // Point: emit at (emit_x, emit_y) with slight jitter\n");
    s.push_str("        let jitter = vec2<f32>((hash(seed) - 0.5) * 0.02, (hash(seed + 1u) - 0.5) * 0.02);\n");
    s.push_str("        return vec2<f32>(params.emit_x, params.emit_y) + jitter;\n");
    s.push_str("    }\n");
    s.push_str("    // Center: emit at origin with slight jitter\n");
    s.push_str("    return vec2<f32>((hash(seed) - 0.5) * 0.02, (hash(seed + 1u) - 0.5) * 0.02);\n");
    s.push_str("}\n\n");

    // Emit velocity based on spread
    s.push_str("fn emit_velocity(idx: u32, t: f32) -> vec2<f32> {\n");
    s.push_str("    let seed = idx * 6271u + u32(t * 1000.0) + 100u;\n");
    s.push_str("    let spread_rad = params.spread * 0.01745329;\n"); // deg to rad
    s.push_str("    let angle = (hash(seed) - 0.5) * spread_rad;\n");
    s.push_str("    let spd = params.speed * (0.5 + hash(seed + 1u) * 0.5);\n");
    s.push_str("    return vec2<f32>(cos(angle), sin(angle)) * spd;\n");
    s.push_str("}\n\n");

    // Compute entry — particle update
    let fade_val = if particles.fade { 1u32 } else { 0u32 };
    s.push_str("@compute @workgroup_size(64)\n");
    s.push_str("fn cs_particles(@builtin(global_invocation_id) gid: vec3<u32>) {\n");
    s.push_str("    let idx = gid.x;\n");
    s.push_str("    if (idx >= params.count) { return; }\n\n");

    s.push_str("    var p = particles[idx];\n\n");

    // Age particle
    s.push_str("    p.life -= params.dt;\n\n");

    // Respawn dead particles
    s.push_str("    if (p.life <= 0.0) {\n");
    s.push_str("        p.pos = emit_position(idx, params.time);\n");
    s.push_str("        p.vel = emit_velocity(idx, params.time);\n");
    s.push_str("        p.life = params.lifetime * (0.5 + hash(idx * 3571u + u32(params.time * 100.0)) * 0.5);\n");
    s.push_str("        p.max_life = p.life;\n");
    s.push_str("        p.size = params.size;\n");
    s.push_str("    }\n\n");

    // Apply gravity (Y-axis)
    s.push_str("    p.vel.y += params.gravity * params.dt;\n\n");

    // Move particle
    s.push_str("    p.pos += p.vel * params.dt;\n\n");

    // Write back
    s.push_str("    particles[idx] = p;\n");
    s.push_str("}\n");

    // Embed fade flag as a comment for the runtime to reference
    s.push_str(&format!("\n// FADE: {fade_val}\n"));

    s
}

/// Generate WGSL compute shader for rasterizing particles into a density/color texture.
///
/// Writes RGBA values into a storage buffer (one vec4 per texel).
/// Each particle contributes a soft radial falloff to nearby pixels.
pub fn generate_raster_wgsl(particles: &ParticlesBlock) -> String {
    let mut s = String::with_capacity(3072);

    s.push_str("struct Particle {\n");
    s.push_str("    pos: vec2<f32>,\n");
    s.push_str("    vel: vec2<f32>,\n");
    s.push_str("    life: f32,\n");
    s.push_str("    max_life: f32,\n");
    s.push_str("    size: f32,\n");
    s.push_str("    _pad: f32,\n");
    s.push_str("};\n\n");

    s.push_str("struct RasterParams {\n");
    s.push_str("    width: u32,\n");
    s.push_str("    height: u32,\n");
    s.push_str("    count: u32,\n");
    s.push_str("    fade: u32,\n");
    s.push_str("};\n\n");

    s.push_str("@group(0) @binding(0) var<uniform> params: RasterParams;\n");
    s.push_str("@group(0) @binding(1) var<storage, read> particles: array<Particle>;\n");
    s.push_str(
        "@group(0) @binding(2) var<storage, read_write> texture: array<f32>;\n\n",
    );

    // Clear + accumulate in one pass: each invocation processes one particle
    // and splats it into the texture buffer
    s.push_str("@compute @workgroup_size(64)\n");
    s.push_str("fn cs_raster(@builtin(global_invocation_id) gid: vec3<u32>) {\n");
    s.push_str("    let idx = gid.x;\n");
    s.push_str("    if (idx >= params.count) { return; }\n\n");

    s.push_str("    let p = particles[idx];\n");
    s.push_str("    if (p.life <= 0.0) { return; }\n\n");

    // Compute alpha based on fade
    let fade_val = if particles.fade { 1u32 } else { 0u32 };
    s.push_str(&format!("    var alpha = 1.0;\n"));
    s.push_str(&format!(
        "    if ({fade_val}u == 1u) {{ alpha = clamp(p.life / p.max_life, 0.0, 1.0); }}\n\n"
    ));

    // Convert pos from [-1,1] to pixel coords
    s.push_str("    let px = (p.pos.x * 0.5 + 0.5) * f32(params.width);\n");
    s.push_str("    let py = (p.pos.y * 0.5 + 0.5) * f32(params.height);\n");
    s.push_str("    let radius = p.size;\n");
    s.push_str("    let ri = i32(ceil(radius));\n\n");

    // Splat with soft radial falloff
    s.push_str("    for (var dy: i32 = -ri; dy <= ri; dy = dy + 1) {\n");
    s.push_str("        for (var dx: i32 = -ri; dx <= ri; dx = dx + 1) {\n");
    s.push_str("            let tx = i32(px) + dx;\n");
    s.push_str("            let ty = i32(py) + dy;\n");
    s.push_str(
        "            if (tx < 0 || tx >= i32(params.width) || ty < 0 || ty >= i32(params.height)) { continue; }\n",
    );
    s.push_str(
        "            let dist = sqrt(f32(dx * dx + dy * dy));\n",
    );
    s.push_str("            if (dist > radius) { continue; }\n");
    s.push_str("            let falloff = 1.0 - (dist / radius);\n");
    s.push_str(
        "            let texIdx = u32(ty) * params.width + u32(tx);\n",
    );
    s.push_str("            // Accumulate particle density\n");
    s.push_str("            texture[texIdx] += falloff * alpha;\n");
    s.push_str("        }\n");
    s.push_str("    }\n");
    s.push_str("}\n");

    s
}

/// Generate JavaScript runtime for particle system GPU dispatch.
///
/// Mirrors the `GameSwarmSim` pattern: constructor → init() → dispatch(dt).
/// Manages particle buffer, sim compute pipeline, and raster compute pipeline.
pub fn generate_particles_runtime_js(
    particles: &ParticlesBlock,
    width: u32,
    height: u32,
) -> String {
    let mut s = String::with_capacity(6144);
    let count = particles.count;
    let lifetime = particles.lifetime;
    let speed = particles.speed;
    let spread = particles.spread;
    let gravity = particles.gravity;
    let size = particles.size;
    let fade_u32: u32 = if particles.fade { 1 } else { 0 };

    // Emit mode encoding: 0=center, 1=random, 2=ring, 3=point
    let (emit_mode, emit_x, emit_y, emit_radius) = match &particles.emit {
        EmitMode::Center => (0u32, 0.0, 0.0, 0.0),
        EmitMode::Random => (1, 0.0, 0.0, 0.0),
        EmitMode::Ring(r) => (2, 0.0, 0.0, *r),
        EmitMode::Point(x, y) => (3, *x, *y, 0.0),
    };

    s.push_str("class GameParticleSim {\n");
    s.push_str(&format!(
        "  constructor(device, simCode, rasterCode) {{ this._count = {count}; this._w = {width}; this._h = {height}; this._device = device; this._simCode = simCode; this._rasterCode = rasterCode; }}\n\n"
    ));

    s.push_str("  async init() {\n");
    s.push_str("    const device = this._device;\n");

    // Sim pipeline
    s.push_str("    const simModule = device.createShaderModule({ code: this._simCode });\n");
    s.push_str("    this._simPipeline = device.createComputePipeline({\n");
    s.push_str("      layout: 'auto',\n");
    s.push_str("      compute: { module: simModule, entryPoint: 'cs_particles' },\n");
    s.push_str("    });\n\n");

    // Raster pipeline
    s.push_str(
        "    const rasterModule = device.createShaderModule({ code: this._rasterCode });\n",
    );
    s.push_str("    this._rasterPipeline = device.createComputePipeline({\n");
    s.push_str("      layout: 'auto',\n");
    s.push_str("      compute: { module: rasterModule, entryPoint: 'cs_raster' },\n");
    s.push_str("    });\n\n");

    // Buffers
    // Particle struct: vec2 pos + vec2 vel + f32 life + f32 max_life + f32 size + f32 pad = 8 floats = 32 bytes
    s.push_str("    const particleSize = 32;\n");
    s.push_str("    this._particleBuf = device.createBuffer({ size: this._count * particleSize, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST });\n");

    // Sim params: 12 floats/u32s = 48 bytes
    s.push_str("    this._simParamBuf = device.createBuffer({ size: 48, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });\n");

    // Raster params: 4 u32s = 16 bytes
    s.push_str("    this._rasterParamBuf = device.createBuffer({ size: 16, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });\n");

    // Texture buffer (density, one f32 per texel)
    s.push_str("    const texSize = this._w * this._h * 4;\n");
    s.push_str("    this._texBuf = device.createBuffer({ size: texSize, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST });\n\n");

    // Initialize particles with staggered lifetimes so they don't all spawn at once
    s.push_str("    const init = new Float32Array(this._count * 8);\n");
    s.push_str("    for (let i = 0; i < this._count; i++) {\n");
    s.push_str("      const base = i * 8;\n");
    s.push_str("      init[base] = 0; init[base+1] = 0;       // pos\n");
    s.push_str("      init[base+2] = 0; init[base+3] = 0;     // vel\n");
    s.push_str(&format!(
        "      init[base+4] = -Math.random() * {};            // life (negative = will respawn immediately)\n",
        lifetime
    ));
    s.push_str(&format!("      init[base+5] = {};                              // max_life\n", lifetime));
    s.push_str(&format!("      init[base+6] = {};                              // size\n", size));
    s.push_str("      init[base+7] = 0;                                // pad\n");
    s.push_str("    }\n");
    s.push_str("    device.queue.writeBuffer(this._particleBuf, 0, init);\n\n");

    // Write raster params (static)
    s.push_str(&format!(
        "    const rp = new Uint32Array([this._w, this._h, this._count, {}]);\n",
        fade_u32
    ));
    s.push_str("    device.queue.writeBuffer(this._rasterParamBuf, 0, rp);\n");
    s.push_str("    this._time = 0;\n");
    s.push_str("  }\n\n");

    // Dispatch
    s.push_str("  dispatch(dt) {\n");
    s.push_str("    this._time += dt;\n");
    s.push_str("    const device = this._device;\n\n");

    // Write sim params
    s.push_str("    const p = new ArrayBuffer(48);\n");
    s.push_str("    const f = new Float32Array(p); const u = new Uint32Array(p);\n");
    s.push_str("    f[0] = dt;\n");
    s.push_str(&format!("    f[1] = {};  // gravity\n", gravity));
    s.push_str(&format!("    f[2] = {};  // speed\n", speed));
    s.push_str(&format!("    f[3] = {};  // spread (degrees)\n", spread));
    s.push_str(&format!("    f[4] = {};  // lifetime\n", lifetime));
    s.push_str("    u[5] = this._count;\n");
    s.push_str("    f[6] = this._time;\n");
    s.push_str(&format!("    f[7] = {};  // size\n", size));
    s.push_str(&format!("    u[8] = {};  // emit_mode\n", emit_mode));
    s.push_str(&format!("    f[9] = {};  // emit_x\n", emit_x));
    s.push_str(&format!("    f[10] = {}; // emit_y\n", emit_y));
    s.push_str(&format!("    f[11] = {}; // emit_radius\n", emit_radius));
    s.push_str("    device.queue.writeBuffer(this._simParamBuf, 0, p);\n\n");

    // Clear texture buffer
    s.push_str("    const clearData = new Float32Array(this._w * this._h);\n");
    s.push_str("    device.queue.writeBuffer(this._texBuf, 0, clearData);\n\n");

    s.push_str("    const enc = device.createCommandEncoder();\n\n");

    // Sim pass
    s.push_str("    const simBG = device.createBindGroup({\n");
    s.push_str("      layout: this._simPipeline.getBindGroupLayout(0),\n");
    s.push_str("      entries: [\n");
    s.push_str("        { binding: 0, resource: { buffer: this._simParamBuf } },\n");
    s.push_str("        { binding: 1, resource: { buffer: this._particleBuf } },\n");
    s.push_str("      ],\n");
    s.push_str("    });\n");
    s.push_str("    const sp = enc.beginComputePass();\n");
    s.push_str("    sp.setPipeline(this._simPipeline);\n");
    s.push_str("    sp.setBindGroup(0, simBG);\n");
    s.push_str("    sp.dispatchWorkgroups(Math.ceil(this._count / 64));\n");
    s.push_str("    sp.end();\n\n");

    // Raster pass
    s.push_str("    const rasterBG = device.createBindGroup({\n");
    s.push_str("      layout: this._rasterPipeline.getBindGroupLayout(0),\n");
    s.push_str("      entries: [\n");
    s.push_str("        { binding: 0, resource: { buffer: this._rasterParamBuf } },\n");
    s.push_str("        { binding: 1, resource: { buffer: this._particleBuf } },\n");
    s.push_str("        { binding: 2, resource: { buffer: this._texBuf } },\n");
    s.push_str("      ],\n");
    s.push_str("    });\n");
    s.push_str("    const rp2 = enc.beginComputePass();\n");
    s.push_str("    rp2.setPipeline(this._rasterPipeline);\n");
    s.push_str("    rp2.setBindGroup(0, rasterBG);\n");
    s.push_str("    rp2.dispatchWorkgroups(Math.ceil(this._count / 64));\n");
    s.push_str("    rp2.end();\n\n");

    s.push_str("    device.queue.submit([enc.finish()]);\n");
    s.push_str("  }\n\n");

    // Accessors
    s.push_str("  get textureBuffer() { return this._texBuf; }\n");
    s.push_str("  get particleBuffer() { return this._particleBuf; }\n");
    s.push_str(&format!(
        "  get particleCount() {{ return {}; }}\n",
        count
    ));
    s.push_str("  get width() { return this._w; }\n");
    s.push_str("  get height() { return this._h; }\n");
    s.push_str("}\n");

    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    fn make_particles() -> ParticlesBlock {
        ParticlesBlock {
            count: 5000,
            emit: EmitMode::Center,
            lifetime: 3.0,
            speed: 0.5,
            spread: 360.0,
            gravity: -0.2,
            size: 2.0,
            fade: true,
            color: "fire".to_string(),
        }
    }

    #[test]
    fn sim_shader_has_entry_point() {
        let wgsl = generate_sim_wgsl(&make_particles());
        assert!(wgsl.contains("fn cs_particles"));
        assert!(wgsl.contains("@compute @workgroup_size(64)"));
    }

    #[test]
    fn sim_shader_has_particle_struct() {
        let wgsl = generate_sim_wgsl(&make_particles());
        assert!(wgsl.contains("struct Particle"));
        assert!(wgsl.contains("pos: vec2<f32>"));
        assert!(wgsl.contains("vel: vec2<f32>"));
        assert!(wgsl.contains("life: f32"));
        assert!(wgsl.contains("max_life: f32"));
    }

    #[test]
    fn sim_shader_has_respawn() {
        let wgsl = generate_sim_wgsl(&make_particles());
        assert!(wgsl.contains("if (p.life <= 0.0)"));
        assert!(wgsl.contains("emit_position"));
        assert!(wgsl.contains("emit_velocity"));
    }

    #[test]
    fn sim_shader_has_gravity() {
        let wgsl = generate_sim_wgsl(&make_particles());
        assert!(wgsl.contains("params.gravity"));
        assert!(wgsl.contains("p.vel.y +="));
    }

    #[test]
    fn sim_shader_has_hash() {
        let wgsl = generate_sim_wgsl(&make_particles());
        assert!(wgsl.contains("fn hash(seed: u32)"));
        assert!(wgsl.contains("0x45d9f3bu"));
    }

    #[test]
    fn sim_shader_has_emit_modes() {
        let wgsl = generate_sim_wgsl(&make_particles());
        assert!(wgsl.contains("emit_mode == 1u")); // random
        assert!(wgsl.contains("emit_mode == 2u")); // ring
        assert!(wgsl.contains("emit_mode == 3u")); // point
    }

    #[test]
    fn raster_shader_has_entry_point() {
        let wgsl = generate_raster_wgsl(&make_particles());
        assert!(wgsl.contains("fn cs_raster"));
        assert!(wgsl.contains("@compute @workgroup_size(64)"));
    }

    #[test]
    fn raster_shader_has_density_splat() {
        let wgsl = generate_raster_wgsl(&make_particles());
        assert!(wgsl.contains("falloff"));
        assert!(wgsl.contains("texture[texIdx]"));
    }

    #[test]
    fn raster_shader_skips_dead_particles() {
        let wgsl = generate_raster_wgsl(&make_particles());
        assert!(wgsl.contains("if (p.life <= 0.0) { return; }"));
    }

    #[test]
    fn runtime_js_generates() {
        let js = generate_particles_runtime_js(&make_particles(), 512, 512);
        assert!(js.contains("class GameParticleSim"));
        assert!(js.contains("cs_particles"));
        assert!(js.contains("cs_raster"));
        assert!(js.contains("5000")); // particle count
    }

    #[test]
    fn runtime_has_dual_pipelines() {
        let js = generate_particles_runtime_js(&make_particles(), 256, 256);
        assert!(js.contains("simPipeline"));
        assert!(js.contains("rasterPipeline"));
    }

    #[test]
    fn runtime_has_accessors() {
        let js = generate_particles_runtime_js(&make_particles(), 256, 256);
        assert!(js.contains("get textureBuffer()"));
        assert!(js.contains("get particleBuffer()"));
        assert!(js.contains("get particleCount()"));
    }

    #[test]
    fn runtime_has_dispatch() {
        let js = generate_particles_runtime_js(&make_particles(), 256, 256);
        assert!(js.contains("dispatch(dt)"));
        assert!(js.contains("dispatchWorkgroups"));
    }

    #[test]
    fn runtime_embeds_gravity() {
        let js = generate_particles_runtime_js(&make_particles(), 256, 256);
        assert!(js.contains("-0.2")); // gravity value
    }

    #[test]
    fn random_emit_mode_encoded() {
        let mut p = make_particles();
        p.emit = EmitMode::Random;
        let js = generate_particles_runtime_js(&p, 256, 256);
        assert!(js.contains("u[8] = 1")); // emit_mode = 1 = random
    }

    #[test]
    fn ring_emit_mode_encoded() {
        let mut p = make_particles();
        p.emit = EmitMode::Ring(0.5);
        let js = generate_particles_runtime_js(&p, 256, 256);
        assert!(js.contains("u[8] = 2")); // emit_mode = 2 = ring
        assert!(js.contains("f[11] = 0.5")); // emit_radius
    }

    #[test]
    fn point_emit_mode_encoded() {
        let mut p = make_particles();
        p.emit = EmitMode::Point(0.3, -0.5);
        let js = generate_particles_runtime_js(&p, 256, 256);
        assert!(js.contains("u[8] = 3")); // emit_mode = 3 = point
        assert!(js.contains("f[9] = 0.3")); // emit_x
        assert!(js.contains("f[10] = -0.5")); // emit_y
    }

    #[test]
    fn fade_disabled_generates_zero() {
        let mut p = make_particles();
        p.fade = false;
        let wgsl = generate_sim_wgsl(&p);
        assert!(wgsl.contains("FADE: 0"));
    }
}
