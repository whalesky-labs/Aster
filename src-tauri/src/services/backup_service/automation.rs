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
        backup_repository::insert_successful_backup(
            conn,
            backup_repository::SuccessfulBackupRecord {
                id: &backup_id,
                backup_file: &backup_path.display().to_string(),
                backup_type,
                app_version: APP_VERSION,
                schema_version,
                host_name: &source_host_name,
                os: &source_os,
                database_size,
                sha256: &database_sha256,
            },
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
