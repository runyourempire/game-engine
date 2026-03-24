import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';

interface GalleryComponent {
  name: string;
  file: string;
  category: string;
  description: string;
  tags: string[];
}

interface GalleryIndex {
  components: GalleryComponent[];
}

export class GalleryPanel {
  public static currentPanel: GalleryPanel | undefined;
  private readonly _panel: vscode.WebviewPanel;
  private readonly _extensionUri: vscode.Uri;
  private _disposables: vscode.Disposable[] = [];

  public static createOrShow(extensionUri: vscode.Uri): void {
    const column = vscode.ViewColumn.One;
    if (GalleryPanel.currentPanel) {
      GalleryPanel.currentPanel._panel.reveal(column);
      return;
    }
    const panel = vscode.window.createWebviewPanel(
      'gameGallery',
      'GAME Component Gallery',
      column,
      {
        enableScripts: true,
        retainContextWhenHidden: false,
      }
    );
    GalleryPanel.currentPanel = new GalleryPanel(panel, extensionUri);
  }

  private constructor(panel: vscode.WebviewPanel, extensionUri: vscode.Uri) {
    this._panel = panel;
    this._extensionUri = extensionUri;
    this._panel.webview.html = this._getHtml();
    this._panel.onDidDispose(() => this.dispose(), null, this._disposables);
    this._panel.webview.onDidReceiveMessage(
      (msg) => this._handleMessage(msg),
      null,
      this._disposables
    );
  }

  private async _handleMessage(msg: { type: string; file?: string; name?: string }): Promise<void> {
    if (msg.type === 'open' && msg.file) {
      const galleryDir = path.join(this._extensionUri.fsPath, 'gallery', 'components');
      const filePath = path.join(galleryDir, msg.file);
      if (fs.existsSync(filePath)) {
        const uri = vscode.Uri.file(filePath);
        vscode.window.showTextDocument(uri, { viewColumn: vscode.ViewColumn.One }).then(() => {
          vscode.commands.executeCommand('game.openPreview');
        });
      } else {
        vscode.window.showWarningMessage(`Component file not found: ${msg.file}`);
      }
    }
    if (msg.type === 'fork' && msg.file && msg.name) {
      await this._forkComponent(msg.file, msg.name);
    }
  }

  private async _forkComponent(file: string, name: string): Promise<void> {
    const galleryDir = path.join(this._extensionUri.fsPath, 'gallery', 'components');
    const srcPath = path.join(galleryDir, file);
    if (!fs.existsSync(srcPath)) {
      vscode.window.showWarningMessage(`Component file not found: ${file}`);
      return;
    }

    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders || workspaceFolders.length === 0) {
      vscode.window.showWarningMessage('Open a workspace folder to fork components into.');
      return;
    }

    const destDir = workspaceFolders[0].uri.fsPath;
    const destFile = path.join(destDir, `${name}.game`);

    if (fs.existsSync(destFile)) {
      const overwrite = await vscode.window.showWarningMessage(
        `${name}.game already exists. Overwrite?`, 'Yes', 'No'
      );
      if (overwrite !== 'Yes') return;
    }

    const content = fs.readFileSync(srcPath, 'utf-8');
    fs.writeFileSync(destFile, content);

