use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use calamine::{open_workbook_auto, Data, Reader};
use rusqlite::{params, Connection, OptionalExtension};
use rust_xlsxwriter::{Format, Workbook, XlsxError};
use serde::Serialize;
use uuid::Uuid;

use crate::app::state::AppState;
use crate::db::stock_repository;
use crate::domain::imports::{
    ExportImportTemplateResult, ImportItemPreview, ImportMessage, ImportMonthPreview,
    ImportPreview, ImportPreviewRequest, ImportResult, RunImportRequest,
};
use crate::domain::stock::{SubmitStockDocumentLine, SubmitStockDocumentRequest};
use crate::error::{AppError, AppResult};
use crate::services::backup_service;

const ITEM_SHEET: &str = "物品档案";
const INBOUND_SHEET: &str = "入库明细";
const OUTBOUND_SHEET: &str = "出库明细";
const TEMPLATE_FILE_NAME: &str = "Aster-Excel导入模板.xlsx";

pub fn preview_excel_import(
    state: &AppState,
    request: ImportPreviewRequest,
) -> AppResult<ImportPreview> {
    crate::services::safety_service::require_local_primary_database(state, "预览 Excel 导入")?;
    let parsed = parse_import_template_workbook(&request.path)?;
    state
        .db
        .with_conn(|conn| build_preview(conn, &request.path, parsed))
}

pub fn export_import_template(state: &AppState) -> AppResult<ExportImportTemplateResult> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    let export_dir = crate::services::status_service::effective_export_dir(state)?;
    fs::create_dir_all(&export_dir)?;
    let path = export_dir.join(TEMPLATE_FILE_NAME);
    write_import_template_workbook(&path)?;
    Ok(ExportImportTemplateResult {
        path: path.display().to_string(),
    })
}

pub fn run_excel_import(state: &AppState, request: RunImportRequest) -> AppResult<ImportResult> {
    crate::services::safety_service::require_dangerous_local_operation(state, "正式导入 Excel")?;
    let mode = ImportMode::from_request(request.mode.as_deref())?;
    let parsed = parse_import_template_workbook(&request.path)?;
    if !parsed.errors.is_empty() {
        return Err(AppError::Validation(format!(
            "导入预览存在 {} 个错误，请修正 Excel 后再导入",
            parsed.errors.len()
        )));
    }

    let source_file = request.path.clone();
    let preflight_preview = state
        .db
        .with_conn(|conn| build_preview_from_parts(conn, &source_file, &parsed))?;
    if !preflight_preview.errors.is_empty() {
        return Err(AppError::Validation(format!(
            "导入预览存在 {} 个错误，请修正 Excel 后再导入",
            preflight_preview.errors.len()
        )));
    }

    backup_service::create_backup_of_type(state, "before_import")?;
    let allow_negative_stock = crate::services::status_service::allow_negative_stock(state)?;

    state.db.with_conn_mut(|conn| {
        let job_id = new_id();
        let preview = build_preview_from_parts(conn, &source_file, &parsed)?;
        if !preview.errors.is_empty() {
            return Err(AppError::Validation(format!(
                "导入预览存在 {} 个错误，请修正 Excel 后再导入",
                preview.errors.len()
            )));
        }
        let result = import_parsed_workbook(
            conn,
            &source_file,
            &job_id,
            &parsed,
            &preview,
            mode,
            allow_negative_stock,
        )?;
        let source_copy_path =
            write_import_source_copy(&state.paths.import_report_dir, &source_file)?;
        let report_path = write_import_report(
            &state.paths.import_report_dir,
            &result,
            &preview,
            mode,
            source_copy_path.as_deref(),
        )?;
        Ok(ImportResult {
            report_path: Some(report_path.display().to_string()),
            source_copy_path: source_copy_path.map(|path| path.display().to_string()),
            ..result
        })
    })
}

include!("import_service/parser.rs");

#[cfg(test)]
#[path = "import_service/tests.rs"]
mod tests;
