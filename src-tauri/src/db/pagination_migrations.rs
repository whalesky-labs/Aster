use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::error::AppResult;

const PAGINATED_TABLES: &[&str] = &[
    "categories",
    "units",
    "departments",
    "suppliers",
    "master_items",
    "users",
    "roles",
    "user_roles",
    "stock_documents",
    "stock_document_lines",
    "stock_movements",
    "stock_batches",
    "stock_batch_movements",
    "stock_balances",
    "stocktake_documents",
    "stocktake_lines",
    "approval_requests",
    "budget_rules",
    "backup_jobs",
    "audit_logs",
];

pub fn run(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS pagination_revision (
           id INTEGER PRIMARY KEY CHECK (id = 1),
           revision INTEGER NOT NULL,
           clients_revision INTEGER NOT NULL DEFAULT 0,
           database_epoch TEXT NOT NULL DEFAULT ''
         );
         INSERT OR IGNORE INTO pagination_revision (id, revision) VALUES (1, 0);",
    )?;
    if !column_exists(conn, "pagination_revision", "clients_revision")? {
        conn.execute(
            "ALTER TABLE pagination_revision ADD COLUMN clients_revision INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !column_exists(conn, "pagination_revision", "database_epoch")? {
        conn.execute(
            "ALTER TABLE pagination_revision ADD COLUMN database_epoch TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    conn.execute(
        "UPDATE pagination_revision SET database_epoch = ?1 WHERE id = 1",
        params![Uuid::new_v4().to_string()],
    )?;
    for table in PAGINATED_TABLES {
        if !table_exists(conn, table)? {
            continue;
        }
        for operation in ["insert", "update", "delete"] {
            conn.execute_batch(&format!(
                "CREATE TRIGGER IF NOT EXISTS pagination_revision_{table}_{operation}
                 AFTER {operation} ON {table}
                 BEGIN
                   UPDATE pagination_revision SET revision = revision + 1 WHERE id = 1;
                 END;"
            ))?;
        }
    }
    for operation in ["insert", "update", "delete"] {
        conn.execute_batch(&format!(
            "DROP TRIGGER IF EXISTS pagination_revision_client_connections_{operation};"
        ))?;
        conn.execute_batch(&format!(
            "CREATE TRIGGER pagination_revision_client_connections_{operation}
             AFTER {operation} ON client_connections
             BEGIN
               UPDATE pagination_revision SET clients_revision = clients_revision + 1 WHERE id = 1;
             END;"
        ))?;
    }
    Ok(())
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> AppResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2)",
        params![table, column],
        |row| row.get(0),
    )?)
}

fn table_exists(conn: &Connection, table: &str) -> AppResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [table],
        |row| row.get(0),
    )?)
}
