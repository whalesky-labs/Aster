use super::*;

#[test]
fn ensure_host_service_for_non_host_mode_stops_runtime() {
    let (_dir, db) = test_db();
    let state = AppState {
        paths: AppPaths {
            data_dir: _dir.path().to_path_buf(),
            database_path: _dir.path().join("aster.sqlite"),
            backup_dir: _dir.path().join("backups"),
            export_dir: _dir.path().join("exports"),
            import_report_dir: _dir.path().join("import-reports"),
        },
        db,
        session: Arc::new(Mutex::new(None)),
        host_service: Arc::new(Mutex::new(HostServiceRuntime::default())),
    };
    {
        let mut runtime = state.host_service.lock().unwrap();
        runtime.running = true;
        runtime.bind_address = "0.0.0.0".to_string();
        runtime.port = 17871;
        runtime.pair_code = Some("123456789012".to_string());
        runtime.clients.insert(
            "client-test".to_string(),
            ClientConnectionInfo {
                id: "client-test".to_string(),
                client_name: "测试客户端".to_string(),
                client_device_id: "device-test".to_string(),
                client_ip: "127.0.0.1".to_string(),
                app_version: "0.1.0".to_string(),
                status: "paired".to_string(),
                last_seen_at: chrono::Local::now().to_rfc3339(),
            },
        );
    }
    state
        .db
        .with_conn(|conn| repository::set_setting(conn, "runtime_mode", "client"))
        .unwrap();

    ensure_host_service_for_mode(&state, "0.1.0-test").unwrap();
    let status = get_host_service_status(&state);

    assert!(!status.running);
    assert!(status.pair_code.is_none());
    assert_eq!(status.client_count, 0);
}

#[test]
fn require_remote_permission_rejects_readonly_user() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO users (id, username, display_name, enabled)
         VALUES ('user-readonly-test', 'readonly-test', '只读测试', 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO user_roles (user_id, role_id)
         VALUES ('user-readonly-test', 'role-readonly')",
        [],
    )
    .unwrap();

    let headers = session_headers_on_conn(&conn, "device-readonly", "user-readonly-test")
        .expect("create session");
    let request = format!("GET /api/test HTTP/1.1\r\n{headers}\r\n\r\n");
    let error = require_remote_permission(&request, &conn, "write_stock").unwrap_err();
    assert!(error.to_string().contains("缺少权限：write_stock"));
}

#[test]
fn require_remote_permission_rejects_user_without_view_reports_permission() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO roles (id, code, name) VALUES ('role-no-report-test', 'no_report', '无报表权限')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO users (id, username, display_name, enabled)
         VALUES ('user-no-report-test', 'no-report-test', '无报表权限测试', 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO user_roles (user_id, role_id)
         VALUES ('user-no-report-test', 'role-no-report-test')",
        [],
    )
    .unwrap();

    let headers = session_headers_on_conn(&conn, "device-no-report", "user-no-report-test")
        .expect("create session");
    let request = format!("GET /api/test HTTP/1.1\r\n{headers}\r\n\r\n");
    let error = require_remote_permission(&request, &conn, "view_reports").unwrap_err();
    assert!(error.to_string().contains("缺少权限：view_reports"));
}

#[test]
fn require_remote_permission_allows_warehouse_user() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO users (id, username, display_name, enabled)
         VALUES ('user-warehouse-test', 'warehouse-test', '仓库测试', 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO user_roles (user_id, role_id)
         VALUES ('user-warehouse-test', 'role-warehouse')",
        [],
    )
    .unwrap();

    let headers = session_headers_on_conn(&conn, "device-warehouse", "user-warehouse-test")
        .expect("create session");
    let request = format!("GET /api/test HTTP/1.1\r\n{headers}\r\n\r\n");
    require_remote_permission(&request, &conn, "write_stock").unwrap();
}

#[test]
fn remote_department_scope_forces_department_viewer_to_bound_department() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO users (id, username, display_name, department_id, enabled)
         VALUES ('user-department-viewer-test', 'viewer-test', '部门查看员测试', 'dept-admin-office', 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO user_roles (user_id, role_id)
         VALUES ('user-department-viewer-test', 'role-department-viewer')",
        [],
    )
    .unwrap();

    let headers = session_headers_on_conn(
        &conn,
        "device-department-viewer",
        "user-department-viewer-test",
    )
    .expect("create session");
    let request = format!("GET /api/test HTTP/1.1\r\n{headers}\r\n\r\n");
    let current = require_remote_permission(&request, &conn, "view_reports").unwrap();
    let scoped_department_id = remote_department_scope(&current)
        .unwrap()
        .or(Some("dept-restaurant".to_string()));

    assert_eq!(scoped_department_id.as_deref(), Some("dept-admin-office"));
}

