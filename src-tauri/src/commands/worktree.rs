use crate::{errors::AppResult, models::{RepositoryState, WorktreeSnapshot}, services::worktree};

#[tauri::command]
pub fn read_repository_state(project_path: String) -> AppResult<RepositoryState> {
    worktree::read_repository_state(&project_path)
}

#[tauri::command]
pub fn read_worktree_snapshot(project_path: String) -> AppResult<WorktreeSnapshot> {
    worktree::read_worktree_snapshot(&project_path)
}
