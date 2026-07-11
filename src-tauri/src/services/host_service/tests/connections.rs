use super::*;

#[test]
fn parse_request_line_extracts_method_and_path() {
    let (method, path) = http_transport::request_line("GET /api/health HTTP/1.1\r\n\r\n");
    assert_eq!(method, "GET");
    assert_eq!(path, "/api/health");
}

#[test]
fn query_param_decodes_percent_encoded_text() {
    let encoded = url_encode("牙刷 测试");
    let path = format!("/api/stock/balances?search={encoded}");
    assert_eq!(query_param(&path, "search"), Some("牙刷 测试".to_string()));
}

#[test]
fn host_connection_validation_rejects_common_operator_input_errors() {
    assert_eq!(
        normalize_host_address(" 192.168.1.10 ").unwrap(),
        "192.168.1.10"
    );
    assert!(normalize_host_address("")
        .unwrap_err()
        .to_string()
        .contains("不能为空"));
    assert!(normalize_host_address("http://192.168.1.10:17871")
        .unwrap_err()
        .to_string()
        .contains("不要包含"));
    assert!(normalize_host_address("192.168.1.10/path")
        .unwrap_err()
        .to_string()
        .contains("不要包含"));
    assert!(validate_host_port(80)
        .unwrap_err()
        .to_string()
        .contains("1024-65535"));
    validate_host_port(17871).unwrap();
}

#[test]
fn pairing_validation_requires_twelve_digit_code_and_client_identity() {
    validate_pairing_request("123456789012", "前台电脑", "device-frontdesk").unwrap();
    assert!(
        validate_pairing_request("12345", "前台电脑", "device-frontdesk")
            .unwrap_err()
            .to_string()
            .contains("12 位数字")
    );
    assert!(
        validate_pairing_request("abcdef", "前台电脑", "device-frontdesk")
            .unwrap_err()
            .to_string()
            .contains("12 位数字")
    );
    assert!(
        validate_pairing_request("123456789012", " ", "device-frontdesk")
            .unwrap_err()
            .to_string()
            .contains("客户端名称不能为空")
    );
    assert!(validate_pairing_request("123456789012", "前台电脑", " ")
        .unwrap_err()
        .to_string()
        .contains("设备 ID 不能为空"));
}

#[test]
fn client_connections_are_persisted_and_touch_updates_status() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    let mut client = ClientConnectionInfo {
        id: "client-db-id".to_string(),
        client_name: "前台电脑".to_string(),
        client_device_id: "device-frontdesk".to_string(),
        client_ip: "192.168.1.20".to_string(),
        app_version: "0.1.0".to_string(),
        status: "paired".to_string(),
        last_seen_at: "2026-06-30T10:00:00+08:00".to_string(),
    };

    upsert_client_connection(&conn, &client, &token_hash("persisted-token")).unwrap();
    client.id = "client-db-id-new".to_string();
    client.client_name = "前台电脑重配".to_string();
    client.client_ip = "192.168.1.21".to_string();
    upsert_client_connection(&conn, &client, &token_hash("new-persisted-token")).unwrap();
    touch_client_connection(&conn, "device-frontdesk", "online").unwrap();
    let clients = crate::db::client_connection_repository::list(&conn).unwrap();

    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].id, "client-db-id-new");
    assert_eq!(clients[0].client_name, "前台电脑重配");
    assert_eq!(clients[0].client_device_id, "device-frontdesk");
    assert_eq!(clients[0].status, "online");
    assert!(
        find_client_connection_by_token_hash(&conn, &token_hash("persisted-token"))
            .unwrap()
            .is_none()
    );
    assert!(
        find_client_connection_by_token_hash(&conn, &token_hash("new-persisted-token"))
            .unwrap()
            .is_some()
    );
}

#[test]
fn list_client_connections_reads_persisted_host_records() {
    let (_dir, db) = test_db();
    let state = admin_state(&_dir, db);
    state
        .db
        .with_conn(|conn| {
            upsert_client_connection(
                conn,
                &ClientConnectionInfo {
                    id: "persisted-client".to_string(),
                    client_name: "客房电脑".to_string(),
                    client_device_id: "device-housekeeping".to_string(),
                    client_ip: "192.168.1.30".to_string(),
                    app_version: "0.1.0".to_string(),
                    status: "paired".to_string(),
                    last_seen_at: "2026-06-30T11:00:00+08:00".to_string(),
                },
                &token_hash("persisted-token"),
            )
        })
        .unwrap();

    let clients = list_client_connections(&state).unwrap();

    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].id, "persisted-client");
    assert_eq!(clients[0].client_name, "客房电脑");
}

