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
    VrchatAvatarAndWorld,
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
