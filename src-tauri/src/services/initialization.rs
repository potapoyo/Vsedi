use crate::{
    errors::{AppError, AppResult, ErrorCode},
    git::{command::run_git, diagnostics},
    models::{IgnoreFilePreview, IgnoreTemplateSettings, RepositoryInitializationPreview, VpmTrackingPolicy},
    platform::process::find_executable,
};
use std::{fs, path::{Path, PathBuf}};

pub fn preview(project_path: &str, policy: VpmTrackingPolicy, templates: &IgnoreTemplateSettings) -> AppResult<RepositoryInitializationPreview> {
    let project = canonical_project(project_path)?;
    let Some(git) = find_executable("git") else { return Err(AppError::simple(ErrorCode::RepositoryInvalid, "System Git が見つかりません。", "preview_repository_initialization")); };
    let existing_root = diagnostics::repository_root(&git, &project)
        .ok_or_else(|| AppError::simple(ErrorCode::WorktreeReadFailed, "Git repository の状態を読み取れませんでした。", "preview_repository_initialization"))?;
    let blocking_reason = existing_root.map(|root| format!("すでに Git repository 内です: {root}"));
    let can_initialize = blocking_reason.is_none();
    let ignore_files = ignore_previews(&project, policy, templates)?;
    let token = initialization_token(&project, policy, templates, &ignore_files)?;
    Ok(RepositoryInitializationPreview { status_token: token, repository_root: project.to_string_lossy().into_owned(), can_initialize, blocking_reason, ignore_files })
}

pub fn initialize(project_path: &str, status_token: &str, policy: VpmTrackingPolicy, templates: &IgnoreTemplateSettings) -> AppResult<()> {
    let preview = preview(project_path, policy, templates)?;
    if !preview.can_initialize { return Err(AppError::simple(ErrorCode::RepositoryInvalid, "すでに Git 管理されているため、初期化できません。", "initialize_repository")); }
    if preview.status_token != status_token { return Err(AppError::simple(ErrorCode::RepositoryStateChanged, "初期化 preview 後に project または ignore template の状態が変わりました。内容を確認してから、もう一度実行してください。", "initialize_repository")); }
    let project = canonical_project(project_path)?;
    let Some(git) = find_executable("git") else { return Err(AppError::simple(ErrorCode::RepositoryInvalid, "System Git が見つかりません。", "initialize_repository")); };
    let output = run_git(&git, &["init"], Some(&project)).map_err(|error| AppError::with_detail(ErrorCode::RepositoryInitializeFailed, "Git repository を初期化できませんでした。", "git_init", error.to_string(), false))?;
    if output.status != Some(0) { return Err(AppError::with_detail(ErrorCode::RepositoryInitializeFailed, "Git repository を初期化できませんでした。", "git_init", output.stderr, false)); }
    for preview in &preview.ignore_files {
        if preview.missing_rules.is_empty() { continue; }
        append_rules(&project.join(&preview.path), &preview.missing_rules).map_err(|error| AppError::with_detail(ErrorCode::RepositoryInitializeFailed, ".gitignore の更新に失敗しました。repository は初期化済みの可能性があります。", "write_gitignore", error.to_string(), true))?;
    }
    Ok(())
}

