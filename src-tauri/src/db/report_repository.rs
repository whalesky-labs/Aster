use rusqlite::{params, Connection};

use crate::domain::reports::{
    CategoryConsumptionRow, DepartmentIssueDetailRow, DepartmentIssueSummaryRow, InboundDetailRow,
    ItemConsumptionRow, MonthlyInventoryRow, ReportBundle, ReportQuery, SalesProfitRow,
    StockBalanceReportRow, StockWarningRow, StocktakeDifferenceReportRow,
};
use crate::error::AppResult;

struct ReportFilters<'a> {
    month: &'a str,
    start_date: Option<String>,
    end_date: Option<String>,
    department_id: Option<&'a str>,
    category_id: Option<&'a str>,
    item_id: Option<&'a str>,
    supplier_id: Option<&'a str>,
}

impl<'a> From<&'a ReportQuery> for ReportFilters<'a> {
    fn from(query: &'a ReportQuery) -> Self {
        Self {
            month: &query.month,
            start_date: query.start_date.as_deref().map(start_datetime_bound),
            end_date: query.end_date.as_deref().map(end_datetime_bound),
            department_id: query.department_id.as_deref(),
            category_id: query.category_id.as_deref(),
            item_id: query.item_id.as_deref(),
            supplier_id: query.supplier_id.as_deref(),
        }
    }
}

fn start_datetime_bound(value: &str) -> String {
    if value.len() == 10 {
        format!("{value} 00:00:00")
    } else {
        value.to_string()
    }
}

fn end_datetime_bound(value: &str) -> String {
    if value.len() == 10 {
        format!("{value} 23:59:59")
    } else {
        value.to_string()
    }
}

pub fn get_report_bundle(conn: &Connection, query: &ReportQuery) -> AppResult<ReportBundle> {
    let filters = ReportFilters::from(query);
    Ok(ReportBundle {
        month: filters.month.to_string(),
        monthly_inventory: monthly_inventory(conn, &filters)?,
        department_summary: department_summary(conn, &filters)?,
        department_details: department_details(conn, &filters)?,
        category_consumption: category_consumption(conn, &filters)?,
        item_consumption_ranking: item_consumption_ranking(conn, &filters)?,
        inbound_details: inbound_details(conn, &filters)?,
        outbound_details: department_details(conn, &filters)?,
        sales_profit: sales_profit(conn, &filters)?,
        stock_balances: stock_balances(conn, &filters)?,
        stock_warnings: stock_warnings(conn, &filters)?,
        stocktake_differences: stocktake_differences(conn, &filters)?,
    })
}

