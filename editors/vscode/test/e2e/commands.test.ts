import { browser } from "@wdio/globals";
import path from "path";

const FIXTURE = path.join(__dirname, "../fixtures/hello.game");

describe("Extension Commands", () => {
  before(async () => {
    // Open a .game file first
    await browser.executeWorkbench(async (vscode, filePath) => {
      const doc = await vscode.workspace.openTextDocument(
        vscode.Uri.file(filePath)
      );
      await vscode.window.showTextDocument(doc);
    }, FIXTURE);
    await browser.pause(2000);
  });

  it("game.openPreview executes without error", async () => {
    const result = await browser.executeWorkbench(async (vscode) => {
      try {
        await vscode.commands.executeCommand("game.openPreview");
        return { ok: true };
      } catch (e: unknown) {
        return { ok: false, error: String(e) };
      }
    });
    expect((result as { ok: boolean }).ok).toBe(true);
  });

  it("game.openGallery executes without error", async () => {
    const result = await browser.executeWorkbench(async (vscode) => {
      try {
        await vscode.commands.executeCommand("game.openGallery");
        return { ok: true };
      } catch (e: unknown) {
        return { ok: false, error: String(e) };
      }
    });
    expect((result as { ok: boolean }).ok).toBe(true);
  });

  it("game.openAi executes without error", async () => {
    const result = await browser.executeWorkbench(async (vscode) => {
      try {
        await vscode.commands.executeCommand("game.openAi");
        return { ok: true };
      } catch (e: unknown) {
        return { ok: false, error: String(e) };
      }
    });
    expect((result as { ok: boolean }).ok).toBe(true);
  });

  it("game.exportCopyJs executes without error", async () => {
    // Need active .game editor for export
    await browser.executeWorkbench(async (vscode, filePath) => {
      const doc = await vscode.workspace.openTextDocument(
        vscode.Uri.file(filePath)
      );
      await vscode.window.showTextDocument(doc);
    }, FIXTURE);
    await browser.pause(1000);

    const result = await browser.executeWorkbench(async (vscode) => {
      try {
        await vscode.commands.executeCommand("game.exportCopyJs");
        return { ok: true };
      } catch (e: unknown) {
        return { ok: false, error: String(e) };
      }
    });
    expect((result as { ok: boolean }).ok).toBe(true);
  });
});
