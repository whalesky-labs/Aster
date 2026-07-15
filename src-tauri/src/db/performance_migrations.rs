use rusqlite::Connection;

use crate::error::AppResult;

pub fn run(conn: &Connection) -> AppResult<()> {
    if has_columns(
        conn,
        "stock_movements",
        &["direction", "movement_date", "item_id", "department_id"],
    )? {
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_stock_movements_direction_date
               ON stock_movements(direction, movement_date);
             CREATE INDEX IF NOT EXISTS idx_stock_movements_item_date
               ON stock_movements(item_id, movement_date);
             CREATE INDEX IF NOT EXISTS idx_stock_movements_department_date
               ON stock_movements(department_id, movement_date);",
        )?;
    }
    if has_columns(
        conn,
        "stock_documents",
        &["document_type", "status", "business_date"],
    )? {
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_stock_documents_type_status_date
             ON stock_documents(document_type, status, business_date)",
            [],
        )?;
    }
    Ok(())
}

fn has_columns(conn: &Connection, table: &str, columns: &[&str]) -> AppResult<bool> {
    let mut stmt = conn.prepare("SELECT name FROM pragma_table_info(?1)")?;
    let present = stmt
        .query_map([table], |row| row.get::<_, String>(0))?
        .collect::<Result<std::collections::HashSet<_>, _>>()?;
    Ok(columns.iter().all(|column| present.contains(*column)))
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    #[test]
    fn monthly_movement_range_uses_the_direction_date_index() {
        let conn = Connection::open_in_memory().expect("database");
        crate::db::migrations::run(&conn).expect("migrations");
        let detail = conn
            .prepare(
                "EXPLAIN QUERY PLAN
                 SELECT id FROM stock_movements
                 WHERE direction = 'out'
                   AND movement_date >= '2026-07-01'
                   AND movement_date < '2026-08-01'",
            )
            .expect("explain")
            .query_map([], |row| row.get::<_, String>(3))
            .expect("plan")
            .collect::<Result<Vec<_>, _>>()
            .expect("rows")
            .join("\n");
        assert!(detail.contains("idx_stock_movements_direction_date"));
        assert!(!detail.contains("SCAN stock_movements"));
    }
}
