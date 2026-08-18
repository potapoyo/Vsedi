import { useEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { StatusPill } from "@/components/ui/status-pill";
import { LogWindow } from "@/LogWindow";
import type { AppError, CommitDetail, EnvironmentDiagnostic, FileDiff, GitCommandEvent, HistoryEntry, IgnoreTemplateSettings, ProjectDiagnostic, ProjectKind, RepositoryIgnorePreview, RepositoryInitializationPreview, RepositoryState, RepositoryTreeSnapshot, SaveResult, SettingsLoadResult, VpmTrackingPolicy, WorktreeSnapshot } from "@/generated/bindings";
import { applyIgnoreRules, exportDiagnosticLog, initializeRepository, inspectEnvironment, inspectProject, isAppError, loadSettings, openLogDirectory, openLogWindow, previewIgnoreRules, previewRepositoryInitialization, readCommitDetail, readCommitDiff, readHistory, readRepositoryState, readRepositoryTree, readWorktreeDiff, readWorktreeSnapshot, saveSettings, saveWorktree } from "@/lib/commands";

type GlobalSettingsSection = "GENERAL" | "DEFAULTS" | "ENVIRONMENT" | "LOGGING";
type RepositorySection = "WORK" | "HISTORY" | "SETTINGS";
type FileViewMode = "CHANGES" | "ALL";
type AppRoute =
  | { page: "HOME" }
  | { page: "GLOBAL_SETTINGS"; section: GlobalSettingsSection }
  | { page: "REPOSITORY"; section: RepositorySection };

let appReadySignaled = false;

function App() {
  useEffect(() => {
    if (appReadySignaled) return;
    appReadySignaled = true;
    // Browser-based UI tests do not have the Tauri event bridge. The rejected
    // promise is intentionally ignored so the same React entrypoint works there.
    void emit("app-ready").catch(() => undefined);
  }, []);

  if (currentWindowLabel() === "logs") return <LogWindow />;
  return <MainWindow />;
}

function MainWindow() {
  const [route, setRoute] = useState<AppRoute>({ page: "HOME" });
  const [environment, setEnvironment] = useState<EnvironmentDiagnostic | null>(null);
  const [settings, setSettings] = useState<SettingsLoadResult | null>(null);
  const [project, setProject] = useState<ProjectDiagnostic | null>(null);
  const [repositoryState, setRepositoryState] = useState<RepositoryState | null>(null);
  const [worktree, setWorktree] = useState<WorktreeSnapshot | null>(null);
  const [repositoryTree, setRepositoryTree] = useState<RepositoryTreeSnapshot | null>(null);
  const [fileViewMode, setFileViewMode] = useState<FileViewMode>("CHANGES");
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [historyNextOffset, setHistoryNextOffset] = useState<number | null>(null);
  const [commitDetail, setCommitDetail] = useState<CommitDetail | null>(null);
  const [fileDiff, setFileDiff] = useState<FileDiff | null>(null);
  const [initializationPreview, setInitializationPreview] = useState<RepositoryInitializationPreview | null>(null);
  const [ignorePreview, setIgnorePreview] = useState<RepositoryIgnorePreview | null>(null);
  const [saveResult, setSaveResult] = useState<SaveResult | null>(null);
  const [gitOutput, setGitOutput] = useState<GitCommandEvent[]>([]);
  const [pending, setPending] = useState<string | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const selectionGeneration = useRef(0);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    void listen<GitCommandEvent>("git-command-output", (event) => {
      if (!active || event.payload.operation !== "save_worktree") return;
      setGitOutput((current) => [...current, event.payload].slice(-300));
    })
      .then((cleanup) => {
        if (active) {
          unlisten = cleanup;
        } else {
          void cleanup();
        }
      })
      .catch(() => undefined);
    return () => {
      active = false;
      if (unlisten) void unlisten();
    };
  }, []);

  const isBusy = pending !== null;
  const run = async (operation: string, action: () => Promise<void>) => {
    setPending(operation);
    setError(null);
    try {
      await action();
    } catch (caught) {
      setError(normalizeError(caught));
    } finally {
      setPending(null);
    }
  };

  const refreshApplication = async () => {
    await run("アプリ情報を更新", async () => {
      const [environmentResult, settingsResult] = await Promise.all([inspectEnvironment(), loadSettings()]);
      setEnvironment(environmentResult);
      setSettings(settingsResult);
    });
  };

  useEffect(() => {
    void refreshApplication();
  }, []);

  const clearRepositoryData = () => {
    setRepositoryState(null);
    setWorktree(null);
    setRepositoryTree(null);
    setHistory([]);
    setHistoryNextOffset(null);
    setCommitDetail(null);
    setFileDiff(null);
    setInitializationPreview(null);
    setIgnorePreview(null);
    setSaveResult(null);
    setGitOutput([]);
  };

  const reloadRepositoryData = async (projectPath = project?.path, expectedGeneration?: number) => {
    if (!projectPath) return;
    const [state, snapshot, historyPage, tree] = await Promise.all([
      readRepositoryState(projectPath),
      readWorktreeSnapshot(projectPath),
      readHistory(projectPath, 0),
      fileViewMode === "ALL" ? readRepositoryTree(projectPath) : Promise.resolve(null),
    ]);
    if (expectedGeneration !== undefined && expectedGeneration !== selectionGeneration.current) return;
    setRepositoryState(state);
    setWorktree(snapshot);
    setHistory(historyPage.entries);
    setHistoryNextOffset(historyPage.nextOffset);
    if (tree) setRepositoryTree(tree);
  };

  const selectProject = async (path: string, replacedPath?: string, targetSection: RepositorySection = "WORK") => {
    if (!settings) return;
    const generation = ++selectionGeneration.current;
    await run("project を開く", async () => {
      const result = await inspectProject(path);
      if (generation !== selectionGeneration.current) return;
      clearRepositoryData();
      setProject(result);
      const existing = settings.settings.recentProjects.find((item) => item.path === result.path)
        ?? settings.settings.recentProjects.find((item) => item.path === replacedPath);
      const updatedProject = { path: result.path, lastOpenedAt: new Date().toISOString(), tags: existing?.tags ?? [] };
      const nextSettings = {
        ...settings.settings,
        recentProjects: [updatedProject, ...settings.settings.recentProjects.filter((item) => item.path !== result.path && item.path !== replacedPath)],
      };
      await saveSettings(nextSettings);
      if (generation !== selectionGeneration.current) return;
      setSettings({
        ...settings,
        settings: nextSettings,
        recentProjects: [{ ...updatedProject, exists: true, projectKind: result.projectKind }, ...settings.recentProjects.filter((item) => item.path !== result.path && item.path !== replacedPath)],
      });
      setRoute({ page: "REPOSITORY", section: targetSection });
      if (result.repository.detected) await reloadRepositoryData(result.path, generation);
    });
  };

  const chooseProject = async () => {
    if (!settings) return;
    try {
      const selected = await open({ directory: true, multiple: false, title: "Vsedi project folder を選択" });
      if (typeof selected === "string") await selectProject(selected);
    } catch (caught) {
      setError(normalizeError(caught));
    }
  };

  const reassignManagedProject = async (path: string) => {
    if (!settings) return;
    try {
      const selected = await open({ directory: true, multiple: false, title: "移動後のproject folderを選択" });
      if (typeof selected === "string") await selectProject(selected, path);
    } catch (caught) {
      setError(normalizeError(caught));
    }
  };

  const removeManagedProject = async (path: string) => {
    if (!settings) return;
    const nextSettings = {
      ...settings.settings,
      recentProjects: settings.settings.recentProjects.filter((item) => item.path !== path),
    };
    await run("管理Projectを削除", async () => {
      await saveSettings(nextSettings);
      setSettings((current) => current ? { ...current, settings: nextSettings, recentProjects: current.recentProjects.filter((item) => item.path !== path) } : current);
    });
  };

  const updateSettings = async (nextSettings: SettingsLoadResult["settings"]) => {
    await run("設定を保存", async () => {
      await saveSettings(nextSettings);
      setSettings((current) => current ? { ...current, settings: nextSettings } : current);
      if (project) {
        const refreshed = await inspectProject(project.path);
        setProject(refreshed);
      }
    });
  };

  const updateVpmTrackingPolicy = async (policy: VpmTrackingPolicy) => {
    if (!settings || settings.settings.vpmTrackingPolicy === policy) return;
    await updateSettings({ ...settings.settings, vpmTrackingPolicy: policy });
  };

  const updateRepositoryVpmTrackingPolicy = async (policy: VpmTrackingPolicy | null) => {
    if (!settings || !project?.repository.root) return;
    const repositoryRoot = project.repository.root;
    const current = settings.settings.repositorySettings.find((item) => item.repositoryRoot === repositoryRoot);
    if ((current?.vpmTrackingPolicyOverride ?? null) === policy) return;
    const nextRepositorySettings = settings.settings.repositorySettings
      .filter((item) => item.repositoryRoot !== repositoryRoot);
    if (policy) {
      nextRepositorySettings.push({ repositoryRoot, vpmTrackingPolicyOverride: policy });
    }
    await updateSettings({ ...settings.settings, repositorySettings: nextRepositorySettings });
  };

  const updateLogLevel = async (logLevel: string) => {
    if (!settings || settings.settings.logLevel === logLevel) return;
    await updateSettings({ ...settings.settings, logLevel });
  };

  const updateIgnoreTemplates = async (ignoreTemplates: IgnoreTemplateSettings) => {
    if (!settings) return;
    await updateSettings({ ...settings.settings, ignoreTemplates });
  };

  const updateProjectTags = async (path: string, tags: string[]) => {
    if (!settings) return;
    const normalizedTags = normalizeTags(tags);
    const updatedAt = new Date().toISOString();
    const nextSettings = {
      ...settings.settings,
      recentProjects: settings.settings.recentProjects.map((item) => item.path === path ? { ...item, tags: normalizedTags, lastOpenedAt: updatedAt } : item),
    };
    await run("タグを保存", async () => {
      await saveSettings(nextSettings);
      setSettings({
        ...settings,
        settings: nextSettings,
        recentProjects: settings.recentProjects
          .map((item) => item.path === path ? { ...item, tags: normalizedTags, lastOpenedAt: updatedAt } : item)
          .sort(compareManagedProjects),
      });
    });
  };

  const previewInitialization = async () => {
    if (!project) return;
    await run("初期化内容を確認", async () => {
      setInitializationPreview(await previewRepositoryInitialization(project.path));
    });
  };

  const previewIgnore = async () => {
    if (!project?.repository.detected) return;
    await run("ignore ruleを確認", async () => {
      setIgnorePreview(await previewIgnoreRules(project.path));
    });
  };

  const applyIgnore = async () => {
    if (!project || !ignorePreview) return;
    await run("ignore ruleを適用", async () => {
      await applyIgnoreRules({ projectPath: project.path, statusToken: ignorePreview.statusToken });
      setIgnorePreview(null);
      const refreshed = await inspectProject(project.path);
      setProject(refreshed);
      await reloadRepositoryData(project.path);
    });
  };

  const applyInitialization = async () => {
    if (!project || !settings || !initializationPreview) return;
    await run("repository を初期化", async () => {
      await initializeRepository({ projectPath: project.path, statusToken: initializationPreview.statusToken });
      setInitializationPreview(null);
      const refreshed = await inspectProject(project.path);
      setProject(refreshed);
      await reloadRepositoryData(refreshed.path);
    });
  };

  const saveCurrentWork = async (memo: string) => {
    if (!project || !worktree) return;
    await run("作業を保存", async () => {
      setSaveResult(null);
      setGitOutput([]);
      const latestWorktree = await readWorktreeSnapshot(project.path);
      if (latestWorktree.statusToken !== worktree.statusToken) {
        setWorktree(latestWorktree);
        setSaveResult(null);
        const staleStateError: AppError = {
          code: "REPOSITORY_STATE_CHANGED",
          message: "保存前に変更内容が変わったため、保存を停止しました。最新の変更を確認してから、もう一度保存してください。",
          technicalDetail: null,
          operation: "save_worktree",
          mayHaveMutated: false,
        };
        throw staleStateError;
      }
      const result = await saveWorktree({ projectPath: project.path, statusToken: worktree.statusToken, memo });
      setSaveResult(result);
      setCommitDetail(null);
      setFileDiff(null);
      await reloadRepositoryData(project.path);
    });
  };

  const refreshCurrentWork = async () => {
    if (!project) return;
    await run("変更を再読込", async () => {
      setSaveResult(null);
      await reloadRepositoryData(project.path);
    });
  };

  const changeFileViewMode = async (mode: FileViewMode) => {
    if (mode === fileViewMode) return;
    if (mode === "CHANGES") {
      setFileViewMode(mode);
      return;
    }
    if (!project) return;
    await run("フォルダ内全体を読み込み", async () => {
      const tree = await readRepositoryTree(project.path);
      setRepositoryTree(tree);
      setFileViewMode(mode);
    });
  };

  const selectCommit = async (entry: HistoryEntry) => {
    if (!project) return;
    await run("保存詳細を読み込む", async () => {
      setFileDiff(null);
      setCommitDetail(await readCommitDetail(project.path, entry.commitId));
    });
  };

  const loadMoreHistory = async () => {
    if (!project || historyNextOffset === null) return;
    const offset = historyNextOffset;
    await run("保存履歴を追加読み込み", async () => {
      const historyPage = await readHistory(project.path, offset);
      setHistory((current) => {
        const knownCommitIds = new Set(current.map((entry) => entry.commitId));
        return [...current, ...historyPage.entries.filter((entry) => !knownCommitIds.has(entry.commitId))];
      });
      setHistoryNextOffset(historyPage.nextOffset);
    });
  };

  const showCommitDiff = async (path: string) => {
    if (!project || !commitDetail) return;
    await run("差分を読み込む", async () => {
      setFileDiff(await readCommitDiff(project.path, commitDetail.commitId, path));
    });
  };

  const showWorktreeDiff = async (path: string) => {
    if (!project) return;
    await run("差分を読み込む", async () => {
      setFileDiff(await readWorktreeDiff(project.path, path));
    });
  };

  const exportLog = async () => {
    try {
      const destination = await save({ defaultPath: "vsedi-diagnostic.log", title: "診断ログを書き出す" });
      if (destination) await run("診断ログを書き出す", async () => { await exportDiagnosticLog(destination); });
    } catch (caught) {
      setError(normalizeError(caught));
    }
  };

  const openLogs = async () => {
    await run("ログを開く", async () => { await openLogWindow(); });
  };

  const openLogFolder = async () => {
    await run("ログフォルダを開く", async () => { await openLogDirectory(); });
  };

  const navigateRepository = (section: RepositorySection) => {
    setFileDiff(null);
    if (section !== "HISTORY") setCommitDetail(null);
    setRoute({ page: "REPOSITORY", section });
  };

  const pageTitle = route.page === "HOME"
    ? "ホーム"
    : route.page === "GLOBAL_SETTINGS"
      ? "全体設定"
      : route.section === "WORK"
        ? "現在の作業"
        : route.section === "HISTORY" ? "保存履歴" : "リポジトリ設定";

  return (
    <main className="h-screen overflow-hidden bg-mist text-ink">
      <div className="mx-auto flex h-full max-w-[1440px]">
        <AppSidebar
          route={route}
          project={project}
          onHome={() => setRoute({ page: "HOME" })}
          onRepository={navigateRepository}
          onGlobalSettings={(section) => setRoute({ page: "GLOBAL_SETTINGS", section })}
        />
        <div className="min-h-0 min-w-0 flex-1 overflow-y-auto px-5 py-6 sm:px-8" data-app-content="true">
          {(route.page !== "REPOSITORY" || !project) && (
            <AppHeader
              pageTitle={pageTitle}
              project={route.page === "REPOSITORY" ? project : null}
              repositoryState={repositoryState}
              pending={pending}
              onAddProject={route.page === "HOME" ? () => void chooseProject() : undefined}
              onRefresh={() => void refreshApplication()}
            />
          )}

          {error && <ErrorNotice error={error} onDismiss={() => setError(null)} />}
          {settings?.recovered && <RecoveryNotice settings={settings} />}

          {route.page === "HOME" && (
            <HomePage
              environment={environment}
              settings={settings}
              busy={isBusy}
              onOpenProject={(path) => void selectProject(path)}
              onOpenRepositorySettings={(path) => void selectProject(path, undefined, "SETTINGS")}
              onReassignProject={(path) => void reassignManagedProject(path)}
              onRemoveProject={(path) => void removeManagedProject(path)}
              onOpenSettings={() => setRoute({ page: "GLOBAL_SETTINGS", section: "ENVIRONMENT" })}
            />
          )}

          {route.page === "GLOBAL_SETTINGS" && (
            <GlobalSettingsPage
              section={route.section}
              environment={environment}
              settings={settings}
              busy={isBusy}
              onChangeSection={(section) => setRoute({ page: "GLOBAL_SETTINGS", section })}
              onUpdateVpm={(policy) => void updateVpmTrackingPolicy(policy)}
              onUpdateLogLevel={(level) => void updateLogLevel(level)}
              onOpenLogs={() => void openLogs()}
              onOpenLogFolder={() => void openLogFolder()}
              onExportLog={() => void exportLog()}
              onUpdateIgnoreTemplates={(templates) => void updateIgnoreTemplates(templates)}
            />
          )}

          {route.page === "REPOSITORY" && project && route.section === "WORK" && (
            <WorkPage
              project={project}
              repositoryState={repositoryState}
              worktree={worktree}
              repositoryTree={repositoryTree}
              fileViewMode={fileViewMode}
              initializationPreview={initializationPreview}
              saveResult={saveResult}
              gitOutput={gitOutput}
              saving={pending === "作業を保存"}
              busy={isBusy}
              onRefresh={() => void refreshCurrentWork()}
              onChangeFileView={(mode) => void changeFileViewMode(mode)}
              onPreviewInitialization={() => void previewInitialization()}
              onApplyInitialization={() => void applyInitialization()}
              onCancelInitialization={() => setInitializationPreview(null)}
              onSave={(memo) => void saveCurrentWork(memo)}
              onShowDiff={(path) => void showWorktreeDiff(path)}
              fileDiff={fileDiff}
              onGoToRepositorySettings={() => navigateRepository("SETTINGS")}
            />
          )}

          {route.page === "REPOSITORY" && project && route.section === "HISTORY" && (
            <HistoryPage
              history={history}
              historyNextOffset={historyNextOffset}
              commitDetail={commitDetail}
              fileDiff={fileDiff}
              busy={isBusy}
              onSelectCommit={(entry) => void selectCommit(entry)}
              onLoadMore={() => void loadMoreHistory()}
              onShowDiff={(path) => void showCommitDiff(path)}
            />
          )}

          {route.page === "REPOSITORY" && project && route.section === "SETTINGS" && (
            <RepositorySettingsPage
              project={project}
              settings={settings}
              initializationPreview={initializationPreview}
              busy={isBusy}
              onPreviewInitialization={() => void previewInitialization()}
              onApplyInitialization={() => void applyInitialization()}
              onCancelInitialization={() => setInitializationPreview(null)}
              onOpenGlobalDefaults={() => setRoute({ page: "GLOBAL_SETTINGS", section: "DEFAULTS" })}
              onUpdateTags={(tags) => void updateProjectTags(project.path, tags)}
              onUpdateVpm={(policy) => void updateRepositoryVpmTrackingPolicy(policy)}
              ignorePreview={ignorePreview}
              onPreviewIgnore={() => void previewIgnore()}
              onApplyIgnore={() => void applyIgnore()}
            />
          )}
        </div>
      </div>
    </main>
  );
}

