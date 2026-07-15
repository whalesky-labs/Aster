use std::sync::{Arc, Mutex};

use crate::domain::status::SaveSystemSettingsRequest;
use crate::domain::users::{CurrentUser, Role};
use crate::{app::paths::AppPaths, app::state::AppState, db::connection::Db};

use super::*;

fn test_state() -> AppState {
    let dir = tempfile::tempdir().expect("temp dir").keep();
    let paths = AppPaths {
        data_dir: dir.to_path_buf(),
        database_path: dir.join("aster.sqlite"),
        backup_dir: dir.join("backups"),
        export_dir: dir.join("exports"),
        import_report_dir: dir.join("import-reports"),
    };
    std::fs::create_dir_all(&paths.backup_dir).unwrap();
    std::fs::create_dir_all(&paths.export_dir).unwrap();
    std::fs::create_dir_all(&paths.import_report_dir).unwrap();
    AppState {
        db: Db::initialize(&paths).unwrap(),
        paths,
        session: Arc::new(Mutex::new(None)),
        host_service: Arc::new(Mutex::new(
            crate::services::host_service::HostServiceRuntime::default(),
        )),
    }
}

fn set_admin_user(state: &AppState) {
    let user = CurrentUser {
        id: "user-admin".to_string(),
        username: "admin".to_string(),
        display_name: "管理员".to_string(),
        department_id: None,
        department_name: None,
        roles: vec![Role {
            id: "role-admin".to_string(),
            code: "admin".to_string(),
            name: "管理员".to_string(),
        }],
        permissions: vec![
            "dangerous_operations".to_string(),
            "manage_settings".to_string(),
        ],
    };
    crate::services::test_support::install_session(state, user).unwrap();
}

#[test]
fn runtime_config_migrates_legacy_pairing_token_without_serializing_it() {
    let state = test_state();
    state
        .db
        .with_conn(|conn| {
            repository::set_setting(conn, "runtime_mode", "client")?;
            repository::set_setting(conn, "client_token", "legacy-token")
        })
        .unwrap();

    let runtime = get_runtime_config(&state).unwrap();
    assert_eq!(runtime.client_token.as_deref(), Some("legacy-token"));
    assert!(runtime.client_paired);
    let serialized = serde_json::to_string(&runtime).unwrap();
    assert!(!serialized.contains("legacy-token"));
    assert!(!serialized.contains("clientToken"));
    let legacy = state
        .db
        .with_conn(|conn| repository::get_setting(conn, "client_token"))
        .unwrap();
    assert!(legacy.is_none());
}

#[test]
fn standalone_runtime_does_not_wait_for_an_unused_pairing_credential() {
    let state = test_state();
    state
        .db
        .with_conn(|conn| repository::set_setting(conn, "client_token", "legacy-token"))
        .unwrap();

    let runtime = get_runtime_config(&state).unwrap();
    assert!(runtime.client_token.is_none());
    assert!(!runtime.client_paired);
    let legacy = state
        .db
        .with_conn(|conn| repository::get_setting(conn, "client_token"))
        .unwrap();
    assert_eq!(legacy.as_deref(), Some("legacy-token"));
}

