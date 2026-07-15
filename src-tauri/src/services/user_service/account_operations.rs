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
    if !crate::domain::passwords::verify(&request.password, &hash) {
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
        if !crate::domain::passwords::verify(old_password, &old_hash) {
            return Err(AppError::Validation("旧密码错误".to_string()));
        }
    }
    let next_hash = crate::domain::passwords::hash(&request.new_password)?;
    user_repository::update_password_hash(conn, &target_user_id, &next_hash)?;
    user_repository::set_must_change_password(conn, &target_user_id, false)?;
    crate::db::session_repository::revoke_user(
        conn,
        &target_user_id,
        chrono::Utc::now().timestamp(),
    )?;
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
        Some(password) if !password.trim().is_empty() => {
            Some(crate::domain::passwords::hash(password)?)
        }
        _ => None,
    };
    let user = user_repository::save_user(
        conn,
        user_repository::SaveUserRecord {
            id: request.id,
            username: request.username.trim(),
            display_name: request.display_name.trim(),
            email: request.email.as_deref().map(str::trim).map(str::to_string),
            password_hash,
            department_id: request.department_id,
            enabled: request.enabled,
            role_codes: &request.role_codes,
        },
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

pub fn request_password_reset_code_on_db(
    db: &crate::db::connection::Db,
    request: RequestPasswordResetCodeRequest,
) -> AppResult<RequestPasswordResetCodeResponse> {
    let password = crate::application::secret_service::load(
        db,
        crate::application::secret_service::ApplicationSecret::SmtpPassword,
    )?
    .unwrap_or_default();
    let prepared = db.with_conn(|conn| prepare_password_reset(conn, request, password))?;
    let code_id = db.with_conn_mut(|conn| {
        let transaction = conn.transaction()?;
        let code_id = user_repository::create_password_reset_code(
            &transaction,
            &prepared.user_id,
            &prepared.code_hash,
            &prepared.expires_at,
        )?;
        transaction.commit()?;
        Ok(code_id)
    })?;
    if let Err(error) = send_password_reset_email(
        &prepared.smtp,
        &prepared.email,
        &prepared.display_name,
        &prepared.code,
    ) {
        let _ = db.with_conn(|conn| user_repository::mark_password_reset_code_used(conn, &code_id));
        return Err(error);
    }
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
             VALUES (?1, 'request_password_reset', 'user', ?2, ?3, 'system')",
            params![
                uuid::Uuid::new_v4().to_string(),
                prepared.user_id,
                format!("发送密码找回验证码：{}", prepared.username)
            ],
        )?;
        Ok(())
    })?;
    Ok(RequestPasswordResetCodeResponse {
        masked_email: mask_email(&prepared.email),
        expires_minutes: PASSWORD_RESET_EXPIRES_MINUTES,
    })
}

struct PreparedPasswordReset {
    user_id: String,
    username: String,
    display_name: String,
    email: String,
    code: String,
    code_hash: String,
    expires_at: String,
    smtp: SmtpSettings,
}

fn prepare_password_reset(
    conn: &rusqlite::Connection,
    request: RequestPasswordResetCodeRequest,
    smtp_password: String,
) -> AppResult<PreparedPasswordReset> {
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
    let smtp = smtp_settings_from_conn(conn, smtp_password)?;
    if !smtp.enabled {
        return Err(AppError::Validation(
            "系统未开启邮箱验证码，请联系管理员配置 SMTP".to_string(),
        ));
    }

    let code = format!("{:06}", rand::thread_rng().gen_range(0..1_000_000));
    let code_hash = crate::domain::passwords::hash(&code)?;
    let expires_at = (chrono::Utc::now()
        + chrono::Duration::minutes(PASSWORD_RESET_EXPIRES_MINUTES))
    .naive_utc()
    .format("%Y-%m-%d %H:%M:%S")
    .to_string();
    Ok(PreparedPasswordReset {
        user_id: user.id,
        username: user.username,
        display_name: user.display_name,
        email: email.to_string(),
        code,
        code_hash,
        expires_at,
        smtp,
    })
}

pub fn reset_password_with_code_on_db(
    db: &crate::db::connection::Db,
    request: ResetPasswordWithCodeRequest,
) -> AppResult<()> {
    db.with_conn_mut(|conn| {
        let transaction = conn.transaction()?;
        reset_password_with_code_on_conn(&transaction, request)?;
        transaction.commit()?;
        Ok(())
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
    if !crate::domain::passwords::verify(code, &code_hash) {
        return Err(AppError::Validation("验证码错误".to_string()));
    }
    let next_hash = crate::domain::passwords::hash(&request.new_password)?;
    user_repository::update_password_hash(conn, &user_id, &next_hash)?;
    user_repository::mark_password_reset_code_used(conn, &code_id)?;
    crate::db::session_repository::revoke_user(conn, &user_id, chrono::Utc::now().timestamp())?;
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
