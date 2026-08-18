//! UI-independent application facade.
//!
//! The Tauri commands and the Slint shell both call this module. Keeping the
//! operation names and DTOs here makes the migration boundary explicit: UI
//! frameworks may change, while Git and project safety rules stay in Rust.

use crate::{
    errors::AppResult,
    models::{
        ApplyIgnoreRulesRequest, CommitDetail, EnvironmentDiagnostic, FileDiff, HistoryPage,
        IgnoreTemplateSettings, InitializeRepositoryRequest, ProjectDiagnostic,
        RepositoryIgnorePreview, RepositoryInitializationPreview, RepositoryState,
        RepositoryTreeSnapshot, SaveRequest, SaveResult, VpmTrackingPolicy, WorktreeSnapshot,
    },
    services,
};

pub fn inspect_environment() -> AppResult<EnvironmentDiagnostic> {
    services::diagnostics::inspect_environment()
}

pub fn inspect_project(
    path: &str,
    vpm_tracking_policy: VpmTrackingPolicy,
) -> AppResult<ProjectDiagnostic> {
    services::projects::inspect_project(path, vpm_tracking_policy)
}

pub fn read_repository_state(path: &str) -> AppResult<RepositoryState> {
    services::worktree::read_repository_state(path)
}

pub fn read_worktree_snapshot(path: &str) -> AppResult<WorktreeSnapshot> {
    services::worktree::read_worktree_snapshot(path)
}

pub fn read_repository_tree(path: &str) -> AppResult<RepositoryTreeSnapshot> {
    services::worktree::read_repository_tree(path)
}

pub fn read_history(path: &str, offset: usize) -> AppResult<HistoryPage> {
    services::history::read_history_page(path, offset)
}

pub fn read_commit_detail(path: &str, commit_id: &str) -> AppResult<CommitDetail> {
    services::history::read_commit_detail(path, commit_id)
}

pub fn read_worktree_diff(path: &str, file_path: &str) -> AppResult<FileDiff> {
    services::diff::read_worktree_diff(path, file_path)
}

pub fn read_commit_diff(path: &str, commit_id: &str, file_path: &str) -> AppResult<FileDiff> {
    services::diff::read_commit_diff(path, commit_id, file_path)
}

pub fn save_worktree<F>(request: SaveRequest, progress: F) -> AppResult<SaveResult>
where
    F: FnMut(crate::models::GitCommandEvent),
{
    services::save::save_with_progress(request, progress)
}

pub fn preview_repository_initialization(
    path: &str,
    policy: VpmTrackingPolicy,
    templates: &IgnoreTemplateSettings,
) -> AppResult<RepositoryInitializationPreview> {
    services::initialization::preview(path, policy, templates)
}

pub fn initialize_repository(
    request: InitializeRepositoryRequest,
    policy: VpmTrackingPolicy,
    templates: &IgnoreTemplateSettings,
) -> AppResult<()> {
    services::initialization::initialize(
        &request.project_path,
        &request.status_token,
        policy,
        templates,
    )
}

pub fn preview_ignore_rules(
    path: &str,
    policy: VpmTrackingPolicy,
    templates: &IgnoreTemplateSettings,
) -> AppResult<RepositoryIgnorePreview> {
    services::initialization::preview_ignore_rules(path, policy, templates)
}

pub fn apply_ignore_rules(
    request: ApplyIgnoreRulesRequest,
    policy: VpmTrackingPolicy,
    templates: &IgnoreTemplateSettings,
) -> AppResult<()> {
    services::initialization::apply_ignore_rules(request, policy, templates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_facade_uses_the_service_layer() {
        let diagnostic = inspect_environment().expect("environment inspection should run");
        assert!(!diagnostic.platform.os.is_empty());
    }
}
