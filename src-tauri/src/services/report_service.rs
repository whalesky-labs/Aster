use rust_xlsxwriter::{Format, Workbook, XlsxError};

use crate::app::state::AppState;
use crate::db::report_repository;
use crate::domain::reports::{ExportReportResult, ReportBundle, ReportBundlePage, ReportQuery};
use crate::domain::runtime::RuntimeMode;
use crate::error::{AppError, AppResult};

pub fn get_report_bundle(state: &AppState, query: ReportQuery) -> AppResult<ReportBundle> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    let scoped_query = scoped_query(state, query)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_get_report_bundle(state, scoped_query);
    }
    state
        .db
        .with_conn(|conn| report_repository::get_report_bundle(conn, &scoped_query))
}

pub fn get_report_bundle_page(
    state: &AppState,
    query: ReportQuery,
    section: String,
    cursor: Option<String>,
) -> AppResult<ReportBundlePage> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    let scoped_query = scoped_query(state, query)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_get_report_bundle_page(
            state,
            scoped_query,
            section,
            cursor,
        );
    }
    state.db.with_conn(|conn| {
        report_repository::get_report_bundle_page(conn, &scoped_query, &section, cursor.as_deref())
    })
}

fn scoped_query(state: &AppState, query: ReportQuery) -> AppResult<ReportQuery> {
    validate_month(&query.month)?;
    validate_optional_date(query.start_date.as_deref(), "开始日期")?;
    validate_optional_date(query.end_date.as_deref(), "结束日期")?;
    if let (Some(start_date), Some(end_date)) = (&query.start_date, &query.end_date) {
        if start_date > end_date {
            return Err(AppError::Validation("开始日期不能晚于结束日期".to_string()));
        }
    }
    let start_date = query
        .start_date
        .as_deref()
        .map(|date| format!("{date} 00:00:00"));
    let end_date = query
        .end_date
        .as_deref()
        .map(|date| format!("{date} 23:59:59"));
    Ok(ReportQuery {
        month: query.month,
        start_date,
        end_date,
        department_id: crate::services::user_service::current_department_scope(state)?
            .or(query.department_id),
        category_id: query.category_id,
        item_id: query.item_id,
        supplier_id: query.supplier_id,
    })
}

fn runtime_mode(state: &AppState) -> AppResult<RuntimeMode> {
    Ok(crate::services::status_service::get_runtime_config(state)?.mode)
}

pub fn export_monthly_report(
    state: &AppState,
    query: ReportQuery,
) -> AppResult<ExportReportResult> {
    let bundle = get_report_bundle(state, query)?;
    let export_dir = crate::services::status_service::effective_export_dir(state)?;
    std::fs::create_dir_all(&export_dir)?;
    let path = export_dir.join(format!("Aster-月度报表-{}.xlsx", bundle.month));
    write_report_workbook(&bundle, &path)?;
    Ok(ExportReportResult {
        path: path.display().to_string(),
    })
}

fn write_report_workbook(bundle: &ReportBundle, path: &std::path::Path) -> AppResult<()> {
    let mut workbook = Workbook::new();
    let header = Format::new().set_bold().set_background_color("#DDEBF7");
    let money = Format::new().set_num_format("#,##0.00");
    let number = Format::new().set_num_format("#,##0.00");

    write_summary_sheets(&mut workbook, bundle, &header, &money, &number)?;
    write_detail_sheets(&mut workbook, bundle, &header, &money, &number)?;
    write_inventory_control_sheets(&mut workbook, bundle, &header, &money, &number)?;
    workbook.save(path).map_err(map_xlsx)?;
    Ok(())
}

include!("report_service/summary_sheets.rs");
include!("report_service/detail_sheets.rs");
include!("report_service/inventory_sheets.rs");

fn write_headers(
    sheet: &mut rust_xlsxwriter::Worksheet,
    header: &Format,
    labels: &[&str],
) -> AppResult<()> {
    for (idx, label) in labels.iter().enumerate() {
        sheet
            .write_string_with_format(0, idx as u16, *label, header)
            .map_err(map_xlsx)?;
    }
    Ok(())
}

fn validate_month(month: &str) -> AppResult<()> {
    let bytes = month.as_bytes();
    if bytes.len() == 7 && bytes[4] == b'-' {
        Ok(())
    } else {
        Err(AppError::Validation("月份格式必须是 YYYY-MM".to_string()))
    }
}

fn validate_optional_date(date: Option<&str>, label: &str) -> AppResult<()> {
    let Some(date) = date else {
        return Ok(());
    };
    let bytes = date.as_bytes();
    if bytes.len() == 10 && bytes[4] == b'-' && bytes[7] == b'-' {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "{label}格式必须是 YYYY-MM-DD"
        )))
    }
}

fn stocktake_scope_label(scope_type: &str) -> &'static str {
    match scope_type {
        "category" => "分类",
        "custom" => "自定义",
        _ => "全部",
    }
}

fn stocktake_status_label(status: &str) -> &'static str {
    match status {
        "confirmed" => "已确认",
        "voided" => "已作废",
        "counting" => "盘点中",
        _ => "草稿",
    }
}

fn map_xlsx(error: XlsxError) -> AppError {
    AppError::Validation(format!("Excel 导出失败：{error}"))
}

#[cfg(test)]
#[path = "report_service/tests.rs"]
mod tests;
