use crate::{
    errors::{AppError, AppResult, ErrorCode},
    git::diagnostics as git_diagnostics,
    models::{
        ConfigFileDiagnostic, DiagnosticSeverity, FileDiagnosticStatus, LargeFileDiagnostic,
        ProjectDiagnostic, ProjectIssue, ProjectKind, ProjectStatus, RepositoryDiagnostic,
        SourceControlDiagnostic, VpmDiagnostic, VpmPackage,
    },
    platform::process::find_executable,
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const LARGE_FILE_THRESHOLD_BYTES: u64 = 50 * 1024 * 1024;
const MAX_SCANNED_FILES: usize = 100_000;
const MAX_LARGE_FILE_RESULTS: usize = 100;

pub fn inspect_project(path: &str) -> AppResult<ProjectDiagnostic> {
    let requested = Path::new(path);
    validate_requested_path(requested)?;

    let selected_symlink = fs::symlink_metadata(requested)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false);
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

    let mut issues = Vec::new();
    if selected_symlink {
        issues.push(issue(
            "PROJECT_ROOT_SYMLINK",
            DiagnosticSeverity::Warning,
            "選択した project root は symlink です。実体の path を管理対象として使用します。",
            Some(requested.to_string_lossy().into_owned()),
        ));
    }

    let project_version_path = root.join("ProjectSettings/ProjectVersion.txt");
    let assets_exists = root.join("Assets").is_dir();
    let project_version_exists = project_version_path.is_file();
    let is_unity_project = assets_exists && project_version_exists;

    if !is_unity_project {
        if !assets_exists {
            issues.push(issue(
                "UNITY_ASSETS_MISSING",
                DiagnosticSeverity::Error,
                "Unity project に必要な Assets folder がありません。",
                Some("Assets".to_owned()),
            ));
        }
        if !project_version_exists {
            issues.push(issue(
                "UNITY_PROJECT_VERSION_MISSING",
                DiagnosticSeverity::Error,
                "ProjectSettings/ProjectVersion.txt がありません。",
                Some("ProjectSettings/ProjectVersion.txt".to_owned()),
            ));
        }
        let repository = inspect_repository(&root, &mut issues);
        let is_git_repository = repository.detected;
        return Ok(ProjectDiagnostic {
            path: root.to_string_lossy().into_owned(),
            status: ProjectStatus::NotUnity,
            is_unity_project: false,
            unity_version: None,
            unity_revision: None,
            project_kind: ProjectKind::Unity,
            vpm: VpmDiagnostic {
                detected: false,
                manifest_path: None,
                packages: Vec::new(),
            },
            repository,
            source_control: not_applicable_source_control(),
            issues,
            is_git_repository,
        });
    }

    let (unity_version, unity_revision) = read_unity_metadata(&project_version_path, &mut issues);
    let vpm = inspect_vpm(&root, &mut issues);
    let project_kind = classify_project(&vpm.packages, &mut issues);
    let repository = inspect_repository(&root, &mut issues);
    let source_control = inspect_source_control(&root, &vpm, &repository, &mut issues);
    let is_git_repository = repository.detected;
    let status = if issues.iter().any(|item| {
        matches!(
            item.severity,
            DiagnosticSeverity::Warning | DiagnosticSeverity::Error
        )
    }) {
        ProjectStatus::NeedsAttention
    } else {
        ProjectStatus::Manageable
    };

    Ok(ProjectDiagnostic {
        path: root.to_string_lossy().into_owned(),
        status,
        is_unity_project: true,
        unity_version,
        unity_revision,
        project_kind,
        vpm,
        repository,
        source_control,
        issues,
        is_git_repository,
    })
}

fn validate_requested_path(requested: &Path) -> AppResult<()> {
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
    Ok(())
}

