use pbkdf2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use pbkdf2::Pbkdf2;
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

const DEFAULT_ADMIN_PASSWORD: &str = "admin123";
const PASSWORD_RESET_EXPIRES_MINUTES: i64 = 10;

pub fn ensure_default_admin(state: &AppState) -> AppResult<()> {
    let hash = hash_password(DEFAULT_ADMIN_PASSWORD)?;
    state
        .db
        .with_conn(|conn| user_repository::ensure_default_admin(conn, &hash))
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
            let user = state.db.with_conn(|conn| login_on_conn(conn, request))?;
            if user.roles.iter().any(|role| role.code == "admin") {
                user
            } else {
                return Err(AppError::Validation(
                    "客户端未配对前只允许本地管理员登录完成主机配置".to_string(),
                ));
            }
        }
        _ => state.db.with_conn(|conn| login_on_conn(conn, request))?,
    };
    *state.session.lock().expect("session mutex poisoned") = Some(user.clone());
    Ok(user)
}

pub fn logout(state: &AppState) -> AppResult<()> {
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
    state
        .db
        .with_conn(|conn| request_password_reset_code_on_conn(conn, request))
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
    state
        .db
        .with_conn(|conn| reset_password_with_code_on_conn(conn, request))
}

pub fn require_admin(state: &AppState) -> AppResult<CurrentUser> {
    let current = state
        .session
        .lock()
        .expect("session mutex poisoned")
        .clone()
        .ok_or_else(|| AppError::Validation("请先登录管理员账号".to_string()))?;
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

fn validate_save_user(request: &SaveUserRequest) -> AppResult<()> {
    if request.username.trim().is_empty() {
        return Err(AppError::Validation("用户名不能为空".to_string()));
    }
    if request.display_name.trim().is_empty() {
        return Err(AppError::Validation("显示名称不能为空".to_string()));
    }
    if request.id.is_none() && request.password.as_deref().unwrap_or("").trim().is_empty() {
        return Err(AppError::Validation("新用户必须设置密码".to_string()));
    }
    if request.role_codes.is_empty() {
        return Err(AppError::Validation("至少选择一个角色".to_string()));
    }
    Ok(())
}

pub fn login_on_conn(conn: &rusqlite::Connection, request: LoginRequest) -> AppResult<CurrentUser> {
    let username = request.username.trim().to_string();
    if username.is_empty() || request.password.is_empty() {
        return Err(AppError::Validation("用户名和密码不能为空".to_string()));
    }
    let Some((user, hash)) = user_repository::find_user_by_username(conn, &username)? else {
        return Err(AppError::Validation("用户名或密码错误".to_string()));
    };
    if !user.enabled {
        return Err(AppError::Validation("用户已停用".to_string()));
    }
    let Some(hash) = hash else {
        return Err(AppError::Validation("用户未设置密码".to_string()));
    };
    if !verify_password(&request.password, &hash) {
        return Err(AppError::Validation("用户名或密码错误".to_string()));
    }
    let current = to_current_user(user);
    conn.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, 'login', 'user', ?2, ?3, ?4)",
        params![
            uuid::Uuid::new_v4().to_string(),
            current.id,
            format!("用户登录：{}", current.username),
            current.username
        ],
    )?;
    Ok(current)
}

pub fn change_password_on_conn(
    conn: &rusqlite::Connection,
    request: ChangePasswordRequest,
    operator: &str,
    current_user_id: &str,
) -> AppResult<()> {
    if request.new_password.len() < 6 {
        return Err(AppError::Validation("新密码至少 6 位".to_string()));
    }
    let target_user_id = request
        .user_id
        .clone()
        .unwrap_or_else(|| current_user_id.to_string());
    let changing_self = target_user_id == current_user_id;
    let Some((_user, old_hash)) = user_repository::find_user_by_id(conn, &target_user_id)? else {
        return Err(AppError::Validation("用户不存在".to_string()));
    };
    if changing_self {
        let old_password = request
            .old_password
            .as_deref()
            .ok_or_else(|| AppError::Validation("请输入旧密码".to_string()))?;
        let Some(old_hash) = old_hash else {
            return Err(AppError::Validation("用户未设置密码".to_string()));
        };
        if !verify_password(old_password, &old_hash) {
            return Err(AppError::Validation("旧密码错误".to_string()));
        }
    }
    let next_hash = hash_password(&request.new_password)?;
    user_repository::update_password_hash(conn, &target_user_id, &next_hash)?;
    conn.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, 'change_password', 'user', ?2, '修改密码', ?3)",
        params![uuid::Uuid::new_v4().to_string(), target_user_id, operator],
    )?;
    Ok(())
}

