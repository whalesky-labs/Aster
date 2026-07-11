#[test]
fn write_import_report_creates_json_migration_report() {
    let dir = tempdir().unwrap();
    let result = ImportResult {
        job_id: "job-report".to_string(),
        source_file: "/tmp/source.xlsx".to_string(),
        imported_items: 1,
        matched_items: 2,
        document_count: 3,
        movement_count: 4,
        warning_count: 0,
        error_count: 0,
        report_path: None,
        source_copy_path: None,
    };
    let preview = ImportPreview {
        source_file: result.source_file.clone(),
        sheet_count: 3,
        row_count: 2,
        item_count: 3,
        new_item_count: 1,
        existing_item_count: 2,
        opening_quantity: 0.0,
        opening_amount: 0.0,
        inbound_quantity: 0.0,
        inbound_amount: 0.0,
        outbound_quantity: 0.0,
        outbound_amount: 0.0,
        document_count: 3,
        warnings: Vec::new(),
        errors: Vec::new(),
        items: Vec::new(),
        months: Vec::new(),
    };

    let report_path =
        write_import_report(dir.path(), &result, &preview, ImportMode::Full, None).unwrap();
    let report_text = fs::read_to_string(report_path).unwrap();
    assert!(report_text.contains("\"jobId\": \"job-report\""));
    assert!(report_text.contains("\"mode\": \"完整导入\""));
    assert!(report_text.contains("\"movementCount\": 4"));
}

#[test]
fn run_excel_import_requires_admin_before_parsing_workbook() {
    let dir = tempdir().unwrap();
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
        id: "user-warehouse".to_string(),
        username: "warehouse".to_string(),
        display_name: "仓库员".to_string(),
        department_id: None,
        department_name: None,
        roles: vec![crate::domain::users::Role {
            id: "role-warehouse".to_string(),
            code: "warehouse".to_string(),
            name: "仓库员".to_string(),
        }],
        permissions: vec!["write_stock".to_string(), "view_reports".to_string()],
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

    let error = run_excel_import(
        &state,
        RunImportRequest {
            path: "/path/that/does/not/exist.xlsx".to_string(),
            mode: None,
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains("需要管理员权限"));
}

#[test]
fn run_excel_import_rejects_preview_errors_without_backup_or_writes() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("invalid-import.xlsx");
    let mut workbook = Workbook::new();
    {
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(ITEM_SHEET).unwrap();
        worksheet.write_string(0, 0, "物品名称").unwrap();
        worksheet.write_string(0, 1, "单位").unwrap();
        worksheet.write_string(1, 0, "缺单位物品").unwrap();
    }
    {
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(INBOUND_SHEET).unwrap();
        for (index, label) in ["业务时间", "物品名称", "数量", "进货单价"]
            .iter()
            .enumerate()
        {
            worksheet.write_string(0, index as u16, *label).unwrap();
        }
    }
    {
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(OUTBOUND_SHEET).unwrap();
        for (index, label) in ["业务时间", "出库类型", "部门", "物品名称", "数量"]
            .iter()
            .enumerate()
        {
            worksheet.write_string(0, index as u16, *label).unwrap();
        }
    }
    workbook.save(&path).unwrap();

    let paths = test_paths(dir.path());
    let state = admin_state(paths);
    let error = run_excel_import(
        &state,
        RunImportRequest {
            path: path.display().to_string(),
            mode: None,
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains("导入预览存在"));
    state
        .db
        .with_conn(|conn| {
            let backup_count: i64 =
                conn.query_row("SELECT COUNT(*) FROM backup_jobs", [], |row| row.get(0))?;
            let item_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM master_items WHERE name = '缺单位物品'",
                [],
                |row| row.get(0),
            )?;
            assert_eq!(backup_count, 0);
            assert_eq!(item_count, 0);
            Ok(())
        })
        .unwrap();
}

