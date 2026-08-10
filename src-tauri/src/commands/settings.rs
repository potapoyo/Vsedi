use crate::{
    errors::AppResult,
    models::{AppSettings, SettingsLoadResult},
    settings,
};
use tauri::AppHandle;

#[tauri::command]
pub fn load_settings(app: AppHandle) -> AppResult<SettingsLoadResult> {
    settings::load(&app)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: AppSettings) -> AppResult<()> {
    settings::save(&app, settings)
}
