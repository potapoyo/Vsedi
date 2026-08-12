use crate::{
    errors::{AppError, AppResult, ErrorCode},
    git::diagnostics,
    logging,
    models::{
        AppSettings, RecentProjectStatus, RepositorySettings, SettingsLoadResult,
        VpmTrackingPolicy, CURRENT_SCHEMA_VERSION,
    },
    platform::paths::app_data_dir,
    platform::process::find_executable,
    settings::migration::migrate,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use tracing::{info, warn};

pub fn load(app: &AppHandle) -> AppResult<SettingsLoadResult> {
    let directory = app_data_dir(app)?;
    fs::create_dir_all(&directory).map_err(|error| {
        AppError::from_io(
            ErrorCode::FilesystemWriteFailed,
            "create_app_data_dir",
            &directory,
            &error,
        )
    })?;
    let path = directory.join("settings.json");
    let (mut settings, recovered, backup_path) = load_file(&path)?;
    let normalized_log_level = logging::normalize_log_level(&settings.log_level).unwrap_or("INFO");
    if settings.log_level != normalized_log_level {
        warn!(
            operation = "normalize_log_level",
            configured_level = %settings.log_level,
            fallback_level = normalized_log_level,
            "invalid log level was replaced with INFO"
        );
        settings.log_level = normalized_log_level.to_owned();
    }
    logging::set_log_level(&settings.log_level)?;

    // The plugin store is the runtime persistence backend. The preflight above keeps
    // manual recovery and migration rules explicit before the store reads the file.
    let store = app.store("settings.json").map_err(|error| {
        AppError::with_detail(
            ErrorCode::SettingsReadFailed,
            "設定ストアを開けませんでした。",
            "open_settings_store",
            error.to_string(),
            false,
        )
    })?;
    store.reload_ignore_defaults().map_err(|error| {
        AppError::with_detail(
            ErrorCode::SettingsReadFailed,
            "設定ストアを再読込できませんでした。",
            "reload_settings_store",
            error.to_string(),
            false,
        )
    })?;

    let mut recent_projects = settings
        .recent_projects
        .iter()
        .map(|project| RecentProjectStatus {
            path: project.path.clone(),
            last_opened_at: project.last_opened_at.clone(),
            tags: project.tags.clone(),
            exists: Path::new(&project.path).is_dir(),
        })
        .collect::<Vec<_>>();
    sort_recent_projects(&mut recent_projects);
    Ok(SettingsLoadResult {
        settings,
        recovered,
        backup_path,
        recent_projects,
    })
}

pub fn save(app: &AppHandle, mut settings: AppSettings) -> AppResult<()> {
    if settings.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(AppError::simple(
            ErrorCode::SettingsUnsupportedSchema,
            "保存できる設定 schemaVersion ではありません。",
            "validate_settings_before_save",
        ));
    }
    let log_level = logging::normalize_log_level(&settings.log_level).ok_or_else(|| {
        AppError::simple(
            ErrorCode::SettingsInvalidLogLevel,
            "ログレベルは ERROR / WARN / INFO / DEBUG / TRACE のいずれかを指定してください。",
            "validate_settings_before_save",
        )
    })?;
    for project in &mut settings.recent_projects {
        let mut normalized_tags = Vec::with_capacity(project.tags.len());
        for tag in project.tags.drain(..) {
            let tag = tag.trim();
            if !tag.is_empty() && !normalized_tags.iter().any(|existing| existing == tag) {
                normalized_tags.push(tag.to_owned());
            }
        }
        project.tags = normalized_tags;
    }
    normalize_repository_settings(&mut settings.repository_settings);

    let store = app.store("settings.json").map_err(|error| {
        AppError::with_detail(
            ErrorCode::SettingsWriteFailed,
            "設定ストアを開けませんでした。",
            "open_settings_store",
            error.to_string(),
            false,
        )
    })?;
    store.set("schemaVersion", json!(settings.schema_version));
    store.set("onboardingCompleted", json!(settings.onboarding_completed));
    store.set(
        "recentProjects",
        serde_json::to_value(&settings.recent_projects).map_err(|error| {
            AppError::with_detail(
                ErrorCode::SettingsWriteFailed,
                "設定をシリアライズできませんでした。",
                "serialize_settings",
                error.to_string(),
                false,
            )
        })?,
    );
    store.set("logLevel", json!(log_level));
    store.set("vpmTrackingPolicy", json!(settings.vpm_tracking_policy));
    store.set(
        "ignoreTemplates",
        serde_json::to_value(&settings.ignore_templates).map_err(|error| {
            AppError::with_detail(
                ErrorCode::SettingsWriteFailed,
                "設定をシリアライズできませんでした。",
                "serialize_settings",
                error.to_string(),
                false,
            )
        })?,
    );
    store.set(
        "repositorySettings",
        serde_json::to_value(&settings.repository_settings).map_err(|error| {
            AppError::with_detail(
                ErrorCode::SettingsWriteFailed,
                "設定をシリアライズできませんでした。",
                "serialize_settings",
                error.to_string(),
                false,
            )
        })?,
    );
    store.save().map_err(|error| {
        AppError::with_detail(
            ErrorCode::SettingsWriteFailed,
            "設定を保存できませんでした。",
            "save_settings",
            error.to_string(),
            true,
        )
    })?;
    logging::set_log_level(log_level)?;
    info!(
        operation = "save_settings",
        log_level, "settings saved through Tauri Store"
    );
    Ok(())
}

