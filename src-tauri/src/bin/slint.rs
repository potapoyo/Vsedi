#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use slint::SharedString;
use std::thread;
use vsedi_lib::{
    application,
    models::{DiagnosticSeverity, ProjectDiagnostic, ProjectStatus, VpmTrackingPolicy},
};

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let window = MainWindow::new()?;
    window.set_environment_status(SharedString::from(environment_text()));

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
                        Ok((summary, token)) => {
                            window.set_worktree_status(SharedString::from(summary));
                            window.set_worktree_token(SharedString::from(token));
                        }
                        Err(error) => {
                            window.set_worktree_status(SharedString::from(error));
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

fn worktree_text(snapshot: vsedi_lib::models::WorktreeSnapshot) -> (String, String) {
    let mut summary = format!("変更 {}件", snapshot.files.len());
    if snapshot.has_conflicts {
        summary.push_str(" / 競合あり");
    }
    if snapshot.has_existing_staged_changes {
        summary.push_str(" / 既存のstaged変更あり");
    }
    (summary, snapshot.status_token)
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
    }
}
