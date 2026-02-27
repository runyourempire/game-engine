const vscode = require('vscode');
const path = require('path');
const fs = require('fs');

/** @type {import('vscode').DiagnosticCollection} */
let diagnosticCollection;

/** @type {any} WASM module (loaded lazily) */
let wasmModule = null;

/** @type {boolean} Whether we've already warned about missing WASM */
let wasmWarningShown = false;

/** @type {import('vscode').WebviewPanel | null} */
let previewPanel = null;

/** @type {NodeJS.Timeout | null} */
let diagnosticDebounce = null;

const DEBOUNCE_MS = 500;

/**
 * Attempt to load the WASM compiler module from the bundled pkg/ directory.
 * Returns the module if available, or null if not found.
 */
async function loadWasm() {
    if (wasmModule) return wasmModule;

    const pkgDir = path.join(__dirname, '..', 'pkg');
    const wasmJsPath = path.join(pkgDir, 'game_compiler.js');

    if (!fs.existsSync(wasmJsPath)) {
        if (!wasmWarningShown) {
            wasmWarningShown = true;
            vscode.window.showInformationMessage(
                'GAME compiler WASM not found. Syntax highlighting is active, but ' +
                'diagnostics and preview require the WASM build. Run `wasm-pack build ' +
                '--target nodejs` in game-compiler/ and copy pkg/ into the extension.'
            );
        }
        return null;
    }

    try {
        wasmModule = require(wasmJsPath);
        // Some wasm-pack targets need explicit init
        if (typeof wasmModule.default === 'function') {
            await wasmModule.default();
        }
        return wasmModule;
    } catch (err) {
        if (!wasmWarningShown) {
            wasmWarningShown = true;
            vscode.window.showWarningMessage(
                `GAME: Failed to load WASM compiler: ${err.message}`
            );
        }
        return null;
    }
}

/**
 * Parse a compiler error message to extract byte offset span.
 * Error format: "message (at byte START..END)"
 * Returns { message: string, startByte: number | null, endByte: number | null }
 */
function parseError(errorMsg) {
    const spanMatch = errorMsg.match(/\(at byte (\d+)\.\.(\d+)\)/);
    if (spanMatch) {
        return {
            message: errorMsg.replace(/\s*\(at byte \d+\.\.\d+\)/, ''),
            startByte: parseInt(spanMatch[1], 10),
            endByte: parseInt(spanMatch[2], 10),
        };
    }
    return { message: errorMsg, startByte: null, endByte: null };
}

/**
 * Convert a byte offset to a vscode.Position using the document text.
 */
function byteOffsetToPosition(text, byteOffset) {
    // JavaScript strings are UTF-16, but byte offsets are from the Rust side (UTF-8).
    // For ASCII-heavy .game files this is usually 1:1.
    // Encode to UTF-8 and count to find the character offset.
    const encoder = new TextEncoder();
    const bytes = encoder.encode(text);
    let charIndex = 0;
    let byteIndex = 0;

    while (byteIndex < byteOffset && charIndex < text.length) {
        const code = text.charCodeAt(charIndex);
        if (code < 0x80) byteIndex += 1;
        else if (code < 0x800) byteIndex += 2;
        else if (code >= 0xD800 && code <= 0xDBFF) { byteIndex += 4; charIndex++; }
        else byteIndex += 3;
        charIndex++;
    }

    // Now convert charIndex to line/column
    let line = 0;
    let col = 0;
    for (let i = 0; i < charIndex && i < text.length; i++) {
        if (text[i] === '\n') {
            line++;
            col = 0;
        } else {
            col++;
        }
    }
    return new vscode.Position(line, col);
}

/**
 * Run validation on the document and publish diagnostics.
 */