fn monthly_inventory(
    conn: &Connection,
    filters: &ReportFilters<'_>,
) -> AppResult<Vec<MonthlyInventoryRow>> {
    let mut stmt = conn.prepare(
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
           AND strftime('%Y-%m', m.movement_date) = ?1
           AND (?2 IS NULL OR m.movement_date >= ?2)
           AND (?3 IS NULL OR m.movement_date <= ?3)
         WHERE i.enabled = 1
           AND (?4 IS NULL OR i.category_id = ?4)
           AND (?5 IS NULL OR i.id = ?5)
           AND (?6 IS NULL OR i.supplier_id = ?6)
         GROUP BY i.id
         ORDER BY i.code ASC",
    )?;
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

fn department_summary(
    conn: &Connection,
    filters: &ReportFilters<'_>,
) -> AppResult<Vec<DepartmentIssueSummaryRow>> {
    let mut stmt = conn.prepare(
        "SELECT d.id, d.name,
                COALESCE(SUM(m.quantity), 0),
                COALESCE(SUM(m.amount), 0)
         FROM departments d
         LEFT JOIN stock_movements m ON m.department_id = d.id
           AND m.direction = 'out'
           AND strftime('%Y-%m', m.movement_date) = ?1
           AND (?2 IS NULL OR m.movement_date >= ?2)
           AND (?3 IS NULL OR m.movement_date <= ?3)
         LEFT JOIN master_items i ON i.id = m.item_id
         WHERE d.enabled = 1
           AND (?4 IS NULL OR d.id = ?4)
           AND (?5 IS NULL OR i.category_id = ?5)
           AND (?6 IS NULL OR i.id = ?6)
         GROUP BY d.id
         ORDER BY d.sort_order ASC, d.name ASC",
    )?;
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

fn department_details(
    conn: &Connection,
    filters: &ReportFilters<'_>,
) -> AppResult<Vec<DepartmentIssueDetailRow>> {
    let mut stmt = conn.prepare(
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
           AND strftime('%Y-%m', m.movement_date) = ?1
           AND (?2 IS NULL OR m.movement_date >= ?2)
           AND (?3 IS NULL OR m.movement_date <= ?3)
           AND (?4 IS NULL OR m.department_id = ?4)
           AND (?5 IS NULL OR i.category_id = ?5)
           AND (?6 IS NULL OR i.id = ?6)
         ORDER BY m.movement_date ASC, d.sort_order ASC, i.code ASC",
    )?;
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

fn sales_profit(conn: &Connection, filters: &ReportFilters<'_>) -> AppResult<Vec<SalesProfitRow>> {
    let mut stmt = conn.prepare(
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
           AND strftime('%Y-%m', d.business_date) = ?1
           AND (?2 IS NULL OR d.business_date >= ?2)
           AND (?3 IS NULL OR d.business_date <= ?3)
           AND (?4 IS NULL OR i.category_id = ?4)
           AND (?5 IS NULL OR i.id = ?5)
         ORDER BY d.business_date ASC, d.document_no ASC, i.code ASC",
    )?;
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

fn category_consumption(
    conn: &Connection,
    filters: &ReportFilters<'_>,
) -> AppResult<Vec<CategoryConsumptionRow>> {
    let mut stmt = conn.prepare(
        "SELECT c.id, COALESCE(c.name, '未分类'),
                COALESCE(SUM(m.quantity), 0),
                COALESCE(SUM(m.amount), 0)
         FROM stock_movements m
         JOIN master_items i ON i.id = m.item_id
         LEFT JOIN categories c ON c.id = i.category_id
         WHERE m.direction = 'out'
           AND strftime('%Y-%m', m.movement_date) = ?1
           AND (?2 IS NULL OR m.movement_date >= ?2)
           AND (?3 IS NULL OR m.movement_date <= ?3)
           AND (?4 IS NULL OR m.department_id = ?4)
           AND (?5 IS NULL OR i.category_id = ?5)
           AND (?6 IS NULL OR i.id = ?6)
         GROUP BY c.id, c.name
         ORDER BY COALESCE(SUM(m.amount), 0) DESC, COALESCE(c.name, '未分类') ASC",
    )?;
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
            Ok(CategoryConsumptionRow {
                category_id: row.get(0)?,
                category_name: row.get(1)?,
                quantity: row.get(2)?,
                amount: row.get(3)?,
            })
        },
    )?;
    collect_rows(rows)
}

fn item_consumption_ranking(
    conn: &Connection,
    filters: &ReportFilters<'_>,
) -> AppResult<Vec<ItemConsumptionRow>> {
    let mut stmt = conn.prepare(
        "SELECT i.id, i.code, i.name, i.spec, u.name,
                COALESCE(SUM(m.quantity), 0),
                COALESCE(SUM(m.amount), 0)
         FROM stock_movements m
         JOIN master_items i ON i.id = m.item_id
         LEFT JOIN units u ON u.id = i.unit_id
         WHERE m.direction = 'out'
           AND strftime('%Y-%m', m.movement_date) = ?1
           AND (?2 IS NULL OR m.movement_date >= ?2)
           AND (?3 IS NULL OR m.movement_date <= ?3)
           AND (?4 IS NULL OR m.department_id = ?4)
           AND (?5 IS NULL OR i.category_id = ?5)
           AND (?6 IS NULL OR i.id = ?6)
         GROUP BY i.id
         ORDER BY COALESCE(SUM(m.amount), 0) DESC, COALESCE(SUM(m.quantity), 0) DESC, i.code ASC",
    )?;
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
            Ok(ItemConsumptionRow {
                item_id: row.get(0)?,
                item_code: row.get(1)?,
                item_name: row.get(2)?,
                spec: row.get(3)?,
                unit_name: row.get(4)?,
                quantity: row.get(5)?,
                amount: row.get(6)?,
            })
        },
    )?;
    collect_rows(rows)
}

