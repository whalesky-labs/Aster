use crate::app::state::AppState;
use crate::domain::approvals::{ApprovalRequest, CreateApprovalRequest, DecideApprovalRequest};
use crate::domain::backups::BackupRecord;
use crate::domain::reports::{ReportBundle, ReportQuery};
use crate::domain::status::{AppStatus, AuditLogRow, SystemSettings};
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

use super::{client_runtime_config, collect_remote_pages, http_get_json, http_post_json};
use crate::infrastructure::http_transport::{page_path, push_query_param, url_encode};
pub fn remote_get_report_bundle(state: &AppState, query: ReportQuery) -> AppResult<ReportBundle> {
    let config = client_runtime_config(state)?;
    let mut params = vec![format!("month={}", url_encode(&query.month))];
    push_query_param(&mut params, "startDate", query.start_date.clone());
    push_query_param(&mut params, "endDate", query.end_date.clone());
    push_query_param(&mut params, "departmentId", query.department_id.clone());
    push_query_param(&mut params, "categoryId", query.category_id.clone());
    push_query_param(&mut params, "itemId", query.item_id.clone());
    push_query_param(&mut params, "supplierId", query.supplier_id.clone());
    let base_path = format!("/api/reports/monthly?{}", params.join("&"));
    crate::application::report_pagination::collect(&query.month, |section, cursor| {
        let section_path = format!("{base_path}&section={}", url_encode(section));
        http_get_json(&config, &page_path(&section_path, cursor))
    })
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
    collect_remote_pages(&config, "/api/stocktakes")
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
    collect_remote_pages(&config, "/api/approvals")
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
    collect_remote_pages(&config, &path)
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
    collect_remote_pages(&config, "/api/backups")
}

pub fn remote_list_users(state: &AppState) -> AppResult<Vec<UserAccount>> {
    let config = client_runtime_config(state)?;
    collect_remote_pages(&config, "/api/users")
}

pub fn remote_list_roles(state: &AppState) -> AppResult<Vec<Role>> {
    let config = client_runtime_config(state)?;
    collect_remote_pages(&config, "/api/roles")
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
    let response: crate::services::remote_session_service::LoginResponse =
        http_post_json(&config, "/api/login", &request)?;
    state
        .host_service
        .lock()
        .map_err(|_| AppError::Validation("客户端会话状态异常".to_string()))?
        .client_session_token = Some(response.session_token);
    Ok(response.user)
}

pub fn remote_logout(state: &AppState) -> AppResult<()> {
    let config = client_runtime_config(state)?;
    if config.session_token.is_none() {
        return Ok(());
    }
    http_post_json::<_, ()>(&config, "/api/logout", &())?;
    state
        .host_service
        .lock()
        .map_err(|_| AppError::Validation("客户端会话状态异常".to_string()))?
        .client_session_token = None;
    Ok(())
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