#[test]
fn run_excel_import_creates_before_import_backup_before_writes() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("valid-import.xlsx");
    write_template_workbook(&path);
    let paths = test_paths(dir.path());
    let state = admin_state(paths);

    let result = run_excel_import(
        &state,
        RunImportRequest {
            path: path.display().to_string(),
            mode: Some("itemsOnly".to_string()),
        },
    )
    .unwrap();

    assert_eq!(result.imported_items, 1);
    assert!(result
        .report_path
        .as_deref()
        .is_some_and(|path| Path::new(path).exists()));
    let backup_file = state
        .db
        .with_conn(|conn| {
            let backup_type: String = conn.query_row(
                "SELECT backup_type FROM backup_jobs ORDER BY created_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )?;
            let backup_file: String = conn.query_row(
                "SELECT backup_file FROM backup_jobs WHERE backup_type = 'before_import'",
                [],
                |row| row.get(0),
            )?;
            let imported_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM master_items WHERE name = '一次性牙刷'",
                [],
                |row| row.get(0),
            )?;
            assert_eq!(backup_type, "before_import");
            assert_eq!(imported_count, 1);
            Ok(backup_file)
        })
        .unwrap();
    assert!(Path::new(&backup_file).exists());

    let backup_database = dir.path().join("before-import.sqlite");
    let backup_zip = fs::File::open(&backup_file).unwrap();
    let mut archive = ZipArchive::new(backup_zip).unwrap();
    let mut database_entry = archive.by_name("aster.sqlite").unwrap();
    let mut database_file = fs::File::create(&backup_database).unwrap();
    std::io::copy(&mut database_entry, &mut database_file).unwrap();
    drop(database_file);
    drop(database_entry);
    drop(archive);

    let backup_conn = Connection::open(&backup_database).unwrap();
    let item_count_in_backup: i64 = backup_conn
        .query_row(
            "SELECT COUNT(*) FROM master_items WHERE name = '一次性牙刷'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(item_count_in_backup, 0);
}

#[test]
fn preview_excel_import_rejects_client_mode_before_parsing_workbook() {
    let dir = tempdir().unwrap();
    let paths = test_paths(dir.path());
    let state = AppState {
        db: Db::initialize(&paths).unwrap(),
        paths,
        session: std::sync::Arc::new(std::sync::Mutex::new(None)),
        host_service: std::sync::Arc::new(std::sync::Mutex::new(
            crate::services::host_service::HostServiceRuntime::default(),
        )),
    };
    state
        .db
        .with_conn(|conn| repository::set_setting(conn, "runtime_mode", "client"))
        .unwrap();

    let error = preview_excel_import(
        &state,
        ImportPreviewRequest {
            path: "/path/that/does/not/exist.xlsx".to_string(),
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("客户端模式不能操作正式数据库"));
}

fn test_paths(root: &Path) -> AppPaths {
    let paths = AppPaths {
        data_dir: root.join("app-data"),
        database_path: root.join("app-data").join("aster.sqlite"),
        backup_dir: root.join("backups"),
        export_dir: root.join("exports"),
        import_report_dir: root.join("import-reports"),
    };
    fs::create_dir_all(&paths.data_dir).unwrap();
    fs::create_dir_all(&paths.backup_dir).unwrap();
    fs::create_dir_all(&paths.export_dir).unwrap();
    fs::create_dir_all(&paths.import_report_dir).unwrap();
    paths
}

fn admin_state(paths: AppPaths) -> AppState {
    let user = crate::domain::users::CurrentUser {
        id: "user-admin".to_string(),
        username: "admin".to_string(),
        display_name: "管理员".to_string(),
        department_id: None,
        department_name: None,
        roles: vec![crate::domain::users::Role {
            id: "role-admin".to_string(),
            code: "admin".to_string(),
            name: "管理员".to_string(),
        }],
        permissions: vec![
            "manage_users".to_string(),
            "manage_settings".to_string(),
            "write_stock".to_string(),
            "view_reports".to_string(),
            "dangerous_operations".to_string(),
        ],
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
    state
}
