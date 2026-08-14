use crate::{
    errors::{AppError, AppResult, ErrorCode},
    git::{command::run_git, diagnostics, status::parse_porcelain_v2},
    models::{
        ChangedFile, RepositoryBlockingReason, RepositoryState, RepositoryTreeFile,
        RepositoryTreeSnapshot, WorktreeSnapshot,
    },
    platform::process::find_executable,
};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};
use tracing::{debug, info};

pub fn read_repository_state(project_path: &str) -> AppResult<RepositoryState> {
    debug!(operation = "read_repository_state", project_path = %project_path, "repository state read started");
    let project = canonical_project(project_path)?;
    let Some(git) = find_executable("git") else {
        return Err(AppError::simple(
            ErrorCode::RepositoryInvalid,
            "System Git が見つかりません。",
            "read_repository_state",
        ));
    };
    let root = diagnostics::repository_root(&git, &project)
        .ok_or_else(|| {
            AppError::simple(
                ErrorCode::WorktreeReadFailed,
                "Git repository の状態を読み取れませんでした。",
                "read_repository_state",
            )
        })?
        .ok_or_else(|| {
            AppError::simple(
                ErrorCode::RepositoryInvalid,
                "この project はまだ Git 管理されていません。",
                "read_repository_state",
            )
        })?;
    let root_path = PathBuf::from(&root);
    let has_head = git_success(&git, &["rev-parse", "--verify", "HEAD"], &root_path)?;
    let branch_name = git_text(
        &git,
        &["symbolic-ref", "--quiet", "--short", "HEAD"],
        &root_path,
    )?
    .filter(|name| !name.is_empty());
    let snapshot = read_worktree_snapshot_at(&git, &root_path, &project)?;
    let blocking_reason = if snapshot.has_conflicts {
        Some(RepositoryBlockingReason::Conflict)
    } else if snapshot.has_existing_staged_changes {
        Some(RepositoryBlockingReason::ExistingStagedChanges)
    } else {
        None
    };
    let state = RepositoryState {
        root,
        needs_initialization: false,
        has_head,
        branch_name,
        can_save: blocking_reason.is_none(),
        blocking_reason,
    };
    info!(
        operation = "read_repository_state",
        repository_root = %state.root,
        has_head = state.has_head,
        can_save = state.can_save,
        blocking_reason = ?state.blocking_reason,
        "repository state read completed"
    );
    Ok(state)
}

pub fn read_worktree_snapshot(project_path: &str) -> AppResult<WorktreeSnapshot> {
    debug!(operation = "read_worktree_snapshot", project_path = %project_path, "worktree snapshot read started");
    let project = canonical_project(project_path)?;
    let Some(git) = find_executable("git") else {
        return Err(AppError::simple(
            ErrorCode::RepositoryInvalid,
            "System Git が見つかりません。",
            "read_worktree_snapshot",
        ));
    };
    let root = diagnostics::repository_root(&git, &project)
        .flatten()
        .ok_or_else(|| {
            AppError::simple(
                ErrorCode::RepositoryInvalid,
                "この project はまだ Git 管理されていません。",
                "read_worktree_snapshot",
            )
        })?;
    let snapshot = read_worktree_snapshot_at(&git, Path::new(&root), &project)?;
    info!(
        operation = "read_worktree_snapshot",
        file_count = snapshot.files.len(),
        has_conflicts = snapshot.has_conflicts,
        has_existing_staged_changes = snapshot.has_existing_staged_changes,
        "worktree snapshot read completed"
    );
    Ok(snapshot)
}