fn inbound_details(
    conn: &Connection,
    filters: &ReportFilters<'_>,
) -> AppResult<Vec<InboundDetailRow>> {
    let mut stmt = conn.prepare(
        "SELECT m.movement_date, COALESCE(m.supplier_name, s.name), i.code, i.name, i.spec, u.name,
                m.quantity, m.unit_price, m.amount, doc.document_no, m.remark
         FROM stock_movements m
         JOIN master_items i ON i.id = m.item_id
         LEFT JOIN units u ON u.id = i.unit_id
         LEFT JOIN suppliers s ON s.id = m.supplier_id
         LEFT JOIN stock_documents doc ON doc.id = m.document_id
         WHERE m.direction = 'in'
           AND strftime('%Y-%m', m.movement_date) = ?1
           AND (?2 IS NULL OR m.movement_date >= ?2)
           AND (?3 IS NULL OR m.movement_date <= ?3)
           AND (?4 IS NULL OR i.category_id = ?4)
           AND (?5 IS NULL OR i.id = ?5)
           AND (?6 IS NULL OR m.supplier_id = ?6)
         ORDER BY m.movement_date ASC, s.name ASC, i.code ASC",
    )?;
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
            Ok(InboundDetailRow {
                movement_date: row.get(0)?,
                supplier_name: row
                    .get::<_, Option<String>>(1)?
                    .unwrap_or_else(|| "未指定".to_string()),
                item_code: row.get(2)?,
                item_name: row.get(3)?,
                spec: row.get(4)?,
                unit_name: row.get(5)?,
                quantity: row.get(6)?,
                unit_price: row.get(7)?,
                amount: row.get(8)?,
                document_no: row.get(9)?,
                remark: row.get(10)?,
            })
        },
    )?;
    collect_rows(rows)
}

fn stock_warnings(
    conn: &Connection,
    filters: &ReportFilters<'_>,
) -> AppResult<Vec<StockWarningRow>> {
    let mut stmt = conn.prepare(
        "SELECT i.id, i.code, i.name, i.spec, u.name,
                COALESCE(b.quantity, 0),
                i.warning_quantity,
                MAX(i.warning_quantity - COALESCE(b.quantity, 0), 0),
                COALESCE(b.amount, 0)
         FROM master_items i
         LEFT JOIN units u ON u.id = i.unit_id
         LEFT JOIN stock_balances b ON b.item_id = i.id
         WHERE i.enabled = 1
           AND COALESCE(b.quantity, 0) >= 0
           AND COALESCE(b.quantity, 0) <= i.warning_quantity
           AND (?1 IS NULL OR i.category_id = ?1)
           AND (?2 IS NULL OR i.id = ?2)
           AND (?3 IS NULL OR i.supplier_id = ?3)
         ORDER BY (i.warning_quantity - COALESCE(b.quantity, 0)) DESC, i.code ASC",
    )?;
    let rows = stmt.query_map(
        params![filters.category_id, filters.item_id, filters.supplier_id],
        |row| {
            Ok(StockWarningRow {
                item_id: row.get(0)?,
                item_code: row.get(1)?,
                item_name: row.get(2)?,
                spec: row.get(3)?,
                unit_name: row.get(4)?,
                quantity: row.get(5)?,
                warning_quantity: row.get(6)?,
                shortage_quantity: row.get(7)?,
                amount: row.get(8)?,
            })
        },
    )?;
    collect_rows(rows)
}