fn read_unity_metadata(
    path: &Path,
    issues: &mut Vec<ProjectIssue>,
) -> (Option<String>, Option<String>) {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            issues.push(issue(
                "UNITY_PROJECT_VERSION_UNREADABLE",
                DiagnosticSeverity::Error,
                format!("Unity version metadata を読み取れません: {error}"),
                Some("ProjectSettings/ProjectVersion.txt".to_owned()),
            ));
            return (None, None);
        }
    };
    let version = text.lines().find_map(|line| {
        line.trim()
            .strip_prefix("m_EditorVersion: ")
            .map(ToOwned::to_owned)
    });
    let revision = text.lines().find_map(|line| {
        line.trim()
            .strip_prefix("m_EditorVersionWithRevision: ")
            .and_then(|value| value.split_once('('))
            .map(|(_, revision)| revision.trim_end_matches(')').trim().to_owned())
    });
    if version.is_none() {
        issues.push(issue(
            "UNITY_VERSION_UNKNOWN",
            DiagnosticSeverity::Warning,
            "ProjectVersion.txt から Unity version を判定できません。",
            Some("ProjectSettings/ProjectVersion.txt".to_owned()),
        ));
    }
    (version, revision)
}

fn inspect_vpm(root: &Path, issues: &mut Vec<ProjectIssue>) -> VpmDiagnostic {
    let vpm_path = root.join("Packages/vpm-manifest.json");
    let unity_manifest_path = root.join("Packages/manifest.json");
    let mut packages = BTreeMap::<String, Option<String>>::new();

    if unity_manifest_path.is_file() {
        read_package_manifest(root, &unity_manifest_path, &mut packages, issues);
    } else {
        issues.push(issue(
            "UNITY_PACKAGE_MANIFEST_MISSING",
            DiagnosticSeverity::Warning,
            "Packages/manifest.json がありません。Unity package 構成を完全には診断できません。",
            Some("Packages/manifest.json".to_owned()),
        ));
    }

    let detected = vpm_path.is_file();
    if detected {
        read_package_manifest(root, &vpm_path, &mut packages, issues);
    }

    VpmDiagnostic {
        detected,
        manifest_path: detected.then(|| "Packages/vpm-manifest.json".to_owned()),
        packages: packages
            .into_iter()
            .map(|(name, version)| VpmPackage { name, version })
            .collect(),
    }
}

fn read_package_manifest(
    root: &Path,
    path: &Path,
    packages: &mut BTreeMap<String, Option<String>>,
    issues: &mut Vec<ProjectIssue>,
) {
    let relative = relative_or_absolute(root, path);
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            issues.push(issue(
                "PACKAGE_MANIFEST_UNREADABLE",
                DiagnosticSeverity::Error,
                format!("package manifest を読み取れません: {error}"),
                Some(relative),
            ));
            return;
        }
    };
    let value: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => {
            issues.push(issue(
                "PACKAGE_MANIFEST_INVALID_JSON",
                DiagnosticSeverity::Error,
                format!("package manifest が正しい JSON ではありません: {error}"),
                Some(relative),
            ));
            return;
        }
    };
    let Some(dependencies) = value.get("dependencies").and_then(Value::as_object) else {
        return;
    };
    for (name, value) in dependencies {
        let version = value.as_str().map(ToOwned::to_owned).or_else(|| {
            value
                .get("version")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        });
        packages.insert(name.clone(), version);
    }
}

fn classify_project(packages: &[VpmPackage], issues: &mut Vec<ProjectIssue>) -> ProjectKind {
    let has_avatar = packages
        .iter()
        .any(|item| item.name == "com.vrchat.avatars");
    let has_world = packages.iter().any(|item| item.name == "com.vrchat.worlds");
    let has_vrchat = packages
        .iter()
        .any(|item| item.name.starts_with("com.vrchat."));
    match (has_avatar, has_world, has_vrchat) {
        (true, false, _) => ProjectKind::VrchatAvatar,
        (false, true, _) => ProjectKind::VrchatWorld,
        (true, true, _) => {
            issues.push(issue(
                "VRCHAT_PROJECT_KIND_MIXED",
                DiagnosticSeverity::Warning,
                "Avatar SDK と Worlds SDK の両方が見つかりました。project 種別を確認してください。",
                Some("Packages".to_owned()),
            ));
            ProjectKind::VrchatAvatarAndWorld
        }
        (false, false, true) => {
            issues.push(issue(
                "VRCHAT_PROJECT_KIND_UNKNOWN",
                DiagnosticSeverity::Warning,
                "VRChat package はありますが、Avatar / World を確定できません。",
                Some("Packages".to_owned()),
            ));
            ProjectKind::VrchatUnknown
        }
        _ => ProjectKind::Unity,
    }
}

