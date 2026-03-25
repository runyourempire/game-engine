import * as vscode from "vscode";
import * as cp from "child_process";
import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import { TunableToken } from "./parameterProvider";

export class PreviewPanel {
  public static currentPanel: PreviewPanel | undefined;
  private static _tunerDragging = false;
  private static _editInFlight = false;
  private readonly _panel: vscode.WebviewPanel;
  private readonly _extensionUri: vscode.Uri;
  private _disposables: vscode.Disposable[] = [];
  private _compileTimeout: NodeJS.Timeout | undefined;
  private _compileProcess: cp.ChildProcess | undefined;

  public static isTunerActive(): boolean {
    return PreviewPanel._tunerDragging || PreviewPanel._editInFlight;
  }

  public static createOrShow(extensionUri: vscode.Uri): void {
    const column = vscode.ViewColumn.Beside;
    if (PreviewPanel.currentPanel) {
      PreviewPanel.currentPanel._panel.reveal(column);
      return;
    }
    const panel = vscode.window.createWebviewPanel(
      "gamePreview",
      "GAME Preview",
      column,
      {
        enableScripts: true,
        retainContextWhenHidden: true,
      }
    );
    PreviewPanel.currentPanel = new PreviewPanel(panel, extensionUri);
  }

  public static updateCode(code: string): void {
    if (!PreviewPanel.currentPanel) return;
    PreviewPanel.currentPanel._scheduleCompile(code);
  }

  public static showTuner(token: TunableToken): void {
    PreviewPanel.currentPanel?._panel.webview.postMessage({
      type: "showTuner",
      ...token,
    });
  }

  public static hideTuner(): void {
    PreviewPanel.currentPanel?._panel.webview.postMessage({ type: "hideTuner" });
  }

  private constructor(panel: vscode.WebviewPanel, extensionUri: vscode.Uri) {
    this._panel = panel;
    this._extensionUri = extensionUri;
    this._panel.webview.html = this._getHtml();
    this._panel.onDidDispose(() => this.dispose(), null, this._disposables);

    // Handle messages from WebView (tuner value changes + drag state)
    this._panel.webview.onDidReceiveMessage(
      (msg) => {
        if (msg.type === "tunerDragStart") {
          PreviewPanel._tunerDragging = true;
        } else if (msg.type === "tunerDragEnd") {
          PreviewPanel._tunerDragging = false;
        } else if (msg.type === "tunerChange") {
          this._applyTunerEdit(msg);
        }
      },
      null,
      this._disposables
    );

    // Send initial code if editor is active
    const editor = vscode.window.activeTextEditor;
    if (editor?.document.languageId === "game") {
      this._scheduleCompile(editor.document.getText());
    }
  }

  private _scheduleCompile(code: string): void {
    if (this._compileTimeout) clearTimeout(this._compileTimeout);
    this._compileTimeout = setTimeout(() => this._compile(code), 300);
  }

  private _compile(code: string): void {
    const config = vscode.workspace.getConfiguration("game");
    const serverPath = config.get<string>("serverPath", "game");

    const tmp = os.tmpdir();
    const inputPath = path.join(tmp, `game-preview-${process.pid}.game`);
    const outputDir = path.join(tmp, `game-preview-out-${process.pid}`);

    fs.writeFileSync(inputPath, code);
    fs.mkdirSync(outputDir, { recursive: true });

    // Clean stale output before compile
    const oldFiles = fs.readdirSync(outputDir).filter((f: string) => f.endsWith('.js') || f.endsWith('.d.ts'));
    oldFiles.forEach((f: string) => fs.unlinkSync(path.join(outputDir, f)));

    this._compileProcess = cp.exec(
      `"${serverPath}" build "${inputPath}" -o "${outputDir}"`,
      { timeout: 10000 },
      (err, _stdout, stderr) => {
        this._compileProcess = undefined;
        // Check for disposal before posting messages
        if (!PreviewPanel.currentPanel) return;

        if (err) {
          let msg = stderr || err.message;
          if (msg.includes("ENOENT") || msg.includes("not found") || msg.includes("not recognized")) {
            msg = "GAME compiler not found. Set game.serverPath in VS Code settings.";
          }
          this._panel.webview.postMessage({ type: "error", message: msg });
          return;
        }

        // Show stderr warnings on successful compile
        if (stderr.trim()) {
          this._panel.webview.postMessage({
            type: "warning",
            message: stderr.trim(),
          });
        }

        const files = fs
          .readdirSync(outputDir)
          .filter((f: string) => f.endsWith(".js"));
        if (files.length === 0) {
          this._panel.webview.postMessage({
            type: "error",
            message: "No output generated",
          });
          return;
        }
        const js = fs.readFileSync(path.join(outputDir, files[0]), "utf-8");
        const name = files[0].replace(".js", "");
        this._panel.webview.postMessage({ type: "compiled", js, name });
      }
    );
  }