function AppSidebar({ route, project, onHome, onRepository, onGlobalSettings }: {
  route: AppRoute;
  project: ProjectDiagnostic | null;
  onHome: () => void;
  onRepository: (section: RepositorySection) => void;
  onGlobalSettings: (section: GlobalSettingsSection) => void;
}) {
  const repositoryOpen = route.page === "REPOSITORY";
  return (
    <aside className="hidden h-full w-64 shrink-0 overflow-y-auto border-r border-slate-200 bg-white px-3 py-6 lg:block">
      <div className="px-3 pb-7"><p className="text-xs font-bold uppercase tracking-[0.28em] text-accent">Local first</p><h1 className="mt-2 text-2xl font-bold tracking-tight">Vsedi</h1></div>
      <nav className="space-y-1" aria-label="メインナビゲーション">
        <NavigationButton active={route.page === "HOME"} onClick={onHome}>ホーム</NavigationButton>
        {repositoryOpen && project && (
          <div className="mt-6 rounded-2xl border border-slate-300 bg-slate-50 p-2" role="group" aria-label="選択中のProject">
            <p className="px-3 pt-2 text-[11px] font-bold uppercase tracking-[0.14em] text-slate-400">選択中の project</p>
            <p className="truncate px-3 py-2 text-sm font-semibold text-slate-700" title={displayPath(project.path)}>{projectName(project.path)}</p>
            <NavigationButton active={route.section === "WORK"} onClick={() => onRepository("WORK")}>現在の作業</NavigationButton>
            <NavigationButton active={route.section === "HISTORY"} onClick={() => onRepository("HISTORY")}>保存履歴</NavigationButton>
            <NavigationButton active={route.section === "SETTINGS"} onClick={() => onRepository("SETTINGS")}>リポジトリ設定</NavigationButton>
          </div>
        )}
        <p className="px-3 pt-6 text-[11px] font-bold uppercase tracking-[0.14em] text-slate-400">アプリ</p>
        <NavigationButton active={route.page === "GLOBAL_SETTINGS"} onClick={() => onGlobalSettings("GENERAL")}>全体設定</NavigationButton>
      </nav>
    </aside>
  );
}

function NavigationButton({ active, children, onClick }: { active: boolean; children: string; onClick: () => void }) {
  return <button type="button" onClick={onClick} className={`flex w-full rounded-xl px-3 py-2 text-left text-sm font-semibold transition ${active ? "bg-slate-900 text-white" : "text-slate-600 hover:bg-slate-100 hover:text-slate-900"}`}>{children}</button>;
}