fn inspect_repository(root: &Path, issues: &mut Vec<ProjectIssue>) -> RepositoryDiagnostic {
    let Some(git) = find_executable("git") else {
        return RepositoryDiagnostic {
            detected: None,
            root: None,
            project_is_root: None,
        };
    };
    let Some(repository_root) = git_diagnostics::repository_root(&git, root) else {
        issues.push(issue(
            "GIT_REPOSITORY_CHECK_FAILED",
            DiagnosticSeverity::Warning,
            "Git repository の境界を確認できませんでした。",
            None,
        ));
        return RepositoryDiagnostic {
            detected: None,
            root: None,
            project_is_root: None,
        };
    };
    let Some(repository_root) = repository_root else {
        return RepositoryDiagnostic {
            detected: Some(false),
            root: None,
            project_is_root: None,
        };
    };
    let repository_path = PathBuf::from(&repository_root);
    let canonical_repository = repository_path.canonicalize().unwrap_or(repository_path);
    let project_is_root = canonical_repository == root;
    if !project_is_root {
        issues.push(issue(
            "GIT_ROOT_OUTSIDE_PROJECT",
            DiagnosticSeverity::Warning,
            "選択した Unity project と Git repository root が一致しません。親 repository や nested boundary を確認してください。",
            Some(repository_root.clone()),
        ));
    }
    RepositoryDiagnostic {
        detected: Some(true),
        root: Some(repository_root),
        project_is_root: Some(project_is_root),
    }
}

fn inspect_source_control(
    root: &Path,
    vpm: &VpmDiagnostic,
    repository: &RepositoryDiagnostic,
    issues: &mut Vec<ProjectIssue>,
) -> SourceControlDiagnostic {
    let gitignore = inspect_gitignore(root, issues);
    let gitattributes = inspect_gitattributes(root, issues);
    let vpm_packages = inspect_vpm_source_control(root, vpm, repository, issues);
    let (large_files, scan_truncated) = scan_large_files(root);
    for file in &large_files {
        issues.push(issue(
            "LARGE_FILE_CANDIDATE",
            DiagnosticSeverity::Warning,
            format!(
                "50 MiB 以上のファイルがあります（{} MiB）。Git LFS の対象か確認してください。",
                file.size_bytes / 1024 / 1024
            ),
            Some(file.path.clone()),
        ));
    }
    if scan_truncated {
        issues.push(issue(
            "LARGE_FILE_SCAN_TRUNCATED",
            DiagnosticSeverity::Warning,
            "ファイル数が多いため、大容量ファイル診断を途中で終了しました。",
            None,
        ));
    }
    SourceControlDiagnostic {
        gitignore,
        gitattributes,
        vpm_packages,
        large_files,
        scan_truncated,
    }
}

fn inspect_gitignore(root: &Path, issues: &mut Vec<ProjectIssue>) -> ConfigFileDiagnostic {
    let relative = ".gitignore".to_owned();
    let path = root.join(&relative);
    if !path.is_file() {
        issues.push(issue(
            "GITIGNORE_MISSING",
            DiagnosticSeverity::Warning,
            ".gitignore がありません。Unity の生成物が保存対象に入る可能性があります。",
            Some(relative.clone()),
        ));
        return config(
            relative,
            FileDiagnosticStatus::Missing,
            "ファイルがありません",
        );
    }
    let Ok(text) = fs::read_to_string(&path) else {
        issues.push(issue(
            "GITIGNORE_UNREADABLE",
            DiagnosticSeverity::Error,
            ".gitignore を読み取れません。",
            Some(relative.clone()),
        ));
        return config(
            relative,
            FileDiagnosticStatus::NeedsAttention,
            "読み取れません",
        );
    };
    let missing = ["Library", "Temp"]
        .into_iter()
        .filter(|directory| !has_ignore_rule(&text, directory))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        config(
            relative,
            FileDiagnosticStatus::Healthy,
            "Unity の主要生成物を除外しています",
        )
    } else {
        issues.push(issue(
            "GITIGNORE_UNITY_RULES_INCOMPLETE",
            DiagnosticSeverity::Warning,
            format!(
                "Unity 生成物の除外ルールが不足しています: {}",
                missing.join(", ")
            ),
            Some(relative.clone()),
        ));
        config(
            relative,
            FileDiagnosticStatus::NeedsAttention,
            "Unity の主要生成物に対するルールが不足しています",
        )
    }
}

