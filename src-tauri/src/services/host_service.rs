use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::application::pairing_service::{
    self, PairFinishRequest, PairFinishResponse, PairStartRequest, PairStartResponse,
    PairingRuntime, VerifiedPairing,
};
use crate::db::connection::Db;
use crate::db::{
    approval_repository, master_data_repository, paginated_stock_repository, report_repository,
    repository, stock_repository, stocktake_repository,
};
use crate::domain::approvals::{CreateApprovalRequest, DecideApprovalRequest};
use crate::domain::master_data::{
    SaveBudgetRuleRequest, SaveCategoryRequest, SaveDepartmentRequest, SaveItemRequest,
    SaveSupplierRequest, SaveUnitRequest,
};
use crate::domain::pagination::Page;
use crate::domain::reports::ReportQuery;
use crate::domain::runtime::{
    ClientConnectionInfo, HostConnectionTestRequest, HostConnectionTestResult, HostDiscoveryResult,
    HostServiceStatus, RemoveClientConnectionRequest, SaveClientConfigRequest,
};
use crate::domain::stock::{
    ConfirmStockDocumentDraftRequest, SaveStockDocumentDraftRequest, StockBalanceQuery,
    StockDocument, StockDocumentQuery, StockMovementQuery, StockMovementRow,
    SubmitAdjustmentRequest, SubmitStockDocumentRequest, VoidStockDocumentRequest,
};
use crate::domain::stocktake::{
    ConfirmStocktakeRequest, CreateStocktakeRequest, UpdateStocktakeCountsRequest,
};
use crate::domain::users::{
    ChangePasswordRequest, CurrentUser, RequestPasswordResetCodeRequest,
    ResetPasswordWithCodeRequest, SaveUserRequest, SetUserEnabledRequest,
};
use crate::error::{AppError, AppResult};
use crate::infrastructure::connection_limiter::ConnectionLimiter;
use crate::infrastructure::http_transport;
use crate::infrastructure::secure_transport;
use crate::{app::state::AppState, domain::runtime::RuntimeMode};

use http_transport::{page_path, query_param};

#[derive(Default)]
pub struct HostServiceRuntime {
    pub running: bool,
    pub bind_address: String,
    pub port: u16,
    pub pair_code: Option<String>,
    pub clients: HashMap<String, ClientConnectionInfo>,
    pub client_session_token: Option<String>,
    pub pairing: PairingRuntime,
    pub certificate_fingerprint: String,
    pub security_rate_limiter: crate::application::security_rate_limiter::SecurityRateLimiter,
}

pub fn ensure_host_service_for_mode(state: &AppState, app_version: &str) -> AppResult<()> {
    let config = crate::services::status_service::get_runtime_config(state)?;
    if config.mode == RuntimeMode::Host {
        start_host_service_internal(state, app_version)?;
    } else {
        stop_host_service_runtime(state);
    }
    Ok(())
}

pub fn start_host_service(state: &AppState, app_version: &str) -> AppResult<HostServiceStatus> {
    crate::services::user_service::require_admin(state)?;
    start_host_service_internal(state, app_version)
}

fn start_host_service_internal(
    state: &AppState,
    app_version: &str,
) -> AppResult<HostServiceStatus> {
    let config = crate::services::status_service::get_runtime_config(state)?;
    let bind_address = "0.0.0.0".to_string();
    let port = config.host_port;
    {
        let runtime = state
            .host_service
            .lock()
            .expect("host runtime mutex poisoned");
        if runtime.running && runtime.port == port {
            return Ok(status_from_runtime(&runtime));
        }
    }

    let listener = TcpListener::bind((bind_address.as_str(), port))
        .map_err(|error| AppError::Validation(format!("主机服务启动失败：{error}")))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| AppError::Validation(format!("主机服务配置失败：{error}")))?;

    let tls_identity = secure_transport::load_or_create_host_identity(&state.db)?;
    let pairing = PairingRuntime::initialize()?;
    let pair_code = pairing.code().map(str::to_owned);
    {
        let mut runtime = state
            .host_service
            .lock()
            .expect("host runtime mutex poisoned");
        runtime.running = true;
        runtime.bind_address = bind_address.clone();
        runtime.port = port;
        runtime.pair_code = pair_code;
        runtime.clients.clear();
        runtime.pairing = pairing;
        runtime.certificate_fingerprint = tls_identity.fingerprint.clone();
    }

    let runtime = Arc::clone(&state.host_service);
    let db = state.db.clone_handle();
    let version = app_version.to_string();
    let tls_config = tls_identity.server_config;
    thread::spawn(move || {
        serve(listener, runtime, db, version, tls_config);
    });

    let runtime = Arc::clone(&state.host_service);
    let db = state.db.clone_handle();
    let version = app_version.to_string();
    thread::spawn(move || {
        serve_discovery(port, runtime, db, version);
    });

    Ok(get_host_service_status(state))
}

pub fn get_host_service_status(state: &AppState) -> HostServiceStatus {
    let runtime = state
        .host_service
        .lock()
        .expect("host runtime mutex poisoned");
    status_from_runtime(&runtime)
}

pub fn stop_host_service_runtime(state: &AppState) {
    let mut runtime = state
        .host_service
        .lock()
        .expect("host runtime mutex poisoned");
    runtime.running = false;
    runtime.pair_code = None;
    runtime.clients.clear();
}

pub fn list_client_connections(state: &AppState) -> AppResult<Vec<ClientConnectionInfo>> {
    crate::services::user_service::require_admin(state)?;
    state
        .db
        .with_conn(crate::db::client_connection_repository::list)
}

pub fn remove_client_connection(
    state: &AppState,
    request: RemoveClientConnectionRequest,
) -> AppResult<()> {
    crate::services::user_service::require_admin(state)?;
    let client_device_id = request.client_device_id.trim().to_string();
    if client_device_id.is_empty() {
        return Err(AppError::Validation("客户端设备不能为空".to_string()));
    }
    let removed = state
        .db
        .with_conn(|conn| remove_client_connection_from_conn(conn, &client_device_id))?;
    {
        let mut runtime = state
            .host_service
            .lock()
            .expect("host runtime mutex poisoned");
        runtime
            .clients
            .retain(|_, client| client.client_device_id != client_device_id);
    }
    state.db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
             VALUES (?1, 'remove_client_connection', 'client_connection', ?2, ?3, ?4)",
            rusqlite::params![
                Uuid::new_v4().to_string(),
                client_device_id,
                format!("移除客户端设备：{}", removed.client_name),
                crate::services::user_service::current_operator(state)
            ],
        )?;
        Ok(())
    })
}

