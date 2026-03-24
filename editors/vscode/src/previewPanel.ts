import * as vscode from "vscode";
import * as cp from "child_process";
import * as path from "path";
import * as fs from "fs";
import * as os from "os";

export class PreviewPanel {
  public static currentPanel: PreviewPanel | undefined;
  private readonly _panel: vscode.WebviewPanel;
  private readonly _extensionUri: vscode.Uri;
  private _disposables: vscode.Disposable[] = [];
  private _compileTimeout: NodeJS.Timeout | undefined;

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

  private constructor(panel: vscode.WebviewPanel, extensionUri: vscode.Uri) {
    this._panel = panel;
    this._extensionUri = extensionUri;
    this._panel.webview.html = this._getHtml();
    this._panel.onDidDispose(() => this.dispose(), null, this._disposables);

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
    const inputPath = path.join(tmp, "game-preview.game");
    const outputDir = path.join(tmp, "game-preview-out");

    fs.writeFileSync(inputPath, code);
    fs.mkdirSync(outputDir, { recursive: true });

    cp.exec(
      `"${serverPath}" build "${inputPath}" -o "${outputDir}"`,
      (err, _stdout, stderr) => {
        if (err) {
          this._panel.webview.postMessage({
            type: "error",
            message: stderr || err.message,
          });
          return;
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
  #component-host > * {
    display: block;
    width: 100%;
    height: 100%;
  }
  #status {
    position: absolute;
    bottom: 8px; left: 8px;
    font: 11px/1 'JetBrains Mono', 'SF Mono', monospace;
    color: #444;
    z-index: 10;
  }
  #status.error { color: #ef4444; }
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
</style>
</head>
<body>
<div id="container">
  <div id="component-host">
    <div class="empty-state">
      <p>Edit a <code>.game</code> file to see live preview</p>
      <p style="margin-top:8px;color:#222">Changes render automatically</p>
    </div>
  </div>
  <div id="status">ready</div>
  <div id="error-overlay"></div>
</div>
<script>
  const host = document.getElementById('component-host');
  const status = document.getElementById('status');
  const errorOverlay = document.getElementById('error-overlay');
  let currentTag = null;
  let scriptElements = [];

  window.addEventListener('message', event => {
    const msg = event.data;

    if (msg.type === 'compiled') {
      // Clear previous component
      host.innerHTML = '';
      scriptElements.forEach(s => s.remove());
      scriptElements = [];
      errorOverlay.classList.remove('visible');

      // Custom elements can only be defined once per name, so append a
      // unique timestamp suffix for each recompile
      const timestamp = Date.now();
      const uniqueJs = msg.js.replace(
        /customElements\\.define\\('([^']+)'/,
        (match, tag) => {
          currentTag = tag + '-' + timestamp;
          return "customElements.define('" + currentTag + "'";
        }
      );

      // Inject the component script
      const script = document.createElement('script');
      script.textContent = uniqueJs;
      document.body.appendChild(script);
      scriptElements.push(script);

      // Create the element
      if (currentTag) {
        const el = document.createElement(currentTag);
        el.style.display = 'block';
        el.style.width = '100%';
        el.style.height = '100%';
        host.appendChild(el);

        status.textContent = msg.name + ' \\u2014 live';
        status.className = 'ok';
      }
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

  public dispose(): void {
    PreviewPanel.currentPanel = undefined;
    this._panel.dispose();
    if (this._compileTimeout) clearTimeout(this._compileTimeout);
    while (this._disposables.length) {
      const d = this._disposables.pop();
      if (d) d.dispose();
    }
  }
}
