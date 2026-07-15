fn write_detail_sheets(
    workbook: &mut Workbook,
    bundle: &ReportBundle,
    header: &Format,
    money: &Format,
    number: &Format,
) -> AppResult<()> {
    let sheet = workbook.add_worksheet();
    sheet.set_name("分类消耗统计").map_err(map_xlsx)?;
    write_headers(sheet, header, &["分类", "数量", "金额"])?;
    for (idx, row) in bundle.category_consumption.iter().enumerate() {
        let r = (idx + 1) as u32;
        sheet
            .write_string(r, 0, &row.category_name)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 1, row.quantity, number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 2, row.amount, money)
            .map_err(map_xlsx)?;
    }
    sheet.autofit();

    let sheet = workbook.add_worksheet();
    sheet.set_name("物品消耗排行").map_err(map_xlsx)?;
    write_headers(
        sheet,
        header,
        &["编码", "物品", "规格", "单位", "数量", "金额"],
    )?;
    for (idx, row) in bundle.item_consumption_ranking.iter().enumerate() {
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
    }
    sheet.autofit();

    let sheet = workbook.add_worksheet();
    sheet.set_name("入库明细").map_err(map_xlsx)?;
    write_headers(
        sheet,
        header,
        &[
            "日期",
            "供应商",
            "编码",
            "物品",
            "规格",
            "单位",
            "数量",
            "单价",
            "金额",
            "单号",
            "备注",
        ],
    )?;
    for (idx, row) in bundle.inbound_details.iter().enumerate() {
        let r = (idx + 1) as u32;
        sheet
            .write_string(r, 0, &row.movement_date)
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 1, &row.supplier_name)
            .map_err(map_xlsx)?;
        sheet.write_string(r, 2, &row.item_code).map_err(map_xlsx)?;
        sheet.write_string(r, 3, &row.item_name).map_err(map_xlsx)?;
        sheet
            .write_string(r, 4, row.spec.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 5, row.unit_name.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 6, row.quantity, number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 7, row.unit_price, money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 8, row.amount, money)
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 9, row.document_no.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 10, row.remark.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
    }
    sheet.autofit();

    let sheet = workbook.add_worksheet();
    sheet.set_name("出库明细").map_err(map_xlsx)?;
    write_headers(
        sheet,
        header,
        &[
            "日期", "部门", "编码", "物品", "规格", "单位", "数量", "单价", "金额", "单号", "用途",
            "备注",
        ],
    )?;
    for (idx, row) in bundle.outbound_details.iter().enumerate() {
        let r = (idx + 1) as u32;
        sheet
            .write_string(r, 0, &row.movement_date)
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 1, &row.department_name)
            .map_err(map_xlsx)?;
        sheet.write_string(r, 2, &row.item_code).map_err(map_xlsx)?;
        sheet.write_string(r, 3, &row.item_name).map_err(map_xlsx)?;
        sheet
            .write_string(r, 4, row.spec.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 5, row.unit_name.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 6, row.quantity, number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 7, row.unit_price, money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 8, row.amount, money)
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 9, row.document_no.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 10, row.purpose.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 11, row.remark.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
    }
    sheet.autofit();

    Ok(())
}
