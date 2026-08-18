import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { expect, test, type Page } from "@playwright/test";

type TestCase = { title: string; screenshot?: boolean };
type TestCases = { cases: Record<string, TestCase> };

const casesPath = fileURLToPath(new URL("./ui-test-cases.json", import.meta.url));
const cases = JSON.parse(readFileSync(casesPath, "utf8")) as TestCases;
const selectedCase = process.env.UI_TEST_CASE?.trim();
const testPlatform = process.env.UI_TEST_PLATFORM === "windows" ? "windows" : "macos";
const testArchitecture = process.env.UI_TEST_ARCHITECTURE ?? (testPlatform === "windows" ? "x86_64" : "aarch64");

if (selectedCase && !cases.cases[selectedCase]) {
  throw new Error(`Unknown UI test case: ${selectedCase}`);
}

const PROJECT_PATH = "/fixtures/avatar-project";
const WORLD_PATH = "/fixtures/world-project";

function settingsFixture() {
  return {
    schemaVersion: 7,
    onboardingCompleted: true,
    recentProjects: [
      { path: PROJECT_PATH, lastOpenedAt: "2026-08-13T00:00:00Z", tags: ["avatar", "featured"] },
      { path: WORLD_PATH, lastOpenedAt: "2026-08-12T23:00:00Z", tags: ["world", "featured"] },
    ],
    logLevel: "INFO",
    vpmTrackingPolicy: "EXCLUDE_PACKAGES",
    ignoreTemplates: {
      unityRules: ["/[Ll]ibrary/*", "/[Tt]emp/"],
      vpmExcludeRules: ["com.vrchat.*"],
    },
    repositorySettings: [],
  };
}

function projectFixture(path: string) {
  const isWorld = path === WORLD_PATH;
  return {
    path,
    status: "MANAGEABLE",
    isUnityProject: true,
    unityVersion: "2022.3.22f1",
    unityRevision: null,
    projectKind: isWorld ? "VRCHAT_WORLD" : "VRCHAT_AVATAR",
    vpm: { detected: true, manifestPath: `${path}/Packages/manifest.json`, packages: [{ name: "com.vrchat.base", version: "3.8.0" }] },
    repository: { detected: true, root: path, projectIsRoot: true },
    sourceControl: {
      gitignore: { path: `${path}/.gitignore`, status: "HEALTHY", summary: "Unity rules are configured." },
      vpmPackages: { path: `${path}/Packages`, status: "HEALTHY", summary: "VPM package policy is configured." },
    },
    issues: [],
    isGitRepository: true,
  };
}

