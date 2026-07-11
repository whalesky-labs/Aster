use super::*;

#[test]
fn remote_stock_and_stocktake_routes_reuse_service_validation() {
    let (_dir, db) = test_db();
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO users (id, username, display_name, enabled)
             VALUES ('user-remote-stock-validation-test', 'remote-stock-validation-test', '远程仓库校验员', 1)",
            [],
        )?;
        conn.execute(
            "INSERT INTO user_roles (user_id, role_id)
             VALUES ('user-remote-stock-validation-test', 'role-warehouse')",
            [],
        )?;
        Ok(())
    })
    .unwrap();
    let request_headers = session_headers(
        &db,
        "token-remote-route-validation-test",
        "user-remote-stock-validation-test",
    )
    .expect("create session");

    let stock_body = serde_json::json!({
        "documentType": "outbound",
        "businessDate": "2026-06-30",
        "departmentId": "dept-admin-office",
        "supplierId": null,
        "handler": "remote",
        "purpose": "校验",
        "remark": null,
        "approvalRequestId": null,
        "lines": []
    })
    .to_string();
    let stock_response = send_test_request(
        db.clone_handle(),
        runtime_with_client("token-remote-route-validation-test"),
        format!(
            "POST /api/stock/document HTTP/1.1\r\n{request_headers}\r\nContent-Length: {}\r\n\r\n{}",
            stock_body.len(),
            stock_body
        ),
    );
    assert!(stock_response.contains("单据至少需要一行物品"));

    let adjustment_body = serde_json::json!({
        "businessDate": "2026-06-30",
        "adjustmentType": "damage",
        "handler": "remote",
        "reason": "",
        "lines": []
    })
    .to_string();
    let adjustment_response = send_test_request(
        db.clone_handle(),
        runtime_with_client("token-remote-route-validation-test"),
        format!(
            "POST /api/stock/adjustment HTTP/1.1\r\n{request_headers}\r\nContent-Length: {}\r\n\r\n{}",
            adjustment_body.len(),
            adjustment_body
        ),
    );
    assert!(adjustment_response.contains("调整原因不能为空"));

    let stocktake_body = serde_json::json!({
        "stocktakeId": "stocktake-test",
        "lines": []
    })
    .to_string();
    let stocktake_response = send_test_request(
        db.clone_handle(),
        runtime_with_client("token-remote-route-validation-test"),
        format!(
            "POST /api/stocktake/counts HTTP/1.1\r\n{request_headers}\r\nContent-Length: {}\r\n\r\n{}",
            stocktake_body.len(),
            stocktake_body
        ),
    );
    assert!(stocktake_response.contains("至少需要提交一行盘点数量"));
}

#[test]
fn remote_master_data_routes_reuse_service_validation() {
    let (_dir, db) = test_db();
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO users (id, username, display_name, enabled)
             VALUES ('user-remote-master-validation-test', 'remote-master-validation-test', '远程资料校验员', 1)",
            [],
        )?;
        conn.execute(
            "INSERT INTO user_roles (user_id, role_id)
             VALUES ('user-remote-master-validation-test', 'role-warehouse')",
            [],
        )?;
        Ok(())
    })
    .unwrap();
    let headers = session_headers(
        &db,
        "token-remote-master-validation-test",
        "user-remote-master-validation-test",
    )
    .expect("create session");

    let unit_body = serde_json::json!({
        "id": null,
        "expectedUpdatedAt": null,
        "name": "",
        "enabled": true,
        "sortOrder": 1
    })
    .to_string();
    let unit_response = send_test_request(
        db.clone_handle(),
        runtime_with_client("token-remote-master-validation-test"),
        format!(
            "POST /api/master/unit HTTP/1.1\r\n{headers}\r\nContent-Length: {}\r\n\r\n{}",
            unit_body.len(),
            unit_body
        ),
    );
    assert!(unit_response.contains("单位名称不能为空"));

    let item_body = serde_json::json!({
        "id": null,
        "expectedUpdatedAt": null,
        "code": "BAD-PRICE",
        "barcode": null,
        "name": "负价格物品",
        "categoryId": null,
        "spec": null,
        "unitId": null,
        "defaultPrice": -1,
        "salePrice": 0,
        "supplierId": null,
        "warningQuantity": 0,
        "enabled": true,
        "remark": null
    })
    .to_string();
    let item_response = send_test_request(
        db.clone_handle(),
        runtime_with_client("token-remote-master-validation-test"),
        format!(
            "POST /api/master/item HTTP/1.1\r\n{headers}\r\nContent-Length: {}\r\n\r\n{}",
            item_body.len(),
            item_body
        ),
    );
    assert!(item_response.contains("参考进价不能小于 0"));

    let (unit_count, item_count): (i64, i64) = db
        .with_conn(|conn| {
            Ok((
                conn.query_row("SELECT COUNT(*) FROM units WHERE name = ''", [], |row| {
                    row.get(0)
                })?,
                conn.query_row(
                    "SELECT COUNT(*) FROM master_items WHERE code = 'BAD-PRICE'",
                    [],
                    |row| row.get(0),
                )?,
            ))
        })
        .unwrap();
    assert_eq!(unit_count, 0);
    assert_eq!(item_count, 0);
}

