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
    crate::application::secret_service::save(
        &target_state.db,
        crate::application::secret_service::ApplicationSecret::SmtpPassword,
        "target-smtp-secret",
    )
    .unwrap();
    crate::application::secret_service::save(
        &target_state.db,
        crate::application::secret_service::ApplicationSecret::ClientToken,
        "target-client-secret",
    )
    .unwrap();
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
    assert!(crate::application::secret_service::load(
        &target_state.db,
        crate::application::secret_service::ApplicationSecret::SmtpPassword,
    )
    .unwrap()
    .is_none());
    assert!(crate::application::secret_service::load(
        &target_state.db,
        crate::application::secret_service::ApplicationSecret::ClientToken,
    )
    .unwrap()
    .is_none());
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
