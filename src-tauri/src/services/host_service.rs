use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::db::approval_repository;
use crate::db::connection::Db;
use crate::db::master_data_repository;
use crate::db::report_repository;
use crate::db::repository;
use crate::db::stock_repository;
use crate::db::stocktake_repository;
use crate::domain::approvals::{ApprovalRequest, CreateApprovalRequest, DecideApprovalRequest};
use crate::domain::backups::BackupRecord;
use crate::domain::master_data::{
    BudgetRule, Category, Department, Item, SaveBudgetRuleRequest, SaveCategoryRequest,
    SaveDepartmentRequest, SaveItemRequest, SaveSupplierRequest, SaveUnitRequest, Supplier, Unit,
};
use crate::domain::reports::{ReportBundle, ReportQuery};
use crate::domain::runtime::{
    ClientConnectionInfo, HostConnectionTestRequest, HostConnectionTestResult, HostDiscoveryResult,
    HostServiceStatus, SaveClientConfigRequest,
};
use crate::domain::status::{AppStatus, AuditLogRow, SystemSettings};
use crate::domain::stock::{
    ConfirmStockDocumentDraftRequest, SaveStockDocumentDraftRequest, StockBalanceQuery,
    StockBalanceRow, StockDocument, StockDocumentQuery, StockMovementQuery, StockMovementRow,
    SubmitAdjustmentRequest, SubmitStockDocumentRequest, VoidStockDocumentRequest,
};
use crate::domain::stocktake::{
    ConfirmStocktakeRequest, CreateStocktakeRequest, StocktakeDetail, StocktakeDocument,
    UpdateStocktakeCountsRequest,
};
use crate::domain::users::{
    ChangePasswordRequest, CurrentUser, LoginRequest, RequestPasswordResetCodeRequest,
    RequestPasswordResetCodeResponse, ResetPasswordWithCodeRequest, Role, SaveUserRequest,
    SetUserEnabledRequest, UserAccount,
};
use crate::error::{AppError, AppResult};
use crate::{app::state::AppState, domain::runtime::RuntimeMode};

#[derive(Default)]
pub struct HostServiceRuntime {
    pub running: bool,
    pub bind_address: String,
    pub port: u16,
    pub pair_code: Option<String>,
    pub clients: HashMap<String, ClientConnectionInfo>,
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

    let pair_code = generate_pair_code();
    {
        let mut runtime = state
            .host_service
            .lock()
            .expect("host runtime mutex poisoned");
        runtime.running = true;
        runtime.bind_address = bind_address.clone();
        runtime.port = port;
        runtime.pair_code = Some(pair_code.clone());
        runtime.clients.clear();
    }

    let runtime = Arc::clone(&state.host_service);
    let db = state.db.clone_handle();
    let version = app_version.to_string();
    thread::spawn(move || {
        serve(listener, runtime, db, version);
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
    state.db.with_conn(list_client_connections_from_conn)
}

pub fn save_client_config(
    state: &AppState,
    request: SaveClientConfigRequest,
) -> AppResult<crate::domain::runtime::RuntimeConfig> {
    crate::services::user_service::require_admin(state)?;
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
    crate::services::user_service::require_admin(state)?;
    validate_pairing_request(&pair_code, &client_name, &client_device_id)?;
    let config = crate::services::status_service::get_runtime_config(state)?;
    let address = config
        .host_address
        .clone()
        .ok_or_else(|| AppError::Validation("请先保存主机地址".to_string()))?;
    let response: PairResponse = http_post_json_without_token(
        &address,
        config.host_port,
        "/api/pair",
        &PairRequest {
            pair_code,
            client_name,
            client_device_id,
            app_version,
        },
    )?;
    if !response.ok {
        return Err(AppError::Validation(response.message));
    }
    let token = response
        .token
        .ok_or_else(|| AppError::Validation("主机未返回连接凭据".to_string()))?;
    state.db.with_conn(|conn| {
        repository::set_setting(conn, "client_token", &token)?;
        Ok(())
    })?;
    crate::services::status_service::get_runtime_config(state)
}

pub fn test_host_connection(
    request: HostConnectionTestRequest,
) -> AppResult<HostConnectionTestResult> {
    validate_host_port(request.host_port)?;
    let host_address = normalize_host_address(&request.host_address)?;
    let mut stream = TcpStream::connect((host_address.as_str(), request.host_port))
        .map_err(|error| AppError::Validation(format!("连接主机失败：{error}")))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|error| AppError::Validation(format!("连接超时设置失败：{error}")))?;
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

pub fn remote_list_stock_documents(
    state: &AppState,
    query: StockDocumentQuery,
) -> AppResult<Vec<StockDocument>> {
    let config = client_runtime_config(state)?;
    let mut params = Vec::new();
    push_query_param(&mut params, "documentType", query.document_type);
    push_query_param(&mut params, "month", query.month);
    push_query_param(&mut params, "departmentId", query.department_id);
    push_query_param(&mut params, "supplierId", query.supplier_id);
    push_query_param(&mut params, "itemId", query.item_id);
    push_query_param(&mut params, "search", query.search);
    let path = if params.is_empty() {
        "/api/stock/documents".to_string()
    } else {
        format!("/api/stock/documents?{}", params.join("&"))
    };
    http_get_json(&config, &path)
}

pub fn remote_list_stock_balances(
    state: &AppState,
    query: StockBalanceQuery,
) -> AppResult<Vec<StockBalanceRow>> {
    let config = client_runtime_config(state)?;
    let mut params = Vec::new();
    push_query_param(&mut params, "search", query.search);
    push_query_param(&mut params, "categoryId", query.category_id);
    push_query_param(&mut params, "itemId", query.item_id);
    push_query_param(&mut params, "stockStatus", query.stock_status);
    let path = if params.is_empty() {
        "/api/stock/balances".to_string()
    } else {
        format!("/api/stock/balances?{}", params.join("&"))
    };
    http_get_json(&config, &path)
}

pub fn remote_list_stock_movements(
    state: &AppState,
    query: StockMovementQuery,
) -> AppResult<Vec<StockMovementRow>> {
    let config = client_runtime_config(state)?;
    let mut params = Vec::new();
    push_query_param(&mut params, "search", query.search);
    push_query_param(&mut params, "itemId", query.item_id);
    push_query_param(&mut params, "departmentId", query.department_id);
    push_query_param(&mut params, "direction", query.direction);
    push_query_param(&mut params, "movementType", query.movement_type);
    let path = if params.is_empty() {
        "/api/stock/movements".to_string()
    } else {
        format!("/api/stock/movements?{}", params.join("&"))
    };
    http_get_json(&config, &path)
}

pub fn remote_submit_stock_document(
    state: &AppState,
    request: SubmitStockDocumentRequest,
) -> AppResult<crate::domain::stock::StockDocumentDetail> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/stock/document", &request)
}

pub fn remote_save_stock_document_draft(
    state: &AppState,
    request: SaveStockDocumentDraftRequest,
) -> AppResult<crate::domain::stock::StockDocumentDetail> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/stock/document/draft", &request)
}

