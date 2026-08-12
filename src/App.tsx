import { useEffect, useMemo, useRef, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { StatusPill } from "@/components/ui/status-pill";
import { LogWindow } from "@/LogWindow";
import type { AppError, CommitDetail, EnvironmentDiagnostic, FileDiff, HistoryEntry, ProjectDiagnostic, RepositoryInitializationPreview, RepositoryState, SaveResult, SettingsLoadResult, VpmTrackingPolicy, WorktreeSnapshot } from "@/generated/bindings";
import { exportDiagnosticLog, initializeRepository, inspectEnvironment, inspectProject, isAppError, loadSettings, openLogDirectory, openLogWindow, previewRepositoryInitialization, readCommitDetail, readCommitDiff, readHistory, readRepositoryState, readWorktreeDiff, readWorktreeSnapshot, saveSettings, saveWorktree } from "@/lib/commands";

function App() {
  if (currentWindowLabel() === "logs") return <LogWindow />;
  return <MainWindow />;
}

function MainWindow() {
  const [environment, setEnvironment] = useState<EnvironmentDiagnostic | null>(null);
  const [settings, setSettings] = useState<SettingsLoadResult | null>(null);
  const [project, setProject] = useState<ProjectDiagnostic | null>(null);
  const [repositoryState, setRepositoryState] = useState<RepositoryState | null>(null);
  const [worktree, setWorktree] = useState<WorktreeSnapshot | null>(null);
  const [initializationPreview, setInitializationPreview] = useState<RepositoryInitializationPreview | null>(null);
  const [saveMemo, setSaveMemo] = useState("");
  const [saveResult, setSaveResult] = useState<SaveResult | null>(null);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [commitDetail, setCommitDetail] = useState<CommitDetail | null>(null);
  const [fileDiff, setFileDiff] = useState<FileDiff | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  const refreshInFlight = useRef<Promise<void> | null>(null);

  const refresh = () => {
    if (refreshInFlight.current) return refreshInFlight.current;
    const task = (async () => {
      setBusy(true);
      setError(null);
      try {
        const [environmentResult, settingsResult] = await Promise.all([inspectEnvironment(), loadSettings()]);
        setEnvironment(environmentResult);
        setSettings(settingsResult);
        if (settingsResult.recentProjects[0]?.exists) {
          setProject(await inspectProject(settingsResult.recentProjects[0].path, settingsResult.settings.vpmTrackingPolicy));
        }
      } catch (caught) {
        setError(normalizeError(caught));
      } finally {
        setBusy(false);
      }
    })();
    refreshInFlight.current = task.finally(() => {
      refreshInFlight.current = null;
    });
    return refreshInFlight.current;
  };

  useEffect(() => {
    void refresh();
  }, []);

  useEffect(() => {
    if (!project?.repository.detected) {
      setRepositoryState(null);
      setWorktree(null);
      setHistory([]);
      setCommitDetail(null);
      setFileDiff(null);
      return;
    }
    void (async () => {
      try {
        const [state, snapshot, historyEntries] = await Promise.all([readRepositoryState(project.path), readWorktreeSnapshot(project.path), readHistory(project.path)]);
        setRepositoryState(state);
        setWorktree(snapshot);
        setHistory(historyEntries);
      } catch (caught) {
        setError(normalizeError(caught));
      }
    })();
  }, [project?.path, project?.repository.detected]);

  const gitTone = environment?.git.status === "AVAILABLE" ? "good" : "warn";
  const chooseProject = async () => {
    setBusy(true);
    setError(null);
    try {
      const selected = await open({ directory: true, multiple: false, title: "Vsedi project folder を選択" });
      if (typeof selected !== "string" || !settings) return;
      const result = await inspectProject(selected, settings.settings.vpmTrackingPolicy);
      setProject(result);
      const nextSettings = {
        ...settings.settings,
        recentProjects: [{ path: result.path, lastOpenedAt: new Date().toISOString() }, ...settings.settings.recentProjects.filter((item) => item.path !== result.path)].slice(0, 10),
      };
      await saveSettings(nextSettings);
      setSettings({ ...settings, settings: nextSettings, recentProjects: [{ ...nextSettings.recentProjects[0], exists: true }, ...settings.recentProjects.filter((item) => item.path !== result.path)].slice(0, 10) });
    } catch (caught) {
      setError(normalizeError(caught));
    } finally {
      setBusy(false);
    }
  };

  const updateVpmTrackingPolicy = async (policy: VpmTrackingPolicy) => {
    if (!settings || settings.settings.vpmTrackingPolicy === policy) return;
    setBusy(true);
    setError(null);
    try {
      const nextSettings = { ...settings.settings, vpmTrackingPolicy: policy };
      await saveSettings(nextSettings);
      setSettings({ ...settings, settings: nextSettings });
      if (project) setProject(await inspectProject(project.path, policy));
    } catch (caught) {
      setError(normalizeError(caught));
    } finally {
      setBusy(false);
    }
  };

  const updateLogLevel = async (logLevel: string) => {
    if (!settings || settings.settings.logLevel === logLevel) return;
    setBusy(true);
    setError(null);
    try {
      const nextSettings = { ...settings.settings, logLevel };
      await saveSettings(nextSettings);
      setSettings({ ...settings, settings: nextSettings });
    } catch (caught) {
      setError(normalizeError(caught));
    } finally {
      setBusy(false);
    }
  };

  const previewInitialization = async () => {
    if (!project || !settings) return;
    setBusy(true); setError(null);
    try { setInitializationPreview(await previewRepositoryInitialization(project.path)); }
    catch (caught) { setError(normalizeError(caught)); }
    finally { setBusy(false); }
  };

  const applyInitialization = async () => {
    if (!project || !settings || !initializationPreview) return;
    setBusy(true); setError(null);
    try {
      await initializeRepository({ projectPath: project.path, statusToken: initializationPreview.statusToken });
      setInitializationPreview(null);
      setProject(await inspectProject(project.path, settings.settings.vpmTrackingPolicy));
    } catch (caught) { setError(normalizeError(caught)); }
    finally { setBusy(false); }
  };

  const saveCurrentWork = async () => {
    if (!project || !worktree) return;
    setBusy(true); setError(null); setSaveResult(null);
    try {
      const result = await saveWorktree({ projectPath: project.path, statusToken: worktree.statusToken, memo: saveMemo });
      setSaveResult(result); setSaveMemo("");
      const [state, snapshot, historyEntries] = await Promise.all([readRepositoryState(project.path), readWorktreeSnapshot(project.path), readHistory(project.path)]);
      setRepositoryState(state); setWorktree(snapshot);
      setHistory(historyEntries);
    } catch (caught) { setError(normalizeError(caught)); }
    finally { setBusy(false); }
  };

  const selectCommit = async (entry: HistoryEntry) => {
    if (!project) return;
    try { setCommitDetail(await readCommitDetail(project.path, entry.commitId)); }
    catch (caught) { setError(normalizeError(caught)); }
  };

  const showCommitDiff = async (path: string) => {
    if (!project || !commitDetail) return;
    try { setFileDiff(await readCommitDiff(project.path, commitDetail.commitId, path)); }
    catch (caught) { setError(normalizeError(caught)); }
  };

  const showWorktreeDiff = async (path: string) => {
    if (!project) return;
    try { setFileDiff(await readWorktreeDiff(project.path, path)); }
    catch (caught) { setError(normalizeError(caught)); }
  };

  const exportLog = async () => {
    try {
      const destination = await save({ defaultPath: "vsedi-diagnostic.log", title: "診断ログを書き出す" });
      if (destination) await exportDiagnosticLog(destination);
    } catch (caught) {
      setError(normalizeError(caught));
    }
  };

  const showLogDirectory = async () => {
    try {
      await openLogDirectory();
    } catch (caught) {
      setError(normalizeError(caught));
    }
  };

  const showLogWindow = async () => {
    try {
      await openLogWindow();
    } catch (caught) {
      setError(normalizeError(caught));
    }
  };

  const platformLabel = useMemo(() => {
    if (!environment) return "確認中";
    return `${environment.platform.os} / ${environment.platform.architecture}`;
  }, [environment]);

  return (
    <main className="min-h-screen bg-mist px-8 py-7 text-ink">
      <div className="mx-auto max-w-6xl">
        <header className="mb-8 flex items-start justify-between gap-6">
          <div>
            <p className="mb-2 text-xs font-bold uppercase tracking-[0.28em] text-accent">Local first · Safety over power</p>
            <h1 className="text-4xl font-bold tracking-tight">Vsedi</h1>
            <p className="mt-2 max-w-xl text-sm leading-6 text-slate-600">Unity / VRChat project の状態を確認し、作業を安全に保存するためのローカルデスクトップ基盤。</p>
          </div>
          <div className="flex gap-2">
            <Button variant="ghost" onClick={() => void showLogWindow()}>ログ表示</Button>
            <Button variant="ghost" onClick={() => void showLogDirectory()}>ログフォルダ</Button>
            <Button variant="secondary" onClick={() => void exportLog()}>診断ログを書き出す</Button>
          </div>
        </header>

        {error && (
          <div className="mb-6 rounded-2xl border border-rose-200 bg-rose-50 px-5 py-4 text-sm text-rose-800">
            <div className="font-semibold">{error.message}</div>
            <div className="mt-1 text-xs text-rose-700">{error.code}{error.mayHaveMutated ? " · 状態が変化した可能性があります" : ""}</div>
          </div>
        )}

        {settings?.recovered && (
          <div className="mb-6 rounded-2xl border border-amber-200 bg-amber-50 px-5 py-4 text-sm text-amber-900">
            settings.json を保全して初期設定を再生成しました。{settings.backupPath ? ` 退避先: ${settings.backupPath}` : ""}
          </div>
        )}

        <section className="grid gap-5 md:grid-cols-2">
          <Card>
            <CardHeader><div className="flex items-center justify-between"><h2 className="font-semibold">実行環境</h2><StatusPill label={environment?.platform.supported ? "対応対象" : "確認が必要"} tone={environment?.platform.supported ? "good" : "warn"} /></div></CardHeader>
            <CardContent><p className="text-sm text-slate-600">{platformLabel}</p><p className="mt-2 text-xs text-slate-400">正式対応: Windows / Apple Silicon macOS</p></CardContent>
          </Card>
          <Card>
            <CardHeader><div className="flex items-center justify-between"><h2 className="font-semibold">System Git</h2><StatusPill label={environment?.git.status === "AVAILABLE" ? "利用可能" : "未検出"} tone={gitTone} /></div></CardHeader>
            <CardContent><p className="text-sm text-slate-600">{environment?.git.version ?? "Git を診断中"}</p><p className="mt-2 truncate text-xs text-slate-400">{environment?.git.executable ?? "PATH から検出します"}</p></CardContent>
          </Card>
        </section>

        <section className="mt-5">
          <Card>
            <CardHeader><div className="flex items-center justify-between gap-4"><div><h2 className="font-semibold">ログ設定</h2><p className="mt-1 text-xs text-slate-500">変更は即時適用され、次回起動後も保持されます。</p></div><StatusPill label={settings?.settings.logLevel ?? "INFO"} tone="neutral" /></div></CardHeader>
            <CardContent className="flex flex-wrap items-center gap-4">
              <label className="text-sm text-slate-700" htmlFor="log-level">ログレベル</label>
              <select id="log-level" className="rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm" value={settings?.settings.logLevel ?? "INFO"} onChange={(event) => void updateLogLevel(event.target.value)} disabled={busy || !settings}>
                {LOG_LEVEL_OPTIONS.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
              </select>
              <p className="text-xs text-slate-500">TRACEを選ぶと、現時点で取得できる最も詳細なログを記録します。ログ表示では保持期間内のログをすべて表示します。</p>
            </CardContent>
          </Card>
        </section>

        <section className="mt-5 grid gap-5 lg:grid-cols-[1.4fr_0.9fr]">
          <Card>
            <CardHeader><div className="flex items-center justify-between"><div><p className="text-xs font-bold uppercase tracking-[0.18em] text-slate-400">Project boundary</p><h2 className="mt-1 text-xl font-semibold">管理する project</h2></div><Button onClick={() => void chooseProject()} disabled={busy}>フォルダを選択</Button></div></CardHeader>
            <CardContent>
              {project ? (
                <div className="space-y-5">
                  <div className="rounded-xl bg-slate-50 px-4 py-3">
                    <p className="break-all text-sm font-medium">{project.path}</p>
                    <p className="mt-1 text-xs text-slate-500">
                      {project.unityVersion ? `Unity ${project.unityVersion}` : "Unity version 不明"}
                      {project.unityRevision ? ` · revision ${project.unityRevision}` : ""}
                    </p>
                  </div>

                  <div className="flex flex-wrap gap-2">
                    <StatusPill label={projectStatusLabel(project.status)} tone={projectStatusTone(project.status)} />
                    <StatusPill label={projectKindLabel(project.projectKind)} tone={project.projectKind === "UNITY" ? "neutral" : "good"} />
                  </div>

                  {settings && (
                    <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-slate-200 bg-slate-50 px-4 py-3">
                      <div>
                        <p className="text-sm font-semibold text-slate-700">VPM packageのGit管理</p>
                        <p className="mt-1 text-xs text-slate-500">すべてのprojectに適用する診断方針です。</p>
                      </div>
                      <div className="flex gap-2">
                        <Button variant={settings.settings.vpmTrackingPolicy === "EXCLUDE_PACKAGES" ? "primary" : "secondary"} onClick={() => void updateVpmTrackingPolicy("EXCLUDE_PACKAGES")} disabled={busy}>除外する</Button>
                        <Button variant={settings.settings.vpmTrackingPolicy === "INCLUDE_PACKAGES" ? "primary" : "secondary"} onClick={() => void updateVpmTrackingPolicy("INCLUDE_PACKAGES")} disabled={busy}>含める</Button>
                      </div>
                    </div>
                  )}

                  {project.isUnityProject && (
                    <div className="grid gap-3 sm:grid-cols-2">
                      <DiagnosticItem label=".gitignore" status={project.sourceControl.gitignore.status} summary={project.sourceControl.gitignore.summary} />
                      <DiagnosticItem label="VPM packages" status={project.sourceControl.vpmPackages.status} summary={project.sourceControl.vpmPackages.summary} />
                    </div>
                  )}

                  {project.vpm.packages.length > 0 && (
                    <div>
                      <p className="text-xs font-semibold uppercase tracking-[0.14em] text-slate-400">検出 package</p>
                      <div className="mt-2 flex flex-wrap gap-2">
                        {project.vpm.packages.filter((item) => item.name.startsWith("com.vrchat.")).map((item) => (
                          <span key={item.name} className="rounded-lg bg-sky-50 px-2.5 py-1 text-xs text-sky-800">
                            {item.name}{item.version ? ` ${item.version}` : ""}
                          </span>
                        ))}
                      </div>
                    </div>
                  )}

                  {project.issues.length > 0 ? (
                    <div className="space-y-2">
                      <p className="text-xs font-semibold uppercase tracking-[0.14em] text-slate-400">診断結果</p>
                      {project.issues.map((issue, index) => (
                        <div key={`${issue.code}-${index}`} className={issueClass(issue.severity)}>
                          <p className="text-sm font-medium">{issue.message}</p>
                          <p className="mt-1 text-xs opacity-70">{issue.code}{shouldShowIssuePath(issue) ? ` · ${issue.path}` : ""}</p>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">
                      現在の診断範囲では、修正が必要な状態は見つかりませんでした。
                    </div>
                  )}

                  {project.repository.detected && (
                    <div className="rounded-xl border border-slate-200 bg-white px-4 py-4">
                      <div className="flex items-center justify-between gap-3">
                        <div>
                          <p className="text-sm font-semibold text-slate-700">現在の変更</p>
                          <p className="mt-1 text-xs text-slate-500">保存対象は repository 全体です。変更内容を確認してから保存できます。</p>
                        </div>
                        <StatusPill label={repositoryState?.canSave ? "保存可能" : repositoryState ? "確認が必要" : "読み込み中"} tone={repositoryState?.canSave ? "good" : "warn"} />
                      </div>
                      {repositoryState?.blockingReason === "EXISTING_STAGED_CHANGES" && <p className="mt-3 text-xs text-amber-800">すでに Git のステージにある変更があるため、安全のため保存を開始できません。</p>}
                      {repositoryState?.blockingReason === "CONFLICT" && <p className="mt-3 text-xs text-rose-800">競合中のファイルがあるため、保存を開始できません。</p>}
                      {worktree && <div className="mt-3 space-y-2">{worktree.files.length ? worktree.files.map((file) => <button type="button" onClick={() => void showWorktreeDiff(file.path)} key={`${file.path}-${file.oldPath ?? ""}`} className="flex w-full items-center justify-between gap-3 rounded-lg bg-slate-50 px-3 py-2 text-left text-xs hover:bg-slate-100"><span className="min-w-0 truncate text-slate-700" title={file.path}>{file.path}{file.oldPath ? ` ← ${file.oldPath}` : ""}</span><span className="shrink-0 text-slate-500">{changeKindLabel(file.changeKind)}{file.outsideProject ? " · project外" : ""}</span></button>) : <p className="rounded-lg bg-slate-50 px-3 py-2 text-xs text-slate-500">保存対象の変更はありません。</p>}</div>}
                      {repositoryState?.canSave && worktree?.files.length ? <div className="mt-3 flex flex-wrap gap-2"><input className="min-w-56 flex-1 rounded-lg border border-slate-300 px-3 py-2 text-sm" value={saveMemo} onChange={(event) => setSaveMemo(event.target.value)} placeholder="保存メモ（例: アバターの表情を調整）" disabled={busy} /><Button onClick={() => void saveCurrentWork()} disabled={busy || !saveMemo.trim()}>作業を保存</Button></div> : null}
                      {saveResult && <p className="mt-3 rounded-lg bg-emerald-50 px-3 py-2 text-xs text-emerald-800">保存しました: {saveResult.shortCommitId} · {saveResult.fileCount} file · {saveResult.authorTime}</p>}
                    </div>
                  )}

                  {project.isUnityProject && project.repository.detected !== true && (
                    <div className="rounded-xl border border-sky-200 bg-sky-50 px-4 py-4">
                      <p className="text-sm font-semibold text-sky-900">ローカル保存を始める</p>
                      <p className="mt-1 text-xs leading-5 text-sky-800">Git repository と Unity 用の ignore rule を作成します。既存の .gitignore は置換せず、不足ルールだけを追記します。</p>
                      {!initializationPreview ? <Button className="mt-3" onClick={() => void previewInitialization()} disabled={busy}>作成内容を確認</Button> : (
                        <div className="mt-3 space-y-3 rounded-lg bg-white/80 p-3">
                          {initializationPreview.ignoreFiles.map((file) => <p key={file.path} className="text-xs text-slate-700"><span className="font-semibold">{file.path}</span>: {file.missingRules.length ? file.missingRules.join("、") : "変更なし"}{file.willCreate ? "（新規作成）" : ""}</p>)}
                          {initializationPreview.canInitialize ? <div className="flex gap-2"><Button onClick={() => void applyInitialization()} disabled={busy}>この内容で初期化</Button><Button variant="secondary" onClick={() => setInitializationPreview(null)} disabled={busy}>キャンセル</Button></div> : <p className="text-xs text-rose-800">{initializationPreview.blockingReason}</p>}
                        </div>
                      )}
                    </div>
                  )}

                  {project.repository.detected && (
                    <div className="rounded-xl border border-slate-200 bg-white px-4 py-4">
                      <p className="text-sm font-semibold text-slate-700">保存履歴</p>
                      {history.length ? <div className="mt-3 space-y-2">{history.map((entry) => <button type="button" key={entry.commitId} onClick={() => void selectCommit(entry)} className="flex w-full items-center justify-between gap-3 rounded-lg bg-slate-50 px-3 py-2 text-left text-xs hover:bg-slate-100"><span className="min-w-0 truncate text-slate-700">{entry.memo}</span><span className="shrink-0 text-slate-500">{entry.shortCommitId}</span></button>)}</div> : <p className="mt-2 text-xs text-slate-500">まだ保存履歴はありません。</p>}
                      {commitDetail && <div className="mt-3 rounded-lg border border-slate-200 p-3 text-xs"><p className="font-semibold text-slate-800">{commitDetail.memo}</p><p className="mt-1 text-slate-500">{commitDetail.commitId} · {commitDetail.authorTime}</p><div className="mt-2 space-y-1">{commitDetail.files.map((file) => <button type="button" onClick={() => void showCommitDiff(file.path)} key={`${file.path}-${file.oldPath ?? ""}`} className="block max-w-full truncate text-left text-slate-600 hover:text-accent">{changeKindLabel(file.changeKind)} · {file.path}{file.oldPath ? ` ← ${file.oldPath}` : ""}</button>)}</div></div>}
                      {fileDiff && <div className="mt-3 rounded-lg border border-slate-200 bg-slate-950 p-3 text-xs text-slate-100"><p className="mb-2 break-all text-slate-300">{fileDiff.path} · {fileDiff.kind === "TEXT" ? "text diff" : fileDiff.kind === "BINARY" ? "binary" : "表示不可"}</p>{fileDiff.patch ? <pre className="max-h-72 overflow-auto whitespace-pre-wrap">{fileDiff.patch}</pre> : <p className="text-slate-300">{fileDiff.truncationReason ?? "テキストとして表示できません。"}</p>}{fileDiff.truncated && <p className="mt-2 text-amber-300">{fileDiff.truncationReason}</p>}</div>}
                    </div>
                  )}
                </div>
              ) : (
                <div className="py-8 text-center text-sm text-slate-500">project folder を選択すると、Rust 側で Unity / VRChat / Git 設定を診断します。</div>
              )}
            </CardContent>
          </Card>
          <Card>
            <CardHeader><h2 className="font-semibold">最近の project</h2></CardHeader>
            <CardContent>{settings?.recentProjects.length ? <div className="space-y-3">{settings.recentProjects.map((item) => <div key={item.path} className="flex items-center justify-between gap-3 text-sm"><span className="truncate text-slate-600" title={item.path}>{item.path}</span><StatusPill label={item.exists ? "存在" : "再指定"} tone={item.exists ? "good" : "warn"} /></div>)}</div> : <p className="text-sm text-slate-500">まだ登録されていません。</p>}</CardContent>
          </Card>
        </section>

        <footer className="mt-8 flex items-center justify-between text-xs text-slate-400"><span>Rust command boundary · structured diagnostics</span><Button variant="ghost" onClick={() => void refresh()} disabled={busy}>{busy ? "確認中…" : "再診断"}</Button></footer>
      </div>

    </main>
  );
}

function currentWindowLabel() {
  try {
    return getCurrentWindow().label;
  } catch {
    return undefined;
  }
}

function normalizeError(error: unknown): AppError {
  if (isAppError(error)) return error;
  return { code: "INTERNAL_ERROR", message: "処理に失敗しました。Vsedi のログを確認してください。", technicalDetail: null, operation: null, mayHaveMutated: false };
}

function projectStatusLabel(status: ProjectDiagnostic["status"]) {
  if (status === "MANAGEABLE") return "管理可能";
  if (status === "NEEDS_ATTENTION") return "要修正";
  return "非 Unity";
}

function projectStatusTone(status: ProjectDiagnostic["status"]): "good" | "warn" | "neutral" {
  if (status === "MANAGEABLE") return "good";
  if (status === "NEEDS_ATTENTION") return "warn";
  return "neutral";
}

function projectKindLabel(kind: ProjectDiagnostic["projectKind"]) {
  const labels: Record<ProjectDiagnostic["projectKind"], string> = {
    UNITY: "Unity project",
    VRCHAT_AVATAR: "VRChat Avatar",
    VRCHAT_WORLD: "VRChat World",
    VRCHAT_UNKNOWN: "VRChat 種別不明",
  };
  return labels[kind];
}

function changeKindLabel(kind: WorktreeSnapshot["files"][number]["changeKind"]) {
  const labels: Record<WorktreeSnapshot["files"][number]["changeKind"], string> = {
    ADDED: "追加", MODIFIED: "変更", DELETED: "削除", RENAMED: "名前変更", COPIED: "複製", TYPE_CHANGED: "種類変更", UNMERGED: "競合", UNTRACKED: "未管理",
  };
  return labels[kind];
}

function DiagnosticItem({ label, status, summary }: { label: string; status: ProjectDiagnostic["sourceControl"]["gitignore"]["status"]; summary: string }) {
  const tone = status === "HEALTHY" ? "border-emerald-200 bg-emerald-50" : status === "NOT_APPLICABLE" ? "border-slate-200 bg-slate-50" : "border-amber-200 bg-amber-50";
  return (
    <div className={`rounded-xl border px-3 py-3 ${tone}`}>
      <p className="text-xs font-semibold text-slate-700">{label}</p>
      <p className="mt-1 text-xs leading-5 text-slate-600">{summary}</p>
    </div>
  );
}

function issueClass(severity: ProjectDiagnostic["issues"][number]["severity"]) {
  if (severity === "ERROR") return "rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-rose-800";
  if (severity === "WARNING") return "rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-amber-900";
  return "rounded-xl border border-sky-200 bg-sky-50 px-4 py-3 text-sky-800";
}

function shouldShowIssuePath(issue: ProjectDiagnostic["issues"][number]) {
  return issue.code !== "GIT_ROOT_OUTSIDE_PROJECT" && Boolean(issue.path);
}

const LOG_LEVEL_OPTIONS = [
  { value: "ERROR", label: "ERROR — エラーのみ" },
  { value: "WARN", label: "WARN — 警告以上" },
  { value: "INFO", label: "INFO — 通常" },
  { value: "DEBUG", label: "DEBUG — 詳細" },
  { value: "TRACE", label: "TRACE — 最詳細" },
] as const;

export default App;
