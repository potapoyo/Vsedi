import { expect } from "@wdio/globals";

describe("Vsedi Windows native UI", () => {
  it("starts the Tauri application and completes Rust diagnostics", async () => {
    const heading = await $("h1=Vsedi");
    await expect(heading).toBeDisplayed();

    const environment = await $("h2=実行環境");
    await expect(environment).toBeDisplayed();

    const refreshButton = await $("button=再診断");
    await expect(refreshButton).toBeClickable();

    const internalError = await $("*=INTERNAL_ERROR");
    await expect(internalError).not.toExist();
  });

  it("opens the native log window", async () => {
    const initialHandles = await browser.getWindowHandles();
    await $("button=ログ表示").click();

    await browser.waitUntil(
      async () => (await browser.getWindowHandles()).length > initialHandles.length,
      { timeout: 15_000, timeoutMsg: "ログウィンドウが開きませんでした" },
    );

    const handles = await browser.getWindowHandles();
    const logHandle = handles.find((handle) => !initialHandles.includes(handle));
    expect(logHandle).toBeDefined();
    await browser.switchToWindow(logHandle!);
    await expect($("pre")).toBeDisplayed();
    await expect($("pre")).toHaveText(expect.stringContaining("ログ"));
  });
});
