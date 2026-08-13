import { mkdirSync } from "node:fs";
import path from "node:path";
import { browser, expect, $ } from "@wdio/globals";

const artifactDirectory = path.resolve("test-results", "native-ui");
const expectedPlatform = process.platform === "win32" ? "windows" : "macos";
const expectedArchitecture = process.platform === "win32" ? "x86_64" : "aarch64";

describe("Vsedi native UI smoke", () => {
  before(() => {
    mkdirSync(artifactDirectory, { recursive: true });
  });

  it("starts the native window and shows the actual execution environment", async () => {
    const homeHeading = await $("h2");
    await homeHeading.waitForDisplayed();
    await expect(homeHeading).toHaveText("ホーム");
    await expect(await $("button=Projectを追加")).toBeDisplayed();

    await (await $("button=全体設定")).click();
    await (await $("button=実行環境")).click();

    await browser.waitUntil(
      async () => (await (await $("body")).getText()).includes(`${expectedPlatform} / ${expectedArchitecture}`),
      {
        timeout: 15_000,
        timeoutMsg: `実行環境に ${expectedPlatform} / ${expectedArchitecture} が表示されませんでした`,
      },
    );
    await expect(await $("body")).toHaveText(expect.stringContaining("System Git"));

    await browser.saveScreenshot(
      path.join(artifactDirectory, `${expectedPlatform}-${expectedArchitecture}-success.png`),
    );
  });
});
