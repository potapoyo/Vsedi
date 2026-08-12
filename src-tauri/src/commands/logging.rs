use crate::{
    errors::{AppError, AppResult, ErrorCode},
    logging,
    models::LogSnapshot,
    platform::{paths::app_log_dir, process::open_directory},
};
use std::path::Path;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

#[tauri::command]
pub fn export_diagnostic_log(app: AppHandle, destination: String) -> AppResult<()> {
    let path = Path::new(&destination);
    if destination.trim().is_empty() {
        return Err(AppError::simple(
            ErrorCode::FilesystemWriteFailed,
            "診断ログの保存先を指定してください。",
            "export_diagnostic_log",
        ));
    }
    logging::export_diagnostic_log(&app, path)
}

#[tauri::command]
pub fn open_log_directory(app: AppHandle) -> AppResult<()> {
    let directory = app_log_dir(&app)?;
    std::fs::create_dir_all(&directory).map_err(|error| {
        AppError::from_io(
            ErrorCode::FilesystemWriteFailed,
            "create_log_dir",
            &directory,
            &error,
        )
    })?;
    open_directory(&directory).map_err(|error| {
        AppError::with_detail(
            ErrorCode::FilesystemReadFailed,
            "ログフォルダを開けませんでした。",
            "open_log_directory",
            error.to_string(),
            false,
        )
    })
}

#[tauri::command]
pub async fn open_log_window(app: AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window("logs") {
        window.show().map_err(|error| {
            AppError::with_detail(
                ErrorCode::InternalError,
                "ログウィンドウを表示できませんでした。",
                "show_log_window",
                error.to_string(),
                false,
            )
        })?;
        window.set_focus().map_err(|error| {
            AppError::with_detail(
                ErrorCode::InternalError,
                "ログウィンドウにフォーカスできませんでした。",
                "focus_log_window",
                error.to_string(),
                false,
            )
        })?;
        return Ok(());
    }

    WebviewWindowBuilder::new(&app, "logs", WebviewUrl::App("index.html".into()))
        .title("Vsedi ログ")
        .inner_size(980.0, 640.0)
        .min_inner_size(640.0, 420.0)
        .build()
        .map(|_| ())
        .map_err(|error| {
            AppError::with_detail(
                ErrorCode::InternalError,
                "ログウィンドウを開けませんでした。",
                "open_log_window",
                error.to_string(),
                false,
            )
        })
}

#[tauri::command]
pub fn read_recent_logs(app: AppHandle) -> AppResult<LogSnapshot> {
    // The log window is a diagnostic tool. Show every retained, redacted line
    // for now so that changing the log level is immediately observable.
    logging::read_recent_logs(&app, usize::MAX)
}
