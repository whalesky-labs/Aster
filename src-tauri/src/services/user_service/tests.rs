use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::app::paths::AppPaths;
use crate::db::{connection::Db, migrations, repository};
use crate::domain::users::{
    ChangePasswordRequest, LoginRequest, SaveUserRequest, SetUserEnabledRequest,
};

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

#[test]
fn password_hash_verifies_only_correct_password() {
    let hash = crate::domain::passwords::hash("admin123").unwrap();
    assert!(crate::domain::passwords::verify("admin123", &hash));
    assert!(!crate::domain::passwords::verify("wrong", &hash));
}

#[test]
fn save_and_disable_user_on_conn_write_audit_logs() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();

    let user = save_user_on_conn(
        &conn,
        SaveUserRequest {
            id: None,
            username: "warehouse01".to_string(),
            display_name: "仓库员 01".to_string(),
            email: None,
            password: Some("secret123".to_string()),
            department_id: None,
            enabled: true,
            role_codes: vec!["warehouse".to_string()],
        },
        "admin",
    )
    .unwrap();
    set_user_enabled_on_conn(
        &conn,
        SetUserEnabledRequest {
            user_id: user.id.clone(),
            enabled: false,
        },
        "admin",
    )
    .unwrap();

    let audit_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_logs WHERE entity_type = 'user' AND operator = 'admin'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(audit_count, 2);
}

#[test]
fn save_user_requires_enabled_department_for_department_viewer() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO departments (id, code, name, enabled)
             VALUES ('dept-disabled-user-test', 'DUT', '停用部门', 0)",
        [],
    )
    .unwrap();

    let missing_error = save_user_on_conn(
        &conn,
        SaveUserRequest {
            id: None,
            username: "viewer-missing-dept".to_string(),
            display_name: "未绑部门查看员".to_string(),
            email: None,
            password: Some("secret123".to_string()),
            department_id: None,
            enabled: true,
            role_codes: vec!["department_viewer".to_string()],
        },
        "admin",
    )
    .unwrap_err();
    assert!(missing_error.to_string().contains("必须绑定所属部门"));

    let disabled_error = save_user_on_conn(
        &conn,
        SaveUserRequest {
            id: None,
            username: "viewer-disabled-dept".to_string(),
            display_name: "停用部门查看员".to_string(),
            email: None,
            password: Some("secret123".to_string()),
            department_id: Some("dept-disabled-user-test".to_string()),
            enabled: true,
            role_codes: vec!["department_viewer".to_string()],
        },
        "admin",
    )
    .unwrap_err();
    assert!(disabled_error.to_string().contains("绑定部门已停用"));
}

#[test]
fn save_and_disable_user_preserve_last_enabled_admin() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    let hash = crate::domain::passwords::hash("admin123").unwrap();
    crate::db::user_repository::ensure_default_admin(&conn, &hash).unwrap();

    let remove_role_error = save_user_on_conn(
        &conn,
        SaveUserRequest {
            id: Some("user-admin".to_string()),
            username: "admin".to_string(),
            display_name: "系统管理员".to_string(),
            email: None,
            password: None,
            department_id: None,
            enabled: true,
            role_codes: vec!["warehouse".to_string()],
        },
        "admin",
    )
    .unwrap_err();
    assert!(remove_role_error.to_string().contains("最后一个启用管理员"));

    let disable_error = set_user_enabled_on_conn(
        &conn,
        SetUserEnabledRequest {
            user_id: "user-admin".to_string(),
            enabled: false,
        },
        "admin",
    )
    .unwrap_err();
    assert!(disable_error.to_string().contains("最后一个启用管理员"));

    save_user_on_conn(
        &conn,
        SaveUserRequest {
            id: None,
            username: "admin02".to_string(),
            display_name: "管理员 02".to_string(),
            email: None,
            password: Some("secret123".to_string()),
            department_id: None,
            enabled: true,
            role_codes: vec!["admin".to_string()],
        },
        "admin",
    )
    .unwrap();
    set_user_enabled_on_conn(
        &conn,
        SetUserEnabledRequest {
            user_id: "user-admin".to_string(),
            enabled: false,
        },
        "admin02",
    )
    .unwrap();
}

#[test]
fn login_and_change_password_on_conn_use_host_user_table() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    let hash = crate::domain::passwords::hash("admin123").unwrap();
    crate::db::user_repository::ensure_default_admin(&conn, &hash).unwrap();

    let current = login_on_conn(
        &conn,
        LoginRequest {
            username: "admin".to_string(),
            password: "admin123".to_string(),
        },
    )
    .unwrap();
    assert_eq!(current.username, "admin");

    change_password_on_conn(
        &conn,
        ChangePasswordRequest {
            user_id: Some(current.id.clone()),
            old_password: Some("admin123".to_string()),
            new_password: "newpass123".to_string(),
        },
        "admin",
        &current.id,
    )
    .unwrap();

    let old_login = login_on_conn(
        &conn,
        LoginRequest {
            username: "admin".to_string(),
            password: "admin123".to_string(),
        },
    );
    assert!(old_login.is_err());

    let new_login = login_on_conn(
        &conn,
        LoginRequest {
            username: "admin".to_string(),
            password: "newpass123".to_string(),
        },
    )
    .unwrap();
    assert_eq!(new_login.id, current.id);
}

#[test]
fn client_mode_without_pairing_token_allows_local_admin_for_pairing_setup() {
    let state = test_state();
    ensure_default_admin(&state).unwrap();
    state
        .db
        .with_conn(|conn| {
            repository::set_setting(
                conn,
                "runtime_mode",
                crate::domain::runtime::RuntimeMode::Client.as_str(),
            )
        })
        .unwrap();

    let current = login(
        &state,
        LoginRequest {
            username: "admin".to_string(),
            password: "admin123".to_string(),
        },
    )
    .unwrap();

    assert_eq!(current.username, "admin");
    assert!(current.roles.iter().any(|role| role.code == "admin"));
    assert!(current.permissions.contains(&"manage_settings".to_string()));
}

#[test]
fn client_mode_without_pairing_token_rejects_local_non_admin() {
    let state = test_state();
    state
        .db
        .with_conn(|conn| {
            repository::set_setting(
                conn,
                "runtime_mode",
                crate::domain::runtime::RuntimeMode::Client.as_str(),
            )?;
            save_user_on_conn(
                conn,
                SaveUserRequest {
                    id: None,
                    username: "warehouse01".to_string(),
                    display_name: "仓库员 01".to_string(),
                    email: None,
                    password: Some("secret123".to_string()),
                    department_id: None,
                    enabled: true,
                    role_codes: vec!["warehouse".to_string()],
                },
                "test",
            )?;
            Ok(())
        })
        .unwrap();

    let error = login(
        &state,
        LoginRequest {
            username: "warehouse01".to_string(),
            password: "secret123".to_string(),
        },
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("客户端未配对前只允许本地管理员登录"));
    assert!(current_user(&state).unwrap().is_none());
}
