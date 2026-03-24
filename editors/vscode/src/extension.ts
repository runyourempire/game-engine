import * as path from "path";
import * as vscode from "vscode";
import { workspace, ExtensionContext, window } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

import { PreviewPanel } from "./previewPanel";
import { GalleryPanel } from "./galleryPanel";
import { AiPanel } from "./aiPanel";
import { registerExportCommands } from "./exportCommands";
import { detectTunableToken } from "./parameterProvider";

let client: LanguageClient | undefined;

export function activate(context: ExtensionContext): void {
  // Live Preview command
  const previewCommand = vscode.commands.registerCommand(
    "game.openPreview",
    () => {
      PreviewPanel.createOrShow(context.extensionUri);
    }
  );
  context.subscriptions.push(previewCommand);

  // Component Gallery command
  const galleryCommand = vscode.commands.registerCommand(
    "game.openGallery",
    () => {
      GalleryPanel.createOrShow(context.extensionUri);
    }
  );
  context.subscriptions.push(galleryCommand);

  // AI Generation command
  const aiCommand = vscode.commands.registerCommand("game.openAi", () => {
    AiPanel.createOrShow(context.extensionUri);
  });
  context.subscriptions.push(aiCommand);

  // Parameter Tuner — track cursor position
  // Guard: skip detection while tuner is actively dragging/editing
  // to prevent the feedback loop (drag → edit → cursor moves → tuner resets)
  const cursorListener = vscode.window.onDidChangeTextEditorSelection((e) => {
    if (PreviewPanel.isTunerActive()) return;
    if (e.textEditor.document.languageId !== "game") return;
    const pos = e.selections[0]?.active;
    if (!pos) return;
    const token = detectTunableToken(e.textEditor.document, pos);
    if (token) {
      PreviewPanel.showTuner(token);
    } else {
      PreviewPanel.hideTuner();
    }
  });
  context.subscriptions.push(cursorListener);

  // Watch for editor text changes
  const changeListener = vscode.workspace.onDidChangeTextDocument((e) => {
    if (e.document.languageId === "game") {
      PreviewPanel.updateCode(e.document.getText());
    }
  });
  context.subscriptions.push(changeListener);

  // Watch for active editor switches
  const editorListener = vscode.window.onDidChangeActiveTextEditor(
    (editor) => {
      if (editor?.document.languageId === "game") {
        PreviewPanel.updateCode(editor.document.getText());
      }
    }
  );
  context.subscriptions.push(editorListener);

  // LSP setup
  const config = workspace.getConfiguration("game");
  const serverPath = config.get<string>("serverPath", "game");

  // Resolve the server binary path
  const command = resolveServerPath(serverPath);

  const serverOptions: ServerOptions = {
    run: {
      command,
      args: ["lsp"],
      transport: TransportKind.stdio,
    },
    debug: {
      command,
      args: ["lsp"],
      transport: TransportKind.stdio,
    },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "game" }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/*.game"),
    },
    outputChannelName: "GAME Language Server",
    traceOutputChannel: window.createOutputChannel("GAME Language Server Trace"),
  };

  client = new LanguageClient(
    "game-language-server",
    "GAME Language Server",
    serverOptions,
    clientOptions
  );

  // Start the client, which also launches the server
  client.start().catch((err) => {
    const message = err instanceof Error ? err.message : String(err);
    window.showWarningMessage(
      `GAME language server failed to start: ${message}. ` +
        `Syntax highlighting will still work. ` +
        `Set "game.serverPath" in settings if the binary is not on PATH.`
    );
  });

  context.subscriptions.push({
    dispose: () => {
      if (client) {
        return client.stop();
      }
    },
  });

  registerExportCommands(context);
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}

/**
 * Resolve the server binary path.
 * If the path is absolute, use it directly.
 * Otherwise, assume it's on PATH and let the OS resolve it.
 */
function resolveServerPath(configPath: string): string {
  if (path.isAbsolute(configPath)) {
    return configPath;
  }
  return configPath;
}