fn ignore_previews(project: &Path, policy: VpmTrackingPolicy, templates: &IgnoreTemplateSettings) -> AppResult<Vec<IgnoreFilePreview>> {
    let mut previews = vec![ignore_preview(project, ".gitignore", &templates.unity_rules)?];
    if policy == VpmTrackingPolicy::ExcludePackages { previews.push(ignore_preview(project, "Packages/.gitignore", &templates.vpm_exclude_rules)?); }
    Ok(previews)
}
fn ignore_preview(project: &Path, relative: &str, required: &[String]) -> AppResult<IgnoreFilePreview> {
    let path = project.join(relative);
    let text = if path.exists() { fs::read_to_string(&path).map_err(|error| AppError::with_detail(ErrorCode::FilesystemReadFailed, ".gitignore を読み取れません。", "preview_repository_initialization", error.to_string(), false))? } else { String::new() };
    let missing_rules = required.iter().filter(|rule| !rule.trim().is_empty() && !contains_rule(&text, rule)).cloned().collect();
    Ok(IgnoreFilePreview { path: relative.to_owned(), missing_rules, will_create: !path.exists() })
}
fn contains_rule(text: &str, rule: &str) -> bool { text.lines().any(|line| line.trim() == rule) }
fn append_rules(path: &Path, rules: &[String]) -> std::io::Result<()> {
    let original = if path.exists() { fs::read_to_string(path)? } else { String::new() };
    let newline = if original.contains("\r\n") { "\r\n" } else { "\n" };
    let mut next = original;
    if !next.is_empty() && !next.ends_with('\n') { next.push_str(newline); }
    if !next.is_empty() && !next.ends_with(&(newline.to_owned() + newline)) { next.push_str(newline); }
    next.push_str("# Added by Vsedi for Unity project files"); next.push_str(newline);
    for rule in rules { next.push_str(rule); next.push_str(newline); }
    fs::write(path, next)
}
fn initialization_token(project: &Path, policy: VpmTrackingPolicy, templates: &IgnoreTemplateSettings, previews: &[IgnoreFilePreview]) -> AppResult<String> {
    let mut material = format!("{}:{policy:?}:{templates:?}", project.display());
    for preview in previews { material.push_str(&format!("|{}:{:?}:{}", preview.path, preview.missing_rules, preview.will_create)); if let Ok(bytes) = fs::read(project.join(&preview.path)) { material.push_str(&format!(":{:x}", checksum(&bytes))); } }
    Ok(format!("{:016x}", checksum(material.as_bytes())))
}
fn checksum(bytes: &[u8]) -> u64 { bytes.iter().fold(0xcbf29ce484222325u64, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)) }
fn canonical_project(path: &str) -> AppResult<PathBuf> { let requested = Path::new(path); if !requested.is_dir() { return Err(AppError::simple(ErrorCode::ProjectNotFound, "project folder を選択してください。", "repository_initialization")); } requested.canonicalize().map_err(|error| AppError::with_detail(ErrorCode::FilesystemReadFailed, "project folder を読み取れません。", "repository_initialization", error.to_string(), false)) }

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    #[test]
    fn appends_rules_without_replacing_existing_content_or_newline_style() {
        let root = std::env::temp_dir().join(format!("vsedi-init-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&root).unwrap(); let path = root.join(".gitignore"); fs::write(&path, "custom\r\n").unwrap();
        append_rules(&path, &["/[Ll]ibrary/".to_owned()]).unwrap(); let value = fs::read_to_string(&path).unwrap();
        assert!(value.starts_with("custom\r\n\r\n")); assert!(value.contains("/[Ll]ibrary/\r\n"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_a_stale_preview_without_initializing_repository() {
        let root = std::env::temp_dir().join(format!("vsedi-init-stale-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&root).unwrap();
        let templates = IgnoreTemplateSettings::default();
        let preview = preview(root.to_str().unwrap(), VpmTrackingPolicy::ExcludePackages, &templates).unwrap();
        fs::write(root.join(".gitignore"), "custom\n").unwrap();

        let error = initialize(root.to_str().unwrap(), &preview.status_token, VpmTrackingPolicy::ExcludePackages, &templates).unwrap_err();

        assert_eq!(error.code, ErrorCode::RepositoryStateChanged);
        assert!(!root.join(".git").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preview_blocks_duplicate_repository_initialization() {
        let root = std::env::temp_dir().join(format!("vsedi-init-existing-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&root).unwrap();
        let status = std::process::Command::new("git").args(["init"]).current_dir(&root).status().unwrap();
        assert!(status.success());

        let preview = preview(root.to_str().unwrap(), VpmTrackingPolicy::ExcludePackages, &IgnoreTemplateSettings::default()).unwrap();

        assert!(!preview.can_initialize);
        assert!(preview.blocking_reason.is_some());
        fs::remove_dir_all(root).unwrap();
    }
}
