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

fn parse_import_template_workbook(path: &str) -> AppResult<ParsedWorkbook> {
    let path_ref = Path::new(path);
    if !path_ref.exists() {
        return Err(AppError::Validation(format!("Excel 文件不存在：{path}")));
    }

    let mut workbook = open_workbook_auto(path_ref)
        .map_err(|error| AppError::Validation(format!("无法读取 Excel：{error}")))?;
    let sheet_names = workbook.sheet_names().to_owned();
    let mut missing_sheets = [ITEM_SHEET, INBOUND_SHEET, OUTBOUND_SHEET]
        .into_iter()
        .filter(|sheet| !sheet_names.iter().any(|name| name.trim() == *sheet))
        .collect::<Vec<_>>();
    if !missing_sheets.is_empty() {
        missing_sheets.sort();
        return Err(AppError::Validation(format!(
            "导入模板不匹配：缺少工作表 {}。请使用系统导出的新版模板。",
            missing_sheets.join("、")
        )));
    }

    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let item_sheet = workbook
        .worksheet_range(ITEM_SHEET)
        .map_err(|error| AppError::Validation(format!("读取工作表失败：{ITEM_SHEET}：{error}")))?;
    let inbound_sheet = workbook.worksheet_range(INBOUND_SHEET).map_err(|error| {
        AppError::Validation(format!("读取工作表失败：{INBOUND_SHEET}：{error}"))
    })?;
    let outbound_sheet = workbook.worksheet_range(OUTBOUND_SHEET).map_err(|error| {
        AppError::Validation(format!("读取工作表失败：{OUTBOUND_SHEET}：{error}"))
    })?;

    let mut items = parse_item_sheet(&item_sheet, &mut warnings, &mut errors)?;
    let inbound_rows = parse_inbound_sheet(&inbound_sheet, &mut items, &mut warnings, &mut errors)?;
    let outbound_rows =
        parse_outbound_sheet(&outbound_sheet, &mut items, &mut warnings, &mut errors)?;

    Ok(ParsedWorkbook {
        items: items.into_values().collect(),
        inbound_rows,
        outbound_rows,
        warnings,
        errors,
        sheet_count: 3,
    })
}

fn parse_item_sheet(
    range: &calamine::Range<Data>,
    warnings: &mut Vec<ImportMessage>,
    errors: &mut Vec<ImportMessage>,
) -> AppResult<BTreeMap<String, ParsedItem>> {
    let header = SheetHeader::from_range(range, ITEM_SHEET, &["物品名称"])?;
    let mut items = BTreeMap::new();
    for (row_index, row) in range.rows().enumerate().skip(header.row_index + 1) {
        let excel_row = row_index + 1;
        if row
            .iter()
            .all(crate::services::import_value_parser::is_empty)
        {
            continue;
        }
        detect_formula_errors(ITEM_SHEET, excel_row, row, errors);
        let name = normalized_name(&header.text(row, "物品名称"));
        if name.is_empty() {
            errors.push(message(
                "error",
                ITEM_SHEET,
                excel_row,
                Some("物品名称"),
                "物品名称不能为空",
            ));
            continue;
        }
        let code = header
            .optional_text(row, "物品编码")
            .filter(|value| !value.is_empty());
        let key = item_lookup_key(code.as_deref(), &name);
        if items.contains_key(&key) {
            errors.push(message(
                "error",
                ITEM_SHEET,
                excel_row,
                Some("物品名称"),
                "物品档案中物品编码/名称重复",
            ));
            continue;
        }
        let unit_name = header
            .optional_text(row, "单位")
            .filter(|value| !value.is_empty());
        if unit_name.is_none() {
            errors.push(message(
                "error",
                ITEM_SHEET,
                excel_row,
                Some("单位"),
                "单位不能为空",
            ));
        }
        let default_price = header.optional_number(row, "参考进价").unwrap_or(0.0);
        let sale_price = header.optional_number(row, "参考售价").unwrap_or(0.0);
        let warning_quantity = header.optional_number(row, "预警库存").unwrap_or(0.0);
        if default_price < 0.0 || sale_price < 0.0 || warning_quantity < 0.0 {
            errors.push(message(
                "error",
                ITEM_SHEET,
                excel_row,
                None,
                "参考进价、参考售价和预警库存不能小于 0",
            ));
        }
        items.insert(
            key.clone(),
            ParsedItem {
                key,
                sheet_name: ITEM_SHEET.to_string(),
                row_number: excel_row,
                code,
                name,
                category_name: header
                    .optional_text(row, "分类")
                    .filter(|value| !value.is_empty()),
                spec: header
                    .optional_text(row, "规格")
                    .filter(|value| !value.is_empty()),
                unit_name,
                default_price,
                sale_price,
                warning_quantity,
                remark: header
                    .optional_text(row, "备注")
                    .filter(|value| !value.is_empty()),
            },
        );
    }
    if items.is_empty() {
        warnings.push(message(
            "warning",
            ITEM_SHEET,
            1,
            None,
            "物品档案为空，将仅根据入库/出库明细自动补充物品档案",
        ));
    }
    Ok(items)
}

fn parse_inbound_sheet(
    range: &calamine::Range<Data>,
    items: &mut BTreeMap<String, ParsedItem>,
    warnings: &mut Vec<ImportMessage>,
    errors: &mut Vec<ImportMessage>,
) -> AppResult<Vec<ParsedInboundLine>> {
    let header = SheetHeader::from_range(
        range,
        INBOUND_SHEET,
        &["业务时间", "物品名称", "数量", "进货单价"],
    )?;
    let mut rows = Vec::new();
    for (row_index, row) in range.rows().enumerate().skip(header.row_index + 1) {
        let excel_row = row_index + 1;
        if row
            .iter()
            .all(crate::services::import_value_parser::is_empty)
        {
            continue;
        }
        detect_formula_errors(INBOUND_SHEET, excel_row, row, errors);
        let business_date = match header.datetime(row, "业务时间") {
            Ok(Some(value)) => value,
            Ok(None) => {
                errors.push(message(
                    "error",
                    INBOUND_SHEET,
                    excel_row,
                    Some("业务时间"),
                    "业务时间不能为空，格式示例：2026-06-01 09:00:00",
                ));
                continue;
            }
            Err(error) => {
                errors.push(message(
                    "error",
                    INBOUND_SHEET,
                    excel_row,
                    Some("业务时间"),
                    &error,
                ));
                continue;
            }
        };
        let item_name = normalized_name(&header.text(row, "物品名称"));
        if item_name.is_empty() {
            errors.push(message(
                "error",
                INBOUND_SHEET,
                excel_row,
                Some("物品名称"),
                "物品名称不能为空",
            ));
            continue;
        }
        let item_code = header
            .optional_text(row, "物品编码")
            .filter(|value| !value.is_empty());
        let key = item_lookup_key(item_code.as_deref(), &item_name);
        ensure_parsed_item_from_line(
            items,
            ParsedItemSource {
                key: &key,
                code: item_code,
                name: &item_name,
                header: &header,
                row,
                sheet_name: INBOUND_SHEET,
                excel_row,
            },
        );
        let quantity = header.optional_number(row, "数量").unwrap_or(0.0);
        let unit_price = header.optional_number(row, "进货单价").unwrap_or(0.0);
        let amount = header
            .optional_number(row, "进货金额")
            .unwrap_or(quantity * unit_price);
        if quantity <= 0.0 {
            errors.push(message(
                "error",
                INBOUND_SHEET,
                excel_row,
                Some("数量"),
                "入库数量必须大于 0",
            ));
        }
        if unit_price < 0.0 || amount < 0.0 {
            errors.push(message(
                "error",
                INBOUND_SHEET,
                excel_row,
                None,
                "进货单价和进货金额不能小于 0",
            ));
        }
        warn_amount_mismatch(
            INBOUND_SHEET,
            excel_row,
            "进货金额",
            quantity,
            unit_price,
            Some(amount),
            warnings,
        );
        rows.push(ParsedInboundLine {
            sheet_name: INBOUND_SHEET.to_string(),
            row_number: excel_row,
            business_date,
            supplier_name: header
                .optional_text(row, "供应商")
                .filter(|value| !value.is_empty()),
            item_key: key,
            quantity,
            unit_price,
            amount: round_money(amount),
            handler: header
                .optional_text(row, "经办人")
                .filter(|value| !value.is_empty()),
            remark: header
                .optional_text(row, "备注")
                .filter(|value| !value.is_empty()),
        });
    }
    Ok(rows)
}

