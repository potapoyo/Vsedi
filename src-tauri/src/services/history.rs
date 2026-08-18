use crate::{
    errors::{AppError, AppResult, ErrorCode},
    git::{command::run_git, diagnostics},
    models::{ChangeKind, ChangedFile, CommitDetail, HistoryEntry, HistoryPage},
    platform::process::find_executable,
};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

const HISTORY_FIELD_SEPARATOR: char = '\x1f';
const HISTORY_RECORD_SEPARATOR: char = '\x1e';
const HISTORY_PAGE_SIZE: usize = 20;

pub fn read_history(project_path: &str) -> AppResult<Vec<HistoryEntry>> {
    Ok(read_history_page(project_path, 0)?.entries)
}

pub fn read_history_page(project_path: &str, offset: usize) -> AppResult<HistoryPage> {
    debug!(operation = "read_history", project_path = %project_path, "history read started");
    let (git, root) = repository(project_path)?;
    let head =
        run_git(&git, &["rev-parse", "--verify", "HEAD"], Some(&root)).map_err(read_error)?;
    if head.status != Some(0) {
        return Ok(HistoryPage {
            entries: Vec::new(),
            next_offset: None,
        });
    }
    let skip_arg = format!("--skip={offset}");
    let mut args = vec![
        "log",
        "-n",
        "21",
        "--no-decorate",
        "--no-color",
        "--pretty=format:%H%x1f%h%x1f%s%x1f%aI%x1e",
    ];
    if offset > 0 {
        args.push(&skip_arg);
    }
    let output = run_git(&git, &args, Some(&root)).map_err(read_error)?;
    if output.status != Some(0) {
        return Err(AppError::with_detail(
            ErrorCode::HistoryReadFailed,
            "保存履歴を読み取れませんでした。",
            "read_history",
            output.stderr,
            false,
        ));
    }
    let values = parse_history_fields(&output.stdout)?;
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
    let page = paginate_history(entries, offset);
    info!(
        operation = "read_history",
        entry_count = page.entries.len(),
        "history read completed"
    );
    Ok(page)
}

fn paginate_history(mut entries: Vec<HistoryEntry>, offset: usize) -> HistoryPage {
    let has_more = entries.len() > HISTORY_PAGE_SIZE;
    entries.truncate(HISTORY_PAGE_SIZE);
    HistoryPage {
        entries,
        next_offset: has_more.then_some(offset.saturating_add(HISTORY_PAGE_SIZE)),
    }
}

fn parse_history_fields(output: &str) -> AppResult<Vec<String>> {
    let mut fields = Vec::new();
    for record in output
        .split(HISTORY_RECORD_SEPARATOR)
        .map(|record| record.trim_matches(['\r', '\n']))
        .filter(|record| !record.is_empty())
    {
        let record_fields = record.split(HISTORY_FIELD_SEPARATOR).collect::<Vec<_>>();
        if record_fields.len() != 4
            || record_fields[0].trim().is_empty()
            || record_fields[1].trim().is_empty()
            || record_fields[3].trim().is_empty()
        {
            return Err(AppError::simple(
                ErrorCode::HistoryReadFailed,
                "保存履歴を安全に解析できませんでした。",
                "read_history",
            ));
        }
        fields.extend([
            record_fields[0].trim().to_owned(),
            record_fields[1].trim().to_owned(),
            record_fields[2].to_owned(),
            record_fields[3].trim().to_owned(),
        ]);
    }
    Ok(fields)
}

fn parse_commit_metadata(output: &str) -> AppResult<[String; 5]> {
    let record = output
        .split(HISTORY_RECORD_SEPARATOR)
        .next()
        .unwrap_or_default()
        .trim_matches(['\r', '\n']);
    let fields = record.split(HISTORY_FIELD_SEPARATOR).collect::<Vec<_>>();
    if fields.len() != 5
        || fields[0].trim().is_empty()
        || fields[1].trim().is_empty()
        || fields[3].trim().is_empty()
    {
        return Err(AppError::simple(
            ErrorCode::HistoryReadFailed,
            "保存履歴を安全に解析できませんでした。",
            "read_commit_detail",
        ));
    }
    Ok([
        fields[0].trim().to_owned(),
        fields[1].trim().to_owned(),
        fields[2].to_owned(),
        fields[3].trim().to_owned(),
        fields[4].trim().to_owned(),
    ])
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
            "--no-decorate",
            "--no-color",
            "--pretty=format:%H%x1f%h%x1f%s%x1f%aI%x1f%P%x1e",
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
    let values = parse_commit_metadata(&metadata.stdout)?;
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
        commit_id: values[0].clone(),
        short_commit_id: values[1].clone(),
        memo: values[2].clone(),
        author_time: values[3].clone(),
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
    fn parses_windows_git_log_records_with_trailing_newlines() {
        let output = concat!(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\x1faaaaaaa\x1ffirst save\x1f",
            "2026-08-13T10:00:00+09:00\x1e\n",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\x1fbbbbbbb\x1fsecond save\x1f",
            "2026-08-13T11:00:00+09:00\x1e\n",
        );
        let fields = parse_history_fields(output).unwrap();

        assert_eq!(fields.len(), 8);
        assert_eq!(fields[2], "first save");
        assert_eq!(fields[4], "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    }

    #[test]
    fn parses_root_commit_metadata_without_parent() {
        let output = concat!(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\x1faaaaaaa\x1ffirst save\x1f",
            "2026-08-13T10:00:00+09:00\x1f\x1e\n",
        );
        let metadata = parse_commit_metadata(output).unwrap();

        assert_eq!(metadata[0], "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert!(metadata[4].is_empty());
    }

    #[test]
    fn rejects_history_records_with_missing_fields() {
        let output = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\x1faaaaaaa\x1ffirst save\x1e\n";

        assert!(parse_history_fields(output).is_err());
    }

    #[test]
    fn paginates_history_and_reports_older_entries() {
        let entries = (0..21)
            .map(|index| HistoryEntry {
                commit_id: format!("{index:040x}"),
                short_commit_id: format!("{index:07x}"),
                memo: format!("save {index}"),
                author_time: "2026-08-13T10:00:00+09:00".to_owned(),
            })
            .collect::<Vec<_>>();

        let first_page = paginate_history(entries, 0);
        assert_eq!(first_page.entries.len(), 20);
        assert_eq!(first_page.next_offset, Some(20));

        let last_page = paginate_history(vec![first_page.entries[0].clone()], 20);
        assert_eq!(last_page.entries.len(), 1);
        assert_eq!(last_page.next_offset, None);
    }

    #[test]
    fn parses_rename_status() {
        let files = parse_name_status("R100\0old file\0new file\0M\0Assets/a.txt\0").unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].old_path.as_deref(), Some("old file"));
        assert_eq!(files[1].change_kind, ChangeKind::Modified);
    }
}