pub fn save_client_config(
    state: &AppState,
    request: SaveClientConfigRequest,
) -> AppResult<crate::domain::runtime::RuntimeConfig> {
    allow_client_bootstrap_or_admin(state, false)?;
    validate_host_port(request.host_port)?;
    let next_address = normalize_host_address(&request.host_address)?;
    state.db.with_conn(|conn| {
        let current_address = repository::get_setting(conn, "host_address")?;
        let current_port = repository::get_setting(conn, "host_port")?;
        let next_port = request.host_port.to_string();
        let host_changed = current_address.as_deref() != Some(next_address.as_str())
            || current_port.as_deref() != Some(next_port.as_str());
        repository::set_setting(conn, "host_address", &next_address)?;
        repository::set_setting(conn, "host_port", &request.host_port.to_string())?;
        repository::set_setting(conn, "runtime_mode", RuntimeMode::Client.as_str())?;
        if host_changed {
            repository::delete_setting(conn, "client_token")?;
            repository::delete_setting(conn, "host_certificate_fingerprint")?;
        }
        Ok(())
    })?;
    crate::services::status_service::get_runtime_config(state)
}

pub fn pair_with_host(
    state: &AppState,
    pair_code: String,
    client_name: String,
    client_device_id: String,
    app_version: String,
) -> AppResult<crate::domain::runtime::RuntimeConfig> {
    allow_client_bootstrap_or_admin(state, true)?;
    validate_pairing_request(&pair_code, &client_name, &client_device_id)?;
    let config = crate::services::status_service::get_runtime_config(state)?;
    let address = config
        .host_address
        .clone()
        .ok_or_else(|| AppError::Validation("请先保存主机地址".to_string()))?;
    let client_nonce = pairing_service::random_nonce();
    let (client_state, ke1) = pairing_service::client_start(&pair_code)?;
    let (start_response, certificate_fingerprint): (PairStartResponse, String) =
        http_post_json_for_pairing(
            &address,
            config.host_port,
            "/api/pair/start",
            &PairStartRequest {
                client_name,
                client_device_id: client_device_id.clone(),
                app_version,
                client_nonce: client_nonce.clone(),
                ke1,
            },
            None,
        )?;
    let ke3 = pairing_service::client_finish(
        client_state,
        &pair_code,
        &start_response,
        &certificate_fingerprint,
        &client_device_id,
        &client_nonce,
    )?;
    let (response, final_fingerprint): (PairFinishResponse, String) = http_post_json_for_pairing(
        &address,
        config.host_port,
        "/api/pair/finish",
        &PairFinishRequest {
            exchange_id: start_response.exchange_id,
            ke3,
        },
        Some(&certificate_fingerprint),
    )?;
    if final_fingerprint != certificate_fingerprint {
        return Err(AppError::Validation("配对期间主机证书发生变化".to_string()));
    }
    state.db.with_conn(|conn| {
        repository::set_setting(conn, "client_token", &response.token)?;
        repository::set_setting(
            conn,
            "host_certificate_fingerprint",
            &certificate_fingerprint,
        )?;
        Ok(())
    })?;
    crate::services::status_service::get_runtime_config(state)
}

fn allow_client_bootstrap_or_admin(state: &AppState, require_client_mode: bool) -> AppResult<()> {
    if crate::services::user_service::current_user(state)?.is_some() {
        crate::services::user_service::require_admin(state)?;
        return Ok(());
    }
    let mode = crate::services::status_service::get_runtime_config(state)?.mode;
    if mode == RuntimeMode::Host || (require_client_mode && mode != RuntimeMode::Client) {
        return Err(AppError::Validation("请先登录管理员账号".to_string()));
    }
    Ok(())
}

pub fn test_host_connection(
    request: HostConnectionTestRequest,
) -> AppResult<HostConnectionTestResult> {
    validate_host_port(request.host_port)?;
    let host_address = normalize_host_address(&request.host_address)?;
    let mut stream = secure_transport::connect(&host_address, request.host_port, None)?.stream;
    stream.write_all(b"GET /api/health HTTP/1.1\r\nHost: aster\r\nConnection: close\r\n\r\n")?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    let Some(body) = response.split("\r\n\r\n").nth(1) else {
        return Ok(HostConnectionTestResult {
            ok: false,
            message: "主机响应格式异常".to_string(),
            app_name: None,
            app_version: None,
            schema_version: None,
        });
    };
    let health: HealthResponse = serde_json::from_str(body)
        .map_err(|error| AppError::Validation(format!("主机响应解析失败：{error}")))?;
    Ok(HostConnectionTestResult {
        ok: health.database_ok,
        message: health.message,
        app_name: Some(health.app_name),
        app_version: Some(health.app_version),
        schema_version: Some(health.schema_version),
    })
}

