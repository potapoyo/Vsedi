import path from "node:path";

const appBinaryPath = process.env.TAURI_APP_BINARY
  ? path.resolve(process.env.TAURI_APP_BINARY)
  : path.resolve(
      "src-tauri",
      "target",
      "debug",
      process.platform === "win32" ? "vsedi.exe" : "vsedi",
    );

export const config: WebdriverIO.Config = {
  runner: "local",
  specs: ["./tests/native-ui.spec.ts"],
  maxInstances: 1,
  capabilities: [
    {
      browserName: "tauri",
      "tauri:options": {
        application: appBinaryPath,
      },
    },
  ],
  services: [
    [
      "@wdio/tauri-service",
      {
        appBinaryPath,
        captureBackendLogs: true,
        driverProvider: "embedded",
        embeddedPort: 4445,
      },
    ],
  ],
  framework: "mocha",
  reporters: ["spec"],
  outputDir: path.resolve("wdio-logs"),
  logLevel: "info",
  bail: 0,
  waitforTimeout: 15_000,
  connectionRetryTimeout: 120_000,
  connectionRetryCount: 1,
  mochaOpts: {
    ui: "bdd",
    timeout: 60_000,
  },
  afterTest: async (_test, _context, result) => {
    if (!result.passed) {
      await browser.saveScreenshot(
        path.resolve("test-results", "native-ui", `${process.platform}-failure.png`),
      );
    }
  },
};
