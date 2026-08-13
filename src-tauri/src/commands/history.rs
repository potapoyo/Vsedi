use crate::{
    errors::AppResult,
    models::{CommitDetail, HistoryEntry},
    services::history,
};
#[tauri::command]
pub fn read_history(project_path: String) -> AppResult<Vec<HistoryEntry>> {
    history::read_history(&project_path)
}
#[tauri::command]
pub fn read_commit_detail(project_path: String, commit_id: String) -> AppResult<CommitDetail> {
    history::read_commit_detail(&project_path, &commit_id)
}
