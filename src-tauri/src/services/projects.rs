use crate::{
    errors::{AppError, AppResult, ErrorCode},
    git::diagnostics as git_diagnostics,
    models::{
        ConfigFileDiagnostic, DiagnosticSeverity, FileDiagnosticStatus, ProjectDiagnostic,
        ProjectIssue, ProjectKind, ProjectStatus, RepositoryDiagnostic, SourceControlDiagnostic,
        VpmDiagnostic, VpmPackage, VpmTrackingPolicy,
    },
    platform::process::find_executable,
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub fn inspect_project(
    path: &str,
    vpm_tracking_policy: VpmTrackingPolicy,
) -> AppResult<ProjectDiagnostic> {
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
    let project_kind = classify_project(&vpm.packages, &mut issues)?;
    let repository = inspect_repository(&root, &mut issues);
    let source_control =
        inspect_source_control(&root, &vpm, &repository, vpm_tracking_policy, &mut issues);
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
                DiagnosticSeverity::Warning,
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
                DiagnosticSeverity::Warning,
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

fn classify_project(
    packages: &[VpmPackage],
    issues: &mut Vec<ProjectIssue>,
) -> AppResult<ProjectKind> {
    let has_avatar = packages
        .iter()
        .any(|item| item.name == "com.vrchat.avatars");
    let has_world = packages.iter().any(|item| item.name == "com.vrchat.worlds");
    let has_vrchat = packages
        .iter()
        .any(|item| item.name.starts_with("com.vrchat."));
    match (has_avatar, has_world, has_vrchat) {
        (true, false, _) => Ok(ProjectKind::VrchatAvatar),
        (false, true, _) => Ok(ProjectKind::VrchatWorld),
        (true, true, _) => Err(AppError::simple(
            ErrorCode::ProjectUnsupportedKind,
            "Avatar SDK と Worlds SDK が同居しているため、この project は取り扱えません。",
            "inspect_project",
        )),
        (false, false, true) => {
            issues.push(issue(
                "VRCHAT_PROJECT_KIND_UNKNOWN",
                DiagnosticSeverity::Warning,
                "VRChat package はありますが、Avatar / World を確定できません。",
                Some("Packages".to_owned()),
            ));
            Ok(ProjectKind::VrchatUnknown)
        }
        _ => Ok(ProjectKind::Unity),
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
            DiagnosticSeverity::Info,
            "Git repository root は Unity project の外側です。関連ファイルを同じrepositoryで管理する構成として扱います。",
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
    vpm_tracking_policy: VpmTrackingPolicy,
    issues: &mut Vec<ProjectIssue>,
) -> SourceControlDiagnostic {
    let gitignore = inspect_gitignore(root, issues);
    let vpm_packages =
        inspect_vpm_source_control(root, vpm, repository, vpm_tracking_policy, issues);
    SourceControlDiagnostic {
        gitignore,
        vpm_packages,
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
            DiagnosticSeverity::Warning,
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

fn inspect_vpm_source_control(
    root: &Path,
    vpm: &VpmDiagnostic,
    repository: &RepositoryDiagnostic,
    policy: VpmTrackingPolicy,
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
    let text = if path.is_file() {
        match fs::read_to_string(&path) {
            Ok(text) => Some(text),
            Err(_) => {
                issues.push(issue(
                    "VPM_GITIGNORE_UNREADABLE",
                    DiagnosticSeverity::Warning,
                    "Packages/.gitignore を読み取れません。",
                    Some(relative.clone()),
                ));
                return config(
                    relative,
                    FileDiagnosticStatus::NeedsAttention,
                    "読み取れません",
                );
            }
        }
    } else {
        None
    };
    let lines = text
        .as_deref()
        .map(|text| meaningful_lines(text).collect::<Vec<_>>())
        .unwrap_or_default();
    let excludes_vrchat = lines
        .iter()
        .any(|line| !line.starts_with('!') && line.contains("com.vrchat.") && line.contains('*'));
    let includes_resolver = lines.iter().any(|line| {
        line.starts_with('!')
            && (line.contains("com.vrchat.core.") || line.contains("com.vrchat.core.vpm-resolver"))
    });
    let tracked_vrchat_package = (repository.detected == Some(true))
        .then(|| find_executable("git"))
        .flatten()
        .and_then(|git| git_diagnostics::tracked_package_files(&git, root))
        .is_some_and(|files| {
            files.iter().any(|path| {
                let normalized = path.replace('\\', "/");
                normalized.starts_with("Packages/com.vrchat.")
                    && !normalized.starts_with("Packages/com.vrchat.core.vpm-resolver/")
            })
        });

    match policy {
        VpmTrackingPolicy::ExcludePackages => {
            let mut healthy = excludes_vrchat && includes_resolver;
            if text.is_none() {
                healthy = false;
                issues.push(issue(
                    "VPM_GITIGNORE_MISSING",
                    DiagnosticSeverity::Warning,
                    "設定ではVPM packageを除外しますが、Packages/.gitignore がありません。",
                    Some(relative.clone()),
                ));
            } else if !healthy {
                issues.push(issue(
                    "VPM_SOURCE_CONTROL_RULES_INCOMPLETE",
                    DiagnosticSeverity::Warning,
                    "VRChat package の除外、または VPM Resolver の例外ルールが不足しています。",
                    Some(relative.clone()),
                ));
            }
            if tracked_vrchat_package {
                healthy = false;
                issues.push(issue(
                    "VPM_PACKAGE_TRACKED",
                    DiagnosticSeverity::Warning,
                    "設定ではVPM packageを除外しますが、package本体がGitの追跡対象です。",
                    Some("Packages".to_owned()),
                ));
            }
            if healthy {
                config(
                    relative,
                    FileDiagnosticStatus::Healthy,
                    "設定どおりVPM packageを除外しています",
                )
            } else {
                config(
                    relative,
                    FileDiagnosticStatus::NeedsAttention,
                    "VPM package除外設定とprojectの状態が一致しません",
                )
            }
        }
        VpmTrackingPolicy::IncludePackages => {
            let mut healthy = !excludes_vrchat;
            if excludes_vrchat {
                issues.push(issue(
                    "VPM_PACKAGE_IGNORED",
                    DiagnosticSeverity::Warning,
                    "設定ではVPM packageをGit管理に含めますが、Packages/.gitignore で除外されています。",
                    Some(relative.clone()),
                ));
            }
            let installed_package_exists = vpm.packages.iter().any(|package| {
                package.name.starts_with("com.vrchat.")
                    && package.name != "com.vrchat.core.vpm-resolver"
                    && root.join("Packages").join(&package.name).is_dir()
            });
            if repository.detected == Some(true)
                && installed_package_exists
                && !tracked_vrchat_package
            {
                healthy = false;
                issues.push(issue(
                    "VPM_PACKAGE_NOT_TRACKED",
                    DiagnosticSeverity::Warning,
                    "設定ではVPM packageをGit管理に含めますが、package本体がまだ追跡されていません。",
                    Some("Packages".to_owned()),
                ));
            }
            if healthy {
                config(
                    relative,
                    FileDiagnosticStatus::Healthy,
                    "設定どおりVPM packageをGit管理に含められます",
                )
            } else {
                config(
                    relative,
                    FileDiagnosticStatus::NeedsAttention,
                    "VPM packageを含める設定とprojectの状態が一致しません",
                )
            }
        }
    }
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
        vpm_packages: config(
            "Packages/.gitignore".to_owned(),
            FileDiagnosticStatus::NotApplicable,
            "Unity project ではありません",
        ),
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
    use super::inspect_project;
    use crate::models::{
        DiagnosticSeverity, FileDiagnosticStatus, ProjectDiagnostic, ProjectKind, ProjectStatus,
        VpmTrackingPolicy,
    };
    use std::{
        fs,
        path::{Path, PathBuf},
        process::Command,
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
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn diagnose(fixture: &Fixture, policy: VpmTrackingPolicy) -> ProjectDiagnostic {
        inspect_project(fixture.path().to_str().expect("path"), policy).expect("diagnostic")
    }

    #[test]
    fn identifies_non_unity_folder() {
        let fixture = Fixture::new("not-unity");
        let result = diagnose(&fixture, VpmTrackingPolicy::ExcludePackages);
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

        let result = diagnose(&fixture, VpmTrackingPolicy::ExcludePackages);
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

        let result = diagnose(&fixture, VpmTrackingPolicy::ExcludePackages);
        assert_eq!(result.status, ProjectStatus::NeedsAttention);
        assert_eq!(
            result.source_control.gitignore.status,
            FileDiagnosticStatus::Missing
        );
    }

    #[test]
    fn identifies_vrchat_world_project() {
        let fixture = Fixture::new("world");
        fixture.unity(r#"{"com.vrchat.worlds":"3.10.4","com.vrchat.base":"3.10.4"}"#);

        let result = diagnose(&fixture, VpmTrackingPolicy::ExcludePackages);
        assert_eq!(result.project_kind, ProjectKind::VrchatWorld);
    }

    #[test]
    fn rejects_projects_with_avatar_and_world_sdks() {
        let fixture = Fixture::new("mixed-vrchat-project");
        fixture.unity(r#"{"com.vrchat.avatars":"3.10.4","com.vrchat.worlds":"3.10.4"}"#);

        let error = inspect_project(
            fixture.path().to_str().expect("path"),
            VpmTrackingPolicy::ExcludePackages,
        )
        .expect_err("mixed VRChat project must be rejected");
        assert_eq!(error.code, crate::errors::ErrorCode::ProjectUnsupportedKind);
        assert!(error.message.contains("Avatar SDK と Worlds SDK"));
    }

    #[test]
    fn reports_unreadable_settings_as_warnings() {
        let fixture = Fixture::new("unreadable-settings");
        fixture.unity("{}");
        fs::write(fixture.path().join(".gitignore"), [0xff, 0xfe]).expect("unreadable gitignore");
        fs::write(fixture.path().join("Packages/manifest.json"), [0xff, 0xfe])
            .expect("unreadable package manifest");

        let result = diagnose(&fixture, VpmTrackingPolicy::ExcludePackages);
        assert_eq!(result.status, ProjectStatus::NeedsAttention);
        assert!(result.issues.iter().any(|item| {
            item.code == "GITIGNORE_UNREADABLE" && item.severity == DiagnosticSeverity::Warning
        }));
        assert!(result.issues.iter().any(|item| {
            item.code == "PACKAGE_MANIFEST_UNREADABLE"
                && item.severity == DiagnosticSeverity::Warning
        }));
    }

    #[test]
    fn parent_repository_is_informational() {
        let fixture = Fixture::new("parent-repository");
        if !Command::new("git")
            .arg("--version")
            .status()
            .is_ok_and(|status| status.success())
        {
            return;
        }
        assert!(Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(fixture.path())
            .status()
            .expect("git init")
            .success());

        let project = fixture.path().join("UnityProject");
        fs::create_dir_all(project.join("Assets")).expect("Assets");
        fs::create_dir_all(project.join("ProjectSettings")).expect("ProjectSettings");
        fs::create_dir_all(project.join("Packages")).expect("Packages");
        fs::write(
            project.join("ProjectSettings/ProjectVersion.txt"),
            "m_EditorVersion: 2022.3.22f1\n",
        )
        .expect("ProjectVersion");
        fs::write(
            project.join("Packages/manifest.json"),
            r#"{"dependencies":{}}"#,
        )
        .expect("manifest");
        fs::write(project.join(".gitignore"), "[Ll]ibrary/\n[Tt]emp/\n").expect("gitignore");

        let result = inspect_project(
            project.to_str().expect("path"),
            VpmTrackingPolicy::ExcludePackages,
        )
        .expect("diagnostic");
        assert_eq!(result.status, ProjectStatus::Manageable);
        assert!(result.issues.iter().any(|issue| {
            issue.code == "GIT_ROOT_OUTSIDE_PROJECT" && issue.severity == DiagnosticSeverity::Info
        }));
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

        let result = diagnose(&fixture, VpmTrackingPolicy::ExcludePackages);
        assert_eq!(result.status, ProjectStatus::NeedsAttention);
        assert!(result
            .issues
            .iter()
            .any(|issue| issue.code == "PACKAGE_MANIFEST_INVALID_JSON"));
    }

    #[test]
    fn include_policy_accepts_vpm_packages_without_exclusion_rule() {
        let fixture = Fixture::new("include-vpm");
        fixture.unity(r#"{"com.vrchat.avatars":"3.10.4"}"#);
        fs::write(
            fixture.path().join("Packages/vpm-manifest.json"),
            r#"{"dependencies":{"com.vrchat.avatars":{"version":"3.10.4"}}}"#,
        )
        .expect("vpm manifest");

        let result = diagnose(&fixture, VpmTrackingPolicy::IncludePackages);
        assert_eq!(
            result.source_control.vpm_packages.status,
            FileDiagnosticStatus::Healthy
        );
    }

    #[test]
    fn include_policy_warns_when_vpm_packages_are_ignored() {
        let fixture = Fixture::new("include-vpm-ignored");
        fixture.unity(r#"{"com.vrchat.avatars":"3.10.4"}"#);
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

        let result = diagnose(&fixture, VpmTrackingPolicy::IncludePackages);
        assert!(result
            .issues
            .iter()
            .any(|issue| issue.code == "VPM_PACKAGE_IGNORED"));
    }
}
