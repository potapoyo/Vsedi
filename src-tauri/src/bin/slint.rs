#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use slint::SharedString;
use std::thread;
use vsedi_lib::{
    application,
    models::{ChangeKind, DiagnosticSeverity, ProjectDiagnostic, ProjectStatus, VpmTrackingPolicy},
};

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let window = MainWindow::new()?;
    // The content surface is intentionally light. Do this before showing the
    // window so standard widgets do not briefly render with the OS dark theme.
    window.invoke_force_light_theme();
    window.set_environment_status(SharedString::from(environment_text()));

    let weak_window = window.as_weak();
    window.on_refresh_application(move || {
        if let Some(window) = weak_window.upgrade() {
            window.set_environment_status(SharedString::from(environment_text()));
        }
    });

    let weak_window = window.as_weak();
    window.on_inspect_project(move |path| {
        let Some(window) = weak_window.upgrade() else {
            return;
        };
        let result =
            application::inspect_project(path.as_str(), VpmTrackingPolicy::ExcludePackages)
                .map(|diagnostic| project_text(&diagnostic))
                .unwrap_or_else(|error| format!("{} ({:?})", error.message, error.code));
        window.set_project_status(SharedString::from(result));
    });

    let weak_window = window.as_weak();
    window.on_refresh_worktree(move |path| {
        let path = path.to_string();
        let weak_window = weak_window.clone();
        thread::spawn(move || {
            let result = application::read_worktree_snapshot(&path)
                .map(worktree_text)
                .map_err(|error| format!("{} ({:?})", error.message, error.code));
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak_window.upgrade() {
                    match result {
                        Ok((summary, token, worktree_files)) => {
                            window.set_worktree_status(SharedString::from(summary));
                            window.set_worktree_files(SharedString::from(worktree_files));
                            window.set_worktree_token(SharedString::from(token));
                        }
                        Err(error) => {
                            window.set_worktree_status(SharedString::from(error));
                            window.set_worktree_files(SharedString::from(
                                "変更一覧を読み込めませんでした。",
                            ));
                            window.set_worktree_token(SharedString::new());
                        }
                    }
                }
            });
        });
    });

    let weak_window = window.as_weak();
    window.on_save_project(move |path, status_token, memo| {
        let path = path.to_string();
        let status_token = status_token.to_string();
        let memo = memo.to_string();
        let weak_window = weak_window.clone();
        thread::spawn(move || {
            let request = vsedi_lib::models::SaveRequest {
                project_path: path,
                status_token,
                memo,
            };
            let result = application::save_worktree(request, |_| {})
                .map(|saved| {
                    format!(
                        "保存しました: {} / {}件",
                        saved.short_commit_id, saved.file_count
                    )
                })
                .unwrap_or_else(|error| format!("{} ({:?})", error.message, error.code));
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak_window.upgrade() {
                    window.set_save_status(SharedString::from(result));
                }
            });
        });
    });

    let weak_window = window.as_weak();
    window.on_refresh_history(move |path| {
        let path = path.to_string();
        let weak_window = weak_window.clone();
        thread::spawn(move || {
            let result = application::read_history(&path, 0)
                .map(history_text)
                .unwrap_or_else(|error| format!("{} ({:?})", error.message, error.code));
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak_window.upgrade() {
                    window.set_history_status(SharedString::from(result));
                }
            });
        });
    });

    let weak_window = window.as_weak();
    window.on_pick_project(move || {
        let weak_window = weak_window.clone();
        thread::spawn(move || {
            let path = rfd::FileDialog::new()
                .set_title("Unity projectを選択")
                .pick_folder()
                .map(|path| path.to_string_lossy().into_owned());
            let _ = slint::invoke_from_event_loop(move || {
                if let (Some(window), Some(path)) = (weak_window.upgrade(), path) {
                    window.set_project_path(SharedString::from(path));
                }
            });
        });
    });

    window.run()?;
    Ok(())
}

