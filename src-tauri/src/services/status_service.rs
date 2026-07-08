use std::fs;
use std::path::Path;

use crate::app::state::AppState;
use crate::db::backup_repository;
use crate::db::repository;
use crate::domain::runtime::{RuntimeConfig, RuntimeMode};
use crate::domain::status::{
    AppStatus, AuditLogRow, DashboardMetrics, HealthStatus, SaveSystemSettingsRequest,
    SystemSettings,
};
use crate::error::{AppError, AppResult};

const CLIENT_DEVICE_ID_KEY: &str = "client_device_id";

pub fn get_runtime_config(state: &AppState) -> AppResult<RuntimeConfig> {
    state.db.with_conn(|conn| {
        let mode_value = repository::get_setting(conn, "runtime_mode")?
            .unwrap_or_else(|| RuntimeMode::Standalone.as_str().to_string());
        let mode = RuntimeMode::parse(&mode_value)?;
        let host_address = repository::get_setting(conn, "host_address")?;
        let client_token = repository::get_setting(conn, "client_token")?;
        let client_device_id = stable_client_device_id(conn)?;
        let host_port = repository::get_setting(conn, "host_port")?
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(17871);

        Ok(RuntimeConfig {
            mode,
            host_address,
            host_port,
            client_token,
            client_device_id,
            data_dir: state.paths.data_dir.display().to_string(),
            database_path: state.paths.database_path.display().to_string(),
            backup_dir: effective_backup_dir_from_conn(conn, state),
            export_dir: effective_export_dir_from_conn(conn, state),
            import_report_dir: state.paths.import_report_dir.display().to_string(),
        })
    })
}