pub fn save_user_on_conn(
    conn: &rusqlite::Connection,
    request: SaveUserRequest,
    operator: &str,
) -> AppResult<UserAccount> {
    validate_save_user(&request)?;
    validate_user_department_binding(conn, &request)?;
    ensure_admin_would_remain_after_save(conn, &request)?;
    let password_hash = match request.password.as_deref() {
        Some(password) if !password.trim().is_empty() => Some(hash_password(password)?),
        _ => None,
    };
    let user = user_repository::save_user(
        conn,
        request.id,
        request.username.trim(),
        request.display_name.trim(),
        request.email.as_deref().map(str::trim).map(str::to_string),
        password_hash,
        request.department_id,
        request.enabled,
        &request.role_codes,
    )?;
    conn.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, 'save_user', 'user', ?2, ?3, ?4)",
        params![
            uuid::Uuid::new_v4().to_string(),
            user.id,
            user.username,
            operator
        ],
    )?;
    Ok(user)
}

pub fn request_password_reset_code_on_conn(
    conn: &rusqlite::Connection,
    request: RequestPasswordResetCodeRequest,
) -> AppResult<RequestPasswordResetCodeResponse> {
    let username = request.username.trim();
    if username.is_empty() {
        return Err(AppError::Validation("请输入用户名".to_string()));
    }
    let Some((user, _hash)) = user_repository::find_user_by_username(conn, username)? else {
        return Err(AppError::Validation("用户不存在或未绑定邮箱".to_string()));
    };
    if !user.enabled {
        return Err(AppError::Validation("用户已停用".to_string()));
    }
    let email = user
        .email
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Validation("用户未绑定邮箱，请联系管理员重置密码".to_string()))?;
    validate_email(email)?;
    let smtp = smtp_settings_from_conn(conn)?;
    if !smtp.enabled {
        return Err(AppError::Validation(
            "系统未开启邮箱验证码，请联系管理员配置 SMTP".to_string(),
        ));
    }

    let code = format!("{:06}", rand::thread_rng().gen_range(0..1_000_000));
    let code_hash = hash_password(&code)?;
    let expires_at = (chrono::Utc::now()
        + chrono::Duration::minutes(PASSWORD_RESET_EXPIRES_MINUTES))
    .naive_utc()
    .format("%Y-%m-%d %H:%M:%S")
    .to_string();
    user_repository::create_password_reset_code(conn, &user.id, &code_hash, &expires_at)?;
    send_password_reset_email(&smtp, email, &user.display_name, &code)?;
    conn.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, 'request_password_reset', 'user', ?2, ?3, 'system')",
        params![
            uuid::Uuid::new_v4().to_string(),
            user.id,
            format!("发送密码找回验证码：{}", user.username)
        ],
    )?;
    Ok(RequestPasswordResetCodeResponse {
        masked_email: mask_email(email),
        expires_minutes: PASSWORD_RESET_EXPIRES_MINUTES,
    })
}

pub fn reset_password_with_code_on_conn(
    conn: &rusqlite::Connection,
    request: ResetPasswordWithCodeRequest,
) -> AppResult<()> {
    let username = request.username.trim();
    let code = request.code.trim();
    if username.is_empty() {
        return Err(AppError::Validation("请输入用户名".to_string()));
    }
    if code.len() != 6 || !code.chars().all(|item| item.is_ascii_digit()) {
        return Err(AppError::Validation("验证码必须是 6 位数字".to_string()));
    }
    if request.new_password.len() < 6 {
        return Err(AppError::Validation("新密码至少 6 位".to_string()));
    }
    let Some((code_id, user_id, stored_username, code_hash)) =
        user_repository::find_active_password_reset_code(conn, username)?
    else {
        return Err(AppError::Validation("验证码无效或已过期".to_string()));
    };
    if !verify_password(code, &code_hash) {
        return Err(AppError::Validation("验证码错误".to_string()));
    }
    let next_hash = hash_password(&request.new_password)?;
    user_repository::update_password_hash(conn, &user_id, &next_hash)?;
    user_repository::mark_password_reset_code_used(conn, &code_id)?;
    conn.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, 'reset_password_with_code', 'user', ?2, ?3, 'system')",
        params![
            uuid::Uuid::new_v4().to_string(),
            user_id,
            format!("通过邮箱验证码重置密码：{}", stored_username)
        ],
    )?;
    Ok(())
}

