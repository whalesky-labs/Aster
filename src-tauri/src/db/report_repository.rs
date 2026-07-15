use rusqlite::{params, Connection};

use crate::db::report_filters::ReportFilters;
use crate::domain::reports::{
    CategoryConsumptionRow, DepartmentIssueDetailRow, DepartmentIssueSummaryRow, InboundDetailRow,
    ItemConsumptionRow, MonthlyInventoryRow, ReportBundle, ReportBundlePage, ReportQuery,
    SalesProfitRow, StockBalanceReportRow, StockWarningRow, StocktakeDifferenceReportRow,
};
use crate::error::AppResult;

pub fn get_report_bundle(conn: &Connection, query: &ReportQuery) -> AppResult<ReportBundle> {
    let filters = ReportFilters::from(query);
    Ok(ReportBundle {
        month: filters.month.to_string(),
        monthly_inventory: monthly_inventory(conn, &filters, None)?,
        department_summary: department_summary(conn, &filters, None)?,
        department_details: department_details(conn, &filters, None)?,
        category_consumption: category_consumption(conn, &filters, None)?,
        item_consumption_ranking: item_consumption_ranking(conn, &filters, None)?,
        inbound_details: inbound_details(conn, &filters, None)?,
        outbound_details: department_details(conn, &filters, None)?,
        sales_profit: sales_profit(conn, &filters, None)?,
        stock_balances: stock_balances(conn, &filters, None)?,
        stock_warnings: stock_warnings(conn, &filters, None)?,
        stocktake_differences: stocktake_differences(conn, &filters, None)?,
    })
}

pub fn get_report_bundle_page(
    conn: &Connection,
    query: &ReportQuery,
    section: &str,
    cursor: Option<&str>,
) -> AppResult<ReportBundlePage> {
    crate::db::paginated_report_repository::get_page(conn, query, section, cursor)
}

fn report_sql(base: &str, offset: Option<i64>) -> String {
    crate::db::pagination::query(base, offset)
}

pub(crate) fn monthly_inventory(
    conn: &Connection,
    filters: &ReportFilters<'_>,
    offset: Option<i64>,
) -> AppResult<Vec<MonthlyInventoryRow>> {
    let mut stmt = conn.prepare(&report_sql(
        "SELECT i.id, i.code, i.name, i.spec, u.name,
                COALESCE(SUM(CASE WHEN m.direction = 'in' THEN m.quantity ELSE 0 END), 0) AS inbound_qty,
                COALESCE(SUM(CASE WHEN m.direction = 'in' THEN m.amount ELSE 0 END), 0) AS inbound_amount,
                COALESCE(SUM(CASE WHEN m.direction = 'out' THEN m.quantity ELSE 0 END), 0) AS outbound_qty,
                COALESCE(SUM(CASE WHEN m.direction = 'out' THEN m.amount ELSE 0 END), 0) AS outbound_amount,
                COALESCE(b.quantity, 0) AS ending_qty,
                COALESCE(b.amount, 0) AS ending_amount
         FROM master_items i
         LEFT JOIN units u ON u.id = i.unit_id
         LEFT JOIN stock_balances b ON b.item_id = i.id
         LEFT JOIN stock_movements m ON m.item_id = i.id
           AND m.movement_date >= ?1 || '-01'
           AND m.movement_date < date(?1 || '-01', '+1 month')
           AND (?2 IS NULL OR m.movement_date >= ?2)
           AND (?3 IS NULL OR m.movement_date <= ?3)
         WHERE i.enabled = 1
           AND (?4 IS NULL OR i.category_id = ?4)
           AND (?5 IS NULL OR i.id = ?5)
           AND (?6 IS NULL OR i.supplier_id = ?6)
         GROUP BY i.id
         ORDER BY i.code ASC, i.id ASC",
        offset,
    ))?;
    let rows = stmt.query_map(
        params![
            filters.month,
            filters.start_date.as_deref(),
            filters.end_date.as_deref(),
            filters.category_id,
            filters.item_id,
            filters.supplier_id
        ],
        |row| {
            Ok(MonthlyInventoryRow {
                item_id: row.get(0)?,
                item_code: row.get(1)?,
                item_name: row.get(2)?,
                spec: row.get(3)?,
                unit_name: row.get(4)?,
                inbound_quantity: row.get(5)?,
                inbound_amount: row.get(6)?,
                outbound_quantity: row.get(7)?,
                outbound_amount: row.get(8)?,
                ending_quantity: row.get(9)?,
                ending_amount: row.get(10)?,
            })
        },
    )?;
    collect_rows(rows)
}