fn inspect_gitattributes(root: &Path, issues: &mut Vec<ProjectIssue>) -> ConfigFileDiagnostic {
    let relative = ".gitattributes".to_owned();
    let path = root.join(&relative);
    if !path.is_file() {
        issues.push(issue(
            "GITATTRIBUTES_MISSING",
            DiagnosticSeverity::Warning,
            ".gitattributes がありません。大容量 binary asset の Git LFS rule を確認できません。",
            Some(relative.clone()),
        ));
        return config(
            relative,
            FileDiagnosticStatus::Missing,
            "ファイルがありません",
        );
    }
    let Ok(text) = fs::read_to_string(&path) else {
        issues.push(issue(
            "GITATTRIBUTES_UNREADABLE",
            DiagnosticSeverity::Error,
            ".gitattributes を読み取れません。",
            Some(relative.clone()),
        ));
        return config(
            relative,
            FileDiagnosticStatus::NeedsAttention,
            "読み取れません",
        );
    };
    let has_lfs = meaningful_lines(&text).any(|line| line.contains("filter=lfs"));
    if has_lfs {
        config(
            relative,
            FileDiagnosticStatus::Healthy,
            "Git LFS rule があります",
        )
    } else {
        issues.push(issue(
            "GITATTRIBUTES_LFS_RULES_MISSING",
            DiagnosticSeverity::Warning,
            "Git LFS rule がありません。Unity の大容量 binary asset に必要か確認してください。",
            Some(relative.clone()),
        ));
        config(
            relative,
            FileDiagnosticStatus::NeedsAttention,
            "Git LFS rule がありません",
        )
    }
}

fn inspect_vpm_source_control(
    root: &Path,
    vpm: &VpmDiagnostic,
    repository: &RepositoryDiagnostic,
    issues: &mut Vec<ProjectIssue>,
) -> ConfigFileDiagnostic {
    let relative = "Packages/.gitignore".to_owned();
    if !vpm.detected {
        return config(
            relative,
            FileDiagnosticStatus::NotApplicable,
            "VPM project ではありません",
        );
    }
    let path = root.join(&relative);
    if !path.is_file() {
        issues.push(issue(
            "VPM_GITIGNORE_MISSING",
            DiagnosticSeverity::Warning,
            "Packages/.gitignore がありません。VRChat package 本体を除外できているか確認してください。",
            Some(relative.clone()),
        ));
        return config(
            relative,
            FileDiagnosticStatus::Missing,
            "VPM package 除外ルールがありません",
        );
    }
    let Ok(text) = fs::read_to_string(&path) else {
        issues.push(issue(
            "VPM_GITIGNORE_UNREADABLE",
            DiagnosticSeverity::Error,
            "Packages/.gitignore を読み取れません。",
            Some(relative.clone()),
        ));
        return config(
            relative,
            FileDiagnosticStatus::NeedsAttention,
            "読み取れません",
        );
    };
    let lines = meaningful_lines(&text).collect::<Vec<_>>();
    let excludes_vrchat = lines
        .iter()
        .any(|line| !line.starts_with('!') && line.contains("com.vrchat.") && line.contains('*'));
    let includes_resolver = lines.iter().any(|line| {
        line.starts_with('!')
            && (line.contains("com.vrchat.core.") || line.contains("com.vrchat.core.vpm-resolver"))
    });
    let mut healthy = excludes_vrchat && includes_resolver;
    if !healthy {
        issues.push(issue(
            "VPM_SOURCE_CONTROL_RULES_INCOMPLETE",
            DiagnosticSeverity::Warning,
            "VRChat package の除外、または VPM Resolver の例外ルールが不足しています。",
            Some(relative.clone()),
        ));
    }

    if repository.project_is_root == Some(true) {
        if let Some(git) = find_executable("git") {
            if let Some(files) = git_diagnostics::tracked_package_files(&git, root) {
                let tracked_vrchat_package = files.iter().any(|path| {
                    let normalized = path.replace('\\', "/");
                    normalized.starts_with("Packages/com.vrchat.")
                        && !normalized.starts_with("Packages/com.vrchat.core.vpm-resolver/")
                });
                if tracked_vrchat_package {
                    healthy = false;
                    issues.push(issue(
                        "VPM_PACKAGE_TRACKED",
                        DiagnosticSeverity::Error,
                        "VPM package 本体が Git の追跡対象です。Resolver 以外の VRChat package は通常除外します。",
                        Some("Packages".to_owned()),
                    ));
                }
            }
        }
    }

    if healthy {
        config(
            relative,
            FileDiagnosticStatus::Healthy,
            "VRChat package を除外し Resolver を保持します",
        )
    } else {
        config(
            relative,
            FileDiagnosticStatus::NeedsAttention,
            "VPM source-control rule を確認してください",
        )
    }
}