#[test]
fn save_system_settings_persists_values_and_writes_audit_log() {
    let state = test_state();
    set_admin_user(&state);
    let export_dir = state.paths.data_dir.join("custom-exports");
    let backup_dir = state.paths.data_dir.join("custom-backups");

    let settings = save_system_settings(
        &state,
        SaveSystemSettingsRequest {
            hotel_name: "测试酒店".to_string(),
            current_period: "2026-06".to_string(),
            default_month: "2026-07".to_string(),
            allow_negative_stock: true,
            quantity_decimals: 3,
            amount_decimals: 2,
            default_export_dir: export_dir.display().to_string(),
            default_backup_dir: backup_dir.display().to_string(),
            auto_backup_enabled: false,
            interval_backup_enabled: true,
            interval_backup_hours: 2,
            smtp_enabled: false,
            smtp_host: String::new(),
            smtp_port: 465,
            smtp_username: String::new(),
            smtp_password: None,
            smtp_from_email: String::new(),
            smtp_from_name: "Aster".to_string(),
        },
    )
    .unwrap();

    assert_eq!(settings.hotel_name, "测试酒店");
    assert!(settings.allow_negative_stock);
    assert_eq!(
        settings.default_export_dir,
        export_dir.display().to_string()
    );
    assert_eq!(
        settings.default_backup_dir,
        backup_dir.display().to_string()
    );
    assert!(!settings.auto_backup_enabled);

    let audit_count: i64 = state
        .db
        .with_conn(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM audit_logs WHERE action = 'save_system_settings'",
                [],
                |row| row.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(audit_count, 1);
}

#[test]
fn save_system_settings_keeps_smtp_password_out_of_sqlite() {
    let state = test_state();
    set_admin_user(&state);
    let settings = save_system_settings(
        &state,
        SaveSystemSettingsRequest {
            hotel_name: "安全酒店".to_string(),
            current_period: "2026-07".to_string(),
            default_month: "2026-07".to_string(),
            allow_negative_stock: false,
            quantity_decimals: 2,
            amount_decimals: 2,
            default_export_dir: state.paths.export_dir.display().to_string(),
            default_backup_dir: state.paths.backup_dir.display().to_string(),
            auto_backup_enabled: true,
            interval_backup_enabled: true,
            interval_backup_hours: 6,
            smtp_enabled: true,
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 465,
            smtp_username: "mailer".to_string(),
            smtp_password: Some("smtp-secret".to_string()),
            smtp_from_email: "mailer@example.com".to_string(),
            smtp_from_name: "Aster".to_string(),
        },
    )
    .unwrap();
    assert!(settings.smtp_password_configured);
    let sqlite_password = state
        .db
        .with_conn(|conn| repository::get_setting(conn, "smtp_password"))
        .unwrap();
    assert!(sqlite_password.is_none());
    let secure_password = crate::application::secret_service::load(
        &state.db,
        crate::application::secret_service::ApplicationSecret::SmtpPassword,
    )
    .unwrap();
    assert_eq!(secure_password.as_deref(), Some("smtp-secret"));
}

#[test]
fn save_system_settings_in_client_mode_only_persists_local_directories() {
    let state = test_state();
    set_admin_user(&state);
    let export_dir = state.paths.data_dir.join("client-exports");
    let backup_dir = state.paths.data_dir.join("client-backups");
    state
        .db
        .with_conn(|conn| repository::set_setting(conn, "runtime_mode", "client"))
        .unwrap();

    let settings = save_system_settings(
        &state,
        SaveSystemSettingsRequest {
            hotel_name: "客户端酒店".to_string(),
            current_period: "2026-06".to_string(),
            default_month: "2026-07".to_string(),
            allow_negative_stock: false,
            quantity_decimals: 2,
            amount_decimals: 2,
            default_export_dir: export_dir.display().to_string(),
            default_backup_dir: backup_dir.display().to_string(),
            auto_backup_enabled: true,
            interval_backup_enabled: true,
            interval_backup_hours: 6,
            smtp_enabled: false,
            smtp_host: String::new(),
            smtp_port: 465,
            smtp_username: String::new(),
            smtp_password: None,
            smtp_from_email: String::new(),
            smtp_from_name: "Aster".to_string(),
        },
    )
    .unwrap();

    assert_eq!(settings.hotel_name, "Aster Hotel");
    assert_eq!(
        settings.default_export_dir,
        export_dir.display().to_string()
    );
    assert_eq!(
        settings.default_backup_dir,
        backup_dir.display().to_string()
    );
    state
        .db
        .with_conn(|conn| {
            let hotel_name = repository::get_setting(conn, "hotel_name")?;
            let audit_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM audit_logs WHERE action = 'save_local_directory_settings'",
                [],
                |row| row.get(0),
            )?;
            assert_ne!(hotel_name.as_deref(), Some("客户端酒店"));
            assert_eq!(audit_count, 1);
            Ok(())
        })
        .unwrap();
}

#[test]
fn list_audit_logs_requires_admin_and_returns_recent_rows() {
    let state = test_state();
    set_admin_user(&state);
    state
            .db
            .with_conn(|conn| {
                Ok(conn.execute(
                    "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator, created_at)
                     VALUES ('audit-service', 'save_item', 'item', 'item-1', '服务层查询', 'admin', '2026-06-30T10:00:00+08:00')",
                    [],
                )?)
            })
            .unwrap();

    let rows = list_audit_logs(&state, Some(20)).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "audit-service");
    assert_eq!(rows[0].operator, "admin");
}

#[test]
fn system_settings_from_conn_reads_stored_values_without_local_fallback() {
    let state = test_state();
    state
        .db
        .with_conn(|conn| {
            repository::set_setting(conn, "hotel_name", "主机酒店")?;
            repository::set_setting(conn, "current_period", "2026-06")?;
            repository::set_setting(conn, "default_month", "2026-07")?;
            repository::set_setting(conn, "default_export_dir", "/host/export")?;
            repository::set_setting(conn, "default_backup_dir", "/host/backup")?;
            system_settings_from_conn(conn, None)
        })
        .map(|settings| {
            assert_eq!(settings.hotel_name, "主机酒店");
            assert_eq!(settings.current_period, "2026-06");
            assert_eq!(settings.default_export_dir, "/host/export");
            assert_eq!(settings.default_backup_dir, "/host/backup");
        })
        .unwrap();
}

