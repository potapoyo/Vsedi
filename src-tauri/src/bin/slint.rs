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
    window.on_navigate(move |page| {
        if let Some(window) = weak_window.upgrade() {
            window.set_current_page(page);
        }
    });

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
                .map(worktree_view)
                .map_err(|error| format!("{} ({:?})", error.message, error.code));
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak_window.upgrade() {
                    match result {
                        Ok(view) => {
                            window.set_worktree_status(SharedString::from(view.summary));
                            window.set_worktree_files(SharedString::from(view.files_text));
                            window
                                .set_worktree_empty_message(SharedString::from(view.empty_message));
                            window.set_worktree_token(SharedString::from(view.token));
                            set_worktree_rows(&window, &view.rows);
                        }
                        Err(error) => {
                            window.set_worktree_status(SharedString::from(error));
                            window.set_worktree_files(SharedString::from(
                                "変更一覧を読み込めませんでした。",
                            ));
                            window.set_worktree_empty_message(SharedString::from(
                                "変更一覧を読み込めませんでした。",
                            ));
                            window.set_worktree_token(SharedString::new());
                            set_worktree_rows(&window, &[]);
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
                .map(history_view)
                .map_err(|error| format!("{} ({:?})", error.message, error.code));
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak_window.upgrade() {
                    match result {
                        Ok((summary, empty_message, rows)) => {
                            window.set_history_status(SharedString::from(summary));
                            window.set_history_empty_message(SharedString::from(empty_message));
                            set_history_rows(&window, &rows);
                        }
                        Err(error) => {
                            window.set_history_status(SharedString::from(error.clone()));
                            window.set_history_empty_message(SharedString::from(error));
                            set_history_rows(&window, &[]);
                        }
                    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorktreeView {
    summary: String,
    token: String,
    files_text: String,
    empty_message: String,
    rows: Vec<(String, String)>,
}

fn worktree_view(snapshot: vsedi_lib::models::WorktreeSnapshot) -> WorktreeView {
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
    let rows = snapshot
        .files
        .iter()
        .take(5)
        .map(|file| {
            (
                change_kind_text(&file.change_kind).to_owned(),
                file.path.clone(),
            )
        })
        .collect::<Vec<_>>();
    let empty_message = if rows.is_empty() {
        "保存対象の変更はありません。".to_owned()
    } else {
        String::new()
    };
    WorktreeView {
        summary,
        token: snapshot.status_token,
        files_text: files.join("\n"),
        empty_message,
        rows,
    }
}

fn set_worktree_rows(window: &MainWindow, rows: &[(String, String)]) {
    let (status, path) = rows.first().cloned().unwrap_or_default();
    window.set_worktree_file_1_status(SharedString::from(status));
    window.set_worktree_file_1_path(SharedString::from(path));
    let (status, path) = rows.get(1).cloned().unwrap_or_default();
    window.set_worktree_file_2_status(SharedString::from(status));
    window.set_worktree_file_2_path(SharedString::from(path));
    let (status, path) = rows.get(2).cloned().unwrap_or_default();
    window.set_worktree_file_3_status(SharedString::from(status));
    window.set_worktree_file_3_path(SharedString::from(path));
    let (status, path) = rows.get(3).cloned().unwrap_or_default();
    window.set_worktree_file_4_status(SharedString::from(status));
    window.set_worktree_file_4_path(SharedString::from(path));
    let (status, path) = rows.get(4).cloned().unwrap_or_default();
    window.set_worktree_file_5_status(SharedString::from(status));
    window.set_worktree_file_5_path(SharedString::from(path));
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

fn history_view(page: vsedi_lib::models::HistoryPage) -> (String, String, Vec<(String, String)>) {
    let rows = page
        .entries
        .iter()
        .take(6)
        .map(|entry| (entry.short_commit_id.clone(), entry.memo.clone()))
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return (
            "保存履歴はありません。".to_owned(),
            "保存履歴はありません。".to_owned(),
            rows,
        );
    }
    let more = if page.next_offset.is_some() {
        "（さらに履歴があります）"
    } else {
        ""
    };
    let summary = format!("{}件の保存履歴{}", page.entries.len(), more);
    (summary, String::new(), rows)
}

fn set_history_rows(window: &MainWindow, rows: &[(String, String)]) {
    let (commit_id, memo) = rows.first().cloned().unwrap_or_default();
    window.set_history_entry_1_id(SharedString::from(commit_id));
    window.set_history_entry_1_memo(SharedString::from(memo));
    let (commit_id, memo) = rows.get(1).cloned().unwrap_or_default();
    window.set_history_entry_2_id(SharedString::from(commit_id));
    window.set_history_entry_2_memo(SharedString::from(memo));
    let (commit_id, memo) = rows.get(2).cloned().unwrap_or_default();
    window.set_history_entry_3_id(SharedString::from(commit_id));
    window.set_history_entry_3_memo(SharedString::from(memo));
    let (commit_id, memo) = rows.get(3).cloned().unwrap_or_default();
    window.set_history_entry_4_id(SharedString::from(commit_id));
    window.set_history_entry_4_memo(SharedString::from(memo));
    let (commit_id, memo) = rows.get(4).cloned().unwrap_or_default();
    window.set_history_entry_5_id(SharedString::from(commit_id));
    window.set_history_entry_5_memo(SharedString::from(memo));
    let (commit_id, memo) = rows.get(5).cloned().unwrap_or_default();
    window.set_history_entry_6_id(SharedString::from(commit_id));
    window.set_history_entry_6_memo(SharedString::from(memo));
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

        for label in ["ホーム", "保存履歴", "設定"] {
            let navigation =
                i_slint_backend_testing::ElementHandle::find_by_accessible_label(&app, label)
                    .collect::<Vec<_>>();
            assert!(
                !navigation.is_empty(),
                "navigation item should be present: {label}"
            );
        }
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

        let view = worktree_view(snapshot);
        assert_eq!(view.summary, "変更 1件");
        assert_eq!(view.token, "token");
        assert_eq!(
            view.rows,
            vec![("変更".to_owned(), "Assets/Scene.unity".to_owned())]
        );
    }

    #[test]
    fn formats_history_entries_for_graphical_rows() {
        let page = vsedi_lib::models::HistoryPage {
            entries: vec![vsedi_lib::models::HistoryEntry {
                commit_id: "abcdef123456".to_owned(),
                short_commit_id: "abcdef1".to_owned(),
                memo: "保存メモ".to_owned(),
                author_time: "2026-08-19T00:00:00Z".to_owned(),
            }],
            next_offset: None,
        };

        let (summary, empty_message, rows) = history_view(page);
        assert_eq!(summary, "1件の保存履歴");
        assert!(empty_message.is_empty());
        assert_eq!(rows, vec![("abcdef1".to_owned(), "保存メモ".to_owned())]);
    }

    #[test]
    fn navigation_actions_select_the_history_page() {
        i_slint_backend_testing::init_no_event_loop();
        let app = MainWindow::new().expect("Slint window should be constructible");
        let weak_app = app.as_weak();
        app.on_navigate(move |page| {
            if let Some(app) = weak_app.upgrade() {
                app.set_current_page(page);
            }
        });

        let history =
            i_slint_backend_testing::ElementHandle::find_by_accessible_label(&app, "保存履歴")
                .next()
                .expect("history navigation should be present");
        history.invoke_accessible_default_action();
        assert_eq!(app.get_current_page(), "HISTORY");
    }
}
