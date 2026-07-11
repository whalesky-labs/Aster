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
    let user = crate::domain::users::CurrentUser {
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
    };
    let state = AppState {
        db: Db::initialize(&paths).unwrap(),
        paths,
        session: std::sync::Arc::new(std::sync::Mutex::new(None)),
        host_service: std::sync::Arc::new(std::sync::Mutex::new(
            crate::services::host_service::HostServiceRuntime::default(),
        )),
    };
    crate::services::test_support::install_session(&state, user).unwrap();
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