#[test]
fn update_settings_snapshot_restores_missing_business_directory_settings() {
    let state = test_state();
    set_admin_user(&state);
    let export_dir = state.paths.data_dir.join("snapshot-exports");
    let backup_dir = state.paths.data_dir.join("snapshot-backups");
    fs::create_dir_all(&export_dir).unwrap();
    fs::create_dir_all(&backup_dir).unwrap();

    state
        .db
        .with_conn(|conn| {
            repository::set_setting(conn, "hotel_name", "升级前酒店")?;
            repository::set_setting(conn, "current_period", "2026-07")?;
            repository::set_setting(conn, "default_month", "2026-07")?;
            repository::set_setting(
                conn,
                "default_export_dir",
                &export_dir.display().to_string(),
            )?;
            repository::set_setting(
                conn,
                "default_backup_dir",
                &backup_dir.display().to_string(),
            )?;
            repository::set_setting(conn, "allow_negative_stock", "true")?;
            repository::set_setting(conn, "interval_backup_hours", "12")
        })
        .unwrap();

    prepare_update_settings_snapshot(&state).unwrap();
    state
        .db
        .with_conn(|conn| {
            repository::delete_setting(conn, "hotel_name")?;
            repository::delete_setting(conn, "default_export_dir")?;
            repository::delete_setting(conn, "default_backup_dir")?;
            repository::delete_setting(conn, "allow_negative_stock")
        })
        .unwrap();

    restore_update_settings_snapshot_if_needed(&state).unwrap();

    let expected_export_dir = export_dir.display().to_string();
    let expected_backup_dir = backup_dir.display().to_string();
    state
        .db
        .with_conn(|conn| {
            assert_eq!(
                repository::get_setting(conn, "hotel_name")?.as_deref(),
                Some("升级前酒店")
            );
            assert_eq!(
                repository::get_setting(conn, "default_export_dir")?.as_deref(),
                Some(expected_export_dir.as_str())
            );
            assert_eq!(
                repository::get_setting(conn, "default_backup_dir")?.as_deref(),
                Some(expected_backup_dir.as_str())
            );
            assert_eq!(
                repository::get_setting(conn, "allow_negative_stock")?.as_deref(),
                Some("true")
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn build_app_status_uses_database_metrics_and_recent_operations() {
    let state = test_state();
    let runtime = get_runtime_config(&state).unwrap();
    state
        .db
        .with_conn(|conn| {
            conn.execute(
                "INSERT INTO master_items (id, code, name, unit_id, default_price)
                     VALUES ('item-status', 'STA-001', '状态物品', 'unit-piece', 8)",
                [],
            )?;
            conn.execute(
                "INSERT INTO stock_balances (item_id, quantity, amount, average_price, updated_at)
                     VALUES ('item-status', -2, -16, 8, '2026-06-30T10:00:00+08:00')",
                [],
            )?;
            conn.execute(
                "INSERT INTO stock_movements (
                       id, movement_date, item_id, direction, quantity, unit_price, amount,
                       department_id, movement_type, created_at
                     )
                     VALUES (
                       'mov-status', '2026-06-30', 'item-status', 'out', 2, 8, 16,
                       'dept-admin-office', 'outbound', '2026-06-30T10:00:00+08:00'
                     )",
                [],
            )?;
            build_app_status(conn, "0.1.0-test", Some(runtime), None)
        })
        .map(|status| {
            assert_eq!(status.metrics.item_count, 1);
            assert_eq!(status.metrics.current_stock_amount, -16.0);
            assert_eq!(status.recent_operations.len(), 1);
            assert_eq!(status.recent_operations[0].item_name, "状态物品");
            assert!(status.health.database_ok);
            assert!(status.health.stock_balance_consistency_ok);
        })
        .unwrap();
}

#[test]
fn build_app_status_scopes_department_viewer_recent_operations_and_outbound_amount() {
    let state = test_state();
    let runtime = get_runtime_config(&state).unwrap();
    let status = state
            .db
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO master_items (id, code, name, unit_id, default_price)
                     VALUES ('item-status-scope', 'STS-001', '状态范围物品', 'unit-piece', 8)",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO stock_movements (
                       id, movement_date, item_id, direction, quantity, unit_price, amount,
                       department_id, department_name, movement_type, created_at
                     )
                     VALUES
                       ('mov-status-admin-scope', '2026-07-01', 'item-status-scope', 'out', 2, 8, 16,
                        'dept-admin-office', '行政办', 'outbound', '2026-07-01T10:00:00+08:00'),
                       ('mov-status-restaurant-scope', '2026-07-01', 'item-status-scope', 'out', 3, 8, 24,
                        'dept-restaurant', '餐饮', 'outbound', '2026-07-01T11:00:00+08:00')",
                    [],
                )?;
                build_app_status(
                    conn,
                    "0.1.0-test",
                    Some(runtime),
                    Some("dept-admin-office"),
                )
            })
            .unwrap();

    assert_eq!(status.metrics.this_month_outbound_amount, 16.0);
    assert_eq!(status.recent_operations.len(), 1);
    assert_eq!(
        status.recent_operations[0].department_name.as_deref(),
        Some("行政办")
    );
}

