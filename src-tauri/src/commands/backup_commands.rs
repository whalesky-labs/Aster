use tauri::State;

use crate::app::state::AppState;
use crate::domain::backups::{
    BackupRecord, BackupSummary, CreateBackupRequest, RestoreBackupRequest, RestorePreview,
    RestoreResult, SetSecondBackupDirRequest,
};
use crate::error::AppResult;
use crate::services::backup_service;

#[tauri::command]
pub fn create_backup(
    state: State<'_, AppState>,
    request: CreateBackupRequest,
) -> AppResult<BackupSummary> {
    backup_service::create_backup(&state, request)
}

#[tauri::command]
pub fn list_backup_records(state: State<'_, AppState>) -> AppResult<Vec<BackupRecord>> {
    backup_service::list_backup_records(&state)
}

#[tauri::command]
pub fn set_second_backup_dir(
    state: State<'_, AppState>,
    request: SetSecondBackupDirRequest,
) -> AppResult<String> {
    backup_service::set_second_backup_dir(&state, request)
}

#[tauri::command]
pub fn preview_restore_backup(
    state: State<'_, AppState>,
    backup_file: String,
) -> AppResult<RestorePreview> {
    backup_service::preview_restore_backup(&state, backup_file)
}

#[tauri::command]
pub fn restore_backup(
    state: State<'_, AppState>,
    request: RestoreBackupRequest,
) -> AppResult<RestoreResult> {
    backup_service::restore_backup(&state, request)
}