pub(crate) fn department_summary(
    conn: &Connection,
    filters: &ReportFilters<'_>,
    offset: Option<i64>,
) -> AppResult<Vec<DepartmentIssueSummaryRow>> {
    let mut stmt = conn.prepare(&report_sql(
        "SELECT d.id, d.name,
                COALESCE(SUM(m.quantity), 0),
                COALESCE(SUM(m.amount), 0)
         FROM departments d
         LEFT JOIN stock_movements m ON m.department_id = d.id
           AND m.direction = 'out'
           AND m.movement_date >= ?1 || '-01'
           AND m.movement_date < date(?1 || '-01', '+1 month')
           AND (?2 IS NULL OR m.movement_date >= ?2)
           AND (?3 IS NULL OR m.movement_date <= ?3)
         LEFT JOIN master_items i ON i.id = m.item_id
         WHERE d.enabled = 1
           AND (?4 IS NULL OR d.id = ?4)
           AND (?5 IS NULL OR i.category_id = ?5)
           AND (?6 IS NULL OR i.id = ?6)
         GROUP BY d.id
         ORDER BY d.sort_order ASC, d.name ASC, d.id ASC",
        offset,
    ))?;
    let rows = stmt.query_map(
        params![
            filters.month,
            filters.start_date.as_deref(),
            filters.end_date.as_deref(),
            filters.department_id,
            filters.category_id,
            filters.item_id
        ],
        |row| {
            Ok(DepartmentIssueSummaryRow {
                department_id: row.get(0)?,
                department_name: row.get(1)?,
                quantity: row.get(2)?,
                amount: row.get(3)?,
            })
        },
    )?;
    collect_rows(rows)
}

pub(crate) fn department_details(
    conn: &Connection,
    filters: &ReportFilters<'_>,
    offset: Option<i64>,
) -> AppResult<Vec<DepartmentIssueDetailRow>> {
    let mut stmt = conn.prepare(&report_sql(
        "SELECT m.movement_date,
                CASE
                  WHEN doc.outbound_kind = 'guest_sale' THEN '酒店客人'
                  ELSE COALESCE(m.department_name, d.name)
                END,
                doc.outbound_kind, i.code, i.name, i.spec, u.name,
                m.quantity, m.unit_price, m.amount,
                l.sale_unit_price, l.sale_amount,
                COALESCE(l.cost_unit_price, m.unit_price),
                COALESCE(l.cost_amount, m.amount),
                CASE
                  WHEN l.sale_amount IS NULL THEN NULL
                  ELSE COALESCE(l.sale_amount, 0) - COALESCE(l.cost_amount, m.amount, 0)
                END,
                CASE
                  WHEN COALESCE(l.sale_amount, 0) <= 0 THEN NULL
                  ELSE (COALESCE(l.sale_amount, 0) - COALESCE(l.cost_amount, m.amount, 0)) / l.sale_amount
                END,
                doc.document_no,
                doc.purpose, m.remark
         FROM stock_movements m
         JOIN master_items i ON i.id = m.item_id
         LEFT JOIN units u ON u.id = i.unit_id
         LEFT JOIN departments d ON d.id = m.department_id
         LEFT JOIN stock_documents doc ON doc.id = m.document_id
         LEFT JOIN stock_document_lines l ON l.id = m.document_line_id
         WHERE m.direction = 'out'
           AND m.movement_date >= ?1 || '-01'
           AND m.movement_date < date(?1 || '-01', '+1 month')
           AND (?2 IS NULL OR m.movement_date >= ?2)
           AND (?3 IS NULL OR m.movement_date <= ?3)
           AND (?4 IS NULL OR m.department_id = ?4)
           AND (?5 IS NULL OR i.category_id = ?5)
           AND (?6 IS NULL OR i.id = ?6)
         ORDER BY m.movement_date ASC, d.sort_order ASC, i.code ASC, m.created_at ASC, m.id ASC",
        offset,
    ))?;
    let rows = stmt.query_map(
        params![
            filters.month,
            filters.start_date.as_deref(),
            filters.end_date.as_deref(),
            filters.department_id,
            filters.category_id,
            filters.item_id
        ],
        |row| {
            Ok(DepartmentIssueDetailRow {
                movement_date: row.get(0)?,
                department_name: row
                    .get::<_, Option<String>>(1)?
                    .unwrap_or_else(|| "未指定".to_string()),
                outbound_kind: row.get(2)?,
                item_code: row.get(3)?,
                item_name: row.get(4)?,
                spec: row.get(5)?,
                unit_name: row.get(6)?,
                quantity: row.get(7)?,
                unit_price: row.get(8)?,
                amount: row.get(9)?,
                sale_unit_price: row.get(10)?,
                sale_amount: row.get(11)?,
                cost_unit_price: row.get(12)?,
                cost_amount: row.get(13)?,
                gross_profit: row.get(14)?,
                gross_margin: row.get(15)?,
                document_no: row.get(16)?,
                purpose: row.get(17)?,
                remark: row.get(18)?,
            })
        },
    )?;
    collect_rows(rows)
}

