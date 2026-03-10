import * as path from "path";
import { workspace, ExtensionContext, window } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export function activate(context: ExtensionContext): void {
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
    traceOutputChannelName: "GAME Language Server Trace",
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