pub fn discover_hosts(host_port: u16) -> AppResult<Vec<HostDiscoveryResult>> {
    validate_host_port(host_port)?;
    let socket = UdpSocket::bind("0.0.0.0:0")
        .map_err(|error| AppError::Validation(format!("主机发现启动失败：{error}")))?;
    socket
        .set_broadcast(true)
        .map_err(|error| AppError::Validation(format!("主机发现广播配置失败：{error}")))?;
    socket
        .set_read_timeout(Some(Duration::from_millis(900)))
        .map_err(|error| AppError::Validation(format!("主机发现超时配置失败：{error}")))?;

    let broadcast = SocketAddr::from(([255, 255, 255, 255], host_port));
    socket
        .send_to(b"ASTER_DISCOVER_V1", broadcast)
        .map_err(|error| AppError::Validation(format!("主机发现广播失败：{error}")))?;

    let mut results: Vec<HostDiscoveryResult> = Vec::new();
    let mut buffer = [0_u8; 2048];
    loop {
        match socket.recv_from(&mut buffer) {
            Ok((bytes, addr)) => {
                let body = String::from_utf8_lossy(&buffer[..bytes]);
                if let Ok(mut result) = serde_json::from_str::<DiscoveryResponse>(&body) {
                    if result.app_name == "Aster" {
                        if result.host_address.trim().is_empty() || result.host_address == "0.0.0.0"
                        {
                            result.host_address = addr.ip().to_string();
                        }
                        if !results.iter().any(|item| {
                            item.host_address == result.host_address
                                && item.host_port == result.host_port
                        }) {
                            results.push(HostDiscoveryResult {
                                host_address: result.host_address,
                                host_port: result.host_port,
                                app_name: result.app_name,
                                app_version: result.app_version,
                                schema_version: result.schema_version,
                                message: result.message,
                            });
                        }
                    }
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(error) => return Err(AppError::Io(error)),
        }
    }
    Ok(results)
}

mod remote_master_data;
mod remote_operations;
mod remote_stock;

pub use remote_master_data::*;
pub use remote_operations::*;
pub use remote_stock::*;

fn serve(
    listener: TcpListener,
    runtime: Arc<Mutex<HostServiceRuntime>>,
    db: Db,
    app_version: String,
    tls_config: Arc<rustls::ServerConfig>,
) {
    let limiter = ConnectionLimiter::new(64, 8);
    loop {
        let running = runtime
            .lock()
            .map(|runtime| runtime.running)
            .unwrap_or(false);
        if !running {
            break;
        }
        match listener.accept() {
            Ok((stream, addr)) => {
                let source = addr.ip().to_string();
                let Some(permit) = limiter.try_acquire(&source) else {
                    drop(stream);
                    continue;
                };
                let runtime = Arc::clone(&runtime);
                let db = db.clone_handle();
                let version = app_version.clone();
                let tls_config = Arc::clone(&tls_config);
                thread::spawn(move || {
                    let _permit = permit;
                    let _ = handle_connection(stream, runtime, db, &version, tls_config);
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(_) => break,
        }
    }
}

fn serve_discovery(
    port: u16,
    runtime: Arc<Mutex<HostServiceRuntime>>,
    db: Db,
    app_version: String,
) {
    let Ok(socket) = UdpSocket::bind(("0.0.0.0", port)) else {
        return;
    };
    let _ = socket.set_read_timeout(Some(Duration::from_millis(500)));
    let mut buffer = [0_u8; 512];
    loop {
        let running = runtime
            .lock()
            .map(|runtime| runtime.running)
            .unwrap_or(false);
        if !running {
            break;
        }
        match socket.recv_from(&mut buffer) {
            Ok((bytes, peer)) => {
                if &buffer[..bytes] != b"ASTER_DISCOVER_V1" {
                    continue;
                }
                let schema_version = db.with_conn(repository::schema_version).unwrap_or_default();
                let response = DiscoveryResponse {
                    host_address: String::new(),
                    host_port: port,
                    app_name: "Aster".to_string(),
                    app_version: app_version.clone(),
                    schema_version,
                    message: "Aster 主机服务可用".to_string(),
                };
                if let Ok(body) = serde_json::to_vec(&response) {
                    let _ = socket.send_to(&body, peer);
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => break,
        }
    }
}

fn handle_connection(
    stream: TcpStream,
    runtime: Arc<Mutex<HostServiceRuntime>>,
    db: Db,
    app_version: &str,
    tls_config: Arc<rustls::ServerConfig>,
) -> AppResult<()> {
    let monitor_socket = stream.try_clone()?;
    let peer_ip = stream
        .peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "-".to_string());
    let mut stream = secure_transport::accept(stream, tls_config)?;
    let result = match http_transport::read_request(&mut stream) {
        Ok(request) => {
            let cancelled = Arc::new(AtomicBool::new(false));
            let monitor_done = Arc::new(AtomicBool::new(false));
            let monitor = start_disconnect_monitor(monitor_socket, &cancelled, &monitor_done);
            let result = crate::db::connection::with_query_control(
                Duration::from_secs(30),
                Arc::clone(&cancelled),
                || handle_connection_inner(&mut stream, runtime, db, app_version, peer_ip, request),
            );
            monitor_done.store(true, Ordering::Release);
            let _ = monitor.join();
            result
        }
        Err(error) => Err(error),
    };
    if let Err(error) = result {
        let status = http_transport::error_status(&error);
        let message = http_transport::public_error_message(&error);
        write_json(
            &mut stream,
            status,
            &serde_json::json!({ "message": message }),
        )?;
    }
    Ok(())
}

fn start_disconnect_monitor(
    socket: TcpStream,
    cancelled: &Arc<AtomicBool>,
    done: &Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    let cancelled = Arc::clone(cancelled);
    let done = Arc::clone(done);
    thread::spawn(move || {
        let _ = socket.set_read_timeout(Some(Duration::from_millis(100)));
        let mut byte = [0_u8; 1];
        while !done.load(Ordering::Acquire) {
            match socket.peek(&mut byte) {
                Ok(0) => {
                    cancelled.store(true, Ordering::Release);
                    return;
                }
                Ok(_) => thread::sleep(Duration::from_millis(25)),
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        || error.kind() == std::io::ErrorKind::TimedOut => {}
                Err(_) => {
                    cancelled.store(true, Ordering::Release);
                    return;
                }
            }
        }
    })
}

fn handle_connection_inner<S: Read + Write>(
    stream: &mut S,
    runtime: Arc<Mutex<HostServiceRuntime>>,
    db: Db,
    app_version: &str,
    peer_ip: String,
    request: String,
) -> AppResult<()> {
    let (method, path) = http_transport::request_line(&request);
    let body = request.split("\r\n\r\n").nth(1).unwrap_or("");
    let auth_request = request.clone();
    enforce_security_rate_limit(&runtime, &path, &peer_ip, &request, body)?;

    match (method.as_str(), path.as_str()) {
        ("GET", "/api/health") => {
            let response = health_response(&db, app_version)?;
            write_json(stream, 200, &response)?;
        }
        ("GET", "/api/version") => {
            let response = version_response(&db, app_version)?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/pair/start") => {
            let request: PairStartRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("配对请求解析失败：{error}")))?;
            let response = begin_pairing(&runtime, request, peer_ip)?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/pair/finish") => {
            let request: PairFinishRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("配对请求解析失败：{error}")))?;
            let response = finish_pairing(&runtime, &db, request)?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/clients") => {
            authenticate_request_and_load_client(&request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let clients = db.with_conn(|conn| {
                require_remote_admin(&auth_request, conn)?;
                crate::db::client_connection_repository::list_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &clients)?;
        }
        ("GET", "/api/status") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let response = db.with_conn(|conn| {
                let current = require_remote_permission(&auth_request, conn, "view_reports")?;
                let department_scope = remote_department_scope(&current)?;
                crate::services::status_service::build_app_status(
                    conn,
                    app_version,
                    None,
                    department_scope.as_deref(),
                )
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", "/api/system-settings") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let response = db.with_conn(|conn| {
                require_remote_admin(&auth_request, conn)?;
                crate::services::status_service::system_settings_from_conn(conn, None)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/backups") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_admin(&auth_request, conn)?;
                crate::db::backup_repository::list_backup_records_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/users") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_admin(&auth_request, conn)?;
                crate::db::user_repository::list_users_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/roles") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_admin(&auth_request, conn)?;
                crate::db::user_repository::list_roles_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/user") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: SaveUserRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("用户请求解析失败：{error}")))?;
            let response = db.with_conn(|conn| {
                require_remote_admin(&auth_request, conn)?;
                crate::services::user_service::save_user_on_conn(conn, request, "client")
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/user/enabled") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: SetUserEnabledRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("用户状态请求解析失败：{error}")))?;
            db.with_conn(|conn| {
                require_remote_admin(&auth_request, conn)?;
                crate::services::user_service::set_user_enabled_on_conn(conn, request, "client")
            })?;
            write_json(stream, 200, &())?;
        }
        ("POST", "/api/login") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let response =
                crate::services::remote_session_service::handle_login(&db, &auth_request, body)?;
            clear_login_rate_limit(&runtime, &peer_ip, &request, body)?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/logout") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            crate::services::remote_session_service::handle_logout(&db, &request)?;
            write_json(stream, 200, &())?;
        }
        ("POST", "/api/user/password") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: ChangePasswordRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("密码请求解析失败：{error}")))?;
            db.with_conn(|conn| {
                let current = remote_current_user(&auth_request, conn)?;
                if request.user_id.as_deref() != Some(current.id.as_str()) {
                    require_remote_admin(&auth_request, conn)?;
                }
                crate::services::user_service::change_password_on_conn(
                    conn,
                    request,
                    "client",
                    &current.id,
                )
            })?;
            write_json(stream, 200, &())?;
        }
        ("POST", "/api/password-reset/request") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: RequestPasswordResetCodeRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("找回密码请求解析失败：{error}")))?;
            let response = db.with_conn(|conn| {
                crate::services::user_service::request_password_reset_code_on_conn(conn, request)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/password-reset/confirm") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: ResetPasswordWithCodeRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("重置密码请求解析失败：{error}")))?;
            db.with_conn(|conn| {
                crate::services::user_service::reset_password_with_code_on_conn(conn, request)
            })?;
            write_json(stream, 200, &())?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/audit-logs") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let limit = query_param(path, "limit").and_then(|value| value.parse::<i64>().ok());
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_admin(&auth_request, conn)?;
                repository::list_audit_logs_page(conn, limit.unwrap_or(100), cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/stock/documents") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let query = StockDocumentQuery {
                document_type: query_param(path, "documentType"),
                outbound_kind: query_param(path, "outboundKind"),
                month: query_param(path, "month"),
                department_id: query_param(path, "departmentId"),
                supplier_id: query_param(path, "supplierId"),
                item_id: query_param(path, "itemId"),
                handler: query_param(path, "handler"),
                search: query_param(path, "search"),
            };
            let cursor = query_param(path, "cursor");
            let response: Page<StockDocument> = db.with_conn(|conn| {
                let current = require_remote_permission(&auth_request, conn, "view_reports")?;
                let mut query = query;
                query.department_id = remote_department_scope(&current)?.or(query.department_id);
                paginated_stock_repository::list_documents_page(conn, query, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/stock/document") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let document_id = query_param(path, "documentId")
                .ok_or_else(|| AppError::Validation("缺少单据 ID".to_string()))?;
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                stock_repository::get_stock_document_detail(conn, &document_id)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/stock/balances") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let query = StockBalanceQuery {
                search: query_param(path, "search"),
                category_id: query_param(path, "categoryId"),
                item_id: query_param(path, "itemId"),
                stock_status: query_param(path, "stockStatus"),
            };
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                stock_repository::list_stock_balances_page(conn, query, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/stock/batches") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let item_id = query_param(path, "itemId")
                .ok_or_else(|| AppError::Validation("缺少物品 ID".to_string()))?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                stock_repository::list_stock_batches_page(conn, &item_id, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/stock/movements") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let query = StockMovementQuery {
                search: query_param(path, "search"),
                item_id: query_param(path, "itemId"),
                department_id: query_param(path, "departmentId"),
                direction: query_param(path, "direction"),
                movement_type: query_param(path, "movementType"),
            };
            let cursor = query_param(path, "cursor");
            let response: Page<StockMovementRow> = db.with_conn(|conn| {
                let current = require_remote_permission(&auth_request, conn, "view_reports")?;
                let mut query = query;
                query.department_id = remote_department_scope(&current)?.or(query.department_id);
                paginated_stock_repository::list_movements_page(conn, query, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/stock/document") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let mut request: SubmitStockDocumentRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("单据请求解析失败：{error}")))?;
            crate::services::stock_service::normalize_business_datetime(
                &mut request.business_date,
                "业务日期",
            )?;
            crate::services::stock_service::validate_document(&request)?;
            let response = db.with_conn_mut(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                let allow_negative_stock =
                    crate::db::repository::get_setting(conn, "allow_negative_stock")?
                        .map(|value| value == "true")
                        .unwrap_or(false);
                stock_repository::submit_stock_document(conn, request, allow_negative_stock)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/stock/document/draft") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let mut request: SaveStockDocumentDraftRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("草稿请求解析失败：{error}")))?;
            crate::services::stock_service::normalize_business_datetime(
                &mut request.business_date,
                "业务日期",
            )?;
            crate::services::stock_service::validate_draft_document(&request)?;
            let response = db.with_conn_mut(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                stock_repository::save_stock_document_draft(conn, request)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/stock/document/draft/confirm") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: ConfirmStockDocumentDraftRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("草稿确认请求解析失败：{error}")))?;
            crate::services::stock_service::validate_confirm_draft(&request)?;
            let response = db.with_conn_mut(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                let allow_negative_stock =
                    crate::db::repository::get_setting(conn, "allow_negative_stock")?
                        .map(|value| value == "true")
                        .unwrap_or(false);
                stock_repository::confirm_stock_document_draft(conn, request, allow_negative_stock)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/stock/adjustment") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let mut request: SubmitAdjustmentRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("调整请求解析失败：{error}")))?;
            crate::services::stock_service::normalize_business_datetime(
                &mut request.business_date,
                "调整日期",
            )?;
            crate::services::stock_service::validate_adjustment(&request)?;
            let response = db.with_conn_mut(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                stock_repository::submit_adjustment(conn, request)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/stock/void") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: VoidStockDocumentRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("作废请求解析失败：{error}")))?;
            crate::services::stock_service::validate_void_document(&request)?;
            let response = db.with_conn_mut(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                stock_repository::void_stock_document(conn, request)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/master/categories") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                master_data_repository::list_categories_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/category") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: SaveCategoryRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("分类请求解析失败：{error}")))?;
            crate::services::master_data_service::validate_category(&request)?;
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                let category = master_data_repository::save_category(conn, request)?;
                write_host_audit(
                    conn,
                    "save_category",
                    "category",
                    &category.id,
                    &category.name,
                )?;
                Ok(category)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/category/enabled") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: SetEnabledRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("分类状态请求解析失败：{error}")))?;
            db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                master_data_repository::set_category_enabled(
                    conn,
                    &request.id,
                    request.enabled,
                    request.expected_updated_at.as_deref(),
                )?;
                write_host_audit(
                    conn,
                    "set_category_enabled",
                    "category",
                    &request.id,
                    &request.enabled.to_string(),
                )
            })?;
            write_json(stream, 200, &())?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/master/units") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                master_data_repository::list_units_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/unit") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: SaveUnitRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("单位请求解析失败：{error}")))?;
            crate::services::master_data_service::validate_unit(&request)?;
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                let unit = master_data_repository::save_unit(conn, request)?;
                write_host_audit(conn, "save_unit", "unit", &unit.id, &unit.name)?;
                Ok(unit)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/unit/enabled") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: SetEnabledRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("单位状态请求解析失败：{error}")))?;
            db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                master_data_repository::set_unit_enabled(
                    conn,
                    &request.id,
                    request.enabled,
                    request.expected_updated_at.as_deref(),
                )?;
                write_host_audit(
                    conn,
                    "set_unit_enabled",
                    "unit",
                    &request.id,
                    &request.enabled.to_string(),
                )
            })?;
            write_json(stream, 200, &())?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/master/departments") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                master_data_repository::list_departments_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/department") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: SaveDepartmentRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("部门请求解析失败：{error}")))?;
            crate::services::master_data_service::validate_department(&request)?;
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                let department = master_data_repository::save_department(conn, request)?;
                write_host_audit(
                    conn,
                    "save_department",
                    "department",
                    &department.id,
                    &department.name,
                )?;
                Ok(department)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/department/enabled") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: SetEnabledRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("部门状态请求解析失败：{error}")))?;
            db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                master_data_repository::set_department_enabled(
                    conn,
                    &request.id,
                    request.enabled,
                    request.expected_updated_at.as_deref(),
                )?;
                write_host_audit(
                    conn,
                    "set_department_enabled",
                    "department",
                    &request.id,
                    &request.enabled.to_string(),
                )
            })?;
            write_json(stream, 200, &())?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/master/suppliers") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                master_data_repository::list_suppliers_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/master/supplier/purchases") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let supplier_id = query_param(path, "supplierId")
                .ok_or_else(|| AppError::Validation("缺少供应商 ID".to_string()))?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                master_data_repository::list_supplier_purchase_records_page(
                    conn,
                    &supplier_id,
                    cursor.as_deref(),
                )
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/supplier") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: SaveSupplierRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("供应商请求解析失败：{error}")))?;
            crate::services::master_data_service::validate_supplier(&request)?;
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                let supplier = master_data_repository::save_supplier(conn, request)?;
                write_host_audit(
                    conn,
                    "save_supplier",
                    "supplier",
                    &supplier.id,
                    &supplier.name,
                )?;
                Ok(supplier)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/supplier/enabled") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: SetEnabledRequest = serde_json::from_str(body).map_err(|error| {
                AppError::Validation(format!("供应商状态请求解析失败：{error}"))
            })?;
            db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                master_data_repository::set_supplier_enabled(
                    conn,
                    &request.id,
                    request.enabled,
                    request.expected_updated_at.as_deref(),
                )?;
                write_host_audit(
                    conn,
                    "set_supplier_enabled",
                    "supplier",
                    &request.id,
                    &request.enabled.to_string(),
                )
            })?;
            write_json(stream, 200, &())?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/master/items") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let search = query_param(path, "search");
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                master_data_repository::list_items_page(conn, search, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/item") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: SaveItemRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("物品请求解析失败：{error}")))?;
            crate::services::master_data_service::validate_item(&request)?;
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                let item = master_data_repository::save_item(conn, request)?;
                write_host_audit(conn, "save_item", "item", &item.id, &item.name)?;
                Ok(item)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/item/enabled") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: SetEnabledRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("物品状态请求解析失败：{error}")))?;
            db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                master_data_repository::set_item_enabled(
                    conn,
                    &request.id,
                    request.enabled,
                    request.expected_updated_at.as_deref(),
                )?;
                write_host_audit(
                    conn,
                    "set_item_enabled",
                    "item",
                    &request.id,
                    &request.enabled.to_string(),
                )
            })?;
            write_json(stream, 200, &())?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/master/budget-rules") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let month = query_param(path, "month");
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_admin(&auth_request, conn)?;
                master_data_repository::list_budget_rules_page(conn, month, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/budget-rule") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: SaveBudgetRuleRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("预算请求解析失败：{error}")))?;
            crate::services::master_data_service::validate_budget_rule(&request)?;
            let response = db.with_conn(|conn| {
                require_remote_admin(&auth_request, conn)?;
                let rule = master_data_repository::save_budget_rule(conn, request)?;
                write_host_audit(
                    conn,
                    "save_budget_rule",
                    "budget_rule",
                    &rule.id,
                    &rule.period_month,
                )?;
                Ok(rule)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/budget-rule/enabled") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: SetEnabledRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("预算状态请求解析失败：{error}")))?;
            db.with_conn(|conn| {
                require_remote_admin(&auth_request, conn)?;
                master_data_repository::set_budget_rule_enabled(
                    conn,
                    &request.id,
                    request.enabled,
                    request.expected_updated_at.as_deref(),
                )?;
                write_host_audit(
                    conn,
                    "set_budget_rule_enabled",
                    "budget_rule",
                    &request.id,
                    &request.enabled.to_string(),
                )
            })?;
            write_json(stream, 200, &())?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/reports/monthly") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let month = query_param(path, "month")
                .ok_or_else(|| AppError::Validation("报表月份不能为空".to_string()))?;
            let start_date = query_param(path, "startDate").map(|date| {
                if date.len() == 10 {
                    format!("{date} 00:00:00")
                } else {
                    date
                }
            });
            let end_date = query_param(path, "endDate").map(|date| {
                if date.len() == 10 {
                    format!("{date} 23:59:59")
                } else {
                    date
                }
            });
            let department_id = query_param(path, "departmentId");
            let category_id = query_param(path, "categoryId");
            let item_id = query_param(path, "itemId");
            let supplier_id = query_param(path, "supplierId");
            let section = query_param(path, "section")
                .ok_or_else(|| AppError::Validation("报表分页 section 不能为空".to_string()))?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                let current = require_remote_permission(&auth_request, conn, "view_reports")?;
                let scoped_department_id = remote_department_scope(&current)?.or(department_id);
                report_repository::get_report_bundle_page(
                    conn,
                    &ReportQuery {
                        month,
                        start_date,
                        end_date,
                        department_id: scoped_department_id,
                        category_id,
                        item_id,
                        supplier_id,
                    },
                    &section,
                    cursor.as_deref(),
                )
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/stocktakes") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                stocktake_repository::list_stocktakes_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/stocktakes") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let mut request: CreateStocktakeRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("盘点创建请求解析失败：{error}")))?;
            crate::services::stock_service::normalize_business_datetime(
                &mut request.business_date,
                "盘点日期",
            )?;
            crate::services::stocktake_service::validate_create_request(&request)?;
            let response = db.with_conn_mut(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                stocktake_repository::create_stocktake(conn, request)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/stocktake") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let stocktake_id = query_param(path, "stocktakeId")
                .ok_or_else(|| AppError::Validation("盘点单不能为空".to_string()))?;
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                stocktake_repository::get_stocktake_detail(conn, &stocktake_id)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/stocktake/counts") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: UpdateStocktakeCountsRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("盘点录入请求解析失败：{error}")))?;
            crate::services::stocktake_service::validate_update_counts(&request)?;
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                stocktake_repository::update_stocktake_counts(conn, request)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/stocktake/confirm") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: ConfirmStocktakeRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("盘点确认请求解析失败：{error}")))?;
            crate::services::stocktake_service::validate_confirm_request(&request)?;
            let response = db.with_conn_mut(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                stocktake_repository::confirm_stocktake(conn, request)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/approvals") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_admin(&auth_request, conn)?;
                approval_repository::list_approval_requests_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/approval") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: CreateApprovalRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("审批请求解析失败：{error}")))?;
            let response = db.with_conn(|conn| {
                let current = require_remote_permission(&auth_request, conn, "write_stock")?;
                crate::services::approval_service::create_approval_request_on_conn(
                    conn,
                    request,
                    Some(current.id),
                    "client",
                )
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/approval/decision") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: DecideApprovalRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("审批决定解析失败：{error}")))?;
            let response = db.with_conn(|conn| {
                let current = require_remote_admin(&auth_request, conn)?;
                crate::services::approval_service::decide_approval_request_on_conn(
                    conn,
                    request,
                    Some(current.id),
                    "client",
                )
            })?;
            write_json(stream, 200, &response)?;
        }
        _ => {
            write_json(
                stream,
                404,
                &serde_json::json!({ "message": "Aster host API not found" }),
            )?;
        }
    }
    Ok(())
}

fn enforce_security_rate_limit(
    runtime: &Arc<Mutex<HostServiceRuntime>>,
    path: &str,
    peer_ip: &str,
    request: &str,
    body: &str,
) -> AppResult<()> {
    let Some((operation, source)) = security_limit_key(path, peer_ip, request, body) else {
        return Ok(());
    };
    runtime
        .lock()
        .map_err(|_| AppError::Validation("主机安全状态异常".to_string()))?
        .security_rate_limiter
        .check(operation, &source)
}

fn clear_login_rate_limit(
    runtime: &Arc<Mutex<HostServiceRuntime>>,
    peer_ip: &str,
    request: &str,
    body: &str,
) -> AppResult<()> {
    let Some((operation, source)) = security_limit_key("/api/login", peer_ip, request, body) else {
        return Ok(());
    };
    runtime
        .lock()
        .map_err(|_| AppError::Validation("主机安全状态异常".to_string()))?
        .security_rate_limiter
        .clear(operation, &source);
    Ok(())
}

fn security_limit_key<'a>(
    path: &'a str,
    peer_ip: &str,
    request: &str,
    body: &str,
) -> Option<(&'a str, String)> {
    let operation = match path {
        "/api/pair/start" | "/api/pair/finish" => "pair",
        "/api/login" => "login",
        "/api/password-reset/request" | "/api/password-reset/confirm" => "password-reset",
        _ => return None,
    };
    let device = header_value(request, "X-Aster-Client-Token").unwrap_or_default();
    let username = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.get("username")?.as_str().map(str::to_lowercase))
        .unwrap_or_default();
    let source = match operation {
        "login" => format!("{peer_ip}:{device}:{username}"),
        "password-reset" => format!("{peer_ip}:{username}"),
        _ => peer_ip.to_string(),
    };
    Some((operation, source))
}

