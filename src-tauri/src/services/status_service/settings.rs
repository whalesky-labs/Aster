pub fn prepare_update_settings_snapshot(state: &AppState) -> AppResult<()> {
    crate::services::user_service::require_admin(state)?;
    write_update_settings_snapshot(state)
}

pub fn restore_update_settings_snapshot_if_needed(state: &AppState) -> AppResult<()> {
    let snapshot_path = update_settings_snapshot_path(state);
    if !snapshot_path.exists() {
        return Ok(());
    }

    let text = fs::read_to_string(&snapshot_path)?;
    let snapshot: UpdateSettingsSnapshot = serde_json::from_str(&text)
        .map_err(|error| AppError::Validation(format!("读取更新前设置快照失败：{error}")))?;

    let restored_count = state.db.with_conn(|conn| {
        let mut restored_count = 0;
        for entry in &snapshot.settings {
            if !protected_update_setting_keys().contains(&entry.key.as_str()) {
                continue;
            }
            let should_restore = repository::get_setting(conn, &entry.key)?
                .map(|value| value.trim().is_empty())
                .unwrap_or(true);
            if should_restore {
                repository::set_setting(conn, &entry.key, &entry.value)?;
                restored_count += 1;
            }
        }
        Ok(restored_count)
    })?;

    if restored_count == 0 {
        let _ = fs::remove_file(snapshot_path);
    }
    Ok(())
}

fn write_update_settings_snapshot(state: &AppState) -> AppResult<()> {
    let settings = state.db.with_conn(|conn| {
        let mut settings = Vec::new();
        for key in protected_update_setting_keys() {
            if let Some(value) =
                repository::get_setting(conn, key)?.filter(|value| !value.trim().is_empty())
            {
                settings.push(UpdateSettingValue {
                    key: key.to_string(),
                    value,
                });
            }
        }
        Ok(settings)
    })?;

    if settings.is_empty() {
        return Ok(());
    }

    fs::create_dir_all(&state.paths.data_dir)?;
    let snapshot = UpdateSettingsSnapshot {
        created_at: chrono::Utc::now().to_rfc3339(),
        settings,
    };
    let text = serde_json::to_string_pretty(&snapshot)
        .map_err(|error| AppError::Validation(format!("生成更新前设置快照失败：{error}")))?;
    fs::write(update_settings_snapshot_path(state), text)?;
    Ok(())
}

fn update_settings_snapshot_path(state: &AppState) -> PathBuf {
    state.paths.data_dir.join(UPDATE_SETTINGS_SNAPSHOT_FILE)
}

fn protected_update_setting_keys() -> &'static [&'static str] {
    &[
        "hotel_name",
        "current_period",
        "default_month",
        "allow_negative_stock",
        "quantity_decimals",
        "amount_decimals",
        "default_export_dir",
        "default_backup_dir",
        "auto_backup_enabled",
        "interval_backup_enabled",
        "interval_backup_hours",
        "smtp_enabled",
        "smtp_host",
        "smtp_port",
        "smtp_username",
        "smtp_password_configured",
        "smtp_from_email",
        "smtp_from_name",
        "second_backup_dir",
    ]
}

fn stable_client_device_id(conn: &rusqlite::Connection) -> AppResult<String> {
    if let Some(existing) = repository::get_setting(conn, CLIENT_DEVICE_ID_KEY)?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return Ok(existing);
    }
    let generated = format!("device-{}", uuid::Uuid::new_v4());
    repository::set_setting(conn, CLIENT_DEVICE_ID_KEY, &generated)?;
    Ok(generated)
}

pub fn get_system_settings(state: &AppState) -> AppResult<SystemSettings> {
    crate::services::user_service::require_admin(state)?;
    if get_runtime_config(state)?.mode == RuntimeMode::Client {
        let remote_settings = crate::services::host_service::remote_get_system_settings(state).ok();
        return state.db.with_conn(|conn| {
            let settings = match remote_settings {
                Some(settings) => settings,
                None => system_settings_from_conn(
                    conn,
                    Some((
                        state.paths.export_dir.display().to_string(),
                        state.paths.backup_dir.display().to_string(),
                    )),
                )?,
            };
            with_local_directory_settings(conn, state, settings)
        });
    }
    let smtp_password_configured = crate::application::secret_service::load(
        &state.db,
        crate::application::secret_service::ApplicationSecret::SmtpPassword,
    )?
    .is_some();
    state.db.with_conn(|conn| {
        let mut settings = system_settings_from_conn(
            conn,
            Some((
                state.paths.export_dir.display().to_string(),
                state.paths.backup_dir.display().to_string(),
            )),
        )?;
        settings.smtp_password_configured = smtp_password_configured;
        Ok(settings)
    })
}

