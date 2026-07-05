use rusqlite::{params, Connection};

use crate::error::AppResult;

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
    }
    if table_exists(conn, "stock_movements")? {
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