pub fn resolve_vpm_tracking_policy_for_project(
    settings: &AppSettings,
    project_path: &str,
) -> VpmTrackingPolicy {
    let project = Path::new(project_path)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(project_path));
    let repository_root = find_executable("git")
        .and_then(|git| diagnostics::repository_root(&git, &project))
        .flatten();
    resolve_vpm_tracking_policy(settings, repository_root.as_deref())
}

pub fn resolve_vpm_tracking_policy(
    settings: &AppSettings,
    repository_root: Option<&str>,
) -> VpmTrackingPolicy {
    let Some(repository_root) = repository_root else {
        return settings.vpm_tracking_policy;
    };
    let Some(repository_root) = canonical_path(repository_root) else {
        return settings.vpm_tracking_policy;
    };
    settings
        .repository_settings
        .iter()
        .find(|entry| {
            canonical_path(&entry.repository_root)
                .is_some_and(|root| paths_equal(&root, &repository_root))
        })
        .and_then(|entry| entry.vpm_tracking_policy_override)
        .unwrap_or(settings.vpm_tracking_policy)
}

fn normalize_repository_settings(settings: &mut Vec<RepositorySettings>) {
    let mut normalized = Vec::with_capacity(settings.len());
    for mut entry in settings.drain(..) {
        let trimmed = entry.repository_root.trim();
        if trimmed.is_empty() {
            continue;
        }
        entry.repository_root = canonical_path(trimmed)
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| trimmed.to_owned());
        if let Some(existing) = normalized
            .iter_mut()
            .find(|item: &&mut RepositorySettings| {
                canonical_path(&item.repository_root).is_some_and(|root| {
                    canonical_path(&entry.repository_root)
                        .is_some_and(|entry_root| paths_equal(&root, &entry_root))
                })
            })
        {
            *existing = entry;
        } else {
            normalized.push(entry);
        }
    }
    *settings = normalized;
}

fn canonical_path(path: &str) -> Option<PathBuf> {
    Path::new(path).canonicalize().ok()
}