pub fn set_user_enabled_on_conn(
    conn: &rusqlite::Connection,
    request: SetUserEnabledRequest,
    operator: &str,
) -> AppResult<()> {
    if request.user_id.trim().is_empty() {
        return Err(AppError::Validation("用户不能为空".to_string()));
    }
    ensure_admin_would_remain_after_enabled_change(conn, &request.user_id, request.enabled)?;
    user_repository::set_user_enabled(conn, &request.user_id, request.enabled)?;
    conn.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, 'set_user_enabled', 'user', ?2, ?3, ?4)",
        params![
            uuid::Uuid::new_v4().to_string(),
            request.user_id,
            request.enabled.to_string(),
            operator
        ],
    )?;
    Ok(())
}

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

struct SmtpSettings {
    enabled: bool,
    host: String,
    port: u16,
    username: String,
    password: String,
    from_email: String,
    from_name: String,
}

fn smtp_settings_from_conn(conn: &rusqlite::Connection) -> AppResult<SmtpSettings> {
    let enabled = repository::get_setting(conn, "smtp_enabled")?
        .as_deref()
        .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
    let port = repository::get_setting(conn, "smtp_port")?
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(465);
    Ok(SmtpSettings {
        enabled,
        host: repository::get_setting(conn, "smtp_host")?.unwrap_or_default(),
        port,
        username: repository::get_setting(conn, "smtp_username")?.unwrap_or_default(),
        password: repository::get_setting(conn, "smtp_password")?.unwrap_or_default(),
        from_email: repository::get_setting(conn, "smtp_from_email")?.unwrap_or_default(),
        from_name: repository::get_setting(conn, "smtp_from_name")?
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "Aster".to_string()),
    })
}

fn send_password_reset_email(
    settings: &SmtpSettings,
    recipient: &str,
    display_name: &str,
    code: &str,
) -> AppResult<()> {
    if settings.host.trim().is_empty()
        || settings.username.trim().is_empty()
        || settings.password.is_empty()
        || settings.from_email.trim().is_empty()
    {
        return Err(AppError::Validation(
            "SMTP 配置不完整，请在系统设置中配置发件邮箱".to_string(),
        ));
    }
    let from = format!(
        "{} <{}>",
        settings.from_name.trim(),
        settings.from_email.trim()
    )
    .parse()
    .map_err(|error| AppError::Validation(format!("发件邮箱格式错误：{error}")))?;
    let to = recipient
        .parse()
        .map_err(|error| AppError::Validation(format!("收件邮箱格式错误：{error}")))?;
    let email = lettre::Message::builder()
        .from(from)
        .to(to)
        .subject("Aster 密码找回验证码")
        .body(format!(
            "{display_name}，您好：\n\n您的 Aster 密码找回验证码是：{code}\n验证码将在 {PASSWORD_RESET_EXPIRES_MINUTES} 分钟后失效。如非本人操作，请忽略本邮件。\n\nAster"
        ))
        .map_err(|error| AppError::Validation(format!("邮件内容生成失败：{error}")))?;
    let credentials = lettre::transport::smtp::authentication::Credentials::new(
        settings.username.clone(),
        settings.password.clone(),
    );
    let builder = if settings.port == 465 {
        lettre::SmtpTransport::relay(settings.host.trim())
    } else {
        lettre::SmtpTransport::starttls_relay(settings.host.trim())
    }
    .map_err(|error| AppError::Validation(format!("SMTP 主机配置错误：{error}")))?;
    let mailer = builder.port(settings.port).credentials(credentials).build();
    lettre::Transport::send(&mailer, &email)
        .map_err(|error| AppError::Validation(format!("验证码邮件发送失败：{error}")))?;
    Ok(())
}

fn validate_email(email: &str) -> AppResult<()> {
    let trimmed = email.trim();
    if trimmed.contains('@') && trimmed.split('@').all(|part| !part.is_empty()) {
        Ok(())
    } else {
        Err(AppError::Validation("邮箱格式不正确".to_string()))
    }
}

fn mask_email(email: &str) -> String {
    let Some((name, domain)) = email.split_once('@') else {
        return "***".to_string();
    };
    let prefix: String = name.chars().take(2).collect();
    format!("{prefix}***@{domain}")
}

fn to_current_user(user: UserAccount) -> CurrentUser {
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

fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Pbkdf2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| AppError::Validation(format!("密码哈希失败：{error}")))
}

fn verify_password(password: &str, encoded_hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(encoded_hash) else {
        return false;
    };
    Pbkdf2.verify_password(password.as_bytes(), &parsed).is_ok()
}

#[cfg(test)]
mod tests {
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
        let hash = hash_password("admin123").unwrap();
        assert!(verify_password("admin123", &hash));
        assert!(!verify_password("wrong", &hash));
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
        let hash = hash_password("admin123").unwrap();
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
        let hash = hash_password("admin123").unwrap();
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
}
