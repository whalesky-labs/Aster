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
