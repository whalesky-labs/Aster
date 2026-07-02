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

    state
        .db
        .replace_database(&state.paths, &extracted_database)?;
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

pub fn create_backup_of_type(state: &AppState, backup_type: &str) -> AppResult<BackupSummary> {
    validate_backup_type(backup_type)?;
    let backup_dir = crate::services::status_service::effective_backup_dir(state)?;
    fs::create_dir_all(&backup_dir)?;

    let schema_version = state.db.with_conn(repository::schema_version)?;
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_id = Uuid::new_v4().to_string();
    let backup_suffix = backup_id
        .split('-')
        .next()
        .unwrap_or(backup_id.as_str())
        .to_string();
    let file_name = format!("aster-backup-{timestamp}-{backup_suffix}.zip");
    let backup_path = backup_dir.join(file_name);
    let snapshot_path =
        backup_dir.join(format!("aster-snapshot-{timestamp}-{backup_suffix}.sqlite"));

    state.db.with_conn(|conn| {
        let escaped = snapshot_path.to_string_lossy().replace('\'', "''");
        conn.execute_batch(&format!("VACUUM INTO '{escaped}';"))?;
        Ok(())
    })?;

    let database_size = fs::metadata(&snapshot_path)?.len();
    let database_sha256 = sha256_file(&snapshot_path)?;
    let source_host_name = current_host_name();
    let source_os = std::env::consts::OS.to_string();
    let metadata = BackupMetadata {
        app_name: "Aster".to_string(),
        app_version: APP_VERSION.to_string(),
        schema_version,
        created_at: chrono::Local::now().to_rfc3339(),
        backup_type: backup_type.to_string(),
        database_file: DATABASE_ENTRY.to_string(),
        database_size,
        database_sha256: database_sha256.clone(),
        source_os: source_os.clone(),
        source_host_name: Some(source_host_name.clone()),
    };

    let app_settings = state.db.with_conn(read_app_settings)?;
    write_backup_zip(
        &backup_path,
        &snapshot_path,
        &metadata,
        &app_settings,
        &state.paths.import_report_dir,
    )?;
    let _ = fs::remove_file(&snapshot_path);

    let second_backup_file = copy_to_second_backup_dir(state, &backup_path)?;
    state.db.with_conn(|conn| {
        backup_repository::insert_backup_record(
            conn,
            &backup_id,
            &backup_path.display().to_string(),
            backup_type,
            APP_VERSION,
            schema_version,
            &source_host_name,
            &source_os,
            database_size,
            &database_sha256,
            "success",
            None,
        )
    })?;
    cleanup_auto_backups(state)?;

    Ok(BackupSummary {
        backup_file: backup_path.display().to_string(),
        backup_type: backup_type.to_string(),
        created_at: metadata.created_at,
        schema_version,
        source_host_name,
        source_os,
        database_size,
        database_sha256,
        second_backup_file: second_backup_file.map(|path| path.display().to_string()),
    })
}

fn cleanup_auto_backups(state: &AppState) -> AppResult<()> {
    let records = state
        .db
        .with_conn(backup_repository::list_auto_backup_records)?;
    let delete_records = auto_backup_records_to_delete(&records, Local::now().date_naive());
    for record in delete_records {
        let backup_path = Path::new(&record.backup_file);
        if backup_path.exists() {
            fs::remove_file(backup_path)?;
        }
        state.db.with_conn(|conn| {
            backup_repository::delete_backup_record(conn, &record.id)?;
            conn.execute(
                "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
                 VALUES (?1, 'cleanup_backup', 'backup', ?2, ?3, 'system')",
                params![
                    Uuid::new_v4().to_string(),
                    record.id,
                    format!("按保留策略清理自动备份：{}", record.backup_file)
                ],
            )?;
            Ok(())
        })?;
    }
    Ok(())
}