#[derive(Clone)]
struct ClientRuntimeConfig {
    address: String,
    port: u16,
    token: String,
    session_token: Option<String>,
    certificate_fingerprint: String,
}

fn client_runtime_config(state: &AppState) -> AppResult<ClientRuntimeConfig> {
    let config = crate::services::status_service::get_runtime_config(state)?;
    if config.mode != RuntimeMode::Client {
        return Err(AppError::Validation("当前不是客户端模式".to_string()));
    }
    let address = config
        .host_address
        .ok_or_else(|| AppError::Validation("未配置主机地址".to_string()))?;
    let token = config
        .client_token
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| AppError::Validation("未完成主机配对，请先在设置中配对".to_string()))?;
    let session_token = state
        .host_service
        .lock()
        .map_err(|_| AppError::Validation("客户端会话状态异常".to_string()))?
        .client_session_token
        .clone();
    let certificate_fingerprint = state
        .db
        .with_conn(|conn| repository::get_setting(conn, "host_certificate_fingerprint"))?
        .filter(|fingerprint| !fingerprint.trim().is_empty())
        .ok_or_else(|| AppError::Validation("主机证书未固定，请重新配对".to_string()))?;
    Ok(ClientRuntimeConfig {
        address,
        port: config.host_port,
        token,
        session_token,
        certificate_fingerprint,
    })
}