pub fn list_audit_logs(state: &AppState, limit: Option<i64>) -> AppResult<Vec<AuditLogRow>> {
    crate::services::user_service::require_admin(state)?;
    if get_runtime_config(state)?.mode == RuntimeMode::Client {
        return crate::services::host_service::remote_list_audit_logs(state, limit);
    }
    state
        .db
        .with_conn(|conn| repository::list_audit_logs(conn, limit.unwrap_or(100)))
}

pub fn system_settings_from_conn(
    conn: &rusqlite::Connection,
    fallback_dirs: Option<(String, String)>,
) -> AppResult<SystemSettings> {
    let (fallback_export_dir, fallback_backup_dir) = fallback_dirs.unwrap_or_default();
    Ok(SystemSettings {
        hotel_name: setting_or(conn, "hotel_name", "Aster Hotel")?,
        current_period: setting_or(
            conn,
            "current_period",
            &chrono::Local::now().format("%Y-%m").to_string(),
        )?,
        default_month: setting_or(
            conn,
            "default_month",
            &chrono::Local::now().format("%Y-%m").to_string(),
        )?,
        allow_negative_stock: setting_bool(conn, "allow_negative_stock", false)?,
        quantity_decimals: setting_i64(conn, "quantity_decimals", 2, 0, 6)?,
        amount_decimals: setting_i64(conn, "amount_decimals", 2, 0, 6)?,
        default_export_dir: setting_or(conn, "default_export_dir", &fallback_export_dir)?,
        default_backup_dir: setting_or(conn, "default_backup_dir", &fallback_backup_dir)?,
        auto_backup_enabled: setting_bool(conn, "auto_backup_enabled", true)?,
        interval_backup_enabled: setting_bool(conn, "interval_backup_enabled", true)?,
        interval_backup_hours: setting_i64(conn, "interval_backup_hours", 6, 1, 168)?,
        smtp_enabled: setting_bool(conn, "smtp_enabled", false)?,
        smtp_host: setting_or(conn, "smtp_host", "")?,
        smtp_port: setting_i64(conn, "smtp_port", 465, 1, 65535)?,
        smtp_username: setting_or(conn, "smtp_username", "")?,
        smtp_from_email: setting_or(conn, "smtp_from_email", "")?,
        smtp_from_name: setting_or(conn, "smtp_from_name", "Aster")?,
        smtp_password_configured: setting_bool(conn, "smtp_password_configured", false)?
            || repository::get_setting(conn, "smtp_password")?
                .as_deref()
                .is_some_and(|value| !value.is_empty()),
    })
}