#[test]
fn remote_budget_route_reuses_service_validation() {
    let (_dir, db) = test_db();
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO users (id, username, display_name, enabled)
             VALUES ('user-remote-budget-admin-test', 'remote-budget-admin-test', '远程预算管理员', 1)",
            [],
        )?;
        conn.execute(
            "INSERT INTO user_roles (user_id, role_id)
             VALUES ('user-remote-budget-admin-test', 'role-admin')",
            [],
        )?;
        Ok(())
    })
    .unwrap();
    let body = serde_json::json!({
        "id": null,
        "expectedUpdatedAt": null,
        "departmentId": "dept-admin-office",
        "categoryId": "cat-consumables",
        "periodMonth": "2026-06",
        "amountLimit": -1,
        "enabled": true
    })
    .to_string();
    let headers = session_headers(
        &db,
        "token-remote-budget-validation-test",
        "user-remote-budget-admin-test",
    )
    .expect("create session");

    let response = send_test_request(
        db,
        runtime_with_client("token-remote-budget-validation-test"),
        format!(
            "POST /api/master/budget-rule HTTP/1.1\r\n{headers}\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        ),
    );

    assert!(response.contains("预算金额不能小于 0"));
}

#[test]
fn local_host_management_requires_admin_session() {
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

    let list_error = list_client_connections(&state).unwrap_err();
    assert!(list_error.to_string().contains("请先登录管理员账号"));
}

#[test]
fn unauthenticated_client_bootstrap_can_save_host_config() {
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

    let config = save_client_config(
        &state,
        SaveClientConfigRequest {
            host_address: "127.0.0.1".to_string(),
            host_port: 17871,
        },
    )
    .unwrap();

    assert_eq!(config.mode, RuntimeMode::Client);
    assert_eq!(config.host_address.as_deref(), Some("127.0.0.1"));
}

#[test]
fn unauthenticated_host_mode_cannot_be_reconfigured_as_client() {
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
    state
        .db
        .with_conn(|conn| repository::set_setting(conn, "runtime_mode", "host"))
        .unwrap();

    let save_error = save_client_config(
        &state,
        SaveClientConfigRequest {
            host_address: "127.0.0.1".to_string(),
            host_port: 17871,
        },
    )
    .unwrap_err();
    assert!(save_error.to_string().contains("请先登录管理员账号"));
}

#[test]
fn save_client_config_clears_pairing_token_when_host_changes() {
    let (_dir, db) = test_db();
    let state = admin_state(&_dir, db);
    state
        .db
        .with_conn(|conn| {
            repository::set_setting(conn, "host_address", "192.168.1.10")?;
            repository::set_setting(conn, "host_port", "17871")?;
            repository::set_setting(conn, "client_token", "old-token")
        })
        .unwrap();

    save_client_config(
        &state,
        SaveClientConfigRequest {
            host_address: "192.168.1.10".to_string(),
            host_port: 17871,
        },
    )
    .unwrap();
    let same_host_token = state
        .db
        .with_conn(|conn| repository::get_setting(conn, "client_token"))
        .unwrap();
    assert_eq!(same_host_token.as_deref(), Some("old-token"));

    let config = save_client_config(
        &state,
        SaveClientConfigRequest {
            host_address: "192.168.1.20".to_string(),
            host_port: 17871,
        },
    )
    .unwrap();

    assert_eq!(config.host_address.as_deref(), Some("192.168.1.20"));
    assert!(config.client_token.is_none());
    let cleared_token = state
        .db
        .with_conn(|conn| repository::get_setting(conn, "client_token"))
        .unwrap();
    assert!(cleared_token.is_none());
}
