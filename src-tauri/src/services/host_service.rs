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

mod auth;
mod client_registry;
mod http_client;
mod remote_master_data;
mod remote_operations;
mod remote_stock;
mod responses;
mod routes;
mod server;

use auth::*;
use client_registry::*;
use http_client::*;
pub use remote_master_data::*;
pub use remote_operations::*;
pub use remote_stock::*;
use responses::*;
use routes::*;
use server::*;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetEnabledRequest {
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
}

#[cfg(test)]
mod tests;