#[test]
fn remove_client_connection_revokes_token_and_runtime_client() {
    let (_dir, db) = test_db();
    let state = admin_state(&_dir, db);
    state
        .db
        .with_conn(|conn| {
            upsert_client_connection(
                conn,
                &ClientConnectionInfo {
                    id: "persisted-client".to_string(),
                    client_name: "前台电脑".to_string(),
                    client_device_id: "device-frontdesk".to_string(),
                    client_ip: "192.168.1.20".to_string(),
                    app_version: "0.1.0".to_string(),
                    status: "paired".to_string(),
                    last_seen_at: "2026-06-30T10:00:00+08:00".to_string(),
                },
                &token_hash("revoked-token"),
            )
        })
        .unwrap();
    {
        let mut runtime = state.host_service.lock().unwrap();
        runtime.clients.insert(
            "revoked-token".to_string(),
            ClientConnectionInfo {
                id: "revoked-token".to_string(),
                client_name: "前台电脑".to_string(),
                client_device_id: "device-frontdesk".to_string(),
                client_ip: "192.168.1.20".to_string(),
                app_version: "0.1.0".to_string(),
                status: "online".to_string(),
                last_seen_at: "2026-06-30T10:00:00+08:00".to_string(),
            },
        );
    }

    remove_client_connection(
        &state,
        RemoveClientConnectionRequest {
            client_device_id: "device-frontdesk".to_string(),
        },
    )
    .unwrap();

    let clients = list_client_connections(&state).unwrap();
    assert!(clients.is_empty());
    let removed_token = state
        .db
        .with_conn(|conn| find_client_connection_by_token_hash(conn, &token_hash("revoked-token")))
        .unwrap();
    assert!(removed_token.is_none());
    let runtime = state.host_service.lock().unwrap();
    assert!(!runtime.clients.contains_key("revoked-token"));
    drop(runtime);
    let audit_count: i64 = state
        .db
        .with_conn(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM audit_logs
                 WHERE action = 'remove_client_connection'
                   AND entity_type = 'client_connection'
                   AND entity_id = 'device-frontdesk'",
                [],
                |row| row.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(audit_count, 1);
}

#[test]
fn persisted_client_token_survives_host_runtime_restart() {
    let (_dir, db) = test_db();
    db.with_conn(|conn| {
        upsert_client_connection(
            conn,
            &ClientConnectionInfo {
                id: "persisted-client".to_string(),
                client_name: "前台电脑".to_string(),
                client_device_id: "device-frontdesk".to_string(),
                client_ip: "192.168.1.20".to_string(),
                app_version: "0.1.0".to_string(),
                status: "paired".to_string(),
                last_seen_at: "2026-06-30T10:00:00+08:00".to_string(),
            },
            &token_hash("survives-restart-token"),
        )
    })
    .unwrap();
    let runtime = Arc::new(Mutex::new(HostServiceRuntime::default()));

    authenticate_request_and_touch_client(
        "GET /api/status HTTP/1.1\r\nX-Aster-Client-Token: survives-restart-token\r\n\r\n",
        &runtime,
        &db,
    )
    .unwrap();

    let restored = runtime.lock().unwrap();
    let client = restored
        .clients
        .values()
        .find(|client| client.client_device_id == "device-frontdesk")
        .expect("restored runtime client");
    assert_eq!(client.id, "survives-restart-token");
    assert_eq!(client.status, "online");
}

#[test]
fn write_host_audit_marks_remote_operator_as_client() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();

    write_host_audit(&conn, "save_item", "item", "item-1", "远程保存").unwrap();

    let operator: String = conn
        .query_row(
            "SELECT operator FROM audit_logs WHERE action = 'save_item'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(operator, "client");
}

#[test]
fn status_endpoint_requires_remote_current_user() {
    let (_dir, db) = test_db();
    let response = send_test_request(
        db,
        runtime_with_client("token-status-test"),
        "GET /api/status HTTP/1.1\r\nX-Aster-Client-Token: token-status-test\r\n\r\n".to_string(),
    );

    assert!(response.contains("远程请求缺少用户会话"));
}

#[test]
fn system_settings_endpoint_requires_remote_admin() {
    let (_dir, db) = test_db();
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO users (id, username, display_name, enabled)
             VALUES ('user-settings-readonly-test', 'settings-readonly-test', '设置只读测试', 1)",
            [],
        )?;
        conn.execute(
            "INSERT INTO user_roles (user_id, role_id)
             VALUES ('user-settings-readonly-test', 'role-readonly')",
            [],
        )?;
        Ok(())
    })
    .unwrap();

    let headers = session_headers(&db, "token-settings-test", "user-settings-readonly-test")
        .expect("create session");
    let response = send_test_request(
        db,
        runtime_with_client("token-settings-test"),
        format!("GET /api/system-settings HTTP/1.1\r\n{headers}\r\n\r\n"),
    );

    assert!(response.contains("需要管理员权限"));
}

