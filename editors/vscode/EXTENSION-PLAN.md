# GAME VS Code Extension — Creative Development Environment

**Transform from syntax highlighter to creative IDE for living visuals.**

## Architecture

```
Extension Host (Node.js)          WebView Panel (Chromium)
┌─────────────────────┐           ┌──────────────────────────┐
│ extension.ts        │  message  │ preview.html             │
│ ├── LSP Client      │◄────────►│ ├── WASM Compiler         │
│ ├── Preview Manager │  passing  │ ├── WebGL Canvas          │
│ ├── Editor Listener │           │ ├── Parameter UI          │
│ └── Export Commands │           │ └── AI Panel              │
└─────────────────────┘           └──────────────────────────┘
```

**Message flow:**
1. User types in editor → extension sends code to WebView
2. WebView compiles via WASM → renders to WebGL canvas
3. User drags slider in WebView → WebView sends value to extension → extension updates editor text
4. User clicks Export → extension reads compiled JS → copies/saves

## Phase A: Live Preview Panel (v0.2.0)

### Files to create:
- `src/previewPanel.ts` — WebView panel lifecycle management
- `src/previewHtml.ts` — Generates the WebView HTML content
- `media/preview.js` — WebView-side JavaScript (WASM loading, compilation, rendering)
- `media/preview.css` — WebView styling

### Files to modify:
- `src/extension.ts` — Register preview command, listen for editor changes
- `package.json` — Add command, keybinding, menu contribution

### extension.ts additions:
```typescript
import { PreviewPanel } from './previewPanel';

// In activate():
const previewCommand = vscode.commands.registerCommand('game.openPreview', () => {
  PreviewPanel.createOrShow(context.extensionUri);
});
context.subscriptions.push(previewCommand);

// Listen for editor changes:
vscode.workspace.onDidChangeTextDocument(e => {
  if (e.document.languageId === 'game') {
    PreviewPanel.updateCode(e.document.getText());
  }
});
vscode.window.onDidChangeActiveTextEditor(editor => {
  if (editor?.document.languageId === 'game') {
    PreviewPanel.updateCode(editor.document.getText());
  }
});
```

### previewPanel.ts:
```typescript
export class PreviewPanel {
  public static currentPanel: PreviewPanel | undefined;
  private readonly _panel: vscode.WebviewPanel;
  private _disposables: vscode.Disposable[] = [];

  public static createOrShow(extensionUri: vscode.Uri): void {
    const column = vscode.ViewColumn.Beside;
    if (PreviewPanel.currentPanel) {
      PreviewPanel.currentPanel._panel.reveal(column);
      return;
    }
    const panel = vscode.window.createWebviewPanel(
      'gamePreview', 'GAME Preview', column,
      {
        enableScripts: true,
        localResourceRoots: [vscode.Uri.joinPath(extensionUri, 'media'), vscode.Uri.joinPath(extensionUri, 'pkg')],
        retainContextWhenHidden: true,
      }
    );
    PreviewPanel.currentPanel = new PreviewPanel(panel, extensionUri);
  }

  public static updateCode(code: string): void {
    PreviewPanel.currentPanel?._panel.webview.postMessage({ type: 'update', code });
  }

  private constructor(panel, extensionUri) {
    this._panel = panel;
    this._panel.webview.html = getPreviewHtml(panel.webview, extensionUri);
    this._panel.onDidDispose(() => this.dispose(), null, this._disposables);
    this._panel.webview.onDidReceiveMessage(msg => this._handleMessage(msg), null, this._disposables);
  }
}
```

### media/preview.js (WebView-side):
```javascript
// Load WASM compiler
import init, { compileGame, validateGame } from './game_compiler.js';

let wasmReady = false;
let currentCode = '';
let canvas, gl;

async function initialize() {
  await init();  // Initialize WASM
  wasmReady = true;
  canvas = document.getElementById('preview-canvas');
  gl = canvas.getContext('webgl2');
  compile(currentCode);
}

function compile(code) {
  if (!wasmReady || !code.trim()) return;
  try {
    const result = compileGame(code, 'webgl2');
    const outputs = JSON.parse(result);
    if (outputs.length > 0) {
      renderComponent(outputs[0]);
      postStatus('ok', outputs[0].name);
    }
  } catch (err) {
    postStatus('error', err.toString());
  }
}

function renderComponent(output) {
  // Create a temporary script element to define the Web Component
  // Then instantiate it inside the preview container
  const container = document.getElementById('component-container');
  container.innerHTML = '';
  const script = document.createElement('script');
  script.textContent = output.js;
  document.head.appendChild(script);
  // Create the custom element
  const tagName = 'game-' + output.name.toLowerCase().replace(/[^a-z0-9]/g, '-');
  const el = document.createElement(tagName);
  el.style.width = '100%';
  el.style.height = '100%';
  container.appendChild(el);
}

// Listen for messages from extension
window.addEventListener('message', event => {
  const msg = event.data;
  if (msg.type === 'update') {
    currentCode = msg.code;
    compile(currentCode);
  }
});

initialize();
```

### package.json additions:
```json
{
  "commands": [
    { "command": "game.openPreview", "title": "GAME: Open Preview", "icon": "$(preview)" }
  ],
  "keybindings": [
    { "command": "game.openPreview", "key": "ctrl+shift+g", "when": "editorLangId == game" }
  ],
  "menus": {
    "editor/title": [
      { "command": "game.openPreview", "when": "editorLangId == game", "group": "navigation" }
    ]
  }
}
```

### Verification:
- Open .game file → Ctrl+Shift+G → preview panel opens
- Type code → preview updates within 300ms
- Resize preview → canvas adapts
- Close preview → clean disposal
- Reopen → state restored

