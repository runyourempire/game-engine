//! Full-page HTML runtime generation — produces a self-contained HTML file
//! that runs a compiled GAME cinematic using the WebGPU API with audio
//! reactivity, parameter modulation, mouse tracking, and arc timelines.

use crate::codegen::CompileOutput;
use super::arc::generate_arc_js;
use super::helpers::*;

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
        warnings: Vec::new(),
        resonance_js: String::new(),
        react_js: String::new(),
        layer_count: 0,
        glsl_vertex: String::new(),
        glsl_fragment: String::new(),
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
{warnings_js}
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

// ── Resonance (cross-layer modulation) ──────────────────────────
{resonance_js}

// ── React (user interaction) ─────────────────────────────────────
{react_js}

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
        warnings_js = generate_warnings_js(&output.warnings),
        data_vars_js = generate_data_vars_js(&output.data_fields),
        arc_js = generate_arc_js(&output.arc_moments),
        resonance_js = output.resonance_js,
        react_js = output.react_js,
        param_init_js = param_init_js,
        param_update_js = param_update_js,
        overlay_display = overlay_display,
    )
}
