use crate::{
    errors::AppResult,
    models::{SaveRequest, SaveResult},
    services::save,
};
#[tauri::command]
pub fn save_worktree(request: SaveRequest) -> AppResult<SaveResult> {
    save::save(request)
}
