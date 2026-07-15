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
