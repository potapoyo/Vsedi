use crate::{errors::{AppError, AppResult, ErrorCode}, git::command::run_git, models::{FileDiff, FileDiffKind}, services::history};
use std::path::Path;

const MAX_DIFF_BYTES: usize = 200 * 1024;

pub fn read_worktree_diff(project_path: &str, path: &str) -> AppResult<FileDiff> { read_diff(project_path, None, path) }
pub fn read_commit_diff(project_path: &str, commit_id: &str, path: &str) -> AppResult<FileDiff> { if !is_object_id(commit_id) { return Err(AppError::simple(ErrorCode::DiffReadFailed, "指定された保存履歴を確認できません。", "read_commit_diff")); } read_diff(project_path, Some(commit_id), path) }
fn read_diff(project_path: &str, commit_id: Option<&str>, path: &str) -> AppResult<FileDiff> {
    validate_path(path)?;
    let (git, root) = repository(project_path)?;
    let mut numstat_args = vec!["diff", "--no-ext-diff", "--numstat", "-z"];
    if let Some(id) = commit_id { numstat_args = vec!["show", "--format=", "--no-ext-diff", "--numstat", "-z", id]; }
    numstat_args.extend(["--", path]);
    let numstat = run_git(&git, &numstat_args, Some(&root)).map_err(read_error)?;
    if numstat.status != Some(0) { return Err(AppError::with_detail(ErrorCode::DiffReadFailed, "ファイルの変更内容を読み取れませんでした。", "read_file_diff", numstat.stderr, false)); }
    if numstat.stdout.is_empty() { return Ok(FileDiff { path: path.to_owned(), kind: FileDiffKind::Unavailable, patch: None, truncated: false, truncation_reason: Some("未管理ファイル、または表示できる差分がありません。".to_owned()) }); }
    if numstat.stdout.split('\0').next().is_some_and(|line| line.starts_with("-\t-\t")) { return Ok(FileDiff { path: path.to_owned(), kind: FileDiffKind::Binary, patch: None, truncated: false, truncation_reason: None }); }
    let mut args = vec!["diff", "--no-ext-diff", "--no-color"];
    if let Some(id) = commit_id { args = vec!["show", "--format=", "--no-ext-diff", "--no-color", id]; }
    args.extend(["--", path]);
    let output = run_git(&git, &args, Some(&root)).map_err(read_error)?;
    if output.status != Some(0) { return Err(AppError::with_detail(ErrorCode::DiffReadFailed, "ファイルの変更内容を読み取れませんでした。", "read_file_diff", output.stderr, false)); }
    let bytes = output.stdout.as_bytes();
    let truncated = bytes.len() > MAX_DIFF_BYTES;
    let patch = if truncated { String::from_utf8_lossy(&bytes[..MAX_DIFF_BYTES]).into_owned() } else { output.stdout };
    Ok(FileDiff { path: path.to_owned(), kind: FileDiffKind::Text, patch: Some(patch), truncated, truncation_reason: truncated.then(|| format!("表示は {} KB で打ち切りました。", MAX_DIFF_BYTES / 1024)) })
}
fn repository(project_path: &str) -> AppResult<(std::path::PathBuf, std::path::PathBuf)> {
    // Reuse history's validation indirectly: it performs no mutations, then locate Git again for the command.
    let _ = history::read_history(project_path)?;
    let root = Path::new(project_path).canonicalize().map_err(|error| AppError::with_detail(ErrorCode::FilesystemReadFailed, "project folder を読み取れません。", "read_file_diff", error.to_string(), false))?;
    let git = crate::platform::process::find_executable("git").ok_or_else(|| AppError::simple(ErrorCode::RepositoryInvalid, "System Git が見つかりません。", "read_file_diff"))?;
    let repository = crate::git::diagnostics::repository_root(&git, &root).flatten().ok_or_else(|| AppError::simple(ErrorCode::RepositoryInvalid, "この project は Git 管理されていません。", "read_file_diff"))?;
    Ok((git, repository.into()))
}
fn validate_path(path: &str) -> AppResult<()> { let path = Path::new(path); if path.is_absolute() || path.components().any(|component| matches!(component, std::path::Component::ParentDir | std::path::Component::RootDir | std::path::Component::Prefix(_))) { return Err(AppError::simple(ErrorCode::DiffReadFailed, "ファイル path を確認できません。", "read_file_diff")); } Ok(()) }
fn read_error(error: std::io::Error) -> AppError { AppError::with_detail(ErrorCode::DiffReadFailed, "ファイルの変更内容を読み取れませんでした。", "read_file_diff", error.to_string(), false) }
fn is_object_id(value: &str) -> bool { (value.len() == 40 || value.len() == 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit()) }

#[cfg(test)]
mod tests { use super::*; #[test] fn rejects_unsafe_paths() { assert!(validate_path("../secret").is_err()); assert!(validate_path("Assets/a.txt").is_ok()); } }