    const uri = vscode.Uri.file(destFile);
    vscode.window.showTextDocument(uri, { viewColumn: vscode.ViewColumn.One });
    vscode.window.showInformationMessage(`Forked ${name}.game to workspace root`);
  }

  private _loadGallery(): GalleryIndex | null {
    const indexPath = path.join(this._extensionUri.fsPath, 'gallery', 'index.json');
    if (!fs.existsSync(indexPath)) {
      return null;
    }
    try {
      const raw = fs.readFileSync(indexPath, 'utf-8');
      return JSON.parse(raw) as GalleryIndex;
    } catch {
      return null;
    }
  }

  private _escapeHtml(s: string): string {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }

  private _loadSourcePreview(file: string): string {
    const galleryDir = path.join(this._extensionUri.fsPath, 'gallery', 'components');
    const filePath = path.join(galleryDir, file);
    if (!fs.existsSync(filePath)) {
      return '// source not available';
    }
    try {
      const content = fs.readFileSync(filePath, 'utf-8');
      const lines = content.split('\n').slice(0, 6);
      return lines.join('\n');
    } catch {
      return '// error reading source';
    }
  }

  private _getHtml(): string {
    const gallery = this._loadGallery();

    if (!gallery) {
      return `<!DOCTYPE html>
<html><head><meta charset="UTF-8">
<style>body{background:#0a0a0a;color:#a0a0a0;font:14px -apple-system,BlinkMacSystemFont,sans-serif;display:flex;align-items:center;justify-content:center;height:100vh;margin:0;}
.msg{text-align:center;}.msg h2{color:#fff;margin-bottom:12px;}</style></head>
<body><div class="msg"><h2>Gallery Not Found</h2><p>The component gallery directory was not found.<br>Ensure <code>gallery/index.json</code> exists in the extension directory.</p></div></body></html>`;
    }

    const categories = [
      { id: 'all', label: 'All' },
      { id: 'backgrounds', label: 'Backgrounds' },
      { id: 'indicators', label: 'Indicators' },
      { id: 'effects', label: 'Effects' },
      { id: 'data-viz', label: 'Data Viz' },
      { id: 'micro-interactions', label: 'Micro-interactions' },
      { id: '4da-production', label: '4DA Production' },
    ];

    const componentCards = gallery.components.map((c) => {
      const preview = this._loadSourcePreview(c.file)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;');
      const kebabName = c.name.replace(/([a-z])([A-Z])/g, '$1-$2').toLowerCase();
      const escapedName = this._escapeHtml(c.name);
      const escapedDesc = this._escapeHtml(c.description);
      const escapedCategory = this._escapeHtml(c.category);
      const escapedFile = this._escapeHtml(c.file);
      const escapedTags = this._escapeHtml(c.tags.join(' '));
      return `<div class="card" data-category="${escapedCategory}" data-name="${escapedName.toLowerCase()}" data-tags="${escapedTags}" data-desc="${escapedDesc.toLowerCase()}">
  <div class="card-header">
    <span class="card-name">${escapedName}</span>
    <span class="card-tag">${escapedCategory}</span>
  </div>
  <p class="card-desc">${escapedDesc}</p>
  <pre class="card-code"><code>${preview}</code></pre>
  <div class="card-actions">
    <button class="btn-open" data-file="${escapedFile}" data-cname="${escapedName}">Open</button>
    <button class="btn-fork" data-file="${escapedFile}" data-cname="${this._escapeHtml(kebabName)}">Fork</button>
  </div>
</div>`;
    }).join('\n');

    const categoryTabs = categories.map((cat) =>
      `<button class="tab${cat.id === 'all' ? ' active' : ''}" data-cat="${cat.id}">${cat.label}</button>`
    ).join('\n');

    return `<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  html, body {
    width: 100%; height: 100%;
    background: #0a0a0a;
    color: #ffffff;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    overflow-y: auto;
  }
  .header {
    position: sticky; top: 0; z-index: 100;
    background: #0a0a0a;
    border-bottom: 1px solid #2a2a2a;
    padding: 16px 24px 0;
  }
  .header h1 {
    font-size: 18px;
    font-weight: 600;
    margin-bottom: 12px;
    color: #ffffff;
  }
  .header h1 span {
    color: #d4af37;
    font-weight: 400;
  }
  .search-bar {
    display: flex;
    gap: 12px;
    margin-bottom: 12px;
  }
  .search-bar input {
    flex: 1;
    padding: 8px 12px;
    background: #141414;
    border: 1px solid #2a2a2a;
    border-radius: 6px;
    color: #ffffff;
    font-size: 13px;
    outline: none;
  }
  .search-bar input::placeholder { color: #666; }
  .search-bar input:focus { border-color: #444; }
  .count {
    font-size: 12px;
    color: #666;
    align-self: center;
    white-space: nowrap;
  }
  .tabs {
    display: flex;
    gap: 4px;
    padding-bottom: 12px;
    overflow-x: auto;
  }
  .tab {
    padding: 6px 14px;
    border-radius: 20px;
    border: 1px solid #2a2a2a;
    background: transparent;
    color: #666;
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.15s;
  }
  .tab:hover { color: #a0a0a0; border-color: #444; }
  .tab.active {
    color: #ffffff;
    background: #1f1f1f;
    border-color: #444;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 16px;
    padding: 20px 24px 40px;
  }
  .card {
    background: #141414;
    border: 1px solid #2a2a2a;
    border-radius: 8px;
    padding: 16px;
    transition: background 0.15s, border-color 0.15s;
    cursor: default;
  }
  .card:hover {
    background: #1f1f1f;
    border-color: #3a3a3a;
  }
  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }
  .card-name {
    font-size: 15px;
    font-weight: 600;
    color: #ffffff;
  }
  .card-tag {
    font-size: 10px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #d4af37;
    background: rgba(212, 175, 55, 0.1);
    padding: 3px 8px;
    border-radius: 10px;
  }
  .card-desc {
    font-size: 12px;
    color: #a0a0a0;
    margin-bottom: 12px;
    line-height: 1.4;
  }
  .card-code {
    background: #1a1a1a;
    border-radius: 6px;
    padding: 10px 12px;
    margin-bottom: 12px;
    overflow: hidden;
    max-height: 108px;
  }
  .card-code code {
    font-family: 'JetBrains Mono', 'SF Mono', 'Fira Code', monospace;
    font-size: 11px;
    line-height: 1.5;
    color: #8a8a8a;
    white-space: pre;
  }
  .card-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .btn-open, .btn-fork {
    padding: 5px 12px;
    border-radius: 4px;
    border: none;
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
  }
  .btn-open {
    background: #1f1f1f;
    color: #ffffff;
    border: 1px solid #2a2a2a;
  }
  .btn-open:hover { background: #2a2a2a; }
  .btn-fork {
    background: transparent;
    color: #d4af37;
    border: 1px solid rgba(212, 175, 55, 0.3);
  }
  .btn-fork:hover {
    background: rgba(212, 175, 55, 0.1);
    border-color: #d4af37;
  }
  .empty {
    grid-column: 1 / -1;
    text-align: center;
    padding: 60px 20px;
    color: #444;
    font-size: 14px;
  }
  .card.hidden { display: none; }
</style>
</head>
<body>
<div class="header">
  <h1>GAME Component Gallery <span>\u2014 ${gallery.components.length} components</span></h1>
  <div class="search-bar">
    <input type="text" id="search" placeholder="Search components..." autocomplete="off" spellcheck="false">
    <span class="count" id="count">${gallery.components.length} shown</span>
  </div>
  <div class="tabs" id="tabs">
    ${categoryTabs}
  </div>
</div>
<div class="grid" id="grid">
  ${componentCards}
  <div class="empty" id="empty" style="display:none">No components match your search.</div>
</div>
<script>
  const vscode = acquireVsCodeApi();
  const search = document.getElementById('search');
  const grid = document.getElementById('grid');
  const tabs = document.getElementById('tabs');
  const countEl = document.getElementById('count');
  const emptyEl = document.getElementById('empty');
  const cards = Array.from(document.querySelectorAll('.card'));

  let activeCategory = 'all';

  function filterCards() {
    const query = search.value.toLowerCase().trim();
    let visible = 0;
    cards.forEach(card => {
      const name = card.dataset.name || '';
      const tags = card.dataset.tags || '';
      const cat = card.dataset.category || '';
      const desc = card.dataset.desc || '';
      const matchesCat = activeCategory === 'all' || cat === activeCategory;
      const matchesSearch = !query || name.includes(query) || tags.includes(query) || cat.includes(query) || desc.includes(query);
      const show = matchesCat && matchesSearch;
      card.classList.toggle('hidden', !show);
      if (show) visible++;
    });
    countEl.textContent = visible + ' shown';
    emptyEl.style.display = visible === 0 ? 'block' : 'none';
  }

  search.addEventListener('input', filterCards);

  tabs.addEventListener('click', (e) => {
    const btn = e.target.closest('.tab');
    if (!btn) return;
    document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
    btn.classList.add('active');
    activeCategory = btn.dataset.cat || 'all';
    filterCards();
  });

  grid.addEventListener('click', (e) => {
    const openBtn = e.target.closest('.btn-open');
    if (openBtn) {
      vscode.postMessage({ type: 'open', file: openBtn.dataset.file, name: openBtn.dataset.cname });
      return;
    }
    const forkBtn = e.target.closest('.btn-fork');
    if (forkBtn) {
      vscode.postMessage({ type: 'fork', file: forkBtn.dataset.file, name: forkBtn.dataset.cname });
      return;
    }
  });
</script>
</body>
</html>`;
  }

  public dispose(): void {
    GalleryPanel.currentPanel = undefined;
    this._panel.dispose();
    while (this._disposables.length) {
      const d = this._disposables.pop();
      if (d) d.dispose();
    }
  }
}
