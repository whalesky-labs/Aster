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
         WHERE key NOT IN ('smtp_password', 'client_token')
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
