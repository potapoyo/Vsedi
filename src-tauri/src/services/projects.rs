use crate::{
    errors::{AppError, AppResult, ErrorCode},
    git::diagnostics as git_diagnostics,
    models::{ProjectDiagnostic, ProjectStatus},
    platform::process::find_executable,
};
use std::{fs, path::Path};

pub fn inspect_project(path: &str) -> AppResult<ProjectDiagnostic> {
    let requested = Path::new(path);
    if !requested.exists() {
        return Err(AppError::simple(
            ErrorCode::ProjectNotFound,
            "選択した project folder が見つかりません。",
            "inspect_project",
        ));
    }
    if !requested.is_dir() {
        return Err(AppError::simple(
            ErrorCode::ProjectNotFound,
            "project folder を選択してください。",
            "inspect_project",
        ));
    }

    let root = requested.canonicalize().map_err(|error| {
        if error.kind() == std::io::ErrorKind::PermissionDenied {
            AppError::simple(
                ErrorCode::ProjectPermissionDenied,
                "project folder を読み取れません。",
                "inspect_project",
            )
        } else {
            AppError::from_io(
                ErrorCode::FilesystemReadFailed,
                "canonicalize_project",
                requested,
                &error,
            )
        }
    })?;
    let project_version = root.join("ProjectSettings").join("ProjectVersion.txt");
    let is_unity_project = root.join("Assets").is_dir() && project_version.is_file();
    let unity_version = is_unity_project
        .then(|| read_unity_version(&project_version))
        .flatten();
    let is_git_repository =
        find_executable("git").and_then(|git| git_diagnostics::is_repository(&git, &root));

    Ok(ProjectDiagnostic {
        path: root.to_string_lossy().into_owned(),
        status: if is_unity_project {
            ProjectStatus::Valid
        } else {
            ProjectStatus::InvalidUnity
        },
        is_unity_project,
        unity_version,
        is_git_repository,
    })
}

fn read_unity_version(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    text.lines().find_map(|line| {
        line.trim()
            .strip_prefix("m_EditorVersion: ")
            .map(ToOwned::to_owned)
    })
}

#[cfg(test)]
mod tests {
    use super::read_unity_version;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn reads_unity_project_version() {
        let path = std::env::temp_dir().join(format!(
            "vsedi-project-version-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(
            &path,
            "m_EditorVersion: 2022.3.22f1\nm_EditorVersionWithRevision: 2022.3.22f1 (abcd)\n",
        )
        .unwrap();
        assert_eq!(read_unity_version(&path), Some("2022.3.22f1".to_owned()));
        let _ = fs::remove_file(path);
    }
}