function AppHeader({ pageTitle, project, repositoryState, pending, onAddProject, onRefresh }: { pageTitle: string; project: ProjectDiagnostic | null; repositoryState: RepositoryState | null; pending: string | null; onAddProject?: () => void; onRefresh: () => void }) {
  return (
    <header className="mb-6 flex flex-wrap items-start justify-between gap-4 border-b border-slate-200 pb-5">
      <div><p className="text-xs font-bold uppercase tracking-[0.18em] text-slate-400">Vsedi</p><h2 className="mt-1 text-2xl font-bold tracking-tight">{pageTitle}</h2>{project && <p className="mt-1 break-all text-sm text-slate-500">{displayPath(project.path)}{repositoryState?.root && repositoryState.root !== project.path ? ` · 保存対象: ${displayPath(repositoryState.root)}` : ""}</p>}</div>
      <div className="flex items-center gap-2">{pending && <span className="text-xs text-slate-500">{pending}…</span>}{onAddProject && <Button variant="secondary" onClick={onAddProject} disabled={Boolean(pending)}>Projectを追加</Button>}<Button variant="ghost" onClick={onRefresh} disabled={Boolean(pending)}>再読込</Button></div>
    </header>
  );
}

function GearIcon() {
  return <svg aria-hidden="true" viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.8"><path strokeLinecap="round" strokeLinejoin="round" d="M10.3 2.8h3.4l.5 2.1c.5.2 1 .4 1.5.8l2-.8 1.7 3-1.6 1.4c.1.5.1 1.1 0 1.7l1.6 1.4-1.7 3-2-.8c-.5.3-1 .6-1.5.8l-.5 2.1h-3.4l-.5-2.1a7 7 0 0 1-1.5-.8l-2 .8-1.7-3 1.6-1.4a7 7 0 0 1 0-1.7L4.6 7.9l1.7-3 2 .8c.5-.3 1-.6 1.5-.8l.5-2.1Z" /><circle cx="12" cy="10.2" r="2.6" /></svg>;
}

function SearchIcon() {
  return <svg aria-hidden="true" viewBox="0 0 24 24" className="h-5 w-5" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="10.8" cy="10.8" r="6.8" /><path strokeLinecap="round" d="m16 16 5 5" /></svg>;
}

function ClearIcon() {
  return <svg aria-hidden="true" viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="2"><path strokeLinecap="round" d="m6 6 12 12M18 6 6 18" /></svg>;
}

function ChevronIcon({ direction }: { direction: "up" | "down" }) {
  return <svg aria-hidden="true" viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="2"><path strokeLinecap="round" strokeLinejoin="round" d={direction === "up" ? "m6 14 6-6 6 6" : "m6 10 6 6 6-6"} /></svg>;
}

function ProjectTypeIcon({ kind }: { kind: ProjectKind | null }) {
  const label = kind === "VRCHAT_AVATAR" ? "アバター" : kind === "VRCHAT_WORLD" ? "ワールド" : kind === "VRCHAT_UNKNOWN" ? "VRChat project" : "Unity project";
  return <span className="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-slate-100 text-slate-500" title={label} aria-label={label} role="img">{kind === "VRCHAT_AVATAR" ? <AvatarIcon /> : kind === "VRCHAT_WORLD" ? <GlobeIcon /> : <FolderIcon />}</span>;
}

function AvatarIcon() {
  return <svg aria-hidden="true" viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.8"><circle cx="12" cy="8" r="3" /><path strokeLinecap="round" strokeLinejoin="round" d="M5.5 20c.7-3.5 2.8-5.3 6.5-5.3s5.8 1.8 6.5 5.3" /></svg>;
}

function GlobeIcon() {
  return <svg aria-hidden="true" viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.8"><circle cx="12" cy="12" r="8.5" /><path strokeLinecap="round" d="M3.8 12h16.4M12 3.5c2.1 2.3 3.2 5.1 3.2 8.5s-1.1 6.2-3.2 8.5c-2.1-2.3-3.2-5.1-3.2-8.5S9.9 5.8 12 3.5Z" /></svg>;
}

function FolderIcon() {
  return <svg aria-hidden="true" viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.8"><path strokeLinecap="round" strokeLinejoin="round" d="M3.5 6.5h6l1.7 2h9.3v9.8a1.7 1.7 0 0 1-1.7 1.7H5.2a1.7 1.7 0 0 1-1.7-1.7V6.5Z" /></svg>;
}

function HomePage({ environment, settings, busy, onOpenProject, onOpenRepositorySettings, onReassignProject, onRemoveProject, onOpenSettings }: { environment: EnvironmentDiagnostic | null; settings: SettingsLoadResult | null; busy: boolean; onOpenProject: (path: string) => void; onOpenRepositorySettings: (path: string) => void; onReassignProject: (path: string) => void; onRemoveProject: (path: string) => void; onOpenSettings: () => void }) {
  const gitAvailable = environment?.git.status === "AVAILABLE";
  const [introCollapsed, setIntroCollapsed] = useState(false);
  return <div className="space-y-6">
    <section className={`relative rounded-3xl bg-slate-900 text-white shadow-panel ${introCollapsed ? "px-6 py-4 sm:px-8" : "px-6 py-7 sm:px-8"}`}><button type="button" className="absolute right-4 top-4 rounded-lg p-2 text-slate-300 transition hover:bg-white/10 hover:text-white" onClick={() => setIntroCollapsed((current) => !current)} aria-expanded={!introCollapsed} aria-label={introCollapsed ? "お知らせを展開" : "お知らせを最小化"} title={introCollapsed ? "お知らせを展開" : "お知らせを最小化"}><ChevronIcon direction={introCollapsed ? "down" : "up"} /></button><div className="pr-8"><p className="text-xs font-bold uppercase tracking-[0.2em] text-sky-200">制作のセーブポイント</p><h3 className={`${introCollapsed ? "mt-1 text-lg" : "mt-3 text-3xl"} font-bold tracking-tight`}>管理する project を選択</h3>{!introCollapsed && <p className="mt-3 max-w-2xl text-sm leading-6 text-slate-300">project を選ぶと、Unity / VRChat / Git の状態を確認して、この repository の作業画面を開きます。</p>}</div></section>
    {!gitAvailable && environment && <Card className="border-amber-200 bg-amber-50"><CardContent className="flex flex-wrap items-center justify-between gap-3"><div><p className="font-semibold text-amber-900">System Git を確認してください</p><p className="mt-1 text-sm text-amber-800">Git が利用できないため、作業を保存できません。</p></div><Button variant="secondary" onClick={onOpenSettings}>実行環境を開く</Button></CardContent></Card>}
    {settings ? <ManagedProjectList projects={settings.recentProjects} busy={busy} onOpenProject={onOpenProject} onOpenRepositorySettings={onOpenRepositorySettings} onReassignProject={onReassignProject} onRemoveProject={onRemoveProject} /> : <Card><CardContent><p className="py-6 text-center text-sm text-slate-500">管理Projectを読み込んでいます…</p></CardContent></Card>}
  </div>;
}

function ManagedProjectList({ projects, busy, onOpenProject, onOpenRepositorySettings, onReassignProject, onRemoveProject }: { projects: SettingsLoadResult["recentProjects"]; busy: boolean; onOpenProject: (path: string) => void; onOpenRepositorySettings: (path: string) => void; onReassignProject: (path: string) => void; onRemoveProject: (path: string) => void }) {
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [searchOpen, setSearchOpen] = useState(false);
  const tags = [...new Set(projects.flatMap((item) => item.tags))].sort((left, right) => left.localeCompare(right, "ja"));
  const tagKey = tags.join("\u0000");
  const sortedProjects = [...projects].sort(compareManagedProjects);
  const normalizedQuery = searchQuery.trim().toLocaleLowerCase("ja-JP");
  const visibleProjects = sortedProjects.filter((item) => {
    const haystack = [projectName(item.path), item.path, ...item.tags].join("\n").toLocaleLowerCase("ja-JP");
    const matchesSearch = !normalizedQuery || haystack.includes(normalizedQuery);
    const matchesTags = selectedTags.length === 0 || selectedTags.some((tag) => item.tags.includes(tag));
    return matchesSearch && matchesTags;
  });

  useEffect(() => {
    setSelectedTags((current) => current.filter((tag) => tags.includes(tag)));
  }, [tagKey]);

  const toggleTag = (tag: string) => {
    setSelectedTags((current) => current.includes(tag) ? current.filter((item) => item !== tag) : [...current, tag]);
  };

  const hasFilters = Boolean(searchQuery.trim()) || selectedTags.length > 0;
  return <section><div className="mb-3 flex flex-wrap items-center justify-between gap-4"><div className="flex min-w-0 items-center gap-2"><h3 className="text-lg font-bold">管理しているProject</h3><button type="button" className={`relative rounded-lg p-1.5 text-slate-500 transition hover:bg-slate-100 hover:text-slate-900 ${searchOpen ? "bg-slate-100 text-slate-900" : ""}`} onClick={() => setSearchOpen((current) => !current)} aria-expanded={searchOpen} aria-controls="managed-project-search" aria-label={searchOpen ? "Project検索を閉じる" : "Projectを検索"} title={searchOpen ? "検索欄を閉じる" : "検索欄を表示"}><SearchIcon />{hasFilters && !searchOpen && <span className="absolute right-1 top-1 h-1.5 w-1.5 rounded-full bg-accent" />}</button><p className="hidden text-sm text-slate-500 sm:block">最終更新が新しい順に表示します。</p></div><span className="text-xs text-slate-400">{visibleProjects.length} / {projects.length} 件</span></div>{searchOpen && <div id="managed-project-search" className="mb-4 space-y-3 rounded-2xl border border-slate-200 bg-white p-4"><label className="block text-xs font-semibold text-slate-500" htmlFor="project-search">Projectを検索</label><div className="relative"><input id="project-search" className="w-full rounded-xl border border-slate-300 px-3 py-2 pr-10 text-sm" value={searchQuery} onChange={(event) => setSearchQuery(event.target.value)} placeholder="Project名、パス、タグを入力" disabled={busy} />{searchQuery && <button type="button" className="absolute right-2 top-1/2 -translate-y-1/2 rounded-lg p-1.5 text-slate-400 transition hover:bg-slate-100 hover:text-slate-700 disabled:opacity-50" onClick={() => setSearchQuery("")} disabled={busy} aria-label="検索文字をクリア" title="検索文字をクリア"><ClearIcon /></button>}</div>{tags.length > 0 && <div className="flex flex-wrap items-center gap-2"><span className="text-xs font-semibold text-slate-500">タグ</span>{tags.map((tag) => <button type="button" key={tag} onClick={() => toggleTag(tag)} className={`rounded-full px-3 py-1 text-xs font-semibold transition ${selectedTags.includes(tag) ? "bg-slate-900 text-white" : "bg-slate-100 text-slate-600 hover:bg-slate-200"}`} aria-pressed={selectedTags.includes(tag)}>{tag}</button>)}{selectedTags.length > 0 && <button type="button" className="ml-1 text-xs font-semibold text-slate-500 underline" onClick={() => setSelectedTags([])}>タグ絞り込みを解除</button>}</div>}</div>}{visibleProjects.length ? <div className="space-y-3">{visibleProjects.map((item) => <Card key={item.path}><CardContent><div className="flex flex-wrap items-start justify-between gap-4"><div className="min-w-0 flex-1"><div className="flex flex-wrap items-center gap-2"><ProjectTypeIcon kind={item.projectKind} /><p className="truncate font-semibold text-slate-800" title={displayPath(item.path)}>{projectName(item.path)}</p><ProjectTagBadges tags={item.tags} />{!item.exists && <StatusPill label="再指定" tone="warn" />}</div><p className="mt-1 truncate text-xs text-slate-500" title={displayPath(item.path)}>{displayPath(item.path)}</p><p className="mt-3 text-xs text-slate-400">{item.lastOpenedAt ? `最終更新: ${formatDate(item.lastOpenedAt)}` : "更新日時は未記録"}</p></div><div className="flex flex-wrap justify-end gap-2">{item.exists && <Button variant="secondary" className="h-10 w-10 p-0" onClick={() => onOpenRepositorySettings(item.path)} disabled={busy} aria-label={`${projectName(item.path)}のリポジトリ設定を開く`} title="リポジトリ設定を開く"><GearIcon /></Button>}{item.exists ? <Button onClick={() => onOpenProject(item.path)} disabled={busy}>開く</Button> : <Button onClick={() => onReassignProject(item.path)} disabled={busy}>場所を再指定</Button>}<Button variant="ghost" onClick={() => onRemoveProject(item.path)} disabled={busy}>一覧から削除</Button></div></div></CardContent></Card>)}</div> : <Card><CardContent><p className="py-6 text-center text-sm text-slate-500">{projects.length ? "検索条件に一致するProjectはありません。" : "まだProjectは登録されていません。"}</p></CardContent></Card>}</section>;
}

