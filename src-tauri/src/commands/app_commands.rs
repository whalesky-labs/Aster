use tauri::State;

use crate::app::state::AppState;
use crate::domain::runtime::{RuntimeConfig, RuntimeMode};
use crate::domain::status::{AppStatus, AuditLogRow, SaveSystemSettingsRequest, SystemSettings};
use crate::error::AppResult;
use crate::services::status_service;

#[tauri::command]
pub fn get_runtime_config(state: State<'_, AppState>) -> AppResult<RuntimeConfig> {
    status_service::get_runtime_config(&state)
}

#[tauri::command]
pub fn set_runtime_mode(mode: String, state: State<'_, AppState>) -> AppResult<RuntimeConfig> {
    let mode = RuntimeMode::parse(&mode)?;
    status_service::set_runtime_mode(&state, mode)
}

#[tauri::command]
pub fn get_app_status(app: tauri::AppHandle, state: State<'_, AppState>) -> AppResult<AppStatus> {
    let version = app.package_info().version.to_string();
    status_service::get_app_status(&state, &version)
}

#[tauri::command]
pub fn get_system_settings(state: State<'_, AppState>) -> AppResult<SystemSettings> {
    status_service::get_system_settings(&state)
}

#[tauri::command]
pub fn list_audit_logs(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> AppResult<Vec<AuditLogRow>> {
    status_service::list_audit_logs(&state, limit)
}

#[tauri::command]
pub fn save_system_settings(
    state: State<'_, AppState>,
    request: SaveSystemSettingsRequest,
) -> AppResult<SystemSettings> {
    status_service::save_system_settings(&state, request)
}
