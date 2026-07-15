struct SmtpSettings {
    enabled: bool,
    host: String,
    port: u16,
    username: String,
    password: String,
    from_email: String,
    from_name: String,
}

fn smtp_settings_from_conn(
    conn: &rusqlite::Connection,
    password: String,
) -> AppResult<SmtpSettings> {
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
        password,
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
