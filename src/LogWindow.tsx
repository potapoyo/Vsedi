import { useEffect, useRef, useState } from "react";
import type { AppError, LogSnapshot } from "@/generated/bindings";
import { isAppError, readRecentLogs } from "@/lib/commands";

export function LogWindow() {
  const [logs, setLogs] = useState<LogSnapshot | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const logView = useRef<HTMLPreElement | null>(null);

  useEffect(() => {
    let active = true;
    const loadLogs = async () => {
      try {
        const snapshot = await readRecentLogs();
        if (active) {
          setLogs(snapshot);
          setError(null);
        }
      } catch (caught) {
        if (active) setError(normalizeLogError(caught));
      }
    };
    void loadLogs();
    const timer = window.setInterval(() => void loadLogs(), 1000);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, []);

  useEffect(() => {
    if (logView.current) logView.current.scrollTop = logView.current.scrollHeight;
  }, [logs]);

  return (
    <main className="flex h-screen flex-col bg-mist p-6 text-ink">
      <header className="pb-4">
        <p className="text-xs text-slate-500">1秒ごとに保持期間内のログをすべて読み込みます。{logs?.currentFile ? ` 現在のファイル: ${logs.currentFile}` : ""}{logs ? ` 表示行数: ${logs.lines.length}` : ""}</p>
      </header>

      {error && (
        <div className="mb-4 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
          <p className="font-semibold">{error.message}</p>
          <p className="mt-1 text-xs text-rose-700">{error.code}</p>
        </div>
      )}

      <pre ref={logView} className="min-h-0 flex-1 overflow-auto whitespace-pre-wrap rounded-xl border border-slate-800 bg-slate-950 px-5 py-4 font-mono text-xs leading-5 text-slate-100">
        {logs?.lines.length ? logs.lines.join("\n") : "ログはまだありません。"}
      </pre>
    </main>
  );
}

function normalizeLogError(error: unknown): AppError {
  if (isAppError(error)) return error;
  return {
    code: "INTERNAL_ERROR",
    message: "ログを読み込めませんでした。",
    technicalDetail: null,
    operation: null,
    mayHaveMutated: false,
  };
}
