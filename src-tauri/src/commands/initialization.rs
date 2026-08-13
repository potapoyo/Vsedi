use crate::{
    errors::AppResult,
    models::{
        ApplyIgnoreRulesRequest, InitializeRepositoryRequest, RepositoryIgnorePreview,
        RepositoryInitializationPreview,
    },
    services::initialization,
    settings,
};
use tauri::AppHandle;
#[tauri::command]
pub fn preview_repository_initialization(
    app: AppHandle,
    project_path: String,
) -> AppResult<RepositoryInitializationPreview> {
    let settings = settings::load(&app)?.settings;
    let policy = settings::resolve_vpm_tracking_policy_for_project(&settings, &project_path);
    initialization::preview(&project_path, policy, &settings.ignore_templates)
}
#[tauri::command]
pub fn initialize_repository(
    app: AppHandle,
    request: InitializeRepositoryRequest,
) -> AppResult<()> {
    let settings = settings::load(&app)?.settings;
    let policy =
        settings::resolve_vpm_tracking_policy_for_project(&settings, &request.project_path);
    initialization::initialize(
        &request.project_path,
        &request.status_token,
        policy,
        &settings.ignore_templates,
    )
}

#[tauri::command]
pub fn preview_ignore_rules(
    app: AppHandle,
    project_path: String,
) -> AppResult<RepositoryIgnorePreview> {
    let settings = settings::load(&app)?.settings;
    let policy = settings::resolve_vpm_tracking_policy_for_project(&settings, &project_path);
    initialization::preview_ignore_rules(&project_path, policy, &settings.ignore_templates)
}

#[tauri::command]
pub fn apply_ignore_rules(app: AppHandle, request: ApplyIgnoreRulesRequest) -> AppResult<()> {
    let settings = settings::load(&app)?.settings;
    let policy =
        settings::resolve_vpm_tracking_policy_for_project(&settings, &request.project_path);
    initialization::apply_ignore_rules(request, policy, &settings.ignore_templates)
}
