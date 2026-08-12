import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { expect, test, type Page } from "@playwright/test";

type Navigation = { action: "click" | "navigate" | "fill"; selector?: string; url?: string; value?: string };
type Assertion = { type: "element" | "button" | "text"; selector: string; exists?: boolean; text?: string; contains?: string };
type Screen = { title: string; url: string; navigations?: Navigation[]; assertions: Assertion[]; screenshot?: { onSuccess?: boolean; onFailure?: boolean } };

const casesPath = fileURLToPath(new URL("./ui-test-cases.json", import.meta.url));
const cases = JSON.parse(readFileSync(casesPath, "utf8")) as { screens: Record<string, Screen> };
const selectedCase = process.env.UI_TEST_CASE?.trim();

if (selectedCase && !cases.screens[selectedCase]) throw new Error(`Unknown UI test case: ${selectedCase}`);

async function navigate(page: Page, action: Navigation) {
  if (action.action === "navigate" && action.url) await page.goto(action.url);
  if (action.action === "click" && action.selector) await page.locator(action.selector).click();
  if (action.action === "fill" && action.selector) await page.locator(action.selector).fill(action.value ?? "");
}

async function installTauriMocks(page: Page) {
  await page.addInitScript(() => {
    const responses: Record<string, unknown> = {
      inspect_environment: {
        platform: { os: "macos", architecture: "aarch64", supported: true },
        git: { status: "AVAILABLE", executable: "/usr/bin/git", version: "git version 2.50.0" },
      },
      load_settings: {
        settings: {
          schemaVersion: 1,
          onboardingCompleted: true,
          recentProjects: [{ path: "/fixtures/test-project", lastOpenedAt: "2026-08-12T09:00:00Z" }],
          logLevel: "INFO",
          vpmTrackingPolicy: "EXCLUDE_PACKAGES",
        },
        recovered: false,
        backupPath: null,
        recentProjects: [{ path: "/fixtures/test-project", lastOpenedAt: "2026-08-12T09:00:00Z", exists: true }],
      },
      inspect_project: {
        path: "/fixtures/test-project",
        status: "MANAGEABLE",
        isUnityProject: true,
        unityVersion: "2022.3.22f1",
        unityRevision: null,
        projectKind: "VRCHAT_WORLD",
        vpm: { detected: true, manifestPath: "/fixtures/test-project/Packages/vpm-manifest.json", packages: [] },
        repository: { detected: true, root: "/fixtures/test-project", projectIsRoot: true },
        sourceControl: {
          gitignore: { path: "/fixtures/test-project/.gitignore", status: "HEALTHY", summary: "Unity rules are configured." },
          vpmPackages: { path: "/fixtures/test-project/Packages", status: "HEALTHY", summary: "VPM package policy is configured." },
        },
        issues: [],
        isGitRepository: true,
      },
    };
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: { invoke: async (command: string) => responses[command] },
    });
  });
}

for (const [name, screen] of Object.entries(cases.screens)) {
  test(name, async ({ page }, testInfo) => {
    test.skip(Boolean(selectedCase && selectedCase !== name), `Only ${selectedCase} was requested`);
    await installTauriMocks(page);
    await page.goto(screen.url);
    for (const action of screen.navigations ?? []) await navigate(page, action);

    for (const assertion of screen.assertions) {
      const candidates = page.locator(assertion.selector);
      const locator = (assertion.type === "button" && assertion.text ? candidates.filter({ hasText: assertion.text }) : candidates).first();
      if (assertion.exists === false) {
        await expect(locator).toHaveCount(0);
        continue;
      }
      await expect(locator).toBeVisible();
      if (assertion.text) await expect(locator).toContainText(assertion.text);
      if (assertion.contains) await expect(locator).toContainText(assertion.contains);
    }

    if (screen.screenshot?.onSuccess) {
      await page.screenshot({ path: testInfo.outputPath(`${name}-success.png`), fullPage: true });
    }
  });
}