async function updateDiagnostics(document) {
    if (document.languageId !== 'game') return;

    const wasm = await loadWasm();
    if (!wasm) {
        diagnosticCollection.clear();
        return;
    }

    const source = document.getText();
    const diagnostics = [];

    try {
        const result = wasm.validate(source);

        if (!result.valid && result.error) {
            const parsed = parseError(result.error);
            let range;

            if (parsed.startByte !== null && parsed.endByte !== null) {
                const startPos = byteOffsetToPosition(source, parsed.startByte);
                const endPos = byteOffsetToPosition(source, parsed.endByte);
                range = new vscode.Range(startPos, endPos);
            } else {
                // No span — underline the first line
                range = new vscode.Range(0, 0, 0, document.lineAt(0).text.length);
            }

            diagnostics.push(new vscode.Diagnostic(
                range,
                parsed.message,
                vscode.DiagnosticSeverity.Error
            ));
        }

        if (result.valid && result.warnings) {
            const warnings = Array.isArray(result.warnings)
                ? result.warnings
                : Array.from(result.warnings || []);

            for (const warning of warnings) {
                const parsed = parseError(warning);
                let range;

                if (parsed.startByte !== null && parsed.endByte !== null) {
                    const startPos = byteOffsetToPosition(source, parsed.startByte);
                    const endPos = byteOffsetToPosition(source, parsed.endByte);
                    range = new vscode.Range(startPos, endPos);
                } else {
                    range = new vscode.Range(0, 0, 0, document.lineAt(0).text.length);
                }

                diagnostics.push(new vscode.Diagnostic(
                    range,
                    parsed.message,
                    vscode.DiagnosticSeverity.Warning
                ));
            }
        }
    } catch (err) {
        // Compilation threw — report as error on first line
        const parsed = parseError(err.message || String(err));
        let range;

        if (parsed.startByte !== null && parsed.endByte !== null) {
            const startPos = byteOffsetToPosition(source, parsed.startByte);
            const endPos = byteOffsetToPosition(source, parsed.endByte);
            range = new vscode.Range(startPos, endPos);
        } else {
            range = new vscode.Range(0, 0, 0, Math.min(document.lineAt(0).text.length, 80));
        }

        diagnostics.push(new vscode.Diagnostic(
            range,
            parsed.message,
            vscode.DiagnosticSeverity.Error
        ));
    }

    diagnosticCollection.set(document.uri, diagnostics);
}

/**
 * Schedule a debounced diagnostics update.
 */
function scheduleDiagnostics(document) {
    if (diagnosticDebounce) {
        clearTimeout(diagnosticDebounce);
    }
    diagnosticDebounce = setTimeout(() => {
        updateDiagnostics(document);
    }, DEBOUNCE_MS);
}

/**
 * Update the preview panel with compiled HTML.
 */
async function updatePreview(document) {
    if (!previewPanel) return;
    if (document.languageId !== 'game') return;

    const wasm = await loadWasm();
    if (!wasm) return;

    const source = document.getText();

    try {
        const html = wasm.compile_to_html(source);
        previewPanel.webview.html = html;
    } catch (err) {
        previewPanel.webview.html = `
            <!DOCTYPE html>
            <html>
            <head>
                <style>
                    body {
                        background: #0A0A0A;
                        color: #EF4444;
                        font-family: 'JetBrains Mono', 'Fira Code', monospace;
                        padding: 2rem;
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        height: 100vh;
                        margin: 0;
                    }
                    .error {
                        background: #1F1F1F;
                        border: 1px solid #2A2A2A;
                        border-left: 4px solid #EF4444;
                        padding: 1.5rem;
                        border-radius: 4px;
                        max-width: 600px;
                        white-space: pre-wrap;
                        word-break: break-word;
                    }
                    .title {
                        color: #A0A0A0;
                        font-size: 0.85em;
                        margin-bottom: 0.5rem;
                    }
                </style>
            </head>
            <body>
                <div class="error">
                    <div class="title">Compilation Error</div>
                    ${escapeHtml(err.message || String(err))}
                </div>
            </body>
            </html>
        `;
    }
}

/**
 * Escape HTML entities for safe embedding.
 */
function escapeHtml(text) {
    return text
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;');
}

/**
 * Compile source and open result in a new untitled editor.
 */
async function compileAndShow(document, compileFunction, languageId) {
    const wasm = await loadWasm();
    if (!wasm) {
        vscode.window.showErrorMessage(
            'GAME compiler WASM not available. Build it first with wasm-pack.'
        );
        return;
    }

    const source = document.getText();

    try {
        const output = compileFunction(wasm, source);
        const doc = await vscode.workspace.openTextDocument({
            content: output,
            language: languageId,
        });
        await vscode.window.showTextDocument(doc, { preview: false });
    } catch (err) {
        vscode.window.showErrorMessage(`GAME compilation failed: ${err.message}`);
    }
}

/**
 * Extension activation.
 * @param {import('vscode').ExtensionContext} context
 */
