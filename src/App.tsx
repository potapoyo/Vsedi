import { useEffect, useMemo, useRef, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { StatusPill } from "@/components/ui/status-pill";
import { LogWindow } from "@/LogWindow";
import type { AppError, CommitDetail, EnvironmentDiagnostic, FileDiff, HistoryEntry, ProjectDiagnostic, RepositoryInitializationPreview, RepositoryState, SaveResult, SettingsLoadResult, VpmTrackingPolicy, WorktreeSnapshot } from "@/generated/bindings";
import { exportDiagnosticLog, initializeRepository, inspectEnvironment, inspectProject, isAppError, loadSettings, openLogDirectory, openLogWindow, previewRepositoryInitialization, readCommitDetail, readCommitDiff, readHistory, readRepositoryState, readWorktreeDiff, readWorktreeSnapshot, saveSettings, saveWorktree } from "@/lib/commands";

type GlobalSettingsSection = "GENERAL" | "DEFAULTS" | "ENVIRONMENT" | "LOGGING";
type RepositorySection = "WORK" | "HISTORY" | "SETTINGS";
type AppRoute =
  | { page: "HOME" }
  | { page: "GLOBAL_SETTINGS"; section: GlobalSettingsSection }
  | { page: "REPOSITORY"; section: RepositorySection };

function App() {
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
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [commitDetail, setCommitDetail] = useState<CommitDetail | null>(null);
  const [fileDiff, setFileDiff] = useState<FileDiff | null>(null);
  const [initializationPreview, setInitializationPreview] = useState<RepositoryInitializationPreview | null>(null);
  const [saveResult, setSaveResult] = useState<SaveResult | null>(null);
  const [pending, setPending] = useState<string | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const selectionGeneration = useRef(0);

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
    setHistory([]);
    setCommitDetail(null);
    setFileDiff(null);
    setInitializationPreview(null);
    setSaveResult(null);
  };

  const reloadRepositoryData = async (projectPath = project?.path, expectedGeneration?: number) => {
    if (!projectPath) return;
    const [state, snapshot, entries] = await Promise.all([
      readRepositoryState(projectPath),
      readWorktreeSnapshot(projectPath),
      readHistory(projectPath),
    ]);
    if (expectedGeneration !== undefined && expectedGeneration !== selectionGeneration.current) return;
    setRepositoryState(state);
    setWorktree(snapshot);
    setHistory(entries);
  };

  const selectProject = async (path: string) => {
    if (!settings) return;
    const generation = ++selectionGeneration.current;
    await run("project を開く", async () => {
      const result = await inspectProject(path, settings.settings.vpmTrackingPolicy);
      if (generation !== selectionGeneration.current) return;
      clearRepositoryData();
      setProject(result);
      const existing = settings.settings.recentProjects.find((item) => item.path === result.path);
      const updatedProject = { path: result.path, lastOpenedAt: new Date().toISOString(), category: existing?.category ?? null };
      const nextSettings = {
        ...settings.settings,
        recentProjects: [updatedProject, ...settings.settings.recentProjects.filter((item) => item.path !== result.path)],
      };
      await saveSettings(nextSettings);
      if (generation !== selectionGeneration.current) return;
      setSettings({
        ...settings,
        settings: nextSettings,
        recentProjects: [{ ...updatedProject, exists: true }, ...settings.recentProjects.filter((item) => item.path !== result.path)],
      });
      setRoute({ page: "REPOSITORY", section: "WORK" });
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

  const updateSettings = async (nextSettings: SettingsLoadResult["settings"]) => {
    await run("設定を保存", async () => {
      await saveSettings(nextSettings);
      setSettings((current) => current ? { ...current, settings: nextSettings } : current);
      if (project) {
        const refreshed = await inspectProject(project.path, nextSettings.vpmTrackingPolicy);
        setProject(refreshed);
      }
    });
  };

  const updateVpmTrackingPolicy = async (policy: VpmTrackingPolicy) => {
    if (!settings || settings.settings.vpmTrackingPolicy === policy) return;
    await updateSettings({ ...settings.settings, vpmTrackingPolicy: policy });
  };

  const updateLogLevel = async (logLevel: string) => {
    if (!settings || settings.settings.logLevel === logLevel) return;
    await updateSettings({ ...settings.settings, logLevel });
  };

  const updateProjectCategory = async (path: string, category: string | null) => {
    if (!settings) return;
    const normalizedCategory = category?.trim() || null;
    const updatedAt = new Date().toISOString();
    const nextSettings = {
      ...settings.settings,
      recentProjects: settings.settings.recentProjects.map((item) => item.path === path ? { ...item, category: normalizedCategory, lastOpenedAt: updatedAt } : item),
    };
    await run("カテゴリを保存", async () => {
      await saveSettings(nextSettings);
      setSettings({
        ...settings,
        settings: nextSettings,
        recentProjects: settings.recentProjects
          .map((item) => item.path === path ? { ...item, category: normalizedCategory, lastOpenedAt: updatedAt } : item)
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

  const applyInitialization = async () => {
    if (!project || !settings || !initializationPreview) return;
    await run("repository を初期化", async () => {
      await initializeRepository({ projectPath: project.path, statusToken: initializationPreview.statusToken });
      setInitializationPreview(null);
      const refreshed = await inspectProject(project.path, settings.settings.vpmTrackingPolicy);
      setProject(refreshed);
      await reloadRepositoryData(refreshed.path);
    });
  };

  const saveCurrentWork = async (memo: string) => {
    if (!project || !worktree) return;
    await run("作業を保存", async () => {
      const result = await saveWorktree({ projectPath: project.path, statusToken: worktree.statusToken, memo });
      setSaveResult(result);
      setCommitDetail(null);
      setFileDiff(null);
      await reloadRepositoryData(project.path);
    });
  };

  const selectCommit = async (entry: HistoryEntry) => {
    if (!project) return;
    await run("保存詳細を読み込む", async () => {
      setFileDiff(null);
      setCommitDetail(await readCommitDetail(project.path, entry.commitId));
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
    <main className="min-h-screen bg-mist text-ink">
      <div className="mx-auto flex min-h-screen max-w-[1440px]">
        <AppSidebar
          route={route}
          project={project}
          onHome={() => setRoute({ page: "HOME" })}
          onRepository={navigateRepository}
          onGlobalSettings={(section) => setRoute({ page: "GLOBAL_SETTINGS", section })}
        />
        <div className="min-w-0 flex-1 px-5 py-6 sm:px-8">
          <AppHeader
            pageTitle={pageTitle}
            project={route.page === "REPOSITORY" ? project : null}
            repositoryState={repositoryState}
            pending={pending}
            onRefresh={() => void refreshApplication()}
          />

          {error && <ErrorNotice error={error} onDismiss={() => setError(null)} />}
          {settings?.recovered && <RecoveryNotice settings={settings} />}

          {route.page === "HOME" && (
            <HomePage
              environment={environment}
              settings={settings}
              busy={isBusy}
              onChooseProject={() => void chooseProject()}
              onOpenProject={(path) => void selectProject(path)}
              onSetCategory={(path, category) => void updateProjectCategory(path, category)}
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
            />
          )}

          {route.page === "REPOSITORY" && project && route.section === "WORK" && (
            <WorkPage
              project={project}
              repositoryState={repositoryState}
              worktree={worktree}
              initializationPreview={initializationPreview}
              saveResult={saveResult}
              busy={isBusy}
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
              commitDetail={commitDetail}
              fileDiff={fileDiff}
              busy={isBusy}
              onSelectCommit={(entry) => void selectCommit(entry)}
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
    <aside className="hidden w-64 shrink-0 border-r border-slate-200 bg-white px-3 py-6 lg:block">
      <div className="px-3 pb-7"><p className="text-xs font-bold uppercase tracking-[0.28em] text-accent">Local first</p><h1 className="mt-2 text-2xl font-bold tracking-tight">Vsedi</h1></div>
      <nav className="space-y-1" aria-label="メインナビゲーション">
        <NavigationButton active={route.page === "HOME"} onClick={onHome}>ホーム</NavigationButton>
        {repositoryOpen && project && (
          <>
            <p className="px-3 pt-6 text-[11px] font-bold uppercase tracking-[0.14em] text-slate-400">選択中の project</p>
            <p className="truncate px-3 pt-2 text-sm font-semibold text-slate-700" title={project.path}>{projectName(project.path)}</p>
            <NavigationButton active={route.section === "WORK"} onClick={() => onRepository("WORK")}>現在の作業</NavigationButton>
            <NavigationButton active={route.section === "HISTORY"} onClick={() => onRepository("HISTORY")}>保存履歴</NavigationButton>
            <NavigationButton active={route.section === "SETTINGS"} onClick={() => onRepository("SETTINGS")}>リポジトリ設定</NavigationButton>
          </>
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

function AppHeader({ pageTitle, project, repositoryState, pending, onRefresh }: { pageTitle: string; project: ProjectDiagnostic | null; repositoryState: RepositoryState | null; pending: string | null; onRefresh: () => void }) {
  return (
    <header className="mb-6 flex flex-wrap items-start justify-between gap-4 border-b border-slate-200 pb-5">
      <div><p className="text-xs font-bold uppercase tracking-[0.18em] text-slate-400">Vsedi</p><h2 className="mt-1 text-2xl font-bold tracking-tight">{pageTitle}</h2>{project && <p className="mt-1 break-all text-sm text-slate-500">{project.path}{repositoryState?.root && repositoryState.root !== project.path ? ` · 保存対象: ${repositoryState.root}` : ""}</p>}</div>
      <div className="flex items-center gap-2">{pending && <span className="text-xs text-slate-500">{pending}…</span>}<Button variant="ghost" onClick={onRefresh} disabled={Boolean(pending)}>再読込</Button></div>
    </header>
  );
}

function HomePage({ environment, settings, busy, onChooseProject, onOpenProject, onSetCategory, onOpenSettings }: { environment: EnvironmentDiagnostic | null; settings: SettingsLoadResult | null; busy: boolean; onChooseProject: () => void; onOpenProject: (path: string) => void; onSetCategory: (path: string, category: string | null) => void; onOpenSettings: () => void }) {
  const gitAvailable = environment?.git.status === "AVAILABLE";
  return <div className="space-y-6">
    <section className="rounded-3xl bg-slate-900 px-6 py-7 text-white shadow-panel sm:px-8"><p className="text-xs font-bold uppercase tracking-[0.2em] text-sky-200">制作のセーブポイント</p><h3 className="mt-3 text-3xl font-bold tracking-tight">管理する project を選択</h3><p className="mt-3 max-w-2xl text-sm leading-6 text-slate-300">project を選ぶと、Unity / VRChat / Git の状態を確認して、この repository の作業画面を開きます。</p><Button className="mt-5 bg-white text-slate-900 hover:bg-slate-100" onClick={onChooseProject} disabled={busy}>project を追加</Button></section>
    {!gitAvailable && environment && <Card className="border-amber-200 bg-amber-50"><CardContent className="flex flex-wrap items-center justify-between gap-3"><div><p className="font-semibold text-amber-900">System Git を確認してください</p><p className="mt-1 text-sm text-amber-800">Git が利用できないため、作業を保存できません。</p></div><Button variant="secondary" onClick={onOpenSettings}>実行環境を開く</Button></CardContent></Card>}
    <ManagedProjectList projects={settings?.recentProjects ?? []} busy={busy} onOpenProject={onOpenProject} onSetCategory={onSetCategory} />
  </div>;
}

function ManagedProjectList({ projects, busy, onOpenProject, onSetCategory }: { projects: SettingsLoadResult["recentProjects"]; busy: boolean; onOpenProject: (path: string) => void; onSetCategory: (path: string, category: string | null) => void }) {
  const [categoryFilter, setCategoryFilter] = useState("ALL");
  const [editingPath, setEditingPath] = useState<string | null>(null);
  const [categoryDraft, setCategoryDraft] = useState("");
  const categories = [...new Set(projects.map((item) => item.category).filter((category): category is string => Boolean(category)))].sort((left, right) => left.localeCompare(right, "ja"));
  const categoryKey = categories.join("\u0000");
  const sortedProjects = [...projects].sort(compareManagedProjects);
  const visibleProjects = categoryFilter === "ALL" ? sortedProjects : sortedProjects.filter((item) => item.category === categoryFilter);

  useEffect(() => {
    if (categoryFilter !== "ALL" && !categories.includes(categoryFilter)) setCategoryFilter("ALL");
  }, [categoryFilter, categoryKey]);

  const startCategoryEdit = (path: string, category: string | null) => {
    setEditingPath(path);
    setCategoryDraft(category ?? "");
  };

  const saveCategory = (path: string) => {
    onSetCategory(path, categoryDraft);
    setEditingPath(null);
  };

  return <section><div className="mb-3 flex flex-wrap items-end justify-between gap-4"><div><h3 className="text-lg font-bold">管理しているProject</h3><p className="mt-1 text-sm text-slate-500">最終更新が新しい順に表示します。</p></div><div className="flex items-center gap-3"><label className="text-xs font-semibold text-slate-500" htmlFor="project-category-filter">カテゴリ</label><select id="project-category-filter" className="rounded-xl border border-slate-300 bg-white px-3 py-2 text-sm" value={categoryFilter} onChange={(event) => setCategoryFilter(event.target.value)}><option value="ALL">すべて</option>{categories.map((category) => <option key={category} value={category}>{category}</option>)}</select><span className="text-xs text-slate-400">{visibleProjects.length} 件</span></div></div>{visibleProjects.length ? <div className="space-y-3">{visibleProjects.map((item) => <Card key={item.path}><CardContent><div className="flex flex-wrap items-start justify-between gap-4"><div className="min-w-0 flex-1"><div className="flex flex-wrap items-center gap-2"><p className="truncate font-semibold text-slate-800" title={item.path}>{projectName(item.path)}</p>{item.category && <span className="rounded-full bg-sky-50 px-2.5 py-1 text-xs font-semibold text-sky-700">{item.category}</span>}{!item.exists && <StatusPill label="再指定" tone="warn" />}</div><p className="mt-1 truncate text-xs text-slate-500" title={item.path}>{item.path}</p><p className="mt-3 text-xs text-slate-400">{item.lastOpenedAt ? `最終更新: ${formatDate(item.lastOpenedAt)}` : "更新日時は未記録"}</p></div><div className="flex gap-2"><Button variant="secondary" onClick={() => startCategoryEdit(item.path, item.category)} disabled={busy}>カテゴリ設定</Button><Button onClick={() => onOpenProject(item.path)} disabled={busy || !item.exists}>開く</Button></div></div>{editingPath === item.path && <div className="mt-4 flex flex-wrap gap-2 border-t border-slate-100 pt-4"><input className="min-w-48 flex-1 rounded-xl border border-slate-300 px-3 py-2 text-sm" value={categoryDraft} maxLength={40} onChange={(event) => setCategoryDraft(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") saveCategory(item.path); }} placeholder="例: Avatar、World、作業中" autoFocus disabled={busy} /><Button onClick={() => saveCategory(item.path)} disabled={busy}>保存</Button>{item.category && <Button variant="secondary" onClick={() => { onSetCategory(item.path, null); setEditingPath(null); }} disabled={busy}>カテゴリを外す</Button>}<Button variant="ghost" onClick={() => setEditingPath(null)} disabled={busy}>キャンセル</Button></div>}</CardContent></Card>)}</div> : <Card><CardContent><p className="py-6 text-center text-sm text-slate-500">{projects.length ? "このカテゴリにProjectはありません。" : "まだProjectは登録されていません。"}</p></CardContent></Card>}</section>;
}

function WorkPage({ project, repositoryState, worktree, initializationPreview, saveResult, busy, onPreviewInitialization, onApplyInitialization, onCancelInitialization, onSave, onShowDiff, fileDiff, onGoToRepositorySettings }: {
  project: ProjectDiagnostic; repositoryState: RepositoryState | null; worktree: WorktreeSnapshot | null; initializationPreview: RepositoryInitializationPreview | null; saveResult: SaveResult | null; busy: boolean; onPreviewInitialization: () => void; onApplyInitialization: () => void; onCancelInitialization: () => void; onSave: (memo: string) => void; onShowDiff: (path: string) => void; fileDiff: FileDiff | null; onGoToRepositorySettings: () => void;
}) {
  const [memo, setMemo] = useState("");
  useEffect(() => { setMemo(""); }, [project.path]);
  if (!project.repository.detected) return <RepositorySetup project={project} preview={initializationPreview} busy={busy} onPreview={onPreviewInitialization} onApply={onApplyInitialization} onCancel={onCancelInitialization} onGoToSettings={onGoToRepositorySettings} />;
  return <div className="space-y-5">
    <section className="grid gap-3 md:grid-cols-3"><SummaryTile label="project" value={projectKindLabel(project.projectKind)} detail={project.unityVersion ? `Unity ${project.unityVersion}` : "Unity version 不明"} /><SummaryTile label="保存状態" value={repositoryState?.canSave ? "保存可能" : repositoryState ? "確認が必要" : "読み込み中"} detail={worktree ? `${worktree.files.length} 件の変更` : "変更を読み込み中"} tone={repositoryState?.canSave ? "good" : "warn"} /><SummaryTile label="診断" value={projectStatusLabel(project.status)} detail={project.issues.length ? `${project.issues.length} 件の確認項目` : "問題は見つかりませんでした"} tone={project.status === "MANAGEABLE" ? "good" : "warn"} /></section>
    <Card><CardHeader><div className="flex items-center justify-between gap-3"><div><h3 className="font-semibold">現在の変更</h3><p className="mt-1 text-xs text-slate-500">保存対象はrepository全体です。project外の変更もここに表示します。</p></div><StatusPill label={repositoryState?.canSave ? "保存可能" : "確認が必要"} tone={repositoryState?.canSave ? "good" : "warn"} /></div></CardHeader><CardContent>
      {repositoryState?.blockingReason === "EXISTING_STAGED_CHANGES" && <BlockingNotice>すでにGitのステージにある変更があるため、安全のため保存を開始できません。</BlockingNotice>}
      {repositoryState?.blockingReason === "CONFLICT" && <BlockingNotice tone="danger">競合中のファイルがあるため、保存を開始できません。</BlockingNotice>}
      <ChangedFiles files={worktree?.files ?? []} onSelect={onShowDiff} />
      {repositoryState?.canSave && worktree?.files.length ? <div className="mt-4 flex flex-wrap gap-2 border-t border-slate-100 pt-4"><input className="min-w-56 flex-1 rounded-xl border border-slate-300 px-3 py-2 text-sm" value={memo} onChange={(event) => setMemo(event.target.value)} placeholder="保存メモ（例: アバターの表情を調整）" disabled={busy} /><Button onClick={() => { onSave(memo); setMemo(""); }} disabled={busy || !memo.trim()}>作業を保存</Button></div> : null}
      {saveResult && <p className="mt-4 rounded-xl bg-emerald-50 px-3 py-2 text-xs text-emerald-800">保存しました: {saveResult.shortCommitId} · {saveResult.fileCount} file · {saveResult.authorTime}</p>}
    </CardContent></Card>
    {fileDiff && <DiffPanel diff={fileDiff} />}
    <DiagnosticSummary project={project} onGoToSettings={onGoToRepositorySettings} />
  </div>;
}

function RepositorySetup({ project, preview, busy, onPreview, onApply, onCancel, onGoToSettings }: { project: ProjectDiagnostic; preview: RepositoryInitializationPreview | null; busy: boolean; onPreview: () => void; onApply: () => void; onCancel: () => void; onGoToSettings?: () => void }) {
  return <div className="space-y-5"><Card className="border-sky-200 bg-sky-50"><CardHeader><h3 className="font-semibold text-sky-950">ローカル保存を準備する</h3></CardHeader><CardContent><p className="text-sm leading-6 text-sky-900">このUnity projectにはまだGit repositoryがありません。作成内容を確認してから、Unity用のignore ruleとともにローカル保存を始められます。</p>{!preview ? <Button className="mt-4" onClick={onPreview} disabled={busy}>作成内容を確認</Button> : <div className="mt-4 space-y-3 rounded-xl bg-white/80 p-4">{preview.ignoreFiles.map((file) => <div key={file.path}><p className="text-sm font-semibold text-slate-800">{file.path}{file.willCreate ? "（新規作成）" : ""}</p><p className="mt-1 text-xs text-slate-600">{file.missingRules.length ? `${file.missingRules.length} 件のruleを追加します。` : "変更はありません。"}</p></div>)}{preview.canInitialize ? <div className="flex gap-2"><Button onClick={onApply} disabled={busy}>この内容で初期化</Button><Button variant="secondary" onClick={onCancel} disabled={busy}>キャンセル</Button></div> : <p className="text-sm text-rose-800">{preview.blockingReason}</p>}</div>}</CardContent></Card>{onGoToSettings && <DiagnosticSummary project={project} onGoToSettings={onGoToSettings} />}</div>;
}

function HistoryPage({ history, commitDetail, fileDiff, busy, onSelectCommit, onShowDiff }: { history: HistoryEntry[]; commitDetail: CommitDetail | null; fileDiff: FileDiff | null; busy: boolean; onSelectCommit: (entry: HistoryEntry) => void; onShowDiff: (path: string) => void }) {
  return <div className="grid gap-5 xl:grid-cols-[0.8fr_1.2fr]"><Card><CardHeader><h3 className="font-semibold">保存履歴</h3><p className="mt-1 text-xs text-slate-500">過去の保存を選ぶと、変更内容を確認できます。</p></CardHeader><CardContent>{history.length ? <div className="space-y-2">{history.map((entry) => <button type="button" key={entry.commitId} onClick={() => onSelectCommit(entry)} disabled={busy} className={`w-full rounded-xl px-3 py-3 text-left transition ${commitDetail?.commitId === entry.commitId ? "bg-slate-900 text-white" : "bg-slate-50 text-slate-700 hover:bg-slate-100"}`}><p className="truncate text-sm font-semibold">{entry.memo}</p><p className={`mt-1 text-xs ${commitDetail?.commitId === entry.commitId ? "text-slate-300" : "text-slate-500"}`}>{entry.shortCommitId} · {entry.authorTime}</p></button>)}</div> : <p className="py-6 text-center text-sm text-slate-500">まだ保存履歴はありません。</p>}</CardContent></Card><div className="space-y-5">{commitDetail ? <Card><CardHeader><h3 className="font-semibold">保存の詳細</h3></CardHeader><CardContent><p className="text-lg font-semibold">{commitDetail.memo}</p><p className="mt-1 break-all text-xs text-slate-500">{commitDetail.commitId} · {commitDetail.authorTime}</p><div className="mt-5 space-y-2">{commitDetail.files.map((file) => <button type="button" onClick={() => onShowDiff(file.path)} disabled={busy} key={`${file.path}-${file.oldPath ?? ""}`} className="flex w-full items-center justify-between gap-3 rounded-xl bg-slate-50 px-3 py-2 text-left text-sm hover:bg-slate-100"><span className="min-w-0 truncate">{file.path}{file.oldPath ? ` ← ${file.oldPath}` : ""}</span><span className="shrink-0 text-xs text-slate-500">{changeKindLabel(file.changeKind)}</span></button>)}</div><p className="mt-5 rounded-xl bg-slate-50 px-3 py-2 text-xs text-slate-500">安全な復元はM4でこの画面から開始します。履歴を選択しただけでは現在の作業は変わりません。</p></CardContent></Card> : <Card><CardContent><p className="py-8 text-center text-sm text-slate-500">左から保存を選択してください。</p></CardContent></Card>}{fileDiff && <DiffPanel diff={fileDiff} />}</div></div>;
}

function RepositorySettingsPage({ project, settings, initializationPreview, busy, onPreviewInitialization, onApplyInitialization, onCancelInitialization, onOpenGlobalDefaults }: { project: ProjectDiagnostic; settings: SettingsLoadResult | null; initializationPreview: RepositoryInitializationPreview | null; busy: boolean; onPreviewInitialization: () => void; onApplyInitialization: () => void; onCancelInitialization: () => void; onOpenGlobalDefaults: () => void }) {
  return <div className="space-y-5"><Card><CardHeader><h3 className="font-semibold">このrepositoryの設定</h3><p className="mt-1 text-xs text-slate-500">この画面で設定を確認してもrepositoryのファイルは変更されません。</p></CardHeader><CardContent className="space-y-5"><div className="rounded-xl bg-slate-50 p-4"><div className="flex flex-wrap items-center justify-between gap-3"><div><p className="font-semibold">VPM packageのGit管理</p><p className="mt-1 text-sm text-slate-600">現在は全体設定の既定値を使用しています。repositoryごとの上書きは次の実装段階で追加します。</p></div><StatusPill label={settings?.settings.vpmTrackingPolicy === "INCLUDE_PACKAGES" ? "含める" : "除外する"} tone="neutral" /></div><Button className="mt-3" variant="secondary" onClick={onOpenGlobalDefaults}>全体の既定値を開く</Button></div><div className="grid gap-3 md:grid-cols-2"><DiagnosticItem label=".gitignore" status={project.sourceControl.gitignore.status} summary={project.sourceControl.gitignore.summary} /><DiagnosticItem label="VPM packages" status={project.sourceControl.vpmPackages.status} summary={project.sourceControl.vpmPackages.summary} /></div></CardContent></Card><Card><CardHeader><h3 className="font-semibold">project情報</h3></CardHeader><CardContent><dl className="grid gap-x-6 gap-y-4 text-sm md:grid-cols-2"><Definition label="project folder" value={project.path} /><Definition label="種別" value={projectKindLabel(project.projectKind)} /><Definition label="Unity" value={project.unityVersion ? `Unity ${project.unityVersion}` : "不明"} /><Definition label="repository" value={project.repository.detected ? "検出済み" : "未作成"} /></dl></CardContent></Card>{project.isUnityProject && !project.repository.detected && <RepositorySetup project={project} preview={initializationPreview} busy={busy} onPreview={onPreviewInitialization} onApply={onApplyInitialization} onCancel={onCancelInitialization} />}</div>;
}

function GlobalSettingsPage({ section, environment, settings, busy, onChangeSection, onUpdateVpm, onUpdateLogLevel, onOpenLogs, onOpenLogFolder, onExportLog }: { section: GlobalSettingsSection; environment: EnvironmentDiagnostic | null; settings: SettingsLoadResult | null; busy: boolean; onChangeSection: (section: GlobalSettingsSection) => void; onUpdateVpm: (policy: VpmTrackingPolicy) => void; onUpdateLogLevel: (level: string) => void; onOpenLogs: () => void; onOpenLogFolder: () => void; onExportLog: () => void }) {
  return <div className="grid gap-5 lg:grid-cols-[13rem_1fr]"><Card className="h-fit"><CardContent className="space-y-1">{GLOBAL_SETTINGS_SECTIONS.map((item) => <button key={item.value} type="button" onClick={() => onChangeSection(item.value)} className={`w-full rounded-xl px-3 py-2 text-left text-sm font-semibold ${section === item.value ? "bg-slate-900 text-white" : "text-slate-600 hover:bg-slate-100"}`}>{item.label}</button>)}</CardContent></Card><div>{section === "GENERAL" && <GeneralSettings settings={settings} />}{section === "DEFAULTS" && <DefaultSettings settings={settings} busy={busy} onUpdateVpm={onUpdateVpm} />}{section === "ENVIRONMENT" && <EnvironmentSettings environment={environment} />}{section === "LOGGING" && <LoggingSettings settings={settings} busy={busy} onUpdateLogLevel={onUpdateLogLevel} onOpenLogs={onOpenLogs} onOpenLogFolder={onOpenLogFolder} onExportLog={onExportLog} />}</div></div>;
}

function GeneralSettings({ settings }: { settings: SettingsLoadResult | null }) { return <Card><CardHeader><h3 className="font-semibold">一般</h3></CardHeader><CardContent><p className="text-sm text-slate-600">登録済みprojectはホームから選択します。存在しなくなったprojectはホームで「再指定」と表示されます。</p><p className="mt-4 text-xs text-slate-400">登録数: {settings?.recentProjects.length ?? 0} 件</p></CardContent></Card>; }
function DefaultSettings({ settings, busy, onUpdateVpm }: { settings: SettingsLoadResult | null; busy: boolean; onUpdateVpm: (policy: VpmTrackingPolicy) => void }) { const policy = settings?.settings.vpmTrackingPolicy ?? "EXCLUDE_PACKAGES"; return <div className="space-y-5"><Card><CardHeader><h3 className="font-semibold">新規repositoryの既定値</h3><p className="mt-1 text-xs text-slate-500">既存repositoryには自動適用しません。</p></CardHeader><CardContent><p className="text-sm font-semibold">VPM packageのGit管理</p><p className="mt-1 text-sm text-slate-600">新しく選択したprojectの診断と初期化previewに使う既定値です。</p><div className="mt-4 flex gap-2"><Button variant={policy === "EXCLUDE_PACKAGES" ? "primary" : "secondary"} onClick={() => onUpdateVpm("EXCLUDE_PACKAGES")} disabled={busy}>除外する</Button><Button variant={policy === "INCLUDE_PACKAGES" ? "primary" : "secondary"} onClick={() => onUpdateVpm("INCLUDE_PACKAGES")} disabled={busy}>含める</Button></div></CardContent></Card><Card><CardHeader><h3 className="font-semibold">ignore template</h3></CardHeader><CardContent><p className="text-sm text-slate-600">Unity用とVPM用のtemplateは現在の `settings.json` で直接編集できます。専用エディタはrepository固有設定の実装と合わせて追加します。</p><p className="mt-3 text-xs text-slate-400">Unity rule: {settings?.settings.ignoreTemplates.unityRules.length ?? 0} 件 · VPM rule: {settings?.settings.ignoreTemplates.vpmExcludeRules.length ?? 0} 件</p></CardContent></Card></div>; }
function EnvironmentSettings({ environment }: { environment: EnvironmentDiagnostic | null }) { return <div className="grid gap-5 md:grid-cols-2"><SummaryTile label="実行環境" value={environment ? `${environment.platform.os} / ${environment.platform.architecture}` : "確認中"} detail="正式対応: Windows / Apple Silicon macOS" tone={environment?.platform.supported ? "good" : "warn"} /><SummaryTile label="System Git" value={environment?.git.status === "AVAILABLE" ? "利用可能" : "未検出"} detail={environment?.git.version ?? "PATHから検出します"} tone={environment?.git.status === "AVAILABLE" ? "good" : "warn"} /></div>; }
function LoggingSettings({ settings, busy, onUpdateLogLevel, onOpenLogs, onOpenLogFolder, onExportLog }: { settings: SettingsLoadResult | null; busy: boolean; onUpdateLogLevel: (level: string) => void; onOpenLogs: () => void; onOpenLogFolder: () => void; onExportLog: () => void }) { return <Card><CardHeader><h3 className="font-semibold">ログと診断</h3><p className="mt-1 text-xs text-slate-500">ログレベルの変更は即時適用され、次回起動後も保持されます。</p></CardHeader><CardContent><label className="text-sm font-semibold" htmlFor="log-level">ログレベル</label><select id="log-level" className="mt-2 block rounded-xl border border-slate-300 bg-white px-3 py-2 text-sm" value={settings?.settings.logLevel ?? "INFO"} onChange={(event) => onUpdateLogLevel(event.target.value)} disabled={busy || !settings}>{LOG_LEVEL_OPTIONS.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}</select><p className="mt-3 text-sm leading-6 text-slate-600">ログ表示では、30日保持の対象となるサニタイズ済みログをすべて表示します。</p><div className="mt-5 flex flex-wrap gap-2"><Button variant="secondary" onClick={onOpenLogs} disabled={busy}>ログ表示</Button><Button variant="secondary" onClick={onOpenLogFolder} disabled={busy}>ログフォルダ</Button><Button onClick={onExportLog} disabled={busy}>診断ログを書き出す</Button></div></CardContent></Card>; }

function SummaryTile({ label, value, detail, tone = "neutral" }: { label: string; value: string; detail: string; tone?: "good" | "warn" | "neutral" }) { return <Card><CardContent><p className="text-xs font-bold uppercase tracking-[0.14em] text-slate-400">{label}</p><div className="mt-3 flex items-center justify-between gap-2"><p className="font-semibold text-slate-800">{value}</p><StatusPill label={tone === "good" ? "OK" : tone === "warn" ? "確認" : "情報"} tone={tone} /></div><p className="mt-2 text-xs leading-5 text-slate-500">{detail}</p></CardContent></Card>; }
function ChangedFiles({ files, onSelect }: { files: WorktreeSnapshot["files"]; onSelect: (path: string) => void }) { return files.length ? <div className="mt-4 space-y-2">{files.map((file) => <button type="button" onClick={() => onSelect(file.path)} key={`${file.path}-${file.oldPath ?? ""}`} className="flex w-full items-center justify-between gap-3 rounded-xl bg-slate-50 px-3 py-2 text-left text-sm transition hover:bg-slate-100"><span className="min-w-0 truncate text-slate-700" title={file.path}>{file.path}{file.oldPath ? ` ← ${file.oldPath}` : ""}</span><span className="shrink-0 text-xs text-slate-500">{changeKindLabel(file.changeKind)}{file.outsideProject ? " · project外" : ""}</span></button>)}</div> : <p className="mt-4 rounded-xl bg-slate-50 px-3 py-3 text-sm text-slate-500">保存対象の変更はありません。</p>; }
function DiagnosticSummary({ project, onGoToSettings }: { project: ProjectDiagnostic; onGoToSettings: () => void }) { return <Card><CardHeader><div className="flex items-center justify-between gap-3"><div><h3 className="font-semibold">projectの診断</h3><p className="mt-1 text-xs text-slate-500">詳細な設定とignore ruleはリポジトリ設定で確認できます。</p></div><Button variant="secondary" onClick={onGoToSettings}>リポジトリ設定</Button></div></CardHeader><CardContent>{project.issues.length ? <div className="space-y-2">{project.issues.map((issue, index) => <div key={`${issue.code}-${index}`} className={issueClass(issue.severity)}><p className="text-sm font-medium">{issue.message}</p><p className="mt-1 text-xs opacity-70">{issue.code}{shouldShowIssuePath(issue) ? ` · ${issue.path}` : ""}</p></div>)}</div> : <p className="rounded-xl bg-emerald-50 px-4 py-3 text-sm text-emerald-800">現在の診断範囲では、修正が必要な状態は見つかりませんでした。</p>}</CardContent></Card>; }
function DiffPanel({ diff }: { diff: FileDiff }) { return <Card><CardHeader><h3 className="truncate font-semibold" title={diff.path}>{diff.path}</h3><p className="mt-1 text-xs text-slate-500">{diff.kind === "TEXT" ? "テキスト差分" : diff.kind === "BINARY" ? "バイナリファイル" : "表示不可"}</p></CardHeader><CardContent>{diff.patch ? <pre className="max-h-[32rem] overflow-auto whitespace-pre-wrap rounded-xl bg-slate-950 p-4 text-xs leading-5 text-slate-100">{diff.patch}</pre> : <p className="rounded-xl bg-slate-50 px-3 py-3 text-sm text-slate-600">{diff.truncationReason ?? "テキストとして表示できません。"}</p>}{diff.truncated && <p className="mt-3 text-xs text-amber-700">{diff.truncationReason}</p>}</CardContent></Card>; }
function BlockingNotice({ children, tone = "warn" }: { children: string; tone?: "warn" | "danger" }) { return <p className={`mt-4 rounded-xl px-3 py-2 text-sm ${tone === "danger" ? "bg-rose-50 text-rose-800" : "bg-amber-50 text-amber-800"}`}>{children}</p>; }
function DiagnosticItem({ label, status, summary }: { label: string; status: ProjectDiagnostic["sourceControl"]["gitignore"]["status"]; summary: string }) { const tone = status === "HEALTHY" ? "border-emerald-200 bg-emerald-50" : status === "NOT_APPLICABLE" ? "border-slate-200 bg-slate-50" : "border-amber-200 bg-amber-50"; return <div className={`rounded-xl border px-3 py-3 ${tone}`}><p className="text-xs font-semibold text-slate-700">{label}</p><p className="mt-1 text-xs leading-5 text-slate-600">{summary}</p></div>; }
function Definition({ label, value }: { label: string; value: string }) { return <div><dt className="text-xs font-semibold text-slate-400">{label}</dt><dd className="mt-1 break-all text-slate-700">{value}</dd></div>; }
function ErrorNotice({ error, onDismiss }: { error: AppError; onDismiss: () => void }) { return <div className="mb-5 flex items-start justify-between gap-4 rounded-2xl border border-rose-200 bg-rose-50 px-5 py-4 text-sm text-rose-800"><div><p className="font-semibold">{error.message}</p><p className="mt-1 text-xs text-rose-700">{error.code}{error.mayHaveMutated ? " · 状態が変化した可能性があります" : ""}</p></div><button className="text-xs font-semibold underline" onClick={onDismiss}>閉じる</button></div>; }
function RecoveryNotice({ settings }: { settings: SettingsLoadResult }) { return <div className="mb-5 rounded-2xl border border-amber-200 bg-amber-50 px-5 py-4 text-sm text-amber-900">settings.json を保全して初期設定を再生成しました。{settings.backupPath ? ` 退避先: ${settings.backupPath}` : ""}</div>; }

function currentWindowLabel() { try { return getCurrentWindow().label; } catch { return undefined; } }
function normalizeError(error: unknown): AppError { if (isAppError(error)) return error; return { code: "INTERNAL_ERROR", message: "処理に失敗しました。Vsedi のログを確認してください。", technicalDetail: null, operation: null, mayHaveMutated: false }; }
function projectName(path: string) { const parts = path.split(/[\\/]/).filter(Boolean); return parts.at(-1) ?? path; }
function formatDate(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString("ja-JP"); }
function compareManagedProjects(left: SettingsLoadResult["recentProjects"][number], right: SettingsLoadResult["recentProjects"][number]) { return (right.lastOpenedAt ?? "").localeCompare(left.lastOpenedAt ?? "") || left.path.localeCompare(right.path, "ja"); }
function projectStatusLabel(status: ProjectDiagnostic["status"]) { return status === "MANAGEABLE" ? "管理可能" : status === "NEEDS_ATTENTION" ? "要確認" : "非 Unity"; }
function projectKindLabel(kind: ProjectDiagnostic["projectKind"]) { const labels: Record<ProjectDiagnostic["projectKind"], string> = { UNITY: "Unity project", VRCHAT_AVATAR: "VRChat Avatar", VRCHAT_WORLD: "VRChat World", VRCHAT_UNKNOWN: "VRChat 種別不明" }; return labels[kind]; }
function changeKindLabel(kind: WorktreeSnapshot["files"][number]["changeKind"]) { const labels: Record<WorktreeSnapshot["files"][number]["changeKind"], string> = { ADDED: "追加", MODIFIED: "変更", DELETED: "削除", RENAMED: "名前変更", COPIED: "複製", TYPE_CHANGED: "種類変更", UNMERGED: "競合", UNTRACKED: "未管理" }; return labels[kind]; }
function issueClass(severity: ProjectDiagnostic["issues"][number]["severity"]) { return severity === "ERROR" ? "rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-rose-800" : severity === "WARNING" ? "rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-amber-900" : "rounded-xl border border-sky-200 bg-sky-50 px-4 py-3 text-sky-800"; }
function shouldShowIssuePath(issue: ProjectDiagnostic["issues"][number]) { return issue.code !== "GIT_ROOT_OUTSIDE_PROJECT" && Boolean(issue.path); }

const GLOBAL_SETTINGS_SECTIONS: { value: GlobalSettingsSection; label: string }[] = [{ value: "GENERAL", label: "一般" }, { value: "DEFAULTS", label: "新規repositoryの既定値" }, { value: "ENVIRONMENT", label: "実行環境" }, { value: "LOGGING", label: "ログと診断" }];
const LOG_LEVEL_OPTIONS = [{ value: "ERROR", label: "ERROR — エラーのみ" }, { value: "WARN", label: "WARN — 警告以上" }, { value: "INFO", label: "INFO — 通常" }, { value: "DEBUG", label: "DEBUG — 詳細" }, { value: "TRACE", label: "TRACE — 最詳細" }] as const;

export default App;