#[cfg(windows)]
fn paths_equal(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

#[cfg(not(windows))]
fn paths_equal(left: &Path, right: &Path) -> bool {
    left == right
}

fn sort_recent_projects(projects: &mut [RecentProjectStatus]) {
    projects.sort_by(|left, right| {
        right
            .last_opened_at
            .cmp(&left.last_opened_at)
            .then_with(|| left.path.cmp(&right.path))
    });
}

fn load_file(path: &Path) -> AppResult<(AppSettings, bool, Option<String>)> {
    if !path.exists() {
        let settings = AppSettings::default();
        write_json(path, &settings)?;
        return Ok((settings, false, None));
    }

    let raw = fs::read_to_string(path).map_err(|error| {
        AppError::from_io(ErrorCode::SettingsReadFailed, "read_settings", path, &error)
    })?;
    let parsed = match serde_json::from_str::<Value>(&raw) {
        Ok(value) => value,
        Err(_error) => {
            let backup = backup_before_recovery(path)?;
            let settings = AppSettings::default();
            write_json(path, &settings)?;
            warn!(operation = "recover_corrupt_settings", backup = %backup.display(), "corrupt settings were quarantined and regenerated");
            return Ok((settings, true, Some(backup.to_string_lossy().into_owned())));
        }
    };

    let Some(schema_version) = parsed
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
    else {
        let backup = backup_before_recovery(path)?;
        let settings = AppSettings::default();
        write_json(path, &settings)?;
        warn!(operation = "recover_invalid_settings_schema", backup = %backup.display(), "settings with an invalid schema were quarantined and regenerated");
        return Ok((settings, true, Some(backup.to_string_lossy().into_owned())));
    };
    let migrated = migrate(parsed, schema_version)?;
    if schema_version != CURRENT_SCHEMA_VERSION {
        let backup = backup_before_recovery(path)?;
        let settings: AppSettings = serde_json::from_value(migrated.clone()).map_err(|error| {
            AppError::with_detail(
                ErrorCode::SettingsInvalidJson,
                "設定の migration に失敗しました。",
                "migrate_settings",
                error.to_string(),
                false,
            )
        })?;
        write_json(path, &settings)?;
        return Ok((settings, true, Some(backup.to_string_lossy().into_owned())));
    }

    let settings = match serde_json::from_value(migrated) {
        Ok(settings) => settings,
        Err(error) => {
            let backup = backup_before_recovery(path)?;
            let default_settings = AppSettings::default();
            write_json(path, &default_settings)?;
            warn!(operation = "recover_invalid_settings", backup = %backup.display(), "invalid settings were quarantined and regenerated");
            info!(operation = "recover_invalid_settings", detail = %error, backup = %backup.display(), "invalid settings were quarantined and regenerated");
            return Ok((
                default_settings,
                true,
                Some(backup.to_string_lossy().into_owned()),
            ));
        }
    };
    Ok((settings, false, None))
}

fn backup_before_recovery(path: &Path) -> AppResult<PathBuf> {
    let backup = path.with_file_name(format!(
        "{}.bak.{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("settings.json"),
        timestamp()
    ));
    fs::copy(path, &backup).map_err(|error| {
        AppError::with_detail(
            ErrorCode::SettingsBackupFailed,
            "元の設定ファイルを退避できませんでした。",
            "backup_settings",
            format!("{}: {}", backup.display(), error),
            false,
        )
    })?;
    Ok(backup)
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> AppResult<()> {
    let parent = path.parent().ok_or_else(|| {
        AppError::simple(
            ErrorCode::SettingsWriteFailed,
            "設定ファイルの保存先が不正です。",
            "write_settings",
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        AppError::from_io(
            ErrorCode::SettingsWriteFailed,
            "create_settings_dir",
            parent,
            &error,
        )
    })?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        AppError::with_detail(
            ErrorCode::SettingsWriteFailed,
            "設定をシリアライズできませんでした。",
            "serialize_settings",
            error.to_string(),
            false,
        )
    })?;
    fs::write(path, bytes).map_err(|error| {
        AppError::from_io(
            ErrorCode::SettingsWriteFailed,
            "write_settings",
            path,
            &error,
        )
    })
}

fn timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[cfg(test)]
mod tests {
    use super::{
        backup_before_recovery, load_file, resolve_vpm_tracking_policy, sort_recent_projects,
    };
    use crate::errors::ErrorCode;
    use crate::models::{
        AppSettings, RepositorySettings, VpmTrackingPolicy, CURRENT_SCHEMA_VERSION,
    };
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "vsedi-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn missing_settings_are_created_with_current_schema() {
        let path = temp_path("missing").join("settings.json");
        let (settings, recovered, _) = load_file(&path).unwrap();
        assert_eq!(settings.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(!recovered);
        assert!(path.exists());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn future_schema_is_not_modified() {
        let path = temp_path("future").join("settings.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, r#"{"schemaVersion":99,"onboardingCompleted":true}"#).unwrap();
        let before = fs::read(&path).unwrap();
        let error = load_file(&path).unwrap_err();
        assert_eq!(error.code, ErrorCode::SettingsUnsupportedSchema);
        assert_eq!(before, fs::read(&path).unwrap());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn corrupt_json_is_backed_up_before_regeneration() {
        let path = temp_path("corrupt").join("settings.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not-json").unwrap();
        let (_, recovered, backup) = load_file(&path).unwrap();
        assert!(recovered);
        assert!(backup
            .as_ref()
            .is_some_and(|backup| std::path::Path::new(backup).exists()));
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn invalid_schema_is_backed_up_before_regeneration() {
        let path = temp_path("schema").join("settings.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, r#"{"onboardingCompleted":true}"#).unwrap();
        let (settings, recovered, backup) = load_file(&path).unwrap();
        assert_eq!(settings.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(recovered);
        assert!(backup
            .as_ref()
            .is_some_and(|backup| std::path::Path::new(backup).exists()));
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn backup_is_distinct_from_source() {
        let path = temp_path("backup").join("settings.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{}\n").unwrap();
        let backup = backup_before_recovery(&path).unwrap();
        assert_ne!(backup, path);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn managed_projects_are_sorted_by_latest_activity() {
        use crate::models::RecentProjectStatus;

        let mut projects = vec![
            RecentProjectStatus {
                path: "/older".to_owned(),
                last_opened_at: Some("2026-08-11T00:00:00Z".to_owned()),
                tags: Vec::new(),
                exists: true,
            },
            RecentProjectStatus {
                path: "/newer".to_owned(),
                last_opened_at: Some("2026-08-12T00:00:00Z".to_owned()),
                tags: vec!["Avatar".to_owned()],
                exists: true,
            },
            RecentProjectStatus {
                path: "/unknown".to_owned(),
                last_opened_at: None,
                tags: Vec::new(),
                exists: false,
            },
        ];

        sort_recent_projects(&mut projects);

        assert_eq!(projects[0].path, "/newer");
        assert_eq!(projects[1].path, "/older");
        assert_eq!(projects[2].path, "/unknown");
    }

    #[test]
    fn repository_policy_override_wins_over_global_default() {
        let root = temp_path("policy");
        fs::create_dir_all(&root).unwrap();
        let settings = AppSettings {
            vpm_tracking_policy: VpmTrackingPolicy::ExcludePackages,
            repository_settings: vec![RepositorySettings {
                repository_root: root.to_string_lossy().into_owned(),
                vpm_tracking_policy_override: Some(VpmTrackingPolicy::IncludePackages),
            }],
            ..AppSettings::default()
        };

        assert_eq!(
            resolve_vpm_tracking_policy(&settings, Some(root.to_str().unwrap())),
            VpmTrackingPolicy::IncludePackages
        );
        assert_eq!(
            resolve_vpm_tracking_policy(&settings, Some("/missing-repository")),
            VpmTrackingPolicy::ExcludePackages
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_repository_override_falls_back_to_global_default() {
        let settings = AppSettings::default();
        assert_eq!(
            resolve_vpm_tracking_policy(&settings, Some("/missing-repository")),
            VpmTrackingPolicy::ExcludePackages
        );
    }
}