  private _getHtml(): string {
    return `<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  html, body { width: 100%; height: 100%; background: #0a0a0a; overflow: hidden; }
  #container {
    width: 100%; height: 100%;
    display: flex; align-items: center; justify-content: center;
    position: relative;
  }
  #component-host {
    width: 80%; height: 80%;
    border-radius: 8px;
    overflow: hidden;
    background: #050505;
  }
  #preview-frame {
    width: 100%;
    height: 100%;
    border: none;
    background: #050505;
  }
  #status {
    position: absolute;
    bottom: 8px; left: 8px;
    font: 11px/1 'JetBrains Mono', 'SF Mono', monospace;
    color: #444;
    z-index: 10;
  }
  #status.error { color: #ef4444; }
  #status.warning { color: #d4af37; }
  #status.ok { color: #22c55e; }
  #error-overlay {
    position: absolute;
    top: 0; left: 0; right: 0;
    padding: 12px 16px;
    background: rgba(239, 68, 68, 0.1);
    border-bottom: 1px solid rgba(239, 68, 68, 0.2);
    font: 12px/1.4 'JetBrains Mono', monospace;
    color: #ef4444;
    display: none;
    z-index: 20;
    white-space: pre-wrap;
    max-height: 30%;
    overflow-y: auto;
  }
  #error-overlay.visible { display: block; }
  .empty-state {
    color: #333;
    font: 13px/1.5 -apple-system, BlinkMacSystemFont, sans-serif;
    text-align: center;
    padding: 40px;
  }
  .empty-state kbd {
    background: #1a1a1a;
    border: 1px solid #2a2a2a;
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 11px;
    font-family: inherit;
  }
  #tuner-overlay {
    position: absolute;
    bottom: 0; left: 0; right: 0;
    background: #1a1a1a;
    border-top: 1px solid #2a2a2a;
    padding: 10px 16px;
    z-index: 30;
    display: none;
    font: 12px/1.4 'JetBrains Mono', monospace;
    color: #a0a0a0;
  }
  #tuner-overlay.visible { display: block; }
  .tuner-header {
    display: flex; justify-content: space-between; align-items: center;
    margin-bottom: 8px; font-size: 11px; color: #666;
  }
  .tuner-header .context { color: #d4af37; }
  .tuner-close { cursor: pointer; color: #666; font-size: 14px; }
  .tuner-close:hover { color: #fff; }
  .slider-row {
    display: flex; align-items: center; gap: 10px;
  }
  .slider-row input[type=range] {
    flex: 1; height: 4px; -webkit-appearance: none; appearance: none;
    background: #2a2a2a; border-radius: 2px; outline: none;
  }
  .slider-row input[type=range]::-webkit-slider-thumb {
    -webkit-appearance: none; width: 14px; height: 14px;
    border-radius: 50%; background: #d4af37; cursor: pointer;
  }
  .slider-row .val {
    min-width: 50px; text-align: right; color: #fff;
    font-variant-numeric: tabular-nums;
  }
  .slider-row .bound { color: #444; font-size: 10px; min-width: 30px; }
  .slider-row .bound.max { text-align: right; }
  .color-row { display: flex; align-items: center; gap: 10px; }
  .color-row input[type=color] {
    width: 36px; height: 28px; border: 1px solid #2a2a2a;
    border-radius: 4px; background: none; cursor: pointer; padding: 0;
  }
  .color-row .hex-val { color: #fff; font-size: 12px; }
  .palette-grid {
    display: flex; flex-wrap: wrap; gap: 6px; max-height: 80px; overflow-y: auto;
  }
  .palette-btn {
    padding: 3px 8px; border-radius: 4px; border: 1px solid #2a2a2a;
    background: #141414; color: #a0a0a0; font-size: 11px; cursor: pointer;
    font-family: 'JetBrains Mono', monospace;
  }
  .palette-btn:hover { border-color: #d4af37; color: #fff; }
  .palette-btn.active { border-color: #d4af37; color: #d4af37; background: #1f1a0f; }
</style>
</head>
<body>
<div id="container" role="main" aria-label="GAME Preview">
  <div id="component-host" role="img" aria-label="Shader preview">
    <iframe id="preview-frame" sandbox="allow-scripts" title="GAME component preview"></iframe>
  </div>
  <div id="status" role="status" aria-live="polite">ready</div>
  <div id="error-overlay" role="alert" aria-live="assertive"></div>
  <div id="tuner-overlay" role="toolbar" aria-label="Parameter tuner">
    <div class="tuner-header">
      <span><span class="context" id="tuner-context"></span></span>
      <span class="tuner-close" id="tuner-close">&times;</span>
    </div>
    <div id="tuner-number" style="display:none">
      <div class="slider-row">
        <span class="bound min" id="tuner-min"></span>
        <input type="range" id="tuner-slider" step="0.01" aria-label="Value slider">
        <span class="bound max" id="tuner-max"></span>
        <span class="val" id="tuner-val"></span>
      </div>
    </div>
    <div id="tuner-color" style="display:none">
      <div class="color-row">
        <input type="color" id="tuner-color-input">
        <span class="hex-val" id="tuner-hex"></span>
      </div>
    </div>
    <div id="tuner-palette" style="display:none">
      <div class="palette-grid" id="tuner-palette-grid"></div>
    </div>
  </div>
</div>
<script>
  const status = document.getElementById('status');
  const errorOverlay = document.getElementById('error-overlay');
  const previewFrame = document.getElementById('preview-frame');

  const vscodeApi = acquireVsCodeApi();
  const tunerOverlay = document.getElementById('tuner-overlay');
  const tunerCtx = document.getElementById('tuner-context');
  const tunerClose = document.getElementById('tuner-close');
  const tunerNumber = document.getElementById('tuner-number');
  const tunerColor = document.getElementById('tuner-color');
  const tunerPalette = document.getElementById('tuner-palette');
  const tunerSlider = document.getElementById('tuner-slider');
  const tunerVal = document.getElementById('tuner-val');
  const tunerMin = document.getElementById('tuner-min');
  const tunerMax = document.getElementById('tuner-max');
  const tunerColorInput = document.getElementById('tuner-color-input');
  const tunerHex = document.getElementById('tuner-hex');
  const tunerPaletteGrid = document.getElementById('tuner-palette-grid');
  let activeTuner = null;
  let isDragging = false;
  let lastSendTime = 0;
  let pendingSend = null;

  // Throttled send — max ~30fps to avoid flooding async editor.edit()
  function throttledSend(msg) {
    const now = Date.now();
    const elapsed = now - lastSendTime;
    if (elapsed >= 33) {
      vscodeApi.postMessage(msg);
      lastSendTime = now;
      if (pendingSend) { clearTimeout(pendingSend); pendingSend = null; }
    } else {
      // Queue the latest value — only the last one matters
      if (pendingSend) clearTimeout(pendingSend);
      pendingSend = setTimeout(() => {
        vscodeApi.postMessage(msg);
        lastSendTime = Date.now();
        pendingSend = null;
      }, 33 - elapsed);
    }
  }

  tunerClose.addEventListener('click', () => {
    tunerOverlay.classList.remove('visible');
    activeTuner = null;
    isDragging = false;
    vscodeApi.postMessage({ type: 'tunerDragEnd' });
  });

  // --- SLIDER: drag tracking ---
  tunerSlider.addEventListener('pointerdown', () => {
    isDragging = true;
    vscodeApi.postMessage({ type: 'tunerDragStart' });
  });
  // Use document-level pointerup so we catch release even if cursor leaves slider
  document.addEventListener('pointerup', () => {
    if (isDragging) {
      isDragging = false;
      vscodeApi.postMessage({ type: 'tunerDragEnd' });
      // Send final value immediately (flush any pending throttle)
      if (pendingSend) { clearTimeout(pendingSend); pendingSend = null; }
      if (activeTuner) {
        const v = parseFloat(tunerSlider.value);
        const step = activeTuner.range?.step || 0.1;
        const decimals = step < 1 ? (step < 0.01 ? 3 : 2) : 0;
        const formatted = v.toFixed(decimals);
        vscodeApi.postMessage({
          type: 'tunerChange', value: formatted,
          line: activeTuner.line, col: activeTuner.col, endCol: activeTuner.endCol
        });
      }
    }
  });

  tunerSlider.addEventListener('input', () => {
    if (!activeTuner) return;
    const v = parseFloat(tunerSlider.value);
    const step = activeTuner.range?.step || 0.1;
    const decimals = step < 1 ? (step < 0.01 ? 3 : 2) : 0;
    const formatted = v.toFixed(decimals);
    tunerVal.textContent = formatted;
    throttledSend({
      type: 'tunerChange', value: formatted,
      line: activeTuner.line, col: activeTuner.col, endCol: activeTuner.endCol
    });
    // Optimistically update endCol for next send
    activeTuner.endCol = activeTuner.col + formatted.length;
  });

  // --- COLOR PICKER ---
  let colorActive = false;
  tunerColorInput.addEventListener('focus', () => {
    colorActive = true;
    isDragging = true;
    vscodeApi.postMessage({ type: 'tunerDragStart' });
  });
  tunerColorInput.addEventListener('blur', () => {
    colorActive = false;
    isDragging = false;
    vscodeApi.postMessage({ type: 'tunerDragEnd' });
  });
  tunerColorInput.addEventListener('input', () => {
    if (!activeTuner) return;
    const hex = tunerColorInput.value;
    tunerHex.textContent = hex;
    const r = (parseInt(hex.slice(1,3),16)/255).toFixed(2);
    const g = (parseInt(hex.slice(3,5),16)/255).toFixed(2);
    const b = (parseInt(hex.slice(5,7),16)/255).toFixed(2);
    const replacement = activeTuner.context === 'tint'
      ? 'tint(' + r + ', ' + g + ', ' + b + ')'
      : hex;
    throttledSend({
      type: 'tunerChange', value: replacement,
      line: activeTuner.line, col: activeTuner.col, endCol: activeTuner.endCol
    });
    activeTuner.endCol = activeTuner.col + replacement.length;
  });

  function showTunerUI(t) {
    activeTuner = t;
    tunerCtx.textContent = t.context + (t.kind === 'number' ? '(' + t.value + ')' : '');
    tunerNumber.style.display = t.kind === 'number' ? '' : 'none';
    tunerColor.style.display = t.kind === 'color' ? '' : 'none';
    tunerPalette.style.display = t.kind === 'palette' ? '' : 'none';

    if (t.kind === 'number' && t.range) {
      tunerSlider.min = t.range.min;
      tunerSlider.max = t.range.max;
      tunerSlider.step = t.range.step;
      tunerSlider.value = t.value;
      tunerVal.textContent = t.value;
      tunerMin.textContent = t.range.min;
      tunerMax.textContent = t.range.max;
    } else if (t.kind === 'color') {
      tunerColorInput.value = t.value;
      tunerHex.textContent = t.value;
    } else if (t.kind === 'palette' && t.palettes) {
      tunerPaletteGrid.innerHTML = '';
      t.palettes.forEach(p => {
        const btn = document.createElement('button');
        btn.className = 'palette-btn' + (p === t.value ? ' active' : '');
        btn.textContent = p;
        btn.onclick = () => {
          tunerPaletteGrid.querySelectorAll('.palette-btn').forEach(b => b.classList.remove('active'));
          btn.classList.add('active');
          // Mark as dragging briefly to prevent cursor-based reset
          isDragging = true;
          vscodeApi.postMessage({ type: 'tunerDragStart' });
          vscodeApi.postMessage({
            type: 'tunerChange', value: p,
            line: activeTuner.line, col: activeTuner.col, endCol: activeTuner.endCol
          });
          activeTuner.endCol = activeTuner.col + p.length;
          activeTuner.value = p;
          setTimeout(() => {
            isDragging = false;
            vscodeApi.postMessage({ type: 'tunerDragEnd' });
          }, 100);
        };
        tunerPaletteGrid.appendChild(btn);
      });
    }
    tunerOverlay.classList.add('visible');
  }

  window.addEventListener('message', event => {
    const msg = event.data;

    if (msg.type === 'showTuner') {
      // CRITICAL: ignore showTuner while dragging — prevents slider reset
      if (isDragging) return;
      showTunerUI(msg);
      return;
    }
    if (msg.type === 'hideTuner') {
      // Don't hide during drag — the cursor moved because of our edit
      if (isDragging) return;
      tunerOverlay.classList.remove('visible');
      activeTuner = null;
      return;
    }
    if (msg.type === 'tunerEndColUpdate') {
      // Extension confirmed the edit — update our tracked endCol
      if (activeTuner) {
        activeTuner.endCol = msg.endCol;
      }
      return;
    }

    if (msg.type === 'compiled') {
      errorOverlay.classList.remove('visible');

      // Build a self-contained HTML document for the iframe.
      // Each srcdoc assignment creates a fresh document context —
      // no Custom Elements Registry leak, no orphaned rAF loops.
      const tagName = 'game-' + msg.name.toLowerCase().replace(/[^a-z0-9-]/g, '-');
      const iframeDoc = '<!DOCTYPE html>' +
        '<html><head><style>' +
        '*{margin:0;padding:0;box-sizing:border-box}' +
        'html,body{width:100%;height:100%;background:#050505;overflow:hidden}' +
        tagName + '{display:block;width:100%;height:100%}' +
        '</style></head><body>' +
        '<' + tagName + '></' + tagName + '>' +
        '<script>' + msg.js + '<\\/script>' +
        '</body></html>';

      previewFrame.srcdoc = iframeDoc;

      status.textContent = msg.name + ' \\u2014 live';
      status.className = 'ok';
    }

    if (msg.type === 'warning') {
      status.textContent = msg.message.split('\\n')[0].substring(0, 80);
      status.className = 'warning';
    }

    if (msg.type === 'error') {
      status.textContent = 'compile error';
      status.className = 'error';
      errorOverlay.textContent = msg.message;
      errorOverlay.classList.add('visible');
    }
  });
</script>
</body>
</html>`;
  }

