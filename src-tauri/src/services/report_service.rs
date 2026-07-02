use rust_xlsxwriter::{Format, Workbook, XlsxError};

use crate::app::state::AppState;
use crate::db::report_repository;
use crate::domain::reports::{ExportReportResult, ReportBundle, ReportQuery};
use crate::domain::runtime::RuntimeMode;
use crate::error::{AppError, AppResult};

pub fn get_report_bundle(state: &AppState, query: ReportQuery) -> AppResult<ReportBundle> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    validate_month(&query.month)?;
    validate_optional_date(query.start_date.as_deref(), "开始日期")?;
    validate_optional_date(query.end_date.as_deref(), "结束日期")?;
    if let (Some(start_date), Some(end_date)) = (&query.start_date, &query.end_date) {
        if start_date > end_date {
            return Err(AppError::Validation("开始日期不能晚于结束日期".to_string()));
        }
    }
    let scoped_query = ReportQuery {
        month: query.month,
        start_date: query.start_date,
        end_date: query.end_date,
        department_id: crate::services::user_service::current_department_scope(state)?
            .or(query.department_id),
        category_id: query.category_id,
        item_id: query.item_id,
        supplier_id: query.supplier_id,
    };
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_get_report_bundle(state, scoped_query);
    }
    state
        .db
        .with_conn(|conn| report_repository::get_report_bundle(conn, &scoped_query))
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

    let sheet = workbook.add_worksheet();
    sheet.set_name("月度进销存").map_err(map_xlsx)?;
    write_headers(
        sheet,
        &header,
        &[
            "编码",
            "物品",
            "规格",
            "单位",
            "入库数量",
            "入库金额",
            "出库数量",
            "出库金额",
            "结存数量",
            "结存金额",
        ],
    )?;
    for (idx, row) in bundle.monthly_inventory.iter().enumerate() {
        let r = (idx + 1) as u32;
        sheet.write_string(r, 0, &row.item_code).map_err(map_xlsx)?;
        sheet.write_string(r, 1, &row.item_name).map_err(map_xlsx)?;
        sheet
            .write_string(r, 2, row.spec.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 3, row.unit_name.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 4, row.inbound_quantity, &number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 5, row.inbound_amount, &money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 6, row.outbound_quantity, &number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 7, row.outbound_amount, &money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 8, row.ending_quantity, &number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 9, row.ending_amount, &money)
            .map_err(map_xlsx)?;
    }
    sheet.autofit();

    let sheet = workbook.add_worksheet();
    sheet.set_name("部门领用汇总").map_err(map_xlsx)?;
    write_headers(sheet, &header, &["部门", "数量", "金额"])?;
    for (idx, row) in bundle.department_summary.iter().enumerate() {
        let r = (idx + 1) as u32;
        sheet
            .write_string(r, 0, &row.department_name)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 1, row.quantity, &number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 2, row.amount, &money)
            .map_err(map_xlsx)?;
    }
    sheet.autofit();

    let sheet = workbook.add_worksheet();
    sheet.set_name("部门领用明细").map_err(map_xlsx)?;
    write_headers(
        sheet,
        &header,
        &[
            "日期", "部门", "编码", "物品", "规格", "单位", "数量", "单价", "金额", "单号", "用途",
            "备注",
        ],
    )?;
    for (idx, row) in bundle.department_details.iter().enumerate() {
        let r = (idx + 1) as u32;
        sheet
            .write_string(r, 0, &row.movement_date)
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 1, &row.department_name)
            .map_err(map_xlsx)?;
        sheet.write_string(r, 2, &row.item_code).map_err(map_xlsx)?;
        sheet.write_string(r, 3, &row.item_name).map_err(map_xlsx)?;
        sheet
            .write_string(r, 4, row.spec.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 5, row.unit_name.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 6, row.quantity, &number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 7, row.unit_price, &money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 8, row.amount, &money)
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 9, row.document_no.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 10, row.purpose.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 11, row.remark.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
    }
    sheet.autofit();

    let sheet = workbook.add_worksheet();
    sheet.set_name("分类消耗统计").map_err(map_xlsx)?;
    write_headers(sheet, &header, &["分类", "数量", "金额"])?;
    for (idx, row) in bundle.category_consumption.iter().enumerate() {
        let r = (idx + 1) as u32;
        sheet
            .write_string(r, 0, &row.category_name)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 1, row.quantity, &number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 2, row.amount, &money)
            .map_err(map_xlsx)?;
    }
    sheet.autofit();

    let sheet = workbook.add_worksheet();
    sheet.set_name("物品消耗排行").map_err(map_xlsx)?;
    write_headers(
        sheet,
        &header,
        &["编码", "物品", "规格", "单位", "数量", "金额"],
    )?;
    for (idx, row) in bundle.item_consumption_ranking.iter().enumerate() {
        let r = (idx + 1) as u32;
        sheet.write_string(r, 0, &row.item_code).map_err(map_xlsx)?;
        sheet.write_string(r, 1, &row.item_name).map_err(map_xlsx)?;
        sheet
            .write_string(r, 2, row.spec.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 3, row.unit_name.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 4, row.quantity, &number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 5, row.amount, &money)
            .map_err(map_xlsx)?;
    }
    sheet.autofit();

    let sheet = workbook.add_worksheet();
    sheet.set_name("入库明细").map_err(map_xlsx)?;
    write_headers(
        sheet,
        &header,
        &[
            "日期",
            "供应商",
            "编码",
            "物品",
            "规格",
            "单位",
            "数量",
            "单价",
            "金额",
            "单号",
            "备注",
        ],
    )?;
    for (idx, row) in bundle.inbound_details.iter().enumerate() {
        let r = (idx + 1) as u32;
        sheet
            .write_string(r, 0, &row.movement_date)
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 1, &row.supplier_name)
            .map_err(map_xlsx)?;
        sheet.write_string(r, 2, &row.item_code).map_err(map_xlsx)?;
        sheet.write_string(r, 3, &row.item_name).map_err(map_xlsx)?;
        sheet
            .write_string(r, 4, row.spec.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 5, row.unit_name.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 6, row.quantity, &number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 7, row.unit_price, &money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 8, row.amount, &money)
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 9, row.document_no.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 10, row.remark.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
    }
    sheet.autofit();

    let sheet = workbook.add_worksheet();
    sheet.set_name("出库明细").map_err(map_xlsx)?;
    write_headers(
        sheet,
        &header,
        &[
            "日期", "部门", "编码", "物品", "规格", "单位", "数量", "单价", "金额", "单号", "用途",
            "备注",
        ],
    )?;
    for (idx, row) in bundle.outbound_details.iter().enumerate() {
        let r = (idx + 1) as u32;
        sheet
            .write_string(r, 0, &row.movement_date)
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 1, &row.department_name)
            .map_err(map_xlsx)?;
        sheet.write_string(r, 2, &row.item_code).map_err(map_xlsx)?;
        sheet.write_string(r, 3, &row.item_name).map_err(map_xlsx)?;
        sheet
            .write_string(r, 4, row.spec.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 5, row.unit_name.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 6, row.quantity, &number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 7, row.unit_price, &money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 8, row.amount, &money)
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 9, row.document_no.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 10, row.purpose.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 11, row.remark.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
    }
    sheet.autofit();

    let sheet = workbook.add_worksheet();
    sheet.set_name("库存余额").map_err(map_xlsx)?;
    write_headers(
        sheet,
        &header,
        &[
            "编码",
            "物品",
            "规格",
            "单位",
            "当前库存",
            "库存金额",
            "移动均价",
            "最近入库价",
            "预警线",
            "状态",
        ],
    )?;
    for (idx, row) in bundle.stock_balances.iter().enumerate() {
        let r = (idx + 1) as u32;
        sheet.write_string(r, 0, &row.item_code).map_err(map_xlsx)?;
        sheet.write_string(r, 1, &row.item_name).map_err(map_xlsx)?;
        sheet
            .write_string(r, 2, row.spec.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 3, row.unit_name.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 4, row.quantity, &number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 5, row.amount, &money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 6, row.average_price, &money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 7, row.last_inbound_price, &money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 8, row.warning_quantity, &number)
            .map_err(map_xlsx)?;
        sheet
            .write_string(
                r,
                9,
                match row.stock_status.as_str() {
                    "negative" => "负库存",
                    "low" => "低库存",
                    _ => "正常",
                },
            )
            .map_err(map_xlsx)?;
    }
    sheet.autofit();

    let sheet = workbook.add_worksheet();
    sheet.set_name("库存预警").map_err(map_xlsx)?;
    write_headers(
        sheet,
        &header,
        &[
            "编码",
            "物品",
            "规格",
            "单位",
            "当前库存",
            "预警线",
            "缺口数量",
            "库存金额",
        ],
    )?;
    for (idx, row) in bundle.stock_warnings.iter().enumerate() {
        let r = (idx + 1) as u32;
        sheet.write_string(r, 0, &row.item_code).map_err(map_xlsx)?;
        sheet.write_string(r, 1, &row.item_name).map_err(map_xlsx)?;
        sheet
            .write_string(r, 2, row.spec.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 3, row.unit_name.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 4, row.quantity, &number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 5, row.warning_quantity, &number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 6, row.shortage_quantity, &number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 7, row.amount, &money)
            .map_err(map_xlsx)?;
    }
    sheet.autofit();

    let sheet = workbook.add_worksheet();
    sheet.set_name("盘点差异").map_err(map_xlsx)?;
    write_headers(
        sheet,
        &header,
        &[
            "日期",
            "单号",
            "范围",
            "状态",
            "编码",
            "物品",
            "规格",
            "单位",
            "账面数",
            "实盘数",
            "差异数",
            "移动均价",
            "差异金额",
            "备注",
        ],
    )?;
    for (idx, row) in bundle.stocktake_differences.iter().enumerate() {
        let r = (idx + 1) as u32;
        sheet
            .write_string(r, 0, &row.business_date)
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 1, &row.document_no)
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 2, stocktake_scope_label(&row.scope_type))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 3, stocktake_status_label(&row.status))
            .map_err(map_xlsx)?;
        sheet.write_string(r, 4, &row.item_code).map_err(map_xlsx)?;
        sheet.write_string(r, 5, &row.item_name).map_err(map_xlsx)?;
        sheet
            .write_string(r, 6, row.spec.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 7, row.unit_name.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 8, row.book_quantity, &number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 9, row.counted_quantity, &number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 10, row.difference_quantity, &number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 11, row.average_price, &money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 12, row.difference_amount, &money)
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 13, row.remark.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
    }
    sheet.autofit();

    workbook.save(path).map_err(map_xlsx)?;
    Ok(())
}

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
mod tests {
    use std::sync::{Arc, Mutex};

