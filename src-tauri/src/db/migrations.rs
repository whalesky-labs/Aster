use crate::error::AppResult;
use rusqlite::{params, Connection};
const MIGRATIONS: &[(i64, &str, &str)] = &[(
    1,
    "initial_schema",
    include_str!("../../migrations/001_initial_schema.sql"),
)];

pub fn run(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );",
    )?;

    for (version, name, sql) in MIGRATIONS {
        let already_applied: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            params![version],
            |row| row.get(0),
        )?;

        if !already_applied {
            conn.execute_batch(sql)?;
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, name) VALUES (?1, ?2)",
                params![version, name],
            )?;
        }
    }

    run_compatibility_migrations(conn)?;
    crate::db::pagination_migrations::run(conn)?;

    Ok(())
}

fn run_compatibility_migrations(conn: &Connection) -> AppResult<()> {
    if !column_exists(conn, "users", "department_id")? {
        conn.execute(
            "ALTER TABLE users ADD COLUMN department_id TEXT REFERENCES departments(id) ON DELETE SET NULL",
            [],
        )?;
    }
    if !column_exists(conn, "users", "email")? {
        conn.execute("ALTER TABLE users ADD COLUMN email TEXT", [])?;
    }
    crate::db::security_migrations::run(conn)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS password_reset_codes (
          id TEXT PRIMARY KEY,
          user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
          code_hash TEXT NOT NULL,
          expires_at TEXT NOT NULL,
          used_at TEXT,
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_password_reset_codes_user_id
         ON password_reset_codes(user_id)",
        [],
    )?;
    if !column_exists(conn, "master_items", "barcode")? {
        conn.execute("ALTER TABLE master_items ADD COLUMN barcode TEXT", [])?;
        conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_master_items_barcode ON master_items(barcode)",
            [],
        )?;
    }
    if !column_exists(conn, "master_items", "sale_price")? {
        conn.execute(
            "ALTER TABLE master_items ADD COLUMN sale_price REAL NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if table_exists(conn, "backup_jobs")? {
        if !column_exists(conn, "backup_jobs", "host_name")? {
            conn.execute("ALTER TABLE backup_jobs ADD COLUMN host_name TEXT", [])?;
        }
        if !column_exists(conn, "backup_jobs", "os")? {
            conn.execute("ALTER TABLE backup_jobs ADD COLUMN os TEXT", [])?;
        }
    }
    if table_exists(conn, "stock_documents")? {
        if !column_exists(conn, "stock_documents", "department_name")? {
            conn.execute(
                "ALTER TABLE stock_documents ADD COLUMN department_name TEXT",
                [],
            )?;
            conn.execute(
                "UPDATE stock_documents
                 SET department_name = (
                   SELECT name FROM departments WHERE departments.id = stock_documents.department_id
                 )
                 WHERE department_id IS NOT NULL",
                [],
            )?;
        }
        if !column_exists(conn, "stock_documents", "supplier_name")? {
            conn.execute(
                "ALTER TABLE stock_documents ADD COLUMN supplier_name TEXT",
                [],
            )?;
            conn.execute(
                "UPDATE stock_documents
                 SET supplier_name = (
                   SELECT name FROM suppliers WHERE suppliers.id = stock_documents.supplier_id
                 )
                 WHERE supplier_id IS NOT NULL",
                [],
            )?;
        }
        if !column_exists(conn, "stock_documents", "approval_request_id")? {
            conn.execute(
                "ALTER TABLE stock_documents ADD COLUMN approval_request_id TEXT",
                [],
            )?;
        }
        if !column_exists(conn, "stock_documents", "outbound_kind")? {
            conn.execute(
                "ALTER TABLE stock_documents ADD COLUMN outbound_kind TEXT",
                [],
            )?;
        }
        if column_exists(conn, "stock_documents", "document_type")? {
            conn.execute(
                "UPDATE stock_documents
                 SET outbound_kind = 'internal'
                 WHERE document_type = 'outbound'
                   AND (outbound_kind IS NULL OR TRIM(outbound_kind) = '')",
                [],
            )?;
        }
        if column_exists(conn, "stock_documents", "business_date")?
            && column_exists(conn, "stock_documents", "created_at")?
        {
            conn.execute(
                "UPDATE stock_documents
                 SET business_date = business_date || ' ' || COALESCE(strftime('%H:%M:%S', created_at), '00:00:00')
                 WHERE length(TRIM(business_date)) = 10",
                [],
            )?;
        }
    }
    if table_exists(conn, "stock_document_lines")? {
        if !column_exists(conn, "stock_document_lines", "purchase_unit_price")? {
            conn.execute(
                "ALTER TABLE stock_document_lines ADD COLUMN purchase_unit_price REAL",
                [],
            )?;
        }
        if !column_exists(conn, "stock_document_lines", "purchase_amount")? {
            conn.execute(
                "ALTER TABLE stock_document_lines ADD COLUMN purchase_amount REAL",
                [],
            )?;
        }
        if !column_exists(conn, "stock_document_lines", "sale_unit_price")? {
            conn.execute(
                "ALTER TABLE stock_document_lines ADD COLUMN sale_unit_price REAL",
                [],
            )?;
        }
        if !column_exists(conn, "stock_document_lines", "sale_amount")? {
            conn.execute(
                "ALTER TABLE stock_document_lines ADD COLUMN sale_amount REAL",
                [],
            )?;
        }
        if !column_exists(conn, "stock_document_lines", "cost_unit_price")? {
            conn.execute(
                "ALTER TABLE stock_document_lines ADD COLUMN cost_unit_price REAL",
                [],
            )?;
        }
        if !column_exists(conn, "stock_document_lines", "cost_amount")? {
            conn.execute(
                "ALTER TABLE stock_document_lines ADD COLUMN cost_amount REAL",
                [],
            )?;
        }
        conn.execute(
            "UPDATE stock_document_lines
             SET purchase_unit_price = COALESCE(purchase_unit_price, unit_price),
                 purchase_amount = COALESCE(purchase_amount, amount)
             WHERE document_id IN (
               SELECT id FROM stock_documents WHERE document_type = 'inbound'
             )",
            [],
        )?;
        conn.execute(
            "UPDATE stock_document_lines
             SET cost_unit_price = COALESCE(cost_unit_price, unit_price),
                 cost_amount = COALESCE(cost_amount, amount)
             WHERE document_id IN (
               SELECT id FROM stock_documents
               WHERE document_type IN ('outbound', 'adjustment', 'stocktake')
             )",
            [],
        )?;
    }
    if table_exists(conn, "stock_movements")? {
        if !column_exists(conn, "stock_movements", "batch_id")? {
            conn.execute("ALTER TABLE stock_movements ADD COLUMN batch_id TEXT", [])?;
        }
        if !column_exists(conn, "stock_movements", "department_name")? {
            conn.execute(
                "ALTER TABLE stock_movements ADD COLUMN department_name TEXT",
                [],
            )?;
            conn.execute(
                "UPDATE stock_movements
                 SET department_name = (
                   SELECT name FROM departments WHERE departments.id = stock_movements.department_id
                 )
                 WHERE department_id IS NOT NULL",
                [],
            )?;
        }
        if !column_exists(conn, "stock_movements", "supplier_name")? {
            conn.execute(
                "ALTER TABLE stock_movements ADD COLUMN supplier_name TEXT",
                [],
            )?;
            conn.execute(
                "UPDATE stock_movements
                 SET supplier_name = (
                   SELECT name FROM suppliers WHERE suppliers.id = stock_movements.supplier_id
                 )
                 WHERE supplier_id IS NOT NULL",
                [],
            )?;
        }
        if column_exists(conn, "stock_movements", "movement_date")?
            && column_exists(conn, "stock_movements", "created_at")?
        {
            conn.execute(
                "UPDATE stock_movements
                 SET movement_date = movement_date || ' ' || COALESCE(strftime('%H:%M:%S', created_at), '00:00:00')
                 WHERE length(TRIM(movement_date)) = 10",
                [],
            )?;
        }
    }
    conn.execute(
        "CREATE TABLE IF NOT EXISTS stock_batches (
          id TEXT PRIMARY KEY,
          item_id TEXT NOT NULL REFERENCES master_items(id) ON DELETE RESTRICT,
          source_document_id TEXT REFERENCES stock_documents(id) ON DELETE SET NULL,
          source_document_line_id TEXT REFERENCES stock_document_lines(id) ON DELETE SET NULL,
          batch_no TEXT NOT NULL UNIQUE,
          inbound_date TEXT NOT NULL,
          supplier_id TEXT REFERENCES suppliers(id) ON DELETE SET NULL,
          supplier_name TEXT,
          original_quantity REAL NOT NULL,
          remaining_quantity REAL NOT NULL,
          unit_price REAL NOT NULL DEFAULT 0,
          original_amount REAL NOT NULL DEFAULT 0,
          remaining_amount REAL NOT NULL DEFAULT 0,
          status TEXT NOT NULL DEFAULT 'available' CHECK(status IN ('available', 'depleted', 'voided', 'adjustment')),
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
          updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS stock_batch_movements (
          id TEXT PRIMARY KEY,
          batch_id TEXT NOT NULL REFERENCES stock_batches(id) ON DELETE RESTRICT,
          stock_movement_id TEXT REFERENCES stock_movements(id) ON DELETE SET NULL,
          document_id TEXT REFERENCES stock_documents(id) ON DELETE SET NULL,
          document_line_id TEXT REFERENCES stock_document_lines(id) ON DELETE SET NULL,
          direction TEXT NOT NULL CHECK(direction IN ('in', 'out')),
          quantity REAL NOT NULL,
          unit_price REAL NOT NULL DEFAULT 0,
          amount REAL NOT NULL DEFAULT 0,
          movement_type TEXT NOT NULL,
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stock_batches_item_available
         ON stock_batches(item_id, status, inbound_date, created_at)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stock_batches_source_line
         ON stock_batches(source_document_line_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stock_batch_movements_batch
         ON stock_batch_movements(batch_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stock_batch_movements_document
         ON stock_batch_movements(document_id)",
        [],
    )?;
    if table_exists(conn, "stock_balances")?
        && column_exists(conn, "stock_balances", "item_id")?
        && column_exists(conn, "stock_balances", "quantity")?
        && column_exists(conn, "stock_balances", "amount")?
        && column_exists(conn, "stock_balances", "average_price")?
        && column_exists(conn, "master_items", "code")?
        && column_exists(conn, "master_items", "default_price")?
    {
        conn.execute(
            "INSERT INTO stock_batches (
               id, item_id, source_document_id, source_document_line_id,
               batch_no, inbound_date, supplier_id, supplier_name,
               original_quantity, remaining_quantity, unit_price,
               original_amount, remaining_amount, status
             )
             SELECT
               lower(hex(randomblob(16))),
               b.item_id,
               NULL,
               NULL,
               'OPEN-' || i.code,
               '1970-01-01',
               NULL,
               '期初库存',
               b.quantity,
               b.quantity,
               CASE
                 WHEN b.average_price > 0 THEN b.average_price
                 WHEN b.quantity > 0 THEN b.amount / b.quantity
                 ELSE i.default_price
               END,
               b.amount,
               b.amount,
               CASE WHEN b.quantity > 0 THEN 'available' ELSE 'depleted' END
             FROM stock_balances b
             JOIN master_items i ON i.id = b.item_id
             WHERE b.quantity > 0
               AND NOT EXISTS (
                 SELECT 1 FROM stock_batches existing
                 WHERE existing.item_id = b.item_id
               )",
            [],
        )?;
    }
    if column_exists(conn, "stock_batches", "inbound_date")? {
        if column_exists(conn, "stock_batches", "created_at")? {
            conn.execute(
                "UPDATE stock_batches
                 SET inbound_date = inbound_date || ' ' || COALESCE(strftime('%H:%M:%S', created_at), '00:00:00')
                 WHERE length(TRIM(inbound_date)) = 10",
                [],
            )?;
        } else {
            conn.execute(
                "UPDATE stock_batches
                 SET inbound_date = inbound_date || ' 00:00:00'
                 WHERE length(TRIM(inbound_date)) = 10",
                [],
            )?;
        }
    }
    conn.execute(
        "CREATE TABLE IF NOT EXISTS client_connections (
          id TEXT PRIMARY KEY,
          client_name TEXT NOT NULL,
          client_device_id TEXT NOT NULL,
          token_hash TEXT NOT NULL DEFAULT '',
          client_ip TEXT,
          app_version TEXT,
          status TEXT NOT NULL DEFAULT 'paired',
          last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
          updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    if !column_exists(conn, "client_connections", "token_hash")? {
        conn.execute(
            "ALTER TABLE client_connections ADD COLUMN token_hash TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    conn.execute(
        "DELETE FROM client_connections
         WHERE rowid NOT IN (
           SELECT MAX(rowid)
           FROM client_connections
           GROUP BY client_device_id
         )",
        [],
    )?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_client_connections_device_id
         ON client_connections(client_device_id)",
        [],
    )?;
    if table_exists(conn, "budget_rules")? {
        conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_budget_rules_department_month
             ON budget_rules(department_id, period_month)
             WHERE category_id IS NULL",
            [],
        )?;
        conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_budget_rules_department_category_month
             ON budget_rules(department_id, category_id, period_month)
             WHERE category_id IS NOT NULL",
            [],
        )?;
    }
    Ok(())
}

fn table_exists(conn: &Connection, table_name: &str) -> AppResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1
        )",
        params![table_name],
        |row| row.get(0),
    )?)
}

