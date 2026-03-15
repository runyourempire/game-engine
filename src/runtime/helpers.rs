//! JavaScript helper code shared by all output formats.

/// Compute type for wiring compute shader output to the fragment shader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeType {
    React,
    Swarm,
    Flow,
}

/// WebGPU renderer class with optional feature support.
///
/// `needs_prev_frame` — enables ping-pong memory textures for `memory:` and `feedback:`
/// `pass_count` — number of post-processing passes (creates FBO chain)
/// `compute_type` — optional compute simulation whose output is bound to the fragment shader
pub fn webgpu_renderer(
    needs_prev_frame: bool,
    pass_count: usize,
    compute_type: Option<ComputeType>,
) -> String {
    let has_passes = pass_count > 0;
    let mut s = String::with_capacity(8192);

    // ── Class declaration ────────────────────────────────────────────
    s.push_str("class GameRenderer {\n");

    // ── Constructor ──────────────────────────────────────────────────
    s.push_str("  constructor(canvas, wgslVertex, wgslFragment, uniformDefs");
    if has_passes {
        s.push_str(", passShaders");
    }
    if compute_type.is_some() {
        s.push_str(", computeType");
    }
    s.push_str(") {\n");
    s.push_str("    this.canvas = canvas;\n");
    s.push_str("    this.wgslVertex = wgslVertex;\n");
    s.push_str("    this.wgslFragment = wgslFragment;\n");
    s.push_str("    this.uniformDefs = uniformDefs;\n");
    if has_passes {
        s.push_str("    this.passShaders = passShaders;\n");
    }
    if compute_type.is_some() {
        s.push_str("    this._computeType = computeType;\n");
        s.push_str("    this._computeBuf = null;\n");
        s.push_str("    this._computeW = 0;\n");
        s.push_str("    this._computeH = 0;\n");
    }
    s.push_str("    this.device = null;\n");
    s.push_str("    this.pipeline = null;\n");
    s.push_str("    this.uniformBuffer = null;\n");
    s.push_str("    this.bindGroup = null;\n");
    s.push_str("    this.running = false;\n");
    s.push_str("    this.startTime = performance.now() / 1000;\n");
    s.push_str("    this.audioData = { bass: 0, mid: 0, treble: 0, energy: 0, beat: 0 };\n");
    s.push_str("    this.mouseX = 0; this.mouseY = 0; this.mouseDown = 0;\n");
    s.push_str("    this.userParams = {};\n");
    s.push_str("    for (const u of uniformDefs) this.userParams[u.name] = u.default;\n");
    s.push_str("    this._onMouseMove = (e) => {\n");
    s.push_str("      const r = this.canvas.getBoundingClientRect();\n");
    s.push_str("      this.mouseX = (e.clientX - r.left) / r.width;\n");
    s.push_str("      this.mouseY = 1.0 - (e.clientY - r.top) / r.height;\n");
    s.push_str("    };\n");
    s.push_str("    this._onMouseDown = () => { this.mouseDown = 1; };\n");
    s.push_str("    this._onMouseUp = () => { this.mouseDown = 0; };\n");
    s.push_str("    this._onTouchStart = (e) => {\n");
    s.push_str("      this.mouseDown = 1;\n");
    s.push_str("      const r = this.canvas.getBoundingClientRect();\n");
    s.push_str("      const t = e.touches[0];\n");
    s.push_str("      this.mouseX = (t.clientX - r.left) / r.width;\n");
    s.push_str("      this.mouseY = 1.0 - (t.clientY - r.top) / r.height;\n");
    s.push_str("    };\n");
    s.push_str("    this._onTouchMove = (e) => {\n");
    s.push_str("      const r = this.canvas.getBoundingClientRect();\n");
    s.push_str("      const t = e.touches[0];\n");
    s.push_str("      this.mouseX = (t.clientX - r.left) / r.width;\n");
    s.push_str("      this.mouseY = 1.0 - (t.clientY - r.top) / r.height;\n");
    s.push_str("    };\n");
    s.push_str("    this._onTouchEnd = () => { this.mouseDown = 0; };\n");
    s.push_str("    this.canvas.addEventListener('mousemove', this._onMouseMove);\n");
    s.push_str("    this.canvas.addEventListener('mousedown', this._onMouseDown);\n");
    s.push_str("    this.canvas.addEventListener('mouseup', this._onMouseUp);\n");
    s.push_str(
        "    this.canvas.addEventListener('touchstart', this._onTouchStart, {passive: true});\n",
    );
    s.push_str(
        "    this.canvas.addEventListener('touchmove', this._onTouchMove, {passive: true});\n",
    );
    s.push_str("    this.canvas.addEventListener('touchend', this._onTouchEnd);\n");
    s.push_str("  }\n\n");

    // ── init() ───────────────────────────────────────────────────────
    s.push_str("  async init() {\n");
    s.push_str("    if (!navigator.gpu) return false;\n");
    s.push_str("    const adapter = await navigator.gpu.requestAdapter();\n");
    s.push_str("    if (!adapter) return false;\n");
    s.push_str("    this.device = await adapter.requestDevice();\n");
    s.push_str("    const ctx = this.canvas.getContext('webgpu');\n");
    s.push_str("    const format = navigator.gpu.getPreferredCanvasFormat();\n");
    if needs_prev_frame {
        s.push_str("    ctx.configure({ device: this.device, format, alphaMode: 'premultiplied', usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_DST });\n");
    } else {
        s.push_str(
            "    ctx.configure({ device: this.device, format, alphaMode: 'premultiplied' });\n",
        );
    }
    s.push_str("    this.ctx = ctx;\n");
    s.push_str("    this.format = format;\n\n");

    s.push_str("    const vMod = this.device.createShaderModule({ code: this.wgslVertex });\n");
    s.push_str("    const fMod = this.device.createShaderModule({ code: this.wgslFragment });\n\n");

    // Uniform buffer
    s.push_str("    const floatCount = 11 + this.uniformDefs.length;\n");
    s.push_str("    const bufSize = Math.ceil(floatCount * 4 / 16) * 16;\n");
    s.push_str("    this.uniformBuffer = this.device.createBuffer({\n");
    s.push_str("      size: bufSize, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST\n");
    s.push_str("    });\n");
    s.push_str("    this.floatCount = floatCount;\n\n");

    // Bind group layout (Group 0 = uniforms)
    s.push_str("    const bindGroupLayout = this.device.createBindGroupLayout({\n");
    s.push_str("      entries: [{ binding: 0, visibility: GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } }]\n");
    s.push_str("    });\n");
    s.push_str("    this.bindGroup = this.device.createBindGroup({\n");
    s.push_str("      layout: bindGroupLayout,\n");
    s.push_str("      entries: [{ binding: 0, resource: { buffer: this.uniformBuffer } }]\n");
    s.push_str("    });\n\n");

    // Compute bind group layout (storage buffer for compute output)
    if compute_type.is_some() {
        s.push_str("    this._computeBGL = this.device.createBindGroupLayout({\n");
        s.push_str("      entries: [{ binding: 0, visibility: GPUShaderStage.FRAGMENT, buffer: { type: 'read-only-storage' } }]\n");
        s.push_str("    });\n");
    }

    // Memory/feedback init + pipeline layout
    if needs_prev_frame {
        s.push_str("    // Memory/feedback: ping-pong textures (Group 1)\n");
        s.push_str("    this._initMemory();\n");
        if compute_type.is_some() {
            s.push_str("    const pipelineLayout = this.device.createPipelineLayout({\n");
            s.push_str("      bindGroupLayouts: [bindGroupLayout, this._memBindGroupLayout, this._computeBGL]\n");
            s.push_str("    });\n\n");
        } else {
            s.push_str("    const pipelineLayout = this.device.createPipelineLayout({\n");
            s.push_str("      bindGroupLayouts: [bindGroupLayout, this._memBindGroupLayout]\n");
            s.push_str("    });\n\n");
        }
    } else if compute_type.is_some() {
        s.push_str("    const pipelineLayout = this.device.createPipelineLayout({\n");
        s.push_str("      bindGroupLayouts: [bindGroupLayout, this._computeBGL]\n");
        s.push_str("    });\n\n");
    } else {
        s.push_str(
            "    const pipelineLayout = this.device.createPipelineLayout({ bindGroupLayouts: [bindGroupLayout] });\n\n",
        );
    }

    // Main render pipeline
    s.push_str("    this.pipeline = this.device.createRenderPipeline({\n");
    s.push_str("      layout: pipelineLayout,\n");
    s.push_str("      vertex: { module: vMod, entryPoint: 'vs_main' },\n");
    s.push_str(
        "      fragment: { module: fMod, entryPoint: 'fs_main', targets: [{ format, blend: { color: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha' }, alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha' } } }] },\n",
    );
    s.push_str("      primitive: { topology: 'triangle-list' }\n");
    s.push_str("    });\n");

    // Pass pipelines
    if has_passes {
        s.push_str("\n    // Post-processing pass pipelines\n");
        s.push_str("    this._passPipelines = [];\n");
        s.push_str("    const passBGL = this.device.createBindGroupLayout({\n");
        s.push_str("      entries: [\n");
        s.push_str("        { binding: 0, visibility: GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },\n");
        s.push_str("        { binding: 3, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float' } },\n");
        s.push_str("        { binding: 4, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }\n");
        s.push_str("      ]\n");
        s.push_str("    });\n");
        s.push_str("    this._passBGL = passBGL;\n");
        s.push_str("    const passPL = this.device.createPipelineLayout({ bindGroupLayouts: [passBGL] });\n");
        s.push_str("    for (const code of this.passShaders) {\n");
        s.push_str("      const mod = this.device.createShaderModule({ code });\n");
        s.push_str("      this._passPipelines.push(this.device.createRenderPipeline({\n");
        s.push_str("        layout: passPL,\n");
        s.push_str("        vertex: { module: vMod, entryPoint: 'vs_main' },\n");
        s.push_str("        fragment: { module: mod, entryPoint: 'fs_main', targets: [{ format, blend: { color: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha' }, alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha' } } }] },\n");
        s.push_str("        primitive: { topology: 'triangle-list' }\n");
        s.push_str("      }));\n");
        s.push_str("    }\n");
        s.push_str("    this._passSampler = this.device.createSampler({ magFilter: 'linear', minFilter: 'linear' });\n");
        s.push_str("    this._initPassFBOs();\n");
    }

    s.push_str("    return true;\n");
    s.push_str("  }\n\n");

    // ── start / stop ─────────────────────────────────────────────────
    s.push_str("  start() {\n");
    s.push_str("    if (this.running) return;\n");
    s.push_str("    this.running = true;\n");
    s.push_str("    const loop = () => {\n");
    s.push_str("      if (!this.running) return;\n");
    s.push_str("      this.render();\n");
    s.push_str("      requestAnimationFrame(loop);\n");
    s.push_str("    };\n");
    s.push_str("    requestAnimationFrame(loop);\n");
    s.push_str("  }\n\n");
    s.push_str("  stop() { this.running = false; }\n\n");

    // ── render() ─────────────────────────────────────────────────────
    s.push_str("  render() {\n");
    s.push_str("    if (this._preRender) this._preRender();\n");
    // Uniform data
    s.push_str("    const t = performance.now() / 1000 - this.startTime;\n");
    s.push_str("    const w = this.canvas.width;\n");
    s.push_str("    const h = this.canvas.height;\n");
    s.push_str("    const data = new Float32Array(this.floatCount);\n");
    s.push_str("    data[0] = t;\n");
    s.push_str("    data[1] = this.audioData.bass;\n");
    s.push_str("    data[2] = this.audioData.mid;\n");
    s.push_str("    data[3] = this.audioData.treble;\n");
    s.push_str("    data[4] = this.audioData.energy;\n");
    s.push_str("    data[5] = this.audioData.beat;\n");
    s.push_str("    data[6] = w; data[7] = h;\n");
    s.push_str("    data[8] = this.mouseX; data[9] = this.mouseY;\n");
    s.push_str("    data[10] = this.mouseDown;\n");
    s.push_str("    let i = 11;\n");
    s.push_str(
        "    for (const u of this.uniformDefs) data[i++] = this.userParams[u.name] ?? u.default;\n",
    );
    s.push_str("    this.device.queue.writeBuffer(this.uniformBuffer, 0, data);\n\n");

    s.push_str("    const encoder = this.device.createCommandEncoder();\n\n");

    // Main render pass target
    if has_passes {
        s.push_str("    // Main pass renders to FBO (input for post-processing)\n");
        s.push_str("    const mainPass = encoder.beginRenderPass({\n");
        s.push_str("      colorAttachments: [{\n");
        s.push_str("        view: this._passFBOs[0].createView(),\n");
        s.push_str(
            "        loadOp: 'clear', storeOp: 'store', clearValue: { r: 0, g: 0, b: 0, a: 0 }\n",
        );
        s.push_str("      }]\n");
        s.push_str("    });\n");
    } else if needs_prev_frame {
        // Memory path: render to memory write texture (has CopySrc), then copy to canvas
        s.push_str("    const memWriteTex = this._memTex[1 - this._memIdx];\n");
        s.push_str("    const mainPass = encoder.beginRenderPass({\n");
        s.push_str("      colorAttachments: [{\n");
        s.push_str("        view: memWriteTex.createView(),\n");
        s.push_str(
            "        loadOp: 'clear', storeOp: 'store', clearValue: { r: 0, g: 0, b: 0, a: 0 }\n",
        );
        s.push_str("      }]\n");
        s.push_str("    });\n");
    } else {
        s.push_str("    const mainPass = encoder.beginRenderPass({\n");
        s.push_str("      colorAttachments: [{\n");
        s.push_str("        view: this.ctx.getCurrentTexture().createView(),\n");
        s.push_str(
            "        loadOp: 'clear', storeOp: 'store', clearValue: { r: 0, g: 0, b: 0, a: 0 }\n",
        );
        s.push_str("      }]\n");
        s.push_str("    });\n");
    }
    s.push_str("    mainPass.setPipeline(this.pipeline);\n");
    s.push_str("    mainPass.setBindGroup(0, this.bindGroup);\n");
    if needs_prev_frame {
        s.push_str("    mainPass.setBindGroup(1, this._memBindGroup);\n");
    }
    if compute_type.is_some() {
        let compute_group_idx = if needs_prev_frame { 2 } else { 1 };
        s.push_str("    if (this._computeBuf) {\n");
        s.push_str("      const computeBG = this.device.createBindGroup({\n");
        s.push_str("        layout: this._computeBGL,\n");
        s.push_str("        entries: [{ binding: 0, resource: { buffer: this._computeBuf } }]\n");
        s.push_str("      });\n");
        s.push_str(&format!(
            "      mainPass.setBindGroup({compute_group_idx}, computeBG);\n"
        ));
        s.push_str("    }\n");
    }
    s.push_str("    mainPass.draw(3);\n");
    s.push_str("    mainPass.end();\n");

    // Memory swap + copy to canvas
    if needs_prev_frame {
        if has_passes {
            // FBO path: copy FBO → memory, passes will write to canvas
            s.push_str("\n    // Capture frame for memory/feedback\n");
            s.push_str("    this._swapMemory(encoder, this._passFBOs[0]);\n");
        } else {
            // Direct path: we rendered to memWriteTex, copy it to canvas for display
            s.push_str("\n    // Copy rendered frame to canvas and swap memory\n");
            s.push_str("    encoder.copyTextureToTexture(\n");
            s.push_str("      { texture: memWriteTex },\n");
            s.push_str("      { texture: this.ctx.getCurrentTexture() },\n");
            s.push_str("      { width: this.canvas.width, height: this.canvas.height }\n");
            s.push_str("    );\n");
            s.push_str("    this._memIdx = 1 - this._memIdx;\n");
            s.push_str("    this._updateMemBindGroup();\n");
        }
    }

    // Post-processing pass chain
    if has_passes {
        s.push_str(&format!(
            "\n    // Post-processing chain ({pass_count} pass{})\n",
            if pass_count > 1 { "es" } else { "" }
        ));
        s.push_str(&format!("    for (let p = 0; p < {pass_count}; p++) {{\n"));
        s.push_str(&format!("      const isLast = (p === {pass_count} - 1);\n"));
        s.push_str("      const readIdx = p % 2;\n");
        s.push_str("      const targetView = isLast\n");
        s.push_str("        ? this.ctx.getCurrentTexture().createView()\n");
        s.push_str("        : this._passFBOs[(p + 1) % 2].createView();\n");
        s.push_str("      const passBindGroup = this.device.createBindGroup({\n");
        s.push_str("        layout: this._passBGL,\n");
        s.push_str("        entries: [\n");
        s.push_str("          { binding: 0, resource: { buffer: this.uniformBuffer } },\n");
        s.push_str("          { binding: 3, resource: this._passFBOs[readIdx].createView() },\n");
        s.push_str("          { binding: 4, resource: this._passSampler }\n");
        s.push_str("        ]\n");
        s.push_str("      });\n");
        s.push_str("      const pp = encoder.beginRenderPass({\n");
        s.push_str("        colorAttachments: [{\n");
        s.push_str("          view: targetView,\n");
        s.push_str(
            "          loadOp: 'clear', storeOp: 'store', clearValue: { r: 0, g: 0, b: 0, a: 0 }\n",
        );
        s.push_str("        }]\n");
        s.push_str("      });\n");
        s.push_str("      pp.setPipeline(this._passPipelines[p]);\n");
        s.push_str("      pp.setBindGroup(0, passBindGroup);\n");
        s.push_str("      pp.draw(3);\n");
        s.push_str("      pp.end();\n");
        s.push_str("    }\n");
    }

    s.push_str("    this.device.queue.submit([encoder.finish()]);\n");
    s.push_str("  }\n\n");

    // ── Memory/feedback methods ──────────────────────────────────────
    if needs_prev_frame {
        s.push_str("  _initMemory() {\n");
        s.push_str("    const w = this.canvas.width || 1;\n");
        s.push_str("    const h = this.canvas.height || 1;\n");
        s.push_str("    const desc = {\n");
        s.push_str("      size: { width: w, height: h },\n");
        s.push_str("      format: this.format,\n");
        s.push_str("      usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC | GPUTextureUsage.COPY_DST\n");
        s.push_str("    };\n");
        s.push_str("    this._memTex = [this.device.createTexture(desc), this.device.createTexture(desc)];\n");
        s.push_str("    this._memIdx = 0;\n");
        s.push_str("    this._memSampler = this.device.createSampler({ magFilter: 'linear', minFilter: 'linear' });\n");
        s.push_str("    this._memBindGroupLayout = this.device.createBindGroupLayout({\n");
        s.push_str("      entries: [\n");
        s.push_str("        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float' } },\n");
        s.push_str("        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }\n");
        s.push_str("      ]\n");
        s.push_str("    });\n");
        s.push_str("    this._updateMemBindGroup();\n");
        s.push_str("  }\n\n");

        s.push_str("  _updateMemBindGroup() {\n");
        s.push_str("    const readTex = this._memTex[this._memIdx];\n");
        s.push_str("    this._memBindGroup = this.device.createBindGroup({\n");
        s.push_str("      layout: this._memBindGroupLayout,\n");
        s.push_str("      entries: [\n");
        s.push_str("        { binding: 0, resource: readTex.createView() },\n");
        s.push_str("        { binding: 1, resource: this._memSampler }\n");
        s.push_str("      ]\n");
        s.push_str("    });\n");
        s.push_str("  }\n\n");

        s.push_str("  _swapMemory(encoder, sourceTex) {\n");
        s.push_str("    const writeTex = this._memTex[1 - this._memIdx];\n");
        s.push_str("    encoder.copyTextureToTexture(\n");
        s.push_str("      { texture: sourceTex },\n");
        s.push_str("      { texture: writeTex },\n");
        s.push_str("      { width: this.canvas.width, height: this.canvas.height }\n");
        s.push_str("    );\n");
        s.push_str("    this._memIdx = 1 - this._memIdx;\n");
        s.push_str("    this._updateMemBindGroup();\n");
        s.push_str("  }\n\n");

        s.push_str("  _resizeMemory() {\n");
        s.push_str("    if (this._memTex) {\n");
        s.push_str("      this._memTex[0].destroy();\n");
        s.push_str("      this._memTex[1].destroy();\n");
        s.push_str("      this._initMemory();\n");
        s.push_str("    }\n");
        s.push_str("  }\n\n");
    }

    // ── Pass FBO methods ─────────────────────────────────────────────
    if has_passes {
        s.push_str("  _initPassFBOs() {\n");
        s.push_str("    const w = this.canvas.width || 1;\n");
        s.push_str("    const h = this.canvas.height || 1;\n");
        s.push_str("    const desc = {\n");
        s.push_str("      size: { width: w, height: h },\n");
        s.push_str("      format: this.format,\n");
        s.push_str("      usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC\n");
        s.push_str("    };\n");
        s.push_str("    this._passFBOs = [this.device.createTexture(desc), this.device.createTexture(desc)];\n");
        s.push_str("  }\n\n");

        s.push_str("  _resizePassFBOs() {\n");
        s.push_str("    if (this._passFBOs) {\n");
        s.push_str("      this._passFBOs[0].destroy();\n");
        s.push_str("      this._passFBOs[1].destroy();\n");
        s.push_str("      this._initPassFBOs();\n");
        s.push_str("    }\n");
        s.push_str("  }\n\n");
    }

    // ── Compute buffer wiring ──────────────────────────────────────
    if compute_type.is_some() {
        s.push_str("  setComputeBuffer(buf, w, h) {\n");
        s.push_str("    this._computeBuf = buf;\n");
        s.push_str("    this._computeW = w;\n");
        s.push_str("    this._computeH = h;\n");
        s.push_str("  }\n\n");
    }

    // ── Utility methods ──────────────────────────────────────────────
    s.push_str("  setParam(name, value) { this.userParams[name] = value; }\n");
    s.push_str("  setAudioData(d) { Object.assign(this.audioData, d); }\n");
    s.push_str("  destroy() {\n");
    s.push_str("    this.stop();\n");
    s.push_str("    this.canvas.removeEventListener('mousemove', this._onMouseMove);\n");
    s.push_str("    this.canvas.removeEventListener('mousedown', this._onMouseDown);\n");
    s.push_str("    this.canvas.removeEventListener('mouseup', this._onMouseUp);\n");
    s.push_str("    this.canvas.removeEventListener('touchstart', this._onTouchStart);\n");
    s.push_str("    this.canvas.removeEventListener('touchmove', this._onTouchMove);\n");
    s.push_str("    this.canvas.removeEventListener('touchend', this._onTouchEnd);\n");
    s.push_str("    this.device?.destroy();\n");
    s.push_str("  }\n");

    s.push_str("}\n");

    s
}