  private _applyTunerEdit(msg: {
    value: string;
    line: number;
    col: number;
    endCol: number;
  }): void {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== "game") return;
    if (PreviewPanel._editInFlight) return;

    // Validate line/col bounds against current document
    if (msg.line >= editor.document.lineCount) return;
    const lineText = editor.document.lineAt(msg.line).text;
    if (msg.col > lineText.length || msg.endCol > lineText.length) return;

    PreviewPanel._editInFlight = true;
    const range = new vscode.Range(msg.line, msg.col, msg.line, msg.endCol);
    editor.edit((b) => b.replace(range, msg.value)).then(
      (ok) => {
        PreviewPanel._editInFlight = false;
        if (ok) {
          // Send updated endCol back so WebView tracks the new range
          const newEndCol = msg.col + msg.value.length;
          this._panel.webview.postMessage({
            type: "tunerEndColUpdate",
            endCol: newEndCol,
          });
        }
      },
      () => {
        PreviewPanel._editInFlight = false;
      }
    );
  }

  public dispose(): void {
    PreviewPanel.currentPanel = undefined;
    if (this._compileProcess) {
      this._compileProcess.kill();
      this._compileProcess = undefined;
    }
    this._panel.dispose();
    if (this._compileTimeout) clearTimeout(this._compileTimeout);
    while (this._disposables.length) {
      const d = this._disposables.pop();
      if (d) d.dispose();
    }
  }
}