pub fn read_repository_tree(project_path: &str) -> AppResult<RepositoryTreeSnapshot> {
    debug!(operation = "read_repository_tree", project_path = %project_path, "repository file tree read started");
    let project = canonical_project(project_path)?;
    let Some(git) = find_executable("git") else {
        return Err(AppError::simple(
            ErrorCode::RepositoryInvalid,
            "System Git が見つかりません。",
            "read_repository_tree",
        ));
    };
    let root = diagnostics::repository_root(&git, &project)
        .flatten()
        .ok_or_else(|| {
            AppError::simple(
                ErrorCode::RepositoryInvalid,
                "この project はまだ Git 管理されていません。",
                "read_repository_tree",
            )
        })?;
    let root_path = PathBuf::from(&root);
    let snapshot = read_worktree_snapshot_at(&git, &root_path, &project)?;
    let output = run_git(
        &git,
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
        Some(&root_path),
    )
    .map_err(|error| {
        AppError::with_detail(
            ErrorCode::WorktreeReadFailed,
            "repositoryのファイル一覧を読み取れませんでした。",
            "read_repository_tree",
            error.to_string(),
            false,
        )
    })?;
    if output.status != Some(0) {
        return Err(AppError::simple(
            ErrorCode::WorktreeReadFailed,
            "repositoryのファイル一覧を読み取れませんでした。",
            "read_repository_tree",
        ));
    }

    let project_prefix = project_prefix(&root_path, &project);
    let changed_paths = snapshot
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<HashSet<_>>();
    let mut files = snapshot
        .files
        .iter()
        .map(repository_tree_file_from_changed)
        .collect::<Vec<_>>();
    for path in output.stdout.split('\0').filter(|path| !path.is_empty()) {
        if changed_paths.contains(path) {
            continue;
        }
        files.push(RepositoryTreeFile {
            path: path.to_owned(),
            old_path: None,
            change_kind: None,
            staged: false,
            unstaged: false,
            binary: false,
            outside_project: project_prefix
                .as_deref()
                .is_some_and(|prefix| !path.starts_with(prefix)),
        });
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let result = RepositoryTreeSnapshot {
        status_token: snapshot.status_token,
        files,
    };
    info!(
        operation = "read_repository_tree",
        file_count = result.files.len(),
        "repository file tree read completed"
    );
    Ok(result)
}

fn read_worktree_snapshot_at(
    git: &Path,
    root: &Path,
    project: &Path,
) -> AppResult<WorktreeSnapshot> {
    let output = run_git(
        git,
        &["status", "--porcelain=v2", "-z", "--untracked-files=all"],
        Some(root),
    )
    .map_err(|error| {
        AppError::with_detail(
            ErrorCode::WorktreeReadFailed,
            "変更一覧を読み取れませんでした。",
            "read_worktree_snapshot",
            error.to_string(),
            false,
        )
    })?;
    if output.status != Some(0) {
        return Err(AppError::simple(
            ErrorCode::WorktreeReadFailed,
            "変更一覧を読み取れませんでした。",
            "read_worktree_snapshot",
        ));
    }
    let project_prefix = project_prefix(root, project);
    let mut snapshot =
        parse_porcelain_v2(&output.stdout, project_prefix.as_deref()).map_err(|detail| {
            AppError::with_detail(
                ErrorCode::WorktreeReadFailed,
                "変更一覧を安全に解析できませんでした。",
                "parse_git_status",
                detail,
                false,
            )
        })?;
    snapshot.status_token = content_aware_status_token(&output.stdout, root, &snapshot.files);
    Ok(snapshot)
}

fn project_prefix(root: &Path, project: &Path) -> Option<String> {
    project
        .strip_prefix(root)
        .ok()
        .and_then(|path| path.to_str())
        .map(|path| {
            if path.is_empty() {
                String::new()
            } else {
                format!("{}/", path.trim_end_matches('/'))
            }
        })
}

fn repository_tree_file_from_changed(file: &ChangedFile) -> RepositoryTreeFile {
    RepositoryTreeFile {
        path: file.path.clone(),
        old_path: file.old_path.clone(),
        change_kind: Some(file.change_kind.clone()),
        staged: file.staged,
        unstaged: file.unstaged,
        binary: file.binary,
        outside_project: file.outside_project,
    }
}

fn canonical_project(path: &str) -> AppResult<PathBuf> {
    let requested = Path::new(path);
    if !requested.is_dir() {
        return Err(AppError::simple(
            ErrorCode::ProjectNotFound,
            "project folder を選択してください。",
            "read_worktree",
        ));
    }
    requested.canonicalize().map_err(|error| {
        AppError::with_detail(
            ErrorCode::FilesystemReadFailed,
            "project folder を読み取れません。",
            "read_worktree",
            error.to_string(),
            false,
        )
    })
}
fn git_success(git: &Path, args: &[&str], root: &Path) -> AppResult<bool> {
    Ok(run_git(git, args, Some(root))
        .map_err(|error| {
            AppError::with_detail(
                ErrorCode::WorktreeReadFailed,
                "Git の状態を読み取れませんでした。",
                "read_repository_state",
                error.to_string(),
                false,
            )
        })?
        .status
        == Some(0))
}
fn git_text(git: &Path, args: &[&str], root: &Path) -> AppResult<Option<String>> {
    let output = run_git(git, args, Some(root)).map_err(|error| {
        AppError::with_detail(
            ErrorCode::WorktreeReadFailed,
            "Git の状態を読み取れませんでした。",
            "read_repository_state",
            error.to_string(),
            false,
        )
    })?;
    Ok((output.status == Some(0)).then(|| output.stdout.trim().to_owned()))
}

fn content_aware_status_token(
    output: &str,
    root: &Path,
    files: &[crate::models::ChangedFile],
) -> String {
    let mut hash = checksum(output.as_bytes());
    for file in files {
        mix(&mut hash, file.path.as_bytes());
        fingerprint_path(&mut hash, root, &file.path);
        if let Some(old_path) = &file.old_path {
            mix(&mut hash, old_path.as_bytes());
            fingerprint_path(&mut hash, root, old_path);
        }
    }
    format!("{hash:016x}")
}

fn fingerprint_path(hash: &mut u64, root: &Path, relative: &str) {
    let path = Path::new(relative);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        mix(hash, b"invalid-path");
        return;
    }
    let path = root.join(path);
    let Ok(metadata) = fs::symlink_metadata(&path) else {
        mix(hash, b"missing");
        return;
    };
    mix(hash, &metadata.len().to_le_bytes());
    if metadata.file_type().is_symlink() {
        match fs::read_link(path) {
            Ok(target) => mix(hash, target.to_string_lossy().as_bytes()),
            Err(_) => mix(hash, b"unreadable-symlink"),
        }
        return;
    }
    let Ok(mut file) = File::open(path) else {
        mix(hash, b"unreadable");
        return;
    };
    let mut buffer = [0_u8; 8192];
    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => mix(hash, &buffer[..read]),
            Err(_) => {
                mix(hash, b"read-error");
                break;
            }
        }
    }
}

fn checksum(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    mix(&mut hash, bytes);
    hash
}

fn mix(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash = (*hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
    }
}
