import { browser } from "@wdio/globals";

describe("Extension Activation", () => {
  it("activates without error", async () => {
    const isActive = (await browser.executeWorkbench(async (vscode) => {
      const ext = vscode.extensions.getExtension("4da-systems.game-language");
      if (!ext) return false;
      await ext.activate();
      return ext.isActive;
    })) as boolean;
    expect(isActive).toBe(true);
  });

  it("registers all commands", async () => {
    const commands = (await browser.executeWorkbench(async (vscode) => {
      const all = await vscode.commands.getCommands(true);
      return all.filter((c: string) => c.startsWith("game."));
    })) as string[];

    const expected = [
      "game.openPreview",
      "game.export",
      "game.exportCopyJs",
      "game.exportCopyHtml",
      "game.exportCopyReact",
      "game.exportSaveJs",
      "game.exportSaveHtml",
      "game.openGallery",
      "game.openAi",
    ];

    for (const cmd of expected) {
      expect(commands).toContain(cmd);
    }
  });

  it("reads game.serverPath setting", async () => {
    const serverPath = (await browser.executeWorkbench(async (vscode) => {
      return vscode.workspace.getConfiguration("game").get("serverPath");
    })) as string;
    expect(serverPath).toBeTruthy();
    expect(typeof serverPath).toBe("string");
  });
});
