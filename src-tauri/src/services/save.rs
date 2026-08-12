use crate::{
    errors::{AppError, AppResult, ErrorCode},
    git::{command::run_git, diagnostics},
    models::{SaveRequest, SaveResult},
    platform::process::find_executable,
    services::worktree,
};
use std::path::{Path, PathBuf};

pub fn save(request: SaveRequest) -> AppResult<SaveResult> {
    let memo = request.memo.trim();
    if memo.is_empty() { return Err(AppError::simple(ErrorCode::SaveMemoInvalid, "保存メモを入力してください。", "save_worktree")); }
    let project = canonical_project(&request.project_path)?;
    let Some(git) = find_executable("git") else { return Err(AppError::simple(ErrorCode::RepositoryInvalid, "System Git が見つかりません。", "save_worktree")); };
    let root = diagnostics::repository_root(&git, &project).flatten().ok_or_else(|| AppError::simple(ErrorCode::RepositoryInvalid, "この project は Git 管理されていません。", "save_worktree"))?;
    let root = PathBuf::from(root);
    let snapshot = worktree::read_worktree_snapshot(&request.project_path)?;
    if snapshot.status_token != request.status_token { return Err(AppError::simple(ErrorCode::RepositoryStateChanged, "表示後に変更内容が変わりました。もう一度確認してから保存してください。", "save_worktree")); }
    if snapshot.has_conflicts { return Err(AppError::simple(ErrorCode::SaveConflict, "競合中のファイルがあるため、作業を保存できません。", "save_worktree")); }
    if snapshot.has_existing_staged_changes { return Err(AppError::simple(ErrorCode::SaveExistingStagedChanges, "すでに Git のステージにある変更があるため、安全のため保存できません。", "save_worktree")); }
    if snapshot.files.is_empty() { return Err(AppError::simple(ErrorCode::SaveNoChanges, "保存する変更がありません。", "save_worktree")); }
    let add = run_git(&git, &["add", "-A"], Some(&root)).map_err(|error| AppError::with_detail(ErrorCode::SaveAddFailed, "変更を保存準備できませんでした。", "git_add", error.to_string(), false))?;
    if add.status != Some(0) { return Err(AppError::with_detail(ErrorCode::SaveAddFailed, "変更を保存準備できませんでした。", "git_add", add.stderr, true)); }
    let commit = run_git(&git, &["commit", "-m", memo], Some(&root)).map_err(|error| AppError::with_detail(ErrorCode::SaveCommitFailed, "保存 commit を作成できませんでした。変更が保存準備済みになっている可能性があります。", "git_commit", error.to_string(), true))?;
    if commit.status != Some(0) { return Err(AppError::with_detail(ErrorCode::SaveCommitFailed, "保存 commit を作成できませんでした。変更が保存準備済みになっている可能性があります。", "git_commit", commit.stderr, true)); }
    let commit_id = git_required_text(&git, &["rev-parse", "HEAD"], &root, "commit ID を確認できませんでした。")?;
    let author_time = git_required_text(&git, &["show", "-s", "--format=%aI", "HEAD"], &root, "保存時刻を確認できませんでした。")?;
    Ok(SaveResult { short_commit_id: commit_id.chars().take(8).collect(), commit_id, memo: memo.to_owned(), author_time, file_count: snapshot.files.len() })
}
fn canonical_project(path: &str) -> AppResult<PathBuf> { let requested = Path::new(path); if !requested.is_dir() { return Err(AppError::simple(ErrorCode::ProjectNotFound, "project folder を選択してください。", "save_worktree")); } requested.canonicalize().map_err(|error| AppError::with_detail(ErrorCode::FilesystemReadFailed, "project folder を読み取れません。", "save_worktree", error.to_string(), false)) }
fn git_required_text(git: &Path, args: &[&str], root: &Path, message: &'static str) -> AppResult<String> { let output = run_git(git, args, Some(root)).map_err(|error| AppError::with_detail(ErrorCode::SaveCommitFailed, message, "verify_save", error.to_string(), true))?; let value = output.stdout.trim(); if output.status != Some(0) || value.is_empty() { return Err(AppError::with_detail(ErrorCode::SaveCommitFailed, message, "verify_save", output.stderr, true)); } Ok(value.to_owned()) }

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, process::Command, time::{SystemTime, UNIX_EPOCH}};

    fn git(root: &Path, args: &[&str]) {
        let status = Command::new("git").args(args).current_dir(root).status().unwrap();
        assert!(status.success());
    }

    #[test]
    fn saves_a_previewed_worktree_as_a_commit() {
        let root = std::env::temp_dir().join(format!("vsedi-save-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init"]); git(&root, &["config", "user.name", "Vsedi test"]); git(&root, &["config", "user.email", "test@example.invalid"]);
        fs::write(root.join("scene with space.txt"), "saved\n").unwrap();
        let snapshot = worktree::read_worktree_snapshot(root.to_str().unwrap()).unwrap();
        let result = save(SaveRequest { project_path: root.to_string_lossy().into_owned(), status_token: snapshot.status_token, memo: "初回保存".to_owned() }).unwrap();
        assert_eq!(result.file_count, 1); assert_eq!(result.memo, "初回保存"); assert_eq!(result.commit_id.len(), 40);
        assert!(worktree::read_worktree_snapshot(root.to_str().unwrap()).unwrap().files.is_empty());
        let history = crate::services::history::read_history(root.to_str().unwrap()).unwrap();
        assert_eq!(history.first().map(|entry| entry.commit_id.as_str()), Some(result.commit_id.as_str()));
        let detail = crate::services::history::read_commit_detail(root.to_str().unwrap(), &result.commit_id).unwrap();
        assert_eq!(detail.files.len(), 1);
        let diff = crate::services::diff::read_commit_diff(root.to_str().unwrap(), &result.commit_id, "scene with space.txt").unwrap();
        assert_eq!(diff.kind, crate::models::FileDiffKind::Text); assert!(diff.patch.unwrap().contains("saved"));
        fs::remove_dir_all(root).unwrap();
    }
}