---

## Phase B: Visual Parameter Tuner (v0.3.0)

### Files to create:
- `src/parameterProvider.ts` — Detects numbers/colors/palettes at cursor position
- `media/tuner.js` — Slider/picker/swatch UI components in WebView

### Files to modify:
- `media/preview.js` — Add tuner overlay on canvas
- `src/previewPanel.ts` — Handle tuner value changes, update editor

### How it works:
1. Extension parses cursor position → identifies token type (number, color, palette)
2. Sends token info to WebView: `{ type: 'showTuner', kind: 'number', value: 2.5, range: [0, 10], line: 5, col: 12 }`
3. WebView shows slider/picker overlay on canvas
4. User drags → WebView sends: `{ type: 'tunerChange', value: 3.2, line: 5, col: 12, endCol: 15 }`
5. Extension applies edit to document: `editor.edit(b => b.replace(range, '3.2'))`

### Tuner types:
- **Number slider**: any float literal → range slider (auto-detect range from context: glow=0-10, radius=0-1, opacity=0-1)
- **Color picker**: `tint(r, g, b)` or `#RRGGBB` → full color picker
- **Palette swatches**: `palette(name)` → 30 visual thumbnails rendered via WASM

### Verification:
- Click on `2.5` in `glow(2.5)` → slider appears → drag → code and preview update simultaneously
- Click on `#D4AF37` → color picker → pick → code updates
- Click on `palette(fire)` → 30 swatches → click `ocean` → code changes to `palette(ocean)` → preview updates

---

## Phase C: Component Library Browser (v0.4.0)

### Files to create:
- `src/galleryPanel.ts` — Side panel showing component gallery
- `media/gallery.html` — Gallery UI (grid of thumbnails with search/filter)
- `media/gallery.js` — Gallery interaction logic
- `media/gallery.css` — Gallery styling
- `gallery/index.json` — Component registry (name, description, category, tags, source URL)
- `gallery/components/` — 50+ .game files organized by category

### Categories:
- **Backgrounds**: nebula, aurora, ember, quantum, ocean, storm (6+)
- **Indicators**: heartbeat, progress ring, status dot, signal meter, boot ring (5+)
- **Micro-interactions**: button hover, card entrance, dismiss, loading spin (4+)
- **Data viz**: sparkline, heat map, flow diagram, radar (4+)
- **Effects**: rain, particles, lightning, portal, fire (5+)
- **4DA Production**: ambient intelligence, signal waveform, achievement progress (3+)

### How it works:
1. Command palette: "GAME: Browse Components" opens gallery panel
2. Gallery fetches index.json → renders grid with category tabs
3. Each thumbnail: compile .game source via WASM → render single frame → display as image
4. Click component → opens in new editor tab with live preview
5. "Fork" button → copies to user's workspace

### Verification:
- Open gallery → see 50+ components with thumbnails
- Filter by "indicators" → shows 5+ results
- Click "Achievement Progress" → opens in editor with live preview
- Edit the forked version → preview updates

---

## Phase D: AI Generation Panel (v0.5.0)

### Files to create:
- `src/aiPanel.ts` — AI panel management
- `media/ai.html` — Chat-style input UI
- `media/ai.js` — Handles prompt submission, streaming, code insertion
- `src/aiProvider.ts` — Claude API integration (user provides API key)

### How it works:
1. Command: "GAME: Generate with AI" opens input panel
2. User types: "Pulsing gold notification with dismiss animation"
3. Extension sends to Claude API with generate-component.md system prompt
4. Response streamed to panel → code inserted into new editor → live preview renders
5. If compilation fails → error fed back to AI → retry (max 3 attempts)

### Settings:
```json
{
  "game.ai.apiKey": { "type": "string", "description": "Anthropic API key for AI generation" },
  "game.ai.model": { "type": "string", "default": "claude-sonnet-4-20250514", "description": "Claude model to use" }
}
```

### Verification:
- Set API key → open AI panel → type description → get working component
- Invalid output → auto-retry with error context → get corrected output
- Generated code compiles and renders in preview

---

## Phase E: One-Click Export (v0.6.0)

### Files to create:
- `src/exportCommands.ts` — Export format handlers

### Files to modify:
- `src/extension.ts` — Register export commands
- `package.json` — Add export commands and menu items

### Export formats:
1. **Copy JS** — Compiled Web Component JS to clipboard
2. **Copy HTML** — `<script>` tag + `<game-name>` element
3. **Copy React** — `import` statement + JSX usage
4. **Copy Vue** — SFC template usage
5. **Copy Svelte** — Component usage
6. **Save JS file** — File dialog → save compiled .js
7. **Save HTML file** — Complete standalone HTML page

### How it works:
1. Toolbar button (or command palette): "GAME: Export Component"
2. Quick pick menu shows format options
3. Extension compiles current .game file via CLI (`game build`)
4. Formats output for selected target
5. Copies to clipboard or saves to file
6. Shows confirmation notification

### Verification:
- Open .game file → Export → Copy React → paste into React app → works
- Export → Save HTML → open in browser → component renders

---

## Version Roadmap

| Version | Phase | What Ships |
|---------|-------|-----------|
| 0.1.0 | Current | Syntax + snippets + LSP |
| **0.2.0** | **A** | **Live Preview Panel** |
| **0.3.0** | **B** | **Visual Parameter Tuner** |
| **0.4.0** | **C** | **Component Gallery** |
| **0.5.0** | **D** | **AI Generation** |
| **0.6.0** | **E** | **One-Click Export** |

Each version independently shippable. Each builds on the last.

---

*Plan authored: 2026-03-25*
