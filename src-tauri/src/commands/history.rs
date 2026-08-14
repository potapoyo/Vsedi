use crate::{
    errors::AppResult,
    models::{CommitDetail, HistoryPage},
    services::history,
};
#[tauri::command]
pub fn read_history(project_path: String, offset: usize) -> AppResult<HistoryPage> {
    history::read_history_page(&project_path, offset)
}
#[tauri::command]
pub fn read_commit_detail(project_path: String, commit_id: String) -> AppResult<CommitDetail> {
    history::read_commit_detail(&project_path, &commit_id)
}