fn parse_outbound_sheet(
    range: &calamine::Range<Data>,
    items: &mut BTreeMap<String, ParsedItem>,
    _warnings: &mut Vec<ImportMessage>,
    errors: &mut Vec<ImportMessage>,
) -> AppResult<Vec<ParsedOutboundLine>> {
    let header = SheetHeader::from_range(
        range,
        OUTBOUND_SHEET,
        &["业务时间", "出库类型", "部门", "物品名称", "数量"],
    )?;
    let mut rows = Vec::new();
    for (row_index, row) in range.rows().enumerate().skip(header.row_index + 1) {
        let excel_row = row_index + 1;
        if row
            .iter()
            .all(crate::services::import_value_parser::is_empty)
        {
            continue;
        }
        detect_formula_errors(OUTBOUND_SHEET, excel_row, row, errors);
        let business_date = match header.datetime(row, "业务时间") {
            Ok(Some(value)) => value,
            Ok(None) => {
                errors.push(message(
                    "error",
                    OUTBOUND_SHEET,
                    excel_row,
                    Some("业务时间"),
                    "业务时间不能为空，格式示例：2026-06-01 09:00:00",
                ));
                continue;
            }
            Err(error) => {
                errors.push(message(
                    "error",
                    OUTBOUND_SHEET,
                    excel_row,
                    Some("业务时间"),
                    &error,
                ));
                continue;
            }
        };
        let outbound_kind_text = header.text(row, "出库类型");
        let Some(outbound_kind) = parse_outbound_kind(&outbound_kind_text) else {
            errors.push(message(
                "error",
                OUTBOUND_SHEET,
                excel_row,
                Some("出库类型"),
                "出库类型只能填写：内部领用 或 酒店客人销售",
            ));
            continue;
        };
        let department_name = normalized_name(&header.text(row, "部门"));
        if department_name.is_empty() {
            errors.push(message(
                "error",
                OUTBOUND_SHEET,
                excel_row,
                Some("部门"),
                "部门不能为空",
            ));
        }
        let item_name = normalized_name(&header.text(row, "物品名称"));
        if item_name.is_empty() {
            errors.push(message(
                "error",
                OUTBOUND_SHEET,
                excel_row,
                Some("物品名称"),
                "物品名称不能为空",
            ));
            continue;
        }
        let item_code = header
            .optional_text(row, "物品编码")
            .filter(|value| !value.is_empty());
        let key = item_lookup_key(item_code.as_deref(), &item_name);
        ensure_parsed_item_from_line(
            items,
            ParsedItemSource {
                key: &key,
                code: item_code,
                name: &item_name,
                header: &header,
                row,
                sheet_name: OUTBOUND_SHEET,
                excel_row,
            },
        );
        let quantity = header.optional_number(row, "数量").unwrap_or(0.0);
        if quantity <= 0.0 {
            errors.push(message(
                "error",
                OUTBOUND_SHEET,
                excel_row,
                Some("数量"),
                "出库数量必须大于 0",
            ));
        }
        let sale_unit_price = header.optional_number(row, "销售单价").unwrap_or(0.0);
        if sale_unit_price < 0.0 {
            errors.push(message(
                "error",
                OUTBOUND_SHEET,
                excel_row,
                Some("销售单价"),
                "销售单价不能小于 0",
            ));
        }
        rows.push(ParsedOutboundLine {
            sheet_name: OUTBOUND_SHEET.to_string(),
            row_number: excel_row,
            business_date,
            outbound_kind: outbound_kind.to_string(),
            department_name,
            item_key: key,
            quantity,
            sale_unit_price,
            handler: header
                .optional_text(row, "经办人")
                .filter(|value| !value.is_empty()),
            purpose: header
                .optional_text(row, "用途")
                .filter(|value| !value.is_empty()),
            remark: header
                .optional_text(row, "备注")
                .filter(|value| !value.is_empty()),
        });
    }
    Ok(rows)
}

struct ParsedItemSource<'a> {
    key: &'a str,
    code: Option<String>,
    name: &'a str,
    header: &'a SheetHeader,
    row: &'a [Data],
    sheet_name: &'a str,
    excel_row: usize,
}

fn ensure_parsed_item_from_line(
    items: &mut BTreeMap<String, ParsedItem>,
    source: ParsedItemSource<'_>,
) {
    items.entry(source.key.to_string()).or_insert_with(|| {
        let default_price = source
            .header
            .optional_number(source.row, "参考进价")
            .unwrap_or_else(|| {
                source
                    .header
                    .optional_number(source.row, "进货单价")
                    .unwrap_or(0.0)
            });
        ParsedItem {
            key: source.key.to_string(),
            sheet_name: source.sheet_name.to_string(),
            row_number: source.excel_row,
            code: source.code,
            name: source.name.to_string(),
            category_name: source
                .header
                .optional_text(source.row, "分类")
                .filter(|value| !value.is_empty()),
            spec: source
                .header
                .optional_text(source.row, "规格")
                .filter(|value| !value.is_empty()),
            unit_name: source
                .header
                .optional_text(source.row, "单位")
                .filter(|value| !value.is_empty()),
            default_price,
            sale_price: source
                .header
                .optional_number(source.row, "参考售价")
                .unwrap_or(0.0),
            warning_quantity: 0.0,
            remark: None,
        }
    });
}

fn build_preview(
    conn: &Connection,
    source_file: &str,
    parsed: ParsedWorkbook,
) -> AppResult<ImportPreview> {
    build_preview_from_parts(conn, source_file, &parsed)
}

fn build_preview_from_parts(
    conn: &Connection,
    source_file: &str,
    parsed: &ParsedWorkbook,
) -> AppResult<ImportPreview> {
    let existing_keys = existing_item_keys(conn)?;
    let mut item_map: BTreeMap<String, ImportItemAccumulator> = BTreeMap::new();
    let mut month_map: BTreeMap<String, ImportMonthAccumulator> = BTreeMap::new();

    for item in &parsed.items {
        item_map
            .entry(item.key.clone())
            .or_insert_with(|| ImportItemAccumulator {
                name: item.name.clone(),
                category_name: item.category_name.clone(),
                spec: item.spec.clone(),
                unit_name: item.unit_name.clone(),
                default_price: item.default_price,
                opening_quantity: 0.0,
                inbound_quantity: 0.0,
                outbound_quantity: 0.0,
                existing: existing_keys.contains(&item.key),
            });
    }
    for line in &parsed.inbound_rows {
        if let Some(item) = item_map.get_mut(&line.item_key) {
            item.inbound_quantity += line.quantity.max(0.0);
        }
        let month = month_map
            .entry(month_from_datetime(&line.business_date))
            .or_default();
        month.row_count += 1;
        month.inbound_quantity += line.quantity.max(0.0);
    }
    for line in &parsed.outbound_rows {
        if let Some(item) = item_map.get_mut(&line.item_key) {
            item.outbound_quantity += line.quantity.max(0.0);
        }
        let month = month_map
            .entry(month_from_datetime(&line.business_date))
            .or_default();
        month.row_count += 1;
        month.outbound_quantity += line.quantity.max(0.0);
        month.outbound_amount += round_money(line.quantity * line.sale_unit_price);
    }

    let inbound_quantity = parsed
        .inbound_rows
        .iter()
        .map(|row| row.quantity.max(0.0))
        .sum();
    let inbound_amount = parsed
        .inbound_rows
        .iter()
        .map(|row| row.amount.max(0.0))
        .sum();
    let outbound_quantity = parsed
        .outbound_rows
        .iter()
        .map(|row| row.quantity.max(0.0))
        .sum();
    let outbound_amount = parsed
        .outbound_rows
        .iter()
        .map(|row| round_money(row.quantity * row.sale_unit_price))
        .sum();
    let items = item_map
        .into_values()
        .map(|item| ImportItemPreview {
            name: item.name,
            category_name: item.category_name,
            spec: item.spec,
            unit_name: item.unit_name,
            default_price: item.default_price,
            opening_quantity: item.opening_quantity,
            inbound_quantity: item.inbound_quantity,
            outbound_quantity: item.outbound_quantity,
            existing: item.existing,
        })
        .collect::<Vec<_>>();
    let existing_item_count = items.iter().filter(|item| item.existing).count();

    Ok(ImportPreview {
        source_file: source_file.to_string(),
        sheet_count: parsed.sheet_count,
        row_count: parsed.items.len() + parsed.inbound_rows.len() + parsed.outbound_rows.len(),
        item_count: items.len(),
        new_item_count: items.len().saturating_sub(existing_item_count),
        existing_item_count,
        opening_quantity: 0.0,
        opening_amount: 0.0,
        inbound_quantity,
        inbound_amount: round_money(inbound_amount),
        outbound_quantity,
        outbound_amount: round_money(outbound_amount),
        document_count: planned_document_count(parsed),
        warnings: parsed.warnings.clone(),
        errors: parsed.errors.clone(),
        items,
        months: month_map
            .into_iter()
            .map(|(month, data)| ImportMonthPreview {
                month,
                row_count: data.row_count,
                opening_quantity: 0.0,
                inbound_quantity: data.inbound_quantity,
                outbound_quantity: data.outbound_quantity,
                outbound_amount: round_money(data.outbound_amount),
            })
            .collect(),
    })
}