fn write_host_audit(
    conn: &rusqlite::Connection,
    action: &str,
    entity_type: &str,
    entity_id: &str,
    summary: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, ?2, ?3, ?4, ?5, 'client')",
        rusqlite::params![
            Uuid::new_v4().to_string(),
            action,
            entity_type,
            entity_id,
            summary
        ],
    )?;
    Ok(())
}

fn remote_current_user(request: &str, conn: &rusqlite::Connection) -> AppResult<CurrentUser> {
    crate::services::remote_session_service::current_user(request, conn)
}

fn require_remote_admin(request: &str, conn: &rusqlite::Connection) -> AppResult<CurrentUser> {
    let current = remote_current_user(request, conn)?;
    if current.roles.iter().any(|role| role.code == "admin") {
        Ok(current)
    } else {
        Err(AppError::Forbidden("需要管理员权限".to_string()))
    }
}

fn require_remote_permission(
    request: &str,
    conn: &rusqlite::Connection,
    permission: &str,
) -> AppResult<CurrentUser> {
    let current = remote_current_user(request, conn)?;
    if current
        .permissions
        .iter()
        .any(|item| item == permission || item == "dangerous_operations")
    {
        Ok(current)
    } else {
        Err(AppError::Forbidden(format!("缺少权限：{permission}")))
    }
}