#[test]
fn master_data_endpoint_requires_remote_current_user() {
    let (_dir, db) = test_db();
    let response = send_test_request(
        db,
        runtime_with_client("token-master-test"),
        "GET /api/master/items HTTP/1.1\r\nX-Aster-Client-Token: token-master-test\r\n\r\n"
            .to_string(),
    );

    assert!(response.contains("远程请求缺少用户会话"));
}

#[test]
fn remote_approval_api_validates_entity_and_binds_remote_users() {
    let (_dir, db) = test_db();
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO users (id, username, display_name, enabled)
             VALUES ('user-approval-warehouse-test', 'approval-warehouse-test', '审批仓库员', 1)",
            [],
        )?;
        conn.execute(
            "INSERT INTO user_roles (user_id, role_id)
             VALUES ('user-approval-warehouse-test', 'role-warehouse')",
            [],
        )?;
        conn.execute(
            "INSERT INTO users (id, username, display_name, enabled)
             VALUES ('user-approval-admin-test', 'approval-admin-test', '审批管理员', 1)",
            [],
        )?;
        conn.execute(
            "INSERT INTO user_roles (user_id, role_id)
             VALUES ('user-approval-admin-test', 'role-admin')",
            [],
        )?;
        Ok(())
    })
    .unwrap();

    let invalid_body = serde_json::json!({
        "entityType": "budget_override",
        "entityId": "dept-admin-office:2026-13",
        "reason": "错误月份"
    })
    .to_string();
    let invalid_headers = session_headers(
        &db,
        "token-approval-invalid-test",
        "user-approval-warehouse-test",
    )
    .expect("create session");
    let invalid_request = format!(
        "POST /api/approval HTTP/1.1\r\n{invalid_headers}\r\nContent-Length: {}\r\n\r\n{}",
        invalid_body.len(),
        invalid_body
    );
    let invalid_response = send_test_request(
        db.clone_handle(),
        runtime_with_client("token-approval-invalid-test"),
        invalid_request,
    );
    assert!(invalid_response.contains("YYYY-MM"));

    let create_body = serde_json::json!({
        "entityType": "budget_override",
        "entityId": "dept-admin-office:2026-06",
        "reason": "远程超预算领用"
    })
    .to_string();
    let create_headers = session_headers(
        &db,
        "token-approval-create-test",
        "user-approval-warehouse-test",
    )
    .expect("create session");
    let create_request = format!(
        "POST /api/approval HTTP/1.1\r\n{create_headers}\r\nContent-Length: {}\r\n\r\n{}",
        create_body.len(),
        create_body
    );
    let create_response = send_test_request(
        db.clone_handle(),
        runtime_with_client("token-approval-create-test"),
        create_request,
    );
    assert!(create_response.contains("\"entityType\":\"budget_override\""));
    let approval_id: String = db
        .with_conn(|conn| {
            conn.query_row(
                "SELECT id FROM approval_requests
                 WHERE requested_by = 'user-approval-warehouse-test'",
                [],
                |row| row.get(0),
            )
            .map_err(Into::into)
        })
        .unwrap();

    let decide_body = serde_json::json!({
        "approvalId": approval_id,
        "approve": true,
        "decisionNote": "远程通过"
    })
    .to_string();
    let decide_headers = session_headers(
        &db,
        "token-approval-decide-test",
        "user-approval-admin-test",
    )
    .expect("create session");
    let decide_request = format!(
        "POST /api/approval/decision HTTP/1.1\r\n{decide_headers}\r\nContent-Length: {}\r\n\r\n{}",
        decide_body.len(),
        decide_body
    );
    let decide_response = send_test_request(
        db.clone_handle(),
        runtime_with_client("token-approval-decide-test"),
        decide_request,
    );
    assert!(decide_response.contains("\"status\":\"approved\""));

    let (requested_by, decided_by, audit_count): (String, String, i64) = db
        .with_conn(|conn| {
            let users = conn.query_row(
                "SELECT requested_by, decided_by
                 FROM approval_requests
                 WHERE id = ?1",
                [approval_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?;
            let audit_count = conn.query_row(
                "SELECT COUNT(*) FROM audit_logs
                 WHERE entity_type = 'approval' AND operator = 'client'",
                [],
                |row| row.get(0),
            )?;
            Ok((users.0, users.1, audit_count))
        })
        .unwrap();
    assert_eq!(requested_by, "user-approval-warehouse-test");
    assert_eq!(decided_by, "user-approval-admin-test");
    assert_eq!(audit_count, 2);
}
