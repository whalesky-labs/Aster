use rand::Rng;
use rusqlite::{params, OptionalExtension};

use crate::app::state::AppState;
use crate::db::repository;
use crate::db::user_repository;
use crate::domain::users::{
    ChangePasswordRequest, CurrentUser, LoginRequest, RequestPasswordResetCodeRequest,
    RequestPasswordResetCodeResponse, ResetPasswordWithCodeRequest, Role, SaveUserRequest,
    SetUserEnabledRequest, UserAccount,
};
use crate::error::{AppError, AppResult};

const PASSWORD_RESET_EXPIRES_MINUTES: i64 = 10;

pub fn ensure_default_admin(state: &AppState) -> AppResult<()> {
    crate::application::authentication_service::ensure_default_admin(state)
}

pub fn login(state: &AppState, request: LoginRequest) -> AppResult<CurrentUser> {
    let username = request.username.trim().to_string();
    if username.is_empty() || request.password.is_empty() {
        return Err(AppError::Validation("用户名和密码不能为空".to_string()));
    }
    let user = match runtime_mode(state)? {
        crate::domain::runtime::RuntimeMode::Client if client_has_pairing_token(state)? => {
            crate::services::host_service::remote_login(state, request)?
        }
        crate::domain::runtime::RuntimeMode::Client => {
            let user = crate::application::authentication_service::login_locally(state, request)?;
            if user.roles.iter().any(|role| role.code == "admin") {
                user
            } else {
                return Err(AppError::Validation(
                    "客户端未配对前只允许本地管理员登录完成主机配置".to_string(),
                ));
            }
        }
        _ => crate::application::authentication_service::login_locally(state, request)?,
    };
    *state.session.lock().expect("session mutex poisoned") = Some(user.clone());
    Ok(user)
}

pub fn logout(state: &AppState) -> AppResult<()> {
    if runtime_mode(state)? == crate::domain::runtime::RuntimeMode::Client
        && client_has_pairing_token(state)?
    {
        crate::services::host_service::remote_logout(state)?;
    }
    *state.session.lock().expect("session mutex poisoned") = None;
    Ok(())
}

pub fn current_user(state: &AppState) -> AppResult<Option<CurrentUser>> {
    Ok(state
        .session
        .lock()
        .expect("session mutex poisoned")
        .clone())
}

pub fn password_change_required(state: &AppState) -> AppResult<bool> {
    crate::application::authentication_service::password_change_required(state)
}

pub fn list_users(state: &AppState) -> AppResult<Vec<UserAccount>> {
    require_admin(state)?;
    if runtime_mode(state)? == crate::domain::runtime::RuntimeMode::Client {
        return crate::services::host_service::remote_list_users(state);
    }
    state.db.with_conn(user_repository::list_users)
}

pub fn list_roles(state: &AppState) -> AppResult<Vec<Role>> {
    require_admin(state)?;
    if runtime_mode(state)? == crate::domain::runtime::RuntimeMode::Client {
        return crate::services::host_service::remote_list_roles(state);
    }
    state.db.with_conn(user_repository::list_roles)
}

pub fn save_user(state: &AppState, request: SaveUserRequest) -> AppResult<UserAccount> {
    require_admin(state)?;
    validate_save_user(&request)?;
    if runtime_mode(state)? == crate::domain::runtime::RuntimeMode::Client {
        return crate::services::host_service::remote_save_user(state, request);
    }
    state
        .db
        .with_conn(|conn| save_user_on_conn(conn, request, &current_operator(state)))
}

pub fn set_user_enabled(state: &AppState, request: SetUserEnabledRequest) -> AppResult<()> {
    require_admin(state)?;
    if request.user_id.trim().is_empty() {
        return Err(AppError::Validation("用户不能为空".to_string()));
    }
    if runtime_mode(state)? == crate::domain::runtime::RuntimeMode::Client {
        return crate::services::host_service::remote_set_user_enabled(state, request);
    }
    state
        .db
        .with_conn(|conn| set_user_enabled_on_conn(conn, request, &current_operator(state)))
}

pub fn change_password(state: &AppState, request: ChangePasswordRequest) -> AppResult<()> {
    let current = state
        .session
        .lock()
        .expect("session mutex poisoned")
        .clone()
        .ok_or_else(|| AppError::Validation("请先登录".to_string()))?;
    if request.new_password.len() < 6 {
        return Err(AppError::Validation("新密码至少 6 位".to_string()));
    }
    let target_user_id = request
        .user_id
        .clone()
        .unwrap_or_else(|| current.id.clone());
    let changing_self = target_user_id == current.id;
    if !changing_self {
        require_admin(state)?;
    }
    if runtime_mode(state)? == crate::domain::runtime::RuntimeMode::Client {
        let mut remote_request = request;
        if remote_request.user_id.is_none() {
            remote_request.user_id = Some(current.id);
        }
        return crate::services::host_service::remote_change_password(state, remote_request);
    }
    state.db.with_conn(|conn| {
        change_password_on_conn(conn, request, &current_operator(state), &current.id)
    })
}

pub fn request_password_reset_code(
    state: &AppState,
    request: RequestPasswordResetCodeRequest,
) -> AppResult<RequestPasswordResetCodeResponse> {
    if runtime_mode(state)? == crate::domain::runtime::RuntimeMode::Client
        && client_has_pairing_token(state)?
    {
        return crate::services::host_service::remote_request_password_reset_code(state, request);
    }
    request_password_reset_code_on_db(&state.db, request)
}

pub fn reset_password_with_code(
    state: &AppState,
    request: ResetPasswordWithCodeRequest,
) -> AppResult<()> {
    if runtime_mode(state)? == crate::domain::runtime::RuntimeMode::Client
        && client_has_pairing_token(state)?
    {
        return crate::services::host_service::remote_reset_password_with_code(state, request);
    }
    reset_password_with_code_on_db(&state.db, request)
}