#[test]
fn remote_department_scope_rejects_unbound_department_viewer() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO users (id, username, display_name, enabled)
         VALUES ('user-unbound-viewer-test', 'unbound-viewer-test', '未绑定部门查看员', 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO user_roles (user_id, role_id)
         VALUES ('user-unbound-viewer-test', 'role-department-viewer')",
        [],
    )
    .unwrap();

    let headers = session_headers_on_conn(&conn, "device-unbound", "user-unbound-viewer-test")
        .expect("create session");
    let request = format!("GET /api/test HTTP/1.1\r\n{headers}\r\n\r\n");
    let current = require_remote_permission(&request, &conn, "view_reports").unwrap();
    let error = remote_department_scope(&current).unwrap_err();

    assert!(error.to_string().contains("部门查看员未绑定所属部门"));
}

#[test]
fn remote_stock_lists_force_department_viewer_scope() {
    let (_dir, db) = test_db();
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO users (id, username, display_name, department_id, enabled)
             VALUES ('user-remote-dept-scope-test', 'remote-dept-scope-test', '远程部门查看员', 'dept-admin-office', 1)",
            [],
        )?;
        conn.execute(
            "INSERT INTO user_roles (user_id, role_id)
             VALUES ('user-remote-dept-scope-test', 'role-department-viewer')",
            [],
        )?;
        conn.execute(
            "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-remote-scope-test', 'RSCOPE-001', '远程范围物品', 'unit-piece', 1)",
            [],
        )?;
        conn.execute(
            "INSERT INTO stock_documents (
               id, document_no, document_type, business_date, department_id, department_name, status
             )
             VALUES
               ('doc-remote-admin-scope', 'OUT-REMOTE-ADMIN', 'outbound', '2026-06-30', 'dept-admin-office', '行政办', 'confirmed'),
               ('doc-remote-restaurant-scope', 'OUT-REMOTE-REST', 'outbound', '2026-06-30', 'dept-restaurant', '餐饮', 'confirmed')",
            [],
        )?;
        conn.execute(
            "INSERT INTO stock_document_lines (id, document_id, item_id, quantity, unit_price, amount)
             VALUES
               ('line-remote-admin-scope', 'doc-remote-admin-scope', 'item-remote-scope-test', 1, 1, 1),
               ('line-remote-restaurant-scope', 'doc-remote-restaurant-scope', 'item-remote-scope-test', 1, 1, 1)",
            [],
        )?;
        conn.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               document_id, department_id, department_name, movement_type
             )
             VALUES
               ('mov-remote-admin-scope', '2026-06-30', 'item-remote-scope-test', 'out', 1, 1, 1, 'doc-remote-admin-scope', 'dept-admin-office', '行政办', 'outbound'),
               ('mov-remote-restaurant-scope', '2026-06-30', 'item-remote-scope-test', 'out', 1, 1, 1, 'doc-remote-restaurant-scope', 'dept-restaurant', '餐饮', 'outbound')",
            [],
        )?;
        Ok(())
    })
    .unwrap();
    let headers = session_headers(
        &db,
        "token-remote-dept-scope-test",
        "user-remote-dept-scope-test",
    )
    .expect("create session");

    let docs_response = send_test_request(
        db.clone_handle(),
        runtime_with_client("token-remote-dept-scope-test"),
        format!(
            "GET /api/stock/documents?departmentId=dept-restaurant HTTP/1.1\r\n{headers}\r\n\r\n"
        ),
    );
    let movements_response = send_test_request(
        db,
        runtime_with_client("token-remote-dept-scope-test"),
        format!(
            "GET /api/stock/movements?departmentId=dept-restaurant HTTP/1.1\r\n{headers}\r\n\r\n"
        ),
    );

    assert!(docs_response.contains("OUT-REMOTE-ADMIN"));
    assert!(!docs_response.contains("OUT-REMOTE-REST"));
    assert!(movements_response.contains("行政办"));
    assert!(!movements_response.contains("餐饮"));
}

#[test]
fn remote_status_forces_department_viewer_scope() {
    let (_dir, db) = test_db();
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO users (id, username, display_name, department_id, enabled)
             VALUES ('user-remote-status-scope-test', 'remote-status-scope-test', '远程状态查看员', 'dept-admin-office', 1)",
            [],
        )?;
        conn.execute(
            "INSERT INTO user_roles (user_id, role_id)
             VALUES ('user-remote-status-scope-test', 'role-department-viewer')",
            [],
        )?;
        conn.execute(
            "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-remote-status-scope', 'RSTAT-001', '远程状态物品', 'unit-piece', 8)",
            [],
        )?;
        conn.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               department_id, department_name, movement_type, created_at
             )
             VALUES
               ('mov-remote-status-admin', '2026-07-01', 'item-remote-status-scope', 'out', 2, 8, 16,
                'dept-admin-office', '行政办', 'outbound', '2026-07-01T10:00:00+08:00'),
               ('mov-remote-status-rest', '2026-07-01', 'item-remote-status-scope', 'out', 3, 8, 24,
                'dept-restaurant', '餐饮', 'outbound', '2026-07-01T11:00:00+08:00')",
            [],
        )?;
        Ok(())
    })
    .unwrap();

    let headers = session_headers(
        &db,
        "token-remote-status-scope-test",
        "user-remote-status-scope-test",
    )
    .expect("create session");
    let response = send_test_request(
        db,
        runtime_with_client("token-remote-status-scope-test"),
        format!("GET /api/status HTTP/1.1\r\n{headers}\r\n\r\n"),
    );

    assert!(response.contains("\"thisMonthOutboundAmount\":16"));
    assert!(response.contains("行政办"));
    assert!(!response.contains("餐饮"));
}

