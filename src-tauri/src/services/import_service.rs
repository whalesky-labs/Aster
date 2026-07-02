use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use calamine::{open_workbook_auto, Data, Reader};
use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::Serialize;
use uuid::Uuid;

use crate::app::state::AppState;
use crate::domain::imports::{
    ImportItemPreview, ImportMessage, ImportMonthPreview, ImportPreview, ImportPreviewRequest,
    ImportResult, RunImportRequest,
};
use crate::error::{AppError, AppResult};
use crate::services::backup_service;

const DEPARTMENT_COLUMNS: [(&str, usize, Option<usize>); 8] = [
    ("行政办", 11, Some(12)),
    ("餐饮", 13, Some(14)),
    ("温泉+前台", 15, Some(16)),
    ("客房", 17, Some(18)),
    ("工程", 19, Some(20)),
    ("安保", 21, Some(22)),
    ("妇女之家", 23, Some(24)),
    ("调物品", 25, None),
];

pub fn preview_excel_import(
    state: &AppState,
    request: ImportPreviewRequest,
) -> AppResult<ImportPreview> {
    crate::services::safety_service::require_local_primary_database(state, "预览 Excel 导入")?;
    let parsed = parse_legacy_workbook(&request.path)?;
    state
        .db
        .with_conn(|conn| build_preview(conn, &request.path, parsed))
}

