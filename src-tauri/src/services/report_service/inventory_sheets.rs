fn write_inventory_control_sheets(
    workbook: &mut Workbook,
    bundle: &ReportBundle,
    header: &Format,
    money: &Format,
    number: &Format,
) -> AppResult<()> {
    let sheet = workbook.add_worksheet();
    sheet.set_name("库存余额").map_err(map_xlsx)?;
    write_headers(
        sheet,
        header,
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
            .write_number_with_format(r, 4, row.quantity, number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 5, row.amount, money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 6, row.average_price, money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 7, row.last_inbound_price, money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 8, row.warning_quantity, number)
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
        header,
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
            .write_number_with_format(r, 4, row.quantity, number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 5, row.warning_quantity, number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 6, row.shortage_quantity, number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 7, row.amount, money)
            .map_err(map_xlsx)?;
    }
    sheet.autofit();

    let sheet = workbook.add_worksheet();
    sheet.set_name("盘点差异").map_err(map_xlsx)?;
    write_headers(
        sheet,
        header,
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
            .write_number_with_format(r, 8, row.book_quantity, number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 9, row.counted_quantity, number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 10, row.difference_quantity, number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 11, row.average_price, money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 12, row.difference_amount, money)
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 13, row.remark.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
    }
    sheet.autofit();

    Ok(())
}
