import { useEffect, useMemo, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { StatusPill } from "@/components/ui/status-pill";
import { LogWindow } from "@/LogWindow";
import type { AppError, EnvironmentDiagnostic, ProjectDiagnostic, SettingsLoadResult, VpmTrackingPolicy } from "@/generated/bindings";
import { exportDiagnosticLog, inspectEnvironment, inspectProject, isAppError, loadSettings, openLogDirectory, openLogWindow, saveSettings } from "@/lib/commands";

function App() {
  if (currentWindowLabel() === "logs") return <LogWindow />;
  return <MainWindow />;
}

function MainWindow() {
  const [environment, setEnvironment] = useState<EnvironmentDiagnostic | null>(null);
  const [settings, setSettings] = useState<SettingsLoadResult | null>(null);
  const [project, setProject] = useState<ProjectDiagnostic | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AppError | null>(null);

  const refresh = async () => {
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
  };

  useEffect(() => {
    void refresh();
  }, []);

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
                          <p className="mt-1 text-xs opacity-70">{issue.code}{issue.path ? ` · ${issue.path}` : ""}</p>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">
                      現在の診断範囲では、修正が必要な状態は見つかりませんでした。
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

export default App;
