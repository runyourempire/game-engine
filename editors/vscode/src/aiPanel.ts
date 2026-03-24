import * as vscode from "vscode";
import * as cp from "child_process";
import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import { AiProvider } from "./aiProvider";

export class AiPanel {
  public static currentPanel: AiPanel | undefined;
  private readonly _panel: vscode.WebviewPanel;
  private readonly _extensionUri: vscode.Uri;
  private readonly _aiProvider: AiProvider;
  private _disposables: vscode.Disposable[] = [];

  public static createOrShow(extensionUri: vscode.Uri): void {
    const column = vscode.ViewColumn.Beside;
    if (AiPanel.currentPanel) {
      AiPanel.currentPanel._panel.reveal(column);
      return;
    }
    const panel = vscode.window.createWebviewPanel(
      "gameAi",
      "GAME AI",
      column,
      { enableScripts: true, retainContextWhenHidden: true }
    );
    AiPanel.currentPanel = new AiPanel(panel, extensionUri);
  }

  private constructor(
    panel: vscode.WebviewPanel,
    extensionUri: vscode.Uri
  ) {
    this._panel = panel;
    this._extensionUri = extensionUri;
    this._aiProvider = new AiProvider();
    this._panel.webview.html = this._getHtml();
    this._panel.onDidDispose(() => this.dispose(), null, this._disposables);
    this._panel.webview.onDidReceiveMessage(
      (msg) => {
        if (msg.type === "generate") {
          this._handleGenerate(msg.prompt);
        }
      },
      null,
      this._disposables
    );
  }

  private async _handleGenerate(prompt: string): Promise<void> {
    this._post({ type: "generating" });

    let retries = 0;
    const maxRetries = 3;

    const attempt = async (
      currentPrompt: string,
      isRetry: boolean,
      previousCode?: string,
      previousError?: string
    ): Promise<void> => {
      try {
        let accumulated = "";
        const onChunk = (text: string): void => {
          accumulated += text;
          this._post({ type: "chunk", text });
        };

        let result;
        if (isRetry && previousCode && previousError) {
          this._post({ type: "retrying", attempt: retries + 1 });
          result = await this._aiProvider.fixCompilationError(
            previousCode,
            previousError,
            onChunk
          );
        } else {
          result = await this._aiProvider.generate(currentPrompt, onChunk);
        }

        // Try to compile
        const compileError = await this._tryCompile(result.code);

        if (compileError && retries < maxRetries) {
          retries++;
          this._post({
            type: "compileError",
            error: compileError,
            attempt: retries,
          });
          await attempt(prompt, true, result.code, compileError);
          return;
        }

        // Open in editor
        const doc = await vscode.workspace.openTextDocument({
          content: result.code,
          language: "game",
        });
        await vscode.window.showTextDocument(doc, vscode.ViewColumn.One);

        // Trigger preview
        vscode.commands.executeCommand("game.openPreview");

        this._post({
          type: "done",
          name: result.name,
          hasError: !!compileError,
        });
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        this._post({ type: "error", message });
      }
    };

    await attempt(prompt, false);
  }

  private _tryCompile(code: string): Promise<string | null> {
    const config = vscode.workspace.getConfiguration("game");
    const serverPath = config.get<string>("serverPath", "game");
    const tmp = os.tmpdir();
    const inputPath = path.join(tmp, "game-ai-gen.game");
    const outputDir = path.join(tmp, "game-ai-gen-out");

    fs.writeFileSync(inputPath, code);
    fs.mkdirSync(outputDir, { recursive: true });

    return new Promise((resolve) => {
      cp.exec(
        `"${serverPath}" build "${inputPath}" -o "${outputDir}"`,
        { timeout: 10000 },
        (err, _stdout, stderr) => {
          if (err) {
            const msg = stderr || err.message;
            if (msg.includes("ENOENT") || msg.includes("not found") || msg.includes("not recognized")) {
              resolve(
                "GAME compiler not found. Install it or set game.serverPath in settings."
              );
            } else {
              resolve(msg);
            }
          } else {
            resolve(null);
          }
        }
      );
    });
  }

