use crate::{
    errors::{AppError, AppResult, ErrorCode},
    git::{command::run_git, diagnostics, status::parse_porcelain_v2},
    models::{RepositoryBlockingReason, RepositoryState, WorktreeSnapshot},
    platform::process::find_executable,
};
use std::path::{Path, PathBuf};

pub fn read_repository_state(project_path: &str) -> AppResult<RepositoryState> {
    let project = canonical_project(project_path)?;
    let Some(git) = find_executable("git") else {
        return Err(AppError::simple(ErrorCode::RepositoryInvalid, "System Git が見つかりません。", "read_repository_state"));
    };
    let root = diagnostics::repository_root(&git, &project)
        .ok_or_else(|| AppError::simple(ErrorCode::WorktreeReadFailed, "Git repository の状態を読み取れませんでした。", "read_repository_state"))?
        .ok_or_else(|| AppError::simple(ErrorCode::RepositoryInvalid, "この project はまだ Git 管理されていません。", "read_repository_state"))?;
    let root_path = PathBuf::from(&root);
    let has_head = git_success(&git, &["rev-parse", "--verify", "HEAD"], &root_path)?;
    let branch_name = git_text(&git, &["symbolic-ref", "--quiet", "--short", "HEAD"], &root_path)?.filter(|name| !name.is_empty());
    let snapshot = read_worktree_snapshot_at(&git, &root_path, &project)?;
    let blocking_reason = if snapshot.has_conflicts { Some(RepositoryBlockingReason::Conflict) } else if snapshot.has_existing_staged_changes { Some(RepositoryBlockingReason::ExistingStagedChanges) } else { None };
    Ok(RepositoryState { root, needs_initialization: false, has_head, branch_name, can_save: blocking_reason.is_none(), blocking_reason })
}

pub fn read_worktree_snapshot(project_path: &str) -> AppResult<WorktreeSnapshot> {
    let project = canonical_project(project_path)?;
    let Some(git) = find_executable("git") else { return Err(AppError::simple(ErrorCode::RepositoryInvalid, "System Git が見つかりません。", "read_worktree_snapshot")); };
    let root = diagnostics::repository_root(&git, &project)
        .flatten()
        .ok_or_else(|| AppError::simple(ErrorCode::RepositoryInvalid, "この project はまだ Git 管理されていません。", "read_worktree_snapshot"))?;
    read_worktree_snapshot_at(&git, Path::new(&root), &project)
}

fn read_worktree_snapshot_at(git: &Path, root: &Path, project: &Path) -> AppResult<WorktreeSnapshot> {
    let output = run_git(git, &["status", "--porcelain=v2", "-z", "--untracked-files=all"], Some(root))
        .map_err(|error| AppError::with_detail(ErrorCode::WorktreeReadFailed, "変更一覧を読み取れませんでした。", "read_worktree_snapshot", error.to_string(), false))?;
    if output.status != Some(0) { return Err(AppError::simple(ErrorCode::WorktreeReadFailed, "変更一覧を読み取れませんでした。", "read_worktree_snapshot")); }
    let project_prefix = project.strip_prefix(root).ok().and_then(|path| path.to_str()).map(|path| if path.is_empty() { String::new() } else { format!("{}/", path.trim_end_matches('/')) });
    parse_porcelain_v2(&output.stdout, project_prefix.as_deref()).map_err(|detail| AppError::with_detail(ErrorCode::WorktreeReadFailed, "変更一覧を安全に解析できませんでした。", "parse_git_status", detail, false))
}

fn canonical_project(path: &str) -> AppResult<PathBuf> {
    let requested = Path::new(path);
    if !requested.is_dir() { return Err(AppError::simple(ErrorCode::ProjectNotFound, "project folder を選択してください。", "read_worktree")); }
    requested.canonicalize().map_err(|error| AppError::with_detail(ErrorCode::FilesystemReadFailed, "project folder を読み取れません。", "read_worktree", error.to_string(), false))
}
fn git_success(git: &Path, args: &[&str], root: &Path) -> AppResult<bool> { Ok(run_git(git, args, Some(root)).map_err(|error| AppError::with_detail(ErrorCode::WorktreeReadFailed, "Git の状態を読み取れませんでした。", "read_repository_state", error.to_string(), false))?.status == Some(0)) }
fn git_text(git: &Path, args: &[&str], root: &Path) -> AppResult<Option<String>> { let output = run_git(git, args, Some(root)).map_err(|error| AppError::with_detail(ErrorCode::WorktreeReadFailed, "Git の状態を読み取れませんでした。", "read_repository_state", error.to_string(), false))?; Ok((output.status == Some(0)).then(|| output.stdout.trim().to_owned())) }
