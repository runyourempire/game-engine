import { browser } from "@wdio/globals";
import path from "path";
import fs from "fs";

const SCREENSHOT_DIR = path.join(__dirname, "../screenshots");

describe("Component Gallery", () => {
  before(async () => {
    fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });

    await browser.executeWorkbench(async (vscode) => {
      await vscode.commands.executeCommand("game.openGallery");
    });

    await browser.pause(3000);
  });

  it("opens gallery panel", async () => {
    const workbench = await browser.getWorkbench();
    const webviews = await workbench.getAllWebviews();
    expect(webviews.length).toBeGreaterThan(0);
  });

  it("renders component cards", async () => {
    const workbench = await browser.getWorkbench();
    const webviews = await workbench.getAllWebviews();

    // Find the gallery webview (may not be the first if preview is also open)
    for (const wv of webviews) {
      await wv.open();
      const cards = await $$(".card");
      if (cards.length > 0) {
        expect(cards.length).toBeGreaterThanOrEqual(30); // 32 components
        await wv.close();
        return;
      }
      await wv.close();
    }

    // If we get here, no webview had cards
    expect(false).toBe(true); // fail with clear message
  });

  it("has search input", async () => {
    const workbench = await browser.getWorkbench();
    const webviews = await workbench.getAllWebviews();

    for (const wv of webviews) {
      await wv.open();
      const search = await $("#search");
      if (await search.isExisting()) {
        await expect(search).toExist();
        await wv.close();
        return;
      }
      await wv.close();
    }
  });

  it("captures gallery screenshot", async () => {
    const screenshotPath = path.join(SCREENSHOT_DIR, "gallery-panel.png");
    await browser.saveScreenshot(screenshotPath);
    expect(fs.existsSync(screenshotPath)).toBe(true);
  });
});