fn import_parsed_workbook(
    conn: &mut Connection,
    source_file: &str,
    job_id: &str,
    parsed: &ParsedWorkbook,
    preview: &ImportPreview,
    mode: ImportMode,
    allow_negative_stock: bool,
) -> AppResult<ImportResult> {
    conn.execute(
        "INSERT INTO import_jobs (id, source_file, status, total_rows, warning_rows, error_rows, report_json)
         VALUES (?1, ?2, 'previewed', ?3, ?4, ?5, ?6)",
        params![
            job_id,
            source_file,
            preview.row_count as i64,
            parsed.warnings.len() as i64,
            parsed.errors.len() as i64,
            serde_json::to_string(preview).unwrap_or_else(|_| "{}".to_string())
        ],
    )?;

    let (item_ids, supplier_ids, department_ids, imported_items, matched_items) = {
        let tx = conn.transaction()?;
        let category_ids = ensure_categories(&tx, parsed)?;
        let unit_ids = ensure_units(&tx, parsed)?;
        let (ids, imported, matched) = ensure_items(&tx, parsed, &category_ids, &unit_ids)?;
        let supplier_ids = if mode.import_movements() {
            ensure_suppliers(&tx, parsed)?
        } else {
            HashMap::new()
        };
        let department_ids = if mode.import_movements() {
            ensure_departments(&tx, parsed)?
        } else {
            HashMap::new()
        };
        tx.commit()?;
        (ids, supplier_ids, department_ids, imported, matched)
    };

    let mut document_count = 0;
    let mut movement_count = 0;
    if mode.import_movements() {
        for request in build_submit_requests(parsed, &item_ids, &supplier_ids, &department_ids)? {
            crate::services::stock_service::validate_document(&request)?;
            let detail =
                stock_repository::submit_stock_document(conn, request, allow_negative_stock)?;
            document_count += 1;
            movement_count += count_document_movements(conn, &detail.document.id)? as usize;
        }
    }

    conn.execute(
        "UPDATE import_jobs
         SET status = 'imported', success_rows = ?1, completed_at = CURRENT_TIMESTAMP
         WHERE id = ?2",
        params![preview.row_count as i64, job_id],
    )?;
    conn.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, 'run_excel_import', 'import_job', ?2, ?3, 'system')",
        params![
            new_id(),
            job_id,
            format!(
                "导入 Excel：{}，{} 行，{} 条流水",
                source_file, preview.row_count, movement_count
            )
        ],
    )?;

    Ok(ImportResult {
        job_id: job_id.to_string(),
        source_file: source_file.to_string(),
        imported_items,
        matched_items,
        document_count,
        movement_count,
        warning_count: parsed.warnings.len(),
        error_count: parsed.errors.len(),
        report_path: None,
        source_copy_path: None,
    })
}

fn ensure_items(
    conn: &Connection,
    parsed: &ParsedWorkbook,
    category_ids: &HashMap<String, String>,
    unit_ids: &HashMap<String, String>,
) -> AppResult<(HashMap<String, String>, usize, usize)> {
    let mut ids = HashMap::new();
    let mut imported = 0;
    let mut matched = 0;
    for item in &parsed.items {
        if item.name.trim().is_empty() {
            return Err(AppError::Validation(format!(
                "{} 第 {} 行物品名称不能为空",
                item.sheet_name, item.row_number
            )));
        }
        if let Some(existing_id) = find_item_id(conn, item.code.as_deref(), &item.name)? {
            matched += 1;
            ids.insert(item.key.clone(), existing_id);
            continue;
        }
        let id = new_id();
        let code = item
            .code
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| {
                next_item_code(conn).unwrap_or_else(|_| format!("IMP-{}", imported + 1))
            });
        conn.execute(
            "INSERT INTO master_items (
               id, code, name, category_id, spec, unit_id, default_price,
               sale_price, warning_quantity, enabled, remark
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10)",
            params![
                id,
                code,
                item.name,
                item.category_name
                    .as_ref()
                    .and_then(|name| category_ids.get(name))
                    .cloned(),
                item.spec,
                item.unit_name
                    .as_ref()
                    .and_then(|name| unit_ids.get(name))
                    .cloned(),
                item.default_price,
                item.sale_price,
                item.warning_quantity,
                item.remark
            ],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO stock_balances (id, item_id) VALUES (?1, ?2)",
            params![new_id(), id],
        )?;
        imported += 1;
        ids.insert(item.key.clone(), id);
    }
    Ok((ids, imported, matched))
}

fn build_submit_requests(
    parsed: &ParsedWorkbook,
    item_ids: &HashMap<String, String>,
    supplier_ids: &HashMap<String, String>,
    department_ids: &HashMap<String, String>,
) -> AppResult<Vec<SubmitStockDocumentRequest>> {
    let mut requests = Vec::new();
    for group in group_inbound_rows(&parsed.inbound_rows) {
        let supplier_id = group
            .supplier_name
            .as_deref()
            .and_then(|name| supplier_ids.get(name))
            .cloned();
        let mut lines = Vec::new();
        for line in group.rows {
            let item_id = item_ids.get(&line.item_key).cloned().ok_or_else(|| {
                AppError::Validation(format!(
                    "{} 第 {} 行物品未能匹配档案",
                    line.sheet_name, line.row_number
                ))
            })?;
            lines.push(SubmitStockDocumentLine {
                item_id,
                quantity: line.quantity,
                unit_price: line.unit_price,
                amount: Some(line.amount),
                remark: line.remark.clone(),
            });
        }
        requests.push(SubmitStockDocumentRequest {
            document_type: "inbound".to_string(),
            outbound_kind: None,
            business_date: group.business_date,
            department_id: None,
            supplier_id,
            handler: group.handler,
            purpose: None,
            remark: group.remark.or_else(|| Some("Excel 导入入库".to_string())),
            approval_request_id: None,
            lines,
        });
    }
    for group in group_outbound_rows(&parsed.outbound_rows) {
        let mut lines = Vec::new();
        for line in group.rows {
            let item_id = item_ids.get(&line.item_key).cloned().ok_or_else(|| {
                AppError::Validation(format!(
                    "{} 第 {} 行物品未能匹配档案",
                    line.sheet_name, line.row_number
                ))
            })?;
            lines.push(SubmitStockDocumentLine {
                item_id,
                quantity: line.quantity,
                unit_price: if line.outbound_kind == "guest_sale" {
                    line.sale_unit_price
                } else {
                    0.0
                },
                amount: if line.outbound_kind == "guest_sale" {
                    Some(round_money(line.quantity * line.sale_unit_price))
                } else {
                    None
                },
                remark: line.remark.clone(),
            });
        }
        requests.push(SubmitStockDocumentRequest {
            document_type: "outbound".to_string(),
            outbound_kind: Some(group.outbound_kind),
            business_date: group.business_date,
            department_id: department_ids.get(&group.department_name).cloned(),
            supplier_id: None,
            handler: group.handler,
            purpose: group.purpose,
            remark: group.remark.or_else(|| Some("Excel 导入出库".to_string())),
            approval_request_id: None,
            lines,
        });
    }
    requests.sort_by(|a, b| {
        a.business_date
            .cmp(&b.business_date)
            .then_with(|| a.document_type.cmp(&b.document_type))
    });
    Ok(requests)
}

fn group_inbound_rows(rows: &[ParsedInboundLine]) -> Vec<InboundGroup<'_>> {
    let mut groups: BTreeMap<InboundGroupKey, Vec<&ParsedInboundLine>> = BTreeMap::new();
    for row in rows {
        groups
            .entry(InboundGroupKey {
                business_date: row.business_date.clone(),
                supplier_name: row.supplier_name.clone(),
                handler: row.handler.clone(),
                remark: row.remark.clone(),
            })
            .or_default()
            .push(row);
    }
    groups
        .into_iter()
        .map(|(key, rows)| InboundGroup {
            business_date: key.business_date,
            supplier_name: key.supplier_name,
            handler: key.handler,
            remark: key.remark,
            rows,
        })
        .collect()
}

fn group_outbound_rows(rows: &[ParsedOutboundLine]) -> Vec<OutboundGroup<'_>> {
    let mut groups: BTreeMap<OutboundGroupKey, Vec<&ParsedOutboundLine>> = BTreeMap::new();
    for row in rows {
        groups
            .entry(OutboundGroupKey {
                business_date: row.business_date.clone(),
                outbound_kind: row.outbound_kind.clone(),
                department_name: row.department_name.clone(),
                handler: row.handler.clone(),
                purpose: row.purpose.clone(),
                remark: row.remark.clone(),
            })
            .or_default()
            .push(row);
    }
    groups
        .into_iter()
        .map(|(key, rows)| OutboundGroup {
            business_date: key.business_date,
            outbound_kind: key.outbound_kind,
            department_name: key.department_name,
            handler: key.handler,
            purpose: key.purpose,
            remark: key.remark,
            rows,
        })
        .collect()
}