function WorkPage({ project, repositoryState, worktree, repositoryTree, fileViewMode, initializationPreview, saveResult, gitOutput, saving, busy, onRefresh, onChangeFileView, onPreviewInitialization, onApplyInitialization, onCancelInitialization, onSave, onShowDiff, fileDiff, onGoToRepositorySettings }: {
  project: ProjectDiagnostic; repositoryState: RepositoryState | null; worktree: WorktreeSnapshot | null; repositoryTree: RepositoryTreeSnapshot | null; fileViewMode: FileViewMode; initializationPreview: RepositoryInitializationPreview | null; saveResult: SaveResult | null; gitOutput: GitCommandEvent[]; saving: boolean; busy: boolean; onRefresh: () => void; onChangeFileView: (mode: FileViewMode) => void; onPreviewInitialization: () => void; onApplyInitialization: () => void; onCancelInitialization: () => void; onSave: (memo: string) => void; onShowDiff: (path: string) => void; fileDiff: FileDiff | null; onGoToRepositorySettings: () => void;
}) {
  const [memo, setMemo] = useState("");
  useEffect(() => { setMemo(""); }, [project.path]);
  if (!project.repository.detected) return <RepositorySetup project={project} preview={initializationPreview} busy={busy} onPreview={onPreviewInitialization} onApply={onApplyInitialization} onCancel={onCancelInitialization} onGoToSettings={onGoToRepositorySettings} />;
  const saveStatus = saveStatusPresentation(repositoryState, worktree);
  const displayFiles = fileViewMode === "ALL" ? repositoryTree?.files ?? [] : worktree?.files ?? [];
  return <div className="space-y-5">
    <WorkSummary project={project} repositoryState={repositoryState} worktree={worktree} />
    <Card><CardHeader><div className="flex flex-wrap items-start justify-between gap-3"><div><h3 className="font-semibold">現在の変更</h3><p className="mt-1 text-xs text-slate-500">保存対象はrepository全体です。project外の変更もここに表示します。</p></div><div className="flex flex-wrap items-center justify-end gap-2"><StatusPill label={saveStatus.label} tone={saveStatus.tone} /><div className="flex rounded-xl border border-slate-200 bg-slate-50 p-1" role="group" aria-label="変更ファイルの表示範囲"><Button className="px-3 py-1.5 text-xs" variant={fileViewMode === "CHANGES" ? "primary" : "ghost"} onClick={() => onChangeFileView("CHANGES")} disabled={busy} aria-pressed={fileViewMode === "CHANGES"}>変更のみを表示</Button><Button className="px-3 py-1.5 text-xs" variant={fileViewMode === "ALL" ? "primary" : "ghost"} onClick={() => onChangeFileView("ALL")} disabled={busy} aria-pressed={fileViewMode === "ALL"}>フォルダ内全体を表示</Button></div><Button variant="secondary" onClick={onRefresh} disabled={busy} aria-label="現在の変更を再読込">変更を再読込</Button></div></div></CardHeader><CardContent>
      {repositoryState?.blockingReason === "EXISTING_STAGED_CHANGES" && <BlockingNotice>すでにGitのステージにある変更があるため、安全のため保存を開始できません。</BlockingNotice>}
      {repositoryState?.blockingReason === "CONFLICT" && <BlockingNotice tone="danger">競合中のファイルがあるため、保存を開始できません。</BlockingNotice>}
      <ChangedFiles files={displayFiles} viewMode={fileViewMode} fileContext="WORKTREE" onSelect={onShowDiff} />
      {repositoryState?.canSave && worktree?.files.length ? <div className="mt-4 flex flex-wrap gap-2 border-t border-slate-100 pt-4"><div className="relative min-w-56 flex-1"><input className="w-full rounded-xl border border-slate-300 px-3 py-2 pr-10 text-sm" value={memo} onChange={(event) => setMemo(event.target.value)} placeholder="保存メモ（例: アバターの表情を調整）" disabled={busy} />{memo && <button type="button" className="absolute right-2 top-1/2 -translate-y-1/2 rounded-lg p-1.5 text-slate-400 transition hover:bg-slate-100 hover:text-slate-700 disabled:opacity-50" onClick={() => setMemo("")} disabled={busy} aria-label="保存メモをクリア" title="保存メモをクリア"><ClearIcon /></button>}</div><Button variant="danger" onClick={() => { onSave(memo); setMemo(""); }} disabled={busy || !memo.trim()}>作業を保存</Button></div> : null}
      {(saving || gitOutput.length > 0) && <GitSaveProgress events={gitOutput} active={saving} />}
      {saveResult && <p className="mt-4 rounded-xl bg-emerald-50 px-3 py-2 text-xs text-emerald-800">保存しました: {saveResult.shortCommitId} · {saveResult.fileCount} file · {saveResult.authorTime}</p>}
    </CardContent></Card>
    {fileDiff && <DiffPanel diff={fileDiff} />}
    <DiagnosticSummary project={project} onGoToSettings={onGoToRepositorySettings} />
  </div>;
}

function GitSaveProgress({ events, active }: { events: GitCommandEvent[]; active: boolean }) {
  return <Card className="mt-4 border-slate-300"><CardHeader><div className="flex flex-wrap items-center justify-between gap-3"><div><h3 className="font-semibold">{active ? "Gitで保存中" : "Git CLIの実行結果"}</h3><p className="mt-1 text-xs text-slate-500">{active ? "変更をステージしてcommitを作成しています。処理が終わるまでお待ちください。" : "保存時に実行したGit CLIのレスポンスです。"}</p></div><StatusPill label={active ? "処理中" : "完了"} tone={active ? "warn" : "good"} /></div></CardHeader><CardContent><div role="log" aria-live="polite" className="max-h-72 overflow-auto rounded-xl bg-slate-950 px-4 py-3 font-mono text-xs leading-5 text-slate-100">{events.length ? events.map((event, index) => <div key={`${event.phase}-${index}`} className={event.phase === "OUTPUT" && event.stream === "STDERR" ? "text-amber-300" : event.phase === "COMPLETED" ? "text-slate-400" : ""}>{event.phase === "STARTED" && <><span className="text-sky-300">$ </span>{formatGitCommand(event)}</>}{event.phase === "OUTPUT" && <><span className="mr-2 text-slate-500">[{event.stream === "STDERR" ? "stderr" : "stdout"}]</span><span className="whitespace-pre-wrap">{event.text}</span></>}{event.phase === "COMPLETED" && <>exit {event.status ?? "unknown"}</>}</div>) : <p className="text-slate-400">Git CLIを起動しています…</p>}</div></CardContent></Card>;
}

function RepositorySetup({ project, preview, busy, onPreview, onApply, onCancel, onGoToSettings }: { project: ProjectDiagnostic; preview: RepositoryInitializationPreview | null; busy: boolean; onPreview: () => void; onApply: () => void; onCancel: () => void; onGoToSettings?: () => void }) {
  return <div className="space-y-5"><Card className="border-sky-200 bg-sky-50"><CardHeader><h3 className="font-semibold text-sky-950">ローカル保存を準備する</h3></CardHeader><CardContent><p className="text-sm leading-6 text-sky-900">このUnity projectにはまだGit repositoryがありません。作成内容を確認してから、Unity用のignore ruleとともにローカル保存を始められます。</p>{!preview ? <Button className="mt-4" onClick={onPreview} disabled={busy}>作成内容を確認</Button> : <div className="mt-4 space-y-3 rounded-xl bg-white/80 p-4">{preview.ignoreFiles.map((file) => <div key={file.path}><p className="text-sm font-semibold text-slate-800">{file.path}{file.willCreate ? "（新規作成）" : ""}</p><p className="mt-1 text-xs text-slate-600">{file.missingRules.length ? `${file.missingRules.length} 件のruleを追加します。` : "変更はありません。"}</p></div>)}{preview.canInitialize ? <div className="flex gap-2"><Button onClick={onApply} disabled={busy}>この内容で初期化</Button><Button variant="secondary" onClick={onCancel} disabled={busy}>キャンセル</Button></div> : <p className="text-sm text-rose-800">{preview.blockingReason}</p>}</div>}</CardContent></Card>{onGoToSettings && <DiagnosticSummary project={project} onGoToSettings={onGoToSettings} />}</div>;
}

