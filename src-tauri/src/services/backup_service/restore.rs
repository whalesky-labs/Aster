fn verify_restored_database_health(conn: &rusqlite::Connection) -> AppResult<()> {
    let integrity: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(AppError::Validation(format!(
            "恢复后的数据库完整性检查异常：{integrity}"
        )));
    }
    let foreign_key_issues: i64 =
        conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })?;
    if foreign_key_issues > 0 {
        return Err(AppError::Validation(format!(
            "恢复后的数据库外键检查异常：{foreign_key_issues} 处"
        )));
    }
    Ok(())
}

fn rollback_restore(
    state: &AppState,
    protected_backup_file: &str,
    failed_restore_database: &Path,
) -> AppResult<()> {
    let protected_database = extract_database_to_temp(Path::new(protected_backup_file))?;
    let rollback_result = state.db.replace_database(&state.paths, &protected_database);
    let _ = fs::remove_file(&protected_database);
    let _ = fs::remove_file(failed_restore_database);
    rollback_result.map_err(|error| {
        AppError::Validation(format!(
            "恢复失败后自动回滚也失败，请手工使用恢复前保护备份：{error}"
        ))
    })
}

fn reset_restored_local_paths(conn: &rusqlite::Connection, state: &AppState) -> AppResult<()> {
    fs::create_dir_all(&state.paths.export_dir)?;
    fs::create_dir_all(&state.paths.backup_dir)?;
    repository::set_setting(
        conn,
        "default_export_dir",
        &state.paths.export_dir.display().to_string(),
    )?;
    repository::set_setting(
        conn,
        "default_backup_dir",
        &state.paths.backup_dir.display().to_string(),
    )?;
    repository::set_setting(conn, "second_backup_dir", "")?;
    Ok(())
}

pub fn run_startup_backup_if_needed(state: &AppState) -> AppResult<()> {
    crate::services::safety_service::require_local_primary_database(state, "启动自动备份")?;
    let should_backup = state.db.with_conn(|conn| {
        let enabled = repository::get_setting(conn, "auto_backup_enabled")?
            .unwrap_or_else(|| "true".to_string());
        if enabled != "true" {
            return Ok(false);
        }
        let count: i64 = conn.query_row(
            "SELECT COUNT(*)
             FROM backup_jobs
             WHERE backup_type = 'auto_startup'
               AND status = 'success'
               AND date(created_at) = date('now', 'localtime')",
            [],
            |row| row.get(0),
        )?;
        Ok(count == 0)
    })?;
    if should_backup {
        let _ = create_backup_of_type(state, "auto_startup")?;
    }
    Ok(())
}

pub fn start_interval_backup_worker(state: &AppState) {
    let paths = state.paths.clone();
    let db = state.db.clone_handle();
    thread::spawn(move || {
        let state = AppState {
            paths,
            db,
            session: std::sync::Arc::new(std::sync::Mutex::new(None)),
            host_service: Arc::new(std::sync::Mutex::new(
                crate::services::host_service::HostServiceRuntime::default(),
            )),
        };
        loop {
            thread::sleep(Duration::from_secs(15 * 60));
            let _ = run_interval_backup_if_needed(&state);
        }
    });
}

pub fn run_interval_backup_if_needed(state: &AppState) -> AppResult<()> {
    crate::services::safety_service::require_local_primary_database(state, "运行中定时备份")?;
    let should_backup = state.db.with_conn(|conn| {
        let auto_enabled = repository::get_setting(conn, "auto_backup_enabled")?
            .unwrap_or_else(|| "true".to_string());
        let interval_enabled = repository::get_setting(conn, "interval_backup_enabled")?
            .unwrap_or_else(|| "true".to_string());
        if auto_enabled != "true" || interval_enabled != "true" {
            return Ok(false);
        }
        let hours = repository::get_setting(conn, "interval_backup_hours")?
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_INTERVAL_BACKUP_HOURS);
        let Some(last_backup_at) =
            backup_repository::latest_successful_backup_at(conn, "auto_interval")?
        else {
            return Ok(true);
        };
        let Ok(last_backup_at) = parse_backup_timestamp(&last_backup_at) else {
            return Ok(true);
        };
        Ok(Local::now()
            .signed_duration_since(last_backup_at)
            .num_hours()
            >= hours)
    })?;
    if should_backup {
        let _ = create_backup_of_type(state, "auto_interval")?;
    }
    Ok(())
}

include!("automation.rs");
include!("archive.rs");
