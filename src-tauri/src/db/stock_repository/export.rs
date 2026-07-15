use rusqlite::Connection;

use crate::domain::stock::StockBalanceExportRow;
use crate::error::AppResult;

pub fn list_stock_balance_export_rows(conn: &Connection) -> AppResult<Vec<StockBalanceExportRow>> {
    let mut statement = conn.prepare(
        "SELECT i.id, i.code, i.name, c.name, i.spec, u.name, s.name,
                COALESCE(b.quantity, 0), COALESCE(b.average_price, 0),
                COALESCE(b.amount, 0), COALESCE(b.last_inbound_price, 0),
                i.warning_quantity, i.enabled
         FROM master_items i
         LEFT JOIN categories c ON c.id = i.category_id
         LEFT JOIN units u ON u.id = i.unit_id
         LEFT JOIN suppliers s ON s.id = i.supplier_id
         LEFT JOIN stock_balances b ON b.item_id = i.id
         ORDER BY i.enabled DESC, i.code ASC, i.id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        let quantity: f64 = row.get(7)?;
        let warning_quantity: f64 = row.get(11)?;
        Ok(StockBalanceExportRow {
            item_id: row.get(0)?,
            item_code: row.get(1)?,
            item_name: row.get(2)?,
            category_name: row.get(3)?,
            spec: row.get(4)?,
            unit_name: row.get(5)?,
            supplier_name: row.get(6)?,
            quantity,
            average_price: row.get(8)?,
            amount: row.get(9)?,
            last_inbound_price: row.get(10)?,
            warning_quantity,
            stock_status: super::stock_status_code(quantity, warning_quantity).to_string(),
            item_enabled: row.get::<_, i64>(12)? == 1,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}