fn stock_balances(
    conn: &Connection,
    filters: &ReportFilters<'_>,
) -> AppResult<Vec<StockBalanceReportRow>> {
    let mut stmt = conn.prepare(
        "SELECT i.id, i.code, i.name, i.spec, u.name,
                COALESCE(b.quantity, 0),
                COALESCE(b.amount, 0),
                COALESCE(b.average_price, 0),
                COALESCE(b.last_inbound_price, 0),
                i.warning_quantity
         FROM master_items i
         LEFT JOIN units u ON u.id = i.unit_id
         LEFT JOIN stock_balances b ON b.item_id = i.id
         WHERE i.enabled = 1
           AND (?1 IS NULL OR i.category_id = ?1)
           AND (?2 IS NULL OR i.id = ?2)
           AND (?3 IS NULL OR i.supplier_id = ?3)
         ORDER BY i.code ASC",
    )?;
    let rows = stmt.query_map(
        params![filters.category_id, filters.item_id, filters.supplier_id],
        |row| {
            let quantity: f64 = row.get(5)?;
            let warning_quantity: f64 = row.get(9)?;
            let stock_status = if quantity < 0.0 {
                "negative"
            } else if quantity <= warning_quantity {
                "low"
            } else {
                "normal"
            };
            Ok(StockBalanceReportRow {
                item_id: row.get(0)?,
                item_code: row.get(1)?,
                item_name: row.get(2)?,
                spec: row.get(3)?,
                unit_name: row.get(4)?,
                quantity,
                amount: row.get(6)?,
                average_price: row.get(7)?,
                last_inbound_price: row.get(8)?,
                warning_quantity,
                stock_status: stock_status.to_string(),
            })
        },
    )?;
    collect_rows(rows)
}

fn stocktake_differences(
    conn: &Connection,
    filters: &ReportFilters<'_>,
) -> AppResult<Vec<StocktakeDifferenceReportRow>> {
    let mut stmt = conn.prepare(
        "SELECT d.business_date, d.document_no, st.scope_type, st.status,
                i.code, i.name, i.spec, u.name,
                l.book_quantity,
                COALESCE(l.counted_quantity, 0),
                l.difference_quantity,
                COALESCE(b.average_price, i.default_price, 0),
                l.difference_quantity * COALESCE(b.average_price, i.default_price, 0),
                l.remark
         FROM stocktake_documents st
         JOIN stock_documents d ON d.id = st.document_id
         JOIN stocktake_lines l ON l.stocktake_id = st.id
         JOIN master_items i ON i.id = l.item_id
         LEFT JOIN units u ON u.id = i.unit_id
         LEFT JOIN stock_balances b ON b.item_id = i.id
         WHERE st.status = 'confirmed'
           AND d.status = 'confirmed'
           AND strftime('%Y-%m', d.business_date) = ?1
           AND (?2 IS NULL OR d.business_date >= ?2)
           AND (?3 IS NULL OR d.business_date <= ?3)
           AND ABS(l.difference_quantity) > 0.000001
           AND (?4 IS NULL OR i.category_id = ?4)
           AND (?5 IS NULL OR i.id = ?5)
         ORDER BY d.business_date ASC, d.document_no ASC, i.code ASC",
    )?;
    let rows = stmt.query_map(
        params![
            filters.month,
            filters.start_date.as_deref(),
            filters.end_date.as_deref(),
            filters.category_id,
            filters.item_id
        ],
        |row| {
            Ok(StocktakeDifferenceReportRow {
                business_date: row.get(0)?,
                document_no: row.get(1)?,
                scope_type: row.get(2)?,
                status: row.get(3)?,
                item_code: row.get(4)?,
                item_name: row.get(5)?,
                spec: row.get(6)?,
                unit_name: row.get(7)?,
                book_quantity: row.get(8)?,
                counted_quantity: row.get(9)?,
                difference_quantity: row.get(10)?,
                average_price: row.get(11)?,
                difference_amount: row.get(12)?,
                remark: row.get(13)?,
            })
        },
    )?;
    collect_rows(rows)
}

fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> AppResult<Vec<T>> {
    let mut output = Vec::new();
    for row in rows {
        output.push(row?);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::db::migrations;

    use super::*;

    #[test]
    fn get_report_bundle_filters_department_summary_and_details_by_department_scope() {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO master_items (id, code, name, unit_id, default_price, enabled)
             VALUES ('item-a', 'A', '测试物品', 'unit-piece', 1, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO stock_balances (id, item_id, quantity, amount)
             VALUES ('balance-a', 'item-a', 10, 10)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               department_id, movement_type
             )
             VALUES
               ('mov-admin', '2026-06-10', 'item-a', 'out', 2, 1, 2, 'dept-admin-office', 'outbound'),
               ('mov-food', '2026-06-11', 'item-a', 'out', 3, 1, 3, 'dept-restaurant', 'outbound')",
            [],
        )
        .unwrap();

        let bundle = get_report_bundle(
            &conn,
            &ReportQuery {
                month: "2026-06".to_string(),
                start_date: None,
                end_date: None,
                department_id: Some("dept-admin-office".to_string()),
                category_id: None,
                item_id: None,
                supplier_id: None,
            },
        )
        .unwrap();

        assert_eq!(bundle.department_summary.len(), 1);
        assert_eq!(
            bundle.department_summary[0].department_id,
            "dept-admin-office"
        );
        assert_eq!(bundle.department_summary[0].quantity, 2.0);
        assert_eq!(bundle.department_details.len(), 1);
        assert_eq!(bundle.department_details[0].department_name, "行政办");
        assert_eq!(bundle.department_details[0].quantity, 2.0);
        assert_eq!(bundle.stock_balances.len(), 1);
        assert_eq!(bundle.stock_balances[0].item_code, "A");
        assert_eq!(bundle.stock_balances[0].quantity, 10.0);
    }

    #[test]
    fn item_consumption_ranking_exports_all_consumed_items() {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();

        for index in 0..120 {
            conn.execute(
                "INSERT INTO master_items (id, code, name, unit_id, default_price, enabled)
                 VALUES (?1, ?2, ?3, 'unit-piece', 1, 1)",
                rusqlite::params![
                    format!("item-rank-{index:03}"),
                    format!("RANK-{index:03}"),
                    format!("排行物品 {index:03}")
                ],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO stock_movements (
                   id, movement_date, item_id, direction, quantity, unit_price, amount,
                   department_id, movement_type
                 )
                 VALUES (?1, '2026-06-15', ?2, 'out', 1, 1, ?3, 'dept-admin-office', 'outbound')",
                rusqlite::params![
                    format!("mov-rank-{index:03}"),
                    format!("item-rank-{index:03}"),
                    (index + 1) as f64
                ],
            )
            .unwrap();
        }

        let bundle = get_report_bundle(
            &conn,
            &ReportQuery {
                month: "2026-06".to_string(),
                start_date: None,
                end_date: None,
                department_id: None,
                category_id: None,
                item_id: None,
                supplier_id: None,
            },
        )
        .unwrap();

        assert_eq!(bundle.item_consumption_ranking.len(), 120);
        assert_eq!(bundle.item_consumption_ranking[0].item_code, "RANK-119");
        assert_eq!(bundle.item_consumption_ranking[119].item_code, "RANK-000");
    }

    #[test]
    fn report_details_use_movement_party_name_snapshots_after_rename() {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO departments (id, code, name)
             VALUES ('dept-report-snapshot', 'RS', '新部门名称')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO suppliers (id, name)
             VALUES ('supplier-report-snapshot', '新供应商名称')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO master_items (id, code, name, unit_id, default_price, enabled)
             VALUES ('item-report-snapshot', 'RPT-SNP', '报表快照物品', 'unit-piece', 10, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               department_id, department_name, supplier_id, supplier_name, movement_type
             )
             VALUES
               ('mov-report-out', '2026-06-10', 'item-report-snapshot', 'out', 1, 10, 10,
                'dept-report-snapshot', '旧部门名称', NULL, NULL, 'outbound'),
               ('mov-report-in', '2026-06-11', 'item-report-snapshot', 'in', 2, 10, 20,
                NULL, NULL, 'supplier-report-snapshot', '旧供应商名称', 'inbound')",
            [],
        )
        .unwrap();

        let bundle = get_report_bundle(
            &conn,
            &ReportQuery {
                month: "2026-06".to_string(),
                start_date: None,
                end_date: None,
                department_id: None,
                category_id: None,
                item_id: None,
                supplier_id: None,
            },
        )
        .unwrap();

        assert_eq!(bundle.department_details[0].department_name, "旧部门名称");
        assert_eq!(bundle.inbound_details[0].supplier_name, "旧供应商名称");
    }

    #[test]
    fn get_report_bundle_filters_movement_reports_by_date_range() {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO categories (id, name, enabled) VALUES ('cat-date', '日期分类', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO master_items (id, code, name, category_id, unit_id, default_price, enabled)
             VALUES ('item-date', 'DATE-001', '日期筛选物品', 'cat-date', 'unit-piece', 10, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               department_id, supplier_id, movement_type
             )
             VALUES
               ('mov-date-in-before', '2026-06-05', 'item-date', 'in', 5, 10, 50, NULL, NULL, 'inbound'),
               ('mov-date-in-inside', '2026-06-15', 'item-date', 'in', 7, 10, 70, NULL, NULL, 'inbound'),
               ('mov-date-out-before', '2026-06-06', 'item-date', 'out', 2, 10, 20, 'dept-admin-office', NULL, 'outbound'),
               ('mov-date-out-inside', '2026-06-16', 'item-date', 'out', 3, 10, 30, 'dept-admin-office', NULL, 'outbound'),
               ('mov-date-out-after', '2026-06-25', 'item-date', 'out', 4, 10, 40, 'dept-admin-office', NULL, 'outbound')",
            [],
        )
        .unwrap();

        let bundle = get_report_bundle(
            &conn,
            &ReportQuery {
                month: "2026-06".to_string(),
                start_date: Some("2026-06-10".to_string()),
                end_date: Some("2026-06-20".to_string()),
                department_id: None,
                category_id: None,
                item_id: None,
                supplier_id: None,
            },
        )
        .unwrap();

        assert_eq!(bundle.monthly_inventory.len(), 1);
        assert_eq!(bundle.monthly_inventory[0].inbound_quantity, 7.0);
        assert_eq!(bundle.monthly_inventory[0].outbound_quantity, 3.0);
        assert_eq!(bundle.department_details.len(), 1);
        assert_eq!(bundle.department_details[0].movement_date, "2026-06-16");
        assert_eq!(bundle.inbound_details.len(), 1);
        assert_eq!(bundle.inbound_details[0].movement_date, "2026-06-15");
        assert_eq!(bundle.department_summary[0].quantity, 3.0);
        assert_eq!(bundle.category_consumption[0].quantity, 3.0);
        assert_eq!(bundle.item_consumption_ranking[0].quantity, 3.0);
    }

    #[test]
    fn get_report_bundle_filters_category_item_and_supplier() {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO categories (id, name, enabled) VALUES ('cat-room', '客房耗材', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO suppliers (id, name, enabled) VALUES ('supplier-a', '供应商 A', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO suppliers (id, name, enabled) VALUES ('supplier-b', '供应商 B', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO master_items (id, code, name, category_id, unit_id, default_price, supplier_id, warning_quantity, enabled)
             VALUES
               ('item-a', 'A', '筛选物品 A', 'cat-room', 'unit-piece', 1, 'supplier-a', 5, 1),
               ('item-b', 'B', '筛选物品 B', NULL, 'unit-piece', 1, 'supplier-b', 5, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO stock_balances (id, item_id, quantity, amount)
             VALUES
               ('balance-a', 'item-a', 4, 4),
               ('balance-b', 'item-b', 4, 4)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               department_id, supplier_id, movement_type
             )
             VALUES
               ('mov-in-a', '2026-06-03', 'item-a', 'in', 8, 1, 8, NULL, 'supplier-a', 'inbound'),
               ('mov-in-b', '2026-06-04', 'item-b', 'in', 9, 1, 9, NULL, 'supplier-b', 'inbound'),
               ('mov-out-a', '2026-06-05', 'item-a', 'out', 2, 1, 2, 'dept-admin-office', NULL, 'outbound'),
               ('mov-out-b', '2026-06-06', 'item-b', 'out', 3, 1, 3, 'dept-restaurant', NULL, 'outbound')",
            [],
        )
        .unwrap();

        let category_bundle = get_report_bundle(
            &conn,
            &ReportQuery {
                month: "2026-06".to_string(),
                start_date: None,
                end_date: None,
                department_id: None,
                category_id: Some("cat-room".to_string()),
                item_id: None,
                supplier_id: None,
            },
        )
        .unwrap();
        assert_eq!(category_bundle.monthly_inventory.len(), 1);
        assert_eq!(category_bundle.monthly_inventory[0].item_code, "A");
        assert_eq!(category_bundle.item_consumption_ranking.len(), 1);
        assert_eq!(category_bundle.stock_balances.len(), 1);
        assert_eq!(category_bundle.stock_warnings.len(), 1);

        let item_bundle = get_report_bundle(
            &conn,
            &ReportQuery {
                month: "2026-06".to_string(),
                start_date: None,
                end_date: None,
                department_id: None,
                category_id: None,
                item_id: Some("item-b".to_string()),
                supplier_id: None,
            },
        )
        .unwrap();
        assert_eq!(item_bundle.department_details.len(), 1);
        assert_eq!(item_bundle.department_details[0].item_code, "B");
        assert_eq!(item_bundle.category_consumption[0].category_name, "未分类");

        let supplier_bundle = get_report_bundle(
            &conn,
            &ReportQuery {
                month: "2026-06".to_string(),
                start_date: None,
                end_date: None,
                department_id: None,
                category_id: None,
                item_id: None,
                supplier_id: Some("supplier-a".to_string()),
            },
        )
        .unwrap();
        assert_eq!(supplier_bundle.inbound_details.len(), 1);
        assert_eq!(supplier_bundle.inbound_details[0].supplier_name, "供应商 A");
        assert_eq!(supplier_bundle.monthly_inventory.len(), 1);
        assert_eq!(supplier_bundle.monthly_inventory[0].item_code, "A");
    }

    #[test]
    fn stocktake_difference_report_filters_month_category_and_item() {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO categories (id, name, enabled) VALUES ('cat-diff', '差异分类', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO master_items (id, code, name, category_id, unit_id, default_price, enabled)
             VALUES
               ('item-diff-a', 'DIFF-A', '差异物品 A', 'cat-diff', 'unit-piece', 10, 1),
               ('item-diff-b', 'DIFF-B', '差异物品 B', NULL, 'unit-piece', 20, 1),
               ('item-diff-zero', 'DIFF-ZERO', '零差异物品', 'cat-diff', 'unit-piece', 5, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO stock_balances (id, item_id, quantity, amount, average_price)
             VALUES
               ('balance-diff-a', 'item-diff-a', 8, 80, 10),
               ('balance-diff-b', 'item-diff-b', 5, 100, 20),
               ('balance-diff-zero', 'item-diff-zero', 8, 40, 5)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_documents (id, document_no, document_type, business_date, status)
             VALUES
               ('doc-diff-before', 'ST-20260605-0001', 'stocktake', '2026-06-05', 'confirmed'),
               ('doc-diff-june', 'ST-20260630-0001', 'stocktake', '2026-06-30', 'confirmed'),
               ('doc-diff-july', 'ST-20260701-0001', 'stocktake', '2026-07-01', 'confirmed'),
               ('doc-diff-draft', 'ST-20260629-0001', 'stocktake', '2026-06-29', 'draft')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stocktake_documents (id, document_id, scope_type, status)
             VALUES
               ('stocktake-diff-before', 'doc-diff-before', 'all', 'confirmed'),
               ('stocktake-diff-june', 'doc-diff-june', 'all', 'confirmed'),
               ('stocktake-diff-july', 'doc-diff-july', 'all', 'confirmed'),
               ('stocktake-diff-draft', 'doc-diff-draft', 'all', 'counting')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stocktake_lines (
               id, stocktake_id, item_id, book_quantity, counted_quantity, difference_quantity, remark
             )
             VALUES
               ('line-before', 'stocktake-diff-before', 'item-diff-a', 8, 7, -1, '范围外'),
               ('line-diff-a', 'stocktake-diff-june', 'item-diff-a', 8, 6, -2, '盘亏'),
               ('line-diff-b', 'stocktake-diff-june', 'item-diff-b', 5, 7, 2, '盘盈'),
               ('line-zero', 'stocktake-diff-june', 'item-diff-zero', 8, 8, 0, '无差异'),
               ('line-july', 'stocktake-diff-july', 'item-diff-a', 8, 9, 1, '下月'),
               ('line-draft', 'stocktake-diff-draft', 'item-diff-a', 8, 3, -5, '未确认')",
            [],
        )
        .unwrap();

        let bundle = get_report_bundle(
            &conn,
            &ReportQuery {
                month: "2026-06".to_string(),
                start_date: None,
                end_date: None,
                department_id: None,
                category_id: Some("cat-diff".to_string()),
                item_id: None,
                supplier_id: None,
            },
        )
        .unwrap();

        assert_eq!(bundle.stocktake_differences.len(), 2);
        let june_difference = bundle
            .stocktake_differences
            .iter()
            .find(|row| row.document_no == "ST-20260630-0001")
            .expect("june difference row");
        assert_eq!(june_difference.item_code, "DIFF-A");
        assert_eq!(june_difference.difference_quantity, -2.0);
        assert_eq!(june_difference.difference_amount, -20.0);

        let item_bundle = get_report_bundle(
            &conn,
            &ReportQuery {
                month: "2026-06".to_string(),
                start_date: None,
                end_date: None,
                department_id: None,
                category_id: None,
                item_id: Some("item-diff-b".to_string()),
                supplier_id: None,
            },
        )
        .unwrap();
        assert_eq!(item_bundle.stocktake_differences.len(), 1);
        assert_eq!(item_bundle.stocktake_differences[0].item_code, "DIFF-B");

        let date_bundle = get_report_bundle(
            &conn,
            &ReportQuery {
                month: "2026-06".to_string(),
                start_date: Some("2026-06-20".to_string()),
                end_date: Some("2026-06-30".to_string()),
                department_id: None,
                category_id: Some("cat-diff".to_string()),
                item_id: None,
                supplier_id: None,
            },
        )
        .unwrap();
        assert_eq!(date_bundle.stocktake_differences.len(), 1);
        assert_eq!(
            date_bundle.stocktake_differences[0].document_no,
            "ST-20260630-0001"
        );
    }

    #[test]
    fn get_report_bundle_includes_guest_sale_profit_rows() {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO master_items (id, code, name, unit_id, default_price, sale_price, enabled)
             VALUES ('item-sale-report', 'SALE-RPT', '销售报表物品', 'unit-piece', 12, 8, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_documents (
               id, document_no, document_type, outbound_kind, business_date, status
             )
             VALUES (
               'doc-sale-report', 'OUT-20260610-0001', 'outbound', 'guest_sale', '2026-06-10', 'confirmed'
             )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_document_lines (
               id, document_id, item_id, quantity, unit_price, amount,
               sale_unit_price, sale_amount, cost_unit_price, cost_amount
             )
             VALUES (
               'line-sale-report', 'doc-sale-report', 'item-sale-report', 2, 8, 16,
               8, 16, 12, 24
             )",
            [],
        )
        .unwrap();

        let bundle = get_report_bundle(
            &conn,
            &ReportQuery {
                month: "2026-06".to_string(),
                start_date: None,
                end_date: None,
                department_id: None,
                category_id: None,
                item_id: None,
                supplier_id: None,
            },
        )
        .unwrap();

        assert_eq!(bundle.sales_profit.len(), 1);
        assert_eq!(bundle.sales_profit[0].sale_amount, 16.0);
        assert_eq!(bundle.sales_profit[0].cost_amount, 24.0);
        assert_eq!(bundle.sales_profit[0].gross_profit, -8.0);
        assert!(bundle.sales_profit[0].negative_profit);
    }
}
