use crate::{
    errors::{AppError, AppResult, ErrorCode},
    git::{command::run_git, diagnostics},
    models::{ChangeKind, ChangedFile, CommitDetail, HistoryEntry},
    platform::process::find_executable,
};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

pub fn read_history(project_path: &str) -> AppResult<Vec<HistoryEntry>> {
    debug!(operation = "read_history", project_path = %project_path, "history read started");
    let (git, root) = repository(project_path)?;
    let head =
        run_git(&git, &["rev-parse", "--verify", "HEAD"], Some(&root)).map_err(read_error)?;
    if head.status != Some(0) {
        return Ok(Vec::new());
    }
    let output = run_git(
        &git,
        &["log", "-n", "50", "--format=%H%x00%h%x00%s%x00%aI%x00"],
        Some(&root),
    )
    .map_err(read_error)?;
    if output.status != Some(0) {
        return Err(AppError::with_detail(
            ErrorCode::HistoryReadFailed,
            "保存履歴を読み取れませんでした。",
            "read_history",
            output.stderr,
            false,
        ));
    }
    let values = output
        .stdout
        .split('\0')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if values.len() % 4 != 0 {
        return Err(AppError::simple(
            ErrorCode::HistoryReadFailed,
            "保存履歴を安全に解析できませんでした。",
            "read_history",
        ));
    }
    let entries = values
        .chunks_exact(4)
        .map(|values| HistoryEntry {
            commit_id: values[0].to_owned(),
            short_commit_id: values[1].to_owned(),
            memo: values[2].to_owned(),
            author_time: values[3].to_owned(),
        })
        .collect::<Vec<_>>();
    info!(
        operation = "read_history",
        entry_count = entries.len(),
        "history read completed"
    );
    Ok(entries)
}

pub fn read_commit_detail(project_path: &str, commit_id: &str) -> AppResult<CommitDetail> {
    debug!(operation = "read_commit_detail", project_path = %project_path, commit_id = %commit_id, "commit detail read started");
    if !is_object_id(commit_id) {
        return Err(AppError::simple(
            ErrorCode::HistoryReadFailed,
            "指定された保存履歴を確認できません。",
            "read_commit_detail",
        ));
    }
    let (git, root) = repository(project_path)?;
    let metadata = run_git(
        &git,
        &[
            "show",
            "-s",
            "--format=%H%x00%h%x00%s%x00%aI%x00%P%x00",
            commit_id,
        ],
        Some(&root),
    )
    .map_err(read_error)?;
    if metadata.status != Some(0) {
        return Err(AppError::with_detail(
            ErrorCode::HistoryReadFailed,
            "保存履歴を読み取れませんでした。",
            "read_commit_detail",
            metadata.stderr,
            false,
        ));
    }
    let values = metadata
        .stdout
        .split('\0')
        .take(5)
        .map(str::trim)
        .collect::<Vec<_>>();
    if values.len() != 5 {
        return Err(AppError::simple(
            ErrorCode::HistoryReadFailed,
            "保存履歴を安全に解析できませんでした。",
            "read_commit_detail",
        ));
    }
    let files_output = run_git(
        &git,
        &[
            "diff-tree",
            "--root",
            "--no-commit-id",
            "--name-status",
            "-r",
            "-z",
            commit_id,
        ],
        Some(&root),
    )
    .map_err(read_error)?;
    if files_output.status != Some(0) {
        return Err(AppError::with_detail(
            ErrorCode::HistoryReadFailed,
            "保存したファイル一覧を読み取れませんでした。",
            "read_commit_detail",
            files_output.stderr,
            false,
        ));
    }
    let detail = CommitDetail {
        commit_id: values[0].to_owned(),
        short_commit_id: values[1].to_owned(),
        memo: values[2].to_owned(),
        author_time: values[3].to_owned(),
        parent_ids: values[4]
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect(),
        files: parse_name_status(&files_output.stdout)?,
    };
    info!(operation = "read_commit_detail", commit_id = %detail.short_commit_id, file_count = detail.files.len(), "commit detail read completed");
    Ok(detail)
}

fn repository(project_path: &str) -> AppResult<(PathBuf, PathBuf)> {
    let project = Path::new(project_path).canonicalize().map_err(|error| {
        AppError::with_detail(
            ErrorCode::FilesystemReadFailed,
            "project folder を読み取れません。",
            "read_history",
            error.to_string(),
            false,
        )
    })?;
    let git = find_executable("git").ok_or_else(|| {
        AppError::simple(
            ErrorCode::RepositoryInvalid,
            "System Git が見つかりません。",
            "read_history",
        )
    })?;
    let root = diagnostics::repository_root(&git, &project)
        .flatten()
        .ok_or_else(|| {
            AppError::simple(
                ErrorCode::RepositoryInvalid,
                "この project は Git 管理されていません。",
                "read_history",
            )
        })?;
    Ok((git, PathBuf::from(root)))
}
fn read_error(error: std::io::Error) -> AppError {
    AppError::with_detail(
        ErrorCode::HistoryReadFailed,
        "保存履歴を読み取れませんでした。",
        "read_history",
        error.to_string(),
        false,
    )
}
fn is_object_id(value: &str) -> bool {
    (value.len() == 40 || value.len() == 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn parse_name_status(output: &str) -> AppResult<Vec<ChangedFile>> {
    let tokens = output
        .split('\0')
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let mut files = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let status = tokens[i];
        let code = status.as_bytes().first().copied().ok_or_else(|| {
            AppError::simple(
                ErrorCode::HistoryReadFailed,
                "変更ファイル一覧を解析できませんでした。",
                "read_commit_detail",
            )
        })?;
        let renamed = matches!(code, b'R' | b'C');
        if renamed {
            let old_path = tokens.get(i + 1).ok_or_else(|| {
                AppError::simple(
                    ErrorCode::HistoryReadFailed,
                    "変更ファイル一覧を解析できませんでした。",
                    "read_commit_detail",
                )
            })?;
            let path = tokens.get(i + 2).ok_or_else(|| {
                AppError::simple(
                    ErrorCode::HistoryReadFailed,
                    "変更ファイル一覧を解析できませんでした。",
                    "read_commit_detail",
                )
            })?;
            files.push(file(
                path,
                Some((*old_path).to_owned()),
                if code == b'R' {
                    ChangeKind::Renamed
                } else {
                    ChangeKind::Copied
                },
            ));
            i += 3;
        } else {
            let path = tokens.get(i + 1).ok_or_else(|| {
                AppError::simple(
                    ErrorCode::HistoryReadFailed,
                    "変更ファイル一覧を解析できませんでした。",
                    "read_commit_detail",
                )
            })?;
            let kind = match code {
                b'A' => ChangeKind::Added,
                b'D' => ChangeKind::Deleted,
                b'T' => ChangeKind::TypeChanged,
                _ => ChangeKind::Modified,
            };
            files.push(file(path, None, kind));
            i += 2;
        }
    }
    Ok(files)
}
fn file(path: &str, old_path: Option<String>, change_kind: ChangeKind) -> ChangedFile {
    ChangedFile {
        path: path.to_owned(),
        old_path,
        change_kind,
        staged: false,
        unstaged: false,
        binary: false,
        outside_project: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_rename_status() {
        let files = parse_name_status("R100\0old file\0new file\0M\0Assets/a.txt\0").unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].old_path.as_deref(), Some("old file"));
        assert_eq!(files[1].change_kind, ChangeKind::Modified);
    }
}
