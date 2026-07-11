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
