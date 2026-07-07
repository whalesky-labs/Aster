use tauri::State;

use crate::app::state::AppState;
use crate::domain::imports::{
    ExportImportTemplateResult, ImportPreview, ImportPreviewRequest, ImportResult, RunImportRequest,
};
use crate::error::AppResult;
use crate::services::import_service;

#[tauri::command]
pub fn preview_excel_import(
    state: State<'_, AppState>,
    request: ImportPreviewRequest,
) -> AppResult<ImportPreview> {
    import_service::preview_excel_import(&state, request)
}

#[tauri::command]
pub fn export_import_template(state: State<'_, AppState>) -> AppResult<ExportImportTemplateResult> {
    import_service::export_import_template(&state)
}

#[tauri::command]
pub fn run_excel_import(
    state: State<'_, AppState>,
    request: RunImportRequest,
) -> AppResult<ImportResult> {
    import_service::run_excel_import(&state, request)
}
