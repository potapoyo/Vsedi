use crate::{errors::AppResult, models::ProjectDiagnostic, services::projects};

#[tauri::command]
pub fn inspect_project(path: String) -> AppResult<ProjectDiagnostic> {
    projects::inspect_project(&path)
}