fn planned_document_count(parsed: &ParsedWorkbook) -> usize {
    group_inbound_rows(&parsed.inbound_rows).len()
        + group_outbound_rows(&parsed.outbound_rows).len()
}

fn ensure_categories(
    conn: &Connection,
    parsed: &ParsedWorkbook,
) -> AppResult<HashMap<String, String>> {
    let mut map = HashMap::new();
    for name in parsed
        .items
        .iter()
        .filter_map(|item| item.category_name.as_ref())
        .filter(|name| !name.trim().is_empty())
    {
        if map.contains_key(name) {
            continue;
        }
        let id = conn
            .query_row(
                "SELECT id FROM categories WHERE parent_id IS NULL AND name = ?1",
                params![name],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_else(new_id);
        conn.execute(
            "INSERT OR IGNORE INTO categories (id, name, enabled, sort_order)
             VALUES (?1, ?2, 1, 999)",
            params![id, name],
        )?;
        map.insert(name.clone(), id);
    }
    Ok(map)
}

fn ensure_units(conn: &Connection, parsed: &ParsedWorkbook) -> AppResult<HashMap<String, String>> {
    let mut map = HashMap::new();
    for name in parsed
        .items
        .iter()
        .filter_map(|item| item.unit_name.as_ref())
        .filter(|name| !name.trim().is_empty())
    {
        if map.contains_key(name) {
            continue;
        }
        let id = conn
            .query_row(
                "SELECT id FROM units WHERE name = ?1",
                params![name],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_else(new_id);
        conn.execute(
            "INSERT OR IGNORE INTO units (id, name, enabled, sort_order)
             VALUES (?1, ?2, 1, 999)",
            params![id, name],
        )?;
        map.insert(name.clone(), id);
    }
    Ok(map)
}

fn ensure_suppliers(
    conn: &Connection,
    parsed: &ParsedWorkbook,
) -> AppResult<HashMap<String, String>> {
    let mut seen = HashSet::new();
    let mut map = HashMap::new();
    for name in parsed
        .inbound_rows
        .iter()
        .filter_map(|row| row.supplier_name.as_ref())
        .filter(|name| !name.trim().is_empty())
    {
        if !seen.insert(name.clone()) {
            continue;
        }
        let id = conn
            .query_row(
                "SELECT id FROM suppliers WHERE name = ?1",
                params![name],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_else(|| format!("supplier:{name}"));
        conn.execute(
            "INSERT OR IGNORE INTO suppliers (id, name, enabled, remark)
             VALUES (?1, ?2, 1, 'Excel 导入自动创建')",
            params![id, name],
        )?;
        map.insert(name.clone(), id);
    }
    Ok(map)
}

fn ensure_departments(
    conn: &Connection,
    parsed: &ParsedWorkbook,
) -> AppResult<HashMap<String, String>> {
    let mut seen = HashSet::new();
    let mut map = HashMap::new();
    for name in parsed
        .outbound_rows
        .iter()
        .map(|row| row.department_name.as_str())
        .filter(|name| !name.trim().is_empty())
    {
        if !seen.insert(name.to_string()) {
            continue;
        }
        let id = conn
            .query_row(
                "SELECT id FROM departments WHERE name = ?1",
                params![name],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_else(|| format!("department:{name}"));
        conn.execute(
            "INSERT OR IGNORE INTO departments (id, code, name, enabled, sort_order, remark)
             VALUES (?1, ?2, ?3, 1, 999, 'Excel 导入自动创建')",
            params![id, next_department_code(conn)?, name],
        )?;
        map.insert(name.to_string(), id);
    }
    Ok(map)
}

fn existing_item_keys(conn: &Connection) -> AppResult<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT code, name FROM master_items")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut keys = HashSet::new();
    for row in rows {
        let (code, name) = row?;
        keys.insert(item_lookup_key(code.as_deref(), &normalized_name(&name)));
        keys.insert(format!("name:{}", normalized_name(&name)));
    }
    Ok(keys)
}

fn find_item_id(conn: &Connection, code: Option<&str>, name: &str) -> AppResult<Option<String>> {
    if let Some(code) = code.map(str::trim).filter(|value| !value.is_empty()) {
        if let Some(id) = conn
            .query_row(
                "SELECT id FROM master_items WHERE code = ?1",
                params![code],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            return Ok(Some(id));
        }
    }
    Ok(conn
        .query_row(
            "SELECT id FROM master_items WHERE name = ?1",
            params![name],
            |row| row.get::<_, String>(0),
        )
        .optional()?)
}

fn next_item_code(conn: &Connection) -> AppResult<String> {
    let mut index: i64 = conn.query_row("SELECT COUNT(*) + 1 FROM master_items", [], |row| {
        row.get(0)
    })?;
    loop {
        let code = format!("IMP-{index:04}");
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM master_items WHERE code = ?1)",
            params![code],
            |row| row.get(0),
        )?;
        if !exists {
            return Ok(code);
        }
        index += 1;
    }
}

fn next_department_code(conn: &Connection) -> AppResult<String> {
    let mut index: i64 =
        conn.query_row("SELECT COUNT(*) + 1 FROM departments", [], |row| row.get(0))?;
    loop {
        let code = format!("DIMP{index:03}");
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM departments WHERE code = ?1)",
            params![code],
            |row| row.get(0),
        )?;
        if !exists {
            return Ok(code);
        }
        index += 1;
    }
}

fn count_document_movements(conn: &Connection, document_id: &str) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM stock_movements WHERE document_id = ?1",
        params![document_id],
        |row| row.get(0),
    )?)
}

fn write_import_template_workbook(path: &Path) -> AppResult<()> {
    let mut workbook = Workbook::new();
    let header = Format::new().set_bold().set_background_color("#DDEBF7");
    let money = Format::new().set_num_format("#,##0.00");
    let number = Format::new().set_num_format("#,##0.00");

    {
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(ITEM_SHEET).map_err(xlsx_error)?;
        write_header(
            worksheet,
            &header,
            &[
                "物品编码",
                "物品名称",
                "分类",
                "规格",
                "单位",
                "参考进价",
                "参考售价",
                "预警库存",
                "备注",
            ],
        )?;
        worksheet
            .write_string(1, 0, "YCYS-001")
            .map_err(xlsx_error)?;
        worksheet
            .write_string(1, 1, "一次性牙刷")
            .map_err(xlsx_error)?;
        worksheet.write_string(1, 2, "客耗品").map_err(xlsx_error)?;
        worksheet.write_string(1, 3, "软毛").map_err(xlsx_error)?;
        worksheet.write_string(1, 4, "支").map_err(xlsx_error)?;
        worksheet
            .write_number_with_format(1, 5, 0.80, &money)
            .map_err(xlsx_error)?;
        worksheet
            .write_number_with_format(1, 6, 3.00, &money)
            .map_err(xlsx_error)?;
        worksheet
            .write_number_with_format(1, 7, 50.0, &number)
            .map_err(xlsx_error)?;
        worksheet
            .write_string(1, 8, "示例，可删除")
            .map_err(xlsx_error)?;
        set_widths(worksheet, &[16, 24, 18, 18, 10, 12, 12, 12, 28])?;
    }

    {
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(INBOUND_SHEET).map_err(xlsx_error)?;
        write_header(
            worksheet,
            &header,
            &[
                "业务时间",
                "供应商",
                "物品编码",
                "物品名称",
                "分类",
                "规格",
                "单位",
                "数量",
                "进货单价",
                "进货金额",
                "经办人",
                "备注",
            ],
        )?;
        worksheet
            .write_string(1, 0, "2026-06-01 09:00:00")
            .map_err(xlsx_error)?;
        worksheet
            .write_string(1, 1, "默认供应商")
            .map_err(xlsx_error)?;
        worksheet
            .write_string(1, 2, "YCYS-001")
            .map_err(xlsx_error)?;
        worksheet
            .write_string(1, 3, "一次性牙刷")
            .map_err(xlsx_error)?;
        worksheet.write_string(1, 4, "客耗品").map_err(xlsx_error)?;
        worksheet.write_string(1, 5, "软毛").map_err(xlsx_error)?;
        worksheet.write_string(1, 6, "支").map_err(xlsx_error)?;
        worksheet
            .write_number_with_format(1, 7, 100.0, &number)
            .map_err(xlsx_error)?;
        worksheet
            .write_number_with_format(1, 8, 0.80, &money)
            .map_err(xlsx_error)?;
        worksheet
            .write_number_with_format(1, 9, 80.0, &money)
            .map_err(xlsx_error)?;
        worksheet
            .write_string(1, 10, "管理员")
            .map_err(xlsx_error)?;
        worksheet
            .write_string(1, 11, "示例，可删除")
            .map_err(xlsx_error)?;
        set_widths(worksheet, &[22, 20, 16, 24, 18, 18, 10, 12, 12, 12, 14, 28])?;
    }

    {
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(OUTBOUND_SHEET).map_err(xlsx_error)?;
        write_header(
            worksheet,
            &header,
            &[
                "业务时间",
                "出库类型",
                "部门",
                "物品编码",
                "物品名称",
                "分类",
                "规格",
                "单位",
                "数量",
                "销售单价",
                "经办人",
                "用途",
                "备注",
            ],
        )?;
        worksheet
            .write_string(1, 0, "2026-06-02 10:30:00")
            .map_err(xlsx_error)?;
        worksheet
            .write_string(1, 1, "内部领用")
            .map_err(xlsx_error)?;
        worksheet.write_string(1, 2, "客房").map_err(xlsx_error)?;
        worksheet
            .write_string(1, 3, "YCYS-001")
            .map_err(xlsx_error)?;
        worksheet
            .write_string(1, 4, "一次性牙刷")
            .map_err(xlsx_error)?;
        worksheet.write_string(1, 5, "客耗品").map_err(xlsx_error)?;
        worksheet.write_string(1, 6, "软毛").map_err(xlsx_error)?;
        worksheet.write_string(1, 7, "支").map_err(xlsx_error)?;
        worksheet
            .write_number_with_format(1, 8, 10.0, &number)
            .map_err(xlsx_error)?;
        worksheet
            .write_number_with_format(1, 9, 0.0, &money)
            .map_err(xlsx_error)?;
        worksheet
            .write_string(1, 10, "管理员")
            .map_err(xlsx_error)?;
        worksheet
            .write_string(1, 11, "客房消耗")
            .map_err(xlsx_error)?;
        worksheet
            .write_string(1, 12, "示例，可删除")
            .map_err(xlsx_error)?;
        worksheet
            .write_string(2, 0, "2026-06-02 15:00:00")
            .map_err(xlsx_error)?;
        worksheet
            .write_string(2, 1, "酒店客人销售")
            .map_err(xlsx_error)?;
        worksheet.write_string(2, 2, "前台").map_err(xlsx_error)?;
        worksheet
            .write_string(2, 3, "YCYS-001")
            .map_err(xlsx_error)?;
        worksheet
            .write_string(2, 4, "一次性牙刷")
            .map_err(xlsx_error)?;
        worksheet
            .write_number_with_format(2, 8, 2.0, &number)
            .map_err(xlsx_error)?;
        worksheet
            .write_number_with_format(2, 9, 3.0, &money)
            .map_err(xlsx_error)?;
        worksheet
            .write_string(2, 10, "管理员")
            .map_err(xlsx_error)?;
        worksheet
            .write_string(2, 11, "客人购买")
            .map_err(xlsx_error)?;
        set_widths(
            worksheet,
            &[22, 18, 16, 16, 24, 18, 18, 10, 12, 12, 14, 18, 28],
        )?;
    }

    workbook.save(path).map_err(xlsx_error)?;
    Ok(())
}

