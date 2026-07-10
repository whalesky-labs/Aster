use std::io::Cursor;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use super::{handle_connection_inner, write_json, ClientConnectionInfo, HostServiceRuntime};
use crate::app::paths::AppPaths;
use crate::app::state::AppState;
use crate::db::connection::Db;
use crate::error::AppResult;
use crate::infrastructure::http_transport;
use crate::services::remote_session_service::token_hash;

pub fn admin_state(directory: &tempfile::TempDir, db: Db) -> AppState {
    let paths = AppPaths {
        data_dir: directory.path().to_path_buf(),
        database_path: directory.path().join("aster.sqlite"),
        backup_dir: directory.path().join("backups"),
        export_dir: directory.path().join("exports"),
        import_report_dir: directory.path().join("import-reports"),
    };
    let state = AppState {
        paths,
        db,
        session: Arc::new(Mutex::new(None)),
        host_service: Arc::new(Mutex::new(HostServiceRuntime::default())),
    };
    crate::services::test_support::install_session(
        &state,
        crate::domain::users::CurrentUser {
            id: "user-admin".to_string(),
            username: "admin".to_string(),
            display_name: "管理员".to_string(),
            department_id: None,
            department_name: None,
            roles: vec![crate::domain::users::Role {
                id: "role-admin".to_string(),
                code: "admin".to_string(),
                name: "管理员".to_string(),
            }],
            permissions: vec!["manage_settings".to_string()],
        },
    )
    .expect("install admin session");
    state
}

pub fn test_db() -> (tempfile::TempDir, Db) {
    let directory = tempfile::tempdir().expect("temp dir");
    let paths = AppPaths {
        data_dir: directory.path().to_path_buf(),
        database_path: directory.path().join("aster.sqlite"),
        backup_dir: directory.path().join("backups"),
        export_dir: directory.path().join("exports"),
        import_report_dir: directory.path().join("import-reports"),
    };
    std::fs::create_dir_all(&paths.backup_dir).unwrap();
    std::fs::create_dir_all(&paths.export_dir).unwrap();
    std::fs::create_dir_all(&paths.import_report_dir).unwrap();
    (directory, Db::initialize(&paths).unwrap())
}

pub fn runtime_with_client(token: &str) -> Arc<Mutex<HostServiceRuntime>> {
    let mut runtime = HostServiceRuntime::default();
    runtime.clients.insert(
        "client-test".to_string(),
        ClientConnectionInfo {
            id: token.to_string(),
            client_name: "测试客户端".to_string(),
            client_device_id: "device-test".to_string(),
            client_ip: "127.0.0.1".to_string(),
            app_version: "0.1.0".to_string(),
            status: "paired".to_string(),
            last_seen_at: chrono::Local::now().to_rfc3339(),
        },
    );
    Arc::new(Mutex::new(runtime))
}

pub fn send_test_request(
    db: Db,
    runtime: Arc<Mutex<HostServiceRuntime>>,
    request: String,
) -> String {
    let request_length = request.len();
    let mut stream = Cursor::new(request.into_bytes());
    let parsed_request = http_transport::read_request(&mut stream).expect("read test request");
    if let Err(error) = handle_connection_inner(
        &mut stream,
        runtime,
        db,
        "0.1.0",
        "127.0.0.1".to_string(),
        parsed_request,
    ) {
        write_json(
            &mut stream,
            400,
            &serde_json::json!({ "message": error.to_string() }),
        )
        .expect("write test error response");
    }
    String::from_utf8(stream.into_inner()[request_length..].to_vec()).expect("response is utf8")
}

pub fn session_headers(db: &Db, device_token: &str, user_id: &str) -> AppResult<String> {
    db.with_conn(|conn| session_headers_on_conn(conn, device_token, user_id))
}

pub fn session_headers_on_conn(
    conn: &Connection,
    device_token: &str,
    user_id: &str,
) -> AppResult<String> {
    let session_token = format!("session-{device_token}-{user_id}");
    crate::db::session_repository::create(
        conn,
        &token_hash(&session_token),
        &token_hash(device_token),
        user_id,
        chrono::Utc::now().timestamp(),
    )?;
    Ok(format!(
        "X-Aster-Client-Token: {device_token}\r\nX-Aster-Session-Token: {session_token}"
    ))
}