pub fn remote_confirm_stock_document_draft(
    state: &AppState,
    request: ConfirmStockDocumentDraftRequest,
) -> AppResult<crate::domain::stock::StockDocumentDetail> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/stock/document/draft/confirm", &request)
}

pub fn remote_submit_adjustment(
    state: &AppState,
    request: SubmitAdjustmentRequest,
) -> AppResult<crate::domain::stock::StockDocumentDetail> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/stock/adjustment", &request)
}

pub fn remote_void_stock_document(
    state: &AppState,
    request: VoidStockDocumentRequest,
) -> AppResult<crate::domain::stock::StockDocumentDetail> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/stock/void", &request)
}

pub fn remote_list_categories(state: &AppState) -> AppResult<Vec<Category>> {
    let config = client_runtime_config(state)?;
    http_get_json(&config, "/api/master/categories")
}

pub fn remote_save_category(state: &AppState, request: SaveCategoryRequest) -> AppResult<Category> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/master/category", &request)
}

pub fn remote_set_category_enabled(
    state: &AppState,
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
) -> AppResult<()> {
    let config = client_runtime_config(state)?;
    http_post_json(
        &config,
        "/api/master/category/enabled",
        &SetEnabledRequest {
            id,
            enabled,
            expected_updated_at,
        },
    )
}

pub fn remote_list_units(state: &AppState) -> AppResult<Vec<Unit>> {
    let config = client_runtime_config(state)?;
    http_get_json(&config, "/api/master/units")
}

pub fn remote_save_unit(state: &AppState, request: SaveUnitRequest) -> AppResult<Unit> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/master/unit", &request)
}

pub fn remote_set_unit_enabled(
    state: &AppState,
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
) -> AppResult<()> {
    let config = client_runtime_config(state)?;
    http_post_json(
        &config,
        "/api/master/unit/enabled",
        &SetEnabledRequest {
            id,
            enabled,
            expected_updated_at,
        },
    )
}

pub fn remote_list_departments(state: &AppState) -> AppResult<Vec<Department>> {
    let config = client_runtime_config(state)?;
    http_get_json(&config, "/api/master/departments")
}

pub fn remote_save_department(
    state: &AppState,
    request: SaveDepartmentRequest,
) -> AppResult<Department> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/master/department", &request)
}

pub fn remote_set_department_enabled(
    state: &AppState,
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
) -> AppResult<()> {
    let config = client_runtime_config(state)?;
    http_post_json(
        &config,
        "/api/master/department/enabled",
        &SetEnabledRequest {
            id,
            enabled,
            expected_updated_at,
        },
    )
}

pub fn remote_list_suppliers(state: &AppState) -> AppResult<Vec<Supplier>> {
    let config = client_runtime_config(state)?;
    http_get_json(&config, "/api/master/suppliers")
}

pub fn remote_list_supplier_purchase_records(
    state: &AppState,
    supplier_id: String,
) -> AppResult<Vec<crate::domain::master_data::SupplierPurchaseRecord>> {
    let config = client_runtime_config(state)?;
    http_get_json(
        &config,
        &format!(
            "/api/master/supplier/purchases?supplierId={}",
            url_encode(&supplier_id)
        ),
    )
}

pub fn remote_save_supplier(state: &AppState, request: SaveSupplierRequest) -> AppResult<Supplier> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/master/supplier", &request)
}

pub fn remote_set_supplier_enabled(
    state: &AppState,
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
) -> AppResult<()> {
    let config = client_runtime_config(state)?;
    http_post_json(
        &config,
        "/api/master/supplier/enabled",
        &SetEnabledRequest {
            id,
            enabled,
            expected_updated_at,
        },
    )
}

pub fn remote_list_items(state: &AppState, search: Option<String>) -> AppResult<Vec<Item>> {
    let config = client_runtime_config(state)?;
    let path = if let Some(search) = search {
        format!("/api/master/items?search={}", url_encode(&search))
    } else {
        "/api/master/items".to_string()
    };
    http_get_json(&config, &path)
}

