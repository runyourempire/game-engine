use crate::codegen::CompileOutput;
use super::util::{serde_json_inline, format_time};

pub(super) fn build_inline_js(output: &CompileOutput, html_json: &str, has_timeline: bool, duration: f64, tag_name: &str) -> String {
    let wgsl_raw_json = serde_json_inline(&output.wgsl);
    let dur_fmt = format_time(duration);

    let timeline_js = if has_timeline {
        format!(r#"
  // Timeline
  var tlPlaying = true;
  var tlDuration = {duration};
  var tlPausedAt = 0;
  var tlDragActive = false;
  var tlPlay = document.getElementById('tl-play');
  var tlFill = document.getElementById('tl-fill');
  var tlHead = document.getElementById('tl-head');
  var tlRail = document.getElementById('tl-rail');
  var tlTime = document.getElementById('tl-time');
  var pauseBtn = document.getElementById('pause-btn');
  var comp = document.getElementById('comp');

  function fmtTime(s) {{
    var m = Math.floor(s / 60);
    var sec = Math.floor(s % 60);
    return m + ':' + (sec < 10 ? '0' : '') + sec;
  }}

  function getTlTime() {{
    if (!comp) return 0;
    var now = performance.now() / 1000;
    var start = comp._startTime || now;
    return now - start;
  }}

  function setTlTime(t) {{
    if (!comp) return;
    var now = performance.now() / 1000;
    comp._startTime = now - t;
  }}

  function updateTlUI(t) {{
    var frac = tlDuration > 0 ? Math.min(t / tlDuration, 1) : 0;
    tlFill.style.width = (frac * 100) + '%';
    tlHead.style.left = (frac * 100) + '%';
    tlTime.textContent = fmtTime(t) + ' / {dur_fmt}';
  }}

  function togglePause() {{
    if (tlPlaying) {{
      tlPausedAt = getTlTime();
      tlPlaying = false;
      if (comp) comp._paused = true;
      tlPlay.innerHTML = '&#9654;';
      if (pauseBtn) pauseBtn.innerHTML = '&#9654;';
    }} else {{
      setTlTime(tlPausedAt);
      tlPlaying = true;
      if (comp) comp._paused = false;
      tlPlay.innerHTML = '&#9646;&#9646;';
      if (pauseBtn) pauseBtn.innerHTML = '&#9646;&#9646;';
    }}
  }}

  function seekTo(t) {{
    t = Math.max(0, Math.min(t, tlDuration));
    if (tlPlaying) {{
      setTlTime(t);
    }} else {{
      tlPausedAt = t;
      setTlTime(t);
    }}
    updateTlUI(t);
  }}

  tlPlay.addEventListener('click', togglePause);

  // Rail click
  tlRail.addEventListener('mousedown', function(e) {{
    if (e.target.classList.contains('timeline-moment')) return;
    if (e.target.classList.contains('timeline-head')) {{ tlDragActive = true; return; }}
    var rect = tlRail.getBoundingClientRect();
    var frac = (e.clientX - rect.left) / rect.width;
    seekTo(frac * tlDuration);
  }});

  // Head drag
  tlHead.addEventListener('mousedown', function(e) {{
    tlDragActive = true;
    e.preventDefault();
  }});
  document.addEventListener('mousemove', function(e) {{
    if (!tlDragActive) return;
    var rect = tlRail.getBoundingClientRect();
    var frac = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    seekTo(frac * tlDuration);
  }});
  document.addEventListener('mouseup', function() {{ tlDragActive = false; }});

  // Moment buttons
  document.querySelectorAll('.timeline-moment, .timeline-moment-btn').forEach(function(el) {{
    el.addEventListener('click', function() {{
      var t = parseFloat(el.dataset.time);
      if (!isNaN(t)) seekTo(t);
    }});
  }});"#,
            duration = duration,
            dur_fmt = dur_fmt,
        )
    } else {
        String::new()
    };

    let timeline_raf = if has_timeline {
        r#"
    if (typeof tlPlaying !== 'undefined' && tlPlaying && typeof getTlTime === 'function') {
      var t = getTlTime();
      updateTlUI(t);
    }"#
    } else {
        ""
    };

    let timeline_keys = if has_timeline {
        r#"
      case ' ': e.preventDefault(); togglePause(); break;
      case 'ArrowLeft': seekTo((tlPlaying ? getTlTime() : tlPausedAt) - 1); break;
      case 'ArrowRight': seekTo((tlPlaying ? getTlTime() : tlPausedAt) + 1); break;
      case 'Home': seekTo(0); break;"#
    } else {
        ""
    };

    format!(r##"<script type="module" src="/component.js"></script>
<script>
  // Inject HTML preview
  var html = {html_json};
  document.getElementById('preview').srcdoc = html;

  // Tab switching
  var wgslHighlighted = false;
  document.querySelectorAll('.tab-btn').forEach(function(btn) {{
    btn.addEventListener('click', function() {{
      var tab = btn.dataset.tab;
      document.querySelectorAll('.tab-btn').forEach(function(b) {{ b.classList.remove('active'); }});
      btn.classList.add('active');
      document.querySelectorAll('.tab-pane').forEach(function(p) {{ p.classList.remove('active'); }});
      var pane = document.getElementById('pane-' + tab);
      if (pane) pane.classList.add('active');
      if (tab === 'wgsl' && !wgslHighlighted) {{ highlightWgsl(); wgslHighlighted = true; }}
    }});
  }});

  // WGSL syntax highlighting
  function highlightWgsl() {{
    var el = document.getElementById('wgsl-code');
    if (!el) return;
    el.querySelectorAll('.line').forEach(function(line) {{
      var t = line.innerHTML;
      // Comments first (greedy per line)
      t = t.replace(/(\/\/.*)$/gm, '<span class="wgsl-comment">$1</span>');
      // Decorators
      t = t.replace(/@(\w+)/g, '<span class="wgsl-deco">@$1</span>');
      // Keywords
      t = t.replace(/\b(fn|var|let|return|struct|if|for|else|loop|break|continue|switch|case|default|while)\b/g, '<span class="wgsl-kw">$1</span>');
      // Types
      t = t.replace(/\b(f32|vec2f|vec4f|vec3f|u32|i32|bool|array|mat4x4f|mat3x3f|ptr)\b/g, '<span class="wgsl-type">$1</span>');
      // Numbers
      t = t.replace(/\b(\d+\.?\d*)\b/g, '<span class="wgsl-num">$1</span>');
      line.innerHTML = t;
    }});
  }}

  // Copy WGSL
  function copyWgsl() {{
    var raw = {wgsl_raw_json};
    navigator.clipboard.writeText(raw);
  }}

  // API panel code block copy
  function copyInner(el) {{
    var ta = document.createElement('textarea');
    ta.innerHTML = el.textContent;
    navigator.clipboard.writeText(ta.value);
  }}

  // Size toggle
  function setSize(size, btn) {{
    var wrapper = document.getElementById('comp-wrapper');
    wrapper.className = 'comp-wrapper size-' + size;
    document.querySelectorAll('.size-bar button').forEach(function(b) {{ b.classList.remove('active'); }});
    btn.classList.add('active');
  }}

  // Copy helper
  function copyText(text, btn) {{
    var ta = document.createElement('textarea');
    ta.innerHTML = text;
    navigator.clipboard.writeText(ta.value).then(function() {{
      btn.classList.add('copied');
      var orig = btn.textContent;
      btn.textContent = 'Copied!';
      setTimeout(function() {{ btn.classList.remove('copied'); btn.textContent = orig; }}, 1200);
    }});
  }}

  // Slider wiring (data params only)
  document.querySelectorAll('.param-slider input[type="range"]').forEach(function(slider) {{
    var field = slider.dataset.field;
    var valSpan = slider.closest('.param-row').querySelector('.param-val');
    var comp = document.getElementById('comp');
    if (comp && field) comp[field] = parseFloat(slider.value);
    slider.addEventListener('input', function() {{
      var v = parseFloat(slider.value);
      if (valSpan) valSpan.textContent = v.toFixed(2);
      if (comp && field) comp[field] = v;
    }});
  }});

  // Divider drag-resize
  var divider = document.getElementById('divider');
  var split = divider.parentElement;
  var dragging = false;
  divider.addEventListener('mousedown', function(e) {{
    dragging = true;
    e.preventDefault();
  }});
  document.addEventListener('mousemove', function(e) {{
    if (!dragging) return;
    var rect = split.getBoundingClientRect();
    var frac = (e.clientX - rect.left) / rect.width;
    var clamped = Math.max(0.2, Math.min(0.8, frac));
    split.style.gridTemplateColumns = clamped + 'fr 4px ' + (1 - clamped) + 'fr';
  }});
  document.addEventListener('mouseup', function() {{ dragging = false; }});

  {timeline_js}

  // Param monitor RAF
  (function() {{
    var comp = document.getElementById('comp');
    function tick() {{
      if (comp && comp._paramValues) {{
        document.querySelectorAll('[data-live]').forEach(function(el) {{
          var idx = parseInt(el.dataset.live);
          var v = comp._paramValues[idx];
          if (v !== undefined) el.textContent = v.toFixed(2);
        }});
      }}
      {timeline_raf}
      requestAnimationFrame(tick);
    }}
    requestAnimationFrame(tick);
  }})();

  // Shortcuts overlay
  function toggleShortcuts() {{
    var ov = document.getElementById('shortcut-overlay');
    ov.classList.toggle('open');
  }}

  // ── X-Ray Mode ──────────────────────────────────────────────────
  var xrayPipelines = [];
  var xrayOriginalPipeline = null;
  var xrayLoaded = false;
  var xrayActive = -1; // -1 = showing original (all stages)

  function toggleXray() {{
    var bar = document.getElementById('xray-bar');
    bar.classList.toggle('visible');
    if (!xrayLoaded) {{
      xrayLoaded = true;
      loadXrayVariants();
    }}
  }}

  function loadXrayVariants() {{
    fetch('/xray.json')
      .then(function(r) {{ return r.json(); }})
      .then(function(data) {{
        var bar = document.getElementById('xray-bar');
        bar.innerHTML = '<span class="xray-label">X-Ray</span>';
        // "All" chip
        var allChip = document.createElement('button');
        allChip.className = 'xray-chip all active';
        allChip.textContent = 'All';
        allChip.addEventListener('click', function() {{ selectXray(-1, allChip); }});
        bar.appendChild(allChip);

        var lastLayer = -1;
        var comp = document.getElementById('comp');
        if (comp && comp._pipeline) {{
          xrayOriginalPipeline = comp._pipeline;
        }}

        data.variants.forEach(function(v, idx) {{
          // Layer separator
          if (v.layer !== lastLayer && lastLayer !== -1) {{
            var sep = document.createElement('span');
            sep.className = 'xray-sep';
            bar.appendChild(sep);
          }}
          lastLayer = v.layer;

          var chip = document.createElement('button');
          chip.className = 'xray-chip';
          chip.textContent = v.stageName;
          chip.title = v.layerName + '.' + v.stageName + ' (stage ' + v.stage + ')';
          chip.addEventListener('click', function() {{ selectXray(idx, chip); }});
          bar.appendChild(chip);

          // Pre-create pipeline
          if (comp && comp._device) {{
            var mod = comp._device.createShaderModule({{ code: v.wgsl }});
            var layout = comp._pipeline.getBindGroupLayout
              ? comp._device.createPipelineLayout({{ bindGroupLayouts: [comp._bindGroup.layout || comp._device.createBindGroupLayout({{ entries: [{{ binding: 0, visibility: 3, buffer: {{ type: 'uniform' }} }}] }})] }})
              : comp._pipeline.layout;
            comp._device.createRenderPipeline({{
              layout: comp._device.createPipelineLayout({{ bindGroupLayouts: [comp._device.createBindGroupLayout({{ entries: [{{ binding: 0, visibility: 3, buffer: {{ type: 'uniform' }} }}] }})] }}),
              vertex: {{ module: mod, entryPoint: 'vs_main' }},
              fragment: {{ module: mod, entryPoint: 'fs_main', targets: [{{ format: comp._format }}] }},
              primitive: {{ topology: 'triangle-strip' }},
            }}).then(function() {{}}).catch(function() {{}});
            // Store promise-free sync pipeline creation
            try {{
              xrayPipelines[idx] = comp._device.createRenderPipeline({{
                layout: comp._device.createPipelineLayout({{ bindGroupLayouts: [comp._device.createBindGroupLayout({{ entries: [{{ binding: 0, visibility: 3, buffer: {{ type: 'uniform' }} }}] }})] }}),
                vertex: {{ module: comp._device.createShaderModule({{ code: v.wgsl }}), entryPoint: 'vs_main' }},
                fragment: {{ module: comp._device.createShaderModule({{ code: v.wgsl }}), entryPoint: 'fs_main', targets: [{{ format: comp._format }}] }},
                primitive: {{ topology: 'triangle-strip' }},
              }});
            }} catch(e) {{ console.warn('X-Ray pipeline error for ' + v.stageName + ':', e); }}
          }}
        }});
      }})
      .catch(function(e) {{ console.warn('X-Ray load error:', e); }});
  }}

  function selectXray(idx, chip) {{
    document.querySelectorAll('.xray-chip').forEach(function(c) {{ c.classList.remove('active'); }});
    chip.classList.add('active');
    var comp = document.getElementById('comp');
    if (!comp) return;
    if (idx === -1) {{
      if (xrayOriginalPipeline) comp._pipeline = xrayOriginalPipeline;
    }} else if (xrayPipelines[idx]) {{
      if (!xrayOriginalPipeline) xrayOriginalPipeline = comp._pipeline;
      comp._pipeline = xrayPipelines[idx];
    }}
    xrayActive = idx;
  }}

  // ── Live Editor ──────────────────────────────────────────────────
  var editorTagName = '{tag_name}';
  var editorTimeout = null;
  var editorLastSource = '';

  function editorCompile() {{
    var ta = document.getElementById('editor-source');
    var status = document.getElementById('editor-status');
    if (!ta) return;
    var src = ta.value;
    if (src === editorLastSource) return;
    editorLastSource = src;
    status.textContent = 'compiling...';
    status.className = 'editor-status saving';

    fetch('/compile', {{
      method: 'POST',
      headers: {{ 'Content-Type': 'application/json' }},
      body: JSON.stringify({{ source: src }})
    }})
    .then(function(r) {{ return r.json(); }})
    .then(function(data) {{
      if (data.error) {{
        status.textContent = data.error;
        status.className = 'editor-status err';
        return;
      }}
      status.textContent = 'OK';
      status.className = 'editor-status ok';

      // Update HTML preview iframe
      var preview = document.getElementById('preview');
      if (preview && data.component_js) {{
        var previewHtml = '<!DOCTYPE html><html><head><style>*{{margin:0;padding:0}}html,body{{width:100%;height:100%;background:#000}}</style></head><body><' + editorTagName + '></' + editorTagName + '><script type="module">' + data.component_js + '<\/script></body></html>';
        preview.srcdoc = previewHtml;
      }}

      // Pipeline hot-swap for live component
      var comp = document.getElementById('comp');
      if (comp && comp._device && data.wgsl) {{
        try {{
          var mod = comp._device.createShaderModule({{ code: data.wgsl }});
          var newPipeline = comp._device.createRenderPipeline({{
            layout: comp._device.createPipelineLayout({{ bindGroupLayouts: [comp._device.createBindGroupLayout({{ entries: [{{ binding: 0, visibility: 3, buffer: {{ type: 'uniform' }} }}] }})] }}),
            vertex: {{ module: mod, entryPoint: 'vs_main' }},
            fragment: {{ module: mod, entryPoint: 'fs_main', targets: [{{ format: comp._format }}] }},
            primitive: {{ topology: 'triangle-strip' }},
          }});
          comp._pipeline = newPipeline;
          if (typeof xrayOriginalPipeline !== 'undefined') {{
            xrayOriginalPipeline = newPipeline;
            xrayLoaded = false;
          }}
        }} catch(e) {{
          status.textContent = 'Pipeline: ' + e.message;
          status.className = 'editor-status err';
        }}
      }}
    }})
    .catch(function(e) {{
      status.textContent = 'Network error';
      status.className = 'editor-status err';
    }});
  }}

  function editorSave() {{
    var ta = document.getElementById('editor-source');
    var status = document.getElementById('editor-status');
    if (!ta) return;
    status.textContent = 'saving...';
    status.className = 'editor-status saving';

    fetch('/save', {{
      method: 'POST',
      headers: {{ 'Content-Type': 'application/json' }},
      body: JSON.stringify({{ source: ta.value }})
    }})
    .then(function(r) {{ return r.json(); }})
    .then(function(data) {{
      if (data.ok) {{
        status.textContent = 'Saved';
        status.className = 'editor-status ok';
        setTimeout(function() {{ status.textContent = ''; }}, 2000);
      }} else {{
        status.textContent = data.error || 'Save failed';
        status.className = 'editor-status err';
      }}
    }})
    .catch(function(e) {{
      status.textContent = 'Network error';
      status.className = 'editor-status err';
    }});
  }}

  // Debounced compile on editor input
  (function() {{
    var ta = document.getElementById('editor-source');
    if (ta) {{
      editorLastSource = ta.value;
      ta.addEventListener('input', function() {{
        clearTimeout(editorTimeout);
        editorTimeout = setTimeout(editorCompile, 300);
      }});
      ta.addEventListener('keydown', function(e) {{
        if (e.key === 'Tab') {{
          e.preventDefault();
          var start = ta.selectionStart;
          var end = ta.selectionEnd;
          ta.value = ta.value.substring(0, start) + '  ' + ta.value.substring(end);
          ta.selectionStart = ta.selectionEnd = start + 2;
          clearTimeout(editorTimeout);
          editorTimeout = setTimeout(editorCompile, 300);
        }}
      }});
    }}
  }})();

  // ── Pixel Autopsy ────────────────────────────────────────────────
  var autopsyActive = false;

  function toggleAutopsy() {{
    autopsyActive = !autopsyActive;
    document.body.classList.toggle('autopsy-mode', autopsyActive);
    if (!autopsyActive) {{
      document.getElementById('autopsy-tooltip').classList.remove('visible');
    }}
  }}

  document.getElementById('comp-wrapper').addEventListener('click', function(e) {{
    if (!autopsyActive) return;
    var comp = document.getElementById('comp');
    if (!comp || !comp.shadowRoot) return;
    var canvas = comp.shadowRoot.querySelector('canvas');
    if (!canvas) return;

    var rect = canvas.getBoundingClientRect();
    var cssX = e.clientX - rect.left;
    var cssY = e.clientY - rect.top;
    var dpr = window.devicePixelRatio || 1;
    var px = Math.floor(cssX * dpr);
    var py = Math.floor(cssY * dpr);
    var u = cssX / rect.width;
    var v = cssY / rect.height;
    var dist = Math.sqrt((u - 0.5) * (u - 0.5) + (v - 0.5) * (v - 0.5));

    var tip = document.getElementById('autopsy-tooltip');
    try {{
      var off = document.createElement('canvas');
      off.width = canvas.width;
      off.height = canvas.height;
      var ctx = off.getContext('2d');
      ctx.drawImage(canvas, 0, 0);
      var d = ctx.getImageData(px, py, 1, 1).data;
      var hex = '#' + [d[0], d[1], d[2]].map(function(c) {{ return ('0' + c.toString(16)).slice(-2); }}).join('');

      tip.innerHTML =
        '<div class="autopsy-swatch" style="background:' + hex + '"></div>' +
        '<div class="autopsy-row"><span class="autopsy-label">px</span><span>' + px + ', ' + py + '</span></div>' +
        '<div class="autopsy-row"><span class="autopsy-label">UV</span><span>' + u.toFixed(3) + ', ' + v.toFixed(3) + '</span></div>' +
        '<div class="autopsy-row"><span class="autopsy-label">RGBA</span><span>' + d[0] + ', ' + d[1] + ', ' + d[2] + ', ' + d[3] + '</span></div>' +
        '<div class="autopsy-row"><span class="autopsy-label">hex</span><span>' + hex + '</span></div>' +
        '<div class="autopsy-row"><span class="autopsy-label">dist</span><span>' + dist.toFixed(3) + '</span></div>';
    }} catch(err) {{
      tip.innerHTML = '<div class="autopsy-row" style="color:#EF4444">Read error: ' + err.message + '</div>';
    }}
    tip.style.left = Math.min(e.clientX + 16, window.innerWidth - 200) + 'px';
    tip.style.top = Math.min(e.clientY + 16, window.innerHeight - 160) + 'px';
    tip.classList.add('visible');
  }});

  document.addEventListener('click', function(e) {{
    if (autopsyActive && !e.target.closest('#comp-wrapper') && !e.target.closest('#autopsy-tooltip')) {{
      document.getElementById('autopsy-tooltip').classList.remove('visible');
    }}
    // Close export dropdown on outside click
    if (!e.target.closest('.export-dropdown')) {{
      document.querySelectorAll('.export-dropdown').forEach(function(d) {{ d.classList.remove('open'); }});
    }}
  }});

  // ── Export (PNG & Video) ───────────────────────────────────────────
  function exportPng() {{
    var comp = document.getElementById('comp');
    if (!comp || !comp.shadowRoot) return;
    var canvas = comp.shadowRoot.querySelector('canvas');
    if (!canvas) return;
    try {{
      var off = document.createElement('canvas');
      off.width = canvas.width;
      off.height = canvas.height;
      var ctx = off.getContext('2d');
      ctx.drawImage(canvas, 0, 0);
      off.toBlob(function(blob) {{
        var url = URL.createObjectURL(blob);
        var a = document.createElement('a');
        a.href = url;
        a.download = editorTagName + '.png';
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
      }}, 'image/png');
    }} catch(e) {{ console.warn('PNG export error:', e); }}
  }}

  function exportVideo() {{
    var comp = document.getElementById('comp');
    if (!comp || !comp.shadowRoot) return;
    var canvas = comp.shadowRoot.querySelector('canvas');
    if (!canvas) return;
    try {{
      var stream = canvas.captureStream(30);
      var recorder = new MediaRecorder(stream, {{ mimeType: 'video/webm; codecs=vp9' }});
      var chunks = [];
      recorder.ondataavailable = function(e) {{ if (e.data.size > 0) chunks.push(e.data); }};
      recorder.onstop = function() {{
        var blob = new Blob(chunks, {{ type: 'video/webm' }});
        var url = URL.createObjectURL(blob);
        var a = document.createElement('a');
        a.href = url;
        a.download = editorTagName + '.webm';
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
      }};
      recorder.start();
      setTimeout(function() {{ recorder.stop(); }}, 5000);
    }} catch(e) {{ console.warn('Video export error:', e); }}
  }}

  // Keyboard shortcuts
  document.addEventListener('keydown', function(e) {{
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {{
      e.preventDefault();
      editorSave();
      return;
    }}
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
    switch(e.key) {{
      case '1': document.querySelector('[data-tab="preview"]').click(); break;
      case '2': document.querySelector('[data-tab="wgsl"]').click(); break;
      case '3': document.querySelector('[data-tab="api"]').click(); break;
      case '4': document.querySelector('[data-tab="editor"]').click(); break;
      case 's': case 'S': document.querySelector('.size-bar button:nth-child(1)').click(); break;
      case 'm': case 'M': document.querySelector('.size-bar button:nth-child(2)').click(); break;
      case 'l': case 'L': document.querySelector('.size-bar button:nth-child(3)').click(); break;
      case 'x': case 'X': toggleXray(); break;
      case 'p': case 'P': toggleAutopsy(); break;
      case '?': toggleShortcuts(); break;
      {timeline_keys}
    }}
  }});
</script>"##,
        html_json = html_json,
        wgsl_raw_json = wgsl_raw_json,
        tag_name = tag_name,
        timeline_js = timeline_js,
        timeline_raf = timeline_raf,
        timeline_keys = timeline_keys,
    )
}