fn remote_department_scope(current: &CurrentUser) -> AppResult<Option<String>> {
    let is_admin_or_warehouse = current
        .roles
        .iter()
        .any(|role| role.code == "admin" || role.code == "warehouse");
    if is_admin_or_warehouse {
        Ok(None)
    } else if current
        .roles
        .iter()
        .any(|role| role.code == "department_viewer")
    {
        current
            .department_id
            .clone()
            .map(Some)
            .ok_or_else(|| AppError::Validation("部门查看员未绑定所属部门".to_string()))
    } else {
        Ok(None)
    }
}

fn health_response(db: &Db, app_version: &str) -> AppResult<HealthResponse> {
    db.with_conn(|conn| {
        let integrity = repository::integrity_check(conn)?;
        let schema_version = repository::schema_version(conn)?;
        Ok(HealthResponse {
            app_name: "Aster".to_string(),
            app_version: app_version.to_string(),
            schema_version,
            database_ok: integrity == "ok",
            message: if integrity == "ok" {
                "主机数据库健康".to_string()
            } else {
                format!("主机数据库异常：{integrity}")
            },
        })
    })
}

fn version_response(db: &Db, app_version: &str) -> AppResult<VersionResponse> {
    db.with_conn(|conn| {
        Ok(VersionResponse {
            app_name: "Aster".to_string(),
            app_version: app_version.to_string(),
            schema_version: repository::schema_version(conn)?,
        })
    })
}

