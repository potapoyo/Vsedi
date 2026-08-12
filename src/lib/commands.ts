import { invoke } from "@tauri-apps/api/core";
import type {
  AppError,
  AppSettings,
  EnvironmentDiagnostic,
  InitializeRepositoryRequest,
  HistoryEntry,
  CommitDetail,
  FileDiff,
  LogSnapshot,
  ProjectDiagnostic,
  RepositoryState,
  RepositoryInitializationPreview,
  SettingsLoadResult,
  SaveRequest,
  SaveResult,
  WorktreeSnapshot,
} from "@/generated/bindings";

export function inspectEnvironment() {
  return invoke<EnvironmentDiagnostic>("inspect_environment");
}

export function inspectProject(path: string) {
  return invoke<ProjectDiagnostic>("inspect_project", { path });
}

export function readRepositoryState(projectPath: string) {
  return invoke<RepositoryState>("read_repository_state", { projectPath });
}

export function readWorktreeSnapshot(projectPath: string) {
  return invoke<WorktreeSnapshot>("read_worktree_snapshot", { projectPath });
}

export function previewRepositoryInitialization(projectPath: string) {
  return invoke<RepositoryInitializationPreview>("preview_repository_initialization", { projectPath });
}

export function initializeRepository(request: InitializeRepositoryRequest) {
  return invoke<void>("initialize_repository", { request });
}

export function saveWorktree(request: SaveRequest) {
  return invoke<SaveResult>("save_worktree", { request });
}

export function readHistory(projectPath: string) {
  return invoke<HistoryEntry[]>("read_history", { projectPath });
}

export function readCommitDetail(projectPath: string, commitId: string) {
  return invoke<CommitDetail>("read_commit_detail", { projectPath, commitId });
}

export function readWorktreeDiff(projectPath: string, path: string) {
  return invoke<FileDiff>("read_worktree_diff", { projectPath, path });
}

export function readCommitDiff(projectPath: string, commitId: string, path: string) {
  return invoke<FileDiff>("read_commit_diff", { projectPath, commitId, path });
}

export function loadSettings() {
  return invoke<SettingsLoadResult>("load_settings");
}

export function saveSettings(settings: AppSettings) {
  return invoke<void>("save_settings", { settings });
}

export function exportDiagnosticLog(destination: string) {
  return invoke<void>("export_diagnostic_log", { destination });
}

export function openLogDirectory() {
  return invoke<void>("open_log_directory");
}

export function openLogWindow() {
  return invoke<void>("open_log_window");
}

export function readRecentLogs() {
  return invoke<LogSnapshot>("read_recent_logs");
}

export function isAppError(error: unknown): error is AppError {
  if (!error || typeof error !== "object") return false;
  const candidate = error as Partial<AppError>;
  return typeof candidate.message === "string" && typeof candidate.code === "string" && typeof candidate.mayHaveMutated === "boolean";
}