fn auto_backup_records_to_delete(records: &[BackupRecord], today: NaiveDate) -> Vec<BackupRecord> {
    let mut keep_ids = std::collections::HashSet::new();
    let mut daily_keep: std::collections::HashMap<NaiveDate, String> =
        std::collections::HashMap::new();
    let mut monthly_keep: std::collections::HashMap<(i32, u32), String> =
        std::collections::HashMap::new();

    for record in records {
        let Ok(created_at) = parse_backup_timestamp(&record.created_at) else {
            keep_ids.insert(record.id.clone());
            continue;
        };
        let created_date = created_at.date_naive();
        let age_days = today.signed_duration_since(created_date).num_days();
        if age_days <= 7 {
            keep_ids.insert(record.id.clone());
        } else if age_days <= 30 {
            daily_keep
                .entry(created_date)
                .or_insert_with(|| record.id.clone());
        } else if age_days <= 365 {
            monthly_keep
                .entry((created_date.year(), created_date.month()))
                .or_insert_with(|| record.id.clone());
        }
    }

    keep_ids.extend(daily_keep.into_values());
    keep_ids.extend(monthly_keep.into_values());

    records
        .iter()
        .filter(|record| !keep_ids.contains(&record.id))
        .cloned()
        .collect()
}

fn copy_to_second_backup_dir(state: &AppState, backup_path: &Path) -> AppResult<Option<PathBuf>> {
    let second_dir = state
        .db
        .with_conn(|conn| repository::get_setting(conn, "second_backup_dir"))?;
    let Some(second_dir) = second_dir.filter(|path| !path.trim().is_empty()) else {
        return Ok(None);
    };
    let target_dir = Path::new(&second_dir);
    fs::create_dir_all(target_dir)?;
    let Some(file_name) = backup_path.file_name() else {
        return Ok(None);
    };
    let target = target_dir.join(file_name);
    fs::copy(backup_path, &target)?;
    Ok(Some(target))
}

fn write_backup_zip(
    backup_path: &Path,
    database_snapshot: &Path,
    metadata: &BackupMetadata,
    app_settings: &[BackupSetting],
    import_report_dir: &Path,
) -> AppResult<()> {
    let file = File::create(backup_path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file(METADATA_ENTRY, options).map_err(zip_error)?;
    zip.write_all(
        serde_json::to_string_pretty(metadata)
            .map_err(|error| AppError::Validation(format!("备份元数据序列化失败：{error}")))?
            .as_bytes(),
    )?;

    zip.start_file(SETTINGS_ENTRY, options).map_err(zip_error)?;
    zip.write_all(
        serde_json::to_string_pretty(app_settings)
            .map_err(|error| AppError::Validation(format!("应用配置序列化失败：{error}")))?
            .as_bytes(),
    )?;

    zip.start_file(DATABASE_ENTRY, options).map_err(zip_error)?;
    let mut database = File::open(database_snapshot)?;
    std::io::copy(&mut database, &mut zip)?;

    zip.add_directory(IMPORT_REPORTS_DIR, options)
        .map_err(zip_error)?;
    add_import_reports_to_zip(&mut zip, import_report_dir, options)?;
    zip.finish().map_err(zip_error)?;
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupSetting {
    key: String,
    value: String,
    updated_at: String,
}

fn read_app_settings(conn: &rusqlite::Connection) -> AppResult<Vec<BackupSetting>> {
    let mut stmt = conn.prepare(
        "SELECT key, value, updated_at
         FROM app_settings
         ORDER BY key",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(BackupSetting {
            key: row.get(0)?,
            value: row.get(1)?,
            updated_at: row.get(2)?,
        })
    })?;
    let mut settings = Vec::new();
    for row in rows {
        settings.push(row?);
    }
    Ok(settings)
}

fn add_import_reports_to_zip(
    zip: &mut ZipWriter<File>,
    import_report_dir: &Path,
    options: SimpleFileOptions,
) -> AppResult<()> {
    if !import_report_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(import_report_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name() else {
            continue;
        };
        let zip_entry = format!("{}{}", IMPORT_REPORTS_DIR, file_name.to_string_lossy());
        zip.start_file(zip_entry, options).map_err(zip_error)?;
        let mut file = File::open(&path)?;
        std::io::copy(&mut file, zip)?;
    }
    Ok(())
}