pub fn run_excel_import(state: &AppState, request: RunImportRequest) -> AppResult<ImportResult> {
    crate::services::safety_service::require_dangerous_local_operation(state, "正式导入 Excel")?;
    let mode = ImportMode::from_request(request.mode.as_deref())?;
    let parsed = parse_legacy_workbook(&request.path)?;
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

    state.db.with_conn_mut(|conn| {
        let mut tx = conn.transaction()?;
        let job_id = new_id();
        let preview = build_preview_for_tx(&tx, &source_file, &parsed)?;
        if !preview.errors.is_empty() {
            return Err(AppError::Validation(format!(
                "导入预览存在 {} 个错误，请修正 Excel 后再导入",
                preview.errors.len()
            )));
        }
        let result =
            import_parsed_workbook(&mut tx, &source_file, &job_id, &parsed, &preview, mode)?;
        tx.commit()?;
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

fn parse_legacy_workbook(path: &str) -> AppResult<ParsedWorkbook> {
    let path_ref = Path::new(path);
    if !path_ref.exists() {
        return Err(AppError::Validation(format!("Excel 文件不存在：{path}")));
    }

    let mut workbook = open_workbook_auto(path_ref)
        .map_err(|error| AppError::Validation(format!("无法读取 Excel：{error}")))?;

    let mut rows = Vec::new();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut parsed_sheets = HashSet::new();
    let mut seen_rows = HashSet::new();

    for sheet_name in workbook.sheet_names().to_owned() {
        let range = workbook.worksheet_range(&sheet_name).map_err(|error| {
            AppError::Validation(format!("读取工作表失败：{sheet_name}：{error}"))
        })?;

        if let Some(month) = parse_sheet_month(&sheet_name) {
            parsed_sheets.insert(sheet_name.clone());
            parse_legacy_sheet(
                &sheet_name,
                &month,
                &range,
                &mut rows,
                &mut warnings,
                &mut errors,
                &mut seen_rows,
            );
            continue;
        }

        if let Some(header) = TemplateHeader::from_range(&range) {
            parsed_sheets.insert(sheet_name.clone());
            parse_template_sheet(
                &sheet_name,
                &range,
                &header,
                &mut rows,
                &mut warnings,
                &mut errors,
                &mut seen_rows,
            );
        }
    }

    if parsed_sheets.is_empty() {
        return Err(AppError::Validation(
            "未找到可识别的工作表。旧表工作表名应类似 2026.1、2026.02；通用模板需包含月份、物品名称等表头".to_string(),
        ));
    }

    Ok(ParsedWorkbook {
        rows,
        warnings,
        errors,
        sheet_count: parsed_sheets.len(),
    })
}

fn parse_legacy_sheet(
    sheet_name: &str,
    month: &str,
    range: &calamine::Range<Data>,
    rows: &mut Vec<ParsedRow>,
    warnings: &mut Vec<ImportMessage>,
    errors: &mut Vec<ImportMessage>,
    seen_rows: &mut HashSet<(String, String, String)>,
) {
    for (row_index, row) in range.rows().enumerate() {
        let excel_row = row_index + 1;
        let item_name = text_at(row, 0);
        if item_name.trim().is_empty() {
            if row.iter().all(is_empty_cell) {
                continue;
            }
            errors.push(message(
                "error",
                sheet_name,
                excel_row,
                Some("A"),
                "物品名称不能为空",
            ));
            continue;
        }
        if should_skip_row(&item_name) {
            continue;
        }

        let category_name = empty_to_none(text_at(row, 1));
        let unit_name = empty_to_none(text_at(row, 5));
        let spec = empty_to_none(text_at(row, 6));
        let opening_quantity = number_at(row, 2).unwrap_or(0.0);
        let opening_price = number_at(row, 3).unwrap_or(0.0);
        let opening_amount = number_at(row, 4).unwrap_or_else(|| opening_quantity * opening_price);
        let inbound_quantity = number_at(row, 7).unwrap_or(0.0);
        let inbound_price = number_at(row, 8).unwrap_or(0.0);
        let inbound_amount = number_at(row, 9).unwrap_or_else(|| inbound_quantity * inbound_price);
        let average_price = number_at(row, 10)
            .or_else(|| positive_price(opening_quantity, opening_amount))
            .or_else(|| positive_price(inbound_quantity, inbound_amount))
            .unwrap_or(0.0);

        detect_formula_errors(sheet_name, excel_row, row, errors);
        validate_import_row(
            sheet_name,
            excel_row,
            month,
            &item_name,
            unit_name.as_deref(),
            opening_quantity,
            opening_price,
            Some(opening_amount),
            inbound_quantity,
            inbound_price,
            Some(inbound_amount),
            None,
            None,
            seen_rows,
            warnings,
            errors,
        );

        let mut outbound_lines = Vec::new();
        for (department_name, quantity_col, amount_col) in DEPARTMENT_COLUMNS {
            let quantity = number_at(row, quantity_col).unwrap_or(0.0);
            let amount = amount_col
                .and_then(|col| number_at(row, col))
                .unwrap_or_else(|| quantity * average_price);
            if quantity > 0.0 {
                outbound_lines.push(ParsedDepartmentIssue {
                    department_name: department_name.to_string(),
                    quantity,
                    amount: round_money(amount),
                });
            }
        }

        detect_unmapped_legacy_department_columns(sheet_name, excel_row, row, warnings);

        rows.push(ParsedRow {
            sheet_name: sheet_name.to_string(),
            row_number: excel_row,
            month: month.to_string(),
            item_name: normalized_name(&item_name),
            category_name,
            unit_name,
            spec,
            opening_quantity,
            opening_price,
            opening_amount: round_money(opening_amount),
            inbound_quantity,
            inbound_price,
            inbound_amount: round_money(inbound_amount),
            average_price,
            outbound_lines,
        });
    }
}

fn parse_template_sheet(
    sheet_name: &str,
    range: &calamine::Range<Data>,
    header: &TemplateHeader,
    rows: &mut Vec<ParsedRow>,
    warnings: &mut Vec<ImportMessage>,
    errors: &mut Vec<ImportMessage>,
    seen_rows: &mut HashSet<(String, String, String)>,
) {
    for (row_index, row) in range.rows().enumerate().skip(header.row_index + 1) {
        let excel_row = row_index + 1;
        let item_name = text_at(row, header.item_name);
        if item_name.trim().is_empty() {
            if row.iter().all(is_empty_cell) {
                continue;
            }
            errors.push(message(
                "error",
                sheet_name,
                excel_row,
                None,
                "物品名称不能为空",
            ));
            continue;
        }

        let Some(month) = header
            .month
            .and_then(|index| parse_month_cell(row.get(index)))
            .or_else(|| {
                header
                    .business_date
                    .and_then(|index| parse_month_cell(row.get(index)))
            })
        else {
            errors.push(message(
                "error",
                sheet_name,
                excel_row,
                None,
                "通用模板必须填写月份，格式示例：2026-06",
            ));
            continue;
        };
        detect_formula_errors(sheet_name, excel_row, row, errors);

        let opening_quantity = header
            .opening_quantity
            .and_then(|index| number_at(row, index))
            .unwrap_or(0.0);
        let opening_price = header
            .opening_price
            .and_then(|index| number_at(row, index))
            .unwrap_or(0.0);
        let opening_amount = header
            .opening_amount
            .and_then(|index| number_at(row, index))
            .unwrap_or_else(|| opening_quantity * opening_price);
        let inbound_quantity = header
            .inbound_quantity
            .and_then(|index| number_at(row, index))
            .unwrap_or(0.0);
        let inbound_price = header
            .inbound_price
            .and_then(|index| number_at(row, index))
            .unwrap_or(0.0);
        let inbound_amount = header
            .inbound_amount
            .and_then(|index| number_at(row, index))
            .unwrap_or_else(|| inbound_quantity * inbound_price);
        let average_price = header
            .average_price
            .and_then(|index| number_at(row, index))
            .or_else(|| positive_price(opening_quantity, opening_amount))
            .or_else(|| positive_price(inbound_quantity, inbound_amount))
            .unwrap_or_else(|| first_positive(opening_price, inbound_price, 0.0));

        let outbound_quantity = header
            .outbound_quantity
            .and_then(|index| number_at(row, index))
            .unwrap_or(0.0);
        let outbound_amount = header
            .outbound_amount
            .and_then(|index| number_at(row, index))
            .unwrap_or_else(|| outbound_quantity * average_price);
        let department_name = header
            .department_name
            .map(|index| text_at(row, index))
            .unwrap_or_default();
        validate_import_row(
            sheet_name,
            excel_row,
            &month,
            &item_name,
            header
                .unit_name
                .and_then(|index| empty_to_none(text_at(row, index)))
                .as_deref(),
            opening_quantity,
            opening_price,
            header.opening_amount.and_then(|_| Some(opening_amount)),
            inbound_quantity,
            inbound_price,
            header.inbound_amount.and_then(|_| Some(inbound_amount)),
            Some(outbound_quantity),
            header.outbound_amount.and_then(|_| Some(outbound_amount)),
            seen_rows,
            warnings,
            errors,
        );
        let mut outbound_lines = Vec::new();
        if outbound_quantity > 0.0 {
            if department_name.trim().is_empty() {
                errors.push(message(
                    "error",
                    sheet_name,
                    excel_row,
                    None,
                    "出库数量大于 0 时必须填写出库部门",
                ));
            } else {
                outbound_lines.push(ParsedDepartmentIssue {
                    department_name: normalized_name(&department_name),
                    quantity: outbound_quantity,
                    amount: round_money(outbound_amount),
                });
            }
        }

        rows.push(ParsedRow {
            sheet_name: sheet_name.to_string(),
            row_number: excel_row,
            month,
            item_name: normalized_name(&item_name),
            category_name: header
                .category_name
                .and_then(|index| empty_to_none(text_at(row, index))),
            unit_name: header
                .unit_name
                .and_then(|index| empty_to_none(text_at(row, index))),
            spec: header
                .spec
                .and_then(|index| empty_to_none(text_at(row, index))),
            opening_quantity,
            opening_price,
            opening_amount: round_money(opening_amount),
            inbound_quantity,
            inbound_price,
            inbound_amount: round_money(inbound_amount),
            average_price,
            outbound_lines,
        });
    }
}

fn build_preview(
    conn: &Connection,
    source_file: &str,
    parsed: ParsedWorkbook,
) -> AppResult<ImportPreview> {
    build_preview_from_parts(conn, source_file, &parsed)
}

fn build_preview_for_tx(
    tx: &Transaction<'_>,
    source_file: &str,
    parsed: &ParsedWorkbook,
) -> AppResult<ImportPreview> {
    build_preview_from_parts(tx, source_file, parsed)
}

fn build_preview_from_parts(
    conn: &Connection,
    source_file: &str,
    parsed: &ParsedWorkbook,
) -> AppResult<ImportPreview> {
    let existing_names = existing_item_names(conn)?;
    let mut item_map: BTreeMap<String, ImportItemAccumulator> = BTreeMap::new();
    let mut month_map: BTreeMap<String, ImportMonthAccumulator> = BTreeMap::new();
    let mut opening_seen = HashSet::new();
    let mut document_keys = HashSet::new();

    for row in &parsed.rows {
        let item = item_map
            .entry(row.item_name.clone())
            .or_insert_with(|| ImportItemAccumulator {
                category_name: row.category_name.clone(),
                spec: row.spec.clone(),
                unit_name: row.unit_name.clone(),
                default_price: first_positive(
                    row.average_price,
                    row.inbound_price,
                    row.opening_price,
                ),
                opening_quantity: 0.0,
                inbound_quantity: 0.0,
                outbound_quantity: 0.0,
                existing: existing_names.contains(&row.item_name),
            });

        if item.default_price <= 0.0 {
            item.default_price =
                first_positive(row.average_price, row.inbound_price, row.opening_price);
        }
        item.inbound_quantity += row.inbound_quantity.max(0.0);
        if !opening_seen.contains(&row.item_name) && row.opening_quantity > 0.0 {
            item.opening_quantity += row.opening_quantity;
            opening_seen.insert(row.item_name.clone());
            document_keys.insert((row.month.clone(), "opening".to_string(), "期初".to_string()));
        }
        for issue in &row.outbound_lines {
            item.outbound_quantity += issue.quantity;
            document_keys.insert((
                row.month.clone(),
                "outbound".to_string(),
                issue.department_name.clone(),
            ));
        }
        if row.inbound_quantity > 0.0 {
            document_keys.insert((
                row.month.clone(),
                "inbound".to_string(),
                "导入入库".to_string(),
            ));
        }

        let month = month_map.entry(row.month.clone()).or_default();
        month.row_count += 1;
        if !opening_seen.contains(&format!("{}:{}", row.month, row.item_name))
            && row.opening_quantity > 0.0
        {
            month.opening_quantity += row.opening_quantity;
        }
        month.inbound_quantity += row.inbound_quantity.max(0.0);
        for issue in &row.outbound_lines {
            month.outbound_quantity += issue.quantity;
            month.outbound_amount += issue.amount;
        }
    }

    let mut first_opening_items = HashSet::new();
    let opening_quantity = parsed
        .rows
        .iter()
        .filter(|row| {
            row.opening_quantity > 0.0 && first_opening_items.insert(row.item_name.clone())
        })
        .map(|row| row.opening_quantity)
        .sum();
    let mut first_opening_amount_items = HashSet::new();
    let opening_amount = parsed
        .rows
        .iter()
        .filter(|row| {
            row.opening_amount > 0.0 && first_opening_amount_items.insert(row.item_name.clone())
        })
        .map(|row| row.opening_amount)
        .sum();
    let inbound_quantity = parsed
        .rows
        .iter()
        .map(|row| row.inbound_quantity.max(0.0))
        .sum();
    let inbound_amount = parsed
        .rows
        .iter()
        .map(|row| row.inbound_amount.max(0.0))
        .sum();
    let outbound_quantity = parsed
        .rows
        .iter()
        .flat_map(|row| &row.outbound_lines)
        .map(|line| line.quantity)
        .sum();
    let outbound_amount = parsed
        .rows
        .iter()
        .flat_map(|row| &row.outbound_lines)
        .map(|line| line.amount)
        .sum();

    let items = item_map
        .into_iter()
        .map(|(name, item)| ImportItemPreview {
            name,
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
        row_count: parsed.rows.len(),
        item_count: items.len(),
        new_item_count: items.len().saturating_sub(existing_item_count),
        existing_item_count,
        opening_quantity,
        opening_amount: round_money(opening_amount),
        inbound_quantity,
        inbound_amount: round_money(inbound_amount),
        outbound_quantity,
        outbound_amount: round_money(outbound_amount),
        document_count: document_keys.len(),
        warnings: parsed.warnings.clone(),
        errors: parsed.errors.clone(),
        items,
        months: month_map
            .into_iter()
            .map(|(month, data)| ImportMonthPreview {
                month,
                row_count: data.row_count,
                opening_quantity: data.opening_quantity,
                inbound_quantity: data.inbound_quantity,
                outbound_quantity: data.outbound_quantity,
                outbound_amount: round_money(data.outbound_amount),
            })
            .collect(),
    })
}

fn import_parsed_workbook(
    tx: &mut Transaction<'_>,
    source_file: &str,
    job_id: &str,
    parsed: &ParsedWorkbook,
    preview: &ImportPreview,
    mode: ImportMode,
) -> AppResult<ImportResult> {
    tx.execute(
        "INSERT INTO import_jobs (id, source_file, status, total_rows, warning_rows, error_rows, report_json)
         VALUES (?1, ?2, 'previewed', ?3, ?4, ?5, ?6)",
        params![
            job_id,
            source_file,
            parsed.rows.len() as i64,
            parsed.warnings.len() as i64,
            parsed.errors.len() as i64,
            serde_json::to_string(preview).unwrap_or_else(|_| "{}".to_string())
        ],
    )?;

    let category_ids = ensure_categories(tx, parsed)?;
    let unit_ids = ensure_units(tx, parsed)?;
    let department_ids = if mode.import_movements() {
        ensure_departments(tx, parsed)?
    } else {
        HashMap::new()
    };
    let mut item_ids = HashMap::new();
    let mut imported_items = 0;
    let mut matched_items = 0;

    for item in &preview.items {
        let item_id = find_item_id(tx, &item.name)?;
        if let Some(existing_id) = item_id {
            matched_items += 1;
            item_ids.insert(item.name.clone(), existing_id);
            continue;
        }

        let id = new_id();
        let code = next_item_code(tx)?;
        tx.execute(
            "INSERT INTO master_items (
               id, code, name, category_id, spec, unit_id, default_price,
               warning_quantity, enabled, remark
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 1, ?8)",
            params![
                id,
                code,
                item.name,
                item.category_name
                    .as_ref()
                    .and_then(|name| category_ids.get(name))
                    .cloned(),
                item.spec.clone(),
                item.unit_name
                    .as_ref()
                    .and_then(|name| unit_ids.get(name))
                    .cloned(),
                item.default_price,
                "Excel 迁移导入"
            ],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO stock_balances (id, item_id) VALUES (?1, ?2)",
            params![new_id(), id],
        )?;
        imported_items += 1;
        item_ids.insert(item.name.clone(), id);
    }

    let mut opening_seen = HashSet::new();
    let mut document_count = 0;
    let mut movement_count = 0;

    if mode.import_movements() {
        for (month, rows) in rows_by_month(parsed) {
            let business_date = month_start_date(&month)?;
            let opening_lines = rows
                .iter()
                .filter(|row| {
                    row.opening_quantity > 0.0 && opening_seen.insert(row.item_name.clone())
                })
                .filter_map(|row| {
                    item_ids
                        .get(&row.item_name)
                        .map(|item_id| (row, item_id.clone()))
                })
                .collect::<Vec<_>>();
            if !opening_lines.is_empty() {
                let document_id = create_document(
                    tx,
                    "inbound",
                    &business_date,
                    None,
                    None,
                    "Excel迁移",
                    "期初库存导入",
                )?;
                document_count += 1;
                for (row, item_id) in opening_lines {
                    let price = first_positive(row.average_price, row.opening_price, 0.0);
                    write_stock_line_and_movement(
                        tx,
                        &document_id,
                        &business_date,
                        &item_id,
                        "in",
                        "opening",
                        row.opening_quantity,
                        price,
                        row.opening_amount,
                        None,
                        None,
                        &format!("{} 第 {} 行期初", row.sheet_name, row.row_number),
                    )?;
                    movement_count += 1;
                }
            }

            let inbound_lines = rows
                .iter()
                .filter(|row| row.inbound_quantity > 0.0)
                .filter_map(|row| {
                    item_ids
                        .get(&row.item_name)
                        .map(|item_id| (row, item_id.clone()))
                })
                .collect::<Vec<_>>();
            if !inbound_lines.is_empty() {
                let document_id = create_document(
                    tx,
                    "inbound",
                    &business_date,
                    None,
                    None,
                    "Excel迁移",
                    "本月入库导入",
                )?;
                document_count += 1;
                for (row, item_id) in inbound_lines {
                    let price =
                        first_positive(row.inbound_price, row.average_price, row.opening_price);
                    write_stock_line_and_movement(
                        tx,
                        &document_id,
                        &business_date,
                        &item_id,
                        "in",
                        "inbound",
                        row.inbound_quantity,
                        price,
                        row.inbound_amount,
                        None,
                        None,
                        &format!("{} 第 {} 行入库", row.sheet_name, row.row_number),
                    )?;
                    movement_count += 1;
                }
            }

            for (department_name, department_rows) in rows_by_department(&rows) {
                let Some(department_id) = department_ids.get(&department_name).cloned() else {
                    continue;
                };
                let document_id = create_document(
                    tx,
                    "outbound",
                    &business_date,
                    Some(&department_id),
                    None,
                    "Excel迁移",
                    &format!("{department_name} 部门领用导入"),
                )?;
                document_count += 1;
                for (row, issue) in department_rows {
                    let Some(item_id) = item_ids.get(&row.item_name) else {
                        continue;
                    };
                    let price = if issue.quantity > 0.0 {
                        round_price(issue.amount / issue.quantity)
                    } else {
                        row.average_price
                    };
                    write_stock_line_and_movement(
                        tx,
                        &document_id,
                        &business_date,
                        item_id,
                        "out",
                        "outbound",
                        issue.quantity,
                        price,
                        issue.amount,
                        Some(&department_id),
                        None,
                        &format!("{} 第 {} 行领用", row.sheet_name, row.row_number),
                    )?;
                    movement_count += 1;
                }
            }
        }
    }

    tx.execute(
        "UPDATE import_jobs
         SET status = 'imported', success_rows = ?1, completed_at = CURRENT_TIMESTAMP
         WHERE id = ?2",
        params![parsed.rows.len() as i64, job_id],
    )?;
    tx.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, 'run_excel_import', 'import_job', ?2, ?3, 'system')",
        params![
            new_id(),
            job_id,
            format!(
                "导入 Excel：{}，{} 行，{} 条流水",
                source_file,
                parsed.rows.len(),
                movement_count
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

fn write_stock_line_and_movement(
    tx: &Transaction<'_>,
    document_id: &str,
    business_date: &str,
    item_id: &str,
    direction: &str,
    movement_type: &str,
    quantity: f64,
    unit_price: f64,
    amount: f64,
    department_id: Option<&str>,
    supplier_id: Option<&str>,
    remark: &str,
) -> AppResult<()> {
    let department_name = snapshot_department_name(tx, department_id)?;
    let supplier_name = snapshot_supplier_name(tx, supplier_id)?;
    let line_id = new_id();
    let rounded_amount = round_money(if amount > 0.0 {
        amount
    } else {
        quantity * unit_price
    });
    tx.execute(
        "INSERT INTO stock_document_lines (id, document_id, item_id, quantity, unit_price, amount, remark)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![line_id, document_id, item_id, quantity, unit_price, rounded_amount, remark],
    )?;

    apply_import_balance(tx, item_id, direction, quantity, unit_price, rounded_amount)?;

    tx.execute(
        "INSERT INTO stock_movements (
           id, movement_date, item_id, direction, quantity, unit_price, amount,
           document_id, document_line_id, department_id, department_name,
           supplier_id, supplier_name, movement_type,
           operator, remark
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 'Excel迁移', ?15)",
        params![
            new_id(),
            business_date,
            item_id,
            direction,
            quantity,
            unit_price,
            rounded_amount,
            document_id,
            line_id,
            department_id,
            department_name,
            supplier_id,
            supplier_name,
            movement_type,
            remark
        ],
    )?;
    Ok(())
}

fn apply_import_balance(
    tx: &Transaction<'_>,
    item_id: &str,
    direction: &str,
    quantity: f64,
    unit_price: f64,
    amount: f64,
) -> AppResult<()> {
    let existing = tx
        .query_row(
            "SELECT quantity, amount, average_price, last_inbound_price
             FROM stock_balances WHERE item_id = ?1",
            params![item_id],
            |row| {
                Ok((
                    row.get::<_, f64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, f64>(3)?,
                ))
            },
        )
        .optional()?
        .unwrap_or((0.0, 0.0, unit_price, 0.0));

    let (old_qty, old_amount, old_avg_price, old_last_price) = existing;
    let (new_qty, new_amount, new_avg_price, new_last_price) = if direction == "in" {
        let next_qty = old_qty + quantity;
        let next_amount = old_amount + amount;
        let next_avg = if next_qty.abs() < f64::EPSILON {
            0.0
        } else {
            round_price(next_amount / next_qty)
        };
        (next_qty, round_money(next_amount), next_avg, unit_price)
    } else {
        let next_qty = old_qty - quantity;
        let out_amount = if amount > 0.0 {
            amount
        } else {
            quantity * old_avg_price.max(unit_price)
        };
        let next_amount = old_amount - out_amount;
        let next_avg = if next_qty.abs() < f64::EPSILON {
            0.0
        } else {
            round_price(next_amount / next_qty)
        };
        (next_qty, round_money(next_amount), next_avg, old_last_price)
    };

    tx.execute(
        "INSERT INTO stock_balances (
           id, item_id, quantity, amount, average_price, last_inbound_price, updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP)
         ON CONFLICT(item_id) DO UPDATE SET
           quantity = excluded.quantity,
           amount = excluded.amount,
           average_price = excluded.average_price,
           last_inbound_price = excluded.last_inbound_price,
           updated_at = CURRENT_TIMESTAMP",
        params![
            new_id(),
            item_id,
            new_qty,
            new_amount,
            new_avg_price,
            new_last_price
        ],
    )?;
    Ok(())
}

fn create_document(
    tx: &Transaction<'_>,
    document_type: &str,
    business_date: &str,
    department_id: Option<&str>,
    supplier_id: Option<&str>,
    handler: &str,
    remark: &str,
) -> AppResult<String> {
    let document_id = new_id();
    let document_no = next_document_no(tx, document_type, business_date)?;
    let department_name = snapshot_department_name(tx, department_id)?;
    let supplier_name = snapshot_supplier_name(tx, supplier_id)?;
    tx.execute(
        "INSERT INTO stock_documents (
           id, document_no, document_type, business_date, department_id,
           department_name, supplier_id, supplier_name, handler, purpose, status, remark, confirmed_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'confirmed', ?11, CURRENT_TIMESTAMP)",
        params![
            document_id,
            document_no,
            document_type,
            business_date,
            department_id,
            department_name,
            supplier_id,
            supplier_name,
            handler,
            if document_type == "outbound" {
                remark
            } else {
                ""
            },
            remark
        ],
    )?;
    Ok(document_id)
}

fn snapshot_department_name(
    tx: &Transaction<'_>,
    department_id: Option<&str>,
) -> AppResult<Option<String>> {
    let Some(department_id) = department_id else {
        return Ok(None);
    };
    tx.query_row(
        "SELECT name FROM departments WHERE id = ?1",
        params![department_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

fn snapshot_supplier_name(
    tx: &Transaction<'_>,
    supplier_id: Option<&str>,
) -> AppResult<Option<String>> {
    let Some(supplier_id) = supplier_id else {
        return Ok(None);
    };
    tx.query_row(
        "SELECT name FROM suppliers WHERE id = ?1",
        params![supplier_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

fn ensure_categories(
    tx: &Transaction<'_>,
    parsed: &ParsedWorkbook,
) -> AppResult<HashMap<String, String>> {
    let mut map = HashMap::new();
    for name in parsed
        .rows
        .iter()
        .filter_map(|row| row.category_name.as_ref())
        .filter(|name| !name.trim().is_empty())
    {
        if map.contains_key(name) {
            continue;
        }
        let id = tx
            .query_row(
                "SELECT id FROM categories WHERE parent_id IS NULL AND name = ?1",
                params![name],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_else(new_id);
        tx.execute(
            "INSERT OR IGNORE INTO categories (id, name, enabled, sort_order)
             VALUES (?1, ?2, 1, 999)",
            params![id, name],
        )?;
        map.insert(name.clone(), id);
    }
    Ok(map)
}

fn ensure_units(
    tx: &Transaction<'_>,
    parsed: &ParsedWorkbook,
) -> AppResult<HashMap<String, String>> {
    let mut map = HashMap::new();
    for name in parsed
        .rows
        .iter()
        .filter_map(|row| row.unit_name.as_ref())
        .filter(|name| !name.trim().is_empty())
    {
        if map.contains_key(name) {
            continue;
        }
        let id = tx
            .query_row(
                "SELECT id FROM units WHERE name = ?1",
                params![name],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_else(new_id);
        tx.execute(
            "INSERT OR IGNORE INTO units (id, name, enabled, sort_order)
             VALUES (?1, ?2, 1, 999)",
            params![id, name],
        )?;
        map.insert(name.clone(), id);
    }
    Ok(map)
}

fn ensure_departments(
    tx: &Transaction<'_>,
    parsed: &ParsedWorkbook,
) -> AppResult<HashMap<String, String>> {
    let mut map = HashMap::new();
    for (index, (name, _, _)) in DEPARTMENT_COLUMNS.iter().enumerate() {
        let id = tx
            .query_row(
                "SELECT id FROM departments WHERE name = ?1",
                params![name],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_else(new_id);
        tx.execute(
            "INSERT OR IGNORE INTO departments (id, code, name, enabled, sort_order, remark)
             VALUES (?1, ?2, ?3, 1, ?4, 'Excel 迁移默认部门')",
            params![id, format!("D{:03}", index + 1), name, index as i64 + 1],
        )?;
        map.insert((*name).to_string(), id);
    }
    for name in parsed
        .rows
        .iter()
        .flat_map(|row| &row.outbound_lines)
        .map(|line| line.department_name.trim())
        .filter(|name| !name.is_empty())
    {
        if map.contains_key(name) {
            continue;
        }
        let id = tx
            .query_row(
                "SELECT id FROM departments WHERE name = ?1",
                params![name],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_else(new_id);
        tx.execute(
            "INSERT OR IGNORE INTO departments (id, code, name, enabled, sort_order, remark)
             VALUES (?1, ?2, ?3, 1, ?4, 'Excel 通用模板导入部门')",
            params![
                id,
                format!("DIMP{:03}", map.len() + 1),
                name,
                map.len() as i64 + 1
            ],
        )?;
        map.insert(name.to_string(), id);
    }
    Ok(map)
}

fn rows_by_month(parsed: &ParsedWorkbook) -> BTreeMap<String, Vec<&ParsedRow>> {
    let mut map: BTreeMap<String, Vec<&ParsedRow>> = BTreeMap::new();
    for row in &parsed.rows {
        map.entry(row.month.clone()).or_default().push(row);
    }
    map
}

fn rows_by_department<'a>(
    rows: &'a [&'a ParsedRow],
) -> BTreeMap<String, Vec<(&'a ParsedRow, &'a ParsedDepartmentIssue)>> {
    let mut map: BTreeMap<String, Vec<(&ParsedRow, &ParsedDepartmentIssue)>> = BTreeMap::new();
    for row in rows {
        for issue in &row.outbound_lines {
            if issue.quantity > 0.0 {
                map.entry(issue.department_name.clone())
                    .or_default()
                    .push((*row, issue));
            }
        }
    }
    map
}

fn existing_item_names(conn: &Connection) -> AppResult<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT name FROM master_items")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut names = HashSet::new();
    for row in rows {
        names.insert(normalized_name(&row?));
    }
    Ok(names)
}

fn find_item_id(tx: &Transaction<'_>, name: &str) -> AppResult<Option<String>> {
    Ok(tx
        .query_row(
            "SELECT id FROM master_items WHERE name = ?1",
            params![name],
            |row| row.get::<_, String>(0),
        )
        .optional()?)
}

fn next_item_code(tx: &Transaction<'_>) -> AppResult<String> {
    let count: i64 = tx.query_row("SELECT COUNT(*) FROM master_items", [], |row| row.get(0))?;
    Ok(format!("HC-{:04}", count + 1))
}

fn next_document_no(
    tx: &Transaction<'_>,
    document_type: &str,
    business_date: &str,
) -> AppResult<String> {
    let prefix = match document_type {
        "inbound" => "IN",
        "outbound" => "OUT",
        other => return Err(AppError::Validation(format!("不支持的单据类型：{other}"))),
    };
    let date_part = business_date.replace('-', "");
    let like = format!("{prefix}-{date_part}-%");
    let count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM stock_documents WHERE document_no LIKE ?1",
        params![like],
        |row| row.get(0),
    )?;
    Ok(format!("{prefix}-{date_part}-{:04}", count + 1))
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

fn parse_sheet_month(name: &str) -> Option<String> {
    let trimmed = name.trim();
    let parts = trimmed.split('.').collect::<Vec<_>>();
    if parts.len() != 2 {
        return None;
    }
    let year = parts[0].parse::<i32>().ok()?;
    let month = parts[1].parse::<u32>().ok()?;
    if !(1..=12).contains(&month) {
        return None;
    }
    Some(format!("{year:04}-{month:02}"))
}

fn parse_month_cell(data: Option<&Data>) -> Option<String> {
    match data {
        Some(Data::DateTime(value)) => {
            let (year, month, _, _, _, _, _) = value.to_ymd_hms_milli();
            Some(format!("{year:04}-{month:02}"))
        }
        Some(Data::String(value)) => parse_month_text(value),
        Some(Data::Float(value)) => parse_month_text(&trim_number(*value)),
        Some(Data::Int(value)) => parse_month_text(&value.to_string()),
        _ => None,
    }
}

fn parse_month_text(value: &str) -> Option<String> {
    let cleaned = value
        .trim()
        .replace('年', "-")
        .replace('月', "")
        .replace('.', "-")
        .replace('/', "-");
    let parts = cleaned
        .split('-')
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    let year = parts[0].parse::<i32>().ok()?;
    let month = parts[1].parse::<u32>().ok()?;
    if !(1..=12).contains(&month) {
        return None;
    }
    Some(format!("{year:04}-{month:02}"))
}

fn month_start_date(month: &str) -> AppResult<String> {
    NaiveDate::parse_from_str(&format!("{month}-01"), "%Y-%m-%d")
        .map(|date| date.to_string())
        .map_err(|_| AppError::Validation(format!("无法解析月份：{month}")))
}

#[allow(clippy::too_many_arguments)]
fn validate_import_row(
    sheet_name: &str,
    excel_row: usize,
    month: &str,
    item_name: &str,
    unit_name: Option<&str>,
    opening_quantity: f64,
    opening_price: f64,
    opening_amount: Option<f64>,
    inbound_quantity: f64,
    inbound_price: f64,
    inbound_amount: Option<f64>,
    outbound_quantity: Option<f64>,
    outbound_amount: Option<f64>,
    seen_rows: &mut HashSet<(String, String, String)>,
    warnings: &mut Vec<ImportMessage>,
    errors: &mut Vec<ImportMessage>,
) {
    if unit_name.map(str::trim).unwrap_or_default().is_empty() {
        errors.push(message(
            "error",
            sheet_name,
            excel_row,
            None,
            "单位为空，请补充单位后再导入",
        ));
    }

    let item_key = (
        month.to_string(),
        normalized_name(item_name),
        normalized_name(unit_name.unwrap_or("")),
    );
    if !seen_rows.insert(item_key) {
        warnings.push(message(
            "warning",
            sheet_name,
            excel_row,
            None,
            "检测到同一月份、物品和单位重复行，导入时会合并统计，请复核",
        ));
    }

    if opening_quantity < 0.0 || inbound_quantity < 0.0 || outbound_quantity.unwrap_or(0.0) < 0.0 {
        errors.push(message(
            "error",
            sheet_name,
            excel_row,
            None,
            "数量异常：期初、入库或出库数量不能为负数",
        ));
    }

    if opening_quantity > 0.0 && opening_price <= 0.0 {
        errors.push(message(
            "error",
            sheet_name,
            excel_row,
            None,
            "单价异常：期初数量大于 0 时期初单价必须大于 0",
        ));
    }
    if inbound_quantity > 0.0 && inbound_price <= 0.0 {
        errors.push(message(
            "error",
            sheet_name,
            excel_row,
            None,
            "单价异常：入库数量大于 0 时入库单价必须大于 0",
        ));
    }

    warn_amount_mismatch(
        sheet_name,
        excel_row,
        "期初金额",
        opening_quantity,
        opening_price,
        opening_amount,
        warnings,
    );
    warn_amount_mismatch(
        sheet_name,
        excel_row,
        "入库金额",
        inbound_quantity,
        inbound_price,
        inbound_amount,
        warnings,
    );
    if let (Some(quantity), Some(amount)) = (outbound_quantity, outbound_amount) {
        let price = if quantity.abs() < f64::EPSILON {
            0.0
        } else {
            amount / quantity
        };
        warn_amount_mismatch(
            sheet_name,
            excel_row,
            "出库金额",
            quantity,
            price,
            Some(amount),
            warnings,
        );
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

fn detect_unmapped_legacy_department_columns(
    sheet_name: &str,
    excel_row: usize,
    row: &[Data],
    warnings: &mut Vec<ImportMessage>,
) {
    let known_columns = DEPARTMENT_COLUMNS
        .iter()
        .flat_map(|(_, quantity_col, amount_col)| [Some(*quantity_col), *amount_col])
        .flatten()
        .collect::<HashSet<_>>();
    for index in 11..row.len() {
        if known_columns.contains(&index) {
            continue;
        }
        if number_at(row, index).unwrap_or(0.0) > 0.0 {
            warnings.push(message(
                "warning",
                sheet_name,
                excel_row,
                Some(&column_label(index)),
                "检测到固定部门列之外的数量/金额，可能存在未识别的部门列",
            ));
        }
    }
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

fn should_skip_row(item_name: &str) -> bool {
    let trimmed = item_name.trim();
    trimmed.is_empty()
        || trimmed.contains("名称")
        || trimmed.contains("品名")
        || trimmed.contains("合计")
        || trimmed.contains("总计")
        || trimmed.contains("月报")
}

fn text_at(row: &[Data], index: usize) -> String {
    row.get(index).map(data_to_text).unwrap_or_default()
}

fn number_at(row: &[Data], index: usize) -> Option<f64> {
    match row.get(index) {
        Some(Data::Float(value)) => Some(*value),
        Some(Data::Int(value)) => Some(*value as f64),
        Some(Data::String(value)) => parse_number_text(value),
        Some(Data::DateTime(value)) => Some(value.as_f64()),
        _ => None,
    }
    .filter(|value| value.is_finite())
}

fn data_to_text(data: &Data) -> String {
    match data {
        Data::String(value) => value.trim().to_string(),
        Data::Float(value) => trim_number(*value),
        Data::Int(value) => value.to_string(),
        Data::Bool(value) => value.to_string(),
        Data::DateTime(value) => trim_number(value.as_f64()),
        _ => String::new(),
    }
}

fn is_empty_cell(data: &Data) -> bool {
    match data {
        Data::Empty => true,
        Data::String(value) => value.trim().is_empty(),
        _ => false,
    }
}

fn normalized_header(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '_' && *ch != '-' && *ch != '/')
        .collect::<String>()
        .to_lowercase()
}

fn header_matches(value: &str, aliases: &[&str]) -> bool {
    let normalized = normalized_header(value);
    aliases
        .iter()
        .any(|alias| normalized == normalized_header(alias))
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

fn parse_number_text(value: &str) -> Option<f64> {
    let cleaned = value
        .trim()
        .replace(',', "")
        .replace('，', "")
        .replace('￥', "");
    if cleaned.is_empty() || cleaned == "-" {
        return None;
    }
    cleaned.parse::<f64>().ok()
}

fn trim_number(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}

fn normalized_name(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn empty_to_none(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn positive_price(quantity: f64, amount: f64) -> Option<f64> {
    if quantity > 0.0 && amount > 0.0 {
        Some(round_price(amount / quantity))
    } else {
        None
    }
}

fn first_positive(a: f64, b: f64, c: f64) -> f64 {
    [a, b, c]
        .into_iter()
        .find(|value| *value > 0.0)
        .unwrap_or(0.0)
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

fn round_price(value: f64) -> f64 {
    (value * 10000.0).round() / 10000.0
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Debug, Clone)]
struct ParsedWorkbook {
    rows: Vec<ParsedRow>,
    warnings: Vec<ImportMessage>,
    errors: Vec<ImportMessage>,
    sheet_count: usize,
}

#[derive(Debug, Clone)]
struct ParsedRow {
    sheet_name: String,
    row_number: usize,
    month: String,
    item_name: String,
    category_name: Option<String>,
    unit_name: Option<String>,
    spec: Option<String>,
    opening_quantity: f64,
    opening_price: f64,
    opening_amount: f64,
    inbound_quantity: f64,
    inbound_price: f64,
    inbound_amount: f64,
    average_price: f64,
    outbound_lines: Vec<ParsedDepartmentIssue>,
}

#[derive(Debug, Clone)]
struct ParsedDepartmentIssue {
    department_name: String,
    quantity: f64,
    amount: f64,
}

struct ImportItemAccumulator {
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
    opening_quantity: f64,
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

#[derive(Debug, Clone)]
struct TemplateHeader {
    row_index: usize,
    month: Option<usize>,
    business_date: Option<usize>,
    item_name: usize,
    category_name: Option<usize>,
    unit_name: Option<usize>,
    spec: Option<usize>,
    department_name: Option<usize>,
    opening_quantity: Option<usize>,
    opening_price: Option<usize>,
    opening_amount: Option<usize>,
    inbound_quantity: Option<usize>,
    inbound_price: Option<usize>,
    inbound_amount: Option<usize>,
    outbound_quantity: Option<usize>,
    outbound_amount: Option<usize>,
    average_price: Option<usize>,
}

impl TemplateHeader {
    fn from_range(range: &calamine::Range<Data>) -> Option<Self> {
        for (row_index, row) in range.rows().take(10).enumerate() {
            let mut header = TemplateHeader {
                row_index,
                month: None,
                business_date: None,
                item_name: usize::MAX,
                category_name: None,
                unit_name: None,
                spec: None,
                department_name: None,
                opening_quantity: None,
                opening_price: None,
                opening_amount: None,
                inbound_quantity: None,
                inbound_price: None,
                inbound_amount: None,
                outbound_quantity: None,
                outbound_amount: None,
                average_price: None,
            };

            for (index, cell) in row.iter().enumerate() {
                let label = data_to_text(cell);
                if label.trim().is_empty() {
                    continue;
                }
                if header_matches(&label, &["月份", "账期", "期间", "month"]) {
                    header.month = Some(index);
                } else if header_matches(&label, &["日期", "业务日期", "单据日期", "date"])
                {
                    header.business_date = Some(index);
                } else if header_matches(&label, &["物品名称", "物品", "品名", "名称", "itemname"])
                {
                    header.item_name = index;
                } else if header_matches(&label, &["分类", "物品分类", "大类", "category"])
                {
                    header.category_name = Some(index);
                } else if header_matches(&label, &["单位", "unit"]) {
                    header.unit_name = Some(index);
                } else if header_matches(&label, &["规格", "型号", "spec"]) {
                    header.spec = Some(index);
                } else if header_matches(&label, &["部门", "出库部门", "领用部门", "department"])
                {
                    header.department_name = Some(index);
                } else if header_matches(&label, &["期初数量", "期初", "openingquantity"]) {
                    header.opening_quantity = Some(index);
                } else if header_matches(&label, &["期初单价", "openingprice"]) {
                    header.opening_price = Some(index);
                } else if header_matches(&label, &["期初金额", "openingamount"]) {
                    header.opening_amount = Some(index);
                } else if header_matches(&label, &["入库数量", "采购数量", "inboundquantity"])
                {
                    header.inbound_quantity = Some(index);
                } else if header_matches(&label, &["入库单价", "采购单价", "inboundprice"])
                {
                    header.inbound_price = Some(index);
                } else if header_matches(&label, &["入库金额", "采购金额", "inboundamount"])
                {
                    header.inbound_amount = Some(index);
                } else if header_matches(&label, &["出库数量", "领用数量", "outboundquantity"])
                {
                    header.outbound_quantity = Some(index);
                } else if header_matches(&label, &["出库金额", "领用金额", "outboundamount"])
                {
                    header.outbound_amount = Some(index);
                } else if header_matches(&label, &["平均单价", "移动均价", "均价", "averageprice"])
                {
                    header.average_price = Some(index);
                }
            }

            if header.item_name != usize::MAX
                && (header.month.is_some() || header.business_date.is_some())
                && (header.opening_quantity.is_some()
                    || header.inbound_quantity.is_some()
                    || header.outbound_quantity.is_some())
            {
                return Some(header);
            }
        }
        None
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

    #[test]
    fn preview_legacy_workbook_extracts_month_items_and_department_issues() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.xlsx");
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name("2026.1").unwrap();
        worksheet.write_string(0, 0, "名称").unwrap();
        worksheet.write_string(1, 0, "一次性牙刷").unwrap();
        worksheet.write_string(1, 1, "客耗").unwrap();
        worksheet.write_number(1, 2, 10.0).unwrap();
        worksheet.write_number(1, 3, 1.2).unwrap();
        worksheet.write_number(1, 4, 12.0).unwrap();
        worksheet.write_string(1, 5, "支").unwrap();
        worksheet.write_string(1, 6, "普通").unwrap();
        worksheet.write_number(1, 7, 20.0).unwrap();
        worksheet.write_number(1, 8, 1.1).unwrap();
        worksheet.write_number(1, 9, 22.0).unwrap();
        worksheet.write_number(1, 10, 1.13).unwrap();
        worksheet.write_number(1, 17, 5.0).unwrap();
        worksheet.write_number(1, 18, 5.65).unwrap();
        workbook.save(&path).unwrap();

        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();

        let parsed = parse_legacy_workbook(path.to_str().unwrap()).unwrap();
        let preview = build_preview(&conn, path.to_str().unwrap(), parsed).unwrap();
        assert_eq!(preview.sheet_count, 1);
        assert_eq!(preview.item_count, 1);
        assert_eq!(preview.opening_quantity, 10.0);
        assert_eq!(preview.inbound_quantity, 20.0);
        assert_eq!(preview.outbound_quantity, 5.0);
        assert_eq!(preview.months[0].month, "2026-01");
    }

    #[test]
    fn preview_template_workbook_extracts_generic_columns() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("template.xlsx");
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name("通用模板").unwrap();
        for (index, label) in [
            "月份",
            "物品名称",
            "分类",
            "规格",
            "单位",
            "期初数量",
            "期初单价",
            "入库数量",
            "入库单价",
            "出库部门",
            "出库数量",
            "出库金额",
        ]
        .iter()
        .enumerate()
        {
            worksheet.write_string(0, index as u16, *label).unwrap();
        }
        worksheet.write_string(1, 0, "2026-06").unwrap();
        worksheet.write_string(1, 1, "洗发水").unwrap();
        worksheet.write_string(1, 2, "客耗").unwrap();
        worksheet.write_string(1, 3, "300ml").unwrap();
        worksheet.write_string(1, 4, "瓶").unwrap();
        worksheet.write_number(1, 5, 8.0).unwrap();
        worksheet.write_number(1, 6, 12.5).unwrap();
        worksheet.write_number(1, 7, 10.0).unwrap();
        worksheet.write_number(1, 8, 11.0).unwrap();
        worksheet.write_string(1, 9, "客房").unwrap();
        worksheet.write_number(1, 10, 3.0).unwrap();
        worksheet.write_number(1, 11, 36.0).unwrap();
        workbook.save(&path).unwrap();

        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();

        let parsed = parse_legacy_workbook(path.to_str().unwrap()).unwrap();
        let preview = build_preview(&conn, path.to_str().unwrap(), parsed).unwrap();
        assert_eq!(preview.sheet_count, 1);
        assert_eq!(preview.row_count, 1);
        assert_eq!(preview.item_count, 1);
        assert_eq!(preview.months[0].month, "2026-06");
        assert_eq!(preview.opening_quantity, 8.0);
        assert_eq!(preview.inbound_quantity, 10.0);
        assert_eq!(preview.outbound_quantity, 3.0);
        assert_eq!(preview.outbound_amount, 36.0);
        assert!(preview.errors.is_empty());
    }

    #[test]
    fn preview_template_workbook_reports_validation_messages() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("template-invalid.xlsx");
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name("通用模板").unwrap();
        for (index, label) in [
            "月份",
            "物品名称",
            "单位",
            "入库数量",
            "入库单价",
            "入库金额",
            "出库部门",
            "出库数量",
            "出库金额",
        ]
        .iter()
        .enumerate()
        {
            worksheet.write_string(0, index as u16, *label).unwrap();
        }
        worksheet.write_string(1, 0, "2026-06").unwrap();
        worksheet.write_string(1, 1, "问题物品").unwrap();
        worksheet.write_string(1, 2, "").unwrap();
        worksheet.write_number(1, 3, -2.0).unwrap();
        worksheet.write_number(1, 4, 0.0).unwrap();
        worksheet.write_number(1, 5, 9.0).unwrap();
        worksheet.write_string(1, 6, "客房").unwrap();
        worksheet.write_number(1, 7, 1.0).unwrap();
        worksheet.write_number(1, 8, 3.0).unwrap();
        worksheet.write_string(2, 0, "2026-06").unwrap();
        worksheet.write_string(2, 1, "问题物品").unwrap();
        worksheet.write_string(2, 2, "").unwrap();
        worksheet.write_string(3, 0, "2026-06").unwrap();
        worksheet.write_string(3, 1, "单价异常物品").unwrap();
        worksheet.write_string(3, 2, "件").unwrap();
        worksheet.write_number(3, 3, 2.0).unwrap();
        worksheet.write_number(3, 4, 0.0).unwrap();
        worksheet.write_number(3, 5, 9.0).unwrap();
        worksheet.write_string(4, 0, "2026-06").unwrap();
        worksheet.write_string(4, 1, "金额不平物品").unwrap();
        worksheet.write_string(4, 2, "件").unwrap();
        worksheet.write_number(4, 3, 2.0).unwrap();
        worksheet.write_number(4, 4, 3.0).unwrap();
        worksheet.write_number(4, 5, 9.0).unwrap();
        workbook.save(&path).unwrap();

        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();

        let parsed = parse_legacy_workbook(path.to_str().unwrap()).unwrap();
        let preview = build_preview(&conn, path.to_str().unwrap(), parsed).unwrap();
        let error_text = preview
            .errors
            .iter()
            .map(|message| message.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let warning_text = preview
            .warnings
            .iter()
            .map(|message| message.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(error_text.contains("单位为空"));
        assert!(error_text.contains("数量异常"));
        assert!(error_text.contains("单价异常"));
        assert!(warning_text.contains("入库金额不平"));
        assert!(warning_text.contains("重复行"));
    }

    #[test]
    fn preview_legacy_workbook_reports_blank_item_name() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy-blank-item.xlsx");
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name("2026.1").unwrap();
        worksheet.write_string(0, 0, "名称").unwrap();
        worksheet.write_string(1, 5, "支").unwrap();
        worksheet.write_number(1, 7, 10.0).unwrap();
        workbook.save(&path).unwrap();

        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();

        let parsed = parse_legacy_workbook(path.to_str().unwrap()).unwrap();
        let preview = build_preview(&conn, path.to_str().unwrap(), parsed).unwrap();
        assert!(preview
            .errors
            .iter()
            .any(|message| message.message.contains("物品名称不能为空")));
        assert_eq!(preview.row_count, 0);
    }

    #[test]
    fn detect_formula_errors_reports_cell_errors() {
        let mut errors = Vec::new();
        detect_formula_errors(
            "通用模板",
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
        assert_eq!(errors[0].sheet, "通用模板");
        assert_eq!(errors[0].row, 3);
        assert_eq!(errors[0].column.as_deref(), Some("C"));
        assert!(errors[0].message.contains("公式错误"));
    }

    #[test]
    fn preview_legacy_workbook_warns_unmapped_department_columns() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy-extra-department.xlsx");
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name("2026.1").unwrap();
        worksheet.write_string(0, 0, "名称").unwrap();
        worksheet.write_string(1, 0, "一次性拖鞋").unwrap();
        worksheet.write_string(1, 5, "双").unwrap();
        worksheet.write_number(1, 7, 10.0).unwrap();
        worksheet.write_number(1, 8, 2.0).unwrap();
        worksheet.write_number(1, 27, 5.0).unwrap();
        workbook.save(&path).unwrap();

        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();

        let parsed = parse_legacy_workbook(path.to_str().unwrap()).unwrap();
        let preview = build_preview(&conn, path.to_str().unwrap(), parsed).unwrap();
        assert!(preview
            .warnings
            .iter()
            .any(|message| message.message.contains("未识别的部门列")));
    }

    #[test]
    fn import_template_workbook_creates_missing_department_and_movements() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("template-import.xlsx");
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name("导入模板").unwrap();
        for (index, label) in [
            "月份",
            "物品名称",
            "单位",
            "入库数量",
            "入库单价",
            "出库部门",
            "出库数量",
            "出库金额",
        ]
        .iter()
        .enumerate()
        {
            worksheet.write_string(0, index as u16, *label).unwrap();
        }
        worksheet.write_string(1, 0, "2026.06").unwrap();
        worksheet.write_string(1, 1, "定制拖鞋").unwrap();
        worksheet.write_string(1, 2, "双").unwrap();
        worksheet.write_number(1, 3, 20.0).unwrap();
        worksheet.write_number(1, 4, 2.5).unwrap();
        worksheet.write_string(1, 5, "新部门").unwrap();
        worksheet.write_number(1, 6, 4.0).unwrap();
        worksheet.write_number(1, 7, 10.0).unwrap();
        workbook.save(&path).unwrap();

        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();
        let parsed = parse_legacy_workbook(path.to_str().unwrap()).unwrap();
        let preview = build_preview(&conn, path.to_str().unwrap(), parsed.clone()).unwrap();

        let mut conn = conn;
        let mut tx = conn.transaction().unwrap();
        let result = import_parsed_workbook(
            &mut tx,
            path.to_str().unwrap(),
            "job-template",
            &parsed,
            &preview,
            ImportMode::Full,
        )
        .unwrap();
        tx.commit().unwrap();

        assert_eq!(result.imported_items, 1);
        assert_eq!(result.movement_count, 2);
        let department_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM departments WHERE name = '新部门'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(department_count, 1);
        let outbound_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stock_movements m
                 JOIN departments d ON d.id = m.department_id
                 WHERE d.name = '新部门' AND m.direction = 'out'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(outbound_count, 1);
    }

    #[test]
    fn import_items_only_creates_items_without_documents_or_movements() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("items-only.xlsx");
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name("2026.6").unwrap();
        worksheet.write_string(0, 0, "名称").unwrap();
        worksheet.write_string(1, 0, "一次性梳子").unwrap();
        worksheet.write_string(1, 1, "客耗").unwrap();
        worksheet.write_number(1, 2, 12.0).unwrap();
        worksheet.write_number(1, 3, 0.8).unwrap();
        worksheet.write_string(1, 5, "把").unwrap();
        worksheet.write_number(1, 7, 6.0).unwrap();
        worksheet.write_number(1, 8, 0.7).unwrap();
        worksheet.write_number(1, 17, 3.0).unwrap();
        workbook.save(&path).unwrap();

        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();
        let parsed = parse_legacy_workbook(path.to_str().unwrap()).unwrap();
        let preview = build_preview(&conn, path.to_str().unwrap(), parsed.clone()).unwrap();

        let mut conn = conn;
        let mut tx = conn.transaction().unwrap();
        let result = import_parsed_workbook(
            &mut tx,
            path.to_str().unwrap(),
            "job-items-only",
            &parsed,
            &preview,
            ImportMode::ItemsOnly,
        )
        .unwrap();
        tx.commit().unwrap();

        assert_eq!(result.imported_items, 1);
        assert_eq!(result.document_count, 0);
        assert_eq!(result.movement_count, 0);
        let item_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM master_items WHERE name = '一次性梳子'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(item_count, 1);
        let movement_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM stock_movements", [], |row| row.get(0))
            .unwrap();
        assert_eq!(movement_count, 0);
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
            sheet_count: 1,
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
    fn preview_existing_hotel_workbook_when_available() {
        let Ok(workbook_path) = std::env::var("ASTER_TEST_LEGACY_WORKBOOK") else {
            return;
        };
        let path = Path::new(&workbook_path);
        if !path.exists() {
            return;
        }

        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();

        let parsed = parse_legacy_workbook(path.to_str().unwrap()).unwrap();
        let preview = build_preview(&conn, path.to_str().unwrap(), parsed).unwrap();
        assert!(preview.sheet_count >= 1);
        assert!(preview.row_count >= 1);
        assert!(preview.item_count >= 1);
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
        let state = AppState {
            db: Db::initialize(&paths).unwrap(),
            paths,
            session: std::sync::Arc::new(std::sync::Mutex::new(Some(
                crate::domain::users::CurrentUser {
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
                },
            ))),
            host_service: std::sync::Arc::new(std::sync::Mutex::new(
                crate::services::host_service::HostServiceRuntime::default(),
            )),
        };

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
        let worksheet = workbook.add_worksheet();
        worksheet.set_name("导入模板").unwrap();
        for (index, label) in ["月份", "物品名称", "单位", "入库数量", "入库单价"]
            .iter()
            .enumerate()
        {
            worksheet.write_string(0, index as u16, *label).unwrap();
        }
        worksheet.write_string(1, 0, "2026-06").unwrap();
        worksheet.write_string(1, 1, "缺单位物品").unwrap();
        worksheet.write_string(1, 2, "").unwrap();
        worksheet.write_number(1, 3, 2.0).unwrap();
        worksheet.write_number(1, 4, 3.0).unwrap();
        workbook.save(&path).unwrap();

        let paths = AppPaths {
            data_dir: dir.path().join("app-data"),
            database_path: dir.path().join("app-data").join("aster.sqlite"),
            backup_dir: dir.path().join("backups"),
            export_dir: dir.path().join("exports"),
            import_report_dir: dir.path().join("import-reports"),
        };
        fs::create_dir_all(&paths.data_dir).unwrap();
        fs::create_dir_all(&paths.backup_dir).unwrap();
        fs::create_dir_all(&paths.export_dir).unwrap();
        fs::create_dir_all(&paths.import_report_dir).unwrap();
        let state = AppState {
            db: Db::initialize(&paths).unwrap(),
            paths,
            session: std::sync::Arc::new(std::sync::Mutex::new(Some(
                crate::domain::users::CurrentUser {
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
                },
            ))),
            host_service: std::sync::Arc::new(std::sync::Mutex::new(
                crate::services::host_service::HostServiceRuntime::default(),
            )),
        };

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
                let movement_count: i64 =
                    conn.query_row("SELECT COUNT(*) FROM stock_movements", [], |row| row.get(0))?;
                assert_eq!(backup_count, 0);
                assert_eq!(item_count, 0);
                assert_eq!(movement_count, 0);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn run_excel_import_creates_before_import_backup_before_writes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("valid-import.xlsx");
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name("导入模板").unwrap();
        for (index, label) in ["月份", "物品名称", "单位", "入库数量", "入库单价"]
            .iter()
            .enumerate()
        {
            worksheet.write_string(0, index as u16, *label).unwrap();
        }
        worksheet.write_string(1, 0, "2026-06").unwrap();
        worksheet.write_string(1, 1, "导入前备份物品").unwrap();
        worksheet.write_string(1, 2, "件").unwrap();
        worksheet.write_number(1, 3, 2.0).unwrap();
        worksheet.write_number(1, 4, 3.0).unwrap();
        workbook.save(&path).unwrap();

        let paths = AppPaths {
            data_dir: dir.path().join("app-data"),
            database_path: dir.path().join("app-data").join("aster.sqlite"),
            backup_dir: dir.path().join("backups"),
            export_dir: dir.path().join("exports"),
            import_report_dir: dir.path().join("import-reports"),
        };
        fs::create_dir_all(&paths.data_dir).unwrap();
        fs::create_dir_all(&paths.backup_dir).unwrap();
        fs::create_dir_all(&paths.export_dir).unwrap();
        fs::create_dir_all(&paths.import_report_dir).unwrap();
        let state = AppState {
            db: Db::initialize(&paths).unwrap(),
            paths,
            session: std::sync::Arc::new(std::sync::Mutex::new(Some(
                crate::domain::users::CurrentUser {
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
                },
            ))),
            host_service: std::sync::Arc::new(std::sync::Mutex::new(
                crate::services::host_service::HostServiceRuntime::default(),
            )),
        };

        let result = run_excel_import(
            &state,
            RunImportRequest {
                path: path.display().to_string(),
                mode: None,
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
                    "SELECT COUNT(*) FROM master_items WHERE name = '导入前备份物品'",
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
                "SELECT COUNT(*) FROM master_items WHERE name = '导入前备份物品'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(item_count_in_backup, 0);
    }

    #[test]
    fn preview_excel_import_rejects_client_mode_before_parsing_workbook() {
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
}