/// WebGL2 fallback renderer class with optional memory support.
///
/// Passes are WebGPU-only (they need separate render targets).
pub fn webgl2_renderer(needs_prev_frame: bool) -> String {
    let mut s = String::with_capacity(4096);

    s.push_str("class GameRendererGL {\n");

    // ── Constructor ──────────────────────────────────────────────────
    s.push_str("  constructor(canvas, glslVertex, glslFragment, uniformDefs) {\n");
    s.push_str("    this.canvas = canvas;\n");
    s.push_str("    this.glslVertex = glslVertex;\n");
    s.push_str("    this.glslFragment = glslFragment;\n");
    s.push_str("    this.uniformDefs = uniformDefs;\n");
    s.push_str("    this.gl = null;\n");
    s.push_str("    this.program = null;\n");
    s.push_str("    this.running = false;\n");
    s.push_str("    this.startTime = performance.now() / 1000;\n");
    s.push_str("    this.audioData = { bass: 0, mid: 0, treble: 0, energy: 0, beat: 0 };\n");
    s.push_str("    this.mouseX = 0; this.mouseY = 0; this.mouseDown = 0;\n");
    s.push_str("    this.userParams = {};\n");
    s.push_str("    for (const u of uniformDefs) this.userParams[u.name] = u.default;\n");
    s.push_str("    this._onMouseMove = (e) => {\n");
    s.push_str("      const r = this.canvas.getBoundingClientRect();\n");
    s.push_str("      this.mouseX = (e.clientX - r.left) / r.width;\n");
    s.push_str("      this.mouseY = 1.0 - (e.clientY - r.top) / r.height;\n");
    s.push_str("    };\n");
    s.push_str("    this._onMouseDown = () => { this.mouseDown = 1; };\n");
    s.push_str("    this._onMouseUp = () => { this.mouseDown = 0; };\n");
    s.push_str("    this._onTouchStart = (e) => {\n");
    s.push_str("      this.mouseDown = 1;\n");
    s.push_str("      const r = this.canvas.getBoundingClientRect();\n");
    s.push_str("      const t = e.touches[0];\n");
    s.push_str("      this.mouseX = (t.clientX - r.left) / r.width;\n");
    s.push_str("      this.mouseY = 1.0 - (t.clientY - r.top) / r.height;\n");
    s.push_str("    };\n");
    s.push_str("    this._onTouchMove = (e) => {\n");
    s.push_str("      const r = this.canvas.getBoundingClientRect();\n");
    s.push_str("      const t = e.touches[0];\n");
    s.push_str("      this.mouseX = (t.clientX - r.left) / r.width;\n");
    s.push_str("      this.mouseY = 1.0 - (t.clientY - r.top) / r.height;\n");
    s.push_str("    };\n");
    s.push_str("    this._onTouchEnd = () => { this.mouseDown = 0; };\n");
    s.push_str("    this.canvas.addEventListener('mousemove', this._onMouseMove);\n");
    s.push_str("    this.canvas.addEventListener('mousedown', this._onMouseDown);\n");
    s.push_str("    this.canvas.addEventListener('mouseup', this._onMouseUp);\n");
    s.push_str(
        "    this.canvas.addEventListener('touchstart', this._onTouchStart, {passive: true});\n",
    );
    s.push_str(
        "    this.canvas.addEventListener('touchmove', this._onTouchMove, {passive: true});\n",
    );
    s.push_str("    this.canvas.addEventListener('touchend', this._onTouchEnd);\n");
    s.push_str("  }\n\n");

    // ── init() ───────────────────────────────────────────────────────
    s.push_str("  init() {\n");
    s.push_str("    const gl = this.canvas.getContext('webgl2', { alpha: true, premultipliedAlpha: true });\n");
    s.push_str("    if (!gl) return false;\n");
    s.push_str("    this.gl = gl;\n\n");

    s.push_str("    const vs = this._compile(gl.VERTEX_SHADER, this.glslVertex);\n");
    s.push_str("    const fs = this._compile(gl.FRAGMENT_SHADER, this.glslFragment);\n");
    s.push_str("    if (!vs || !fs) return false;\n\n");

    s.push_str("    this.program = gl.createProgram();\n");
    s.push_str("    gl.attachShader(this.program, vs);\n");
    s.push_str("    gl.attachShader(this.program, fs);\n");
    s.push_str("    gl.linkProgram(this.program);\n");
    s.push_str("    if (!gl.getProgramParameter(this.program, gl.LINK_STATUS)) {\n");
    s.push_str("      console.error('GAME link error:', gl.getProgramInfoLog(this.program));\n");
    s.push_str("      return false;\n");
    s.push_str("    }\n");
    s.push_str("    gl.useProgram(this.program);\n\n");

    // Uniform locations
    s.push_str("    this.locs = {\n");
    s.push_str("      time: gl.getUniformLocation(this.program, 'u_time'),\n");
    s.push_str("      bass: gl.getUniformLocation(this.program, 'u_audio_bass'),\n");
    s.push_str("      mid: gl.getUniformLocation(this.program, 'u_audio_mid'),\n");
    s.push_str("      treble: gl.getUniformLocation(this.program, 'u_audio_treble'),\n");
    s.push_str("      energy: gl.getUniformLocation(this.program, 'u_audio_energy'),\n");
    s.push_str("      beat: gl.getUniformLocation(this.program, 'u_audio_beat'),\n");
    s.push_str("      resolution: gl.getUniformLocation(this.program, 'u_resolution'),\n");
    s.push_str("      mouse: gl.getUniformLocation(this.program, 'u_mouse'),\n");
    s.push_str("      mouse_down: gl.getUniformLocation(this.program, 'u_mouse_down'),\n");
    s.push_str("    };\n");
    s.push_str("    this.paramLocs = {};\n");
    s.push_str("    for (const u of this.uniformDefs) {\n");
    s.push_str(
        "      this.paramLocs[u.name] = gl.getUniformLocation(this.program, 'u_p_' + u.name);\n",
    );
    s.push_str("    }\n");

    if needs_prev_frame {
        s.push_str("    this._initMemoryGL();\n");
    }

    s.push_str("    return true;\n");
    s.push_str("  }\n\n");

    // ── _compile ─────────────────────────────────────────────────────
    s.push_str("  _compile(type, src) {\n");
    s.push_str("    const gl = this.gl;\n");
    s.push_str("    const s = gl.createShader(type);\n");
    s.push_str("    gl.shaderSource(s, src);\n");
    s.push_str("    gl.compileShader(s);\n");
    s.push_str("    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {\n");
    s.push_str("      console.error('GAME shader error:', gl.getShaderInfoLog(s));\n");
    s.push_str("      return null;\n");
    s.push_str("    }\n");
    s.push_str("    return s;\n");
    s.push_str("  }\n\n");

    // ── start / stop ─────────────────────────────────────────────────
    s.push_str("  start() {\n");
    s.push_str("    if (this.running) return;\n");
    s.push_str("    this.running = true;\n");
    s.push_str("    const loop = () => {\n");
    s.push_str("      if (!this.running) return;\n");
    s.push_str("      this.render();\n");
    s.push_str("      requestAnimationFrame(loop);\n");
    s.push_str("    };\n");
    s.push_str("    requestAnimationFrame(loop);\n");
    s.push_str("  }\n\n");
    s.push_str("  stop() { this.running = false; }\n\n");

    // ── render() ─────────────────────────────────────────────────────
    s.push_str("  render() {\n");
    s.push_str("    const gl = this.gl;\n");
    s.push_str("    const t = performance.now() / 1000 - this.startTime;\n");
    s.push_str("    gl.viewport(0, 0, this.canvas.width, this.canvas.height);\n");
    s.push_str("    gl.clearColor(0, 0, 0, 0);\n");
    s.push_str("    gl.clear(gl.COLOR_BUFFER_BIT);\n");
    s.push_str("    gl.enable(gl.BLEND);\n");
    s.push_str("    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);\n");
    s.push_str("    gl.useProgram(this.program);\n\n");

    if needs_prev_frame {
        s.push_str("    // Bind previous frame texture\n");
        s.push_str("    gl.activeTexture(gl.TEXTURE1);\n");
        s.push_str("    gl.bindTexture(gl.TEXTURE_2D, this._memTex[this._memIdx]);\n");
        s.push_str("    gl.uniform1i(this._memLoc, 1);\n\n");
    }

    s.push_str("    gl.uniform1f(this.locs.time, t);\n");
    s.push_str("    gl.uniform1f(this.locs.bass, this.audioData.bass);\n");
    s.push_str("    gl.uniform1f(this.locs.mid, this.audioData.mid);\n");
    s.push_str("    gl.uniform1f(this.locs.treble, this.audioData.treble);\n");
    s.push_str("    gl.uniform1f(this.locs.energy, this.audioData.energy);\n");
    s.push_str("    gl.uniform1f(this.locs.beat, this.audioData.beat);\n");
    s.push_str("    gl.uniform2f(this.locs.resolution, this.canvas.width, this.canvas.height);\n");
    s.push_str("    gl.uniform2f(this.locs.mouse, this.mouseX, this.mouseY);\n");
    s.push_str("    gl.uniform1f(this.locs.mouse_down, this.mouseDown);\n");
    s.push_str("    for (const u of this.uniformDefs) {\n");
    s.push_str(
        "      gl.uniform1f(this.paramLocs[u.name], this.userParams[u.name] ?? u.default);\n",
    );
    s.push_str("    }\n");
    s.push_str("    gl.drawArrays(gl.TRIANGLES, 0, 3);\n");

    if needs_prev_frame {
        s.push_str("\n    // Capture frame for memory/feedback\n");
        s.push_str("    this._swapMemoryGL();\n");
    }

    s.push_str("  }\n\n");

    // ── Memory methods (WebGL2) ──────────────────────────────────────
    if needs_prev_frame {
        s.push_str("  _initMemoryGL() {\n");
        s.push_str("    const gl = this.gl;\n");
        s.push_str("    const w = this.canvas.width || 1;\n");
        s.push_str("    const h = this.canvas.height || 1;\n");
        s.push_str("    this._memFbo = [gl.createFramebuffer(), gl.createFramebuffer()];\n");
        s.push_str("    this._memTex = [gl.createTexture(), gl.createTexture()];\n");
        s.push_str("    for (let i = 0; i < 2; i++) {\n");
        s.push_str("      gl.bindTexture(gl.TEXTURE_2D, this._memTex[i]);\n");
        s.push_str("      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);\n");
        s.push_str("      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);\n");
        s.push_str("      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);\n");
        s.push_str("      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);\n");
        s.push_str("      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);\n");
        s.push_str("      gl.bindFramebuffer(gl.FRAMEBUFFER, this._memFbo[i]);\n");
        s.push_str("      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, this._memTex[i], 0);\n");
        s.push_str("    }\n");
        s.push_str("    gl.bindFramebuffer(gl.FRAMEBUFFER, null);\n");
        s.push_str("    gl.bindTexture(gl.TEXTURE_2D, null);\n");
        s.push_str("    this._memIdx = 0;\n");
        s.push_str("    this._memLoc = gl.getUniformLocation(this.program, 'u_prev_frame');\n");
        s.push_str("  }\n\n");

        s.push_str("  _swapMemoryGL() {\n");
        s.push_str("    const gl = this.gl;\n");
        s.push_str("    const w = this.canvas.width;\n");
        s.push_str("    const h = this.canvas.height;\n");
        s.push_str("    const writeIdx = 1 - this._memIdx;\n");
        s.push_str("    gl.bindFramebuffer(gl.READ_FRAMEBUFFER, null);\n");
        s.push_str("    gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER, this._memFbo[writeIdx]);\n");
        s.push_str(
            "    gl.blitFramebuffer(0, 0, w, h, 0, 0, w, h, gl.COLOR_BUFFER_BIT, gl.NEAREST);\n",
        );
        s.push_str("    gl.bindFramebuffer(gl.FRAMEBUFFER, null);\n");
        s.push_str("    this._memIdx = writeIdx;\n");
        s.push_str("  }\n\n");

        s.push_str("  _resizeMemory() {\n");
        s.push_str("    if (this._memTex) {\n");
        s.push_str("      const gl = this.gl;\n");
        s.push_str("      const w = this.canvas.width || 1;\n");
        s.push_str("      const h = this.canvas.height || 1;\n");
        s.push_str("      for (let i = 0; i < 2; i++) {\n");
        s.push_str("        gl.bindTexture(gl.TEXTURE_2D, this._memTex[i]);\n");
        s.push_str("        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);\n");
        s.push_str("      }\n");
        s.push_str("      gl.bindTexture(gl.TEXTURE_2D, null);\n");
        s.push_str("    }\n");
        s.push_str("  }\n\n");
    }

    // ── Utility methods ──────────────────────────────────────────────
    s.push_str("  setParam(name, value) { this.userParams[name] = value; }\n");
    s.push_str("  setAudioData(d) { Object.assign(this.audioData, d); }\n");
    s.push_str("  destroy() {\n");
    s.push_str("    this.stop();\n");
    s.push_str("    this.canvas.removeEventListener('mousemove', this._onMouseMove);\n");
    s.push_str("    this.canvas.removeEventListener('mousedown', this._onMouseDown);\n");
    s.push_str("    this.canvas.removeEventListener('mouseup', this._onMouseUp);\n");
    s.push_str("    this.canvas.removeEventListener('touchstart', this._onTouchStart);\n");
    s.push_str("    this.canvas.removeEventListener('touchmove', this._onTouchMove);\n");
    s.push_str("    this.canvas.removeEventListener('touchend', this._onTouchEnd);\n");
    s.push_str("  }\n");

    s.push_str("}\n");

    s
}
