use crate::errors::{AppError, AppResult, ErrorCode};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub fn app_data_dir(app: &AppHandle) -> AppResult<PathBuf> {
    app.path().app_data_dir().map_err(|error| {
        AppError::with_detail(
            ErrorCode::FilesystemWriteFailed,
            "アプリデータ領域を解決できませんでした。",
            "resolve_app_data_dir",
            error.to_string(),
            false,
        )
    })
}

pub fn app_log_dir(app: &AppHandle) -> AppResult<PathBuf> {
    app.path().app_log_dir().map_err(|error| {
        AppError::with_detail(
            ErrorCode::FilesystemWriteFailed,
            "ログ領域を解決できませんでした。",
            "resolve_app_log_dir",
            error.to_string(),
            false,
        )
    })
}
