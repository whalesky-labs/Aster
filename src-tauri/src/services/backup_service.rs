use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone, Utc};
use rusqlite::params;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::app::state::AppState;
use crate::db::{backup_repository, repository};
use crate::domain::backups::{
    BackupMetadata, BackupRecord, BackupSummary, CreateBackupRequest, RestoreBackupRequest,
    RestorePreview, RestoreResult, SetSecondBackupDirRequest,
};
use crate::error::{AppError, AppResult};
use crate::services::user_service;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const DATABASE_ENTRY: &str = "aster.sqlite";
const METADATA_ENTRY: &str = "backup-meta.json";
const SETTINGS_ENTRY: &str = "app-settings.json";
const IMPORT_REPORTS_DIR: &str = "import-reports/";
const RESTORE_CONFIRMATION: &str = "RESTORE";
const DEFAULT_INTERVAL_BACKUP_HOURS: i64 = 6;

pub fn create_backup(state: &AppState, request: CreateBackupRequest) -> AppResult<BackupSummary> {
    crate::services::safety_service::require_dangerous_local_operation(state, "创建备份")?;
    let backup_type = request.backup_type.unwrap_or_else(|| "manual".to_string());
    create_backup_of_type(state, &backup_type)
}

pub fn list_backup_records(state: &AppState) -> AppResult<Vec<BackupRecord>> {
    user_service::require_admin(state)?;
    if crate::services::status_service::get_runtime_config(state)?.mode
        == crate::domain::runtime::RuntimeMode::Client
    {
        return crate::services::host_service::remote_list_backup_records(state);
    }
    state.db.with_conn(backup_repository::list_backup_records)
}

pub fn set_second_backup_dir(
    state: &AppState,
    request: SetSecondBackupDirRequest,
) -> AppResult<String> {
    crate::services::safety_service::require_dangerous_local_operation(state, "设置第二备份目录")?;
    let path = request.path.trim();
    if path.is_empty() {
        state
            .db
            .with_conn(|conn| repository::set_setting(conn, "second_backup_dir", ""))?;
        return Ok(String::new());
    }
    let dir = Path::new(path);
    fs::create_dir_all(dir)?;
    if !dir.is_dir() {
        return Err(AppError::Validation(
            "第二备份目录不是有效文件夹".to_string(),
        ));
    }
    let test_file = dir.join(".aster-write-test");
    fs::write(&test_file, "ok")?;
    fs::remove_file(test_file)?;
    state
        .db
        .with_conn(|conn| repository::set_setting(conn, "second_backup_dir", path))?;
    Ok(path.to_string())
}

pub fn preview_restore_backup(state: &AppState, backup_file: String) -> AppResult<RestorePreview> {
    crate::services::safety_service::require_dangerous_local_operation(state, "预览恢复备份")?;
    let backup_path = Path::new(&backup_file);
    let metadata = validate_backup_archive(state, backup_path)?;
    let validation_token = restore_validation_token(backup_path, &metadata)?;
    Ok(RestorePreview {
        backup_file,
        valid: true,
        message: "备份包结构、Schema、来源系统和 SHA256 校验通过".to_string(),
        metadata,
        validation_token,
    })
}

pub fn restore_backup(state: &AppState, request: RestoreBackupRequest) -> AppResult<RestoreResult> {
    crate::services::safety_service::require_dangerous_local_operation(state, "恢复备份")?;
    if request.confirmation.trim() != RESTORE_CONFIRMATION {
        return Err(AppError::Validation(
            "恢复前必须输入 RESTORE 作为确认文本".to_string(),
        ));
    }

    let backup_file = Path::new(&request.backup_file);
    let metadata = validate_backup_archive(state, backup_file)?;
    let expected_validation_token = restore_validation_token(backup_file, &metadata)?;
    if request.validation_token.trim().is_empty()
        || request.validation_token != expected_validation_token
    {
        return Err(AppError::Validation(
            "备份文件未校验或路径已变化，请重新校验后再恢复".to_string(),
        ));
    }
    let protected_backup = create_backup_of_type(state, "before_restore")?;
    let extracted_database = extract_database_to_temp(backup_file)?;

    let previous_smtp_password = crate::application::secret_service::load(
        &state.db,
        crate::application::secret_service::ApplicationSecret::SmtpPassword,
    )?;
    let previous_client_token = crate::application::secret_service::load(
        &state.db,
        crate::application::secret_service::ApplicationSecret::ClientToken,
    )?;
    if let Err(error) = clear_restored_secrets(state) {
        restore_previous_secrets(state, previous_smtp_password, previous_client_token)?;
        let _ = fs::remove_file(&extracted_database);
        return Err(error);
    }

    if let Err(error) = state.db.replace_database(&state.paths, &extracted_database) {
        restore_previous_secrets(state, previous_smtp_password, previous_client_token)?;
        let _ = fs::remove_file(&extracted_database);
        return Err(error);
    }
    let restore_check = state.db.with_conn(|conn| {
        reset_restored_local_paths(conn, state)?;
        verify_restored_database_health(conn)?;
        conn.execute(
            "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
             VALUES (?1, 'restore_backup', 'backup', ?2, ?3, 'system')",
            params![
                Uuid::new_v4().to_string(),
                request.backup_file,
                format!("恢复备份，恢复前保护备份：{}", protected_backup.backup_file)
            ],
        )?;
        Ok(())
    });
    if let Err(error) = restore_check {
        rollback_restore(state, &protected_backup.backup_file, &extracted_database)?;
        restore_previous_secrets(state, previous_smtp_password, previous_client_token)?;
        return Err(AppError::Validation(format!(
            "{error}；已自动回滚到恢复前保护备份：{}",
            protected_backup.backup_file
        )));
    }
    let _ = fs::remove_file(&extracted_database);

    Ok(RestoreResult {
        restored_from: request.backup_file,
        protected_backup_file: protected_backup.backup_file,
        schema_version: metadata.schema_version,
        integrity: "ok".to_string(),
    })
}

fn clear_restored_secrets(state: &AppState) -> AppResult<()> {
    crate::application::secret_service::delete(
        &state.db,
        crate::application::secret_service::ApplicationSecret::SmtpPassword,
    )?;
    crate::application::secret_service::delete(
        &state.db,
        crate::application::secret_service::ApplicationSecret::ClientToken,
    )
}

fn restore_previous_secrets(
    state: &AppState,
    smtp_password: Option<String>,
    client_token: Option<String>,
) -> AppResult<()> {
    if let Some(password) = smtp_password {
        crate::application::secret_service::save(
            &state.db,
            crate::application::secret_service::ApplicationSecret::SmtpPassword,
            &password,
        )?;
    }
    if let Some(token) = client_token {
        crate::application::secret_service::save(
            &state.db,
            crate::application::secret_service::ApplicationSecret::ClientToken,
            &token,
        )?;
    }
    Ok(())
}

include!("backup_service/restore.rs");

#[cfg(test)]
#[path = "backup_service/tests.rs"]
mod tests;