fn validate_backup_archive(state: &AppState, backup_file: &Path) -> AppResult<BackupMetadata> {
    let file = File::open(backup_file)?;
    let mut archive = ZipArchive::new(file).map_err(zip_error)?;
    let metadata = read_backup_metadata_from_archive(&mut archive)?;

    if metadata.app_name != "Aster" {
        return Err(AppError::Validation(format!(
            "备份来源系统不匹配：{}",
            metadata.app_name
        )));
    }
    if metadata.database_file != DATABASE_ENTRY {
        return Err(AppError::Validation(format!(
            "备份数据库文件名不匹配：{}",
            metadata.database_file
        )));
    }

    let current_schema = state.db.with_conn(repository::schema_version)?;
    if metadata.schema_version > current_schema {
        return Err(AppError::Validation(format!(
            "备份 Schema v{} 高于当前程序支持的 Schema v{}，请先升级 Aster",
            metadata.schema_version, current_schema
        )));
    }

    archive.by_name(SETTINGS_ENTRY).map_err(zip_error)?;
    archive.by_name(IMPORT_REPORTS_DIR).map_err(zip_error)?;

    let mut database_file = archive.by_name(DATABASE_ENTRY).map_err(zip_error)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    let mut database_size = 0_u64;
    loop {
        let bytes = database_file.read(&mut buffer)?;
        if bytes == 0 {
            break;
        }
        database_size += bytes as u64;
        hasher.update(&buffer[..bytes]);
    }
    let database_sha256 = format!("{:x}", hasher.finalize());
    if database_size != metadata.database_size {
        return Err(AppError::Validation(format!(
            "备份数据库大小校验失败：metadata={} actual={}",
            metadata.database_size, database_size
        )));
    }
    if database_sha256 != metadata.database_sha256 {
        return Err(AppError::Validation(
            "备份数据库 SHA256 校验失败，文件可能已损坏".to_string(),
        ));
    }

    Ok(metadata)
}

fn restore_validation_token(backup_file: &Path, metadata: &BackupMetadata) -> AppResult<String> {
    let canonical_backup_file = backup_file.canonicalize()?;
    let mut hasher = Sha256::new();
    hasher.update(canonical_backup_file.to_string_lossy().as_bytes());
    hasher.update(b"\0");
    hasher.update(metadata.database_sha256.as_bytes());
    hasher.update(b"\0");
    hasher.update(metadata.database_size.to_string().as_bytes());
    hasher.update(b"\0");
    hasher.update(metadata.schema_version.to_string().as_bytes());
    hasher.update(b"\0");
    hasher.update(metadata.created_at.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

fn read_backup_metadata_from_archive(archive: &mut ZipArchive<File>) -> AppResult<BackupMetadata> {
    let mut metadata_file = archive.by_name(METADATA_ENTRY).map_err(zip_error)?;
    let mut text = String::new();
    metadata_file.read_to_string(&mut text)?;
    serde_json::from_str(&text)
        .map_err(|error| AppError::Validation(format!("备份元数据解析失败：{error}")))
}

fn extract_database_to_temp(backup_file: &Path) -> AppResult<PathBuf> {
    let file = File::open(backup_file)?;
    let mut archive = ZipArchive::new(file).map_err(zip_error)?;
    let mut database_file = archive.by_name(DATABASE_ENTRY).map_err(zip_error)?;
    let path = std::env::temp_dir().join(format!("aster-restore-{}.sqlite", Uuid::new_v4()));
    let mut output = File::create(&path)?;
    std::io::copy(&mut database_file, &mut output)?;
    Ok(path)
}

fn sha256_file(path: &Path) -> AppResult<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let bytes = file.read(&mut buffer)?;
        if bytes == 0 {
            break;
        }
        hasher.update(&buffer[..bytes]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn parse_backup_timestamp(value: &str) -> Result<DateTime<Local>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Local))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                .map(|value| Utc.from_utc_datetime(&value).with_timezone(&Local))
        })
}

fn validate_backup_type(backup_type: &str) -> AppResult<()> {
    match backup_type {
        "auto_startup" | "auto_interval" | "manual" | "before_import" | "before_restore"
        | "before_migration" => Ok(()),
        other => Err(AppError::Validation(format!("不支持的备份类型：{other}"))),
    }
}

fn current_host_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .map(|value| value.trim().to_string())
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown-host".to_string())
}

