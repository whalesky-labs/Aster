fn validate_settings(request: &SaveSystemSettingsRequest) -> AppResult<()> {
    if request.hotel_name.trim().is_empty() {
        return Err(AppError::Validation("酒店名称不能为空".to_string()));
    }
    validate_month("当前账期", &request.current_period)?;
    validate_month("默认月份", &request.default_month)?;
    if !(0..=6).contains(&request.quantity_decimals) {
        return Err(AppError::Validation(
            "数量小数位必须在 0 到 6 之间".to_string(),
        ));
    }
    if !(0..=6).contains(&request.amount_decimals) {
        return Err(AppError::Validation(
            "金额小数位必须在 0 到 6 之间".to_string(),
        ));
    }
    if request.default_export_dir.trim().is_empty() {
        return Err(AppError::Validation("默认导出目录不能为空".to_string()));
    }
    if request.default_backup_dir.trim().is_empty() {
        return Err(AppError::Validation("默认备份目录不能为空".to_string()));
    }
    if !(1..=168).contains(&request.interval_backup_hours) {
        return Err(AppError::Validation(
            "定时备份间隔必须在 1 到 168 小时之间".to_string(),
        ));
    }
    if !(1..=65535).contains(&request.smtp_port) {
        return Err(AppError::Validation(
            "SMTP 端口必须在 1 到 65535 之间".to_string(),
        ));
    }
    if request.smtp_enabled {
        if request.smtp_host.trim().is_empty() {
            return Err(AppError::Validation("SMTP 主机不能为空".to_string()));
        }
        if request.smtp_username.trim().is_empty() {
            return Err(AppError::Validation("SMTP 账号不能为空".to_string()));
        }
        validate_email_field("发件邮箱", &request.smtp_from_email)?;
    }
    Ok(())
}

fn validate_local_directory_settings(request: &SaveSystemSettingsRequest) -> AppResult<()> {
    if request.default_export_dir.trim().is_empty() {
        return Err(AppError::Validation("默认导出目录不能为空".to_string()));
    }
    if request.default_backup_dir.trim().is_empty() {
        return Err(AppError::Validation("默认备份目录不能为空".to_string()));
    }
    Ok(())
}

fn validate_email_field(label: &str, value: &str) -> AppResult<()> {
    let trimmed = value.trim();
    if trimmed.contains('@') && trimmed.split('@').all(|part| !part.is_empty()) {
        Ok(())
    } else {
        Err(AppError::Validation(format!("{label}格式不正确")))
    }
}

fn validate_month(label: &str, value: &str) -> AppResult<()> {
    let value = value.trim();
    if value.len() != 7 || value.as_bytes().get(4) != Some(&b'-') {
        return Err(AppError::Validation(format!("{label}格式必须是 YYYY-MM")));
    }
    let year = value[0..4].parse::<i32>().ok();
    let month = value[5..7].parse::<u32>().ok();
    if year.is_none() || !matches!(month, Some(1..=12)) {
        return Err(AppError::Validation(format!("{label}格式必须是 YYYY-MM")));
    }
    Ok(())
}

fn ensure_writable_dir(path: &Path, label: &str) -> AppResult<()> {
    if !path.is_dir() {
        return Err(AppError::Validation(format!("{label}不是有效文件夹")));
    }
    let test_file = path.join(".aster-write-test");
    fs::write(&test_file, "ok")?;
    fs::remove_file(test_file)?;
    Ok(())
}

fn setting_or(conn: &rusqlite::Connection, key: &str, fallback: &str) -> AppResult<String> {
    Ok(repository::get_setting(conn, key)?.unwrap_or_else(|| fallback.to_string()))
}

fn setting_bool(conn: &rusqlite::Connection, key: &str, fallback: bool) -> AppResult<bool> {
    Ok(repository::get_setting(conn, key)?
        .map(|value| value == "true")
        .unwrap_or(fallback))
}

fn setting_i64(
    conn: &rusqlite::Connection,
    key: &str,
    fallback: i64,
    min: i64,
    max: i64,
) -> AppResult<i64> {
    Ok(repository::get_setting(conn, key)?
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| (*value >= min) && (*value <= max))
        .unwrap_or(fallback))
}

fn bool_setting(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn effective_export_dir_from_conn(conn: &rusqlite::Connection, state: &AppState) -> String {
    repository::get_setting(conn, "default_export_dir")
        .ok()
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| state.paths.export_dir.display().to_string())
}

fn effective_backup_dir_from_conn(conn: &rusqlite::Connection, state: &AppState) -> String {
    repository::get_setting(conn, "default_backup_dir")
        .ok()
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| state.paths.backup_dir.display().to_string())
}

fn with_local_directory_settings(
    conn: &rusqlite::Connection,
    state: &AppState,
    mut settings: SystemSettings,
) -> AppResult<SystemSettings> {
    settings.default_export_dir = effective_export_dir_from_conn(conn, state);
    settings.default_backup_dir = effective_backup_dir_from_conn(conn, state);
    Ok(settings)
}