fn scan_large_files(root: &Path) -> (Vec<LargeFileDiagnostic>, bool) {
    let mut results = Vec::new();
    let mut stack = ["Assets", "Packages", "ProjectSettings"]
        .into_iter()
        .map(|name| root.join(name))
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    let mut scanned = 0usize;
    let mut truncated = false;

    while let Some(directory) = stack.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            if scanned >= MAX_SCANNED_FILES || results.len() >= MAX_LARGE_FILE_RESULTS {
                truncated = true;
                break;
            }
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() {
                if !skip_scan_directory(&path) {
                    stack.push(path);
                }
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            scanned += 1;
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.len() >= LARGE_FILE_THRESHOLD_BYTES {
                results.push(LargeFileDiagnostic {
                    path: relative_or_absolute(root, &path),
                    size_bytes: metadata.len(),
                });
            }
        }
        if truncated {
            break;
        }
    }
    results.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));
    (results, truncated)
}

fn skip_scan_directory(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(name.as_str(), ".git" | "library" | "temp" | "obj" | "logs")
        || (path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            == Some("Packages")
            && name.starts_with("com.vrchat.")
            && name != "com.vrchat.core.vpm-resolver")
}

fn has_ignore_rule(text: &str, directory: &str) -> bool {
    let needle = directory.to_ascii_lowercase();
    meaningful_lines(text).any(|line| {
        !line.starts_with('!')
            && line
                .replace(['[', ']'], "")
                .to_ascii_lowercase()
                .contains(&needle)
    })
}

fn meaningful_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

fn not_applicable_source_control() -> SourceControlDiagnostic {
    SourceControlDiagnostic {
        gitignore: config(
            ".gitignore".to_owned(),
            FileDiagnosticStatus::NotApplicable,
            "Unity project ではありません",
        ),
        gitattributes: config(
            ".gitattributes".to_owned(),
            FileDiagnosticStatus::NotApplicable,
            "Unity project ではありません",
        ),
        vpm_packages: config(
            "Packages/.gitignore".to_owned(),
            FileDiagnosticStatus::NotApplicable,
            "Unity project ではありません",
        ),
        large_files: Vec::new(),
        scan_truncated: false,
    }
}

fn config(
    path: String,
    status: FileDiagnosticStatus,
    summary: impl Into<String>,
) -> ConfigFileDiagnostic {
    ConfigFileDiagnostic {
        path,
        status,
        summary: summary.into(),
    }
}

fn issue(
    code: impl Into<String>,
    severity: DiagnosticSeverity,
    message: impl Into<String>,
    path: Option<String>,
) -> ProjectIssue {
    ProjectIssue {
        code: code.into(),
        severity,
        message: message.into(),
        path,
    }
}

