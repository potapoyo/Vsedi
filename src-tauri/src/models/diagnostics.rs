use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticStatus {
    Available,
    NotInstalled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitDiagnostic {
    pub status: DiagnosticStatus,
    pub executable: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDiagnostic {
    pub os: String,
    pub architecture: String,
    pub supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentDiagnostic {
    pub platform: PlatformDiagnostic,
    pub git: GitDiagnostic,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProjectStatus {
    Manageable,
    NeedsAttention,
    NotUnity,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProjectKind {
    Unity,
    VrchatAvatar,
    VrchatWorld,
    VrchatUnknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FileDiagnosticStatus {
    Healthy,
    Missing,
    NeedsAttention,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectIssue {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFileDiagnostic {
    pub path: String,
    pub status: FileDiagnosticStatus,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VpmPackage {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VpmDiagnostic {
    pub detected: bool,
    pub manifest_path: Option<String>,
    pub packages: Vec<VpmPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryDiagnostic {
    pub detected: Option<bool>,
    pub root: Option<String>,
    pub project_is_root: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlDiagnostic {
    pub gitignore: ConfigFileDiagnostic,
    pub vpm_packages: ConfigFileDiagnostic,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDiagnostic {
    pub path: String,
    pub status: ProjectStatus,
    pub is_unity_project: bool,
    pub unity_version: Option<String>,
    pub unity_revision: Option<String>,
    pub project_kind: ProjectKind,
    pub vpm: VpmDiagnostic,
    pub repository: RepositoryDiagnostic,
    pub source_control: SourceControlDiagnostic,
    pub issues: Vec<ProjectIssue>,
    pub is_git_repository: Option<bool>,
}

/// M3 の保存 preview で使う repository の読み取り状態。Git の内部用語は
/// frontend 側で通常の「作業の状態」に言い換えて表示する。
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryState {
    pub root: String,
    pub needs_initialization: bool,
    pub has_head: bool,
    pub branch_name: Option<String>,
    pub can_save: bool,
    pub blocking_reason: Option<RepositoryBlockingReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepositoryBlockingReason {
    NotRepository,
    Conflict,
    ExistingStagedChanges,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Unmerged,
    Untracked,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangedFile {
    pub path: String,
    pub old_path: Option<String>,
    pub change_kind: ChangeKind,
    pub staged: bool,
    pub unstaged: bool,
    pub binary: bool,
    pub outside_project: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeSnapshot {
    pub status_token: String,
    pub files: Vec<ChangedFile>,
    pub has_conflicts: bool,
    pub has_existing_staged_changes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IgnoreFilePreview {
    pub path: String,
    pub missing_rules: Vec<String>,
    pub will_create: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryInitializationPreview {
    pub status_token: String,
    pub repository_root: String,
    pub can_initialize: bool,
    pub blocking_reason: Option<String>,
    pub ignore_files: Vec<IgnoreFilePreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRepositoryRequest {
    pub project_path: String,
    pub status_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveRequest {
    pub project_path: String,
    pub status_token: String,
    pub memo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveResult {
    pub commit_id: String,
    pub short_commit_id: String,
    pub memo: String,
    pub author_time: String,
    pub file_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub commit_id: String,
    pub short_commit_id: String,
    pub memo: String,
    pub author_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommitDetail {
    pub commit_id: String,
    pub short_commit_id: String,
    pub memo: String,
    pub author_time: String,
    pub parent_ids: Vec<String>,
    pub files: Vec<ChangedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FileDiffKind {
    Text,
    Binary,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub path: String,
    pub kind: FileDiffKind,
    pub patch: Option<String>,
    pub truncated: bool,
    pub truncation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsLoadResult {
    pub settings: crate::models::settings::AppSettings,
    pub recovered: bool,
    pub backup_path: Option<String>,
    pub recent_projects: Vec<crate::models::settings::RecentProjectStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LogSnapshot {
    pub lines: Vec<String>,
    pub current_file: Option<String>,
}
