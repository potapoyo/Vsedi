import { mkdirSync } from "node:fs";
import { resolve } from "node:path";

const resultsDirectory = resolve("wdio-results");

export const config: WebdriverIO.Config = {
  runner: "local",
  specs: ["./native-tests/specs/**/*.e2e.ts"],
  maxInstances: 1,
  capabilities: [{ browserName: "tauri" }],
  services: [["@wdio/tauri-service", {
    appBinaryPath: resolve("src-tauri/target/release/vsedi.exe"),
    driverProvider: "external",
    autoInstallTauriDriver: true,
    autoDownloadEdgeDriver: true,
  }]],
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: {
    timeout: 60_000,
  },
  waitforTimeout: 15_000,
  connectionRetryTimeout: 120_000,
  connectionRetryCount: 2,
  beforeSession() {
    mkdirSync(resultsDirectory, { recursive: true });
  },
  async afterTest(test, _context, result) {
    const status = result.passed ? "success" : "failure";
    const safeTitle = test.title.replace(/[^a-zA-Z0-9_-]+/g, "-").replace(/^-|-$/g, "");
    await browser.saveScreenshot(resolve(resultsDirectory, `${safeTitle}-${status}.png`));
  },
};
