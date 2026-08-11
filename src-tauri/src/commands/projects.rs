use crate::{
    errors::AppResult,
    models::{ProjectDiagnostic, VpmTrackingPolicy},
    services::projects,
};

#[tauri::command]
pub fn inspect_project(
    path: String,
    vpm_tracking_policy: VpmTrackingPolicy,
) -> AppResult<ProjectDiagnostic> {
    projects::inspect_project(&path, vpm_tracking_policy)
}
