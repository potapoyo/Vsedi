use crate::{errors::AppResult, models::EnvironmentDiagnostic, services::diagnostics};

#[tauri::command]
pub fn inspect_environment() -> AppResult<EnvironmentDiagnostic> {
    diagnostics::inspect_environment()
}