fn column_exists(conn: &Connection, table_name: &str, column_name: &str) -> AppResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2
        )",
        params![table_name, column_name],
        |row| row.get(0),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatibility_migrations_upgrade_existing_version_one_database() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
             );
             INSERT INTO schema_migrations (version, name) VALUES (1, 'initial_schema');
             CREATE TABLE users (id TEXT PRIMARY KEY);
             CREATE TABLE master_items (id TEXT PRIMARY KEY);
             CREATE TABLE backup_jobs (
                id TEXT PRIMARY KEY,
                backup_file TEXT NOT NULL,
                backup_type TEXT NOT NULL,
                app_version TEXT NOT NULL,
                schema_version INTEGER NOT NULL,
                database_size INTEGER NOT NULL DEFAULT 0,
                sha256 TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                error_message TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
             );
             CREATE TABLE client_connections (
                id TEXT PRIMARY KEY,
                client_name TEXT NOT NULL,
                client_device_id TEXT NOT NULL,
                client_ip TEXT,
                app_version TEXT,
                status TEXT NOT NULL DEFAULT 'paired',
                last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
             );
             CREATE TABLE departments (id TEXT PRIMARY KEY, name TEXT NOT NULL);
             CREATE TABLE suppliers (id TEXT PRIMARY KEY, name TEXT NOT NULL);
             CREATE TABLE stock_documents (
                id TEXT PRIMARY KEY,
                department_id TEXT,
                supplier_id TEXT
             );
             CREATE TABLE stock_movements (
                id TEXT PRIMARY KEY,
                department_id TEXT,
                supplier_id TEXT
             );
             INSERT INTO departments (id, name) VALUES ('dept-old', '旧部门');
             INSERT INTO suppliers (id, name) VALUES ('supplier-old', '旧供应商');
             INSERT INTO stock_documents (id, department_id, supplier_id)
             VALUES ('doc-old', 'dept-old', 'supplier-old');
             INSERT INTO stock_movements (id, department_id, supplier_id)
             VALUES ('mov-old', 'dept-old', 'supplier-old');
             INSERT INTO client_connections (
                id, client_name, client_device_id, client_ip, app_version, status
             )
             VALUES
                ('old-client-a', '前台旧记录', 'device-frontdesk', '192.168.1.20', '0.1.0', 'paired'),
                ('old-client-b', '前台新记录', 'device-frontdesk', '192.168.1.21', '0.1.0', 'online');",
        )
        .unwrap();

        run(&conn).unwrap();

        assert!(column_exists(&conn, "users", "department_id").unwrap());
        assert!(column_exists(&conn, "master_items", "barcode").unwrap());
        assert!(column_exists(&conn, "backup_jobs", "host_name").unwrap());
        assert!(column_exists(&conn, "backup_jobs", "os").unwrap());
        assert!(column_exists(&conn, "stock_documents", "department_name").unwrap());
        assert!(column_exists(&conn, "stock_documents", "supplier_name").unwrap());
        assert!(column_exists(&conn, "stock_movements", "department_name").unwrap());
        assert!(column_exists(&conn, "stock_movements", "supplier_name").unwrap());
        let document_names: (String, String) = conn
            .query_row(
                "SELECT department_name, supplier_name FROM stock_documents WHERE id = 'doc-old'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            document_names,
            ("旧部门".to_string(), "旧供应商".to_string())
        );
        let movement_names: (String, String) = conn
            .query_row(
                "SELECT department_name, supplier_name FROM stock_movements WHERE id = 'mov-old'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            movement_names,
            ("旧部门".to_string(), "旧供应商".to_string())
        );
        assert!(table_exists(&conn, "client_connections").unwrap());
        assert!(column_exists(&conn, "client_connections", "token_hash").unwrap());
        let duplicate_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM client_connections WHERE client_device_id = 'device-frontdesk'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(duplicate_count, 1);
        conn.execute(
            "INSERT INTO client_connections (id, client_name, client_device_id, status)
             VALUES ('duplicate-client', '重复客户端', 'device-frontdesk', 'paired')",
            [],
        )
        .unwrap_err();
    }
}
