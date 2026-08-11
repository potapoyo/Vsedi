import { invoke } from "@tauri-apps/api/core";
import type {
  AppError,
  AppSettings,
  EnvironmentDiagnostic,
  LogSnapshot,
  ProjectDiagnostic,
  SettingsLoadResult,
  VpmTrackingPolicy,
} from "@/generated/bindings";

export function inspectEnvironment() {
  return invoke<EnvironmentDiagnostic>("inspect_environment");
}

export function inspectProject(path: string, vpmTrackingPolicy: VpmTrackingPolicy) {
  return invoke<ProjectDiagnostic>("inspect_project", { path, vpmTrackingPolicy });
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

export function readRecentLogs() {
  return invoke<LogSnapshot>("read_recent_logs");
}

export function isAppError(error: unknown): error is AppError {
  if (!error || typeof error !== "object") return false;
  const candidate = error as Partial<AppError>;
  return typeof candidate.message === "string" && typeof candidate.code === "string" && typeof candidate.mayHaveMutated === "boolean";
}
