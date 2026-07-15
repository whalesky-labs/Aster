use std::fs;
use std::path::{Path, PathBuf};

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
const UPDATE_SETTINGS_SNAPSHOT_FILE: &str = "aster-update-settings-snapshot.json";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct UpdateSettingsSnapshot {
    created_at: String,
    settings: Vec<UpdateSettingValue>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct UpdateSettingValue {
    key: String,
    value: String,
}

pub fn get_runtime_config(state: &AppState) -> AppResult<RuntimeConfig> {
    let mut config = state.db.with_conn(|conn| {
        let mode_value = repository::get_setting(conn, "runtime_mode")?
            .unwrap_or_else(|| RuntimeMode::Standalone.as_str().to_string());
        let mode = RuntimeMode::parse(&mode_value)?;
        let host_address = repository::get_setting(conn, "host_address")?;
        let client_device_id = stable_client_device_id(conn)?;
        let host_port = repository::get_setting(conn, "host_port")?
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(17871);

        Ok(RuntimeConfig {
            mode,
            host_address,
            host_port,
            client_token: None,
            client_paired: false,
            client_device_id,
            data_dir: state.paths.data_dir.display().to_string(),
            database_path: state.paths.database_path.display().to_string(),
            backup_dir: effective_backup_dir_from_conn(conn, state),
            export_dir: effective_export_dir_from_conn(conn, state),
            import_report_dir: state.paths.import_report_dir.display().to_string(),
        })
    })?;
    if config.mode == RuntimeMode::Client {
        config.client_token = crate::application::secret_service::load(
            &state.db,
            crate::application::secret_service::ApplicationSecret::ClientToken,
        )?;
        config.client_paired = config
            .client_token
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
    }
    Ok(config)
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

include!("status_service/settings.rs");

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
        client_paired: false,
        client_device_id: String::new(),
        data_dir: String::new(),
        database_path: String::new(),
        backup_dir: String::new(),
        export_dir: String::new(),
        import_report_dir: String::new(),
    }
}

include!("status_service/settings_helpers.rs");

#[cfg(test)]
#[path = "status_service/tests.rs"]
mod tests;
