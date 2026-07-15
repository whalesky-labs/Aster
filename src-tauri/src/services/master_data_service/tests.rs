use std::sync::{Arc, Mutex};

use crate::app::paths::AppPaths;
use crate::app::state::AppState;
use crate::db::connection::Db;
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

#[test]
fn list_items_requires_view_reports_permission() {
    let state = test_state();
    let error = list_items(&state, None, None).unwrap_err();

    assert!(error.to_string().contains("请先登录"));
}

#[test]
fn export_items_applies_supplier_filter() {
    use calamine::Reader;

    let state = test_state();
    crate::services::test_support::install_session(
        &state,
        CurrentUser {
            id: "user-item-export".to_string(),
            username: "item-exporter".to_string(),
            display_name: "物品导出员".to_string(),
            department_id: None,
            department_name: None,
            roles: vec![Role {
                id: "role-warehouse".to_string(),
                code: "warehouse".to_string(),
                name: "仓库员".to_string(),
            }],
            permissions: vec!["view_reports".to_string()],
        },
    )
    .unwrap();
    state
        .db
        .with_conn(|conn| {
            conn.execute(
                "INSERT INTO suppliers (id, name) VALUES
                   ('supplier-export-a', '导出供应商 A'),
                   ('supplier-export-b', '导出供应商 B')",
                [],
            )?;
            conn.execute(
                "INSERT INTO master_items (
                   id, code, name, supplier_id, unit_id, default_price
                 ) VALUES
                   ('item-export-a', 'ITEM-EXP-A', '导出物品 A', 'supplier-export-a', 'unit-piece', 1),
                   ('item-export-b', 'ITEM-EXP-B', '导出物品 B', 'supplier-export-b', 'unit-piece', 1)",
                [],
            )?;
            Ok(())
        })
        .unwrap();

    let result = export_items(&state, None, Some("supplier-export-a".to_string())).unwrap();

    let mut workbook = calamine::open_workbook_auto(result.path).unwrap();
    let range = workbook.worksheet_range("物品档案").unwrap();
    assert_eq!(range.height(), 2);
    assert_eq!(range.get((1, 0)).unwrap().to_string(), "ITEM-EXP-A");
}
