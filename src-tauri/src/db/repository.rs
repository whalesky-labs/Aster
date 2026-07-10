use rusqlite::{params, Connection, OptionalExtension};

use crate::db::pagination;
use crate::domain::pagination::Page;
use crate::domain::status::{AuditLogRow, DashboardMetrics, RecentOperation};
use crate::error::AppResult;

pub fn get_setting(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO app_settings (key, value, updated_at)
         VALUES (?1, ?2, CURRENT_TIMESTAMP)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
        params![key, value],
    )?;
    Ok(())
}

pub fn delete_setting(conn: &Connection, key: &str) -> AppResult<()> {
    conn.execute("DELETE FROM app_settings WHERE key = ?1", params![key])?;
    Ok(())
}

pub fn schema_version(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?)
}

pub fn dashboard_metrics(
    conn: &Connection,
    department_id: Option<&str>,
) -> AppResult<DashboardMetrics> {
    let item_count = count(conn, "master_items", "enabled = 1")?;
    let department_count = count(conn, "departments", "enabled = 1")?;
    let supplier_count = count(conn, "suppliers", "enabled = 1")?;
    let current_stock_amount = sum(conn, "stock_balances", "amount", "1 = 1")?;
    let low_stock_count = conn.query_row(
        "SELECT COUNT(*)
         FROM stock_balances b
         JOIN master_items i ON i.id = b.item_id
         WHERE i.enabled = 1
           AND b.quantity >= 0
           AND b.quantity <= i.warning_quantity",
        [],
        |row| row.get(0),
    )?;
    let negative_stock_count = count(conn, "stock_balances", "quantity < 0")?;
    let this_month_inbound_amount = sum_current_month(conn, "in")?;
    let this_month_outbound_amount = sum_current_month_for_department(conn, "out", department_id)?;

    Ok(DashboardMetrics {
        item_count,
        department_count,
        supplier_count,
        current_stock_amount,
        low_stock_count,
        negative_stock_count,
        this_month_inbound_amount,
        this_month_outbound_amount,
    })
}

pub fn latest_successful_backup(conn: &Connection) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT created_at
             FROM backup_jobs
             WHERE status = 'success'
             ORDER BY created_at DESC
             LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn latest_movement_month(conn: &Connection) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT strftime('%Y-%m', MAX(movement_date))
             FROM stock_movements",
            [],
            |row| row.get(0),
        )
        .optional()?
        .flatten())
}

pub fn recent_operations(
    conn: &Connection,
    limit: i64,
    department_id: Option<&str>,
) -> AppResult<Vec<RecentOperation>> {
    let mut stmt = conn.prepare(
        "SELECT m.id, m.created_at, m.movement_type, i.name, m.quantity,
                COALESCE(m.department_name, dep.name),
                COALESCE(m.supplier_name, sup.name)
         FROM stock_movements m
         JOIN master_items i ON i.id = m.item_id
         LEFT JOIN departments dep ON dep.id = m.department_id
         LEFT JOIN suppliers sup ON sup.id = m.supplier_id
         WHERE (?2 IS NULL OR m.department_id = ?2)
         ORDER BY m.created_at DESC, m.movement_date DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit, department_id], |row| {
        Ok(RecentOperation {
            id: row.get(0)?,
            occurred_at: row.get(1)?,
            business_type: row.get(2)?,
            item_name: row.get(3)?,
            quantity: row.get(4)?,
            department_name: row.get(5)?,
            supplier_name: row.get(6)?,
        })
    })?;
    let mut output = Vec::new();
    for row in rows {
        output.push(row?);
    }
    Ok(output)
}

pub fn list_audit_logs(conn: &Connection, limit: i64) -> AppResult<Vec<AuditLogRow>> {
    pagination::collect_all(|cursor| list_audit_logs_page(conn, limit, cursor))
}

