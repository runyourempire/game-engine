//! Generates a self-contained HTML file that runs a compiled GAME cinematic
//! using the WebGPU API. Zero dependencies.
//!
//! M1: Includes Web Audio API integration, parameter modulation,
//! mouse tracking, audio file drag-and-drop, microphone input,
//! and audio playback controls (pause/play, volume, level meters).

use crate::codegen::CompileOutput;

/// Legacy wrapper (M0 compatibility). Wraps WGSL in a basic HTML runtime.
pub fn wrap_html(wgsl: &str, title: &str) -> String {
    let output = CompileOutput {
        wgsl: wgsl.to_string(),
        title: title.to_string(),
        audio_file: None,
        params: Vec::new(),
        uses_audio: false,
        uses_mouse: false,
        uses_data: false,
        data_fields: Vec::new(),
        render_mode: crate::codegen::RenderMode::Flat,
        uniform_float_count: 10,
        arc_moments: Vec::new(),
    };
    wrap_html_full(&output)
}

/// Full HTML runtime generation from CompileOutput.
pub fn wrap_html_full(output: &CompileOutput) -> String {
    let title = &output.title;
    let wgsl = output.wgsl.replace('`', "\\`").replace("${", "\\${");
    let total_floats = output.uniform_float_count;
    let buffer_size = (total_floats * 4).div_ceil(16) * 16;

    let param_init_js = generate_param_init_js(&output.params);
    let param_update_js = generate_param_update_js(&output.params);

    // Audio overlay only visible for audio-reactive cinematics
    let overlay_display = if output.uses_audio { "" } else { " style=\"display:none\"" };

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} — GAME</title>
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  html, body {{ width: 100%; height: 100%; overflow: hidden; background: #000; }}
  canvas {{
    display: block;
    width: 100vw;
    height: 100vh;
  }}
  #fallback {{
    display: none;
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    color: #666;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 14px;
    text-align: center;
    line-height: 1.8;
  }}
  #info {{
    position: fixed;
    bottom: 12px;
    right: 16px;
    color: #333;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    pointer-events: none;
    transition: color 0.3s;
    z-index: 10;
  }}
  #info:hover {{ color: #666; }}
  #audio-overlay {{
    position: fixed;
    top: 0; left: 0; right: 0; bottom: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    z-index: 100;
    cursor: pointer;
    transition: opacity 0.5s;
  }}
  #audio-overlay.hidden {{
    opacity: 0;
    pointer-events: none;
  }}
  .audio-prompt {{
    color: #444;
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    text-align: center;
    line-height: 2;
    padding: 24px;
    border: 1px solid #222;
    border-radius: 8px;
    background: rgba(10, 10, 10, 0.9);
    backdrop-filter: blur(4px);
  }}
  .audio-prompt span {{ color: #666; font-size: 11px; }}
  #audio-controls {{
    position: fixed;
    bottom: 12px;
    left: 16px;
    display: none;
    align-items: center;
    gap: 10px;
    z-index: 10;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: #555;
    padding: 6px 12px;
    border: 1px solid #222;
    border-radius: 6px;
    background: rgba(10, 10, 10, 0.8);
    backdrop-filter: blur(4px);
    transition: opacity 0.3s;
  }}
  #audio-controls:hover {{ color: #888; }}
  #audio-controls button {{
    background: none;
    border: 1px solid #333;
    color: #666;
    width: 28px;
    height: 24px;
    cursor: pointer;
    font-size: 12px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
  }}
  #audio-controls button:hover {{ color: #aaa; border-color: #555; }}
  #vol-slider {{
    -webkit-appearance: none;
    appearance: none;
    width: 56px;
    height: 3px;
    background: #333;
    border-radius: 2px;
    outline: none;
    cursor: pointer;
  }}
  #vol-slider::-webkit-slider-thumb {{
    -webkit-appearance: none;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #666;
    cursor: pointer;
  }}
  #audio-levels {{
    display: flex;
    gap: 2px;
    align-items: flex-end;
    height: 16px;
  }}
  .lvl {{ width: 4px; border-radius: 1px; min-height: 2px; transition: height 0.06s; }}
  .lvl-b {{ background: #D4AF37; }}
  .lvl-m {{ background: #888; }}
  .lvl-t {{ background: #555; }}
</style>
</head>
<body>
<canvas id="canvas"></canvas>
<div id="fallback">
  <p>WebGPU is not available in this browser.</p>
  <p style="margin-top: 8px; color: #444;">Try Chrome 113+, Edge 113+, or Firefox Nightly.</p>
</div>
<div id="audio-overlay"{overlay_display}>
  <div class="audio-prompt">
    <p>drop an audio file to begin</p>
    <p>or click anywhere for microphone</p>
    <p><span>press F for fullscreen</span></p>
  </div>
</div>
<div id="audio-controls">
  <button id="btn-toggle" title="Pause/Resume">&#9646;&#9646;</button>
  <button id="btn-stop" title="Stop">&#9632;</button>
  <input type="range" id="vol-slider" min="0" max="1" step="0.01" value="1" title="Volume">
  <div id="audio-levels">
    <div class="lvl lvl-b" id="lvl-b"></div>
    <div class="lvl lvl-m" id="lvl-m"></div>
    <div class="lvl lvl-t" id="lvl-t"></div>
  </div>
</div>
<div id="info"></div>

<script type="module">
// ═══════════════════════════════════════════════════════════════════
// GAME Runtime v0.2.1 — Generative Animation Matrix Engine
// ═══════════════════════════════════════════════════════════════════

const SHADER = `{wgsl}`;

// ── Audio engine ──────────────────────────────────────────────────
let audioCtx = null;
let analyser = null;
let gainNode = null;
let audioData = new Uint8Array(0);
let audioActive = false;
let currentSource = null;
let isMicSource = false;
let micStream = null;

function initAudioCtx() {{
  if (!audioCtx) {{
    audioCtx = new AudioContext();
    analyser = audioCtx.createAnalyser();
    analyser.fftSize = 256;
    analyser.smoothingTimeConstant = 0.75;
    audioData = new Uint8Array(analyser.frequencyBinCount);
    gainNode = audioCtx.createGain();
    gainNode.connect(audioCtx.destination);
    analyser.connect(gainNode);
  }}
  if (audioCtx.state === 'suspended') {{
    audioCtx.resume();
  }}
}}

function stopCurrentSource() {{
  if (currentSource) {{
    try {{ currentSource.disconnect(); }} catch(_) {{}}
    if (!isMicSource) {{
      try {{ currentSource.stop(); }} catch(_) {{}}
    }}
    currentSource = null;
  }}
  if (micStream) {{
    micStream.getTracks().forEach(t => t.stop());
    micStream = null;
  }}
  isMicSource = false;
}}

function connectSource(sourceNode, isMic) {{
  stopCurrentSource();
  sourceNode.connect(analyser);
  currentSource = sourceNode;
  isMicSource = !!isMic;
  audioActive = true;
  document.getElementById('audio-overlay').classList.add('hidden');
  document.getElementById('audio-controls').style.display = 'flex';
}}

function getAudioBands() {{
  if (!analyser || !audioActive) {{
    return {{ bass: 0, mid: 0, treble: 0, energy: 0, beat: 0 }};
  }}
  analyser.getByteFrequencyData(audioData);
  const len = audioData.length;

  const avg = (start, end) => {{
    let sum = 0;
    const e = Math.min(end, len);
    for (let i = start; i < e; i++) sum += audioData[i];
    return (e > start) ? sum / (e - start) / 255 : 0;
  }};

  const bass = avg(0, 4);
  const mid = avg(4, 32);
  const treble = avg(32, len);
  const energy = avg(0, len);

  return {{ bass, mid, treble, energy, beat: bass > 0.5 ? 1.0 : 0.0 }};
}}

// ── Audio controls ───────────────────────────────────────────────
const btnToggle = document.getElementById('btn-toggle');
const btnStop = document.getElementById('btn-stop');
const volSlider = document.getElementById('vol-slider');
const lvlB = document.getElementById('lvl-b');
const lvlM = document.getElementById('lvl-m');
const lvlT = document.getElementById('lvl-t');
let audioPaused = false;

btnToggle.addEventListener('click', () => {{
  if (!audioCtx) return;
  if (audioPaused) {{
    audioCtx.resume();
    btnToggle.innerHTML = '&#9646;&#9646;';
    audioPaused = false;
  }} else {{
    audioCtx.suspend();
    btnToggle.innerHTML = '&#9654;';
    audioPaused = true;
  }}
}});

btnStop.addEventListener('click', () => {{
  stopCurrentSource();
  audioActive = false;
  document.getElementById('audio-controls').style.display = 'none';
  document.getElementById('audio-overlay').classList.remove('hidden');
  if (audioCtx) {{ audioCtx.suspend(); }}
  audioPaused = false;
  btnToggle.innerHTML = '&#9646;&#9646;';
}});

volSlider.addEventListener('input', () => {{
  if (gainNode) {{
    gainNode.gain.value = parseFloat(volSlider.value);
  }}
}});

function updateLevels(audio) {{
  if (!audioActive) return;
  lvlB.style.height = Math.max(2, audio.bass * 16) + 'px';
  lvlM.style.height = Math.max(2, audio.mid * 16) + 'px';
  lvlT.style.height = Math.max(2, audio.treble * 16) + 'px';
}}

// ── Mouse tracking ────────────────────────────────────────────────
let mouseX = 0.5, mouseY = 0.5;
document.addEventListener('mousemove', (e) => {{
  mouseX = e.clientX / window.innerWidth;
  mouseY = 1.0 - e.clientY / window.innerHeight;
}});

// ── Audio file drag-and-drop ──────────────────────────────────────
document.addEventListener('dragover', (e) => {{ e.preventDefault(); }});
document.addEventListener('drop', async (e) => {{
  e.preventDefault();
  const file = e.dataTransfer.files[0];
  if (!file) return;
  initAudioCtx();
  try {{
    const arrayBuffer = await file.arrayBuffer();
    const audioBuffer = await audioCtx.decodeAudioData(arrayBuffer);
    const source = audioCtx.createBufferSource();
    source.buffer = audioBuffer;
    source.loop = true;
    connectSource(source, false);
    source.start();
  }} catch (err) {{
    console.error('Audio decode error:', err);
  }}
}});

// ── Microphone on click ───────────────────────────────────────────
document.getElementById('audio-overlay').addEventListener('click', async () => {{
  initAudioCtx();
  try {{
    const stream = await navigator.mediaDevices.getUserMedia({{ audio: true }});
    micStream = stream;
    const source = audioCtx.createMediaStreamSource(stream);
    connectSource(source, true);
  }} catch (err) {{
    console.error('Microphone error:', err);
    document.getElementById('audio-overlay').classList.add('hidden');
  }}
}});

// ── Data signals ─────────────────────────────────────────────────
{data_vars_js}

// ── Arc timeline ─────────────────────────────────────────────────
{arc_js}

// ── Parameter modulation ──────────────────────────────────────────
{param_init_js}

function updateParams(time, audio) {{
  const audioBass = audio.bass;
  const audioMid = audio.mid;
  const audioTreble = audio.treble;
  const audioEnergy = audio.energy;
  const audioBeat = audio.beat;

  // Apply arc timeline — override base values for params with active transitions
  if (typeof arcUpdate === 'function') {{
    arcUpdate(time);
  }}

{param_update_js}}}

// ── WebGPU init ───────────────────────────────────────────────────
async function init() {{
  if (!navigator.gpu) {{
    document.getElementById('fallback').style.display = 'block';
    document.getElementById('canvas').style.display = 'none';
    document.getElementById('audio-overlay').style.display = 'none';
    return;
  }}

  const adapter = await navigator.gpu.requestAdapter({{
    powerPreference: 'high-performance',
  }});
  if (!adapter) {{
    document.getElementById('fallback').style.display = 'block';
    document.getElementById('canvas').style.display = 'none';
    document.getElementById('audio-overlay').style.display = 'none';
    return;
  }}

  const device = await adapter.requestDevice();
  device.lost.then((info) => {{
    console.error('WebGPU device lost:', info.message);
  }});

  const canvas = document.getElementById('canvas');
  const ctx = canvas.getContext('webgpu');
  const format = navigator.gpu.getPreferredCanvasFormat();

  function resize() {{
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.floor(canvas.clientWidth * dpr);
    canvas.height = Math.floor(canvas.clientHeight * dpr);
    ctx.configure({{
      device,
      format,
      alphaMode: 'opaque',
    }});
  }}
  resize();
  window.addEventListener('resize', resize);

  const shaderModule = device.createShaderModule({{ code: SHADER }});
  const compilationInfo = await shaderModule.getCompilationInfo();
  for (const msg of compilationInfo.messages) {{
    if (msg.type === 'error') {{
      console.error('Shader error:', msg.message, `(line ${{msg.lineNum}}, col ${{msg.linePos}})`);
    }}
  }}

  // ── Uniform buffer ({buffer_size} bytes, {total_floats} floats) ────
  const UNIFORM_FLOATS = {total_floats};
  const BUFFER_SIZE = {buffer_size};
  const uniformBuffer = device.createBuffer({{
    size: BUFFER_SIZE,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  }});

  const bindGroupLayout = device.createBindGroupLayout({{
    entries: [{{
      binding: 0,
      visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT,
      buffer: {{ type: 'uniform' }},
    }}],
  }});

  const bindGroup = device.createBindGroup({{
    layout: bindGroupLayout,
    entries: [{{ binding: 0, resource: {{ buffer: uniformBuffer }} }}],
  }});

  const pipeline = device.createRenderPipeline({{
    layout: device.createPipelineLayout({{
      bindGroupLayouts: [bindGroupLayout],
    }}),
    vertex: {{
      module: shaderModule,
      entryPoint: 'vs_main',
    }},
    fragment: {{
      module: shaderModule,
      entryPoint: 'fs_main',
      targets: [{{ format }}],
    }},
    primitive: {{
      topology: 'triangle-strip',
      stripIndexFormat: undefined,
    }},
  }});

  // ── Animation loop ────────────────────────────────────────────────
  const startTime = performance.now() / 1000;
  const uniformData = new Float32Array(Math.ceil(BUFFER_SIZE / 4));
  const infoEl = document.getElementById('info');
  let frameCount = 0;
  let lastFpsTime = performance.now();

  function frame() {{
    requestAnimationFrame(frame);

    const now = performance.now() / 1000;
    const time = now - startTime;
    const audio = getAudioBands();

    // Update modulated params
    updateParams(time, audio);

    // Update level meters
    updateLevels(audio);

    // Write uniforms: system + params
    uniformData[0] = time;
    uniformData[1] = audio.bass;
    uniformData[2] = audio.mid;
    uniformData[3] = audio.treble;
    uniformData[4] = audio.energy;
    uniformData[5] = audio.beat;
    uniformData[6] = canvas.width;
    uniformData[7] = canvas.height;
    uniformData[8] = mouseX;
    uniformData[9] = mouseY;

    // Dynamic params (indices 10+)
    for (let i = 0; i < params.length; i++) {{
      uniformData[10 + i] = params[i].value;
    }}

    device.queue.writeBuffer(uniformBuffer, 0, uniformData);

    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({{
      colorAttachments: [{{
        view: ctx.getCurrentTexture().createView(),
        loadOp: 'clear',
        clearValue: {{ r: 0, g: 0, b: 0, a: 1 }},
        storeOp: 'store',
      }}],
    }});
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.draw(4, 1, 0, 0);
    pass.end();
    device.queue.submit([encoder.finish()]);

    frameCount++;
    const elapsed = performance.now() - lastFpsTime;
    if (elapsed >= 1000) {{
      const fps = Math.round(frameCount * 1000 / elapsed);
      const audioTag = audioActive ? ' | audio' : '';
      infoEl.textContent = `${{fps}} fps${{audioTag}}`;
      frameCount = 0;
      lastFpsTime = performance.now();
    }}
  }}

  requestAnimationFrame(frame);

  document.addEventListener('keydown', (e) => {{
    if (e.key === 'f' || e.key === 'F') {{
      if (document.fullscreenElement) {{
        document.exitFullscreen();
      }} else {{
        canvas.requestFullscreen();
      }}
    }}
    if (e.key === ' ') {{
      e.preventDefault();
      btnToggle.click();
    }}
  }});
}}

init().catch((err) => {{
  console.error('GAME runtime error:', err);
  document.getElementById('fallback').style.display = 'block';
  document.getElementById('audio-overlay').style.display = 'none';
  document.getElementById('fallback').innerHTML =
    '<p style="color: #EF4444;">Runtime error</p>' +
    '<p style="color: #666; margin-top: 8px;">' + err.message + '</p>';
}});
</script>
</body>
</html>"##,
        title = title,
        wgsl = wgsl,
        buffer_size = buffer_size,
        total_floats = total_floats,
        data_vars_js = generate_data_vars_js(&output.data_fields),
        arc_js = generate_arc_js(&output.arc_moments),
        param_init_js = param_init_js,
        param_update_js = param_update_js,
        overlay_display = overlay_display,
    )
}

/// Generate JS variable declarations for `data.*` signals.
fn generate_data_vars_js(data_fields: &[String]) -> String {
    if data_fields.is_empty() {
        return String::new();
    }
    let mut lines = Vec::new();
    for field in data_fields {
        lines.push(format!("let data_{field} = 0;"));
    }
    lines.join("\n")
}

/// Generate JS code to initialize the params array.
fn generate_param_init_js(params: &[crate::codegen::CompiledParam]) -> String {
    if params.is_empty() {
        return "const params = [];".to_string();
    }

    let mut lines = Vec::new();
    lines.push("const params = [".to_string());
    for p in params {
        let mod_expr = p.mod_js.as_deref().unwrap_or("0");
        lines.push(format!(
            "  {{ name: '{}', base: {}, modExpr: (audioBass, audioMid, audioTreble, audioEnergy, audioBeat, time, mouseX, mouseY) => {}, value: {} }},",
            p.name, p.base_value, mod_expr, p.base_value
        ));
    }
    lines.push("];".to_string());
    lines.join("\n")
}

/// Generate JS code to update param values each frame.
fn generate_param_update_js(params: &[crate::codegen::CompiledParam]) -> String {
    if params.is_empty() {
        return String::new();
    }

    let mut lines = Vec::new();
    for (i, _p) in params.iter().enumerate() {
        lines.push(format!(
            "  params[{i}].value = params[{i}].base + params[{i}].modExpr(audioBass, audioMid, audioTreble, audioEnergy, audioBeat, time, mouseX, mouseY);"
        ));
    }
    lines.join("\n")
}

// ═══════════════════════════════════════════════════════════════════════
// Web Component output
// ═══════════════════════════════════════════════════════════════════════

/// Generate a self-contained ES module that exports a Custom Element.
pub fn wrap_web_component(output: &CompileOutput, tag_name: &str) -> String {
    let wgsl = output.wgsl.replace('`', "\\`").replace("${", "\\${");
    let total_floats = output.uniform_float_count;
    let buffer_size = (total_floats * 4).div_ceil(16) * 16;
    let class_name = tag_to_class_name(tag_name);

    let observed_attrs = generate_observed_attrs(&output.data_fields);
    let prop_accessors = generate_prop_accessors(&output.data_fields);
    let data_init = generate_data_init(&output.data_fields);
    let param_update_inline = generate_component_param_update(
        &output.params,
        &output.data_fields,
    );

    format!(
        r##"// {tag_name}.js — Generated by GAME compiler v0.2.0
// Zero-dependency WebGPU component. Import and use:
//   <script type="module" src="./{tag_name}.js"></script>
//   <{tag_name}></{tag_name}>

const SHADER = `{wgsl}`;

class {class_name} extends HTMLElement {{
  static get observedAttributes() {{
    return [{observed_attrs}];
  }}

  constructor() {{
    super();
    this.attachShadow({{ mode: 'open' }});
    this._data = {{ {data_init} }};
    this._device = null;
    this._ctx = null;
    this._pipeline = null;
    this._uniformBuffer = null;
    this._uniformData = null;
    this._bindGroup = null;
    this._animFrame = null;
    this._resizeObserver = null;
    this._startTime = 0;
    this._canvas = null;
    this._format = null;
    this._mouseX = 0.5;
    this._mouseY = 0.5;
    this._paramValues = new Float32Array({param_count});
  }}

{prop_accessors}
  attributeChangedCallback(name, _, newVal) {{
    if (name in this._data) {{
      this._data[name] = parseFloat(newVal) || 0;
    }}
  }}

  connectedCallback() {{
    this._init();
  }}

  disconnectedCallback() {{
    if (this._animFrame) {{
      cancelAnimationFrame(this._animFrame);
      this._animFrame = null;
    }}
    if (this._resizeObserver) {{
      this._resizeObserver.disconnect();
      this._resizeObserver = null;
    }}
    this._device = null;
    this._pipeline = null;
    this._uniformBuffer = null;
    this._ctx = null;
  }}

  async _init() {{
    this.shadowRoot.innerHTML = `
      <style>
        :host {{ display: block; position: relative; background: #000; overflow: hidden; }}
        canvas {{ display: block; width: 100%; height: 100%; }}
        .fallback {{ display: flex; align-items: center; justify-content: center; width: 100%; height: 100%; color: #444; font: 11px/1 system-ui, sans-serif; text-align: center; }}
      </style>
      <canvas></canvas>
    `;

    this._canvas = this.shadowRoot.querySelector('canvas');

    if (!navigator.gpu) {{
      this._canvas.style.display = 'none';
      const fb = document.createElement('div');
      fb.className = 'fallback';
      fb.textContent = 'WebGPU not available';
      this.shadowRoot.appendChild(fb);
      return;
    }}

    const adapter = await navigator.gpu.requestAdapter({{
      powerPreference: 'high-performance',
    }});
    if (!adapter) return;

    this._device = await adapter.requestDevice();
    this._device.lost.then((info) => {{
      console.error('WebGPU device lost:', info.message);
    }});

    this._ctx = this._canvas.getContext('webgpu');
    this._format = navigator.gpu.getPreferredCanvasFormat();

    this._resize();
    this._resizeObserver = new ResizeObserver(() => this._resize());
    this._resizeObserver.observe(this);

    const shaderModule = this._device.createShaderModule({{ code: SHADER }});

    const BUFFER_SIZE = {buffer_size};
    this._uniformBuffer = this._device.createBuffer({{
      size: BUFFER_SIZE,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    }});

    const bindGroupLayout = this._device.createBindGroupLayout({{
      entries: [{{
        binding: 0,
        visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT,
        buffer: {{ type: 'uniform' }},
      }}],
    }});

    this._bindGroup = this._device.createBindGroup({{
      layout: bindGroupLayout,
      entries: [{{ binding: 0, resource: {{ buffer: this._uniformBuffer }} }}],
    }});

    this._pipeline = this._device.createRenderPipeline({{
      layout: this._device.createPipelineLayout({{
        bindGroupLayouts: [bindGroupLayout],
      }}),
      vertex: {{
        module: shaderModule,
        entryPoint: 'vs_main',
      }},
      fragment: {{
        module: shaderModule,
        entryPoint: 'fs_main',
        targets: [{{ format: this._format }}],
      }},
      primitive: {{
        topology: 'triangle-strip',
      }},
    }});

    this._uniformData = new Float32Array(Math.ceil(BUFFER_SIZE / 4));
    this._startTime = performance.now() / 1000;

    // Mouse tracking scoped to this element
    this.addEventListener('mousemove', (e) => {{
      const rect = this.getBoundingClientRect();
      this._mouseX = (e.clientX - rect.left) / rect.width;
      this._mouseY = 1.0 - (e.clientY - rect.top) / rect.height;
    }});

    this._frame();
  }}

  _resize() {{
    if (!this._device || !this._ctx || !this._format) return;
    const rect = this.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return;
    const dpr = window.devicePixelRatio || 1;
    this._canvas.width = Math.floor(rect.width * dpr);
    this._canvas.height = Math.floor(rect.height * dpr);
    this._ctx.configure({{
      device: this._device,
      format: this._format,
      alphaMode: 'opaque',
    }});
  }}

  _frame() {{
    this._animFrame = requestAnimationFrame(() => this._frame());
    if (!this._device || !this._pipeline || !this._uniformBuffer) return;

    const now = performance.now() / 1000;
    const time = now - this._startTime;
    const mouseX = this._mouseX;
    const mouseY = this._mouseY;

    // Apply arc timeline (override param bases)
{arc_component_update}

    // Update param values
{param_update_inline}

    // Write uniforms
    this._uniformData[0] = time;
    this._uniformData[1] = 0; // audio_bass (unused in component mode)
    this._uniformData[2] = 0; // audio_mid
    this._uniformData[3] = 0; // audio_treble
    this._uniformData[4] = 0; // audio_energy
    this._uniformData[5] = 0; // audio_beat
    this._uniformData[6] = this._canvas.width;
    this._uniformData[7] = this._canvas.height;
    this._uniformData[8] = mouseX;
    this._uniformData[9] = mouseY;

    for (let i = 0; i < {param_count}; i++) {{
      this._uniformData[10 + i] = this._paramValues[i];
    }}

    this._device.queue.writeBuffer(this._uniformBuffer, 0, this._uniformData);

    const encoder = this._device.createCommandEncoder();
    const pass = encoder.beginRenderPass({{
      colorAttachments: [{{
        view: this._ctx.getCurrentTexture().createView(),
        loadOp: 'clear',
        clearValue: {{ r: 0, g: 0, b: 0, a: 1 }},
        storeOp: 'store',
      }}],
    }});
    pass.setPipeline(this._pipeline);
    pass.setBindGroup(0, this._bindGroup);
    pass.draw(4, 1, 0, 0);
    pass.end();
    this._device.queue.submit([encoder.finish()]);
  }}
}}

customElements.define('{tag_name}', {class_name});
export {{ {class_name} }};
export default {class_name};
"##,
        tag_name = tag_name,
        wgsl = wgsl,
        class_name = class_name,
        buffer_size = buffer_size,
        param_count = output.params.len(),
        observed_attrs = observed_attrs,
        prop_accessors = prop_accessors,
        data_init = data_init,
        param_update_inline = param_update_inline,
        arc_component_update = generate_arc_component_js(&output.arc_moments),
    )
}

/// Convert a kebab-case tag name to PascalCase class name.
/// e.g. "game-loading" → "GameLoading"
fn tag_to_class_name(tag: &str) -> String {
    tag.split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

/// Generate `observedAttributes` array content.
fn generate_observed_attrs(data_fields: &[String]) -> String {
    data_fields
        .iter()
        .map(|f| format!("'{f}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Generate property getter/setter pairs for data fields.
fn generate_prop_accessors(data_fields: &[String]) -> String {
    let mut lines = Vec::new();
    for field in data_fields {
        lines.push(format!(
            "  set {field}(v) {{ this._data.{field} = typeof v === 'number' ? v : parseFloat(v) || 0; }}"
        ));
        lines.push(format!("  get {field}() {{ return this._data.{field}; }}"));
        lines.push(String::new());
    }
    lines.join("\n")
}

/// Generate data field initializers for constructor.
fn generate_data_init(data_fields: &[String]) -> String {
    data_fields
        .iter()
        .map(|f| format!("{f}: 0"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Generate inline param update code for web component frame loop.
fn generate_component_param_update(
    params: &[crate::codegen::CompiledParam],
    data_fields: &[String],
) -> String {
    if params.is_empty() {
        return String::new();
    }

    let mut lines = Vec::new();

    // Declare data variables from component properties
    for field in data_fields {
        lines.push(format!("    const data_{field} = this._data.{field} || 0;"));
    }

    // Param value computation
    for (i, p) in params.iter().enumerate() {
        let mod_expr = p.mod_js.as_deref().unwrap_or("0");
        // Use arc base override if available, otherwise static base value
        lines.push(format!(
            "    this._paramValues[{i}] = (this._arcBaseOverrides?.[{i}] ?? {}) + {};",
            p.base_value, mod_expr
        ));
    }

    lines.join("\n")
}

// ═══════════════════════════════════════════════════════════════════════
// Arc timeline JS generation
// ═══════════════════════════════════════════════════════════════════════

use crate::codegen::CompiledMoment;

/// Shared easing function library (emitted once, used by both HTML and component).
const EASING_FUNCTIONS_JS: &str = r#"const ease = {
  linear: t => t,
  expo_in: t => t === 0 ? 0 : Math.pow(2, 10 * (t - 1)),
  expo_out: t => t === 1 ? 1 : 1 - Math.pow(2, -10 * t),
  cubic_in_out: t => t < 0.5 ? 4*t*t*t : 1 - Math.pow(-2*t + 2, 3) / 2,
  smooth: t => t * t * (3 - 2 * t),
  elastic: t => t === 0 || t === 1 ? t : -Math.pow(2, 10*t - 10) * Math.sin((t*10 - 10.75) * (2*Math.PI/3)),
  bounce: t => { const n=7.5625, d=2.75; if(t<1/d) return n*t*t; if(t<2/d) return n*(t-=1.5/d)*t+0.75; if(t<2.5/d) return n*(t-=2.25/d)*t+0.9375; return n*(t-=2.625/d)*t+0.984375; },
};"#;

/// Generate JS arc timeline code for the HTML runtime.
/// Emits: easing functions, timeline data, arcUpdate() function that modifies params[].base.
fn generate_arc_js(moments: &[CompiledMoment]) -> String {
    if moments.is_empty() {
        return "function arcUpdate() {}".to_string();
    }

    let mut js = String::with_capacity(1024);

    // Easing library
    js.push_str(EASING_FUNCTIONS_JS);
    js.push('\n');

    // Timeline data: array of { t, transitions: [{ paramIdx, target, animated, ease, dur }] }
    js.push_str("const arcTimeline = [\n");
    for (i, m) in moments.iter().enumerate() {
        let name_str = m.name.as_deref().unwrap_or("");
        js.push_str(&format!(
            "  {{ t: {}, name: '{}', transitions: [",
            m.time_seconds, name_str
        ));
        for t in &m.transitions {
            let dur = match t.duration_secs {
                Some(d) => format!("{d}"),
                None => {
                    // Duration until next moment (or 1s if last)
                    let next_t = moments.get(i + 1).map(|m| m.time_seconds).unwrap_or(m.time_seconds + 1.0);
                    format!("{}", next_t - m.time_seconds)
                }
            };
            js.push_str(&format!(
                "{{ pi: {}, to: {}, anim: {}, ease: '{}', dur: {} }},",
                t.param_index, t.target_value, t.is_animated, t.easing, dur
            ));
        }
        js.push_str("] },\n");
    }
    js.push_str("];\n\n");

    // Track "from" values (snapshotted when a transition starts)
    js.push_str("const arcState = new Map();\n");

    // arcUpdate function — called each frame with current time
    js.push_str(r#"function arcUpdate(time) {
  for (let mi = 0; mi < arcTimeline.length; mi++) {
    const m = arcTimeline[mi];
    if (time < m.t) continue;
    for (const tr of m.transitions) {
      const key = `${mi}_${tr.pi}`;
      if (!tr.anim) {
        // Instant set — override base immediately
        params[tr.pi].base = tr.to;
        continue;
      }
      // Animated transition
      const elapsed = time - m.t;
      const progress = Math.min(elapsed / tr.dur, 1.0);
      // Snapshot "from" value on first encounter
      if (!arcState.has(key)) {
        arcState.set(key, params[tr.pi].base);
      }
      const from = arcState.get(key);
      const easeFn = ease[tr.ease] || ease.linear;
      const t = easeFn(progress);
      params[tr.pi].base = from + (tr.to - from) * t;
    }
  }
}
"#);

    js
}

/// Generate inline arc update code for Web Component _frame() method.
fn generate_arc_component_js(moments: &[CompiledMoment]) -> String {
    if moments.is_empty() {
        return String::new();
    }

    let mut js = String::with_capacity(512);

    // Initialize arc data on first frame
    js.push_str("    if (!this._arcTimeline) {\n");
    js.push_str("      ");
    js.push_str(EASING_FUNCTIONS_JS);
    js.push('\n');
    js.push_str("      this._arcEase = ease;\n");
    js.push_str("      this._arcTimeline = [\n");

    for (i, m) in moments.iter().enumerate() {
        let name_str = m.name.as_deref().unwrap_or("");
        js.push_str(&format!(
            "        {{ t: {}, name: '{}', transitions: [",
            m.time_seconds, name_str
        ));
        for t in &m.transitions {
            let dur = match t.duration_secs {
                Some(d) => format!("{d}"),
                None => {
                    let next_t = moments.get(i + 1).map(|m| m.time_seconds).unwrap_or(m.time_seconds + 1.0);
                    format!("{}", next_t - m.time_seconds)
                }
            };
            js.push_str(&format!(
                "{{ pi: {}, to: {}, anim: {}, ease: '{}', dur: {} }},",
                t.param_index, t.target_value, t.is_animated, t.easing, dur
            ));
        }
        js.push_str("] },\n");
    }

    js.push_str("      ];\n");
    js.push_str("      this._arcState = new Map();\n");
    js.push_str("    }\n");

    // Inline arc update
    js.push_str(r#"    for (let mi = 0; mi < this._arcTimeline.length; mi++) {
      const m = this._arcTimeline[mi];
      if (time < m.t) continue;
      for (const tr of m.transitions) {
        const key = `${mi}_${tr.pi}`;
        if (!tr.anim) {
          this._arcBaseOverrides = this._arcBaseOverrides || {};
          this._arcBaseOverrides[tr.pi] = tr.to;
          continue;
        }
        const elapsed = time - m.t;
        const progress = Math.min(elapsed / tr.dur, 1.0);
        if (!this._arcState.has(key)) {
          this._arcState.set(key, this._arcBaseOverrides?.[tr.pi] ?? 0);
        }
        const from = this._arcState.get(key);
        const easeFn = this._arcEase[tr.ease] || this._arcEase.linear;
        const t = easeFn(progress);
        this._arcBaseOverrides = this._arcBaseOverrides || {};
        this._arcBaseOverrides[tr.pi] = from + (tr.to - from) * t;
      }
    }
"#);

    js
}