fn environment_text() -> String {
    match application::inspect_environment() {
        Ok(diagnostic) => format!(
            "{} / {} — {}\nSystem Git: {}",
            diagnostic.platform.os,
            diagnostic.platform.architecture,
            if diagnostic.platform.supported {
                "対応対象"
            } else {
                "対象外"
            },
            diagnostic
                .git
                .version
                .unwrap_or_else(|| "利用できません".to_owned())
        ),
        Err(error) => format!("環境診断に失敗しました: {}", error.message),
    }
}

fn project_text(diagnostic: &ProjectDiagnostic) -> String {
    let issue_count = diagnostic
        .issues
        .iter()
        .filter(|issue| {
            matches!(
                issue.severity,
                DiagnosticSeverity::Warning | DiagnosticSeverity::Error
            )
        })
        .count();
    let status = match diagnostic.status {
        ProjectStatus::Manageable => "管理可能",
        ProjectStatus::NeedsAttention => "要確認",
        ProjectStatus::NotUnity => "Unity projectではありません",
    };
    format!(
        "{}\n{} / Unity {} / 問題 {}件",
        diagnostic.path,
        status,
        diagnostic.unity_version.as_deref().unwrap_or("不明"),
        issue_count
    )
}

fn worktree_text(snapshot: vsedi_lib::models::WorktreeSnapshot) -> (String, String, String) {
    let mut summary = format!("変更 {}件", snapshot.files.len());
    if snapshot.has_conflicts {
        summary.push_str(" / 競合あり");
    }
    if snapshot.has_existing_staged_changes {
        summary.push_str(" / 既存のstaged変更あり");
    }
    let mut files = snapshot
        .files
        .iter()
        .take(5)
        .map(|file| format!("{}  {}", change_kind_text(&file.change_kind), file.path))
        .collect::<Vec<_>>();
    if snapshot.files.len() > 5 {
        files.push(format!("…さらに {}件あります。", snapshot.files.len() - 5));
    }
    if files.is_empty() {
        files.push("保存対象の変更はありません。".to_owned());
    }
    (summary, snapshot.status_token, files.join("\n"))
}

fn change_kind_text(kind: &ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Added => "追加",
        ChangeKind::Modified => "変更",
        ChangeKind::Deleted => "削除",
        ChangeKind::Renamed => "名前変更",
        ChangeKind::Copied => "複製",
        ChangeKind::TypeChanged => "種類変更",
        ChangeKind::Unmerged => "競合",
        ChangeKind::Untracked => "未管理",
    }
}

fn history_text(page: vsedi_lib::models::HistoryPage) -> String {
    if page.entries.is_empty() {
        return "保存履歴はありません。".to_owned();
    }
    let mut lines = page
        .entries
        .iter()
        .map(|entry| format!("{}  {}", entry.short_commit_id, entry.memo))
        .collect::<Vec<_>>();
    if page.next_offset.is_some() {
        lines.push("…さらに履歴があります。".to_owned());
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_primary_controls_to_the_slint_testing_backend() {
        i_slint_backend_testing::init_no_event_loop();
        let app = MainWindow::new().expect("Slint window should be constructible");
        let controls =
            i_slint_backend_testing::ElementHandle::find_by_accessible_label(&app, "Projectを診断")
                .collect::<Vec<_>>();
        assert_eq!(controls.len(), 1);

        let pickers = i_slint_backend_testing::ElementHandle::find_by_accessible_label(
            &app,
            "フォルダを選択",
        )
        .collect::<Vec<_>>();
        assert_eq!(pickers.len(), 1);
    }

    #[test]
    fn formats_changed_files_for_the_graphical_work_card() {
        let snapshot = vsedi_lib::models::WorktreeSnapshot {
            status_token: "token".to_owned(),
            files: vec![vsedi_lib::models::ChangedFile {
                path: "Assets/Scene.unity".to_owned(),
                old_path: None,
                change_kind: ChangeKind::Modified,
                staged: false,
                unstaged: true,
                binary: false,
                outside_project: false,
            }],
            has_conflicts: false,
            has_existing_staged_changes: false,
        };

        let (summary, token, files) = worktree_text(snapshot);
        assert_eq!(summary, "変更 1件");
        assert_eq!(token, "token");
        assert_eq!(files, "変更  Assets/Scene.unity");
    }
}