#[test]
fn forged_user_id_header_cannot_replace_session_token() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    let request = "GET /api/test HTTP/1.1\r\nX-Aster-Client-Token: forged-device\r\nX-Aster-User-Id: user-admin\r\n\r\n";
    let error = require_remote_admin(request, &conn).unwrap_err();
    assert!(error.to_string().contains("缺少用户会话"));
}

#[test]
fn remote_inventory_export_rejects_non_admin_user() {
    let (_dir, db) = test_db();
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO users (id, username, display_name, enabled)
             VALUES ('user-export-warehouse', 'export-warehouse', '导出仓库员', 1)",
            [],
        )?;
        conn.execute(
            "INSERT INTO user_roles (user_id, role_id)
             VALUES ('user-export-warehouse', 'role-warehouse')",
            [],
        )?;
        Ok(())
    })
    .unwrap();
    let token = "token-export-warehouse";
    let headers = session_headers(&db, token, "user-export-warehouse").unwrap();

    let response = send_test_request_bytes(
        db,
        runtime_with_client(token),
        format!("GET /api/stock/balances/export HTTP/1.1\r\n{headers}\r\n\r\n"),
    );
    let response = String::from_utf8(response).unwrap();

    assert!(response.starts_with("HTTP/1.1 403 Forbidden"));
    assert!(response.contains("需要管理员权限"));
}

#[test]
fn remote_inventory_export_returns_xlsx_and_audits_actual_admin() {
    use std::io::Cursor;

    let (_dir, db) = test_db();
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO users (id, username, display_name, enabled)
             VALUES ('user-export-admin', 'export-admin', '远程导出管理员', 1)",
            [],
        )?;
        conn.execute(
            "INSERT INTO user_roles (user_id, role_id)
             VALUES ('user-export-admin', 'role-admin')",
            [],
        )?;
        conn.execute(
            "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-remote-export', 'REMOTE-EXP', '远程导出物品', 'unit-piece', 1)",
            [],
        )?;
        Ok(())
    })
    .unwrap();
    let token = "token-export-admin";
    let headers = session_headers(&db, token, "user-export-admin").unwrap();

    let response = send_test_request_bytes(
        db.clone_handle(),
        runtime_with_client(token),
        format!("GET /api/stock/balances/export HTTP/1.1\r\n{headers}\r\n\r\n"),
    );
    let parsed = http_transport::read_xlsx_response(Cursor::new(response)).unwrap();

    assert_eq!(parsed.row_count, 1);
    assert!(parsed.body.starts_with(b"PK"));
    let audit: (String, String) = db
        .with_conn(|conn| {
            Ok(conn.query_row(
                "SELECT operator, summary FROM audit_logs
                 WHERE action = 'read_stock_balance_export'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?)
        })
        .unwrap();
    assert_eq!(audit.0, "export-admin");
    assert!(audit.1.contains("device-test"));
    assert!(audit.1.contains("1 项"));
}

#[test]
fn remote_item_list_applies_supplier_filter() {
    let (_dir, db) = test_db();
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO users (id, username, display_name, enabled)
             VALUES ('user-remote-items', 'remote-items', '远程物品查看员', 1)",
            [],
        )?;
        conn.execute(
            "INSERT INTO user_roles (user_id, role_id)
             VALUES ('user-remote-items', 'role-warehouse')",
            [],
        )?;
        conn.execute(
            "INSERT INTO suppliers (id, name) VALUES
               ('remote-supplier-a', '远程供应商 A'),
               ('remote-supplier-b', '远程供应商 B')",
            [],
        )?;
        conn.execute(
            "INSERT INTO master_items (id, code, name, supplier_id, unit_id, default_price) VALUES
               ('remote-item-a', 'REMOTE-A', '远程物品 A', 'remote-supplier-a', 'unit-piece', 1),
               ('remote-item-b', 'REMOTE-B', '远程物品 B', 'remote-supplier-b', 'unit-piece', 1)",
            [],
        )?;
        Ok(())
    })
    .unwrap();
    let token = "token-remote-items";
    let headers = session_headers(&db, token, "user-remote-items").unwrap();

    let response = send_test_request(
        db,
        runtime_with_client(token),
        format!("GET /api/master/items?supplierId=remote-supplier-a HTTP/1.1\r\n{headers}\r\n\r\n"),
    );

    assert!(response.contains("REMOTE-A"));
    assert!(!response.contains("REMOTE-B"));
}
