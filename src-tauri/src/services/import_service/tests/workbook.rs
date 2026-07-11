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