function activate(context) {
    diagnosticCollection = vscode.languages.createDiagnosticCollection('game');
    context.subscriptions.push(diagnosticCollection);

    // ── Diagnostics on document change (debounced) ──────────────────────

    context.subscriptions.push(
        vscode.workspace.onDidChangeTextDocument((event) => {
            if (event.document.languageId === 'game') {
                scheduleDiagnostics(event.document);
            }
        })
    );

    context.subscriptions.push(
        vscode.workspace.onDidOpenTextDocument((document) => {
            if (document.languageId === 'game') {
                scheduleDiagnostics(document);
            }
        })
    );

    context.subscriptions.push(
        vscode.workspace.onDidSaveTextDocument((document) => {
            if (document.languageId === 'game') {
                updateDiagnostics(document);
                updatePreview(document);
            }
        })
    );

    // Run diagnostics on already-open .game files
    for (const editor of vscode.window.visibleTextEditors) {
        if (editor.document.languageId === 'game') {
            scheduleDiagnostics(editor.document);
        }
    }

    // ── Preview command ─────────────────────────────────────────────────

    context.subscriptions.push(
        vscode.commands.registerCommand('game.preview', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'game') {
                vscode.window.showWarningMessage('Open a .game file to preview.');
                return;
            }

            if (previewPanel) {
                previewPanel.reveal(vscode.ViewColumn.Beside);
            } else {
                previewPanel = vscode.window.createWebviewPanel(
                    'gamePreview',
                    'GAME Preview',
                    vscode.ViewColumn.Beside,
                    {
                        enableScripts: true,
                        retainContextWhenHidden: true,
                    }
                );

                previewPanel.onDidDispose(() => {
                    previewPanel = null;
                }, null, context.subscriptions);
            }

            await updatePreview(editor.document);

            // Also update preview on text change (debounced)
            const changeDisposable = vscode.workspace.onDidChangeTextDocument(
                (event) => {
                    if (event.document === editor.document) {
                        // Debounce preview updates to avoid excessive recompilation
                        if (diagnosticDebounce) clearTimeout(diagnosticDebounce);
                        diagnosticDebounce = setTimeout(() => {
                            updatePreview(event.document);
                            updateDiagnostics(event.document);
                        }, DEBOUNCE_MS);
                    }
                }
            );

            previewPanel.onDidDispose(() => {
                changeDisposable.dispose();
            });
        })
    );

    // ── Compile to WGSL command ─────────────────────────────────────────

    context.subscriptions.push(
        vscode.commands.registerCommand('game.compile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'game') {
                vscode.window.showWarningMessage('Open a .game file to compile.');
                return;
            }
            await compileAndShow(
                editor.document,
                (wasm, source) => wasm.compile_to_wgsl(source),
                'wgsl'
            );
        })
    );

    // ── Compile to HTML command ─────────────────────────────────────────

    context.subscriptions.push(
        vscode.commands.registerCommand('game.compileHtml', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'game') {
                vscode.window.showWarningMessage('Open a .game file to compile.');
                return;
            }
            await compileAndShow(
                editor.document,
                (wasm, source) => wasm.compile_to_html(source),
                'html'
            );
        })
    );

    // ── Compile to Component command ────────────────────────────────────

    context.subscriptions.push(
        vscode.commands.registerCommand('game.compileComponent', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'game') {
                vscode.window.showWarningMessage('Open a .game file to compile.');
                return;
            }

            const fileName = path.basename(
                editor.document.fileName,
                '.game'
            );
            // Derive a tag name: strip leading digits, ensure hyphen
            let tagName = fileName
                .replace(/^\d+-?/, '')
                .replace(/_/g, '-')
                .toLowerCase();
            if (!tagName.includes('-')) {
                tagName = `game-${tagName}`;
            }

            await compileAndShow(
                editor.document,
                (wasm, source) => wasm.compile_to_component(source, tagName),
                'javascript'
            );
        })
    );

    // ── Auto-preview on open (if configured) ────────────────────────────

    const config = vscode.workspace.getConfiguration('game');
    if (config.get('autoPreview')) {
        context.subscriptions.push(
            vscode.window.onDidChangeActiveTextEditor((editor) => {
                if (editor && editor.document.languageId === 'game' && !previewPanel) {
                    vscode.commands.executeCommand('game.preview');
                }
            })
        );
    }
}

/**
 * Extension deactivation.
 */
function deactivate() {
    if (diagnosticDebounce) {
        clearTimeout(diagnosticDebounce);
    }
    if (diagnosticCollection) {
        diagnosticCollection.dispose();
    }
    if (previewPanel) {
        previewPanel.dispose();
        previewPanel = null;
    }
}

module.exports = { activate, deactivate };
