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

include!("execution.rs");
include!("master_data.rs");
include!("workbook.rs");
include!("models.rs");