function installTauriMocks(page: Page) {
  return page.addInitScript(({ projectPath, worldPath, initialSettings, platform, architecture }) => {
    const settings = structuredClone(initialSettings);
    const projects = {
      [projectPath]: {
        project: {
          path: projectPath,
          status: "MANAGEABLE",
          isUnityProject: true,
          unityVersion: "2022.3.22f1",
          unityRevision: null,
          projectKind: "VRCHAT_AVATAR",
          vpm: { detected: true, manifestPath: `${projectPath}/Packages/manifest.json`, packages: [{ name: "com.vrchat.base", version: "3.8.0" }] },
          repository: { detected: true, root: projectPath, projectIsRoot: true },
          sourceControl: {
            gitignore: { path: `${projectPath}/.gitignore`, status: "HEALTHY", summary: "Unity rules are configured." },
            vpmPackages: { path: `${projectPath}/Packages`, status: "HEALTHY", summary: "VPM package policy is configured." },
          },
          issues: [],
          isGitRepository: true,
        },
        worktree: {
          statusToken: "work-token",
          files: [{ path: "Assets/avatar.txt", oldPath: null, changeKind: "MODIFIED", staged: false, unstaged: true, binary: false, outsideProject: false }],
          hasConflicts: false,
          hasExistingStagedChanges: false,
        },
        repositoryTree: {
          statusToken: "work-token",
          files: [
            { path: "Assets/avatar.txt", oldPath: null, changeKind: "MODIFIED", staged: false, unstaged: true, binary: false, outsideProject: false },
            { path: "Assets/avatar-material.mat", oldPath: null, changeKind: null, staged: false, unstaged: false, binary: false, outsideProject: false },
            { path: "Packages/manifest.json", oldPath: null, changeKind: null, staged: false, unstaged: false, binary: false, outsideProject: false },
          ],
        },
        history: [{ commitId: "1111111111111111111111111111111111111111", shortCommitId: "1111111", memo: "baseline", authorTime: "2026-08-12T12:00:00Z" }],
      },
      [worldPath]: {
        project: {
          path: worldPath,
          status: "MANAGEABLE",
          isUnityProject: true,
          unityVersion: "2022.3.22f1",
          unityRevision: null,
          projectKind: "VRCHAT_WORLD",
          vpm: { detected: true, manifestPath: `${worldPath}/Packages/manifest.json`, packages: [] },
          repository: { detected: true, root: worldPath, projectIsRoot: true },
          sourceControl: {
            gitignore: { path: `${worldPath}/.gitignore`, status: "HEALTHY", summary: "Unity rules are configured." },
            vpmPackages: { path: `${worldPath}/Packages`, status: "HEALTHY", summary: "VPM package policy is configured." },
          },
          issues: [],
          isGitRepository: true,
        },
        worktree: { statusToken: "world-token", files: [], hasConflicts: false, hasExistingStagedChanges: false },
        history: [],
      },
    } as Record<string, any>;
    const calls: string[] = [];
    const callbacks = new Map<string, { callback: (event: any) => void; once: boolean }>();
    const eventListeners = new Map<string, string[]>();
    let nextCallbackId = 0;
    (window as unknown as { __mockCalls?: string[] }).__mockCalls = calls;
    (window as unknown as { __mockState?: any }).__mockState = projects[projectPath];

    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {
        metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
        transformCallback: (callback: (event: any) => void, once = false) => {
          const id = `mock-callback-${++nextCallbackId}`;
          callbacks.set(id, { callback, once });
          return id;
        },
        invoke: async (command: string, args?: Record<string, any>) => {
          calls.push(command);
          if (command === "plugin:event|listen") {
            const listeners = eventListeners.get(args?.event) ?? [];
            listeners.push(args?.handler);
            eventListeners.set(args?.event, listeners);
            return args?.handler;
          }
          if (command === "plugin:event|unlisten") {
            const listeners = eventListeners.get(args?.event) ?? [];
            eventListeners.set(args?.event, listeners.filter((id) => id !== args?.eventId));
            callbacks.delete(args?.eventId);
            return null;
          }
          if (command === "plugin:event|emit") {
            for (const id of eventListeners.get(args?.event) ?? []) {
              const listener = callbacks.get(id);
              if (!listener) continue;
              listener.callback({ event: args?.event, id, payload: args?.payload });
              if (listener.once) callbacks.delete(id);
            }
            return null;
          }
          const currentPath = args?.path ?? args?.projectPath ?? args?.request?.projectPath ?? projectPath;
          const state = projects[currentPath] ?? projects[projectPath];
          switch (command) {
            case "inspect_environment":
              return {
                platform: { os: platform, architecture, supported: true },
                git: {
                  status: "AVAILABLE",
                  executable: platform === "windows" ? "C:\\Program Files\\Git\\cmd\\git.exe" : "/usr/bin/git",
                  version: platform === "windows" ? "git version 2.47.0.windows.2" : "git version 2.50.1",
                },
              };
            case "load_settings":
              return {
                settings,
                recovered: false,
                backupPath: null,
                recentProjects: settings.recentProjects.map((item: any) => ({ ...item, exists: true, projectKind: item.path === worldPath ? "VRCHAT_WORLD" : "VRCHAT_AVATAR" })),
              };
            case "save_settings":
              Object.assign(settings, structuredClone(args?.settings));
              return null;
            case "inspect_project":
              return state.project;
            case "read_repository_state":
              return { root: state.project.repository.root, needsInitialization: false, hasHead: true, branchName: "main", canSave: true, blockingReason: null };
            case "read_worktree_snapshot":
              return structuredClone(state.worktree);
            case "read_repository_tree":
              return structuredClone(state.repositoryTree ?? { statusToken: state.worktree.statusToken, files: state.worktree.files });
            case "read_history":
              {
                const offset = args?.offset ?? 0;
                const entries = state.history.slice(offset, offset + 20);
                return { entries, nextOffset: offset + entries.length < state.history.length ? offset + 20 : null };
              }
            case "save_worktree": {
              if (state.saveDelayMs) await new Promise((resolve) => setTimeout(resolve, state.saveDelayMs));
              const commitId = "2222222222222222222222222222222222222222";
              state.worktree = { ...state.worktree, files: [] };
              state.history = [{ commitId, shortCommitId: "2222222", memo: args?.request?.memo ?? "", authorTime: "2026-08-13T00:10:00Z" }, ...state.history];
              return { commitId, shortCommitId: "2222222", memo: args?.request?.memo ?? "", authorTime: "2026-08-13T00:10:00Z", fileCount: 1 };
            }
            case "read_commit_detail":
              return { commitId: "2222222222222222222222222222222222222222", shortCommitId: "2222222", memo: "current UI smoke", authorTime: "2026-08-13T00:10:00Z", parentIds: ["1111111111111111111111111111111111111111"], files: [{ path: "Assets/avatar.txt", oldPath: null, changeKind: "MODIFIED", staged: true, unstaged: false, binary: false, outsideProject: false }] };
            case "read_commit_diff":
              return { path: args?.path ?? "Assets/avatar.txt", kind: "TEXT", patch: "@@ -1 +1 @@\n-avatar\n+updated avatar", truncated: false, truncationReason: null };
            case "read_worktree_diff":
              return { path: args?.path ?? "Assets/avatar.txt", kind: "TEXT", patch: "@@ -1 +1 @@\n-avatar\n+working", truncated: false, truncationReason: null };
            case "preview_ignore_rules":
              return { statusToken: "ignore-token", repositoryRoot: state.project.repository.root, canApply: true, blockingReason: null, ignoreFiles: [{ path: ".gitignore", missingRules: [], willCreate: false }] };
            case "read_recent_logs":
              return { currentFile: "vsedi.log.2026-08-13", lines: ["2026-08-13T00:00:00Z INFO UI smoke test"] };
            case "open_log_window":
            case "open_log_directory":
            case "export_diagnostic_log":
              return null;
            default:
              return null;
          }
        },
      },
    });
  }, {
    projectPath: PROJECT_PATH,
    worldPath: WORLD_PATH,
    initialSettings: settingsFixture(),
    platform: testPlatform,
    architecture: testArchitecture,
  });
}

