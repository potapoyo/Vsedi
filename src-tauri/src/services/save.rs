use crate::{
    errors::{AppError, AppResult, ErrorCode},
    git::{command::run_git, diagnostics},
    models::{SaveRequest, SaveResult},
    platform::process::find_executable,
    services::worktree,
};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

pub fn save(request: SaveRequest) -> AppResult<SaveResult> {
    debug!(operation = "save_worktree", project_path = %request.project_path, "worktree save started");
    let memo = request.memo.trim();
    if memo.is_empty() {
        return Err(AppError::simple(
            ErrorCode::SaveMemoInvalid,
            "保存メモを入力してください。",
            "save_worktree",
        ));
    }
    let project = canonical_project(&request.project_path)?;
    let Some(git) = find_executable("git") else {
        return Err(AppError::simple(
            ErrorCode::RepositoryInvalid,
            "System Git が見つかりません。",
            "save_worktree",
        ));
    };
    let root = diagnostics::repository_root(&git, &project)
        .flatten()
        .ok_or_else(|| {
            AppError::simple(
                ErrorCode::RepositoryInvalid,
                "この project は Git 管理されていません。",
                "save_worktree",
            )
        })?;
    let root = PathBuf::from(root);
    let snapshot = worktree::read_worktree_snapshot(&request.project_path)?;
    if snapshot.status_token != request.status_token {
        warn!(
            operation = "save_worktree",
            reason = "state_changed",
            "save stopped because the preview is stale"
        );
        return Err(AppError::simple(
            ErrorCode::RepositoryStateChanged,
            "表示後に変更内容が変わりました。もう一度確認してから保存してください。",
            "save_worktree",
        ));
    }
    if snapshot.has_conflicts {
        warn!(
            operation = "save_worktree",
            reason = "conflict",
            "save stopped because conflicts are present"
        );
        return Err(AppError::simple(
            ErrorCode::SaveConflict,
            "競合中のファイルがあるため、作業を保存できません。",
            "save_worktree",
        ));
    }
    if snapshot.has_existing_staged_changes {
        warn!(
            operation = "save_worktree",
            reason = "existing_staged_changes",
            "save stopped because staged changes already exist"
        );
        return Err(AppError::simple(
            ErrorCode::SaveExistingStagedChanges,
            "すでに Git のステージにある変更があるため、安全のため保存できません。",
            "save_worktree",
        ));
    }
    if snapshot.files.is_empty() {
        debug!(
            operation = "save_worktree",
            reason = "no_changes",
            "save skipped because the worktree is clean"
        );
        return Err(AppError::simple(
            ErrorCode::SaveNoChanges,
            "保存する変更がありません。",
            "save_worktree",
        ));
    }
    info!(
        operation = "save_worktree",
        file_count = snapshot.files.len(),
        "saving worktree changes"
    );
    let add = run_git(&git, &["add", "-A"], Some(&root)).map_err(|error| {
        AppError::with_detail(
            ErrorCode::SaveAddFailed,
            "変更を保存準備できませんでした。",
            "git_add",
            error.to_string(),
            false,
        )
    })?;
    if add.status != Some(0) {
        return Err(AppError::with_detail(
            ErrorCode::SaveAddFailed,
            "変更を保存準備できませんでした。",
            "git_add",
            add.stderr,
            true,
        ));
    }
    let commit = run_git(&git, &["commit", "-m", memo], Some(&root)).map_err(|error| {
        AppError::with_detail(
            ErrorCode::SaveCommitFailed,
            "保存 commit を作成できませんでした。変更が保存準備済みになっている可能性があります。",
            "git_commit",
            error.to_string(),
            true,
        )
    })?;
    if commit.status != Some(0) {
        return Err(AppError::with_detail(
            ErrorCode::SaveCommitFailed,
            "保存 commit を作成できませんでした。変更が保存準備済みになっている可能性があります。",
            "git_commit",
            commit.stderr,
            true,
        ));
    }
    let commit_id = git_required_text(
        &git,
        &["rev-parse", "HEAD"],
        &root,
        "commit ID を確認できませんでした。",
    )?;
    let author_time = git_required_text(
        &git,
        &["show", "-s", "--format=%aI", "HEAD"],
        &root,
        "保存時刻を確認できませんでした。",
    )?;
    let result = SaveResult {
        short_commit_id: commit_id.chars().take(8).collect(),
        commit_id,
        memo: memo.to_owned(),
        author_time,
        file_count: snapshot.files.len(),
    };
    info!(operation = "save_worktree", commit_id = %result.short_commit_id, file_count = result.file_count, "worktree save completed");
    Ok(result)
}
fn canonical_project(path: &str) -> AppResult<PathBuf> {
    let requested = Path::new(path);
    if !requested.is_dir() {
        return Err(AppError::simple(
            ErrorCode::ProjectNotFound,
            "project folder を選択してください。",
            "save_worktree",
        ));
    }
    requested.canonicalize().map_err(|error| {
        AppError::with_detail(
            ErrorCode::FilesystemReadFailed,
            "project folder を読み取れません。",
            "save_worktree",
            error.to_string(),
            false,
        )
    })
}
fn git_required_text(
    git: &Path,
    args: &[&str],
    root: &Path,
    message: &'static str,
) -> AppResult<String> {
    let output = run_git(git, args, Some(root)).map_err(|error| {
        AppError::with_detail(
            ErrorCode::SaveCommitFailed,
            message,
            "verify_save",
            error.to_string(),
            true,
        )
    })?;
    let value = output.stdout.trim();
    if output.status != Some(0) || value.is_empty() {
        return Err(AppError::with_detail(
            ErrorCode::SaveCommitFailed,
            message,
            "verify_save",
            output.stderr,
            true,
        ));
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .unwrap();
        assert!(status.success());
    }

    fn repository(root: &Path) {
        fs::create_dir_all(root).unwrap();
        git(root, &["init"]);
        git(root, &["config", "user.name", "Vsedi test"]);
        git(root, &["config", "user.email", "test@example.invalid"]);
    }

    fn head(root: &Path) -> String {
        String::from_utf8(
            Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(root)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_owned()
    }

    #[test]
    fn saves_a_previewed_worktree_as_a_commit() {
        let root = std::env::temp_dir().join(format!(
            "vsedi-save-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init"]);
        git(&root, &["config", "user.name", "Vsedi test"]);
        git(&root, &["config", "user.email", "test@example.invalid"]);
        fs::write(root.join("scene with space.txt"), "saved\n").unwrap();
        let snapshot = worktree::read_worktree_snapshot(root.to_str().unwrap()).unwrap();
        let result = save(SaveRequest {
            project_path: root.to_string_lossy().into_owned(),
            status_token: snapshot.status_token,
            memo: "初回保存".to_owned(),
        })
        .unwrap();
        assert_eq!(result.file_count, 1);
        assert_eq!(result.memo, "初回保存");
        assert_eq!(result.commit_id.len(), 40);
        assert!(worktree::read_worktree_snapshot(root.to_str().unwrap())
            .unwrap()
            .files
            .is_empty());
        let history = crate::services::history::read_history(root.to_str().unwrap()).unwrap();
        assert_eq!(
            history.first().map(|entry| entry.commit_id.as_str()),
            Some(result.commit_id.as_str())
        );
        let detail =
            crate::services::history::read_commit_detail(root.to_str().unwrap(), &result.commit_id)
                .unwrap();
        assert_eq!(detail.files.len(), 1);
        let diff = crate::services::diff::read_commit_diff(
            root.to_str().unwrap(),
            &result.commit_id,
            "scene with space.txt",
        )
        .unwrap();
        assert_eq!(diff.kind, crate::models::FileDiffKind::Text);
        assert!(diff.patch.unwrap().contains("saved"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn initialization_save_history_and_detail_form_one_safe_flow() {
        let root = std::env::temp_dir().join(format!(
            "vsedi-m3-flow-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("Packages")).unwrap();
        let templates = crate::models::IgnoreTemplateSettings {
            unity_rules: vec!["Library/".to_owned()],
            vpm_exclude_rules: vec!["com.vrchat.*".to_owned()],
        };
        let preview = crate::services::initialization::preview(
            root.to_str().unwrap(),
            crate::models::VpmTrackingPolicy::ExcludePackages,
            &templates,
        )
        .unwrap();
        crate::services::initialization::initialize(
            root.to_str().unwrap(),
            &preview.status_token,
            crate::models::VpmTrackingPolicy::ExcludePackages,
            &templates,
        )
        .unwrap();
        git(&root, &["config", "user.name", "Vsedi test"]);
        git(&root, &["config", "user.email", "test@example.invalid"]);
        fs::write(root.join("scene.txt"), "saved through M3\n").unwrap();

        let snapshot = worktree::read_worktree_snapshot(root.to_str().unwrap()).unwrap();
        let result = save(SaveRequest {
            project_path: root.to_string_lossy().into_owned(),
            status_token: snapshot.status_token,
            memo: "M3 flow".to_owned(),
        })
        .unwrap();
        let history = crate::services::history::read_history(root.to_str().unwrap()).unwrap();
        let detail =
            crate::services::history::read_commit_detail(root.to_str().unwrap(), &result.commit_id)
                .unwrap();

        assert_eq!(
            history.first().map(|entry| entry.commit_id.as_str()),
            Some(result.commit_id.as_str())
        );
        assert_eq!(
            history.first().map(|entry| entry.memo.as_str()),
            Some("M3 flow")
        );
        assert!(detail.files.iter().any(|file| file.path == "scene.txt"));
        assert!(worktree::read_worktree_snapshot(root.to_str().unwrap())
            .unwrap()
            .files
            .is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refuses_existing_staged_changes_without_mutating_head() {
        let root = std::env::temp_dir().join(format!(
            "vsedi-save-staged-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        repository(&root);
        fs::write(root.join("baseline.txt"), "baseline\n").unwrap();
        git(&root, &["add", "baseline.txt"]);
        git(&root, &["commit", "-m", "baseline"]);
        let before = head(&root);
        fs::write(root.join("staged.txt"), "staged\n").unwrap();
        git(&root, &["add", "staged.txt"]);
        let snapshot = worktree::read_worktree_snapshot(root.to_str().unwrap()).unwrap();

        let error = save(SaveRequest {
            project_path: root.to_string_lossy().into_owned(),
            status_token: snapshot.status_token,
            memo: "拒否される保存".to_owned(),
        })
        .unwrap_err();

        assert_eq!(error.code, ErrorCode::SaveExistingStagedChanges);
        assert_eq!(head(&root), before);
        assert!(root.join("staged.txt").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refuses_a_stale_preview_before_git_add() {
        let root = std::env::temp_dir().join(format!(
            "vsedi-save-stale-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        repository(&root);
        fs::write(root.join("baseline.txt"), "baseline\n").unwrap();
        git(&root, &["add", "baseline.txt"]);
        git(&root, &["commit", "-m", "baseline"]);
        fs::write(root.join("scene.txt"), "before preview\n").unwrap();
        let snapshot = worktree::read_worktree_snapshot(root.to_str().unwrap()).unwrap();
        fs::write(root.join("scene.txt"), "after preview\n").unwrap();

        let error = save(SaveRequest {
            project_path: root.to_string_lossy().into_owned(),
            status_token: snapshot.status_token,
            memo: "状態変化".to_owned(),
        })
        .unwrap_err();

        assert_eq!(error.code, ErrorCode::RepositoryStateChanged);
        assert!(!root.join(".git/index.lock").exists());
        assert!(worktree::read_worktree_snapshot(root.to_str().unwrap())
            .unwrap()
            .files
            .iter()
            .any(|file| file.path == "scene.txt"));
        fs::remove_dir_all(root).unwrap();
    }
}