pub fn require_admin(state: &AppState) -> AppResult<CurrentUser> {
    let current = state
        .session
        .lock()
        .expect("session mutex poisoned")
        .clone()
        .ok_or_else(|| AppError::Validation("请先登录管理员账号".to_string()))?;
    crate::application::authentication_service::require_password_changed(state, &current)?;
    if current.roles.iter().any(|role| role.code == "admin") {
        Ok(current)
    } else {
        Err(AppError::Validation("需要管理员权限".to_string()))
    }
}

pub fn require_permission(state: &AppState, permission: &str) -> AppResult<CurrentUser> {
    let current = state
        .session
        .lock()
        .expect("session mutex poisoned")
        .clone()
        .ok_or_else(|| AppError::Validation("请先登录".to_string()))?;
    crate::application::authentication_service::require_password_changed(state, &current)?;
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

pub fn current_department_scope(state: &AppState) -> AppResult<Option<String>> {
    let Some(current) = state
        .session
        .lock()
        .expect("session mutex poisoned")
        .clone()
    else {
        return Err(AppError::Validation("请先登录".to_string()));
    };
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
            .map(Some)
            .ok_or_else(|| AppError::Validation("部门查看员未绑定所属部门".to_string()))
    } else {
        Ok(None)
    }
}

pub fn current_operator(state: &AppState) -> String {
    state
        .session
        .lock()
        .expect("session mutex poisoned")
        .as_ref()
        .map(|user| user.username.clone())
        .unwrap_or_else(|| "system".to_string())
}

include!("user_service/account_operations.rs");

fn validate_user_department_binding(
    conn: &rusqlite::Connection,
    request: &SaveUserRequest,
) -> AppResult<()> {
    let needs_department = request
        .role_codes
        .iter()
        .any(|role| role.trim() == "department_viewer");
    if !needs_department {
        return Ok(());
    }
    let department_id = request
        .department_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Validation("部门查看员必须绑定所属部门".to_string()))?;
    let department = conn
        .query_row(
            "SELECT name, enabled FROM departments WHERE id = ?1",
            params![department_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? == 1)),
        )
        .optional()?
        .ok_or_else(|| AppError::Validation("绑定部门不存在".to_string()))?;
    if !department.1 {
        return Err(AppError::Validation(format!(
            "绑定部门已停用：{}",
            department.0
        )));
    }
    Ok(())
}

fn ensure_admin_would_remain_after_save(
    conn: &rusqlite::Connection,
    request: &SaveUserRequest,
) -> AppResult<()> {
    let Some(user_id) = request.id.as_deref() else {
        return Ok(());
    };
    let currently_admin = user_has_role(conn, user_id, "admin")?;
    if !currently_admin {
        return Ok(());
    }
    let will_be_enabled_admin =
        request.enabled && request.role_codes.iter().any(|role| role.trim() == "admin");
    if will_be_enabled_admin || enabled_admin_count_excluding(conn, Some(user_id))? > 0 {
        Ok(())
    } else {
        Err(AppError::Validation(
            "不能停用或移除最后一个启用管理员".to_string(),
        ))
    }
}

fn ensure_admin_would_remain_after_enabled_change(
    conn: &rusqlite::Connection,
    user_id: &str,
    enabled: bool,
) -> AppResult<()> {
    if enabled || !user_has_role(conn, user_id, "admin")? {
        return Ok(());
    }
    if enabled_admin_count_excluding(conn, Some(user_id))? > 0 {
        Ok(())
    } else {
        Err(AppError::Validation(
            "不能停用最后一个启用管理员".to_string(),
        ))
    }
}

fn user_has_role(conn: &rusqlite::Connection, user_id: &str, role_code: &str) -> AppResult<bool> {
    conn.query_row(
        "SELECT EXISTS(
           SELECT 1
           FROM user_roles ur
           JOIN roles r ON r.id = ur.role_id
           WHERE ur.user_id = ?1 AND r.code = ?2
         )",
        params![user_id, role_code],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

fn enabled_admin_count_excluding(
    conn: &rusqlite::Connection,
    excluded_user_id: Option<&str>,
) -> AppResult<i64> {
    conn.query_row(
        "SELECT COUNT(*)
         FROM users u
         JOIN user_roles ur ON ur.user_id = u.id
         JOIN roles r ON r.id = ur.role_id
         WHERE u.enabled = 1
           AND r.code = 'admin'
           AND (?1 IS NULL OR u.id <> ?1)",
        params![excluded_user_id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

fn runtime_mode(state: &AppState) -> AppResult<crate::domain::runtime::RuntimeMode> {
    Ok(crate::services::status_service::get_runtime_config(state)?.mode)
}

fn client_has_pairing_token(state: &AppState) -> AppResult<bool> {
    Ok(crate::services::status_service::get_runtime_config(state)?
        .client_token
        .as_deref()
        .is_some_and(|token| !token.trim().is_empty()))
}

include!("user_service/password_reset_mail.rs");

pub(crate) fn to_current_user(user: UserAccount) -> CurrentUser {
    let permissions = permissions_for_roles(&user.roles);
    CurrentUser {
        id: user.id,
        username: user.username,
        display_name: user.display_name,
        department_id: user.department_id,
        department_name: user.department_name,
        roles: user.roles,
        permissions,
    }
}

fn permissions_for_roles(roles: &[Role]) -> Vec<String> {
    let mut permissions = Vec::new();
    for role in roles {
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
    permissions.into_iter().map(str::to_string).collect()
}

#[cfg(test)]
#[path = "user_service/tests.rs"]
mod tests;
