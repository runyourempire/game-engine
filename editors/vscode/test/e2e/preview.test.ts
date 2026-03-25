import { browser } from "@wdio/globals";
import path from "path";
import fs from "fs";

const FIXTURE = path.join(__dirname, "../fixtures/hello.game");
const SCREENSHOT_DIR = path.join(__dirname, "../screenshots");

describe("Preview WebView Panel", () => {
  before(async () => {
    fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });

    // Open a .game file and trigger preview
    await browser.executeWorkbench(async (vscode, filePath) => {
      const doc = await vscode.workspace.openTextDocument(
        vscode.Uri.file(filePath)
      );
      await vscode.window.showTextDocument(doc);
      await vscode.commands.executeCommand("game.openPreview");
    }, FIXTURE);

    // Wait for compile + WebView render
    await browser.pause(5000);
  });

  it("opens preview panel", async () => {
    const workbench = await browser.getWorkbench();
    const webviews = await workbench.getAllWebviews();
    expect(webviews.length).toBeGreaterThan(0);
  });

  it("renders iframe in preview", async () => {
    const workbench = await browser.getWorkbench();
    const webviews = await workbench.getAllWebviews();
    const preview = webviews[0];
    await preview.open();

    const iframe = await $("iframe#preview-frame");
    await expect(iframe).toExist();

    await preview.close();
  });

  it("captures screenshot for visual verification", async () => {
    const screenshotPath = path.join(SCREENSHOT_DIR, "preview-panel.png");
    await browser.saveScreenshot(screenshotPath);
    expect(fs.existsSync(screenshotPath)).toBe(true);
  });
});