pub(crate) fn sales_profit(
    conn: &Connection,
    filters: &ReportFilters<'_>,
    offset: Option<i64>,
) -> AppResult<Vec<SalesProfitRow>> {
    let mut stmt = conn.prepare(&report_sql(
        "SELECT d.business_date, i.code, i.name, i.spec, u.name,
                l.quantity,
                COALESCE(l.sale_unit_price, l.unit_price, 0),
                COALESCE(l.sale_amount, l.amount, 0),
                COALESCE(l.cost_unit_price, 0),
                COALESCE(l.cost_amount, 0),
                COALESCE(l.sale_amount, l.amount, 0) - COALESCE(l.cost_amount, 0),
                CASE
                  WHEN COALESCE(l.sale_amount, l.amount, 0) <= 0 THEN NULL
                  ELSE (COALESCE(l.sale_amount, l.amount, 0) - COALESCE(l.cost_amount, 0))
                       / COALESCE(l.sale_amount, l.amount, 0)
                END,
                d.document_no, d.purpose, l.remark
         FROM stock_document_lines l
         JOIN stock_documents d ON d.id = l.document_id
         JOIN master_items i ON i.id = l.item_id
         LEFT JOIN units u ON u.id = i.unit_id
         WHERE d.document_type = 'outbound'
           AND d.outbound_kind = 'guest_sale'
           AND d.status = 'confirmed'
           AND d.business_date >= ?1 || '-01'
           AND d.business_date < date(?1 || '-01', '+1 month')
           AND (?2 IS NULL OR d.business_date >= ?2)
           AND (?3 IS NULL OR d.business_date <= ?3)
           AND (?4 IS NULL OR i.category_id = ?4)
           AND (?5 IS NULL OR i.id = ?5)
         ORDER BY d.business_date ASC, d.document_no ASC, i.code ASC, l.id ASC",
        offset,
    ))?;
    let rows = stmt.query_map(
        params![
            filters.month,
            filters.start_date.as_deref(),
            filters.end_date.as_deref(),
            filters.category_id,
            filters.item_id
        ],
        |row| {
            let sale_amount: f64 = row.get(7)?;
            let cost_amount: f64 = row.get(9)?;
            let gross_profit: f64 = row.get(10)?;
            Ok(SalesProfitRow {
                movement_date: row.get(0)?,
                item_code: row.get(1)?,
                item_name: row.get(2)?,
                spec: row.get(3)?,
                unit_name: row.get(4)?,
                quantity: row.get(5)?,
                sale_unit_price: row.get(6)?,
                sale_amount,
                cost_unit_price: row.get(8)?,
                cost_amount,
                gross_profit,
                gross_margin: row.get(11)?,
                negative_profit: gross_profit < 0.0,
                document_no: row.get(12)?,
                purpose: row.get(13)?,
                remark: row.get(14)?,
            })
        },
    )?;
    collect_rows(rows)
}

include!("report_repository/consumption.rs");

#[cfg(test)]
#[path = "report_repository/tests.rs"]
mod tests;
