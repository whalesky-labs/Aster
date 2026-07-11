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