#[test]
fn build_app_status_marks_stock_balance_mismatch_unhealthy() {
    let state = test_state();
    let runtime = get_runtime_config(&state).unwrap();
    let status = state
        .db
        .with_conn(|conn| {
            conn.execute(
                "INSERT INTO master_items (id, code, name, unit_id, default_price)
                     VALUES ('item-mismatch', 'MIS-001', '异常物品', 'unit-piece', 8)",
                [],
            )?;
            conn.execute(
                "INSERT INTO stock_balances (item_id, quantity, amount, average_price, updated_at)
                     VALUES ('item-mismatch', 5, 40, 8, '2026-06-30T10:00:00+08:00')",
                [],
            )?;
            conn.execute(
                "INSERT INTO stock_movements (
                       id, movement_date, item_id, direction, quantity, unit_price, amount,
                       movement_type, created_at
                     )
                     VALUES (
                       'mov-mismatch', '2026-06-30', 'item-mismatch', 'in', 4, 8, 32,
                       'inbound', '2026-06-30T10:00:00+08:00'
                     )",
                [],
            )?;
            build_app_status(conn, "0.1.0-test", Some(runtime), None)
        })
        .unwrap();

    assert!(!status.health.database_ok);
    assert!(!status.health.stock_balance_consistency_ok);
    assert_eq!(status.health.stock_balance_issue_count, 1);
    assert!(status
        .health
        .message
        .contains("库存余额与流水存在 1 项不一致"));
}

#[test]
fn get_runtime_config_generates_stable_client_device_id() {
    let state = test_state();

    let first = get_runtime_config(&state).unwrap();
    let second = get_runtime_config(&state).unwrap();
    let stored = state
        .db
        .with_conn(|conn| repository::get_setting(conn, CLIENT_DEVICE_ID_KEY))
        .unwrap();

    assert!(first.client_device_id.starts_with("device-"));
    assert_eq!(first.client_device_id, second.client_device_id);
    assert_eq!(stored.as_deref(), Some(first.client_device_id.as_str()));
}

#[test]
fn client_mode_status_without_login_uses_local_shell_status() {
    let state = test_state();
    state
        .db
        .with_conn(|conn| {
            repository::set_setting(conn, "runtime_mode", RuntimeMode::Client.as_str())?;
            repository::set_setting(conn, "host_address", "127.0.0.1")?;
            repository::set_setting(conn, "host_port", "17871")
        })
        .unwrap();

    let status = get_app_status(&state, "0.1.0-test").unwrap();

    assert_eq!(status.runtime.mode, RuntimeMode::Client);
    assert_eq!(status.runtime.host_address.as_deref(), Some("127.0.0.1"));
    assert_eq!(status.metrics.item_count, 0);
    assert!(status.health.message.contains("未登录"));
    assert!(status.health.message.contains("本机连接配置"));
}

#[test]
fn client_mode_status_with_unreachable_host_falls_back_to_local_shell_status() {
    let state = test_state();
    *state.session.lock().expect("session mutex poisoned") = Some(CurrentUser {
        id: "user-admin".to_string(),
        username: "admin".to_string(),
        display_name: "管理员".to_string(),
        department_id: None,
        department_name: None,
        roles: vec![Role {
            id: "role-admin".to_string(),
            code: "admin".to_string(),
            name: "管理员".to_string(),
        }],
        permissions: vec![
            "dangerous_operations".to_string(),
            "manage_settings".to_string(),
            "view_reports".to_string(),
        ],
    });
    state
        .db
        .with_conn(|conn| {
            repository::set_setting(conn, "runtime_mode", RuntimeMode::Client.as_str())?;
            repository::set_setting(conn, "host_address", "127.0.0.1")?;
            repository::set_setting(conn, "host_port", "9")?;
            repository::set_setting(conn, "client_token", "paired-token")
        })
        .unwrap();

    let status = get_app_status(&state, "0.1.0-test").unwrap();

    assert_eq!(status.runtime.mode, RuntimeMode::Client);
    assert_eq!(status.runtime.host_port, 9);
    assert_eq!(status.metrics.item_count, 0);
    assert!(status.health.message.contains("主机连接异常"));
    assert!(status.health.message.contains("业务操作已暂停"));
}