fn begin_pairing(
    runtime: &Arc<Mutex<HostServiceRuntime>>,
    request: PairStartRequest,
    client_ip: String,
) -> AppResult<PairStartResponse> {
    let mut runtime = runtime
        .lock()
        .map_err(|_| AppError::Validation("主机配对状态异常".to_string()))?;
    let fingerprint = runtime.certificate_fingerprint.clone();
    runtime.pairing.begin(request, client_ip, &fingerprint)
}

fn finish_pairing(
    runtime: &Arc<Mutex<HostServiceRuntime>>,
    db: &Db,
    request: PairFinishRequest,
) -> AppResult<PairFinishResponse> {
    let verified = {
        let mut runtime = runtime
            .lock()
            .map_err(|_| AppError::Validation("主机配对状态异常".to_string()))?;
        let verified = runtime.pairing.finish(request)?;
        runtime.pair_code = runtime.pairing.code().map(str::to_owned);
        verified
    };
    register_paired_client(runtime, db, verified)
}

fn register_paired_client(
    runtime: &Arc<Mutex<HostServiceRuntime>>,
    db: &Db,
    verified: VerifiedPairing,
) -> AppResult<PairFinishResponse> {
    let id = Uuid::new_v4().to_string();
    let token = Uuid::new_v4().to_string();
    let now = chrono::Local::now().to_rfc3339();
    let client = ClientConnectionInfo {
        id: id.clone(),
        client_name: verified.client_name,
        client_device_id: verified.client_device_id,
        client_ip: verified.client_ip,
        app_version: verified.app_version,
        status: "paired".to_string(),
        last_seen_at: now,
    };
    {
        let mut runtime = runtime
            .lock()
            .map_err(|_| AppError::Validation("主机配对状态异常".to_string()))?;
        runtime.clients.insert(
            token.clone(),
            ClientConnectionInfo {
                id: token.clone(),
                ..client.clone()
            },
        );
    }
    db.with_conn(|conn| upsert_client_connection(conn, &client, &token_hash(&token)))?;
    Ok(PairFinishResponse {
        token,
        message: "配对成功".to_string(),
    })
}

fn status_from_runtime(runtime: &HostServiceRuntime) -> HostServiceStatus {
    HostServiceStatus {
        running: runtime.running,
        bind_address: runtime.bind_address.clone(),
        port: runtime.port,
        pair_code: runtime.pair_code.clone(),
        client_count: runtime.clients.len(),
        message: if runtime.running {
            format!("主机服务运行中：{}:{}", runtime.bind_address, runtime.port)
        } else {
            "主机服务未启动".to_string()
        },
    }
}

fn normalize_host_address(value: &str) -> AppResult<String> {
    let address = value.trim();
    if address.is_empty() {
        return Err(AppError::Validation("主机地址不能为空".to_string()));
    }
    if address.contains("://") || address.contains('/') {
        return Err(AppError::Validation(
            "主机地址只填写 IP 或主机名，不要包含 http://、端口或路径".to_string(),
        ));
    }
    if address.contains(':') && !address.contains('.') {
        return Err(AppError::Validation(
            "主机地址和端口请分开填写；IPv6 地址当前不作为局域网自动发现验收范围".to_string(),
        ));
    }
    Ok(address.to_string())
}

fn validate_host_port(port: u16) -> AppResult<()> {
    if port < 1024 {
        return Err(AppError::Validation(
            "主机端口必须在 1024-65535 之间，建议使用默认 17871".to_string(),
        ));
    }
    Ok(())
}

fn validate_pairing_request(
    pair_code: &str,
    client_name: &str,
    client_device_id: &str,
) -> AppResult<()> {
    let code = pair_code.trim();
    if code.len() != 12 || !code.chars().all(|item| item.is_ascii_digit()) {
        return Err(AppError::Validation("配对码必须是 12 位数字".to_string()));
    }
    if client_name.trim().is_empty() {
        return Err(AppError::Validation("客户端名称不能为空".to_string()));
    }
    if client_device_id.trim().is_empty() {
        return Err(AppError::Validation("设备 ID 不能为空".to_string()));
    }
    Ok(())
}

fn authenticate_request(request: &str, runtime: &Arc<Mutex<HostServiceRuntime>>) -> AppResult<()> {
    let token = header_value(request, "X-Aster-Client-Token")
        .ok_or_else(|| AppError::Unauthorized("缺少客户端连接凭据".to_string()))?;
    let mut runtime = runtime.lock().expect("host runtime mutex poisoned");
    let Some(client) = runtime
        .clients
        .values_mut()
        .find(|client| client.id == token)
    else {
        return Err(AppError::Unauthorized(
            "客户端连接凭据无效，请重新配对".to_string(),
        ));
    };
    client.last_seen_at = chrono::Local::now().to_rfc3339();
    client.status = "online".to_string();
    Ok(())
}

fn authenticate_request_and_touch_client(
    request: &str,
    runtime: &Arc<Mutex<HostServiceRuntime>>,
    db: &Db,
) -> AppResult<()> {
    let client_device_id = authenticate_request_and_load_client(request, runtime, db)?;
    db.with_conn(|conn| touch_client_connection(conn, &client_device_id, "online"))?;
    Ok(())
}

fn authenticate_request_and_load_client(
    request: &str,
    runtime: &Arc<Mutex<HostServiceRuntime>>,
    db: &Db,
) -> AppResult<String> {
    let token = header_value(request, "X-Aster-Client-Token")
        .ok_or_else(|| AppError::Unauthorized("缺少客户端连接凭据".to_string()))?;
    if authenticate_request(request, runtime).is_err() {
        let Some(persisted_client) =
            db.with_conn(|conn| find_client_connection_by_token_hash(conn, &token_hash(&token)))?
        else {
            return Err(AppError::Unauthorized(
                "客户端连接凭据无效，请重新配对".to_string(),
            ));
        };
        let mut runtime = runtime.lock().expect("host runtime mutex poisoned");
        runtime.clients.insert(
            token.clone(),
            ClientConnectionInfo {
                id: token.clone(),
                status: "online".to_string(),
                last_seen_at: chrono::Local::now().to_rfc3339(),
                ..persisted_client
            },
        );
    }
    Ok({
        let runtime = runtime.lock().expect("host runtime mutex poisoned");
        runtime
            .clients
            .values()
            .find(|client| client.id == token)
            .map(|client| client.client_device_id.clone())
            .ok_or_else(|| AppError::Unauthorized("客户端连接凭据无效，请重新配对".to_string()))?
    })
}