pub fn remote_save_item(state: &AppState, request: SaveItemRequest) -> AppResult<Item> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/master/item", &request)
}

pub fn remote_set_item_enabled(
    state: &AppState,
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
) -> AppResult<()> {
    let config = client_runtime_config(state)?;
    http_post_json(
        &config,
        "/api/master/item/enabled",
        &SetEnabledRequest {
            id,
            enabled,
            expected_updated_at,
        },
    )
}

pub fn remote_list_budget_rules(
    state: &AppState,
    period_month: Option<String>,
) -> AppResult<Vec<BudgetRule>> {
    let config = client_runtime_config(state)?;
    let path = if let Some(period_month) = period_month {
        format!(
            "/api/master/budget-rules?month={}",
            url_encode(&period_month)
        )
    } else {
        "/api/master/budget-rules".to_string()
    };
    http_get_json(&config, &path)
}

pub fn remote_save_budget_rule(
    state: &AppState,
    request: SaveBudgetRuleRequest,
) -> AppResult<BudgetRule> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/master/budget-rule", &request)
}

pub fn remote_set_budget_rule_enabled(
    state: &AppState,
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
) -> AppResult<()> {
    let config = client_runtime_config(state)?;
    http_post_json(
        &config,
        "/api/master/budget-rule/enabled",
        &SetEnabledRequest {
            id,
            enabled,
            expected_updated_at,
        },
    )
}

pub fn remote_get_report_bundle(state: &AppState, query: ReportQuery) -> AppResult<ReportBundle> {
    let config = client_runtime_config(state)?;
    let mut params = vec![format!("month={}", url_encode(&query.month))];
    push_query_param(&mut params, "startDate", query.start_date);
    push_query_param(&mut params, "endDate", query.end_date);
    push_query_param(&mut params, "departmentId", query.department_id);
    push_query_param(&mut params, "categoryId", query.category_id);
    push_query_param(&mut params, "itemId", query.item_id);
    push_query_param(&mut params, "supplierId", query.supplier_id);
    let path = format!("/api/reports/monthly?{}", params.join("&"));
    http_get_json(&config, &path)
}

pub fn remote_create_stocktake(
    state: &AppState,
    request: CreateStocktakeRequest,
) -> AppResult<StocktakeDetail> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/stocktakes", &request)
}

pub fn remote_list_stocktakes(state: &AppState) -> AppResult<Vec<StocktakeDocument>> {
    let config = client_runtime_config(state)?;
    http_get_json(&config, "/api/stocktakes")
}

pub fn remote_get_stocktake_detail(
    state: &AppState,
    stocktake_id: String,
) -> AppResult<StocktakeDetail> {
    let config = client_runtime_config(state)?;
    http_get_json(
        &config,
        &format!("/api/stocktake?stocktakeId={}", url_encode(&stocktake_id)),
    )
}

pub fn remote_update_stocktake_counts(
    state: &AppState,
    request: UpdateStocktakeCountsRequest,
) -> AppResult<StocktakeDetail> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/stocktake/counts", &request)
}

pub fn remote_confirm_stocktake(
    state: &AppState,
    request: ConfirmStocktakeRequest,
) -> AppResult<StocktakeDetail> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/stocktake/confirm", &request)
}

pub fn remote_list_approval_requests(state: &AppState) -> AppResult<Vec<ApprovalRequest>> {
    let config = client_runtime_config(state)?;
    http_get_json(&config, "/api/approvals")
}

pub fn remote_create_approval_request(
    state: &AppState,
    request: CreateApprovalRequest,
) -> AppResult<ApprovalRequest> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/approval", &request)
}

pub fn remote_decide_approval_request(
    state: &AppState,
    request: DecideApprovalRequest,
) -> AppResult<ApprovalRequest> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/approval/decision", &request)
}

pub fn remote_list_audit_logs(state: &AppState, limit: Option<i64>) -> AppResult<Vec<AuditLogRow>> {
    let config = client_runtime_config(state)?;
    let path = if let Some(limit) = limit {
        format!("/api/audit-logs?limit={}", limit.clamp(1, 500))
    } else {
        "/api/audit-logs".to_string()
    };
    http_get_json(&config, &path)
}

pub fn remote_get_app_status(state: &AppState) -> AppResult<AppStatus> {
    let config = client_runtime_config(state)?;
    http_get_json(&config, "/api/status")
}

pub fn remote_get_system_settings(state: &AppState) -> AppResult<SystemSettings> {
    let config = client_runtime_config(state)?;
    http_get_json(&config, "/api/system-settings")
}

pub fn remote_list_backup_records(state: &AppState) -> AppResult<Vec<BackupRecord>> {
    let config = client_runtime_config(state)?;
    http_get_json(&config, "/api/backups")
}

pub fn remote_list_users(state: &AppState) -> AppResult<Vec<UserAccount>> {
    let config = client_runtime_config(state)?;
    http_get_json(&config, "/api/users")
}

pub fn remote_list_roles(state: &AppState) -> AppResult<Vec<Role>> {
    let config = client_runtime_config(state)?;
    http_get_json(&config, "/api/roles")
}

pub fn remote_save_user(state: &AppState, request: SaveUserRequest) -> AppResult<UserAccount> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/user", &request)
}

pub fn remote_set_user_enabled(state: &AppState, request: SetUserEnabledRequest) -> AppResult<()> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/user/enabled", &request)
}

pub fn remote_login(state: &AppState, request: LoginRequest) -> AppResult<CurrentUser> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/login", &request)
}

