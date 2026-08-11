use crate::{
    errors::{AppError, AppResult, ErrorCode},
    git::command::run_git,
    logging::sanitize_text,
    models::{DiagnosticStatus, GitDiagnostic},
    platform::process::{find_executable, ProcessOutput},
};
use std::path::Path;
use tracing::info;

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
    info!(
        operation = "inspect_git",
        executable = %executable.display(),
        version = version.as_deref().unwrap_or("unknown"),
        "system git diagnostic completed"
    );
    Ok(GitDiagnostic {
        status: DiagnosticStatus::Available,
        executable: Some(executable.to_string_lossy().into_owned()),
        version,
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

pub fn repository_root(executable: &Path, project_path: &Path) -> Option<Option<String>> {
    let output = run_git(
        executable,
        &["rev-parse", "--show-toplevel"],
        Some(project_path),
    )
    .ok()?;
    if output.status != Some(0) {
        return Some(None);
    }
    let root = output.stdout.lines().next()?.trim();
    (!root.is_empty()).then(|| Some(root.to_owned()))
}

pub fn tracked_package_files(executable: &Path, project_path: &Path) -> Option<Vec<String>> {
    let output = run_git(
        executable,
        &["ls-files", "--", "Packages"],
        Some(project_path),
    )
    .ok()?;
    (output.status == Some(0)).then(|| {
        output
            .stdout
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    })
}

fn parse_git_version(stdout: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        line.trim()
            .strip_prefix("git version ")
            .map(ToOwned::to_owned)
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
    use super::parse_git_version;

    #[test]
    fn parses_git_version() {
        assert_eq!(
            parse_git_version("git version 2.45.1\n"),
            Some("2.45.1".to_owned())
        );
    }
}
