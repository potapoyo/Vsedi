pub mod commands;
pub mod errors;
pub mod git;
pub mod logging;
pub mod models;
pub mod platform;
pub mod services;
pub mod settings;

use logging::LogGuard;
use tauri::{Listener, Manager};

pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(feature = "native-ui-test")]
    let builder = builder
        .plugin(tauri_plugin_wdio::init())
        .plugin(tauri_plugin_wdio_webdriver::init());

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let guard = logging::initialize(app.handle())?;
            app.manage(LogGuard::new(guard));

            // Keep the main window hidden until the React shell has painted once.
            // The static splash window is visible from startup and is replaced by
            // the main window when the frontend emits `app-ready`.
            let app_handle = app.handle().clone();
            app.listen("app-ready", move |_| {
                if let Some(main_window) = app_handle.get_webview_window("main") {
                    let _ = main_window.show();
                    let _ = main_window.set_focus();
                }
                if let Some(splash_window) = app_handle.get_webview_window("splashscreen") {
                    let _ = splash_window.close();
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::environment::inspect_environment,
            commands::projects::inspect_project,
            commands::initialization::preview_repository_initialization,
            commands::initialization::initialize_repository,
            commands::initialization::preview_ignore_rules,
            commands::initialization::apply_ignore_rules,
            commands::save::save_worktree,
            commands::history::read_history,
            commands::history::read_commit_detail,
            commands::diff::read_worktree_diff,
            commands::diff::read_commit_diff,
            commands::worktree::read_repository_state,
            commands::worktree::read_worktree_snapshot,
            commands::worktree::read_repository_tree,
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
