use tauri::State;

use crate::app::state::AppState;
use crate::domain::reports::{ExportReportResult, ReportBundle, ReportQuery};
use crate::error::AppResult;
use crate::services::report_service;

#[tauri::command]
pub fn get_report_bundle(
    query: ReportQuery,
    state: State<'_, AppState>,
) -> AppResult<ReportBundle> {
    report_service::get_report_bundle(&state, query)
}

#[tauri::command]
pub fn export_monthly_report(
    query: ReportQuery,
    state: State<'_, AppState>,
) -> AppResult<ExportReportResult> {
    report_service::export_monthly_report(&state, query)
}