fn upsert_client_connection(
    conn: &rusqlite::Connection,
    client: &ClientConnectionInfo,
    token_hash: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO client_connections (
           id, client_name, client_device_id, token_hash, client_ip, app_version, status, last_seen_at, updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, CURRENT_TIMESTAMP)
         ON CONFLICT(client_device_id) DO UPDATE SET
           id = excluded.id,
           client_name = excluded.client_name,
           token_hash = excluded.token_hash,
           client_ip = excluded.client_ip,
           app_version = excluded.app_version,
           status = excluded.status,
           last_seen_at = excluded.last_seen_at,
           updated_at = CURRENT_TIMESTAMP",
        rusqlite::params![
            client.id,
            client.client_name,
            client.client_device_id,
            token_hash,
            client.client_ip,
            client.app_version,
            client.status,
            client.last_seen_at
        ],
    )?;
    Ok(())
}

fn find_client_connection_by_token_hash(
    conn: &rusqlite::Connection,
    token_hash: &str,
) -> AppResult<Option<ClientConnectionInfo>> {
    use rusqlite::OptionalExtension;

    Ok(conn
        .query_row(
            "SELECT id, client_name, client_device_id, COALESCE(client_ip, ''),
                    COALESCE(app_version, ''), status, last_seen_at
             FROM client_connections
             WHERE token_hash = ?1
             LIMIT 1",
            rusqlite::params![token_hash],
            |row| {
                Ok(ClientConnectionInfo {
                    id: row.get(0)?,
                    client_name: row.get(1)?,
                    client_device_id: row.get(2)?,
                    client_ip: row.get(3)?,
                    app_version: row.get(4)?,
                    status: row.get(5)?,
                    last_seen_at: row.get(6)?,
                })
            },
        )
        .optional()?)
}

fn touch_client_connection(
    conn: &rusqlite::Connection,
    client_device_id: &str,
    status: &str,
) -> AppResult<()> {
    conn.execute(
        "UPDATE client_connections
         SET status = ?2, last_seen_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
         WHERE client_device_id = ?1",
        rusqlite::params![client_device_id, status],
    )?;
    Ok(())
}

fn remove_client_connection_from_conn(
    conn: &rusqlite::Connection,
    client_device_id: &str,
) -> AppResult<ClientConnectionInfo> {
    let client = conn
        .query_row(
            "SELECT id, client_name, client_device_id, COALESCE(client_ip, ''),
                    COALESCE(app_version, ''), status, last_seen_at
             FROM client_connections
             WHERE client_device_id = ?1",
            rusqlite::params![client_device_id],
            |row| {
                Ok(ClientConnectionInfo {
                    id: row.get(0)?,
                    client_name: row.get(1)?,
                    client_device_id: row.get(2)?,
                    client_ip: row.get(3)?,
                    app_version: row.get(4)?,
                    status: row.get(5)?,
                    last_seen_at: row.get(6)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| AppError::Validation("客户端设备不存在".to_string()))?;
    conn.execute(
        "DELETE FROM client_connections WHERE client_device_id = ?1",
        rusqlite::params![client_device_id],
    )?;
    Ok(client)
}

fn token_hash(token: &str) -> String {
    crate::services::remote_session_service::token_hash(token)
}

fn header_value(request: &str, key: &str) -> Option<String> {
    http_transport::header_value(request, key)
}

fn http_get_json<T: for<'de> Deserialize<'de>>(
    config: &ClientRuntimeConfig,
    path: &str,
) -> AppResult<T> {
    let mut stream = secure_transport::connect(
        &config.address,
        config.port,
        Some(&config.certificate_fingerprint),
    )?
    .stream;
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: aster\r\nX-Aster-Client-Token: {}\r\n{}Connection: close\r\n\r\n",
        config.token,
        session_header(config)
    );
    stream.write_all(request.as_bytes())?;
    http_transport::read_json_response(stream)
}

fn collect_remote_pages<T: for<'de> Deserialize<'de>>(
    config: &ClientRuntimeConfig,
    path: &str,
) -> AppResult<Vec<T>> {
    crate::application::remote_pagination::collect_all(|cursor| {
        http_get_json(config, &page_path(path, cursor))
    })
}

fn http_post_json<T: Serialize, R: for<'de> Deserialize<'de>>(
    config: &ClientRuntimeConfig,
    path: &str,
    body: &T,
) -> AppResult<R> {
    let body = serde_json::to_string(body)
        .map_err(|error| AppError::Validation(format!("JSON 序列化失败：{error}")))?;
    let mut stream = secure_transport::connect(
        &config.address,
        config.port,
        Some(&config.certificate_fingerprint),
    )?
    .stream;
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: aster\r\nX-Aster-Client-Token: {}\r\n{}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        config.token,
        session_header(config),
        body.len(),
        body
    );
    stream.write_all(request.as_bytes())?;
    http_transport::read_json_response(stream)
}

fn session_header(config: &ClientRuntimeConfig) -> String {
    config
        .session_token
        .as_deref()
        .map(|token| format!("X-Aster-Session-Token: {token}\r\n"))
        .unwrap_or_default()
}

fn http_post_json_for_pairing<T: Serialize, R: for<'de> Deserialize<'de>>(
    address: &str,
    port: u16,
    path: &str,
    body: &T,
    expected_fingerprint: Option<&str>,
) -> AppResult<(R, String)> {
    let body = serde_json::to_string(body)
        .map_err(|error| AppError::Validation(format!("JSON 序列化失败：{error}")))?;
    let connected = secure_transport::connect(address, port, expected_fingerprint)?;
    let fingerprint = connected.fingerprint;
    let mut stream = connected.stream;
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: aster\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(request.as_bytes())?;
    Ok((http_transport::read_json_response(stream)?, fingerprint))
}

fn write_json<T: Serialize>(stream: &mut impl Write, status: u16, body: &T) -> AppResult<()> {
    http_transport::write_json(stream, status, body)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    app_name: String,
    app_version: String,
    schema_version: i64,
    database_ok: bool,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionResponse {
    app_name: String,
    app_version: String,
    schema_version: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiscoveryResponse {
    host_address: String,
    host_port: u16,
    app_name: String,
    app_version: String,
    schema_version: i64,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetEnabledRequest {
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
}

#[cfg(test)]
mod tests;
