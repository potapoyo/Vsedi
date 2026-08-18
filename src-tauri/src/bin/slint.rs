#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use slint::SharedString;
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