async function prepare(page: Page) {
  await installTauriMocks(page);
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "ホーム" })).toBeVisible();
}

async function saveScreenshot(page: Page, testInfo: { outputPath: (name: string) => string }, name: string) {
  if (cases.cases[name]?.screenshot) await page.screenshot({ path: testInfo.outputPath(`${name}-success.png`), fullPage: true });
}

test("home-project-management", async ({ page }, testInfo) => {
  test.skip(Boolean(selectedCase && selectedCase !== "home-project-management"), `Only ${selectedCase} was requested`);
  await prepare(page);
  await expect(page.getByRole("button", { name: "Projectを追加" })).toBeVisible();
  await expect(page.locator("header")).toBeVisible();
  await expect(page.getByRole("button", { name: "お知らせを最小化" })).toBeVisible();
  await expect(page.getByText("管理しているProject")).toBeVisible();
  await expect(page.getByRole("img", { name: "アバター" })).toBeVisible();
  await expect(page.getByRole("img", { name: "ワールド" })).toBeVisible();
  await page.getByRole("button", { name: "お知らせを最小化" }).click();
  await expect(page.getByRole("button", { name: "お知らせを展開" })).toBeVisible();
  await page.getByRole("button", { name: "Projectを検索" }).click();
  const search = page.getByRole("textbox", { name: "Projectを検索" });
  await search.fill("world");
  await expect(page.getByText("world-project", { exact: true })).toBeVisible();
  await expect(page.getByText("avatar-project", { exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "検索文字をクリア" }).click();
  await expect(page.getByText("avatar-project", { exact: true })).toBeVisible();
  await expect(page.getByText("world-project", { exact: true })).toBeVisible();
  await saveScreenshot(page, testInfo, "home-project-management");
});

test("repository-work-history", async ({ page }, testInfo) => {
  test.skip(Boolean(selectedCase && selectedCase !== "repository-work-history"), `Only ${selectedCase} was requested`);
  await prepare(page);
  await page.getByRole("button", { name: "avatar-projectのリポジトリ設定を開く" }).click();
  await expect(page.getByRole("heading", { name: "このProjectの設定" })).toBeVisible();
  await page.getByRole("button", { name: "現在の作業" }).click();
  await expect(page.getByRole("heading", { name: "現在の変更" })).toBeVisible();
  await expect(page.locator("header")).toHaveCount(0);
  await expect(page.getByRole("group", { name: "選択中のProject" })).toBeVisible();
  await expect(page.getByRole("button", { name: "projectの情報を展開する" })).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByRole("button", { name: "保存状態の情報を展開する" })).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByRole("button", { name: "診断の情報を展開する" })).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByRole("button", { name: "Assets/avatar.txt 変更" })).toBeVisible();
  await expect(page.getByText("名前", { exact: true })).toBeVisible();
  await expect(page.getByText("テキスト", { exact: true })).toBeVisible();
  await expect(page.getByText("作業中", { exact: true })).toBeVisible();
  await expect(page.getByText("未保存の変更あり", { exact: true })).toHaveCount(2);
  await expect(page.locator("span").filter({ hasText: "未保存の変更あり" }).first()).toHaveClass(/bg-rose-100/);
  await expect(page.getByRole("button", { name: "Assetsを折りたたむ" })).toBeVisible();
  const detailColumnHandle = page.getByRole("button", { name: "詳細列の幅を調整" }).first();
  await expect(detailColumnHandle).toBeVisible();
  const detailHandleBox = await detailColumnHandle.boundingBox();
  expect(detailHandleBox).not.toBeNull();
  const gridBeforeResize = await page.locator('[data-file-tree-header="true"]').getAttribute("style");
  if (detailHandleBox) {
    await page.mouse.move(detailHandleBox.x + detailHandleBox.width / 2, detailHandleBox.y + detailHandleBox.height / 2);
    await page.mouse.down();
    await page.mouse.move(detailHandleBox.x + detailHandleBox.width / 2 + 120, detailHandleBox.y + detailHandleBox.height / 2);
    await page.mouse.up();
  }
  await expect(page.locator('[data-file-tree-header="true"]')).not.toHaveAttribute("style", gridBeforeResize ?? "");
  await page.evaluate(() => {
    void (window as unknown as { __TAURI_INTERNALS__: { invoke: (command: string, args: Record<string, unknown>) => Promise<unknown> } }).__TAURI_INTERNALS__.invoke("plugin:event|emit", {
      event: "git-command-output",
      payload: {
        operation: "save_worktree",
        executable: "/usr/bin/git",
        args: ["commit", "-m", "current UI smoke"],
        phase: "STARTED",
        stream: null,
        text: "",
        status: null,
      },
    });
  });
  await expect(page.getByText("Git CLIの実行結果")).toBeVisible();
  await expect(page.getByRole("log")).toContainText("git commit");
  const memo = page.getByRole("textbox", { name: "保存メモ（例: アバターの表情を調整）" });
  await memo.fill("temporary memo");
  await page.getByRole("button", { name: "保存メモをクリア" }).click();
  await expect(memo).toHaveValue("");
  await memo.fill("current UI smoke");
  await expect(page.getByRole("button", { name: "作業を保存" })).toHaveClass(/bg-rose-600/);
  await page.getByRole("button", { name: "作業を保存" }).click();
  await expect(page.getByText("保存しました: 2222222")).toBeVisible();
  await expect(page.getByText("保存済み", { exact: true })).toHaveCount(2);
  await expect(page.getByText("保存対象の変更はありません。 ")).toBeVisible();
  await page.getByRole("button", { name: "保存履歴" }).click();
  await expect(page.getByRole("heading", { name: "保存履歴" })).toBeVisible();
  await expect(page.getByText("保存メモ", { exact: true })).toBeVisible();
  await expect(page.getByText("保存日時", { exact: true })).toBeVisible();
  await expect(page.getByText("commit", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: /current UI smoke/ }).click();
  await expect(page.getByRole("heading", { name: "保存の詳細" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Assetsを折りたたむ" })).toBeVisible();
  await expect(page.getByRole("button", { name: "詳細列の幅を調整" })).toBeVisible();
  await expect(page.getByText("保存済み", { exact: true })).toHaveCount(2);
  await expect(page.getByText("詳細", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: /Assets\/avatar.txt/ }).click();
  await expect(page.getByText("+updated avatar")).toBeVisible();
  await saveScreenshot(page, testInfo, "repository-work-history");
});

test("repository-history-scrolls-long-list", async ({ page }) => {
  test.skip(Boolean(selectedCase && selectedCase !== "repository-history-scrolls-long-list"), `Only ${selectedCase} was requested`);
  await prepare(page);
  await page.evaluate(() => {
    const state = (window as unknown as { __mockState?: { history: unknown[] } }).__mockState;
    if (state) {
      state.history = Array.from({ length: 40 }, (_, index) => ({
        commitId: String(index + 1).padStart(40, "0"),
        shortCommitId: String(index + 1).padStart(7, "0"),
        memo: `history entry ${index + 1}`,
        authorTime: "2026-08-13T00:00:00Z",
      }));
    }
  });
  await page.getByRole("button", { name: "avatar-projectのリポジトリ設定を開く" }).click();
  await page.getByRole("button", { name: "保存履歴" }).click();
  await expect(page.getByRole("heading", { name: "保存履歴" })).toBeVisible();
  const content = page.locator('[data-app-content="true"]');
  await expect(content).toHaveClass(/overflow-y-auto/);
  const metrics = await content.evaluate((element) => ({
    clientHeight: element.clientHeight,
    overflowY: getComputedStyle(element).overflowY,
    scrollHeight: element.scrollHeight,
  }));
  expect(metrics.overflowY).toBe("auto");
  expect(metrics.scrollHeight).toBeGreaterThan(metrics.clientHeight);
  await content.evaluate((element) => { element.scrollTop = element.scrollHeight; });
  await expect.poll(() => content.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
});

test("repository-history-loads-older", async ({ page }) => {
  test.skip(Boolean(selectedCase && selectedCase !== "repository-history-loads-older"), `Only ${selectedCase} was requested`);
  await prepare(page);
  await page.evaluate(() => {
    const state = (window as unknown as { __mockState?: { history: unknown[] } }).__mockState;
    if (state) {
      state.history = Array.from({ length: 40 }, (_, index) => ({
        commitId: String(index + 1).padStart(40, "0"),
        shortCommitId: String(index + 1).padStart(7, "0"),
        memo: `history entry ${index + 1}`,
        authorTime: "2026-08-13T00:00:00Z",
      }));
    }
  });
  await page.getByRole("button", { name: "avatar-projectのリポジトリ設定を開く" }).click();
  await page.getByRole("button", { name: "保存履歴" }).click();
  const historyList = page.getByRole("list", { name: "保存履歴" });
  await expect(historyList.getByRole("button")).toHaveCount(20);
  await expect(page.getByRole("button", { name: "さらに読み込む" })).toBeVisible();
  await page.getByRole("button", { name: "さらに読み込む" }).click();
  await expect(historyList.getByRole("button")).toHaveCount(40);
  await expect(page.getByRole("button", { name: "さらに読み込む" })).toHaveCount(0);
  await expect(historyList.getByRole("button", { name: /history entry 40/ })).toBeVisible();
});

test("repository-work-expands-summary-on-abnormal-state", async ({ page }) => {
  test.skip(Boolean(selectedCase && selectedCase !== "repository-work-expands-summary-on-abnormal-state"), `Only ${selectedCase} was requested`);
  await prepare(page);
  await page.evaluate(() => {
    const state = (window as unknown as { __mockState?: { project: { status: string; issues: unknown[] } } }).__mockState;
    if (state) {
      state.project.status = "NEEDS_ATTENTION";
      state.project.issues = [{ code: "TEST_WARNING", severity: "WARNING", message: "確認が必要な状態です。", path: null }];
    }
  });
  await page.getByRole("button", { name: "avatar-projectのリポジトリ設定を開く" }).click();
  await page.getByRole("button", { name: "現在の作業" }).click();
  await expect(page.getByRole("button", { name: "projectの情報を折りたたむ" })).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByRole("button", { name: "診断の情報を折りたたむ" })).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByRole("button", { name: "保存状態の情報を展開する" })).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByText("1 件の確認項目", { exact: true })).toBeVisible();
});