pub fn set_runtime_mode(state: &AppState, mode: RuntimeMode) -> AppResult<RuntimeConfig> {
    crate::services::user_service::require_admin(state)?;
    state.db.with_conn(|conn| {
        repository::set_setting(conn, "runtime_mode", mode.as_str())?;
        repository::set_setting(
            conn,
            "last_runtime_mode_changed_at",
            &chrono::Utc::now().to_rfc3339(),
        )?;
        conn.execute(
            "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
             VALUES (?1, 'set_runtime_mode', 'setting', 'runtime_mode', ?2, ?3)",
            rusqlite::params![
                uuid::Uuid::new_v4().to_string(),
                format!("切换运行模式：{}", mode.as_str()),
                crate::services::user_service::current_operator(state)
            ],
        )?;
        Ok(())
    })?;
    crate::services::host_service::ensure_host_service_for_mode(state, env!("CARGO_PKG_VERSION"))?;
    get_runtime_config(state)
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
    state.db.with_conn(|conn| {
        system_settings_from_conn(
            conn,
            Some((
                state.paths.export_dir.display().to_string(),
                state.paths.backup_dir.display().to_string(),
            )),
        )
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
        smtp_password_configured: repository::get_setting(conn, "smtp_password")?
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

    state.db.with_conn(|conn| {
        repository::set_setting(conn, "hotel_name", request.hotel_name.trim())?;
        repository::set_setting(conn, "current_period", request.current_period.trim())?;
        repository::set_setting(conn, "default_month", request.default_month.trim())?;
        repository::set_setting(
            conn,
            "allow_negative_stock",
            bool_setting(request.allow_negative_stock),
        )?;
        repository::set_setting(
            conn,
            "quantity_decimals",
            &request.quantity_decimals.to_string(),
        )?;
        repository::set_setting(
            conn,
            "amount_decimals",
            &request.amount_decimals.to_string(),
        )?;
        repository::set_setting(
            conn,
            "default_export_dir",
            request.default_export_dir.trim(),
        )?;
        repository::set_setting(
            conn,
            "default_backup_dir",
            request.default_backup_dir.trim(),
        )?;
        repository::set_setting(
            conn,
            "auto_backup_enabled",
            bool_setting(request.auto_backup_enabled),
        )?;
        repository::set_setting(
            conn,
            "interval_backup_enabled",
            bool_setting(request.interval_backup_enabled),
        )?;
        repository::set_setting(
            conn,
            "interval_backup_hours",
            &request.interval_backup_hours.to_string(),
        )?;
        repository::set_setting(conn, "smtp_enabled", bool_setting(request.smtp_enabled))?;
        repository::set_setting(conn, "smtp_host", request.smtp_host.trim())?;
        repository::set_setting(conn, "smtp_port", &request.smtp_port.to_string())?;
        repository::set_setting(conn, "smtp_username", request.smtp_username.trim())?;
        repository::set_setting(conn, "smtp_from_email", request.smtp_from_email.trim())?;
        repository::set_setting(conn, "smtp_from_name", request.smtp_from_name.trim())?;
        if let Some(password) = request.smtp_password.as_deref() {
            let trimmed_password = password.trim();
            if !trimmed_password.is_empty() {
                repository::set_setting(conn, "smtp_password", trimmed_password)?;
            }
        }
        conn.execute(
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
        Ok(())
    })?;
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

    state.db.with_conn(|conn| {
        repository::set_setting(
            conn,
            "default_export_dir",
            request.default_export_dir.trim(),
        )?;
        repository::set_setting(
            conn,
            "default_backup_dir",
            request.default_backup_dir.trim(),
        )?;
        conn.execute(
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
        Ok(())
    })?;
    get_system_settings(state)
}

pub fn allow_negative_stock(state: &AppState) -> AppResult<bool> {
    state
        .db
        .with_conn(|conn| setting_bool(conn, "allow_negative_stock", false))
}

pub fn effective_export_dir(state: &AppState) -> AppResult<std::path::PathBuf> {
    state.db.with_conn(|conn| {
        Ok(std::path::PathBuf::from(effective_export_dir_from_conn(
            conn, state,
        )))
    })
}

pub fn effective_backup_dir(state: &AppState) -> AppResult<std::path::PathBuf> {
    state.db.with_conn(|conn| {
        Ok(std::path::PathBuf::from(effective_backup_dir_from_conn(
            conn, state,
        )))
    })
}

pub fn get_app_status(state: &AppState, app_version: &str) -> AppResult<AppStatus> {
    let local_runtime = get_runtime_config(state)?;
    if crate::services::user_service::current_user(state)?.is_none() {
        return state.db.with_conn(|conn| {
            client_shell_status(
                conn,
                app_version,
                local_runtime,
                "未登录，仅显示本机连接配置".to_string(),
            )
        });
    }
    if local_runtime.mode == RuntimeMode::Client {
        return match crate::services::host_service::remote_get_app_status(state) {
            Ok(mut remote_status) => {
                remote_status.runtime = local_runtime;
                Ok(remote_status)
            }
            Err(error) => state.db.with_conn(|conn| {
                client_shell_status(
                    conn,
                    app_version,
                    local_runtime,
                    format!("主机连接异常，业务操作已暂停：{error}"),
                )
            }),
        };
    }

    let department_scope = crate::services::user_service::current_department_scope(state)?;
    state.db.with_conn(|conn| {
        build_app_status(
            conn,
            app_version,
            Some(local_runtime),
            department_scope.as_deref(),
        )
    })
}

fn client_shell_status(
    conn: &rusqlite::Connection,
    app_version: &str,
    runtime: RuntimeConfig,
    message: String,
) -> AppResult<AppStatus> {
    Ok(AppStatus {
        app_name: "Aster".to_string(),
        app_version: app_version.to_string(),
        schema_version: repository::schema_version(conn)?,
        runtime,
        latest_movement_month: repository::latest_movement_month(conn)?,
        metrics: DashboardMetrics {
            item_count: 0,
            department_count: 0,
            supplier_count: 0,
            current_stock_amount: 0.0,
            low_stock_count: 0,
            negative_stock_count: 0,
            this_month_inbound_amount: 0.0,
            this_month_outbound_amount: 0.0,
        },
        recent_operations: Vec::new(),
        health: HealthStatus {
            database_ok: true,
            stock_balance_consistency_ok: true,
            stock_balance_issue_count: 0,
            latest_backup_at: None,
            latest_interval_backup_at: None,
            auto_backup_enabled: false,
            interval_backup_enabled: false,
            interval_backup_hours: 0,
            second_backup_ok: false,
            message,
        },
    })
}

pub fn build_app_status(
    conn: &rusqlite::Connection,
    app_version: &str,
    runtime: Option<RuntimeConfig>,
    department_scope: Option<&str>,
) -> AppResult<AppStatus> {
    let schema_version = repository::schema_version(conn)?;
    let metrics = repository::dashboard_metrics(conn, department_scope)?;
    let recent_operations = repository::recent_operations(conn, 8, department_scope)?;
    let latest_movement_month = repository::latest_movement_month(conn)?;
    let latest_backup_at = repository::latest_successful_backup(conn)?;
    let latest_interval_backup_at =
        backup_repository::latest_successful_backup_at(conn, "auto_interval")?;
    let auto_backup_enabled = repository::get_setting(conn, "auto_backup_enabled")?
        .unwrap_or_else(|| "true".to_string())
        == "true";
    let interval_backup_enabled = repository::get_setting(conn, "interval_backup_enabled")?
        .unwrap_or_else(|| "true".to_string())
        == "true";
    let interval_backup_hours = repository::get_setting(conn, "interval_backup_hours")?
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(6);
    let integrity = repository::integrity_check(conn)?;
    let stock_balance_issue_count = repository::stock_balance_consistency_issue_count(conn)?;
    let stock_balance_consistency_ok = stock_balance_issue_count == 0;
    let second_backup_path = repository::get_setting(conn, "second_backup_dir")?;
    let second_backup_ok = second_backup_path
        .as_deref()
        .map(|path| {
            fs::metadata(path)
                .map(|meta| meta.is_dir())
                .unwrap_or(false)
        })
        .unwrap_or(false);

    Ok(AppStatus {
        app_name: "Aster".to_string(),
        app_version: app_version.to_string(),
        schema_version,
        runtime: runtime.unwrap_or_else(host_runtime_config),
        latest_movement_month,
        metrics,
        recent_operations,
        health: HealthStatus {
            database_ok: integrity == "ok" && stock_balance_consistency_ok,
            stock_balance_consistency_ok,
            stock_balance_issue_count,
            latest_backup_at,
            latest_interval_backup_at,
            auto_backup_enabled,
            interval_backup_enabled,
            interval_backup_hours,
            second_backup_ok,
            message: health_message(&integrity, stock_balance_issue_count),
        },
    })
}

fn health_message(integrity: &str, stock_balance_issue_count: i64) -> String {
    match (integrity == "ok", stock_balance_issue_count) {
        (true, 0) => "数据库健康检查通过，库存余额与流水一致".to_string(),
        (false, 0) => format!("数据库健康检查异常：{integrity}"),
        (true, count) => format!("库存余额与流水存在 {count} 项不一致"),
        (false, count) => {
            format!("数据库健康检查异常：{integrity}；库存余额与流水存在 {count} 项不一致")
        }
    }
}

fn host_runtime_config() -> RuntimeConfig {
    RuntimeConfig {
        mode: RuntimeMode::Host,
        host_address: None,
        host_port: 17871,
        client_token: None,
        client_device_id: String::new(),
        data_dir: String::new(),
        database_path: String::new(),
        backup_dir: String::new(),
        export_dir: String::new(),
        import_report_dir: String::new(),
    }
}

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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::app::paths::AppPaths;
    use crate::app::state::AppState;
    use crate::db::connection::Db;
    use crate::domain::status::SaveSystemSettingsRequest;
    use crate::domain::users::{CurrentUser, Role};

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

    fn set_admin_user(state: &AppState) {
        *state.session.lock().expect("session mutex poisoned") = Some(CurrentUser {
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
            permissions: vec![
                "dangerous_operations".to_string(),
                "manage_settings".to_string(),
            ],
        });
    }

    #[test]
    fn save_system_settings_persists_values_and_writes_audit_log() {
        let state = test_state();
        set_admin_user(&state);
        let export_dir = state.paths.data_dir.join("custom-exports");
        let backup_dir = state.paths.data_dir.join("custom-backups");

        let settings = save_system_settings(
            &state,
            SaveSystemSettingsRequest {
                hotel_name: "测试酒店".to_string(),
                current_period: "2026-06".to_string(),
                default_month: "2026-07".to_string(),
                allow_negative_stock: true,
                quantity_decimals: 3,
                amount_decimals: 2,
                default_export_dir: export_dir.display().to_string(),
                default_backup_dir: backup_dir.display().to_string(),
                auto_backup_enabled: false,
                interval_backup_enabled: true,
                interval_backup_hours: 2,
                smtp_enabled: false,
                smtp_host: String::new(),
                smtp_port: 465,
                smtp_username: String::new(),
                smtp_password: None,
                smtp_from_email: String::new(),
                smtp_from_name: "Aster".to_string(),
            },
        )
        .unwrap();

        assert_eq!(settings.hotel_name, "测试酒店");
        assert!(settings.allow_negative_stock);
        assert_eq!(
            settings.default_export_dir,
            export_dir.display().to_string()
        );
        assert_eq!(
            settings.default_backup_dir,
            backup_dir.display().to_string()
        );
        assert!(!settings.auto_backup_enabled);

        let audit_count: i64 = state
            .db
            .with_conn(|conn| {
                Ok(conn.query_row(
                    "SELECT COUNT(*) FROM audit_logs WHERE action = 'save_system_settings'",
                    [],
                    |row| row.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(audit_count, 1);
    }

    #[test]
    fn save_system_settings_in_client_mode_only_persists_local_directories() {
        let state = test_state();
        set_admin_user(&state);
        let export_dir = state.paths.data_dir.join("client-exports");
        let backup_dir = state.paths.data_dir.join("client-backups");
        state
            .db
            .with_conn(|conn| repository::set_setting(conn, "runtime_mode", "client"))
            .unwrap();

        let settings = save_system_settings(
            &state,
            SaveSystemSettingsRequest {
                hotel_name: "客户端酒店".to_string(),
                current_period: "2026-06".to_string(),
                default_month: "2026-07".to_string(),
                allow_negative_stock: false,
                quantity_decimals: 2,
                amount_decimals: 2,
                default_export_dir: export_dir.display().to_string(),
                default_backup_dir: backup_dir.display().to_string(),
                auto_backup_enabled: true,
                interval_backup_enabled: true,
                interval_backup_hours: 6,
                smtp_enabled: false,
                smtp_host: String::new(),
                smtp_port: 465,
                smtp_username: String::new(),
                smtp_password: None,
                smtp_from_email: String::new(),
                smtp_from_name: "Aster".to_string(),
            },
        )
        .unwrap();

        assert_eq!(settings.hotel_name, "Aster Hotel");
        assert_eq!(
            settings.default_export_dir,
            export_dir.display().to_string()
        );
        assert_eq!(
            settings.default_backup_dir,
            backup_dir.display().to_string()
        );
        state
            .db
            .with_conn(|conn| {
                let hotel_name = repository::get_setting(conn, "hotel_name")?;
                let audit_count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM audit_logs WHERE action = 'save_local_directory_settings'",
                    [],
                    |row| row.get(0),
                )?;
                assert_ne!(hotel_name.as_deref(), Some("客户端酒店"));
                assert_eq!(audit_count, 1);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn list_audit_logs_requires_admin_and_returns_recent_rows() {
        let state = test_state();
        set_admin_user(&state);
        state
            .db
            .with_conn(|conn| {
                Ok(conn.execute(
                    "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator, created_at)
                     VALUES ('audit-service', 'save_item', 'item', 'item-1', '服务层查询', 'admin', '2026-06-30T10:00:00+08:00')",
                    [],
                )?)
            })
            .unwrap();

        let rows = list_audit_logs(&state, Some(20)).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "audit-service");
        assert_eq!(rows[0].operator, "admin");
    }

    #[test]
    fn system_settings_from_conn_reads_stored_values_without_local_fallback() {
        let state = test_state();
        state
            .db
            .with_conn(|conn| {
                repository::set_setting(conn, "hotel_name", "主机酒店")?;
                repository::set_setting(conn, "current_period", "2026-06")?;
                repository::set_setting(conn, "default_month", "2026-07")?;
                repository::set_setting(conn, "default_export_dir", "/host/export")?;
                repository::set_setting(conn, "default_backup_dir", "/host/backup")?;
                system_settings_from_conn(conn, None)
            })
            .map(|settings| {
                assert_eq!(settings.hotel_name, "主机酒店");
                assert_eq!(settings.current_period, "2026-06");
                assert_eq!(settings.default_export_dir, "/host/export");
                assert_eq!(settings.default_backup_dir, "/host/backup");
            })
            .unwrap();
    }

    #[test]
    fn build_app_status_uses_database_metrics_and_recent_operations() {
        let state = test_state();
        let runtime = get_runtime_config(&state).unwrap();
        state
            .db
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO master_items (id, code, name, unit_id, default_price)
                     VALUES ('item-status', 'STA-001', '状态物品', 'unit-piece', 8)",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO stock_balances (item_id, quantity, amount, average_price, updated_at)
                     VALUES ('item-status', -2, -16, 8, '2026-06-30T10:00:00+08:00')",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO stock_movements (
                       id, movement_date, item_id, direction, quantity, unit_price, amount,
                       department_id, movement_type, created_at
                     )
                     VALUES (
                       'mov-status', '2026-06-30', 'item-status', 'out', 2, 8, 16,
                       'dept-admin-office', 'outbound', '2026-06-30T10:00:00+08:00'
                     )",
                    [],
                )?;
                build_app_status(conn, "0.1.0-test", Some(runtime), None)
            })
            .map(|status| {
                assert_eq!(status.metrics.item_count, 1);
                assert_eq!(status.metrics.current_stock_amount, -16.0);
                assert_eq!(status.recent_operations.len(), 1);
                assert_eq!(status.recent_operations[0].item_name, "状态物品");
                assert!(status.health.database_ok);
                assert!(status.health.stock_balance_consistency_ok);
            })
            .unwrap();
    }

    #[test]
    fn build_app_status_scopes_department_viewer_recent_operations_and_outbound_amount() {
        let state = test_state();
        let runtime = get_runtime_config(&state).unwrap();
        let status = state
            .db
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO master_items (id, code, name, unit_id, default_price)
                     VALUES ('item-status-scope', 'STS-001', '状态范围物品', 'unit-piece', 8)",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO stock_movements (
                       id, movement_date, item_id, direction, quantity, unit_price, amount,
                       department_id, department_name, movement_type, created_at
                     )
                     VALUES
                       ('mov-status-admin-scope', '2026-07-01', 'item-status-scope', 'out', 2, 8, 16,
                        'dept-admin-office', '行政办', 'outbound', '2026-07-01T10:00:00+08:00'),
                       ('mov-status-restaurant-scope', '2026-07-01', 'item-status-scope', 'out', 3, 8, 24,
                        'dept-restaurant', '餐饮', 'outbound', '2026-07-01T11:00:00+08:00')",
                    [],
                )?;
                build_app_status(
                    conn,
                    "0.1.0-test",
                    Some(runtime),
                    Some("dept-admin-office"),
                )
            })
            .unwrap();

        assert_eq!(status.metrics.this_month_outbound_amount, 16.0);
        assert_eq!(status.recent_operations.len(), 1);
        assert_eq!(
            status.recent_operations[0].department_name.as_deref(),
            Some("行政办")
        );
    }

    #[test]
    fn build_app_status_marks_stock_balance_mismatch_unhealthy() {
        let state = test_state();
        let runtime = get_runtime_config(&state).unwrap();
        let status = state
            .db
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO master_items (id, code, name, unit_id, default_price)
                     VALUES ('item-mismatch', 'MIS-001', '异常物品', 'unit-piece', 8)",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO stock_balances (item_id, quantity, amount, average_price, updated_at)
                     VALUES ('item-mismatch', 5, 40, 8, '2026-06-30T10:00:00+08:00')",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO stock_movements (
                       id, movement_date, item_id, direction, quantity, unit_price, amount,
                       movement_type, created_at
                     )
                     VALUES (
                       'mov-mismatch', '2026-06-30', 'item-mismatch', 'in', 4, 8, 32,
                       'inbound', '2026-06-30T10:00:00+08:00'
                     )",
                    [],
                )?;
                build_app_status(conn, "0.1.0-test", Some(runtime), None)
            })
            .unwrap();

        assert!(!status.health.database_ok);
        assert!(!status.health.stock_balance_consistency_ok);
        assert_eq!(status.health.stock_balance_issue_count, 1);
        assert!(status
            .health
            .message
            .contains("库存余额与流水存在 1 项不一致"));
    }

    #[test]
    fn get_runtime_config_generates_stable_client_device_id() {
        let state = test_state();

        let first = get_runtime_config(&state).unwrap();
        let second = get_runtime_config(&state).unwrap();
        let stored = state
            .db
            .with_conn(|conn| repository::get_setting(conn, CLIENT_DEVICE_ID_KEY))
            .unwrap();

        assert!(first.client_device_id.starts_with("device-"));
        assert_eq!(first.client_device_id, second.client_device_id);
        assert_eq!(stored.as_deref(), Some(first.client_device_id.as_str()));
    }

    #[test]
    fn client_mode_status_without_login_uses_local_shell_status() {
        let state = test_state();
        state
            .db
            .with_conn(|conn| {
                repository::set_setting(conn, "runtime_mode", RuntimeMode::Client.as_str())?;
                repository::set_setting(conn, "host_address", "127.0.0.1")?;
                repository::set_setting(conn, "host_port", "17871")
            })
            .unwrap();

        let status = get_app_status(&state, "0.1.0-test").unwrap();

        assert_eq!(status.runtime.mode, RuntimeMode::Client);
        assert_eq!(status.runtime.host_address.as_deref(), Some("127.0.0.1"));
        assert_eq!(status.metrics.item_count, 0);
        assert!(status.health.message.contains("客户端未登录"));
    }

    #[test]
    fn client_mode_status_with_unreachable_host_falls_back_to_local_shell_status() {
        let state = test_state();
        *state.session.lock().expect("session mutex poisoned") = Some(CurrentUser {
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
            permissions: vec![
                "dangerous_operations".to_string(),
                "manage_settings".to_string(),
                "view_reports".to_string(),
            ],
        });
        state
            .db
            .with_conn(|conn| {
                repository::set_setting(conn, "runtime_mode", RuntimeMode::Client.as_str())?;
                repository::set_setting(conn, "host_address", "127.0.0.1")?;
                repository::set_setting(conn, "host_port", "9")?;
                repository::set_setting(conn, "client_token", "paired-token")
            })
            .unwrap();

        let status = get_app_status(&state, "0.1.0-test").unwrap();

        assert_eq!(status.runtime.mode, RuntimeMode::Client);
        assert_eq!(status.runtime.host_port, 9);
        assert_eq!(status.metrics.item_count, 0);
        assert!(status.health.message.contains("主机连接异常"));
        assert!(status.health.message.contains("业务操作已暂停"));
    }
}
