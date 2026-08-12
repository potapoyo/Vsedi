pub mod commands;
pub mod errors;
pub mod git;
pub mod logging;
pub mod models;
pub mod platform;
pub mod services;
pub mod settings;

use logging::LogGuard;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let guard = logging::initialize(app.handle())?;
            app.manage(LogGuard::new(guard));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::environment::inspect_environment,
            commands::projects::inspect_project,
            commands::initialization::preview_repository_initialization,
            commands::initialization::initialize_repository,
            commands::save::save_worktree,
            commands::history::read_history,
            commands::history::read_commit_detail,
            commands::diff::read_worktree_diff,
            commands::diff::read_commit_diff,
            commands::worktree::read_repository_state,
            commands::worktree::read_worktree_snapshot,
            commands::settings::load_settings,
            commands::settings::save_settings,
            commands::logging::export_diagnostic_log,
            commands::logging::open_log_directory,
            commands::logging::open_log_window,
            commands::logging::read_recent_logs
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vsedi");
}