fn write_header(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    format: &Format,
    headers: &[&str],
) -> AppResult<()> {
    for (index, label) in headers.iter().enumerate() {
        worksheet
            .write_string_with_format(0, index as u16, *label, format)
            .map_err(xlsx_error)?;
    }
    Ok(())
}

fn set_widths(worksheet: &mut rust_xlsxwriter::Worksheet, widths: &[u16]) -> AppResult<()> {
    for (index, width) in widths.iter().enumerate() {
        worksheet
            .set_column_width(index as u16, *width)
            .map_err(xlsx_error)?;
    }
    Ok(())
}

fn write_import_source_copy(report_dir: &Path, source_file: &str) -> AppResult<Option<PathBuf>> {
    let source = Path::new(source_file);
    let Some(file_name) = source.file_name() else {
        return Ok(None);
    };
    fs::create_dir_all(report_dir)?;
    let target = report_dir.join(format!(
        "{}-{}",
        chrono::Local::now().format("%Y%m%d%H%M%S"),
        file_name.to_string_lossy()
    ));
    fs::copy(source, &target)?;
    Ok(Some(target))
}

fn write_import_report(
    report_dir: &Path,
    result: &ImportResult,
    preview: &ImportPreview,
    mode: ImportMode,
    source_copy_path: Option<&Path>,
) -> AppResult<PathBuf> {
    fs::create_dir_all(report_dir)?;
    let report = ImportReportFile {
        job_id: result.job_id.clone(),
        source_file: result.source_file.clone(),
        source_copy_path: source_copy_path.map(|path| path.display().to_string()),
        mode: mode.label().to_string(),
        generated_at: chrono::Local::now().to_rfc3339(),
        sheet_count: preview.sheet_count,
        row_count: preview.row_count,
        item_count: preview.item_count,
        new_item_count: preview.new_item_count,
        existing_item_count: preview.existing_item_count,
        imported_items: result.imported_items,
        matched_items: result.matched_items,
        document_count: result.document_count,
        movement_count: result.movement_count,
        warning_count: result.warning_count,
        error_count: result.error_count,
        months: preview.months.clone(),
        warnings: preview.warnings.clone(),
        errors: preview.errors.clone(),
        items: preview.items.clone(),
    };
    let path = report_dir.join(format!(
        "aster-import-report-{}-{}.json",
        chrono::Local::now().format("%Y%m%d-%H%M%S"),
        result.job_id
    ));
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| AppError::Validation(format!("导入报告生成失败：{error}")))?;
    fs::write(&path, json)?;
    Ok(path)
}

fn detect_formula_errors(
    sheet_name: &str,
    excel_row: usize,
    row: &[Data],
    errors: &mut Vec<ImportMessage>,
) {
    for (index, cell) in row.iter().enumerate() {
        if let Data::Error(error) = cell {
            errors.push(message(
                "error",
                sheet_name,
                excel_row,
                Some(&column_label(index)),
                &format!("公式错误或单元格错误：{error:?}"),
            ));
        }
    }
}

fn warn_amount_mismatch(
    sheet_name: &str,
    excel_row: usize,
    label: &str,
    quantity: f64,
    price: f64,
    amount: Option<f64>,
    warnings: &mut Vec<ImportMessage>,
) {
    let Some(amount) = amount else {
        return;
    };
    if quantity <= 0.0 || price <= 0.0 {
        return;
    }
    let expected = round_money(quantity * price);
    if (round_money(amount) - expected).abs() > 0.01 {
        warnings.push(message(
            "warning",
            sheet_name,
            excel_row,
            None,
            &format!("{label}不平：表内金额 {amount:.2}，按数量×单价应为 {expected:.2}"),
        ));
    }
}

fn data_to_text(data: &Data) -> String {
    crate::services::import_value_parser::to_text(data)
}

fn data_to_number(data: &Data) -> Option<f64> {
    crate::services::import_value_parser::to_number(data)
}

fn data_to_datetime(data: &Data) -> Result<Option<String>, String> {
    match data {
        Data::Empty => Ok(None),
        Data::String(value) => parse_datetime_text(value),
        Data::DateTime(value) => {
            let (year, month, day, hour, minute, second, _) = value.to_ymd_hms_milli();
            let text = format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}");
            validate_datetime_text(&text).map(Some)
        }
        _ => Err("业务时间格式不支持，请填写为 2026-06-01 09:00:00".to_string()),
    }
}

fn parse_datetime_text(value: &str) -> Result<Option<String>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let normalized = trimmed.replace('T', " ");
    if normalized.len() == 10 {
        return Err("业务时间必须包含真实时间，格式示例：2026-06-01 09:00:00".to_string());
    }
    validate_datetime_text(&normalized).map(Some)
}

fn validate_datetime_text(value: &str) -> Result<String, String> {
    crate::services::stock_service::validate_business_datetime(value, "业务时间")
        .map_err(|error| error.to_string())?;
    let mut normalized = value.to_string();
    crate::services::stock_service::normalize_business_datetime(&mut normalized, "业务时间")
        .map_err(|error| error.to_string())?;
    Ok(normalized)
}

fn normalized_name(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn item_lookup_key(code: Option<&str>, name: &str) -> String {
    code.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("code:{value}"))
        .unwrap_or_else(|| format!("name:{}", normalized_name(name)))
}

fn parse_outbound_kind(value: &str) -> Option<&'static str> {
    match normalized_header(value).as_str() {
        "内部领用" | "内部员工领用" | "internal" => Some("internal"),
        "酒店客人销售" | "客人销售" | "销售" | "guestsale" => Some("guest_sale"),
        _ => None,
    }
}

fn month_from_datetime(value: &str) -> String {
    value.chars().take(7).collect()
}

fn normalized_header(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '_' && *ch != '-' && *ch != '/')
        .collect::<String>()
        .to_lowercase()
}

