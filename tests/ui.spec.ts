import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { expect, test, type Page } from "@playwright/test";

type TestCase = { title: string; screenshot?: boolean };
type TestCases = { cases: Record<string, TestCase> };

const casesPath = fileURLToPath(new URL("./ui-test-cases.json", import.meta.url));
const cases = JSON.parse(readFileSync(casesPath, "utf8")) as TestCases;
const selectedCase = process.env.UI_TEST_CASE?.trim();

if (selectedCase && !cases.cases[selectedCase]) {
  throw new Error(`Unknown UI test case: ${selectedCase}`);
}

const PROJECT_PATH = "/fixtures/avatar-project";
const WORLD_PATH = "/fixtures/world-project";

function settingsFixture() {
  return {
    schemaVersion: 6,
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
  return page.addInitScript(({ projectPath, worldPath, initialSettings }) => {
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
    (window as unknown as { __mockCalls?: string[] }).__mockCalls = calls;

    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {
        metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
        invoke: async (command: string, args?: Record<string, any>) => {
          calls.push(command);
          const currentPath = args?.path ?? args?.projectPath ?? args?.request?.projectPath ?? projectPath;
          const state = projects[currentPath] ?? projects[projectPath];
          switch (command) {
            case "inspect_environment":
              return { platform: { os: "macos", architecture: "aarch64", supported: true }, git: { status: "AVAILABLE", executable: "/usr/bin/git", version: "git version 2.50.1" } };
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
              return state.worktree;
            case "read_history":
              return state.history;
            case "save_worktree": {
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
  }, { projectPath: PROJECT_PATH, worldPath: WORLD_PATH, initialSettings: settingsFixture() });
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
  await expect(page.getByRole("heading", { name: "リポジトリ設定" })).toBeVisible();
  await page.getByRole("button", { name: "現在の作業" }).click();
  await expect(page.getByRole("heading", { name: "現在の作業" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Assets/avatar.txt 変更" })).toBeVisible();
  const memo = page.getByRole("textbox", { name: "保存メモ（例: アバターの表情を調整）" });
  await memo.fill("temporary memo");
  await page.getByRole("button", { name: "保存メモをクリア" }).click();
  await expect(memo).toHaveValue("");
  await memo.fill("current UI smoke");
  await page.getByRole("button", { name: "作業を保存" }).click();
  await expect(page.getByText("保存しました: 2222222")).toBeVisible();
  await expect(page.getByText("保存対象の変更はありません。 ")).toBeVisible();
  await page.getByRole("button", { name: "保存履歴" }).click();
  await expect(page.getByRole("heading", { name: "保存履歴", level: 2 })).toBeVisible();
  await page.getByRole("button", { name: /current UI smoke/ }).click();
  await expect(page.getByRole("heading", { name: "保存の詳細" })).toBeVisible();
  await page.getByRole("button", { name: /Assets\/avatar.txt/ }).click();
  await expect(page.getByText("+updated avatar")).toBeVisible();
  await saveScreenshot(page, testInfo, "repository-work-history");
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
  await expect(page.getByText("macos / aarch64")).toBeVisible();
  await page.getByRole("button", { name: "ログと診断" }).click();
  await expect(page.getByRole("heading", { name: "ログと診断" })).toBeVisible();
  await expect(page.getByRole("button", { name: "ログ表示" })).toBeVisible();
  await saveScreenshot(page, testInfo, "global-settings");
});