fn zip_error(error: zip::result::ZipError) -> AppError {
    AppError::Validation(format!("备份包处理失败：{error}"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::app::paths::AppPaths;
    use crate::db::connection::Db;
    use crate::db::repository;

    fn backup_record(id: &str, created_at: &str) -> BackupRecord {
        BackupRecord {
            id: id.to_string(),
            backup_file: format!("/tmp/{id}.zip"),
            backup_type: "auto_interval".to_string(),
            app_version: "0.1.0".to_string(),
            schema_version: 1,
            host_name: Some("test-host".to_string()),
            os: Some(std::env::consts::OS.to_string()),
            database_size: 1,
            sha256: Some(id.to_string()),
            status: "success".to_string(),
            error_message: None,
            created_at: created_at.to_string(),
        }
    }

    fn test_state(role_code: &str, permissions: Vec<String>) -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths {
            data_dir: dir.path().to_path_buf(),
            database_path: dir.path().join("aster.sqlite"),
            backup_dir: dir.path().join("backups"),
            export_dir: dir.path().join("exports"),
            import_report_dir: dir.path().join("import-reports"),
        };
        fs::create_dir_all(&paths.backup_dir).unwrap();
        fs::create_dir_all(&paths.export_dir).unwrap();
        fs::create_dir_all(&paths.import_report_dir).unwrap();
        let state = AppState {
            db: Db::initialize(&paths).unwrap(),
            paths,
            session: std::sync::Arc::new(std::sync::Mutex::new(Some(
                crate::domain::users::CurrentUser {
                    id: format!("user-{role_code}"),
                    username: role_code.to_string(),
                    display_name: role_code.to_string(),
                    department_id: None,
                    department_name: None,
                    roles: vec![crate::domain::users::Role {
                        id: format!("role-{role_code}"),
                        code: role_code.to_string(),
                        name: role_code.to_string(),
                    }],
                    permissions,
                },
            ))),
            host_service: std::sync::Arc::new(std::sync::Mutex::new(
                crate::services::host_service::HostServiceRuntime::default(),
            )),
        };
        (dir, state)
    }

    #[test]
    fn backup_zip_contains_required_archive_entries() {
        let (_dir, state) = test_state("admin", vec!["dangerous_operations".to_string()]);
        fs::write(
            state
                .paths
                .import_report_dir
                .join("aster-import-report-test.json"),
            "{\"ok\":true}",
        )
        .unwrap();
        state
            .db
            .with_conn(|conn| repository::set_setting(conn, "hotel_name", "Aster Hotel"))
            .unwrap();

        let summary = create_backup_of_type(&state, "manual").unwrap();
        let metadata = validate_backup_archive(&state, Path::new(&summary.backup_file)).unwrap();
        assert_eq!(metadata.app_name, "Aster");
        assert_eq!(metadata.database_file, DATABASE_ENTRY);
        assert!(metadata.database_size > 0);
        assert_eq!(metadata.database_sha256, summary.database_sha256);
        assert_eq!(metadata.source_os, std::env::consts::OS);
        assert!(metadata
            .source_host_name
            .as_deref()
            .is_some_and(|value| !value.is_empty()));

        let file = File::open(&summary.backup_file).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        archive.by_name(DATABASE_ENTRY).unwrap();
        archive.by_name(METADATA_ENTRY).unwrap();

        let mut settings = String::new();
        archive
            .by_name(SETTINGS_ENTRY)
            .unwrap()
            .read_to_string(&mut settings)
            .unwrap();
        assert!(settings.contains("hotel_name"));

        let mut report = String::new();
        archive
            .by_name("import-reports/aster-import-report-test.json")
            .unwrap()
            .read_to_string(&mut report)
            .unwrap();
        assert!(report.contains("\"ok\":true"));
    }

    #[test]
    fn preview_restore_backup_validates_schema_source_and_database_sha() {
        let (_dir, state) = test_state("admin", vec!["dangerous_operations".to_string()]);
        let summary = create_backup_of_type(&state, "manual").unwrap();

        let preview =
            preview_restore_backup(&state, summary.backup_file.clone()).expect("preview backup");

        assert!(preview.valid);
        assert!(preview.message.contains("SHA256"));
        assert_eq!(preview.metadata.app_name, "Aster");
        assert_eq!(preview.metadata.database_file, DATABASE_ENTRY);
        assert_eq!(preview.metadata.database_sha256, summary.database_sha256);
        assert!(!preview.validation_token.is_empty());
        assert_eq!(preview.metadata.source_os, std::env::consts::OS);
        assert!(preview
            .metadata
            .source_host_name
            .as_deref()
            .is_some_and(|value| !value.is_empty()));
    }

    #[test]
    fn created_backup_record_tracks_source_host_and_os() {
        let (_dir, state) = test_state("admin", vec!["dangerous_operations".to_string()]);

        let summary = create_backup_of_type(&state, "manual").unwrap();
        let records = list_backup_records(&state).unwrap();
        let record = records
            .iter()
            .find(|record| record.backup_file == summary.backup_file)
            .expect("created backup record");

        assert_eq!(record.os.as_deref(), Some(std::env::consts::OS));
        assert!(record
            .host_name
            .as_deref()
            .is_some_and(|value| !value.is_empty()));
        assert_eq!(
            record.sha256.as_deref(),
            Some(summary.database_sha256.as_str())
        );
    }

    #[test]
    fn create_backup_uses_default_backup_dir_setting() {
        let (dir, state) = test_state("admin", vec!["dangerous_operations".to_string()]);
        let custom_backup_dir = dir.path().join("custom-backups");
        state
            .db
            .with_conn(|conn| {
                repository::set_setting(
                    conn,
                    "default_backup_dir",
                    &custom_backup_dir.display().to_string(),
                )
            })
            .unwrap();

        let summary = create_backup_of_type(&state, "manual").unwrap();

        assert!(Path::new(&summary.backup_file).starts_with(&custom_backup_dir));
        assert!(Path::new(&summary.backup_file).exists());
    }

    #[test]
    fn rapid_backups_use_unique_file_names_and_records() {
        let (_dir, state) = test_state("admin", vec!["dangerous_operations".to_string()]);

        let first = create_backup_of_type(&state, "manual").unwrap();
        let second = create_backup_of_type(&state, "before_restore").unwrap();

        assert_ne!(first.backup_file, second.backup_file);
        assert!(Path::new(&first.backup_file).exists());
        assert!(Path::new(&second.backup_file).exists());
        assert!(Path::new(&first.backup_file)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("aster-backup-"));
        assert!(Path::new(&second.backup_file)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("aster-backup-"));

        let records = list_backup_records(&state).unwrap();
        assert!(records
            .iter()
            .any(|record| record.backup_file == first.backup_file));
        assert!(records
            .iter()
            .any(|record| record.backup_file == second.backup_file));
    }

    #[test]
    fn preview_restore_backup_rejects_tampered_database_sha() {
        let (_dir, state) = test_state("admin", vec!["dangerous_operations".to_string()]);
        let database_path = state.paths.backup_dir.join("tampered.sqlite");
        state
            .db
            .with_conn(|conn| {
                let escaped = database_path.to_string_lossy().replace('\'', "''");
                conn.execute_batch(&format!("VACUUM INTO '{escaped}';"))?;
                Ok(())
            })
            .unwrap();
        let metadata = BackupMetadata {
            app_name: "Aster".to_string(),
            app_version: APP_VERSION.to_string(),
            schema_version: state.db.with_conn(repository::schema_version).unwrap(),
            created_at: chrono::Local::now().to_rfc3339(),
            backup_type: "manual".to_string(),
            database_file: DATABASE_ENTRY.to_string(),
            database_size: fs::metadata(&database_path).unwrap().len(),
            database_sha256: "bad-sha256".to_string(),
            source_os: std::env::consts::OS.to_string(),
            source_host_name: Some("tampered-host".to_string()),
        };
        let backup_file = state.paths.backup_dir.join("tampered.zip");
        write_backup_zip(
            &backup_file,
            &database_path,
            &metadata,
            &[],
            &state.paths.import_report_dir,
        )
        .unwrap();

        let error = preview_restore_backup(&state, backup_file.display().to_string()).unwrap_err();

        assert!(error.to_string().contains("SHA256"));
    }

    #[test]
    fn preview_restore_backup_rejects_future_schema() {
        let (_dir, state) = test_state("admin", vec!["dangerous_operations".to_string()]);
        let database_path = state.paths.backup_dir.join("future-schema.sqlite");
        state
            .db
            .with_conn(|conn| {
                let escaped = database_path.to_string_lossy().replace('\'', "''");
                conn.execute_batch(&format!("VACUUM INTO '{escaped}';"))?;
                Ok(())
            })
            .unwrap();
        let metadata = BackupMetadata {
            app_name: "Aster".to_string(),
            app_version: APP_VERSION.to_string(),
            schema_version: 999,
            created_at: chrono::Local::now().to_rfc3339(),
            backup_type: "manual".to_string(),
            database_file: DATABASE_ENTRY.to_string(),
            database_size: fs::metadata(&database_path).unwrap().len(),
            database_sha256: sha256_file(&database_path).unwrap(),
            source_os: std::env::consts::OS.to_string(),
            source_host_name: Some("future-schema-host".to_string()),
        };
        let backup_file = state.paths.backup_dir.join("future-schema.zip");
        write_backup_zip(
            &backup_file,
            &database_path,
            &metadata,
            &[],
            &state.paths.import_report_dir,
        )
        .unwrap();

        let error = preview_restore_backup(&state, backup_file.display().to_string()).unwrap_err();

        assert!(error.to_string().contains("高于当前程序支持"));
    }

    #[test]
    fn create_backup_copies_to_second_backup_dir() {
        let (dir, state) = test_state("admin", vec!["dangerous_operations".to_string()]);
        let second_backup_dir = dir.path().join("second-backups");
        set_second_backup_dir(
            &state,
            SetSecondBackupDirRequest {
                path: second_backup_dir.display().to_string(),
            },
        )
        .unwrap();

        let summary = create_backup_of_type(&state, "manual").unwrap();
        let second_backup_file = summary.second_backup_file.expect("second backup copy path");

        assert!(Path::new(&summary.backup_file).exists());
        assert!(Path::new(&second_backup_file).exists());
        assert_eq!(
            sha256_file(Path::new(&summary.backup_file)).unwrap(),
            sha256_file(Path::new(&second_backup_file)).unwrap()
        );
    }

    #[test]
    fn create_backup_rejects_client_mode_even_for_admin() {
        let (_dir, state) = test_state("admin", vec!["dangerous_operations".to_string()]);
        state
            .db
            .with_conn(|conn| repository::set_setting(conn, "runtime_mode", "client"))
            .unwrap();

        let error = create_backup(&state, CreateBackupRequest { backup_type: None }).unwrap_err();
        assert!(error.to_string().contains("只能在单机模式或主机本机执行"));
    }

    #[test]
    fn backup_dangerous_operations_reject_client_mode() {
        let (dir, state) = test_state("admin", vec!["dangerous_operations".to_string()]);
        state
            .db
            .with_conn(|conn| repository::set_setting(conn, "runtime_mode", "client"))
            .unwrap();

        let second_backup_error = set_second_backup_dir(
            &state,
            SetSecondBackupDirRequest {
                path: dir.path().join("second-backups").display().to_string(),
            },
        )
        .unwrap_err();
        assert!(second_backup_error
            .to_string()
            .contains("客户端模式不能操作正式数据库"));

        let preview_error =
            preview_restore_backup(&state, dir.path().join("backup.zip").display().to_string())
                .unwrap_err();
        assert!(preview_error
            .to_string()
            .contains("客户端模式不能操作正式数据库"));

        let restore_error = restore_backup(
            &state,
            RestoreBackupRequest {
                backup_file: dir.path().join("backup.zip").display().to_string(),
                confirmation: RESTORE_CONFIRMATION.to_string(),
                validation_token: "token".to_string(),
            },
        )
        .unwrap_err();
        assert!(restore_error
            .to_string()
            .contains("客户端模式不能操作正式数据库"));
    }

    #[test]
    fn startup_backup_runs_once_per_day_when_enabled() {
        let (_dir, state) = test_state("admin", vec![]);

        run_startup_backup_if_needed(&state).unwrap();
        run_startup_backup_if_needed(&state).unwrap();

        state
            .db
            .with_conn(|conn| {
                let backup_count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM backup_jobs WHERE backup_type = 'auto_startup'",
                    [],
                    |row| row.get(0),
                )?;
                let backup_file: String = conn.query_row(
                    "SELECT backup_file FROM backup_jobs WHERE backup_type = 'auto_startup'",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(backup_count, 1);
                assert!(Path::new(&backup_file).exists());
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn interval_backup_runs_when_due_and_respects_settings() {
        let (_dir, state) = test_state("admin", vec![]);

        run_interval_backup_if_needed(&state).unwrap();
        run_interval_backup_if_needed(&state).unwrap();

        state
            .db
            .with_conn(|conn| {
                let backup_count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM backup_jobs WHERE backup_type = 'auto_interval'",
                    [],
                    |row| row.get(0),
                )?;
                let backup_file: String = conn.query_row(
                    "SELECT backup_file FROM backup_jobs WHERE backup_type = 'auto_interval'",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(backup_count, 1);
                assert!(Path::new(&backup_file).exists());

                conn.execute(
                    "DELETE FROM backup_jobs WHERE backup_type = 'auto_interval'",
                    [],
                )?;
                repository::set_setting(conn, "interval_backup_enabled", "false")?;
                Ok(())
            })
            .unwrap();

        run_interval_backup_if_needed(&state).unwrap();

        state
            .db
            .with_conn(|conn| {
                let backup_count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM backup_jobs WHERE backup_type = 'auto_interval'",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(backup_count, 0);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn parse_backup_timestamp_treats_sqlite_timestamp_as_utc() {
        let parsed = parse_backup_timestamp("2026-07-01 05:20:00").unwrap();
        let expected = Utc
            .with_ymd_and_hms(2026, 7, 1, 5, 20, 0)
            .unwrap()
            .with_timezone(&Local);

        assert_eq!(parsed.timestamp(), expected.timestamp());
        assert!(parse_backup_timestamp("2026-07-01T05:20:00Z").is_ok());
    }

    #[test]
    fn restore_backup_resets_machine_local_paths() {
        let (_source_dir, source_state) =
            test_state("admin", vec!["dangerous_operations".to_string()]);
        source_state
            .db
            .with_conn(|conn| {
                repository::set_setting(conn, "default_export_dir", "C:\\old-host\\exports")?;
                repository::set_setting(conn, "default_backup_dir", "C:\\old-host\\backups")?;
                repository::set_setting(conn, "second_backup_dir", "Z:\\aster-second-backup")
            })
            .unwrap();
        let snapshot_path = source_state
            .paths
            .backup_dir
            .join("old-host-settings.sqlite");
        source_state
            .db
            .with_conn(|conn| {
                let escaped = snapshot_path.to_string_lossy().replace('\'', "''");
                conn.execute_batch(&format!("VACUUM INTO '{escaped}';"))?;
                Ok(())
            })
            .unwrap();
        let metadata = BackupMetadata {
            app_name: "Aster".to_string(),
            app_version: APP_VERSION.to_string(),
            schema_version: source_state
                .db
                .with_conn(repository::schema_version)
                .unwrap(),
            created_at: chrono::Local::now().to_rfc3339(),
            backup_type: "manual".to_string(),
            database_file: DATABASE_ENTRY.to_string(),
            database_size: fs::metadata(&snapshot_path).unwrap().len(),
            database_sha256: sha256_file(&snapshot_path).unwrap(),
            source_os: "windows".to_string(),
            source_host_name: Some("old-windows-host".to_string()),
        };
        let backup_file = source_state.paths.backup_dir.join("old-host-settings.zip");
        write_backup_zip(
            &backup_file,
            &snapshot_path,
            &metadata,
            &[],
            &source_state.paths.import_report_dir,
        )
        .unwrap();

        let (_target_dir, target_state) =
            test_state("admin", vec!["dangerous_operations".to_string()]);
        let preview =
            preview_restore_backup(&target_state, backup_file.display().to_string()).unwrap();
        let result = restore_backup(
            &target_state,
            RestoreBackupRequest {
                backup_file: backup_file.display().to_string(),
                confirmation: RESTORE_CONFIRMATION.to_string(),
                validation_token: preview.validation_token,
            },
        )
        .unwrap();

        assert_eq!(result.integrity, "ok");
        target_state
            .db
            .with_conn(|conn| {
                assert_eq!(
                    repository::get_setting(conn, "default_export_dir")?.unwrap(),
                    target_state.paths.export_dir.display().to_string()
                );
                assert_eq!(
                    repository::get_setting(conn, "default_backup_dir")?.unwrap(),
                    target_state.paths.backup_dir.display().to_string()
                );
                assert_eq!(
                    repository::get_setting(conn, "second_backup_dir")?.unwrap(),
                    ""
                );
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn restore_backup_requires_matching_preview_validation_token() {
        let (dir, state) = test_state("admin", vec!["dangerous_operations".to_string()]);
        let first_backup = create_backup_of_type(&state, "manual").unwrap();
        let preview = preview_restore_backup(&state, first_backup.backup_file.clone()).unwrap();

        let missing_token_error = restore_backup(
            &state,
            RestoreBackupRequest {
                backup_file: first_backup.backup_file.clone(),
                confirmation: RESTORE_CONFIRMATION.to_string(),
                validation_token: String::new(),
            },
        )
        .unwrap_err();
        assert!(missing_token_error.to_string().contains("请重新校验"));

        let copied_backup = dir.path().join("copied-backup.zip");
        fs::copy(&first_backup.backup_file, &copied_backup).unwrap();
        let changed_path_error = restore_backup(
            &state,
            RestoreBackupRequest {
                backup_file: copied_backup.display().to_string(),
                confirmation: RESTORE_CONFIRMATION.to_string(),
                validation_token: preview.validation_token,
            },
        )
        .unwrap_err();

        assert!(changed_path_error.to_string().contains("请重新校验"));
    }

    #[test]
    fn restore_backup_rolls_back_when_restored_database_fails_integrity_check() {
        let (_dir, state) = test_state("admin", vec!["dangerous_operations".to_string()]);
        state
            .db
            .with_conn(|conn| {
                repository::set_setting(conn, "hotel_name", "恢复前酒店")?;
                conn.execute(
                    "CREATE TABLE broken_integrity_child (
                         id INTEGER PRIMARY KEY,
                         missing_parent_id INTEGER NOT NULL,
                         FOREIGN KEY(missing_parent_id) REFERENCES missing_parent(id)
                     )",
                    [],
                )?;
                Ok(())
            })
            .unwrap();

        let backup = create_backup_of_type(&state, "manual").unwrap();
        let broken_database = state.paths.backup_dir.join("broken-restore.sqlite");
        state
            .db
            .with_conn(|conn| {
                let escaped = broken_database.to_string_lossy().replace('\'', "''");
                conn.execute_batch(&format!("VACUUM INTO '{escaped}';"))?;
                Ok(())
            })
            .unwrap();
        {
            let conn = rusqlite::Connection::open(&broken_database).unwrap();
            conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
            conn.execute(
                "INSERT INTO broken_integrity_child (id, missing_parent_id) VALUES (1, 999)",
                [],
            )
            .unwrap();
        }
        let broken_sha = sha256_file(&broken_database).unwrap();
        let broken_metadata = BackupMetadata {
            app_name: "Aster".to_string(),
            app_version: APP_VERSION.to_string(),
            schema_version: 1,
            created_at: chrono::Local::now().to_rfc3339(),
            backup_type: "manual".to_string(),
            database_file: DATABASE_ENTRY.to_string(),
            database_size: fs::metadata(&broken_database).unwrap().len(),
            database_sha256: broken_sha,
            source_os: std::env::consts::OS.to_string(),
            source_host_name: Some("broken-host".to_string()),
        };
        let broken_backup = state.paths.backup_dir.join("broken-restore.zip");
        write_backup_zip(
            &broken_backup,
            &broken_database,
            &broken_metadata,
            &[],
            &state.paths.import_report_dir,
        )
        .unwrap();
        let preview = preview_restore_backup(&state, broken_backup.display().to_string()).unwrap();

        let error = restore_backup(
            &state,
            RestoreBackupRequest {
                backup_file: broken_backup.display().to_string(),
                confirmation: RESTORE_CONFIRMATION.to_string(),
                validation_token: preview.validation_token,
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("已自动回滚到恢复前保护备份"));
        state
            .db
            .with_conn(|conn| {
                assert_eq!(
                    repository::get_setting(conn, "hotel_name")?.unwrap(),
                    "恢复前酒店"
                );
                Ok(())
            })
            .unwrap();

        let _ = fs::remove_file(backup.backup_file);
        let _ = fs::remove_file(broken_database);
    }

    #[test]
    fn auto_backup_retention_keeps_recent_daily_and_monthly_records() {
        let records = vec![
            backup_record("recent-a", "2026-06-29T08:00:00+08:00"),
            backup_record("recent-b", "2026-06-23T08:00:00+08:00"),
            backup_record("daily-latest", "2026-06-10T18:00:00+08:00"),
            backup_record("daily-old", "2026-06-10T08:00:00+08:00"),
            backup_record("monthly-latest", "2026-03-28T18:00:00+08:00"),
            backup_record("monthly-old", "2026-03-02T08:00:00+08:00"),
            backup_record("expired", "2025-01-01T08:00:00+08:00"),
        ];

        let delete_records =
            auto_backup_records_to_delete(&records, NaiveDate::from_ymd_opt(2026, 6, 30).unwrap());
        let delete_ids = delete_records
            .into_iter()
            .map(|record| record.id)
            .collect::<Vec<_>>();

        assert_eq!(delete_ids, vec!["daily-old", "monthly-old", "expired"]);
    }
}
