use super::*;
pub(super) fn enforce_security_rate_limit(
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

pub(super) fn clear_login_rate_limit(
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

pub(super) fn security_limit_key<'a>(
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

pub(super) fn write_host_audit(
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

pub(super) fn remote_current_user(
    request: &str,
    conn: &rusqlite::Connection,
) -> AppResult<CurrentUser> {
    crate::services::remote_session_service::current_user(request, conn)
}

pub(super) fn require_remote_admin(
    request: &str,
    conn: &rusqlite::Connection,
) -> AppResult<CurrentUser> {
    let current = remote_current_user(request, conn)?;
    if current.roles.iter().any(|role| role.code == "admin") {
        Ok(current)
    } else {
        Err(AppError::Forbidden("需要管理员权限".to_string()))
    }
}

pub(super) fn require_remote_permission(
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

pub(super) fn remote_department_scope(current: &CurrentUser) -> AppResult<Option<String>> {
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
