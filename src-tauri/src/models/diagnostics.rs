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
pub struct GitLfsDiagnostic {
    pub status: DiagnosticStatus,
    pub version: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitDiagnostic {
    pub status: DiagnosticStatus,
    pub executable: Option<String>,
    pub version: Option<String>,
    pub lfs: GitLfsDiagnostic,
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
    Valid,
    Missing,
    InvalidUnity,
    PermissionDenied,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDiagnostic {
    pub path: String,
    pub status: ProjectStatus,
    pub is_unity_project: bool,
    pub unity_version: Option<String>,
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