fn relative_or_absolute(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::{inspect_project, LARGE_FILE_THRESHOLD_BYTES};
    use crate::models::{FileDiagnosticStatus, ProjectKind, ProjectStatus};
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    struct Fixture(PathBuf);

    impl Fixture {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "vsedi-{name}-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            ));
            fs::create_dir_all(&path).expect("fixture root");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn unity(&self, dependencies: &str) {
            fs::create_dir_all(self.0.join("Assets")).expect("Assets");
            fs::create_dir_all(self.0.join("ProjectSettings")).expect("ProjectSettings");
            fs::create_dir_all(self.0.join("Packages")).expect("Packages");
            fs::write(
                self.0.join("ProjectSettings/ProjectVersion.txt"),
                "m_EditorVersion: 2022.3.22f1\nm_EditorVersionWithRevision: 2022.3.22f1 (abcd1234)\n",
            )
            .expect("ProjectVersion");
            fs::write(
                self.0.join("Packages/manifest.json"),
                format!(r#"{{"dependencies":{dependencies}}}"#),
            )
            .expect("manifest");
            fs::write(self.0.join(".gitignore"), "[Ll]ibrary/\n[Tt]emp/\n").expect("gitignore");
            fs::write(
                self.0.join(".gitattributes"),
                "*.psd filter=lfs diff=lfs merge=lfs -text\n",
            )
            .expect("gitattributes");
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn identifies_non_unity_folder() {
        let fixture = Fixture::new("not-unity");
        let result = inspect_project(fixture.path().to_str().expect("path")).expect("diagnostic");
        assert_eq!(result.status, ProjectStatus::NotUnity);
        assert!(!result.is_unity_project);
        assert!(result
            .issues
            .iter()
            .any(|issue| issue.code == "UNITY_ASSETS_MISSING"));
    }

    #[test]
    fn identifies_vpm_avatar_project_with_healthy_rules() {
        let fixture = Fixture::new("avatar");
        fixture.unity(r#"{"com.vrchat.avatars":"3.10.4","com.vrchat.base":"3.10.4"}"#);
        fs::write(
            fixture.path().join("Packages/vpm-manifest.json"),
            r#"{"dependencies":{"com.vrchat.avatars":{"version":"3.10.4"}}}"#,
        )
        .expect("vpm manifest");
        fs::write(
            fixture.path().join("Packages/.gitignore"),
            "com.vrchat.*\n!com.vrchat.core.*\n",
        )
        .expect("package gitignore");

        let result = inspect_project(fixture.path().to_str().expect("path")).expect("diagnostic");
        assert_eq!(result.status, ProjectStatus::Manageable);
        assert_eq!(result.project_kind, ProjectKind::VrchatAvatar);
        assert_eq!(result.unity_revision.as_deref(), Some("abcd1234"));
        assert_eq!(
            result.source_control.vpm_packages.status,
            FileDiagnosticStatus::Healthy
        );
    }

    #[test]
    fn reports_missing_source_control_files() {
        let fixture = Fixture::new("missing-rules");
        fixture.unity("{}");
        fs::remove_file(fixture.path().join(".gitignore")).expect("remove gitignore");
        fs::remove_file(fixture.path().join(".gitattributes")).expect("remove gitattributes");

        let result = inspect_project(fixture.path().to_str().expect("path")).expect("diagnostic");
        assert_eq!(result.status, ProjectStatus::NeedsAttention);
        assert_eq!(
            result.source_control.gitignore.status,
            FileDiagnosticStatus::Missing
        );
        assert_eq!(
            result.source_control.gitattributes.status,
            FileDiagnosticStatus::Missing
        );
    }

    #[test]
    fn identifies_vrchat_world_project() {
        let fixture = Fixture::new("world");
        fixture.unity(r#"{"com.vrchat.worlds":"3.10.4","com.vrchat.base":"3.10.4"}"#);

        let result = inspect_project(fixture.path().to_str().expect("path")).expect("diagnostic");
        assert_eq!(result.project_kind, ProjectKind::VrchatWorld);
    }

    #[test]
    fn reports_invalid_vpm_manifest() {
        let fixture = Fixture::new("invalid-vpm");
        fixture.unity("{}");
        fs::write(
            fixture.path().join("Packages/vpm-manifest.json"),
            "{invalid",
        )
        .expect("vpm manifest");

        let result = inspect_project(fixture.path().to_str().expect("path")).expect("diagnostic");
        assert_eq!(result.status, ProjectStatus::NeedsAttention);
        assert!(result
            .issues
            .iter()
            .any(|issue| issue.code == "PACKAGE_MANIFEST_INVALID_JSON"));
    }

    #[test]
    fn reports_large_file_candidates() {
        let fixture = Fixture::new("large-file");
        fixture.unity("{}");
        let large = fixture.path().join("Assets/large.psd");
        let file = fs::File::create(&large).expect("large file");
        file.set_len(LARGE_FILE_THRESHOLD_BYTES)
            .expect("sparse file");

        let result = inspect_project(fixture.path().to_str().expect("path")).expect("diagnostic");
        assert_eq!(result.source_control.large_files.len(), 1);
        assert_eq!(
            result.source_control.large_files[0].path,
            "Assets/large.psd"
        );
    }
}
