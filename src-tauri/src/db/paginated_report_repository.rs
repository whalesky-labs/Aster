use rusqlite::Connection;

use crate::db::{pagination, report_filters::ReportFilters, report_repository};
use crate::domain::reports::{ReportBundle, ReportBundlePage, ReportQuery};
use crate::error::{AppError, AppResult};

pub fn get_page(
    conn: &Connection,
    query: &ReportQuery,
    section: &str,
    cursor: Option<&str>,
) -> AppResult<ReportBundlePage> {
    let scope = scope(query, section)?;
    let offset = pagination::offset(conn, &scope, cursor)?;
    let filters = ReportFilters::from(query);
    let mut bundle = empty_bundle(&query.month);
    let next_cursor = match section {
        "monthlyInventory" => assign(
            conn,
            &scope,
            offset,
            report_repository::monthly_inventory(conn, &filters, Some(offset))?,
            |items| bundle.monthly_inventory = items,
        )?,
        "departmentSummary" => assign(
            conn,
            &scope,
            offset,
            report_repository::department_summary(conn, &filters, Some(offset))?,
            |items| bundle.department_summary = items,
        )?,
        "departmentDetails" => assign(
            conn,
            &scope,
            offset,
            report_repository::department_details(conn, &filters, Some(offset))?,
            |items| bundle.department_details = items,
        )?,
        "categoryConsumption" => assign(
            conn,
            &scope,
            offset,
            report_repository::category_consumption(conn, &filters, Some(offset))?,
            |items| bundle.category_consumption = items,
        )?,
        "itemConsumptionRanking" => assign(
            conn,
            &scope,
            offset,
            report_repository::item_consumption_ranking(conn, &filters, Some(offset))?,
            |items| bundle.item_consumption_ranking = items,
        )?,
        "inboundDetails" => assign(
            conn,
            &scope,
            offset,
            report_repository::inbound_details(conn, &filters, Some(offset))?,
            |items| bundle.inbound_details = items,
        )?,
        "outboundDetails" => assign(
            conn,
            &scope,
            offset,
            report_repository::department_details(conn, &filters, Some(offset))?,
            |items| bundle.outbound_details = items,
        )?,
        "salesProfit" => assign(
            conn,
            &scope,
            offset,
            report_repository::sales_profit(conn, &filters, Some(offset))?,
            |items| bundle.sales_profit = items,
        )?,
        "stockBalances" => assign(
            conn,
            &scope,
            offset,
            report_repository::stock_balances(conn, &filters, Some(offset))?,
            |items| bundle.stock_balances = items,
        )?,
        "stockWarnings" => assign(
            conn,
            &scope,
            offset,
            report_repository::stock_warnings(conn, &filters, Some(offset))?,
            |items| bundle.stock_warnings = items,
        )?,
        "stocktakeDifferences" => assign(
            conn,
            &scope,
            offset,
            report_repository::stocktake_differences(conn, &filters, Some(offset))?,
            |items| bundle.stocktake_differences = items,
        )?,
        _ => return Err(AppError::Validation("未知报表分页 section".to_string())),
    };
    Ok(ReportBundlePage {
        section: section.to_string(),
        bundle,
        next_cursor,
    })
}

fn assign<T>(
    conn: &Connection,
    scope: &str,
    offset: i64,
    items: Vec<T>,
    set: impl FnOnce(Vec<T>),
) -> AppResult<Option<String>> {
    let page = pagination::page(conn, scope, offset, items)?;
    set(page.items);
    Ok(page.next_cursor)
}

fn empty_bundle(month: &str) -> ReportBundle {
    ReportBundle {
        month: month.to_string(),
        monthly_inventory: Vec::new(),
        department_summary: Vec::new(),
        department_details: Vec::new(),
        category_consumption: Vec::new(),
        item_consumption_ranking: Vec::new(),
        inbound_details: Vec::new(),
        outbound_details: Vec::new(),
        sales_profit: Vec::new(),
        stock_balances: Vec::new(),
        stock_warnings: Vec::new(),
        stocktake_differences: Vec::new(),
    }
}

fn scope(query: &ReportQuery, section: &str) -> AppResult<String> {
    let query = serde_json::to_string(query)
        .map_err(|error| AppError::Validation(format!("报表查询序列化失败：{error}")))?;
    Ok(format!("report:{section}:{query}"))
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::db::migrations;

    use super::*;

    #[test]
    fn section_paginates_over_five_hundred_rows() {
        let conn = Connection::open_in_memory().expect("database");
        migrations::run(&conn).expect("migrations");
        for index in 0..501 {
            conn.execute(
                "INSERT INTO master_items (id, code, name, unit_id, default_price, enabled)
                 VALUES (?1, ?2, ?3, 'unit-piece', 1, 1)",
                rusqlite::params![
                    format!("item-page-{index:03}"),
                    format!("PAGE-{index:03}"),
                    format!("分页物品 {index:03}")
                ],
            )
            .expect("item");
        }
        let query = ReportQuery {
            month: "2026-06".to_string(),
            start_date: None,
            end_date: None,
            department_id: None,
            category_id: None,
            item_id: None,
            supplier_id: None,
        };
        let first = get_page(&conn, &query, "stockBalances", None).expect("first");
        assert_eq!(first.bundle.stock_balances.len(), 500);
        let cursor = first.next_cursor.expect("cursor");
        let second = get_page(&conn, &query, "stockBalances", Some(&cursor)).expect("second");
        assert_eq!(second.bundle.stock_balances.len(), 1);
        assert_eq!(second.bundle.stock_balances[0].item_code, "PAGE-500");
        assert!(get_page(&conn, &query, "stockWarnings", Some(&cursor)).is_err());
    }
}