pub fn remote_change_password(state: &AppState, request: ChangePasswordRequest) -> AppResult<()> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/user/password", &request)
}

pub fn remote_request_password_reset_code(
    state: &AppState,
    request: RequestPasswordResetCodeRequest,
) -> AppResult<RequestPasswordResetCodeResponse> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/password-reset/request", &request)
}

pub fn remote_reset_password_with_code(
    state: &AppState,
    request: ResetPasswordWithCodeRequest,
) -> AppResult<()> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/password-reset/confirm", &request)
}

fn serve(
    listener: TcpListener,
    runtime: Arc<Mutex<HostServiceRuntime>>,
    db: Db,
    app_version: String,
) {
    loop {
        let running = runtime
            .lock()
            .map(|runtime| runtime.running)
            .unwrap_or(false);
        if !running {
            break;
        }
        match listener.accept() {
            Ok((stream, _addr)) => {
                let runtime = Arc::clone(&runtime);
                let db = db.clone_handle();
                let version = app_version.clone();
                thread::spawn(move || {
                    let _ = handle_connection(stream, runtime, db, &version);
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
                let schema_version = db
                    .with_conn(|conn| repository::schema_version(conn))
                    .unwrap_or_default();
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
    mut stream: TcpStream,
    runtime: Arc<Mutex<HostServiceRuntime>>,
    db: Db,
    app_version: &str,
) -> AppResult<()> {
    if let Err(error) = handle_connection_inner(&mut stream, runtime, db, app_version) {
        write_json(
            &mut stream,
            400,
            &serde_json::json!({ "message": error.to_string() }),
        )?;
    }
    Ok(())
}

fn handle_connection_inner(
    stream: &mut TcpStream,
    runtime: Arc<Mutex<HostServiceRuntime>>,
    db: Db,
    app_version: &str,
) -> AppResult<()> {
    let peer_ip = stream
        .peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "-".to_string());
    let request = read_http_request(stream)?;
    let (method, path) = parse_request_line(&request);
    let body = request.split("\r\n\r\n").nth(1).unwrap_or("");
    let auth_request = request.clone();

    match (method.as_str(), path.as_str()) {
        ("GET", "/api/health") => {
            let response = health_response(&db, app_version)?;
            write_json(stream, 200, &response)?;
        }
        ("GET", "/api/version") => {
            let response = version_response(&db, app_version)?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/pair") => {
            let request: PairRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("配对请求解析失败：{error}")))?;
            let response = pair_client(&runtime, &db, request, peer_ip)?;
            write_json(stream, 200, &response)?;
        }
        ("GET", "/api/clients") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            db.with_conn(|conn| require_remote_admin(&auth_request, conn))?;
            let clients = db.with_conn(list_client_connections_from_conn)?;
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
        ("GET", "/api/backups") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let response = db.with_conn(|conn| {
                require_remote_admin(&auth_request, conn)?;
                crate::db::backup_repository::list_backup_records(conn)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", "/api/users") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let response = db.with_conn(|conn| {
                require_remote_admin(&auth_request, conn)?;
                crate::db::user_repository::list_users(conn)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", "/api/roles") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let response = db.with_conn(|conn| {
                require_remote_admin(&auth_request, conn)?;
                crate::db::user_repository::list_roles(conn)
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
            let request: LoginRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("登录请求解析失败：{error}")))?;
            let response =
                db.with_conn(|conn| crate::services::user_service::login_on_conn(conn, request))?;
            write_json(stream, 200, &response)?;
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
        ("GET", path) if path.starts_with("/api/audit-logs") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let limit = query_param(path, "limit").and_then(|value| value.parse::<i64>().ok());
            let response = db.with_conn(|conn| {
                require_remote_admin(&auth_request, conn)?;
                repository::list_audit_logs(conn, limit.unwrap_or(100))
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if path.starts_with("/api/stock/documents") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let query = StockDocumentQuery {
                document_type: query_param(path, "documentType"),
                month: query_param(path, "month"),
                department_id: query_param(path, "departmentId"),
                supplier_id: query_param(path, "supplierId"),
                item_id: query_param(path, "itemId"),
                search: query_param(path, "search"),
            };
            let response = db.with_conn(|conn| {
                let current = require_remote_permission(&auth_request, conn, "view_reports")?;
                let mut query = query;
                query.department_id = remote_department_scope(&current)?.or(query.department_id);
                stock_repository::list_stock_documents(conn, query)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if path.starts_with("/api/stock/balances") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let query = StockBalanceQuery {
                search: query_param(path, "search"),
                category_id: query_param(path, "categoryId"),
                item_id: query_param(path, "itemId"),
                stock_status: query_param(path, "stockStatus"),
            };
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                stock_repository::list_stock_balances(conn, query)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if path.starts_with("/api/stock/movements") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let query = StockMovementQuery {
                search: query_param(path, "search"),
                item_id: query_param(path, "itemId"),
                department_id: query_param(path, "departmentId"),
                direction: query_param(path, "direction"),
                movement_type: query_param(path, "movementType"),
            };
            let response = db.with_conn(|conn| {
                let current = require_remote_permission(&auth_request, conn, "view_reports")?;
                let mut query = query;
                query.department_id = remote_department_scope(&current)?.or(query.department_id);
                stock_repository::list_stock_movements(conn, query)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/stock/document") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: SubmitStockDocumentRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("单据请求解析失败：{error}")))?;
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
            let request: SaveStockDocumentDraftRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("草稿请求解析失败：{error}")))?;
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
            let request: SubmitAdjustmentRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("调整请求解析失败：{error}")))?;
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
        ("GET", "/api/master/categories") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                master_data_repository::list_categories(conn)
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
        ("GET", "/api/master/units") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                master_data_repository::list_units(conn)
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
        ("GET", "/api/master/departments") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                master_data_repository::list_departments(conn)
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
        ("GET", "/api/master/suppliers") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                master_data_repository::list_suppliers(conn)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if path.starts_with("/api/master/supplier/purchases") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let supplier_id = query_param(path, "supplierId")
                .ok_or_else(|| AppError::Validation("缺少供应商 ID".to_string()))?;
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                master_data_repository::list_supplier_purchase_records(conn, &supplier_id)
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
        ("GET", path) if path.starts_with("/api/master/items") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let search = query_param(path, "search");
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                master_data_repository::list_items(conn, search)
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
        ("GET", path) if path.starts_with("/api/master/budget-rules") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let month = query_param(path, "month");
            let response = db.with_conn(|conn| {
                require_remote_admin(&auth_request, conn)?;
                master_data_repository::list_budget_rules(conn, month)
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
        ("GET", path) if path.starts_with("/api/reports/monthly") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let month = query_param(path, "month")
                .ok_or_else(|| AppError::Validation("报表月份不能为空".to_string()))?;
            let start_date = query_param(path, "startDate");
            let end_date = query_param(path, "endDate");
            let department_id = query_param(path, "departmentId");
            let category_id = query_param(path, "categoryId");
            let item_id = query_param(path, "itemId");
            let supplier_id = query_param(path, "supplierId");
            let response = db.with_conn(|conn| {
                let current = require_remote_permission(&auth_request, conn, "view_reports")?;
                let scoped_department_id = remote_department_scope(&current)?.or(department_id);
                report_repository::get_report_bundle(
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
                )
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", "/api/stocktakes") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let response = db.with_conn(|conn| {
                require_remote_permission(&auth_request, conn, "view_reports")?;
                stocktake_repository::list_stocktakes(conn)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/stocktakes") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let request: CreateStocktakeRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("盘点创建请求解析失败：{error}")))?;
            crate::services::stocktake_service::validate_create_request(&request)?;
            let response = db.with_conn_mut(|conn| {
                require_remote_permission(&auth_request, conn, "write_stock")?;
                stocktake_repository::create_stocktake(conn, request)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if path.starts_with("/api/stocktake?") => {
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
        ("GET", "/api/approvals") => {
            authenticate_request_and_touch_client(&request, &runtime, &db)?;
            let response = db.with_conn(|conn| {
                require_remote_admin(&auth_request, conn)?;
                approval_repository::list_approval_requests(conn)
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

#[derive(Clone)]
struct ClientRuntimeConfig {
    address: String,
    port: u16,
    token: String,
    user_id: Option<String>,
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
    Ok(ClientRuntimeConfig {
        address,
        port: config.host_port,
        token,
        user_id: crate::services::user_service::current_user(state)?.map(|user| user.id),
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
    let user_id = header_value(request, "X-Aster-User-Id")
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::Validation("远程请求缺少当前用户".to_string()))?;
    let Some((user, _hash)) = crate::db::user_repository::find_user_by_id(conn, &user_id)? else {
        return Err(AppError::Validation("远程当前用户不存在".to_string()));
    };
    if !user.enabled {
        return Err(AppError::Validation("远程当前用户已停用".to_string()));
    }
    Ok(to_remote_current_user(user))
}

fn require_remote_admin(request: &str, conn: &rusqlite::Connection) -> AppResult<CurrentUser> {
    let current = remote_current_user(request, conn)?;
    if current.roles.iter().any(|role| role.code == "admin") {
        Ok(current)
    } else {
        Err(AppError::Validation("需要管理员权限".to_string()))
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
        Err(AppError::Validation(format!("缺少权限：{permission}")))
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

fn to_remote_current_user(user: UserAccount) -> CurrentUser {
    let mut permissions = Vec::new();
    for role in &user.roles {
        match role.code.as_str() {
            "admin" => permissions.extend([
                "manage_users",
                "manage_settings",
                "write_stock",
                "view_reports",
                "dangerous_operations",
            ]),
            "warehouse" => permissions.extend(["write_stock", "view_reports"]),
            "department_viewer" => permissions.extend(["view_reports"]),
            "readonly" => permissions.extend(["view_reports"]),
            _ => {}
        }
    }
    permissions.sort();
    permissions.dedup();
    CurrentUser {
        id: user.id,
        username: user.username,
        display_name: user.display_name,
        department_id: user.department_id,
        department_name: user.department_name,
        roles: user.roles,
        permissions: permissions.into_iter().map(str::to_string).collect(),
    }
}

fn read_http_request(stream: &mut TcpStream) -> AppResult<String> {
    let mut buffer = Vec::new();
    let mut temp = [0_u8; 4096];
    let header_end;
    loop {
        let bytes = stream.read(&mut temp)?;
        if bytes == 0 {
            header_end = buffer.len();
            break;
        }
        buffer.extend_from_slice(&temp[..bytes]);
        if let Some(index) = find_header_end(&buffer) {
            header_end = index;
            break;
        }
        if buffer.len() > 64 * 1024 {
            return Err(AppError::Validation("HTTP 请求头过大".to_string()));
        }
    }
    let header_text = String::from_utf8_lossy(&buffer[..header_end]).to_string();
    let content_length = header_value(&header_text, "Content-Length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let body_start = header_end + 4;
    while buffer.len().saturating_sub(body_start) < content_length {
        let bytes = stream.read(&mut temp)?;
        if bytes == 0 {
            break;
        }
        buffer.extend_from_slice(&temp[..bytes]);
    }
    Ok(String::from_utf8_lossy(&buffer).to_string())
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
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

fn pair_client(
    runtime: &Arc<Mutex<HostServiceRuntime>>,
    db: &Db,
    request: PairRequest,
    client_ip: String,
) -> AppResult<PairResponse> {
    let id = Uuid::new_v4().to_string();
    let token = Uuid::new_v4().to_string();
    let now = chrono::Local::now().to_rfc3339();
    let client = ClientConnectionInfo {
        id: id.clone(),
        client_name: request.client_name,
        client_device_id: request.client_device_id,
        client_ip,
        app_version: request.app_version,
        status: "paired".to_string(),
        last_seen_at: now,
    };
    {
        let mut runtime = runtime.lock().expect("host runtime mutex poisoned");
        let expected = runtime.pair_code.clone().unwrap_or_default();
        if request.pair_code.trim() != expected {
            return Ok(PairResponse {
                ok: false,
                token: None,
                message: "配对码错误".to_string(),
            });
        }
        runtime.clients.insert(
            token.clone(),
            ClientConnectionInfo {
                id: token.clone(),
                ..client.clone()
            },
        );
        runtime.pair_code = Some(generate_pair_code());
    }
    db.with_conn(|conn| upsert_client_connection(conn, &client, &token_hash(&token)))?;
    Ok(PairResponse {
        ok: true,
        token: Some(token),
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

fn parse_request_line(request: &str) -> (String, String) {
    let mut parts = request
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    (
        parts.next().unwrap_or_default().to_string(),
        parts.next().unwrap_or_default().to_string(),
    )
}

fn query_param(path: &str, key: &str) -> Option<String> {
    let query = path.split_once('?')?.1;
    for pair in query.split('&') {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        if name == key {
            return Some(url_decode(value));
        }
    }
    None
}

fn push_query_param(params: &mut Vec<String>, key: &str, value: Option<String>) {
    let Some(value) = value else {
        return;
    };
    let value = value.trim();
    if !value.is_empty() {
        params.push(format!("{key}={}", url_encode(value)));
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
    if code.len() != 6 || !code.chars().all(|item| item.is_ascii_digit()) {
        return Err(AppError::Validation("配对码必须是 6 位数字".to_string()));
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
        .ok_or_else(|| AppError::Validation("缺少客户端连接凭据".to_string()))?;
    let mut runtime = runtime.lock().expect("host runtime mutex poisoned");
    let Some(client) = runtime
        .clients
        .values_mut()
        .find(|client| client.id == token)
    else {
        return Err(AppError::Validation(
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
    let token = header_value(request, "X-Aster-Client-Token")
        .ok_or_else(|| AppError::Validation("缺少客户端连接凭据".to_string()))?;
    if authenticate_request(request, runtime).is_err() {
        let Some(persisted_client) =
            db.with_conn(|conn| find_client_connection_by_token_hash(conn, &token_hash(&token)))?
        else {
            return Err(AppError::Validation(
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
    let client_device_id = {
        let runtime = runtime.lock().expect("host runtime mutex poisoned");
        runtime
            .clients
            .values()
            .find(|client| client.id == token)
            .map(|client| client.client_device_id.clone())
            .ok_or_else(|| AppError::Validation("客户端连接凭据无效，请重新配对".to_string()))?
    };
    db.with_conn(|conn| touch_client_connection(conn, &client_device_id, "online"))?;
    Ok(())
}

fn list_client_connections_from_conn(
    conn: &rusqlite::Connection,
) -> AppResult<Vec<ClientConnectionInfo>> {
    let mut stmt = conn.prepare(
        "SELECT id, client_name, client_device_id, COALESCE(client_ip, ''),
                COALESCE(app_version, ''), status, last_seen_at
         FROM client_connections
         ORDER BY last_seen_at DESC, updated_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ClientConnectionInfo {
            id: row.get(0)?,
            client_name: row.get(1)?,
            client_device_id: row.get(2)?,
            client_ip: row.get(3)?,
            app_version: row.get(4)?,
            status: row.get(5)?,
            last_seen_at: row.get(6)?,
        })
    })?;
    let mut clients = Vec::new();
    for row in rows {
        clients.push(row?);
    }
    Ok(clients)
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

fn token_hash(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn header_value(request: &str, key: &str) -> Option<String> {
    request.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case(key) {
            Some(value.trim().to_string())
        } else {
            None
        }
    })
}

fn http_get_json<T: for<'de> Deserialize<'de>>(
    config: &ClientRuntimeConfig,
    path: &str,
) -> AppResult<T> {
    let mut stream = TcpStream::connect((config.address.as_str(), config.port))
        .map_err(|error| AppError::Validation(format!("连接主机失败：{error}")))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(8)))
        .map_err(|error| AppError::Validation(format!("连接超时设置失败：{error}")))?;
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: aster\r\nX-Aster-Client-Token: {}\r\nX-Aster-User-Id: {}\r\nConnection: close\r\n\r\n",
        config.token,
        config.user_id.as_deref().unwrap_or("")
    );
    stream.write_all(request.as_bytes())?;
    read_http_json(stream)
}

fn http_post_json<T: Serialize, R: for<'de> Deserialize<'de>>(
    config: &ClientRuntimeConfig,
    path: &str,
    body: &T,
) -> AppResult<R> {
    let body = serde_json::to_string(body)
        .map_err(|error| AppError::Validation(format!("JSON 序列化失败：{error}")))?;
    let mut stream = TcpStream::connect((config.address.as_str(), config.port))
        .map_err(|error| AppError::Validation(format!("连接主机失败：{error}")))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(8)))
        .map_err(|error| AppError::Validation(format!("连接超时设置失败：{error}")))?;
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: aster\r\nX-Aster-Client-Token: {}\r\nX-Aster-User-Id: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        config.token,
        config.user_id.as_deref().unwrap_or(""),
        body.as_bytes().len(),
        body
    );
    stream.write_all(request.as_bytes())?;
    read_http_json(stream)
}

fn http_post_json_without_token<T: Serialize, R: for<'de> Deserialize<'de>>(
    address: &str,
    port: u16,
    path: &str,
    body: &T,
) -> AppResult<R> {
    let body = serde_json::to_string(body)
        .map_err(|error| AppError::Validation(format!("JSON 序列化失败：{error}")))?;
    let mut stream = TcpStream::connect((address, port))
        .map_err(|error| AppError::Validation(format!("连接主机失败：{error}")))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(8)))
        .map_err(|error| AppError::Validation(format!("连接超时设置失败：{error}")))?;
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: aster\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.as_bytes().len(),
        body
    );
    stream.write_all(request.as_bytes())?;
    read_http_json(stream)
}

fn read_http_json<T: for<'de> Deserialize<'de>>(mut stream: TcpStream) -> AppResult<T> {
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| AppError::Validation("主机响应格式异常".to_string()))?;
    if !head.starts_with("HTTP/1.1 200") {
        let message = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|value| {
                value
                    .get("message")
                    .and_then(|message| message.as_str())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| body.to_string());
        return Err(AppError::Validation(format!("主机返回错误：{message}")));
    }
    serde_json::from_str(body)
        .map_err(|error| AppError::Validation(format!("主机响应解析失败：{error}")))
}

fn url_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            b' ' => vec!['+'],
            other => format!("%{other:02X}").chars().collect(),
        })
        .collect()
}

fn url_decode(value: &str) -> String {
    let mut bytes = Vec::new();
    let mut chars = value.as_bytes().iter().copied().peekable();
    while let Some(byte) = chars.next() {
        if byte == b'%' {
            let hi = chars.next();
            let lo = chars.next();
            if let (Some(hi), Some(lo)) = (hi, lo) {
                if let Ok(hex) = std::str::from_utf8(&[hi, lo]) {
                    if let Ok(decoded) = u8::from_str_radix(hex, 16) {
                        bytes.push(decoded);
                        continue;
                    }
                }
            }
            bytes.push(byte);
        } else if byte == b'+' {
            bytes.push(b' ');
        } else {
            bytes.push(byte);
        }
    }
    String::from_utf8_lossy(&bytes).to_string()
}

fn write_json<T: Serialize>(stream: &mut TcpStream, status: u16, body: &T) -> AppResult<()> {
    let text = serde_json::to_string(body)
        .map_err(|error| AppError::Validation(format!("JSON 序列化失败：{error}")))?;
    let status_text = if status == 200 { "OK" } else { "Not Found" };
    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        text.as_bytes().len(),
        text
    );
    stream.write_all(response.as_bytes())?;
    Ok(())
}

fn generate_pair_code() -> String {
    let value = (Uuid::new_v4().as_u128() % 900_000) + 100_000;
    value.to_string()
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
struct PairRequest {
    pair_code: String,
    client_name: String,
    client_device_id: String,
    app_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PairResponse {
    ok: bool,
    token: Option<String>,
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
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    use rusqlite::Connection;

    use crate::app::paths::AppPaths;
    use crate::db::connection::Db;
    use crate::db::migrations;

    use super::*;

    fn test_db() -> (tempfile::TempDir, Db) {
        let dir = tempfile::tempdir().expect("temp dir");
        let paths = AppPaths {
            data_dir: dir.path().to_path_buf(),
            database_path: dir.path().join("aster.sqlite"),
            backup_dir: dir.path().join("backups"),
            export_dir: dir.path().join("exports"),
            import_report_dir: dir.path().join("import-reports"),
        };
        std::fs::create_dir_all(&paths.backup_dir).unwrap();
        std::fs::create_dir_all(&paths.export_dir).unwrap();
        std::fs::create_dir_all(&paths.import_report_dir).unwrap();
        (dir, Db::initialize(&paths).unwrap())
    }

    fn runtime_with_client(token: &str) -> Arc<Mutex<HostServiceRuntime>> {
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

    fn send_test_request(
        db: Db,
        runtime: Arc<Mutex<HostServiceRuntime>>,
        request: String,
    ) -> String {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test listener");
        let addr = listener.local_addr().expect("test listener addr");
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept test request");
            handle_connection(stream, runtime, db, "0.1.0").unwrap();
        });
        let mut stream = std::net::TcpStream::connect(addr).expect("connect test listener");
        stream.write_all(request.as_bytes()).expect("write request");
        stream.shutdown(std::net::Shutdown::Write).ok();
        let mut response = String::new();
        stream.read_to_string(&mut response).expect("read response");
        handle.join().expect("request thread");
        response
    }

    #[test]
    fn parse_request_line_extracts_method_and_path() {
        let (method, path) = parse_request_line("GET /api/health HTTP/1.1\r\n\r\n");
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
    fn pairing_validation_requires_six_digit_code_and_client_identity() {
        validate_pairing_request("123456", "前台电脑", "device-frontdesk").unwrap();
        assert!(
            validate_pairing_request("12345", "前台电脑", "device-frontdesk")
                .unwrap_err()
                .to_string()
                .contains("6 位数字")
        );
        assert!(
            validate_pairing_request("abcdef", "前台电脑", "device-frontdesk")
                .unwrap_err()
                .to_string()
                .contains("6 位数字")
        );
        assert!(validate_pairing_request("123456", " ", "device-frontdesk")
            .unwrap_err()
            .to_string()
            .contains("客户端名称不能为空"));
        assert!(validate_pairing_request("123456", "前台电脑", " ")
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
        let clients = list_client_connections_from_conn(&conn).unwrap();

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
        let state = AppState {
            paths: AppPaths {
                data_dir: _dir.path().to_path_buf(),
                database_path: _dir.path().join("aster.sqlite"),
                backup_dir: _dir.path().join("backups"),
                export_dir: _dir.path().join("exports"),
                import_report_dir: _dir.path().join("import-reports"),
            },
            db,
            session: Arc::new(Mutex::new(Some(CurrentUser {
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
                permissions: vec!["manage_settings".to_string()],
            }))),
            host_service: Arc::new(Mutex::new(HostServiceRuntime::default())),
        };
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
            "GET /api/status HTTP/1.1\r\nX-Aster-Client-Token: token-status-test\r\n\r\n"
                .to_string(),
        );

        assert!(response.contains("远程请求缺少当前用户"));
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

        let response = send_test_request(
            db,
            runtime_with_client("token-settings-test"),
            "GET /api/system-settings HTTP/1.1\r\nX-Aster-Client-Token: token-settings-test\r\nX-Aster-User-Id: user-settings-readonly-test\r\n\r\n"
                .to_string(),
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

        assert!(response.contains("远程请求缺少当前用户"));
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
        let invalid_request = format!(
            "POST /api/approval HTTP/1.1\r\nX-Aster-Client-Token: token-approval-invalid-test\r\nX-Aster-User-Id: user-approval-warehouse-test\r\nContent-Length: {}\r\n\r\n{}",
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
        let create_request = format!(
            "POST /api/approval HTTP/1.1\r\nX-Aster-Client-Token: token-approval-create-test\r\nX-Aster-User-Id: user-approval-warehouse-test\r\nContent-Length: {}\r\n\r\n{}",
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
        let decide_request = format!(
            "POST /api/approval/decision HTTP/1.1\r\nX-Aster-Client-Token: token-approval-decide-test\r\nX-Aster-User-Id: user-approval-admin-test\r\nContent-Length: {}\r\n\r\n{}",
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
        let request_headers = "X-Aster-Client-Token: token-remote-route-validation-test\r\nX-Aster-User-Id: user-remote-stock-validation-test";

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
        let headers = "X-Aster-Client-Token: token-remote-master-validation-test\r\nX-Aster-User-Id: user-remote-master-validation-test";

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
        assert!(item_response.contains("默认单价不能小于 0"));

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

        let response = send_test_request(
            db,
            runtime_with_client("token-remote-budget-validation-test"),
            format!(
                "POST /api/master/budget-rule HTTP/1.1\r\nX-Aster-Client-Token: token-remote-budget-validation-test\r\nX-Aster-User-Id: user-remote-budget-admin-test\r\nContent-Length: {}\r\n\r\n{}",
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
        let state = AppState {
            paths: AppPaths {
                data_dir: _dir.path().to_path_buf(),
                database_path: _dir.path().join("aster.sqlite"),
                backup_dir: _dir.path().join("backups"),
                export_dir: _dir.path().join("exports"),
                import_report_dir: _dir.path().join("import-reports"),
            },
            db,
            session: Arc::new(Mutex::new(Some(CurrentUser {
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
                permissions: vec!["manage_settings".to_string()],
            }))),
            host_service: Arc::new(Mutex::new(HostServiceRuntime::default())),
        };
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
            runtime.pair_code = Some("123456".to_string());
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

        let request = "GET /api/test HTTP/1.1\r\nX-Aster-User-Id: user-readonly-test\r\n\r\n";
        let error = require_remote_permission(request, &conn, "write_stock").unwrap_err();
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

        let request = "GET /api/test HTTP/1.1\r\nX-Aster-User-Id: user-no-report-test\r\n\r\n";
        let error = require_remote_permission(request, &conn, "view_reports").unwrap_err();
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

        let request = "GET /api/test HTTP/1.1\r\nX-Aster-User-Id: user-warehouse-test\r\n\r\n";
        require_remote_permission(request, &conn, "write_stock").unwrap();
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

        let request =
            "GET /api/test HTTP/1.1\r\nX-Aster-User-Id: user-department-viewer-test\r\n\r\n";
        let current = require_remote_permission(request, &conn, "view_reports").unwrap();
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

        let request = "GET /api/test HTTP/1.1\r\nX-Aster-User-Id: user-unbound-viewer-test\r\n\r\n";
        let current = require_remote_permission(request, &conn, "view_reports").unwrap();
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
        let headers = "X-Aster-Client-Token: token-remote-dept-scope-test\r\nX-Aster-User-Id: user-remote-dept-scope-test";

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

        let response = send_test_request(
            db,
            runtime_with_client("token-remote-status-scope-test"),
            "GET /api/status HTTP/1.1\r\nX-Aster-Client-Token: token-remote-status-scope-test\r\nX-Aster-User-Id: user-remote-status-scope-test\r\n\r\n"
                .to_string(),
        );

        assert!(response.contains("\"thisMonthOutboundAmount\":16"));
        assert!(response.contains("行政办"));
        assert!(!response.contains("餐饮"));
    }
}
