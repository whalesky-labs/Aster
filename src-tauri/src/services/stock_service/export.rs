use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Local;
use rust_xlsxwriter::{Format, Workbook, Worksheet, XlsxError};
use uuid::Uuid;

use crate::app::state::AppState;
use crate::db::stock_repository;
use crate::domain::runtime::RuntimeMode;
use crate::domain::stock::{ExportStockBalancesResult, StockBalanceExportRow};
use crate::error::{AppError, AppResult};

pub fn export_stock_balances(state: &AppState) -> AppResult<ExportStockBalancesResult> {
    crate::services::user_service::require_admin(state)?;
    let export_dir = crate::services::status_service::effective_export_dir(state)?;
    prepare_export_dir(&export_dir)?;
    let path = unique_export_path(&export_dir);
    let mode = crate::services::status_service::get_runtime_config(state)?.mode;
    let (bytes, row_count) = if mode == RuntimeMode::Client {
        let response = crate::services::host_service::remote_export_stock_balances(state)?;
        (response.body, response.row_count)
    } else {
        let rows = state
            .db
            .with_conn(stock_repository::list_stock_balance_export_rows)?;
        let row_count = rows.len();
        (stock_balance_export_workbook_bytes(&rows)?, row_count)
    };
    write_atomic(&path, &bytes)?;
    if mode != RuntimeMode::Client {
        if let Err(error) = write_export_audit(state, row_count) {
            let _ = fs::remove_file(&path);
            return Err(error);
        }
    }
    Ok(ExportStockBalancesResult {
        path: path.display().to_string(),
        row_count,
    })
}

pub(crate) fn stock_balance_export_workbook_bytes(
    rows: &[StockBalanceExportRow],
) -> AppResult<Vec<u8>> {
    let mut workbook = Workbook::new();
    let header = Format::new().set_bold().set_background_color("#DDEBF7");
    let number = Format::new().set_num_format("#,##0.00");
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("库存台账").map_err(map_xlsx)?;
    write_headers(worksheet, &header)?;
    for (index, row) in rows.iter().enumerate() {
        write_row(worksheet, (index + 1) as u32, row, &number)?;
    }
    set_column_widths(worksheet)?;
    workbook.save_to_buffer().map_err(map_xlsx)
}

fn write_headers(worksheet: &mut Worksheet, header: &Format) -> AppResult<()> {
    let headers = [
        "物品编码",
        "物品名称",
        "分类",
        "规格",
        "单位",
        "供应商",
        "库存数量",
        "平均成本",
        "库存金额",
        "最后入库价",
        "预警数量",
        "库存状态",
        "物品状态",
    ];
    for (column, title) in headers.iter().enumerate() {
        worksheet
            .write_string_with_format(0, column as u16, *title, header)
            .map_err(map_xlsx)?;
    }
    Ok(())
}

fn write_row(
    worksheet: &mut Worksheet,
    row_index: u32,
    row: &StockBalanceExportRow,
    number: &Format,
) -> AppResult<()> {
    worksheet
        .write_string(row_index, 0, &row.item_code)
        .map_err(map_xlsx)?;
    worksheet
        .write_string(row_index, 1, &row.item_name)
        .map_err(map_xlsx)?;
    write_optional(worksheet, row_index, 2, row.category_name.as_deref())?;
    write_optional(worksheet, row_index, 3, row.spec.as_deref())?;
    write_optional(worksheet, row_index, 4, row.unit_name.as_deref())?;
    write_optional(worksheet, row_index, 5, row.supplier_name.as_deref())?;
    for (column, value) in [
        (6, row.quantity),
        (7, row.average_price),
        (8, row.amount),
        (9, row.last_inbound_price),
        (10, row.warning_quantity),
    ] {
        worksheet
            .write_number_with_format(row_index, column, value, number)
            .map_err(map_xlsx)?;
    }
    worksheet
        .write_string(row_index, 11, stock_status_label(&row.stock_status))
        .map_err(map_xlsx)?;
    worksheet
        .write_string(
            row_index,
            12,
            if row.item_enabled { "启用" } else { "停用" },
        )
        .map_err(map_xlsx)?;
    Ok(())
}

fn write_optional(
    worksheet: &mut Worksheet,
    row: u32,
    column: u16,
    value: Option<&str>,
) -> AppResult<()> {
    worksheet
        .write_string(row, column, value.unwrap_or(""))
        .map(|_| ())
        .map_err(map_xlsx)
}

fn set_column_widths(worksheet: &mut Worksheet) -> AppResult<()> {
    for (column, width) in [18, 24, 16, 18, 10, 20, 14, 14, 14, 14, 14, 12, 12]
        .iter()
        .enumerate()
    {
        worksheet
            .set_column_width(column as u16, *width)
            .map_err(map_xlsx)?;
    }
    Ok(())
}

fn stock_status_label(status: &str) -> &'static str {
    match status {
        "negative" => "负库存",
        "low" => "低库存",
        _ => "正常",
    }
}

fn prepare_export_dir(directory: &Path) -> AppResult<()> {
    fs::create_dir_all(directory)?;
    let probe = directory.join(format!(".aster-write-probe-{}", Uuid::new_v4()));
    let result = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .and_then(|mut file| file.write_all(b"ok"));
    let cleanup = fs::remove_file(&probe);
    result?;
    cleanup?;
    Ok(())
}

fn unique_export_path(directory: &Path) -> PathBuf {
    let base = format!("Aster-库存台账-{}", Local::now().format("%Y%m%d%H%M%S"));
    for suffix in 1..=999_u16 {
        let file_name = if suffix == 1 {
            format!("{base}.xlsx")
        } else {
            format!("{base}-{suffix}.xlsx")
        };
        let path = directory.join(file_name);
        if !path.exists() {
            return path;
        }
    }
    directory.join(format!("{base}-{}.xlsx", Uuid::new_v4()))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let temporary = path.with_file_name(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("inventory"),
        Uuid::new_v4()
    ));
    if let Err(error) = fs::write(&temporary, bytes) {
        let _ = fs::remove_file(&temporary);
        return Err(AppError::Io(error));
    }
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(AppError::Io(error));
    }
    Ok(())
}

fn write_export_audit(state: &AppState, row_count: usize) -> AppResult<()> {
    state.db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
             VALUES (?1, 'export_stock_balances', 'stock_balances', 'all', ?2, ?3)",
            rusqlite::params![
                Uuid::new_v4().to_string(),
                format!("导出全部库存台账：{row_count} 项"),
                crate::services::user_service::current_operator(state)
            ],
        )?;
        Ok(())
    })
}

fn map_xlsx(error: XlsxError) -> AppError {
    AppError::Validation(format!("库存台账导出失败：{error}"))
}