  private _post(msg: Record<string, unknown>): void {
    this._panel.webview.postMessage(msg);
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
  #chat { display: flex; flex-direction: column; height: 100%; }
  #messages {
    flex: 1; overflow-y: auto; padding: 16px;
    display: flex; flex-direction: column; gap: 12px;
  }
  .msg {
    max-width: 90%; padding: 10px 14px; border-radius: 8px;
    font: 13px/1.5 -apple-system, BlinkMacSystemFont, sans-serif;
    color: #e0e0e0; word-wrap: break-word;
  }
  .msg.assistant { background: #141414; align-self: flex-start; }
  .msg.user { background: #1a1a3a; align-self: flex-end; }
  .msg.error { background: rgba(239,68,68,0.1); border: 1px solid rgba(239,68,68,0.2); color: #ef4444; }
  .msg.system { background: #0f1f0f; color: #22c55e; font-size: 12px; align-self: center; }
  .msg pre {
    background: #0d0d0d; border: 1px solid #1f1f1f; border-radius: 4px;
    padding: 8px 10px; margin: 8px 0 0; overflow-x: auto;
    font: 12px/1.5 'JetBrains Mono', monospace; color: #c0c0c0;
  }
  .streaming-dot {
    display: inline-block; width: 8px; height: 8px;
    background: #d4af37; border-radius: 50%;
    animation: pulse 1s ease-in-out infinite;
  }
  @keyframes pulse { 0%,100% { opacity: 0.3; } 50% { opacity: 1; } }
  #input-area {
    border-top: 1px solid #2a2a2a; padding: 12px; background: #0e0e0e;
    display: flex; gap: 8px; align-items: flex-end;
  }
  #prompt-input {
    flex: 1; background: #141414; border: 1px solid #2a2a2a; border-radius: 6px;
    color: #fff; padding: 8px 12px; font: 13px/1.5 -apple-system, BlinkMacSystemFont, sans-serif;
    resize: none; outline: none; min-height: 38px; max-height: 120px;
  }
  #prompt-input:focus { border-color: #d4af37; }
  #prompt-input::placeholder { color: #555; }
  #gen-btn {
    background: #d4af37; color: #0a0a0a; border: none; border-radius: 6px;
    padding: 8px 16px; font: 600 13px/1 -apple-system, BlinkMacSystemFont, sans-serif;
    cursor: pointer; white-space: nowrap;
  }
  #gen-btn:hover { background: #e5c348; }
  #gen-btn:disabled { opacity: 0.4; cursor: default; }
</style>
</head>
<body>
<div id="chat">
  <div id="messages">
    <div class="msg assistant">Describe a visual and I'll generate GAME code for it.</div>
  </div>
  <div id="input-area">
    <textarea id="prompt-input" placeholder="e.g. pulsing gold notification badge with particle trail" rows="1"></textarea>
    <button id="gen-btn">Generate</button>
  </div>
</div>
<script>
  const vscodeApi = acquireVsCodeApi();
  const msgs = document.getElementById('messages');
  const input = document.getElementById('prompt-input');
  const btn = document.getElementById('gen-btn');
  let streamEl = null;
  let streamText = '';
  let generating = false;

  function addMsg(cls, html) {
    const d = document.createElement('div');
    d.className = 'msg ' + cls;
    d.innerHTML = html;
    msgs.appendChild(d);
    msgs.scrollTop = msgs.scrollHeight;
    return d;
  }

  function escHtml(s) {
    return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
  }

  function submit() {
    const text = input.value.trim();
    if (!text || generating) return;
    addMsg('user', escHtml(text));
    input.value = '';
    input.style.height = 'auto';
    generating = true;
    btn.disabled = true;
    vscodeApi.postMessage({ type: 'generate', prompt: text });
  }

  btn.addEventListener('click', submit);
  input.addEventListener('keydown', e => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  });
  input.addEventListener('input', () => {
    input.style.height = 'auto';
    input.style.height = Math.min(input.scrollHeight, 120) + 'px';
  });

  window.addEventListener('message', event => {
    const msg = event.data;

    if (msg.type === 'generating') {
      streamEl = addMsg('assistant', '<span class="streaming-dot"></span>');
      streamText = '';
    }

    if (msg.type === 'chunk') {
      streamText += msg.text;
      if (streamEl) {
        const hasCode = streamText.includes('\`\`\`');
        if (hasCode) {
          const parts = streamText.split(/\`\`\`(?:game)?\\n?/);
          let html = escHtml(parts[0] || '');
          if (parts.length > 1) {
            const codePart = parts[1].split('\`\`\`');
            html += '<pre>' + escHtml(codePart[0]) + '</pre>';
            if (codePart[1]) html += escHtml(codePart[1]);
          }
          html += ' <span class="streaming-dot"></span>';
          streamEl.innerHTML = html;
        } else {
          streamEl.innerHTML = escHtml(streamText) + ' <span class="streaming-dot"></span>';
        }
        msgs.scrollTop = msgs.scrollHeight;
      }
    }

    if (msg.type === 'compileError') {
      addMsg('error', 'Compile error (attempt ' + msg.attempt + '/3): ' + escHtml(msg.error));
    }

    if (msg.type === 'retrying') {
      addMsg('system', 'Auto-fixing... attempt ' + msg.attempt);
      streamEl = addMsg('assistant', '<span class="streaming-dot"></span>');
      streamText = '';
    }

    if (msg.type === 'done') {
      // Finalize stream display
      if (streamEl && streamText) {
        const hasCode = streamText.includes('\`\`\`');
        if (hasCode) {
          const parts = streamText.split(/\`\`\`(?:game)?\\n?/);
          let html = escHtml(parts[0] || '');
          if (parts.length > 1) {
            const codePart = parts[1].split('\`\`\`');
            html += '<pre>' + escHtml(codePart[0]) + '</pre>';
            if (codePart[1]) html += escHtml(codePart[1]);
          }
          streamEl.innerHTML = html;
        } else {
          streamEl.innerHTML = escHtml(streamText);
        }
      }
      const status = msg.hasError
        ? 'Opened ' + msg.name + ' (with compilation warnings)'
        : 'Opened ' + msg.name + ' in editor with live preview';
      addMsg('system', status);
      generating = false;
      btn.disabled = false;
      input.focus();
    }

    if (msg.type === 'error') {
      if (streamEl) {
        streamEl.remove();
        streamEl = null;
      }
      addMsg('error', escHtml(msg.message));
      generating = false;
      btn.disabled = false;
      input.focus();
    }
  });

  input.focus();
</script>
</body>
</html>`;
  }

  public dispose(): void {
    AiPanel.currentPanel = undefined;
    this._panel.dispose();
    while (this._disposables.length) {
      const d = this._disposables.pop();
      if (d) d.dispose();
    }
  }
}