fn column_label(index: usize) -> String {
    let mut n = index + 1;
    let mut label = String::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        label.insert(0, (b'A' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    label
}

fn message(
    level: &str,
    sheet: &str,
    row: usize,
    column: Option<&str>,
    message: &str,
) -> ImportMessage {
    ImportMessage {
        level: level.to_string(),
        sheet: sheet.to_string(),
        row,
        column: column.map(str::to_string),
        message: message.to_string(),
    }
}

fn round_money(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn xlsx_error(error: XlsxError) -> AppError {
    AppError::Validation(format!("Excel 模板生成失败：{error}"))
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Debug, Clone)]
struct ParsedWorkbook {
    items: Vec<ParsedItem>,
    inbound_rows: Vec<ParsedInboundLine>,
    outbound_rows: Vec<ParsedOutboundLine>,
    warnings: Vec<ImportMessage>,
    errors: Vec<ImportMessage>,
    sheet_count: usize,
}

#[derive(Debug, Clone)]
struct ParsedItem {
    key: String,
    sheet_name: String,
    row_number: usize,
    code: Option<String>,
    name: String,
    category_name: Option<String>,
    spec: Option<String>,
    unit_name: Option<String>,
    default_price: f64,
    sale_price: f64,
    warning_quantity: f64,
    remark: Option<String>,
}

#[derive(Debug, Clone)]
struct ParsedInboundLine {
    sheet_name: String,
    row_number: usize,
    business_date: String,
    supplier_name: Option<String>,
    item_key: String,
    quantity: f64,
    unit_price: f64,
    amount: f64,
    handler: Option<String>,
    remark: Option<String>,
}

#[derive(Debug, Clone)]
struct ParsedOutboundLine {
    sheet_name: String,
    row_number: usize,
    business_date: String,
    outbound_kind: String,
    department_name: String,
    item_key: String,
    quantity: f64,
    sale_unit_price: f64,
    handler: Option<String>,
    purpose: Option<String>,
    remark: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct InboundGroupKey {
    business_date: String,
    supplier_name: Option<String>,
    handler: Option<String>,
    remark: Option<String>,
}

struct InboundGroup<'a> {
    business_date: String,
    supplier_name: Option<String>,
    handler: Option<String>,
    remark: Option<String>,
    rows: Vec<&'a ParsedInboundLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct OutboundGroupKey {
    business_date: String,
    outbound_kind: String,
    department_name: String,
    handler: Option<String>,
    purpose: Option<String>,
    remark: Option<String>,
}

struct OutboundGroup<'a> {
    business_date: String,
    outbound_kind: String,
    department_name: String,
    handler: Option<String>,
    purpose: Option<String>,
    remark: Option<String>,
    rows: Vec<&'a ParsedOutboundLine>,
}

struct ImportItemAccumulator {
    name: String,
    category_name: Option<String>,
    spec: Option<String>,
    unit_name: Option<String>,
    default_price: f64,
    opening_quantity: f64,
    inbound_quantity: f64,
    outbound_quantity: f64,
    existing: bool,
}

#[derive(Default)]
struct ImportMonthAccumulator {
    row_count: usize,
    inbound_quantity: f64,
    outbound_quantity: f64,
    outbound_amount: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportReportFile {
    job_id: String,
    source_file: String,
    source_copy_path: Option<String>,
    mode: String,
    generated_at: String,
    sheet_count: usize,
    row_count: usize,
    item_count: usize,
    new_item_count: usize,
    existing_item_count: usize,
    imported_items: usize,
    matched_items: usize,
    document_count: usize,
    movement_count: usize,
    warning_count: usize,
    error_count: usize,
    months: Vec<ImportMonthPreview>,
    warnings: Vec<ImportMessage>,
    errors: Vec<ImportMessage>,
    items: Vec<ImportItemPreview>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportMode {
    Full,
    ItemsOnly,
}

impl ImportMode {
    fn from_request(value: Option<&str>) -> AppResult<Self> {
        match value.unwrap_or("full") {
            "full" => Ok(Self::Full),
            "itemsOnly" | "items_only" => Ok(Self::ItemsOnly),
            other => Err(AppError::Validation(format!("不支持的导入模式：{other}"))),
        }
    }

    fn import_movements(self) -> bool {
        self == Self::Full
    }

    fn label(self) -> &'static str {
        match self {
            Self::Full => "完整导入",
            Self::ItemsOnly => "只导入物品档案",
        }
    }
}

struct SheetHeader {
    row_index: usize,
    columns: HashMap<String, usize>,
}

impl SheetHeader {
    fn from_range(
        range: &calamine::Range<Data>,
        sheet_name: &str,
        required: &[&str],
    ) -> AppResult<Self> {
        for (row_index, row) in range.rows().take(10).enumerate() {
            let mut columns = HashMap::new();
            for (index, cell) in row.iter().enumerate() {
                let text = data_to_text(cell);
                if text.trim().is_empty() {
                    continue;
                }
                columns.insert(normalized_header(&text), index);
            }
            if required
                .iter()
                .all(|label| columns.contains_key(&normalized_header(label)))
            {
                return Ok(Self { row_index, columns });
            }
        }
        Err(AppError::Validation(format!(
            "{sheet_name} 表头不匹配，缺少必填列：{}",
            required.join("、")
        )))
    }

    fn text(&self, row: &[Data], label: &str) -> String {
        self.columns
            .get(&normalized_header(label))
            .and_then(|index| row.get(*index))
            .map(data_to_text)
            .unwrap_or_default()
    }

    fn optional_text(&self, row: &[Data], label: &str) -> Option<String> {
        let value = self.text(row, label);
        if value.trim().is_empty() {
            None
        } else {
            Some(value)
        }
    }

    fn optional_number(&self, row: &[Data], label: &str) -> Option<f64> {
        self.columns
            .get(&normalized_header(label))
            .and_then(|index| row.get(*index))
            .and_then(data_to_number)
    }

    fn datetime(&self, row: &[Data], label: &str) -> Result<Option<String>, String> {
        self.columns
            .get(&normalized_header(label))
            .and_then(|index| row.get(*index))
            .map(data_to_datetime)
            .unwrap_or(Ok(None))
    }
}

#[allow(dead_code)]
fn _assert_preview_serializable<T: Serialize>(_value: &T) {}

#[cfg(test)]
mod tests {
    use calamine::CellErrorType;
    use rusqlite::Connection;
    use rust_xlsxwriter::Workbook;
    use tempfile::tempdir;
    use zip::ZipArchive;

    use super::*;
    use crate::app::paths::AppPaths;
    use crate::db::connection::Db;
    use crate::db::migrations;
    use crate::db::repository;

    fn write_template_workbook(path: &Path) {
        let mut workbook = Workbook::new();
        {
            let worksheet = workbook.add_worksheet();
            worksheet.set_name(ITEM_SHEET).unwrap();
            for (index, label) in [
                "物品编码",
                "物品名称",
                "分类",
                "规格",
                "单位",
                "参考进价",
                "参考售价",
                "预警库存",
                "备注",
            ]
            .iter()
            .enumerate()
            {
                worksheet.write_string(0, index as u16, *label).unwrap();
            }
            worksheet.write_string(1, 0, "ITEM-001").unwrap();
            worksheet.write_string(1, 1, "一次性牙刷").unwrap();
            worksheet.write_string(1, 2, "客耗").unwrap();
            worksheet.write_string(1, 3, "软毛").unwrap();
            worksheet.write_string(1, 4, "支").unwrap();
            worksheet.write_number(1, 5, 3.0).unwrap();
            worksheet.write_number(1, 6, 10.0).unwrap();
        }
        {
            let worksheet = workbook.add_worksheet();
            worksheet.set_name(INBOUND_SHEET).unwrap();
            for (index, label) in [
                "业务时间",
                "供应商",
                "物品编码",
                "物品名称",
                "分类",
                "规格",
                "单位",
                "数量",
                "进货单价",
                "进货金额",
                "经办人",
                "备注",
            ]
            .iter()
            .enumerate()
            {
                worksheet.write_string(0, index as u16, *label).unwrap();
            }
            worksheet.write_string(1, 0, "2026-06-01 09:00:00").unwrap();
            worksheet.write_string(1, 1, "供应商A").unwrap();
            worksheet.write_string(1, 2, "ITEM-001").unwrap();
            worksheet.write_string(1, 3, "一次性牙刷").unwrap();
            worksheet.write_string(1, 4, "客耗").unwrap();
            worksheet.write_string(1, 5, "软毛").unwrap();
            worksheet.write_string(1, 6, "支").unwrap();
            worksheet.write_number(1, 7, 10.0).unwrap();
            worksheet.write_number(1, 8, 3.0).unwrap();
            worksheet.write_number(1, 9, 30.0).unwrap();
        }
        {
            let worksheet = workbook.add_worksheet();
            worksheet.set_name(OUTBOUND_SHEET).unwrap();
            for (index, label) in [
                "业务时间",
                "出库类型",
                "部门",
                "物品编码",
                "物品名称",
                "分类",
                "规格",
                "单位",
                "数量",
                "销售单价",
                "经办人",
                "用途",
                "备注",
            ]
            .iter()
            .enumerate()
            {
                worksheet.write_string(0, index as u16, *label).unwrap();
            }
            worksheet.write_string(1, 0, "2026-06-02 10:00:00").unwrap();
            worksheet.write_string(1, 1, "酒店客人销售").unwrap();
            worksheet.write_string(1, 2, "前台").unwrap();
            worksheet.write_string(1, 3, "ITEM-001").unwrap();
            worksheet.write_string(1, 4, "一次性牙刷").unwrap();
            worksheet.write_number(1, 8, 2.0).unwrap();
            worksheet.write_number(1, 9, 10.0).unwrap();
        }
        workbook.save(path).unwrap();
    }

    #[test]
    fn export_import_template_creates_three_sheet_workbook() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("template.xlsx");
        write_import_template_workbook(&path).unwrap();
        let parsed = parse_import_template_workbook(path.to_str().unwrap()).unwrap();
        assert_eq!(parsed.sheet_count, 3);
        assert!(!parsed.items.is_empty());
        assert!(!parsed.inbound_rows.is_empty());
        assert!(!parsed.outbound_rows.is_empty());
    }

    #[test]
    fn preview_import_template_reads_item_inbound_outbound_sheets() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("template.xlsx");
        write_template_workbook(&path);
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();

        let parsed = parse_import_template_workbook(path.to_str().unwrap()).unwrap();
        let preview = build_preview(&conn, path.to_str().unwrap(), parsed).unwrap();
        assert_eq!(preview.sheet_count, 3);
        assert_eq!(preview.item_count, 1);
        assert_eq!(preview.inbound_quantity, 10.0);
        assert_eq!(preview.inbound_amount, 30.0);
        assert_eq!(preview.outbound_quantity, 2.0);
        assert_eq!(preview.outbound_amount, 20.0);
        assert_eq!(preview.document_count, 2);
        assert!(preview.errors.is_empty());
    }

    #[test]
    fn preview_import_template_rejects_old_monthly_workbook() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.xlsx");
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name("2026.1").unwrap();
        worksheet.write_string(0, 0, "名称").unwrap();
        worksheet.write_string(1, 0, "一次性牙刷").unwrap();
        workbook.save(&path).unwrap();

        let error = parse_import_template_workbook(path.to_str().unwrap()).unwrap_err();
        assert!(error.to_string().contains("缺少工作表"));
    }

    #[test]
    fn preview_template_workbook_reports_validation_messages() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("invalid.xlsx");
        let mut workbook = Workbook::new();
        {
            let worksheet = workbook.add_worksheet();
            worksheet.set_name(ITEM_SHEET).unwrap();
            for (index, label) in ["物品名称", "单位"].iter().enumerate() {
                worksheet.write_string(0, index as u16, *label).unwrap();
            }
            worksheet.write_string(1, 0, "问题物品").unwrap();
            worksheet.write_string(1, 1, "").unwrap();
        }
        {
            let worksheet = workbook.add_worksheet();
            worksheet.set_name(INBOUND_SHEET).unwrap();
            for (index, label) in ["业务时间", "物品名称", "数量", "进货单价", "进货金额"]
                .iter()
                .enumerate()
            {
                worksheet.write_string(0, index as u16, *label).unwrap();
            }
            worksheet.write_string(1, 0, "2026-06-01").unwrap();
            worksheet.write_string(1, 1, "问题物品").unwrap();
            worksheet.write_number(1, 2, -2.0).unwrap();
            worksheet.write_number(1, 3, 3.0).unwrap();
            worksheet.write_number(1, 4, 9.0).unwrap();
            worksheet.write_string(2, 0, "2026-06-01 09:00:00").unwrap();
            worksheet.write_string(2, 1, "金额不平物品").unwrap();
            worksheet.write_number(2, 2, 2.0).unwrap();
            worksheet.write_number(2, 3, 3.0).unwrap();
            worksheet.write_number(2, 4, 9.0).unwrap();
        }
        {
            let worksheet = workbook.add_worksheet();
            worksheet.set_name(OUTBOUND_SHEET).unwrap();
            for (index, label) in ["业务时间", "出库类型", "部门", "物品名称", "数量"]
                .iter()
                .enumerate()
            {
                worksheet.write_string(0, index as u16, *label).unwrap();
            }
        }
        workbook.save(&path).unwrap();

        let parsed = parse_import_template_workbook(path.to_str().unwrap()).unwrap();
        let error_text = parsed
            .errors
            .iter()
            .map(|message| message.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let warning_text = parsed
            .warnings
            .iter()
            .map(|message| message.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(error_text.contains("单位不能为空"));
        assert!(error_text.contains("必须包含真实时间"));
        assert!(warning_text.contains("进货金额不平"));
    }

    #[test]
    fn detect_formula_errors_reports_cell_errors() {
        let mut errors = Vec::new();
        detect_formula_errors(
            "导入模板",
            3,
            &[
                Data::String("2026-06".to_string()),
                Data::String("公式问题物品".to_string()),
                Data::Error(CellErrorType::Div0),
            ],
            &mut errors,
        );

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].level, "error");
        assert_eq!(errors[0].row, 3);
        assert_eq!(errors[0].column.as_deref(), Some("C"));
        assert!(errors[0].message.contains("公式错误"));
    }

    #[test]
    fn import_template_creates_batches_and_guest_sale_costs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("template-import.xlsx");
        write_template_workbook(&path);

        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();
        let parsed = parse_import_template_workbook(path.to_str().unwrap()).unwrap();
        let preview = build_preview(&conn, path.to_str().unwrap(), parsed.clone()).unwrap();

        let result = import_parsed_workbook(
            &mut conn,
            path.to_str().unwrap(),
            "job-template",
            &parsed,
            &preview,
            ImportMode::Full,
            false,
        )
        .unwrap();

        assert_eq!(result.imported_items, 1);
        assert_eq!(result.document_count, 2);
        let batch_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM stock_batches", [], |row| row.get(0))
            .unwrap();
        assert_eq!(batch_count, 1);
        let (sale_amount, cost_amount): (f64, f64) = conn
            .query_row(
                "SELECT sale_amount, cost_amount
                 FROM stock_document_lines
                 WHERE sale_amount IS NOT NULL",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(sale_amount, 20.0);
        assert_eq!(cost_amount, 6.0);
    }

    #[test]
    fn import_template_uses_existing_supplier_and_department_ids() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("template-existing-party.xlsx");
        write_template_workbook(&path);

        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO suppliers (id, name, enabled) VALUES ('supplier-existing', '供应商A', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO departments (id, code, name, enabled) VALUES ('dept-existing-front', 'D900', '前台', 1)",
            [],
        )
        .unwrap();
        let parsed = parse_import_template_workbook(path.to_str().unwrap()).unwrap();
        let preview = build_preview(&conn, path.to_str().unwrap(), parsed.clone()).unwrap();

        import_parsed_workbook(
            &mut conn,
            path.to_str().unwrap(),
            "job-existing-party",
            &parsed,
            &preview,
            ImportMode::Full,
            false,
        )
        .unwrap();

        let supplier_id: String = conn
            .query_row(
                "SELECT supplier_id FROM stock_documents WHERE document_type = 'inbound'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let department_id: String = conn
            .query_row(
                "SELECT department_id FROM stock_documents WHERE document_type = 'outbound'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(supplier_id, "supplier-existing");
        assert_eq!(department_id, "dept-existing-front");
    }

    #[test]
    fn import_items_only_creates_items_without_documents_or_movements() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("items-only.xlsx");
        write_template_workbook(&path);

        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();
        let parsed = parse_import_template_workbook(path.to_str().unwrap()).unwrap();
        let preview = build_preview(&conn, path.to_str().unwrap(), parsed.clone()).unwrap();

        let result = import_parsed_workbook(
            &mut conn,
            path.to_str().unwrap(),
            "job-items-only",
            &parsed,
            &preview,
            ImportMode::ItemsOnly,
            false,
        )
        .unwrap();

        assert_eq!(result.imported_items, 1);
        assert_eq!(result.document_count, 0);
        assert_eq!(result.movement_count, 0);
        let item_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM master_items WHERE name = '一次性牙刷'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(item_count, 1);
    }

    #[test]
    fn write_import_report_creates_json_migration_report() {
        let dir = tempdir().unwrap();
        let result = ImportResult {
            job_id: "job-report".to_string(),
            source_file: "/tmp/source.xlsx".to_string(),
            imported_items: 1,
            matched_items: 2,
            document_count: 3,
            movement_count: 4,
            warning_count: 0,
            error_count: 0,
            report_path: None,
            source_copy_path: None,
        };
        let preview = ImportPreview {
            source_file: result.source_file.clone(),
            sheet_count: 3,
            row_count: 2,
            item_count: 3,
            new_item_count: 1,
            existing_item_count: 2,
            opening_quantity: 0.0,
            opening_amount: 0.0,
            inbound_quantity: 0.0,
            inbound_amount: 0.0,
            outbound_quantity: 0.0,
            outbound_amount: 0.0,
            document_count: 3,
            warnings: Vec::new(),
            errors: Vec::new(),
            items: Vec::new(),
            months: Vec::new(),
        };

        let report_path =
            write_import_report(dir.path(), &result, &preview, ImportMode::Full, None).unwrap();
        let report_text = fs::read_to_string(report_path).unwrap();
        assert!(report_text.contains("\"jobId\": \"job-report\""));
        assert!(report_text.contains("\"mode\": \"完整导入\""));
        assert!(report_text.contains("\"movementCount\": 4"));
    }

    #[test]
    fn run_excel_import_requires_admin_before_parsing_workbook() {
        let dir = tempdir().unwrap();
        let paths = AppPaths {
            data_dir: dir.path().to_path_buf(),
            database_path: dir.path().join("aster.sqlite"),
            backup_dir: dir.path().join("backups"),
            export_dir: dir.path().join("exports"),
            import_report_dir: dir.path().join("import-reports"),
        };
        fs::create_dir_all(&paths.backup_dir).unwrap();
        fs::create_dir_all(&paths.export_dir).unwrap();
        fs::create_dir_all(&paths.import_report_dir).unwrap();
        let user = crate::domain::users::CurrentUser {
            id: "user-warehouse".to_string(),
            username: "warehouse".to_string(),
            display_name: "仓库员".to_string(),
            department_id: None,
            department_name: None,
            roles: vec![crate::domain::users::Role {
                id: "role-warehouse".to_string(),
                code: "warehouse".to_string(),
                name: "仓库员".to_string(),
            }],
            permissions: vec!["write_stock".to_string(), "view_reports".to_string()],
        };
        let state = AppState {
            db: Db::initialize(&paths).unwrap(),
            paths,
            session: std::sync::Arc::new(std::sync::Mutex::new(None)),
            host_service: std::sync::Arc::new(std::sync::Mutex::new(
                crate::services::host_service::HostServiceRuntime::default(),
            )),
        };
        crate::services::test_support::install_session(&state, user).unwrap();

        let error = run_excel_import(
            &state,
            RunImportRequest {
                path: "/path/that/does/not/exist.xlsx".to_string(),
                mode: None,
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("需要管理员权限"));
    }

    #[test]
    fn run_excel_import_rejects_preview_errors_without_backup_or_writes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("invalid-import.xlsx");
        let mut workbook = Workbook::new();
        {
            let worksheet = workbook.add_worksheet();
            worksheet.set_name(ITEM_SHEET).unwrap();
            worksheet.write_string(0, 0, "物品名称").unwrap();
            worksheet.write_string(0, 1, "单位").unwrap();
            worksheet.write_string(1, 0, "缺单位物品").unwrap();
        }
        {
            let worksheet = workbook.add_worksheet();
            worksheet.set_name(INBOUND_SHEET).unwrap();
            for (index, label) in ["业务时间", "物品名称", "数量", "进货单价"]
                .iter()
                .enumerate()
            {
                worksheet.write_string(0, index as u16, *label).unwrap();
            }
        }
        {
            let worksheet = workbook.add_worksheet();
            worksheet.set_name(OUTBOUND_SHEET).unwrap();
            for (index, label) in ["业务时间", "出库类型", "部门", "物品名称", "数量"]
                .iter()
                .enumerate()
            {
                worksheet.write_string(0, index as u16, *label).unwrap();
            }
        }
        workbook.save(&path).unwrap();

        let paths = test_paths(dir.path());
        let state = admin_state(paths);
        let error = run_excel_import(
            &state,
            RunImportRequest {
                path: path.display().to_string(),
                mode: None,
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("导入预览存在"));
        state
            .db
            .with_conn(|conn| {
                let backup_count: i64 =
                    conn.query_row("SELECT COUNT(*) FROM backup_jobs", [], |row| row.get(0))?;
                let item_count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM master_items WHERE name = '缺单位物品'",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(backup_count, 0);
                assert_eq!(item_count, 0);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn run_excel_import_creates_before_import_backup_before_writes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("valid-import.xlsx");
        write_template_workbook(&path);
        let paths = test_paths(dir.path());
        let state = admin_state(paths);

        let result = run_excel_import(
            &state,
            RunImportRequest {
                path: path.display().to_string(),
                mode: Some("itemsOnly".to_string()),
            },
        )
        .unwrap();

        assert_eq!(result.imported_items, 1);
        assert!(result
            .report_path
            .as_deref()
            .is_some_and(|path| Path::new(path).exists()));
        let backup_file = state
            .db
            .with_conn(|conn| {
                let backup_type: String = conn.query_row(
                    "SELECT backup_type FROM backup_jobs ORDER BY created_at DESC LIMIT 1",
                    [],
                    |row| row.get(0),
                )?;
                let backup_file: String = conn.query_row(
                    "SELECT backup_file FROM backup_jobs WHERE backup_type = 'before_import'",
                    [],
                    |row| row.get(0),
                )?;
                let imported_count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM master_items WHERE name = '一次性牙刷'",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(backup_type, "before_import");
                assert_eq!(imported_count, 1);
                Ok(backup_file)
            })
            .unwrap();
        assert!(Path::new(&backup_file).exists());

        let backup_database = dir.path().join("before-import.sqlite");
        let backup_zip = fs::File::open(&backup_file).unwrap();
        let mut archive = ZipArchive::new(backup_zip).unwrap();
        let mut database_entry = archive.by_name("aster.sqlite").unwrap();
        let mut database_file = fs::File::create(&backup_database).unwrap();
        std::io::copy(&mut database_entry, &mut database_file).unwrap();
        drop(database_file);
        drop(database_entry);
        drop(archive);

        let backup_conn = Connection::open(&backup_database).unwrap();
        let item_count_in_backup: i64 = backup_conn
            .query_row(
                "SELECT COUNT(*) FROM master_items WHERE name = '一次性牙刷'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(item_count_in_backup, 0);
    }

    #[test]
    fn preview_excel_import_rejects_client_mode_before_parsing_workbook() {
        let dir = tempdir().unwrap();
        let paths = test_paths(dir.path());
        let state = AppState {
            db: Db::initialize(&paths).unwrap(),
            paths,
            session: std::sync::Arc::new(std::sync::Mutex::new(None)),
            host_service: std::sync::Arc::new(std::sync::Mutex::new(
                crate::services::host_service::HostServiceRuntime::default(),
            )),
        };
        state
            .db
            .with_conn(|conn| repository::set_setting(conn, "runtime_mode", "client"))
            .unwrap();

        let error = preview_excel_import(
            &state,
            ImportPreviewRequest {
                path: "/path/that/does/not/exist.xlsx".to_string(),
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("客户端模式不能操作正式数据库"));
    }

    fn test_paths(root: &Path) -> AppPaths {
        let paths = AppPaths {
            data_dir: root.join("app-data"),
            database_path: root.join("app-data").join("aster.sqlite"),
            backup_dir: root.join("backups"),
            export_dir: root.join("exports"),
            import_report_dir: root.join("import-reports"),
        };
        fs::create_dir_all(&paths.data_dir).unwrap();
        fs::create_dir_all(&paths.backup_dir).unwrap();
        fs::create_dir_all(&paths.export_dir).unwrap();
        fs::create_dir_all(&paths.import_report_dir).unwrap();
        paths
    }

    fn admin_state(paths: AppPaths) -> AppState {
        let user = crate::domain::users::CurrentUser {
            id: "user-admin".to_string(),
            username: "admin".to_string(),
            display_name: "管理员".to_string(),
            department_id: None,
            department_name: None,
            roles: vec![crate::domain::users::Role {
                id: "role-admin".to_string(),
                code: "admin".to_string(),
                name: "管理员".to_string(),
            }],
            permissions: vec![
                "manage_users".to_string(),
                "manage_settings".to_string(),
                "write_stock".to_string(),
                "view_reports".to_string(),
                "dangerous_operations".to_string(),
            ],
        };
        let state = AppState {
            db: Db::initialize(&paths).unwrap(),
            paths,
            session: std::sync::Arc::new(std::sync::Mutex::new(None)),
            host_service: std::sync::Arc::new(std::sync::Mutex::new(
                crate::services::host_service::HostServiceRuntime::default(),
            )),
        };
        crate::services::test_support::install_session(&state, user).unwrap();
        state
    }
}