test("repository-work-shows-save-progress", async ({ page }) => {
  test.skip(Boolean(selectedCase && selectedCase !== "repository-work-shows-save-progress"), `Only ${selectedCase} was requested`);
  await prepare(page);
  await page.getByRole("button", { name: "avatar-projectのリポジトリ設定を開く" }).click();
  await page.getByRole("button", { name: "現在の作業" }).click();
  await expect(page.getByRole("button", { name: "Assets/avatar.txt 変更" })).toBeVisible();
  await page.evaluate(() => {
    const state = (window as unknown as { __mockState?: { saveDelayMs?: number } }).__mockState;
    if (state) state.saveDelayMs = 500;
  });
  await page.getByRole("textbox", { name: "保存メモ（例: アバターの表情を調整）" }).fill("save progress");
  await page.getByRole("button", { name: "作業を保存" }).click();
  await expect(page.getByRole("heading", { name: "Gitで保存中" })).toBeVisible();
  await expect(page.getByRole("log")).toContainText("Git CLIを起動しています");
  await page.evaluate(() => {
    void (window as unknown as { __TAURI_INTERNALS__: { invoke: (command: string, args: Record<string, unknown>) => Promise<unknown> } }).__TAURI_INTERNALS__.invoke("plugin:event|emit", {
      event: "git-command-output",
      payload: {
        operation: "save_worktree",
        executable: "/usr/bin/git",
        args: ["commit", "-m", "save progress"],
        phase: "OUTPUT",
        stream: "STDOUT",
        text: "created commit\n",
        status: null,
      },
    });
  });
  await expect(page.getByRole("log")).toContainText("created commit");
  await expect(page.getByText("保存しました: 2222222")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Git CLIの実行結果" })).toBeVisible();
});

test("repository-work-refreshes-changes", async ({ page }) => {
  test.skip(Boolean(selectedCase && selectedCase !== "repository-work-refreshes-changes"), `Only ${selectedCase} was requested`);
  await prepare(page);
  await page.getByRole("button", { name: "avatar-projectのリポジトリ設定を開く" }).click();
  await page.getByRole("button", { name: "現在の作業" }).click();
  await expect(page.getByRole("button", { name: "Assets/avatar.txt 変更" })).toBeVisible();
  await page.evaluate(() => {
    const state = (window as unknown as { __mockState?: { worktree: any } }).__mockState;
    if (state) state.worktree = {
      statusToken: "refreshed-token",
      files: [{ path: "ProjectSettings/EditorSettings.asset", oldPath: null, changeKind: "ADDED", staged: false, unstaged: true, binary: false, outsideProject: false }],
      hasConflicts: false,
      hasExistingStagedChanges: false,
    };
  });
  await page.getByRole("button", { name: "現在の変更を再読込" }).click();
  await expect(page.getByRole("button", { name: "ProjectSettingsを折りたたむ" })).toBeVisible();
  await expect(page.getByRole("button", { name: "ProjectSettings/EditorSettings.asset 追加" })).toBeVisible();
  await expect(page.getByText("未保存の変更あり", { exact: true })).toHaveCount(2);
  await expect(page.getByRole("button", { name: "Assets/avatar.txt 変更" })).toHaveCount(0);
});

test("repository-work-toggles-file-view", async ({ page }) => {
  test.skip(Boolean(selectedCase && selectedCase !== "repository-work-toggles-file-view"), `Only ${selectedCase} was requested`);
  await prepare(page);
  await page.getByRole("button", { name: "avatar-projectのリポジトリ設定を開く" }).click();
  await page.getByRole("button", { name: "現在の作業" }).click();
  await expect(page.getByRole("button", { name: "Assets/avatar.txt 変更" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Assets/avatar-material.mat 変更なし" })).toHaveCount(0);
  const allFiles = page.getByRole("button", { name: "フォルダ内全体を表示" });
  await allFiles.click();
  await expect(allFiles).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByRole("button", { name: "Assets/avatar-material.mat 変更なし" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Packages/manifest.json 変更なし" })).toBeVisible();
  const changedOnly = page.getByRole("button", { name: "変更のみを表示" });
  await changedOnly.click();
  await expect(changedOnly).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByRole("button", { name: "Assets/avatar-material.mat 変更なし" })).toHaveCount(0);
});

test("repository-work-warns-before-saving-stale-state", async ({ page }) => {
  test.skip(Boolean(selectedCase && selectedCase !== "repository-work-warns-before-saving-stale-state"), `Only ${selectedCase} was requested`);
  await prepare(page);
  await page.getByRole("button", { name: "avatar-projectのリポジトリ設定を開く" }).click();
  await page.getByRole("button", { name: "現在の作業" }).click();
  await expect(page.getByRole("button", { name: "Assets/avatar.txt 変更" })).toBeVisible();
  const memo = page.getByRole("textbox", { name: "保存メモ（例: アバターの表情を調整）" });
  await memo.fill("stale state check");
  await page.evaluate(() => {
    const state = (window as unknown as { __mockState?: { worktree: { statusToken: string } } }).__mockState;
    if (state) state.worktree.statusToken = "changed-before-save";
  });
  await page.getByRole("button", { name: "作業を保存" }).click();
  await expect(page.getByRole("alert")).toContainText("保存前に変更内容が変わったため、保存を停止しました");
  await expect(page.getByText("保存前の変更を確認してください")).toBeVisible();
  await expect(page.getByText("保存しました:")).toHaveCount(0);
  const calls = await page.evaluate(() => (window as unknown as { __mockCalls?: string[] }).__mockCalls ?? []);
  expect(calls).not.toContain("save_worktree");
});

test("repository-settings", async ({ page }, testInfo) => {
  test.skip(Boolean(selectedCase && selectedCase !== "repository-settings"), `Only ${selectedCase} was requested`);
  await prepare(page);
  await page.getByRole("button", { name: "avatar-projectのリポジトリ設定を開く" }).click();
  await expect(page.getByRole("heading", { name: "このProjectの設定" })).toBeVisible();
  const tagInput = page.getByRole("textbox", { name: "例: Avatar、作業中" });
  await tagInput.fill("temporary");
  await page.getByRole("button", { name: "タグ入力をクリア" }).click();
  await expect(tagInput).toHaveValue("");
  await tagInput.fill("smoke");
  await page.getByRole("button", { name: "タグを追加" }).click();
  await expect(page.getByText("smoke")).toBeVisible();
  const saveTags = page.getByRole("button", { name: "タグを保存" });
  await expect(saveTags).toHaveClass(/bg-rose-600/);
  await saveTags.click();
  await expect(saveTags).toBeDisabled();
  await page.getByRole("button", { name: "含める" }).click();
  await expect(page.getByText(/repository固有の設定/)).toBeVisible();
  await saveScreenshot(page, testInfo, "repository-settings");
});

test("global-settings", async ({ page }, testInfo) => {
  test.skip(Boolean(selectedCase && selectedCase !== "global-settings"), `Only ${selectedCase} was requested`);
  await prepare(page);
  await page.getByRole("button", { name: "全体設定" }).click();
  await expect(page.getByRole("heading", { name: "一般" })).toBeVisible();
  await page.getByRole("button", { name: "新規repositoryの既定値" }).click();
  await expect(page.getByRole("heading", { name: "新規repositoryの既定値" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "ignore template" })).toBeVisible();
  await page.getByRole("button", { name: "実行環境" }).click();
  await expect(page.getByText(`${testPlatform} / ${testArchitecture}`)).toBeVisible();
  await page.getByRole("button", { name: "ログと診断" }).click();
  await expect(page.getByRole("heading", { name: "ログと診断" })).toBeVisible();
  await expect(page.getByRole("button", { name: "ログ表示" })).toBeVisible();
  await saveScreenshot(page, testInfo, "global-settings");
});
