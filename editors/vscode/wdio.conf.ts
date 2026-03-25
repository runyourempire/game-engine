import path from "path";

const isWindows = process.platform === "win32";
const gameBinary = path.join(
  __dirname,
  "..",
  "..",
  "target",
  "release",
  isWindows ? "game.exe" : "game"
);

export const config: WebdriverIO.Config = {
  runner: "local",
  specs: ["./test/e2e/**/*.test.ts"],
  maxInstances: 1,
  framework: "mocha",
  reporters: ["spec"],
  services: ["vscode"],
  capabilities: [
    {
      browserName: "vscode",
      browserVersion: "stable",
      "wdio:vscodeOptions": {
        extensionPath: __dirname,
        userSettings: {
          "game.serverPath": gameBinary,
          "game.trace.server": "off",
          "editor.quickSuggestionsDelay": 0,
        },
        workspacePath: path.join(__dirname, "test", "fixtures"),
      },
    },
  ],
  mochaOpts: {
    timeout: 60000,
    retries: 1,
  },

  afterTest: async function (
    test: { title: string },
    _context: unknown,
    { error }: { error?: Error }
  ) {
    if (error) {
      const safeName = test.title.replace(/[^a-z0-9]/gi, "-").toLowerCase();
      const dir = path.join(__dirname, "test", "screenshots");
      await browser.saveScreenshot(path.join(dir, `FAIL-${safeName}.png`));
    }
  },
};
