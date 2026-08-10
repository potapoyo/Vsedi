use crate::{
    errors::{AppError, AppResult, ErrorCode},
    git::command::run_git,
    logging::sanitize_text,
    models::{DiagnosticStatus, GitDiagnostic, GitLfsDiagnostic},
    platform::process::{find_executable, ProcessOutput},
};
use std::path::Path;
use tracing::{info, warn};

pub fn inspect() -> AppResult<GitDiagnostic> {
    let Some(executable) = find_executable("git") else {
        info!(
            operation = "inspect_git",
            status = "not_installed",
            "system git was not found"
        );
        return Ok(GitDiagnostic {
            status: DiagnosticStatus::NotInstalled,
            executable: None,
            version: None,
            lfs: GitLfsDiagnostic {
                status: DiagnosticStatus::NotInstalled,
                version: None,
                detail: Some("Git が見つからないため Git LFS を診断できません。".to_owned()),
            },
        });
    };

    let version_output = run_git(&executable, &["--version"], None).map_err(|error| {
        AppError::with_detail(
            ErrorCode::EnvGitVersionFailed,
            "Git のバージョンを確認できませんでした。",
            "inspect_git_version",
            error.to_string(),
            false,
        )
    })?;
    if version_output.status != Some(0) {
        return Err(AppError::with_detail(
            ErrorCode::EnvGitVersionFailed,
            "Git のバージョンを確認できませんでした。",
            "inspect_git_version",
            sanitize_process_detail(&version_output),
            false,
        ));
    }

    let version = parse_git_version(&version_output.stdout);
    let lfs = inspect_lfs(&executable)?;
    info!(
        operation = "inspect_git",
        executable = %executable.display(),
        version = version.as_deref().unwrap_or("unknown"),
        lfs_status = ?lfs.status,
        "system git diagnostic completed"
    );
    Ok(GitDiagnostic {
        status: DiagnosticStatus::Available,
        executable: Some(executable.to_string_lossy().into_owned()),
        version,
        lfs,
    })
}

fn inspect_lfs(executable: &Path) -> AppResult<GitLfsDiagnostic> {
    let output = run_git(executable, &["lfs", "version"], None).map_err(|error| {
        AppError::with_detail(
            ErrorCode::EnvGitLfsVersionFailed,
            "Git LFS の診断を実行できませんでした。",
            "inspect_git_lfs_version",
            error.to_string(),
            false,
        )
    })?;

    if output.status == Some(0) {
        return Ok(GitLfsDiagnostic {
            status: DiagnosticStatus::Available,
            version: parse_lfs_version(&output.stdout),
            detail: None,
        });
    }

    warn!(
        operation = "inspect_git_lfs_version",
        status = output.status.unwrap_or(-1),
        detail = %sanitize_process_detail(&output),
        "git lfs is not available"
    );
    Ok(GitLfsDiagnostic {
        status: DiagnosticStatus::NotInstalled,
        version: None,
        detail: Some("Git LFS は利用できません。Git の拡張として診断しました。".to_owned()),
    })
}

pub fn is_repository(executable: &Path, project_path: &Path) -> Option<bool> {
    let output = run_git(
        executable,
        &["rev-parse", "--is-inside-work-tree"],
        Some(project_path),
    )
    .ok()?;
    Some(output.status == Some(0) && output.stdout.trim() == "true")
}

fn parse_git_version(stdout: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        line.trim()
            .strip_prefix("git version ")
            .map(ToOwned::to_owned)
    })
}

fn parse_lfs_version(stdout: &str) -> Option<String> {
    let first_line = stdout.lines().next()?.trim();
    first_line
        .strip_prefix("git-lfs/")
        .or_else(|| first_line.strip_prefix("git-lfs version "))
        .map(|version| {
            version
                .split_whitespace()
                .next()
                .unwrap_or(version)
                .to_owned()
        })
}

fn sanitize_process_detail(output: &ProcessOutput) -> String {
    sanitize_text(&format!(
        "exit={:?}; {}",
        output.status,
        output.stderr.trim()
    ))
}

#[cfg(test)]
mod tests {
    use super::{parse_git_version, parse_lfs_version};

    #[test]
    fn parses_git_version() {
        assert_eq!(
            parse_git_version("git version 2.45.1\n"),
            Some("2.45.1".to_owned())
        );
    }

    #[test]
    fn parses_lfs_version() {
        assert_eq!(
            parse_lfs_version("git-lfs/3.5.1 (GitHub; darwin arm64; go 1.22.0)"),
            Some("3.5.1".to_owned())
        );
    }
}
