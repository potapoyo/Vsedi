use crate::{errors::AppResult, models::ProjectDiagnostic, services::projects, settings};
use tauri::AppHandle;

#[tauri::command]
pub fn inspect_project(app: AppHandle, path: String) -> AppResult<ProjectDiagnostic> {
    let settings = settings::load(&app)?.settings;
    let vpm_tracking_policy = settings::resolve_vpm_tracking_policy_for_project(&settings, &path);
    projects::inspect_project(&path, vpm_tracking_policy)
}