function HistoryPage({ history, historyNextOffset, commitDetail, fileDiff, busy, onSelectCommit, onLoadMore, onShowDiff }: { history: HistoryEntry[]; historyNextOffset: number | null; commitDetail: CommitDetail | null; fileDiff: FileDiff | null; busy: boolean; onSelectCommit: (entry: HistoryEntry) => void; onLoadMore: () => void; onShowDiff: (path: string) => void }) {
  return <div className="grid gap-5 xl:grid-cols-[0.8fr_1.2fr]">
    <Card>
      <CardHeader><h3 className="font-semibold">保存履歴</h3><p className="mt-1 text-xs text-slate-500">新しい履歴から20件ずつ表示します。過去の保存を選ぶと、変更内容を確認できます。</p></CardHeader>
      <CardContent>
        {history.length ? <>
          <div className="overflow-hidden rounded-xl border border-slate-200 bg-white">
            <div className="min-w-[30rem]">
              <div className="grid grid-cols-[minmax(14rem,1fr)_11rem_7rem] gap-3 border-b border-slate-200 bg-slate-50 px-3 py-2 text-xs font-semibold text-slate-500"><span>保存メモ</span><span>保存日時</span><span>commit</span></div>
              <div aria-label="保存履歴" role="list">
                {history.map((entry) => {
                  const selected = commitDetail?.commitId === entry.commitId;
                  return <div role="listitem" key={entry.commitId}><button type="button" onClick={() => onSelectCommit(entry)} disabled={busy} aria-pressed={selected} className={`grid w-full grid-cols-[minmax(14rem,1fr)_11rem_7rem] items-center gap-3 border-b border-slate-100 px-3 py-2 text-left text-sm transition last:border-b-0 ${selected ? "bg-slate-900 text-white" : "text-slate-700 hover:bg-slate-50"}`}><span className="truncate font-semibold" title={entry.memo}>{entry.memo}</span><span className={`truncate text-xs ${selected ? "text-slate-300" : "text-slate-500"}`} title={entry.authorTime}>{formatDate(entry.authorTime)}</span><span className={`font-mono text-xs ${selected ? "text-slate-300" : "text-slate-500"}`} title={entry.commitId}>{entry.shortCommitId}</span></button></div>;
                })}
              </div>
            </div>
          </div>
          {historyNextOffset !== null && <div className="mt-3 flex justify-center"><Button variant="secondary" onClick={onLoadMore} disabled={busy}>{busy ? "読み込み中…" : "さらに読み込む"}</Button></div>}
        </> : <p className="py-6 text-center text-sm text-slate-500">まだ保存履歴はありません。</p>}
      </CardContent>
    </Card>
    <div className="space-y-5">
      {commitDetail ? <Card><CardHeader><div className="flex flex-wrap items-start justify-between gap-3"><div><h3 className="font-semibold">保存の詳細</h3><p className="mt-1 truncate text-sm text-slate-700" title={commitDetail.memo}>{commitDetail.memo}</p><p className="mt-1 break-all text-xs text-slate-500">{commitDetail.commitId} · {formatDate(commitDetail.authorTime)}</p></div><StatusPill label="保存済み" tone="good" /></div></CardHeader><CardContent><ChangedFiles files={commitDetail.files} viewMode="CHANGES" fileContext="COMMIT" onSelect={onShowDiff} /><p className="mt-4 rounded-xl bg-slate-50 px-3 py-2 text-xs text-slate-500">安全な復元はM4でこの画面から開始します。履歴を選択しただけでは現在の作業は変わりません。</p></CardContent></Card> : <Card><CardContent><p className="py-8 text-center text-sm text-slate-500">左から保存を選択してください。</p></CardContent></Card>}
      {fileDiff && <DiffPanel diff={fileDiff} />}
    </div>
  </div>;
}