    use zip::ZipArchive;

    use super::*;
    use crate::app::paths::AppPaths;
    use crate::db::connection::Db;
    use crate::db::repository;
    use crate::domain::reports::{
        CategoryConsumptionRow, DepartmentIssueDetailRow, DepartmentIssueSummaryRow,
        InboundDetailRow, ItemConsumptionRow, MonthlyInventoryRow, StockBalanceReportRow,
        StockWarningRow, StocktakeDifferenceReportRow,
    };
    use crate::domain::users::{CurrentUser, Role};

    fn report_test_state() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().expect("temp dir");
        let paths = AppPaths {
            data_dir: dir.path().to_path_buf(),
            database_path: dir.path().join("aster.sqlite"),
            backup_dir: dir.path().join("backups"),
            export_dir: dir.path().join("exports"),
            import_report_dir: dir.path().join("import-reports"),
        };
        std::fs::create_dir_all(&paths.backup_dir).unwrap();
        std::fs::create_dir_all(&paths.export_dir).unwrap();
        std::fs::create_dir_all(&paths.import_report_dir).unwrap();
        let state = AppState {
            db: Db::initialize(&paths).unwrap(),
            paths,
            session: Arc::new(Mutex::new(Some(CurrentUser {
                id: "user-report".to_string(),
                username: "report".to_string(),
                display_name: "报表用户".to_string(),
                department_id: None,
                department_name: None,
                roles: vec![Role {
                    id: "role-report".to_string(),
                    code: "admin".to_string(),
                    name: "管理员".to_string(),
                }],
                permissions: vec!["view_reports".to_string()],
            }))),
            host_service: Arc::new(Mutex::new(
                crate::services::host_service::HostServiceRuntime::default(),
            )),
        };
        (dir, state)
    }

    #[test]
    fn write_report_workbook_creates_openable_xlsx_package() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("report.xlsx");
        let bundle = ReportBundle {
            month: "2026-06".to_string(),
            monthly_inventory: vec![MonthlyInventoryRow {
                item_id: "item-1".to_string(),
                item_code: "IT-001".to_string(),
                item_name: "测试物品".to_string(),
                spec: Some("标准".to_string()),
                unit_name: Some("件".to_string()),
                inbound_quantity: 10.0,
                inbound_amount: 120.0,
                outbound_quantity: 4.0,
                outbound_amount: 48.0,
                ending_quantity: 6.0,
                ending_amount: 72.0,
            }],
            department_summary: vec![DepartmentIssueSummaryRow {
                department_id: "dept-admin-office".to_string(),
                department_name: "行政办".to_string(),
                quantity: 4.0,
                amount: 48.0,
            }],
            department_details: vec![DepartmentIssueDetailRow {
                movement_date: "2026-06-30".to_string(),
                department_name: "行政办".to_string(),
                item_code: "IT-001".to_string(),
                item_name: "测试物品".to_string(),
                spec: Some("标准".to_string()),
                unit_name: Some("件".to_string()),
                quantity: 4.0,
                unit_price: 12.0,
                amount: 48.0,
                document_no: Some("OUT-20260630-0001".to_string()),
                purpose: Some("测试".to_string()),
                remark: None,
            }],
            category_consumption: vec![CategoryConsumptionRow {
                category_id: Some("cat-1".to_string()),
                category_name: "客耗".to_string(),
                quantity: 4.0,
                amount: 48.0,
            }],
            item_consumption_ranking: vec![ItemConsumptionRow {
                item_id: "item-1".to_string(),
                item_code: "IT-001".to_string(),
                item_name: "测试物品".to_string(),
                spec: Some("标准".to_string()),
                unit_name: Some("件".to_string()),
                quantity: 4.0,
                amount: 48.0,
            }],
            inbound_details: vec![InboundDetailRow {
                movement_date: "2026-06-01".to_string(),
                supplier_name: "测试供应商".to_string(),
                item_code: "IT-001".to_string(),
                item_name: "测试物品".to_string(),
                spec: Some("标准".to_string()),
                unit_name: Some("件".to_string()),
                quantity: 10.0,
                unit_price: 12.0,
                amount: 120.0,
                document_no: Some("IN-20260601-0001".to_string()),
                remark: None,
            }],
            outbound_details: vec![DepartmentIssueDetailRow {
                movement_date: "2026-06-30".to_string(),
                department_name: "行政办".to_string(),
                item_code: "IT-001".to_string(),
                item_name: "测试物品".to_string(),
                spec: Some("标准".to_string()),
                unit_name: Some("件".to_string()),
                quantity: 4.0,
                unit_price: 12.0,
                amount: 48.0,
                document_no: Some("OUT-20260630-0001".to_string()),
                purpose: Some("测试".to_string()),
                remark: None,
            }],
            stock_balances: vec![StockBalanceReportRow {
                item_id: "item-1".to_string(),
                item_code: "IT-001".to_string(),
                item_name: "测试物品".to_string(),
                spec: Some("标准".to_string()),
                unit_name: Some("件".to_string()),
                quantity: 6.0,
                amount: 72.0,
                average_price: 12.0,
                last_inbound_price: 12.0,
                warning_quantity: 8.0,
                stock_status: "low".to_string(),
            }],
            stock_warnings: vec![StockWarningRow {
                item_id: "item-1".to_string(),
                item_code: "IT-001".to_string(),
                item_name: "测试物品".to_string(),
                spec: Some("标准".to_string()),
                unit_name: Some("件".to_string()),
                quantity: 6.0,
                warning_quantity: 8.0,
                shortage_quantity: 2.0,
                amount: 72.0,
            }],
            stocktake_differences: vec![StocktakeDifferenceReportRow {
                business_date: "2026-06-30".to_string(),
                document_no: "ST-20260630-0001".to_string(),
                scope_type: "all".to_string(),
                status: "confirmed".to_string(),
                item_code: "IT-001".to_string(),
                item_name: "测试物品".to_string(),
                spec: Some("标准".to_string()),
                unit_name: Some("件".to_string()),
                book_quantity: 6.0,
                counted_quantity: 5.0,
                difference_quantity: -1.0,
                average_price: 12.0,
                difference_amount: -12.0,
                remark: Some("测试差异".to_string()),
            }],
        };

        write_report_workbook(&bundle, &path).expect("write workbook");

        let file = std::fs::File::open(&path).expect("open workbook");
        let mut archive = ZipArchive::new(file).expect("xlsx zip");
        let mut workbook_xml = String::new();
        {
            use std::io::Read;
            let mut workbook_entry = archive
                .by_name("xl/workbook.xml")
                .expect("workbook xml exists");
            workbook_entry
                .read_to_string(&mut workbook_xml)
                .expect("read workbook xml");
        }
        assert!(workbook_xml.contains("库存余额"));
        assert!(workbook_xml.contains("盘点差异"));
        archive
            .by_name("xl/worksheets/sheet1.xml")
            .expect("first sheet exists");
    }

    #[test]
    fn export_monthly_report_uses_default_export_dir_setting() {
        let (dir, state) = report_test_state();
        let custom_export_dir = dir.path().join("custom-exports");
        state
            .db
            .with_conn(|conn| {
                repository::set_setting(
                    conn,
                    "default_export_dir",
                    &custom_export_dir.display().to_string(),
                )
            })
            .unwrap();

        let result = export_monthly_report(
            &state,
            ReportQuery {
                month: "2026-06".to_string(),
                start_date: None,
                end_date: None,
                department_id: None,
                category_id: None,
                item_id: None,
                supplier_id: None,
            },
        )
        .unwrap();

        assert!(std::path::Path::new(&result.path).starts_with(&custom_export_dir));
        assert!(std::path::Path::new(&result.path).exists());
    }
}
