use crate::{
    errors::AppResult,
    models::{SaveRequest, SaveResult},
    services::save,
};
use tauri::{AppHandle, Emitter};

pub const GIT_COMMAND_OUTPUT_EVENT: &str = "git-command-output";

#[tauri::command]
pub fn save_worktree(app: AppHandle, request: SaveRequest) -> AppResult<SaveResult> {
    save::save_with_progress(request, |event| {
        let _ = app.emit(GIT_COMMAND_OUTPUT_EVENT, event);
    })
}