pub fn save_system_settings(
    state: &AppState,
    request: SaveSystemSettingsRequest,
) -> AppResult<SystemSettings> {
    crate::services::user_service::require_admin(state)?;
    if get_runtime_config(state)?.mode == RuntimeMode::Client {
        return save_client_local_directory_settings(state, request);
    }
    crate::services::safety_service::require_local_primary_database(state, "保存系统设置")?;
    validate_settings(&request)?;
    fs::create_dir_all(request.default_export_dir.trim())?;
    fs::create_dir_all(request.default_backup_dir.trim())?;
    ensure_writable_dir(Path::new(request.default_export_dir.trim()), "默认导出目录")?;
    ensure_writable_dir(Path::new(request.default_backup_dir.trim()), "默认备份目录")?;
    let next_password = request
        .smtp_password
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let previous_password = crate::application::secret_service::load(
        &state.db,
        crate::application::secret_service::ApplicationSecret::SmtpPassword,
    )?;
    if let Some(password) = next_password.as_deref() {
        crate::application::secret_service::save(
            &state.db,
            crate::application::secret_service::ApplicationSecret::SmtpPassword,
            password,
        )?;
    }
    let smtp_password_configured = next_password.is_some() || previous_password.is_some();

    let persist_result = state.db.with_conn_mut(|conn| {
        let transaction = conn.transaction()?;
        repository::set_setting(&transaction, "hotel_name", request.hotel_name.trim())?;
        repository::set_setting(
            &transaction,
            "current_period",
            request.current_period.trim(),
        )?;
        repository::set_setting(&transaction, "default_month", request.default_month.trim())?;
        repository::set_setting(
            &transaction,
            "allow_negative_stock",
            bool_setting(request.allow_negative_stock),
        )?;
        repository::set_setting(
            &transaction,
            "quantity_decimals",
            &request.quantity_decimals.to_string(),
        )?;
        repository::set_setting(
            &transaction,
            "amount_decimals",
            &request.amount_decimals.to_string(),
        )?;
        repository::set_setting(
            &transaction,
            "default_export_dir",
            request.default_export_dir.trim(),
        )?;
        repository::set_setting(
            &transaction,
            "default_backup_dir",
            request.default_backup_dir.trim(),
        )?;
        repository::set_setting(
            &transaction,
            "auto_backup_enabled",
            bool_setting(request.auto_backup_enabled),
        )?;
        repository::set_setting(
            &transaction,
            "interval_backup_enabled",
            bool_setting(request.interval_backup_enabled),
        )?;
        repository::set_setting(
            &transaction,
            "interval_backup_hours",
            &request.interval_backup_hours.to_string(),
        )?;
        repository::set_setting(
            &transaction,
            "smtp_enabled",
            bool_setting(request.smtp_enabled),
        )?;
        repository::set_setting(&transaction, "smtp_host", request.smtp_host.trim())?;
        repository::set_setting(&transaction, "smtp_port", &request.smtp_port.to_string())?;
        repository::set_setting(&transaction, "smtp_username", request.smtp_username.trim())?;
        repository::set_setting(
            &transaction,
            "smtp_from_email",
            request.smtp_from_email.trim(),
        )?;
        repository::set_setting(
            &transaction,
            "smtp_from_name",
            request.smtp_from_name.trim(),
        )?;
        repository::set_setting(
            &transaction,
            crate::application::secret_service::SMTP_PASSWORD_CONFIGURED_SETTING,
            bool_setting(smtp_password_configured),
        )?;
        repository::delete_setting(
            &transaction,
            crate::application::secret_service::SMTP_PASSWORD_SETTING,
        )?;
        transaction.execute(
            "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
             VALUES (?1, 'save_system_settings', 'setting', 'system', ?2, ?3)",
            rusqlite::params![
                uuid::Uuid::new_v4().to_string(),
                format!(
                    "保存系统设置：酒店={}，账期={}，允许负库存={}",
                    request.hotel_name.trim(),
                    request.current_period.trim(),
                    request.allow_negative_stock
                ),
                crate::services::user_service::current_operator(state)
            ],
        )?;
        transaction.commit()?;
        Ok(())
    });
    if let Err(error) = persist_result {
        if next_password.is_some() {
            let rollback = match previous_password {
                Some(previous) => crate::application::secret_service::save(
                    &state.db,
                    crate::application::secret_service::ApplicationSecret::SmtpPassword,
                    &previous,
                ),
                None => crate::application::secret_service::delete(
                    &state.db,
                    crate::application::secret_service::ApplicationSecret::SmtpPassword,
                ),
            };
            if let Err(rollback_error) = rollback {
                return Err(AppError::Validation(format!(
                    "{error}；SMTP 安全凭据回滚失败：{rollback_error}"
                )));
            }
        }
        return Err(error);
    }
    get_system_settings(state)
}

fn save_client_local_directory_settings(
    state: &AppState,
    request: SaveSystemSettingsRequest,
) -> AppResult<SystemSettings> {
    validate_local_directory_settings(&request)?;
    fs::create_dir_all(request.default_export_dir.trim())?;
    fs::create_dir_all(request.default_backup_dir.trim())?;
    ensure_writable_dir(Path::new(request.default_export_dir.trim()), "默认导出目录")?;
    ensure_writable_dir(Path::new(request.default_backup_dir.trim()), "默认备份目录")?;

    state.db.with_conn_mut(|conn| {
        let transaction = conn.transaction()?;
        repository::set_setting(
            &transaction,
            "default_export_dir",
            request.default_export_dir.trim(),
        )?;
        repository::set_setting(
            &transaction,
            "default_backup_dir",
            request.default_backup_dir.trim(),
        )?;
        transaction.execute(
            "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
             VALUES (?1, 'save_local_directory_settings', 'setting', 'local_directories', ?2, ?3)",
            rusqlite::params![
                uuid::Uuid::new_v4().to_string(),
                format!(
                    "保存本机目录设置：导出目录={}，备份目录={}",
                    request.default_export_dir.trim(),
                    request.default_backup_dir.trim()
                ),
                crate::services::user_service::current_operator(state)
            ],
        )?;
        transaction.commit()?;
        Ok(())
    })?;
    get_system_settings(state)
}
