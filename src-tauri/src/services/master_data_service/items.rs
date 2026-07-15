pub fn list_items(
    state: &AppState,
    search: Option<String>,
    supplier_id: Option<String>,
) -> AppResult<Vec<Item>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_items(state, search, supplier_id);
    }
    state
        .db
        .with_conn(|conn| master_data_repository::list_items(conn, search, supplier_id))
}

pub fn list_items_page(
    state: &AppState,
    search: Option<String>,
    supplier_id: Option<String>,
    cursor: Option<String>,
) -> AppResult<crate::domain::pagination::Page<Item>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_items_page(
            state,
            search,
            supplier_id,
            cursor,
        );
    }
    state.db.with_conn(|conn| {
        master_data_repository::list_items_page(
            conn,
            search,
            supplier_id,
            cursor.as_deref(),
        )
    })
}

pub fn export_items(
    state: &AppState,
    search: Option<String>,
    supplier_id: Option<String>,
) -> AppResult<ExportItemsResult> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    let export_dir = crate::services::status_service::effective_export_dir(state)?;
    fs::create_dir_all(&export_dir)?;
    let file_name = format!(
        "Aster-物品档案-{}.xlsx",
        Local::now().format("%Y%m%d%H%M%S")
    );
    let path = export_dir.join(file_name);
    let items = if runtime_mode(state)? == RuntimeMode::Client {
        crate::services::host_service::remote_list_items(state, search, supplier_id)?
    } else {
        state
            .db
            .with_conn(|conn| master_data_repository::list_items(conn, search, supplier_id))?
    };
    write_items_workbook(&path, &items)?;
    if runtime_mode(state)? != RuntimeMode::Client {
        state.db.with_conn(|conn| {
            write_audit(
                conn,
                "export_items",
                "item",
                "items",
                &format!("导出物品档案：{} 种", items.len()),
            )
        })?;
    }
    Ok(ExportItemsResult {
        path: path.display().to_string(),
    })
}

pub fn save_item(state: &AppState, request: SaveItemRequest) -> AppResult<Item> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    validate_item(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_save_item(state, request);
    }
    state.db.with_conn(|conn| {
        let item = master_data_repository::save_item(conn, request)?;
        write_audit(conn, "save_item", "item", &item.id, &item.name)?;
        Ok(item)
    })
}

fn write_items_workbook(path: &std::path::Path, items: &[Item]) -> AppResult<()> {
    let mut workbook = Workbook::new();
    let header = Format::new().set_bold().set_background_color("#DDEBF7");
    let money = Format::new().set_num_format("#,##0.00");
    let number = Format::new().set_num_format("#,##0.00");
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("物品档案").map_err(xlsx_error)?;
    let headers = [
        "物品编码",
        "条码",
        "物品名称",
        "分类",
        "规格",
        "单位",
        "参考进价",
        "参考售价",
        "供应商",
        "预警库存",
        "状态",
        "备注",
        "创建时间",
        "更新时间",
    ];
    for (col, title) in headers.iter().enumerate() {
        worksheet
            .write_string_with_format(0, col as u16, *title, &header)
            .map_err(xlsx_error)?;
    }
    for (index, item) in items.iter().enumerate() {
        let row = (index + 1) as u32;
        worksheet
            .write_string(row, 0, &item.code)
            .map_err(xlsx_error)?;
        write_optional_string(worksheet, row, 1, item.barcode.as_deref())?;
        worksheet
            .write_string(row, 2, &item.name)
            .map_err(xlsx_error)?;
        write_optional_string(worksheet, row, 3, item.category_name.as_deref())?;
        write_optional_string(worksheet, row, 4, item.spec.as_deref())?;
        write_optional_string(worksheet, row, 5, item.unit_name.as_deref())?;
        worksheet
            .write_number_with_format(row, 6, item.default_price, &money)
            .map_err(xlsx_error)?;
        worksheet
            .write_number_with_format(row, 7, item.sale_price, &money)
            .map_err(xlsx_error)?;
        write_optional_string(worksheet, row, 8, item.supplier_name.as_deref())?;
        worksheet
            .write_number_with_format(row, 9, item.warning_quantity, &number)
            .map_err(xlsx_error)?;
        worksheet
            .write_string(row, 10, if item.enabled { "启用" } else { "停用" })
            .map_err(xlsx_error)?;
        write_optional_string(worksheet, row, 11, item.remark.as_deref())?;
        worksheet
            .write_string(row, 12, &item.created_at)
            .map_err(xlsx_error)?;
        worksheet
            .write_string(row, 13, &item.updated_at)
            .map_err(xlsx_error)?;
    }
    for (col, width) in [16, 18, 24, 16, 18, 10, 12, 12, 18, 12, 10, 28, 22, 22]
        .iter()
        .enumerate()
    {
        worksheet
            .set_column_width(col as u16, *width)
            .map_err(xlsx_error)?;
    }
    workbook
        .save(path)
        .map_err(|error| AppError::Validation(format!("物品档案导出失败：{error}")))
}

fn write_optional_string(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    col: u16,
    value: Option<&str>,
) -> AppResult<()> {
    worksheet
        .write_string(row, col, value.unwrap_or(""))
        .map(|_| ())
        .map_err(xlsx_error)
}

fn xlsx_error(error: XlsxError) -> AppError {
    AppError::Validation(format!("物品档案导出失败：{error}"))
}
