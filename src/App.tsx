import { useEffect, useMemo, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { StatusPill } from "@/components/ui/status-pill";
import type { AppError, EnvironmentDiagnostic, ProjectDiagnostic, SettingsLoadResult } from "@/generated/bindings";
import { exportDiagnosticLog, inspectEnvironment, inspectProject, isAppError, loadSettings, openLogDirectory, saveSettings } from "@/lib/commands";

function App() {
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
        setProject(await inspectProject(settingsResult.recentProjects[0].path));
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
  const lfsTone = environment?.git.lfs.status === "AVAILABLE" ? "good" : "warn";

  const chooseProject = async () => {
    setBusy(true);
    setError(null);
    try {
      const selected = await open({ directory: true, multiple: false, title: "Vsedi project folder を選択" });
      if (typeof selected !== "string" || !settings) return;
      const result = await inspectProject(selected);
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

        <section className="grid gap-5 md:grid-cols-3">
          <Card>
            <CardHeader><div className="flex items-center justify-between"><h2 className="font-semibold">実行環境</h2><StatusPill label={environment?.platform.supported ? "対応対象" : "確認が必要"} tone={environment?.platform.supported ? "good" : "warn"} /></div></CardHeader>
            <CardContent><p className="text-sm text-slate-600">{platformLabel}</p><p className="mt-2 text-xs text-slate-400">正式対応: Windows / Apple Silicon macOS</p></CardContent>
          </Card>
          <Card>
            <CardHeader><div className="flex items-center justify-between"><h2 className="font-semibold">System Git</h2><StatusPill label={environment?.git.status === "AVAILABLE" ? "利用可能" : "未検出"} tone={gitTone} /></div></CardHeader>
            <CardContent><p className="text-sm text-slate-600">{environment?.git.version ?? "Git を診断中"}</p><p className="mt-2 truncate text-xs text-slate-400">{environment?.git.executable ?? "PATH から検出します"}</p></CardContent>
          </Card>
          <Card>
            <CardHeader><div className="flex items-center justify-between"><h2 className="font-semibold">Git LFS</h2><StatusPill label={environment?.git.lfs.status === "AVAILABLE" ? "利用可能" : "未導入"} tone={lfsTone} /></div></CardHeader>
            <CardContent><p className="text-sm text-slate-600">{environment?.git.lfs.version ?? "検出済み Git から診断"}</p><p className="mt-2 text-xs text-slate-400">git-lfs executable は独立探索しません</p></CardContent>
          </Card>
        </section>

        <section className="mt-5 grid gap-5 lg:grid-cols-[1.4fr_0.9fr]">
          <Card>
            <CardHeader><div className="flex items-center justify-between"><div><p className="text-xs font-bold uppercase tracking-[0.18em] text-slate-400">Project boundary</p><h2 className="mt-1 text-xl font-semibold">管理する project</h2></div><Button onClick={() => void chooseProject()} disabled={busy}>フォルダを選択</Button></div></CardHeader>
            <CardContent>
              {project ? <div className="space-y-4"><div className="rounded-xl bg-slate-50 px-4 py-3"><p className="break-all text-sm font-medium">{project.path}</p><p className="mt-1 text-xs text-slate-500">{project.unityVersion ? `Unity ${project.unityVersion}` : "Unity project として未確認"}</p></div><div className="flex flex-wrap gap-2"><StatusPill label={project.status === "VALID" ? "Unity project" : "Unity 要確認"} tone={project.status === "VALID" ? "good" : "warn"} /><StatusPill label={project.isGitRepository ? "Git repository" : "Git 未初期化"} tone={project.isGitRepository ? "good" : "neutral"} /></div></div> : <div className="py-8 text-center text-sm text-slate-500">project folder を選択すると、Rust 側で Unity / Git の診断を行います。</div>}
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

function normalizeError(error: unknown): AppError {
  if (isAppError(error)) return error;
  return { code: "INTERNAL_ERROR", message: "処理に失敗しました。Vsedi のログを確認してください。", technicalDetail: null, operation: null, mayHaveMutated: false };
}

export default App;