pub fn list_audit_logs_page(
    conn: &Connection,
    limit: i64,
    cursor: Option<&str>,
) -> AppResult<Page<AuditLogRow>> {
    let limit = limit.clamp(1, 500);
    let scope = format!("audit-logs:{limit}");
    let offset = pagination::offset(conn, &scope, cursor)?;
    let remaining = (limit - offset).max(0);
    if remaining == 0 {
        return Ok(Page {
            items: Vec::new(),
            next_cursor: None,
        });
    }
    let fetch = remaining;
    let mut stmt = conn.prepare(
        "SELECT id, action, entity_type, entity_id, summary, operator, created_at
         FROM audit_logs
         ORDER BY created_at DESC, rowid DESC
         LIMIT ?1 OFFSET ?2",
    )?;
    let rows = stmt.query_map(params![fetch, offset], |row| {
        Ok(AuditLogRow {
            id: row.get(0)?,
            action: row.get(1)?,
            entity_type: row.get(2)?,
            entity_id: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            summary: row.get(4)?,
            operator: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;
    let mut output = Vec::new();
    for row in rows {
        output.push(row?);
    }
    Ok(Page {
        items: output,
        next_cursor: None,
    })
}

pub fn integrity_check(conn: &Connection) -> AppResult<String> {
    Ok(conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?)
}

pub fn stock_balance_consistency_issue_count(conn: &Connection) -> AppResult<i64> {
    let movement_mismatch_count: i64 = conn.query_row(
        "WITH movement_totals AS (
           SELECT item_id,
                  ROUND(SUM(CASE WHEN direction = 'in' THEN quantity ELSE -quantity END), 4) AS movement_quantity,
                  ROUND(SUM(CASE WHEN direction = 'in' THEN amount ELSE -amount END), 4) AS movement_amount
           FROM stock_movements
           GROUP BY item_id
         )
         SELECT COUNT(*)
         FROM movement_totals mt
         LEFT JOIN stock_balances b ON b.item_id = mt.item_id
         WHERE b.item_id IS NULL
            OR ABS(ROUND(COALESCE(b.quantity, 0), 4) - mt.movement_quantity) > 0.0001
            OR ABS(ROUND(COALESCE(b.amount, 0), 4) - mt.movement_amount) > 0.0001",
        [],
        |row| row.get(0),
    )?;
    let orphan_balance_count: i64 = conn.query_row(
        "SELECT COUNT(*)
         FROM stock_balances b
         LEFT JOIN stock_movements m ON m.item_id = b.item_id
         WHERE m.item_id IS NULL
           AND (ABS(ROUND(b.quantity, 4)) > 0.0001 OR ABS(ROUND(b.amount, 4)) > 0.0001)",
        [],
        |row| row.get(0),
    )?;
    Ok(movement_mismatch_count + orphan_balance_count)
}

fn count(conn: &Connection, table: &str, predicate: &str) -> AppResult<i64> {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE {predicate}");
    Ok(conn.query_row(&sql, [], |row| row.get(0))?)
}

fn sum(conn: &Connection, table: &str, column: &str, predicate: &str) -> AppResult<f64> {
    let sql = format!("SELECT COALESCE(SUM({column}), 0) FROM {table} WHERE {predicate}");
    Ok(conn.query_row(&sql, [], |row| row.get(0))?)
}

fn sum_current_month(conn: &Connection, direction: &str) -> AppResult<f64> {
    sum_current_month_for_department(conn, direction, None)
}

fn sum_current_month_for_department(
    conn: &Connection,
    direction: &str,
    department_id: Option<&str>,
) -> AppResult<f64> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(amount), 0)
         FROM stock_movements
         WHERE direction = ?1
           AND strftime('%Y-%m', movement_date) = strftime('%Y-%m', 'now')
           AND (?2 IS NULL OR department_id = ?2)",
        params![direction, department_id],
        |row| row.get(0),
    )?)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::db::migrations;

    use super::*;

    #[test]
    fn recent_operations_returns_stock_movements_with_business_context() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-recent', 'REC-001', '最近操作物品', 'unit-piece', 10)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               department_id, movement_type, created_at
             )
             VALUES (
               'mov-recent', '2026-06-30', 'item-recent', 'out', 3, 10, 30,
               'dept-admin-office', 'outbound', '2026-06-30T10:00:00+08:00'
             )",
            [],
        )
        .unwrap();

        let rows = recent_operations(&conn, 8, None).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].business_type, "outbound");
        assert_eq!(rows[0].item_name, "最近操作物品");
        assert_eq!(rows[0].quantity, 3.0);
        assert_eq!(rows[0].department_name.as_deref(), Some("行政办"));
    }

    #[test]
    fn list_audit_logs_returns_latest_rows_with_limit() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator, created_at)
             VALUES
             ('audit-old', 'save_item', 'item', 'item-old', '旧记录', 'admin', '2026-06-30T09:00:00+08:00'),
             ('audit-new', 'create_backup', 'backup', 'backup-new', '新记录', 'admin', '2026-06-30T10:00:00+08:00')",
            [],
        )
        .unwrap();

        let rows = list_audit_logs(&conn, 1).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "audit-new");
        assert_eq!(rows[0].summary, "新记录");
    }

    #[test]
    fn list_audit_logs_handles_null_entity_id() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator, created_at)
             VALUES ('audit-null-entity', 'rebuild_stock_balances', 'stock_balances', NULL, '重建库存余额', 'system', '2026-07-08T15:08:08+08:00')",
            [],
        )
        .unwrap();

        let rows = list_audit_logs(&conn, 20).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "audit-null-entity");
        assert_eq!(rows[0].entity_id, "");
    }

    #[test]
    fn stock_balance_consistency_counts_movement_balance_mismatches() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-consistency', 'CON-001', '一致性物品', 'unit-piece', 10)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_balances (id, item_id, quantity, amount, average_price)
             VALUES ('balance-consistency', 'item-consistency', 5, 50, 10)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount, movement_type
             )
             VALUES (
               'mov-consistency', '2026-06-30', 'item-consistency', 'in', 4, 10, 40, 'inbound'
             )",
            [],
        )
        .unwrap();

        assert_eq!(stock_balance_consistency_issue_count(&conn).unwrap(), 1);

        conn.execute(
            "UPDATE stock_balances SET quantity = 4, amount = 40 WHERE item_id = 'item-consistency'",
            [],
        )
        .unwrap();
        assert_eq!(stock_balance_consistency_issue_count(&conn).unwrap(), 0);
    }
}