function ProjectTagEditor({ tags, busy, onSave }: { tags: string[]; busy: boolean; onSave: (tags: string[]) => void }) {
  const [draftTags, setDraftTags] = useState<string[]>(tags);
  const [input, setInput] = useState("");
  const tagKey = tags.join("\u0000");
  const hasUnsavedChanges = normalizeTags(draftTags).join("\u0000") !== normalizeTags(tags).join("\u0000");

  useEffect(() => {
    setDraftTags(tags);
    setInput("");
  }, [tagKey]);

  const addInputTags = () => {
    const additions = parseTags(input);
    if (!additions.length) return;
    setDraftTags((current) => normalizeTags([...current, ...additions]));
    setInput("");
  };

  return <div><p className="text-sm font-semibold">Projectタグ</p><p className="mt-1 text-sm text-slate-600">複数のタグを設定できます。タグはアプリ内の管理Project検索・絞り込みに使用します。</p><div className="mt-4 flex flex-wrap gap-2">{draftTags.length ? draftTags.map((tag) => <span key={tag} className="inline-flex items-center gap-1 rounded-full bg-sky-50 px-3 py-1 text-xs font-semibold text-sky-700">{tag}<button type="button" className="rounded-full px-1 text-sky-700 hover:bg-sky-100" onClick={() => setDraftTags((current) => current.filter((item) => item !== tag))} disabled={busy} aria-label={`${tag}タグを削除`}>×</button></span>) : <span className="text-sm text-slate-500">タグはまだ設定されていません。</span>}</div><div className="mt-4 flex flex-wrap gap-2"><div className="relative min-w-56 flex-1"><input className="w-full rounded-xl border border-slate-300 px-3 py-2 pr-10 text-sm" value={input} maxLength={80} onChange={(event) => setInput(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter" || event.key === ",") { event.preventDefault(); addInputTags(); } }} placeholder="例: Avatar、作業中" disabled={busy} />{input && <button type="button" className="absolute right-2 top-1/2 -translate-y-1/2 rounded-lg p-1.5 text-slate-400 transition hover:bg-slate-100 hover:text-slate-700 disabled:opacity-50" onClick={() => setInput("")} disabled={busy} aria-label="タグ入力をクリア" title="タグ入力をクリア"><ClearIcon /></button>}</div><Button variant="secondary" onClick={addInputTags} disabled={busy || !input.trim()}>タグを追加</Button></div><div className="mt-4 flex flex-wrap justify-end gap-2"><Button variant={hasUnsavedChanges ? "danger" : "primary"} onClick={() => onSave(draftTags)} disabled={busy || !hasUnsavedChanges}>タグを保存</Button><Button variant="secondary" onClick={() => setDraftTags([])} disabled={busy || !draftTags.length}>タグ全消去</Button></div></div>;
}

function RepositorySettingsPage({ project, settings, initializationPreview, ignorePreview, busy, onPreviewInitialization, onApplyInitialization, onCancelInitialization, onOpenGlobalDefaults, onUpdateTags, onUpdateVpm, onPreviewIgnore, onApplyIgnore }: { project: ProjectDiagnostic; settings: SettingsLoadResult | null; initializationPreview: RepositoryInitializationPreview | null; ignorePreview: RepositoryIgnorePreview | null; busy: boolean; onPreviewInitialization: () => void; onApplyInitialization: () => void; onCancelInitialization: () => void; onOpenGlobalDefaults: () => void; onUpdateTags: (tags: string[]) => void; onUpdateVpm: (policy: VpmTrackingPolicy | null) => void; onPreviewIgnore: () => void; onApplyIgnore: () => void }) {
  const repositoryRoot = project.repository.root;
  const override = repositoryRoot
    ? settings?.settings.repositorySettings.find((item) => item.repositoryRoot === repositoryRoot)?.vpmTrackingPolicyOverride ?? null
    : null;
  const effectivePolicy = override ?? settings?.settings.vpmTrackingPolicy ?? "EXCLUDE_PACKAGES";
  const effectiveLabel = effectivePolicy === "INCLUDE_PACKAGES" ? "含める" : "除外する";
  const hasMissingRules = ignorePreview?.ignoreFiles.some((file) => file.missingRules.length > 0) ?? false;
  const managedProject = settings?.settings.recentProjects.find((item) => item.path === project.path);
  return <div className="space-y-5"><Card><CardHeader><h3 className="font-semibold">このProjectの設定</h3><p className="mt-1 text-xs text-slate-500">タグやrepository設定はアプリのsettings.jsonに保存され、このrepositoryのファイルは変更しません。</p></CardHeader><CardContent><ProjectTagEditor tags={managedProject?.tags ?? []} busy={busy} onSave={onUpdateTags} /></CardContent></Card><Card><CardHeader><h3 className="font-semibold">このrepositoryの設定</h3><p className="mt-1 text-xs text-slate-500">設定はアプリのsettings.jsonに保存され、このrepositoryのファイルは変更しません。</p></CardHeader><CardContent className="space-y-5"><div className="rounded-xl bg-slate-50 p-4"><div className="flex flex-wrap items-start justify-between gap-3"><div><p className="font-semibold">VPM packageのGit管理</p><p className="mt-1 text-sm text-slate-600">実効値: {effectiveLabel}（{override ? "repository固有の設定" : "全体設定の既定値"}）</p></div><StatusPill label={effectiveLabel} tone="neutral" /></div>{repositoryRoot ? <div className="mt-4 flex flex-wrap gap-2"><Button variant={override === null ? "primary" : "secondary"} onClick={() => onUpdateVpm(null)} disabled={busy}>全体設定に従う</Button><Button variant={override === "EXCLUDE_PACKAGES" ? "primary" : "secondary"} onClick={() => onUpdateVpm("EXCLUDE_PACKAGES")} disabled={busy}>除外する</Button><Button variant={override === "INCLUDE_PACKAGES" ? "primary" : "secondary"} onClick={() => onUpdateVpm("INCLUDE_PACKAGES")} disabled={busy}>含める</Button></div> : <p className="mt-4 text-sm text-slate-600">repositoryが未作成のため、全体設定の既定値を使用します。</p>}<Button className="mt-3" variant="ghost" onClick={onOpenGlobalDefaults}>全体の既定値を開く</Button></div><div className="grid gap-3 md:grid-cols-2"><DiagnosticItem label=".gitignore" status={project.sourceControl.gitignore.status} summary={project.sourceControl.gitignore.summary} /><DiagnosticItem label="VPM packages" status={project.sourceControl.vpmPackages.status} summary={project.sourceControl.vpmPackages.summary} /></div></CardContent></Card><Card><CardHeader><div className="flex flex-wrap items-center justify-between gap-3"><div><h3 className="font-semibold">ignore rule</h3><p className="mt-1 text-xs text-slate-500">不足しているruleだけを確認して追加します。既存ruleは削除しません。</p></div><Button variant="secondary" onClick={onPreviewIgnore} disabled={busy || !project.repository.detected}>不足ruleを確認</Button></div></CardHeader><CardContent>{ignorePreview ? <div className="space-y-3">{ignorePreview.ignoreFiles.map((file) => <div key={file.path} className="rounded-xl bg-slate-50 px-4 py-3"><p className="text-sm font-semibold">{file.path}{file.willCreate ? "（新規作成）" : ""}</p>{file.missingRules.length ? <p className="mt-1 whitespace-pre-wrap text-xs leading-5 text-slate-600">{file.missingRules.join("\n")}</p> : <p className="mt-1 text-xs text-slate-500">不足ruleはありません。</p>}</div>)}{ignorePreview.blockingReason && <BlockingNotice tone="danger">{ignorePreview.blockingReason}</BlockingNotice>}{ignorePreview.canApply && hasMissingRules && <Button onClick={onApplyIgnore} disabled={busy}>この内容を適用</Button>}{ignorePreview.canApply && !hasMissingRules && <p className="rounded-xl bg-emerald-50 px-3 py-2 text-sm text-emerald-800">現在のtemplateとの差分はありません。</p>}</div> : <p className="text-sm text-slate-600">確認すると、現在のtemplateとrepositoryのignore ruleとの差分を表示します。</p>}</CardContent></Card><Card><CardHeader><h3 className="font-semibold">project情報</h3></CardHeader><CardContent><dl className="grid gap-x-6 gap-y-4 text-sm md:grid-cols-2"><Definition label="project folder" value={project.path} /><Definition label="種別" value={projectKindLabel(project.projectKind)} /><Definition label="Unity" value={project.unityVersion ? `Unity ${project.unityVersion}` : "不明"} /><Definition label="repository" value={project.repository.detected ? "検出済み" : "未作成"} /></dl></CardContent></Card>{project.isUnityProject && !project.repository.detected && <RepositorySetup project={project} preview={initializationPreview} busy={busy} onPreview={onPreviewInitialization} onApply={onApplyInitialization} onCancel={onCancelInitialization} />}</div>;
}

function GlobalSettingsPage({ section, environment, settings, busy, onChangeSection, onUpdateVpm, onUpdateIgnoreTemplates, onUpdateLogLevel, onOpenLogs, onOpenLogFolder, onExportLog }: { section: GlobalSettingsSection; environment: EnvironmentDiagnostic | null; settings: SettingsLoadResult | null; busy: boolean; onChangeSection: (section: GlobalSettingsSection) => void; onUpdateVpm: (policy: VpmTrackingPolicy) => void; onUpdateIgnoreTemplates: (templates: IgnoreTemplateSettings) => void; onUpdateLogLevel: (level: string) => void; onOpenLogs: () => void; onOpenLogFolder: () => void; onExportLog: () => void }) {
  return <div className="grid gap-5 lg:grid-cols-[13rem_1fr]"><Card className="h-fit"><CardContent className="space-y-1">{GLOBAL_SETTINGS_SECTIONS.map((item) => <button key={item.value} type="button" onClick={() => onChangeSection(item.value)} className={`w-full rounded-xl px-3 py-2 text-left text-sm font-semibold ${section === item.value ? "bg-slate-900 text-white" : "text-slate-600 hover:bg-slate-100"}`}>{item.label}</button>)}</CardContent></Card><div>{section === "GENERAL" && <GeneralSettings settings={settings} />}{section === "DEFAULTS" && <DefaultSettings settings={settings} busy={busy} onUpdateVpm={onUpdateVpm} onUpdateIgnoreTemplates={onUpdateIgnoreTemplates} />}{section === "ENVIRONMENT" && <EnvironmentSettings environment={environment} />}{section === "LOGGING" && <LoggingSettings settings={settings} busy={busy} onUpdateLogLevel={onUpdateLogLevel} onOpenLogs={onOpenLogs} onOpenLogFolder={onOpenLogFolder} onExportLog={onExportLog} />}</div></div>;
}

function GeneralSettings({ settings }: { settings: SettingsLoadResult | null }) { return <Card><CardHeader><h3 className="font-semibold">一般</h3></CardHeader><CardContent><p className="text-sm text-slate-600">登録済みprojectはホームから選択します。存在しなくなったprojectはホームで「再指定」と表示されます。</p><p className="mt-4 text-xs text-slate-400">登録数: {settings?.recentProjects.length ?? 0} 件</p></CardContent></Card>; }
function DefaultSettings({ settings, busy, onUpdateVpm, onUpdateIgnoreTemplates }: { settings: SettingsLoadResult | null; busy: boolean; onUpdateVpm: (policy: VpmTrackingPolicy) => void; onUpdateIgnoreTemplates: (templates: IgnoreTemplateSettings) => void }) { const policy = settings?.settings.vpmTrackingPolicy ?? "EXCLUDE_PACKAGES"; return <div className="space-y-5"><Card><CardHeader><h3 className="font-semibold">新規repositoryの既定値</h3><p className="mt-1 text-xs text-slate-500">既存repositoryには自動適用しません。</p></CardHeader><CardContent><p className="text-sm font-semibold">VPM packageのGit管理</p><p className="mt-1 text-sm text-slate-600">新しく選択したprojectの診断と初期化previewに使う既定値です。</p><div className="mt-4 flex gap-2"><Button variant={policy === "EXCLUDE_PACKAGES" ? "primary" : "secondary"} onClick={() => onUpdateVpm("EXCLUDE_PACKAGES")} disabled={busy}>除外する</Button><Button variant={policy === "INCLUDE_PACKAGES" ? "primary" : "secondary"} onClick={() => onUpdateVpm("INCLUDE_PACKAGES")} disabled={busy}>含める</Button></div></CardContent></Card><IgnoreTemplateEditor settings={settings} busy={busy} onSave={onUpdateIgnoreTemplates} /></div>; }

function IgnoreTemplateEditor({ settings, busy, onSave }: { settings: SettingsLoadResult | null; busy: boolean; onSave: (templates: IgnoreTemplateSettings) => void }) {
  const [unityText, setUnityText] = useState("");
  const [vpmText, setVpmText] = useState("");
  const unitySource = settings?.settings.ignoreTemplates.unityRules.join("\n") ?? "";
  const vpmSource = settings?.settings.ignoreTemplates.vpmExcludeRules.join("\n") ?? "";
  useEffect(() => setUnityText(unitySource), [unitySource]);
  useEffect(() => setVpmText(vpmSource), [vpmSource]);
  return <Card><CardHeader><h3 className="font-semibold">ignore template</h3><p className="mt-1 text-xs text-slate-500">新規repositoryの初期化と、既存repositoryの不足rule previewに使います。既存repositoryへ自動適用はしません。</p></CardHeader><CardContent className="space-y-4"><label className="block text-sm font-semibold" htmlFor="unity-ignore-template">Unity rules</label><textarea id="unity-ignore-template" className="min-h-48 w-full rounded-xl border border-slate-300 bg-white px-3 py-2 font-mono text-xs leading-5" value={unityText} onChange={(event) => setUnityText(event.target.value)} disabled={busy || !settings} spellCheck={false} /><label className="block text-sm font-semibold" htmlFor="vpm-ignore-template">VPM exclude rules</label><textarea id="vpm-ignore-template" className="min-h-32 w-full rounded-xl border border-slate-300 bg-white px-3 py-2 font-mono text-xs leading-5" value={vpmText} onChange={(event) => setVpmText(event.target.value)} disabled={busy || !settings} spellCheck={false} /><div className="flex items-center justify-between gap-3"><p className="text-xs text-slate-500">空行とコメントはtemplateの一部として保持します。</p><Button onClick={() => onSave({ unityRules: parseTemplateText(unityText), vpmExcludeRules: parseTemplateText(vpmText) })} disabled={busy || !settings}>templateを保存</Button></div></CardContent></Card>;
}
function EnvironmentSettings({ environment }: { environment: EnvironmentDiagnostic | null }) { return <div className="grid gap-5 md:grid-cols-2"><SummaryTile label="実行環境" value={environment ? `${environment.platform.os} / ${environment.platform.architecture}` : "確認中"} detail="正式対応: Windows / Apple Silicon macOS" tone={environment?.platform.supported ? "good" : "warn"} /><SummaryTile label="System Git" value={environment?.git.status === "AVAILABLE" ? "利用可能" : "未検出"} detail={environment?.git.version ?? "PATHから検出します"} tone={environment?.git.status === "AVAILABLE" ? "good" : "warn"} /></div>; }
function LoggingSettings({ settings, busy, onUpdateLogLevel, onOpenLogs, onOpenLogFolder, onExportLog }: { settings: SettingsLoadResult | null; busy: boolean; onUpdateLogLevel: (level: string) => void; onOpenLogs: () => void; onOpenLogFolder: () => void; onExportLog: () => void }) { return <Card><CardHeader><h3 className="font-semibold">ログと診断</h3><p className="mt-1 text-xs text-slate-500">ログレベルの変更は即時適用され、次回起動後も保持されます。</p></CardHeader><CardContent><label className="text-sm font-semibold" htmlFor="log-level">ログレベル</label><select id="log-level" className="mt-2 block rounded-xl border border-slate-300 bg-white px-3 py-2 text-sm" value={settings?.settings.logLevel ?? "INFO"} onChange={(event) => onUpdateLogLevel(event.target.value)} disabled={busy || !settings}>{LOG_LEVEL_OPTIONS.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}</select><p className="mt-3 text-sm leading-6 text-slate-600">ログ表示では、30日保持の対象となるサニタイズ済みログをすべて表示します。</p><div className="mt-5 flex flex-wrap gap-2"><Button variant="secondary" onClick={onOpenLogs} disabled={busy}>ログ表示</Button><Button variant="secondary" onClick={onOpenLogFolder} disabled={busy}>ログフォルダ</Button><Button onClick={onExportLog} disabled={busy}>診断ログを書き出す</Button></div></CardContent></Card>; }

function SummaryTile({ label, value, detail, tone = "neutral" }: { label: string; value: string; detail: string; tone?: "good" | "warn" | "danger" | "neutral" }) { return <Card><CardContent><p className="text-xs font-bold uppercase tracking-[0.14em] text-slate-400">{label}</p><div className="mt-3 flex items-center justify-between gap-2"><p className="font-semibold text-slate-800">{value}</p><StatusPill label={tone === "good" ? "OK" : tone === "danger" ? "注意" : tone === "warn" ? "確認" : "情報"} tone={tone} /></div><p className="mt-2 text-xs leading-5 text-slate-500">{detail}</p></CardContent></Card>; }
type StatusTone = "good" | "warn" | "danger" | "neutral";

type WorkSummaryKey = "project" | "save" | "diagnostic";

function WorkSummary({ project, repositoryState, worktree }: { project: ProjectDiagnostic; repositoryState: RepositoryState | null; worktree: WorktreeSnapshot | null }) {
  const saveStatus = saveStatusPresentation(repositoryState, worktree);
  const projectAbnormal = project.status !== "MANAGEABLE";
  const saveAbnormal = repositoryState !== null && (!repositoryState.canSave || repositoryState.blockingReason !== null);
  const diagnosticAbnormal = projectAbnormal || project.issues.length > 0;
  const [expanded, setExpanded] = useState<Record<WorkSummaryKey, boolean>>(() => ({
    project: projectAbnormal,
    save: saveAbnormal,
    diagnostic: diagnosticAbnormal,
  }));

  useEffect(() => {
    setExpanded((current) => ({
      project: current.project || projectAbnormal,
      save: current.save || saveAbnormal,
      diagnostic: current.diagnostic || diagnosticAbnormal,
    }));
  }, [diagnosticAbnormal, projectAbnormal, saveAbnormal]);

  const toggle = (key: WorkSummaryKey) => setExpanded((current) => ({ ...current, [key]: !current[key] }));
  return <section className="grid gap-3 md:grid-cols-3"><CollapsibleSummaryTile label="project" value={projectKindLabel(project.projectKind)} detail={project.unityVersion ? `Unity ${project.unityVersion}` : "Unity version 不明"} tone={projectAbnormal ? "warn" : "neutral"} expanded={expanded.project} onToggle={() => toggle("project")} abnormal={projectAbnormal} /><CollapsibleSummaryTile label="保存状態" value={saveStatus.label} detail={saveStatus.detail} tone={saveStatus.tone} expanded={expanded.save} onToggle={() => toggle("save")} abnormal={saveAbnormal} /><CollapsibleSummaryTile label="診断" value={projectStatusLabel(project.status)} detail={project.issues.length ? `${project.issues.length} 件の確認項目` : "問題は見つかりませんでした"} tone={diagnosticAbnormal ? "warn" : "good"} expanded={expanded.diagnostic} onToggle={() => toggle("diagnostic")} abnormal={diagnosticAbnormal} /></section>;
}

function CollapsibleSummaryTile({ label, value, detail, tone, expanded, abnormal, onToggle }: { label: string; value: string; detail: string; tone: StatusTone; expanded: boolean; abnormal: boolean; onToggle: () => void }) {
  const statusLabel = tone === "good" ? "OK" : tone === "danger" ? "注意" : tone === "warn" ? "確認" : "情報";
  return <Card className={abnormal ? "border-amber-300" : undefined}><CardContent className="p-0"><button type="button" className="w-full rounded-2xl px-5 py-4 text-left transition hover:bg-slate-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent" onClick={onToggle} aria-expanded={expanded} aria-label={`${label}の情報を${expanded ? "折りたたむ" : "展開する"}`}><div className="flex items-center justify-between gap-3"><div className="min-w-0"><p className="text-xs font-bold uppercase tracking-[0.14em] text-slate-400">{label}</p>{!expanded && <p className="mt-1 truncate text-sm font-semibold text-slate-800" title={value}>{value}</p>}</div><div className="flex shrink-0 items-center gap-2"><StatusPill label={statusLabel} tone={tone} /><ChevronIcon direction={expanded ? "up" : "down"} /></div></div>{expanded && <><p className="mt-3 font-semibold text-slate-800">{value}</p><p className="mt-2 text-xs leading-5 text-slate-500">{detail}</p></>}</button></CardContent></Card>;
}

function saveStatusPresentation(repositoryState: RepositoryState | null, worktree: WorktreeSnapshot | null): { label: string; detail: string; tone: StatusTone } {
  if (!repositoryState) return { label: "読み込み中", detail: "変更を確認中です。", tone: "neutral" };
  if (repositoryState.blockingReason === "CONFLICT") return { label: "競合あり", detail: "競合を解消してから保存してください。", tone: "danger" };
  if (repositoryState.blockingReason === "EXISTING_STAGED_CHANGES") return { label: "ステージ済みの変更あり", detail: "既存のステージ済み変更があるため保存を停止しています。", tone: "danger" };
  if (!repositoryState.canSave) return { label: "確認が必要", detail: "repositoryの状態を確認してください。", tone: "warn" };
  const changeCount = worktree?.files.length ?? 0;
  if (changeCount > 0) return { label: "未保存の変更あり", detail: `未保存のデータが${changeCount}件あります。保存できます。`, tone: "danger" };
  return { label: "保存済み", detail: "現在、未保存の変更はありません。", tone: "good" };
}
type WorktreeFile = WorktreeSnapshot["files"][number];
type DisplayFile = WorktreeFile | RepositoryTreeSnapshot["files"][number];
type FileTreeNode = { name: string; path: string; kind: "folder" | "file"; children: FileTreeNode[]; files: DisplayFile[]; file?: DisplayFile };
type FileTreeColumn = "name" | "status" | "detail" | "type";
type FileTreeColumnWidths = Record<FileTreeColumn, number>;
type ColumnResizeState = { column: FileTreeColumn; startX: number; startWidth: number };
const DEFAULT_FILE_TREE_COLUMN_WIDTHS: FileTreeColumnWidths = { name: 320, status: 128, detail: 224, type: 128 };
const MIN_FILE_TREE_COLUMN_WIDTHS: FileTreeColumnWidths = { name: 220, status: 88, detail: 160, type: 96 };

function ChangedFiles({ files, viewMode, fileContext, onSelect }: { files: DisplayFile[]; viewMode: FileViewMode; fileContext: "WORKTREE" | "COMMIT"; onSelect: (path: string) => void }) {
  const tree = useMemo(() => buildFileTree(files), [files]);
  const treeKey = files.map((file) => `${file.path}:${file.changeKind ?? "UNCHANGED"}:${file.staged ? "s" : ""}:${file.unstaged ? "u" : ""}`).join("\u0000");
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(() => new Set(defaultExpandedPaths(tree)));
  const [columnWidths, setColumnWidths] = useState<FileTreeColumnWidths>(DEFAULT_FILE_TREE_COLUMN_WIDTHS);
  const [columnResize, setColumnResize] = useState<ColumnResizeState | null>(null);

  useEffect(() => {
    if (!columnResize) return;
    const handleMove = (event: PointerEvent) => {
      const delta = event.clientX - columnResize.startX;
      setColumnWidths((current) => ({
        ...current,
        [columnResize.column]: Math.max(MIN_FILE_TREE_COLUMN_WIDTHS[columnResize.column], columnResize.startWidth + delta),
      }));
    };
    const handleUp = () => setColumnResize(null);
    window.addEventListener("pointermove", handleMove);
    window.addEventListener("pointerup", handleUp);
    window.addEventListener("pointercancel", handleUp);
    return () => {
      window.removeEventListener("pointermove", handleMove);
      window.removeEventListener("pointerup", handleUp);
      window.removeEventListener("pointercancel", handleUp);
    };
  }, [columnResize]);

  useEffect(() => {
    setExpandedPaths(new Set(defaultExpandedPaths(tree)));
  }, [tree, treeKey]);

  if (!files.length) return <p className="mt-4 rounded-xl bg-slate-50 px-3 py-3 text-sm text-slate-500">{viewMode === "ALL" ? "表示できるファイルはありません。" : "保存対象の変更はありません。"}</p>;

  const toggleFolder = (path: string) => {
    setExpandedPaths((current) => {
      const next = new Set(current);
      if (next.has(path)) next.delete(path); else next.add(path);
      return next;
    });
  };

  const gridTemplateColumns = `${columnWidths.name}px ${columnWidths.status}px ${columnWidths.detail}px ${columnWidths.type}px`;
  const totalWidth = Object.values(columnWidths).reduce((sum, width) => sum + width, 0);
  const beginColumnResize = (event: ReactPointerEvent<HTMLButtonElement>, column: FileTreeColumn) => {
    event.preventDefault();
    setColumnResize({ column, startX: event.clientX, startWidth: columnWidths[column] });
  };

  return <div className="mt-4 overflow-x-auto rounded-xl border border-slate-200 bg-white"><div style={{ minWidth: `${totalWidth}px` }}><div data-file-tree-header="true" className="grid gap-3 border-b border-slate-200 bg-slate-50 px-3 py-2 text-xs font-semibold text-slate-500" style={{ gridTemplateColumns }}><FileTreeColumnHeader label="名前" column="name" onPointerDown={beginColumnResize} /><FileTreeColumnHeader label="状態" column="status" onPointerDown={beginColumnResize} /><FileTreeColumnHeader label="詳細" column="detail" onPointerDown={beginColumnResize} /><FileTreeColumnHeader label="種類" column="type" onPointerDown={beginColumnResize} /></div><div role="tree" aria-label={fileContext === "COMMIT" ? "保存ファイルのツリー" : "変更ファイルのツリー"}>{tree.map((node) => <FileTreeNodeRow key={node.path} node={node} depth={0} expandedPaths={expandedPaths} fileContext={fileContext} gridTemplateColumns={gridTemplateColumns} onToggle={toggleFolder} onSelect={onSelect} />)}</div></div></div>;
}

function FileTreeColumnHeader({ label, column, onPointerDown }: { label: string; column: FileTreeColumn; onPointerDown: (event: ReactPointerEvent<HTMLButtonElement>, column: FileTreeColumn) => void }) {
  return <span className="relative flex min-w-0 items-center justify-between gap-1"><span className="truncate">{label}</span><button type="button" className="-mr-2 flex h-6 w-3 shrink-0 cursor-col-resize items-center justify-center rounded text-slate-400 hover:bg-slate-200 hover:text-slate-700" style={{ touchAction: "none" }} onPointerDown={(event) => onPointerDown(event, column)} aria-label={`${label}列の幅を調整`} title="ドラッグして列幅を調整"><span aria-hidden="true">⋮</span></button></span>;
}

function FileTreeNodeRow({ node, depth, expandedPaths, fileContext, gridTemplateColumns, onToggle, onSelect }: { node: FileTreeNode; depth: number; expandedPaths: Set<string>; fileContext: "WORKTREE" | "COMMIT"; gridTemplateColumns: string; onToggle: (path: string) => void; onSelect: (path: string) => void }) {
  const expanded = expandedPaths.has(node.path);
  const paddingLeft = `${12 + depth * 22}px`;
  const children = node.kind === "folder" && expanded ? node.children.map((child) => <FileTreeNodeRow key={child.path} node={child} depth={depth + 1} expandedPaths={expandedPaths} fileContext={fileContext} gridTemplateColumns={gridTemplateColumns} onToggle={onToggle} onSelect={onSelect} />) : null;
  if (node.kind === "folder") return <><div role="treeitem" aria-expanded={expanded} className="grid items-center gap-3 border-b border-slate-100 px-3 py-2 text-sm hover:bg-slate-50" style={{ gridTemplateColumns }}><button type="button" className="flex min-w-0 items-center gap-2 text-left font-semibold text-slate-700" style={{ paddingLeft }} onClick={() => onToggle(node.path)} aria-label={expanded ? `${node.name}を折りたたむ` : `${node.name}を展開する`}><span className="inline-flex h-4 w-4 shrink-0 items-center justify-center text-slate-400">{expanded ? "⌄" : "›"}</span><FolderIcon /><span className="truncate" title={node.path}>{node.name}</span></button><span className={`text-xs ${fileTreeStatusClass(node.files, fileContext)}`}>{fileTreeStatusLabel(node.files)}</span><span className="truncate text-xs text-slate-500" title={fileTreeChangeSummary(node.files)}>{node.files.length} 件 · {fileTreeChangeSummary(node.files)}</span><span className="text-xs text-slate-400">フォルダ</span></div>{children}</>;
  const file = node.file!;
  const selectable = file.changeKind !== null;
  return <button type="button" onClick={() => onSelect(file.path)} disabled={!selectable} aria-label={`${file.path} ${changeKindLabel(file.changeKind)}`} title={file.path} className={`grid w-full items-center gap-3 border-b border-slate-100 px-3 py-2 text-left text-sm ${selectable ? "transition hover:bg-slate-50" : "cursor-default"}`} style={{ gridTemplateColumns }}><span className="flex min-w-0 items-center gap-2 text-slate-700" style={{ paddingLeft: `${12 + depth * 22 + 22}px` }}><FileIcon /><span className="truncate">{node.name}</span></span><span className={`text-xs ${fileStatusClass(file)}`}>{changeKindLabel(file.changeKind)}</span><span className="truncate text-xs text-slate-500" title={fileDetailLabel(file, fileContext)}>{fileDetailLabel(file, fileContext)}</span><span className="text-xs text-slate-400">{file.binary ? "バイナリ" : "テキスト"}</span></button>;
}

function buildFileTree(files: DisplayFile[]): FileTreeNode[] {
  const root: FileTreeNode = { name: "", path: "", kind: "folder", children: [], files: [...files] };
  for (const file of files) {
    const segments = file.path.replaceAll("\\", "/").split("/").filter(Boolean);
    let parent = root;
    for (const [index, segment] of segments.entries()) {
      const path = segments.slice(0, index + 1).join("/");
      let child = parent.children.find((candidate) => candidate.name === segment);
      if (!child) {
        child = { name: segment, path, kind: index === segments.length - 1 ? "file" : "folder", children: [], files: [], file: index === segments.length - 1 ? file : undefined };
        parent.children.push(child);
      }
      child.files.push(file);
      parent = child;
    }
  }
  sortFileTree(root);
  return root.children;
}

function sortFileTree(node: FileTreeNode) {
  node.children.sort((left, right) => Number(right.kind === "folder") - Number(left.kind === "folder") || left.name.localeCompare(right.name, "ja"));
  node.children.forEach(sortFileTree);
}

function defaultExpandedPaths(nodes: FileTreeNode[]): string[] {
  return nodes.flatMap((node) => node.kind === "folder" && node.files.length <= 40 ? [node.path, ...defaultExpandedPaths(node.children)] : []);
}

function fileTreeStatusLabel(files: DisplayFile[]) {
  const labels = [...new Set(files.filter((file) => file.changeKind !== null).map((file) => changeKindLabel(file.changeKind)))];
  return labels.length === 0 ? "変更なし" : labels.length === 1 ? labels[0] : "複数";
}

function fileStatusClass(file: DisplayFile) {
  if (file.changeKind === null) return "text-slate-400";
  return file.changeKind === "ADDED" || file.changeKind === "UNTRACKED" || file.changeKind === "TYPE_CHANGED"
    ? "font-semibold text-amber-700"
    : file.changeKind === "RENAMED" || file.changeKind === "COPIED"
      ? "font-semibold text-sky-700"
      : "font-semibold text-rose-700";
}

function fileTreeStatusClass(files: DisplayFile[], fileContext: "WORKTREE" | "COMMIT") {
  if (files.every((file) => file.changeKind === null)) return "text-slate-400";
  return files.some((file) => file.changeKind === "UNMERGED" || file.changeKind === "DELETED" || (fileContext === "WORKTREE" && file.staged))
    ? "font-semibold text-rose-700"
    : "font-semibold text-amber-700";
}

function fileTreeChangeSummary(files: DisplayFile[]) {
  const counts = new Map<string, number>();
  for (const file of files) {
    const label = changeKindLabel(file.changeKind);
    counts.set(label, (counts.get(label) ?? 0) + 1);
  }
  return [...counts.entries()].map(([label, count]) => `${label} ${count}`).join(" · ");
}

function fileDetailLabel(file: DisplayFile, fileContext: "WORKTREE" | "COMMIT" = "WORKTREE") {
  if (file.changeKind === null) return file.outsideProject ? "変更なし · project外" : "変更なし";
  if (fileContext === "COMMIT") return file.oldPath ? `保存済み · 旧: ${file.oldPath}` : "保存済み";
  const details = [file.staged && file.unstaged ? "index + 作業中" : file.staged ? "ステージ済み" : "作業中"];
  if (file.outsideProject) details.push("project外");
  if (file.oldPath) details.push(`旧: ${file.oldPath}`);
  return details.join(" · ");
}

function FileIcon() {
  return <svg aria-hidden="true" viewBox="0 0 24 24" className="h-4 w-4 shrink-0 text-slate-400" fill="none" stroke="currentColor" strokeWidth="1.7"><path strokeLinecap="round" strokeLinejoin="round" d="M6 3.5h8l4 4v13H6z" /><path strokeLinecap="round" d="M14 3.5v4h4" /></svg>;
}
function DiagnosticSummary({ project, onGoToSettings }: { project: ProjectDiagnostic; onGoToSettings: () => void }) { return <Card><CardHeader><div className="flex items-center justify-between gap-3"><div><h3 className="font-semibold">projectの診断</h3><p className="mt-1 text-xs text-slate-500">詳細な設定とignore ruleはリポジトリ設定で確認できます。</p></div><Button variant="secondary" onClick={onGoToSettings}>リポジトリ設定</Button></div></CardHeader><CardContent>{project.issues.length ? <div className="space-y-2">{project.issues.map((issue, index) => <div key={`${issue.code}-${index}`} className={issueClass(issue.severity)}><p className="text-sm font-medium">{issue.message}</p><p className="mt-1 text-xs opacity-70">{issue.code}{shouldShowIssuePath(issue) ? ` · ${issue.path}` : ""}</p></div>)}</div> : <p className="rounded-xl bg-emerald-50 px-4 py-3 text-sm text-emerald-800">現在の診断範囲では、修正が必要な状態は見つかりませんでした。</p>}</CardContent></Card>; }
function DiffPanel({ diff }: { diff: FileDiff }) { return <Card><CardHeader><h3 className="truncate font-semibold" title={diff.path}>{diff.path}</h3><p className="mt-1 text-xs text-slate-500">{diff.kind === "TEXT" ? "テキスト差分" : diff.kind === "BINARY" ? "バイナリファイル" : "表示不可"}</p></CardHeader><CardContent>{diff.patch ? <pre className="max-h-[32rem] overflow-auto whitespace-pre-wrap rounded-xl bg-slate-950 p-4 text-xs leading-5 text-slate-100">{diff.patch}</pre> : <p className="rounded-xl bg-slate-50 px-3 py-3 text-sm text-slate-600">{diff.truncationReason ?? "テキストとして表示できません。"}</p>}{diff.truncated && <p className="mt-3 text-xs text-amber-700">{diff.truncationReason}</p>}</CardContent></Card>; }
function BlockingNotice({ children, tone = "warn" }: { children: string; tone?: "warn" | "danger" }) { return <p className={`mt-4 rounded-xl px-3 py-2 text-sm ${tone === "danger" ? "bg-rose-50 text-rose-800" : "bg-amber-50 text-amber-800"}`}>{children}</p>; }
function DiagnosticItem({ label, status, summary }: { label: string; status: ProjectDiagnostic["sourceControl"]["gitignore"]["status"]; summary: string }) { const tone = status === "HEALTHY" ? "border-emerald-200 bg-emerald-50" : status === "NOT_APPLICABLE" ? "border-slate-200 bg-slate-50" : "border-amber-200 bg-amber-50"; return <div className={`rounded-xl border px-3 py-3 ${tone}`}><p className="text-xs font-semibold text-slate-700">{label}</p><p className="mt-1 text-xs leading-5 text-slate-600">{summary}</p></div>; }
function Definition({ label, value }: { label: string; value: string }) { return <div><dt className="text-xs font-semibold text-slate-400">{label}</dt><dd className="mt-1 break-all text-slate-700">{label === "project folder" ? displayPath(value) : value}</dd></div>; }
function ErrorNotice({ error, onDismiss }: { error: AppError; onDismiss: () => void }) { const warning = error.code === "REPOSITORY_STATE_CHANGED"; return <div className={`mb-5 flex items-start justify-between gap-4 rounded-2xl px-5 py-4 text-sm ${warning ? "border border-amber-200 bg-amber-50 text-amber-900" : "border border-rose-200 bg-rose-50 text-rose-800"}`} role="alert"><div><p className="font-semibold">{warning ? "保存前の変更を確認してください" : error.message}</p>{warning && <p className="mt-1">{error.message}</p>}<p className={`mt-1 text-xs ${warning ? "text-amber-800" : "text-rose-700"}`}>{error.code}{error.mayHaveMutated ? " · 状態が変化した可能性があります" : ""}</p></div><button className="text-xs font-semibold underline" onClick={onDismiss}>閉じる</button></div>; }
function RecoveryNotice({ settings }: { settings: SettingsLoadResult }) { return <div className="mb-5 rounded-2xl border border-amber-200 bg-amber-50 px-5 py-4 text-sm text-amber-900">settings.json を保全して初期設定を再生成しました。{settings.backupPath ? ` 退避先: ${settings.backupPath}` : ""}</div>; }

function currentWindowLabel() { try { return getCurrentWindow().label; } catch { return undefined; } }
function normalizeError(error: unknown): AppError { if (isAppError(error)) return error; return { code: "INTERNAL_ERROR", message: "処理に失敗しました。Vsedi のログを確認してください。", technicalDetail: null, operation: null, mayHaveMutated: false }; }
function displayPath(path: string) {
  const longPathPrefix = "\\\\?" + "\\";
  const withoutLongPathPrefix = path.startsWith(longPathPrefix) ? path.slice(longPathPrefix.length) : path;
  const uncPrefix = "UNC" + "\\";
  return withoutLongPathPrefix.startsWith(uncPrefix)
    ? `//${withoutLongPathPrefix.slice(uncPrefix.length).replaceAll("\\", "/")}`
    : withoutLongPathPrefix.replaceAll("\\", "/");
}
function projectName(path: string) { const parts = path.split(/[\\/]/).filter(Boolean); return parts.at(-1) ?? path; }
function formatDate(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString("ja-JP"); }
function formatGitCommand(event: GitCommandEvent) { return [event.executable, ...event.args].map((value) => /^(?:[A-Za-z0-9_./-]+)$/.test(value) ? value : `"${value.replaceAll("\\", "\\\\").replaceAll('"', '\\"')}"`).join(" "); }
function ProjectTagBadges({ tags }: { tags: string[] }) { return tags.length ? <div className="flex flex-wrap gap-1">{tags.map((tag) => <span key={tag} className="rounded-full bg-sky-50 px-2.5 py-1 text-xs font-semibold text-sky-700">{tag}</span>)}</div> : null; }
function parseTags(value: string) { return normalizeTags(value.split(/[\n,、]/)); }
function normalizeTags(tags: string[]) { return tags.map((tag) => tag.trim()).filter(Boolean).filter((tag, index, values) => values.indexOf(tag) === index).slice(0, 20); }
function parseTemplateText(value: string) { const lines = value.replace(/\r\n/g, "\n").split("\n"); return lines.at(-1) === "" ? lines.slice(0, -1) : lines; }
function compareManagedProjects(left: SettingsLoadResult["recentProjects"][number], right: SettingsLoadResult["recentProjects"][number]) { return (right.lastOpenedAt ?? "").localeCompare(left.lastOpenedAt ?? "") || left.path.localeCompare(right.path, "ja"); }
function projectStatusLabel(status: ProjectDiagnostic["status"]) { return status === "MANAGEABLE" ? "管理可能" : status === "NEEDS_ATTENTION" ? "要確認" : "非 Unity"; }
function projectKindLabel(kind: ProjectDiagnostic["projectKind"]) { const labels: Record<ProjectDiagnostic["projectKind"], string> = { UNITY: "Unity project", VRCHAT_AVATAR: "VRChat Avatar", VRCHAT_WORLD: "VRChat World", VRCHAT_UNKNOWN: "VRChat 種別不明" }; return labels[kind]; }
function changeKindLabel(kind: WorktreeSnapshot["files"][number]["changeKind"] | null) { if (kind === null) return "変更なし"; const labels: Record<WorktreeSnapshot["files"][number]["changeKind"], string> = { ADDED: "追加", MODIFIED: "変更", DELETED: "削除", RENAMED: "名前変更", COPIED: "複製", TYPE_CHANGED: "種類変更", UNMERGED: "競合", UNTRACKED: "未管理" }; return labels[kind]; }
function issueClass(severity: ProjectDiagnostic["issues"][number]["severity"]) { return severity === "ERROR" ? "rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-rose-800" : severity === "WARNING" ? "rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-amber-900" : "rounded-xl border border-sky-200 bg-sky-50 px-4 py-3 text-sky-800"; }
function shouldShowIssuePath(issue: ProjectDiagnostic["issues"][number]) { return issue.code !== "GIT_ROOT_OUTSIDE_PROJECT" && Boolean(issue.path); }

const GLOBAL_SETTINGS_SECTIONS: { value: GlobalSettingsSection; label: string }[] = [{ value: "GENERAL", label: "一般" }, { value: "DEFAULTS", label: "新規repositoryの既定値" }, { value: "ENVIRONMENT", label: "実行環境" }, { value: "LOGGING", label: "ログと診断" }];
const LOG_LEVEL_OPTIONS = [{ value: "ERROR", label: "ERROR — エラーのみ" }, { value: "WARN", label: "WARN — 警告以上" }, { value: "INFO", label: "INFO — 通常" }, { value: "DEBUG", label: "DEBUG — 詳細" }, { value: "TRACE", label: "TRACE — 最詳細" }] as const;

export default App;
