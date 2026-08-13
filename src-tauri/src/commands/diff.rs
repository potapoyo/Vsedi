use crate::{errors::AppResult, models::FileDiff, services::diff};
#[tauri::command]
pub fn read_worktree_diff(project_path: String, path: String) -> AppResult<FileDiff> {
    diff::read_worktree_diff(&project_path, &path)
}
#[tauri::command]
pub fn read_commit_diff(
    project_path: String,
    commit_id: String,
    path: String,
) -> AppResult<FileDiff> {
    diff::read_commit_diff(&project_path, &commit_id, &path)
}
