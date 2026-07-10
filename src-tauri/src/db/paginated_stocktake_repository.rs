use rusqlite::{params, Connection};

use crate::db::pagination::{self, FETCH_SIZE};
use crate::domain::pagination::Page;
use crate::domain::stocktake::StocktakeDocument;
use crate::error::AppResult;

pub fn list_page(conn: &Connection, cursor: Option<&str>) -> AppResult<Page<StocktakeDocument>> {
    let offset = pagination::offset(conn, "stocktakes", cursor)?;
    let mut statement = conn.prepare(
        "SELECT st.id, st.document_id, d.document_no, d.business_date, st.scope_type,
                st.status, d.handler, d.remark, COUNT(l.id),
                SUM(CASE WHEN l.counted_quantity IS NOT NULL THEN 1 ELSE 0 END),
                SUM(CASE WHEN ABS(l.difference_quantity) > 0.000001 THEN 1 ELSE 0 END),
                COALESCE(SUM(CASE WHEN l.difference_quantity > 0 THEN l.difference_quantity * COALESCE(b.average_price, i.default_price, 0) ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN l.difference_quantity < 0 THEN ABS(l.difference_quantity) * COALESCE(b.average_price, i.default_price, 0) ELSE 0 END), 0),
                st.created_at, d.confirmed_at
         FROM stocktake_documents st JOIN stock_documents d ON d.id = st.document_id
         LEFT JOIN stocktake_lines l ON l.stocktake_id = st.id
         LEFT JOIN master_items i ON i.id = l.item_id LEFT JOIN stock_balances b ON b.item_id = l.item_id
         GROUP BY st.id ORDER BY st.created_at DESC, st.id DESC LIMIT ?1 OFFSET ?2",
    )?;
    let rows = statement.query_map(params![FETCH_SIZE, offset], |row| {
        Ok(StocktakeDocument {
            id: row.get(0)?,
            document_id: row.get(1)?,
            document_no: row.get(2)?,
            business_date: row.get(3)?,
            scope_type: row.get(4)?,
            status: row.get(5)?,
            handler: row.get(6)?,
            remark: row.get(7)?,
            line_count: row.get(8)?,
            counted_count: row.get(9)?,
            difference_count: row.get(10)?,
            gain_amount: row.get(11)?,
            loss_amount: row.get(12)?,
            created_at: row.get(13)?,
            confirmed_at: row.get(14)?,
        })
    })?;
    let items = rows.collect::<Result<Vec<_>, _>>()?;
    pagination::page(conn, "stocktakes", offset, items)
}
