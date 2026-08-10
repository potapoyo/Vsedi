use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub path: String,
    pub last_opened_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecentProjectStatus {
    pub path: String,
    pub last_opened_at: Option<String>,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub schema_version: u32,
    #[serde(default)]
    pub onboarding_completed: bool,
    #[serde(default)]
    pub recent_projects: Vec<RecentProject>,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_log_level() -> String {
    "INFO".to_owned()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            onboarding_completed: false,
            recent_projects: Vec::new(),
            log_level: default_log_level(),
        }
    }
}
