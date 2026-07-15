fn write_summary_sheets(
    workbook: &mut Workbook,
    bundle: &ReportBundle,
    header: &Format,
    money: &Format,
    number: &Format,
) -> AppResult<()> {
    let sheet = workbook.add_worksheet();
    sheet.set_name("月度进销存").map_err(map_xlsx)?;
    write_headers(
        sheet,
        header,
        &[
            "编码",
            "物品",
            "规格",
            "单位",
            "入库数量",
            "入库金额",
            "出库数量",
            "出库金额",
            "结存数量",
            "结存金额",
        ],
    )?;
    for (idx, row) in bundle.monthly_inventory.iter().enumerate() {
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
            .write_number_with_format(r, 4, row.inbound_quantity, number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 5, row.inbound_amount, money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 6, row.outbound_quantity, number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 7, row.outbound_amount, money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 8, row.ending_quantity, number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 9, row.ending_amount, money)
            .map_err(map_xlsx)?;
    }
    sheet.autofit();

    let sheet = workbook.add_worksheet();
    sheet.set_name("部门领用汇总").map_err(map_xlsx)?;
    write_headers(sheet, header, &["部门", "数量", "金额"])?;
    for (idx, row) in bundle.department_summary.iter().enumerate() {
        let r = (idx + 1) as u32;
        sheet
            .write_string(r, 0, &row.department_name)
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
    sheet.set_name("部门领用明细").map_err(map_xlsx)?;
    write_headers(
        sheet,
        header,
        &[
            "日期",
            "部门",
            "编码",
            "物品",
            "规格",
            "单位",
            "数量",
            "成本单价",
            "成本金额",
            "单号",
            "用途",
            "备注",
        ],
    )?;
    for (idx, row) in bundle.department_details.iter().enumerate() {
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

    let sheet = workbook.add_worksheet();
    sheet.set_name("销售毛利").map_err(map_xlsx)?;
    write_headers(
        sheet,
        header,
        &[
            "日期",
            "编码",
            "物品",
            "规格",
            "单位",
            "数量",
            "销售单价",
            "销售金额",
            "成本单价",
            "成本金额",
            "毛利",
            "毛利率",
            "是否亏损",
            "单号",
            "用途",
            "备注",
        ],
    )?;
    for (idx, row) in bundle.sales_profit.iter().enumerate() {
        let r = (idx + 1) as u32;
        sheet
            .write_string(r, 0, &row.movement_date)
            .map_err(map_xlsx)?;
        sheet.write_string(r, 1, &row.item_code).map_err(map_xlsx)?;
        sheet.write_string(r, 2, &row.item_name).map_err(map_xlsx)?;
        sheet
            .write_string(r, 3, row.spec.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 4, row.unit_name.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 5, row.quantity, number)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 6, row.sale_unit_price, money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 7, row.sale_amount, money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 8, row.cost_unit_price, money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 9, row.cost_amount, money)
            .map_err(map_xlsx)?;
        sheet
            .write_number_with_format(r, 10, row.gross_profit, money)
            .map_err(map_xlsx)?;
        if let Some(gross_margin) = row.gross_margin {
            sheet
                .write_number_with_format(r, 11, gross_margin, number)
                .map_err(map_xlsx)?;
        }
        sheet
            .write_string(r, 12, if row.negative_profit { "是" } else { "否" })
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 13, row.document_no.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 14, row.purpose.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
        sheet
            .write_string(r, 15, row.remark.as_deref().unwrap_or(""))
            .map_err(map_xlsx)?;
    }
    sheet.autofit();

    Ok(())
}
