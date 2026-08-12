use crate::{errors::AppResult, models::{InitializeRepositoryRequest, RepositoryInitializationPreview}, services::initialization, settings};
use tauri::AppHandle;
#[tauri::command]
pub fn preview_repository_initialization(app: AppHandle, project_path: String) -> AppResult<RepositoryInitializationPreview> { let settings = settings::load(&app)?.settings; initialization::preview(&project_path, settings.vpm_tracking_policy, &settings.ignore_templates) }
#[tauri::command]
pub fn initialize_repository(app: AppHandle, request: InitializeRepositoryRequest) -> AppResult<()> { let settings = settings::load(&app)?.settings; initialization::initialize(&request.project_path, &request.status_token, settings.vpm_tracking_policy, &settings.ignore_templates) }
