use std::sync::{Arc, Mutex};

use crate::app::paths::AppPaths;
use crate::app::state::AppState;
use crate::db::connection::Db;
use crate::domain::stock::{
    SubmitAdjustmentLine, SubmitAdjustmentRequest, SubmitStockDocumentLine,
    SubmitStockDocumentRequest,
};
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

fn sample_request() -> SubmitStockDocumentRequest {
    SubmitStockDocumentRequest {
        document_type: "inbound".to_string(),
        outbound_kind: None,
        business_date: "2026-06-30".to_string(),
        department_id: None,
        supplier_id: None,
        handler: None,
        purpose: None,
        remark: None,
        approval_request_id: None,
        lines: vec![SubmitStockDocumentLine {
            item_id: String::new(),
            quantity: 1.0,
            unit_price: 1.0,
            amount: None,
            remark: None,
        }],
    }
}

fn set_warehouse_user(state: &AppState) {
    let user = CurrentUser {
        id: "user-warehouse".to_string(),
        username: "warehouse".to_string(),
        display_name: "仓库员".to_string(),
        department_id: None,
        department_name: None,
        roles: vec![Role {
            id: "role-warehouse".to_string(),
            code: "warehouse".to_string(),
            name: "仓库员".to_string(),
        }],
        permissions: vec!["write_stock".to_string(), "view_reports".to_string()],
    };
    crate::services::test_support::install_session(state, user).unwrap();
}

fn set_admin_user(state: &AppState) {
    let user = CurrentUser {
        id: "user-admin-export".to_string(),
        username: "admin-export".to_string(),
        display_name: "导出管理员".to_string(),
        department_id: None,
        department_name: None,
        roles: vec![Role {
            id: "role-admin".to_string(),
            code: "admin".to_string(),
            name: "管理员".to_string(),
        }],
        permissions: vec!["view_reports".to_string()],
    };
    crate::services::test_support::install_session(state, user).unwrap();
}

fn set_department_viewer(state: &AppState, department_id: &str) {
    let user = CurrentUser {
        id: "user-department-viewer".to_string(),
        username: "dept-viewer".to_string(),
        display_name: "部门查看员".to_string(),
        department_id: Some(department_id.to_string()),
        department_name: Some("绑定部门".to_string()),
        roles: vec![Role {
            id: "role-department-viewer".to_string(),
            code: "department_viewer".to_string(),
            name: "部门查看员".to_string(),
        }],
        permissions: vec!["view_reports".to_string()],
    };
    crate::services::test_support::install_session(state, user).unwrap();
}

#[test]
fn submit_stock_document_requires_write_stock_permission() {
    let state = test_state();
    let error = submit_stock_document(&state, sample_request()).unwrap_err();
    assert!(error.to_string().contains("请先登录"));
}

#[test]
fn list_stock_balances_requires_view_reports_permission() {
    let state = test_state();
    let error = list_stock_balances(&state, StockBalanceQuery::default()).unwrap_err();
    assert!(error.to_string().contains("请先登录"));
}

#[test]
fn export_stock_balances_requires_admin_role() {
    let state = test_state();
    set_warehouse_user(&state);

    let error = export_stock_balances(&state).unwrap_err();

    assert!(error.to_string().contains("管理员权限"));
    assert!(std::fs::read_dir(&state.paths.export_dir)
        .unwrap()
        .next()
        .is_none());
}

#[test]
fn export_stock_balances_writes_complete_workbook_and_audit() {
    use calamine::Reader;

    let state = test_state();
    state
        .db
        .with_conn(|conn| {
            conn.execute(
                "INSERT INTO master_items (
                   id, code, name, unit_id, default_price, warning_quantity, enabled
                 ) VALUES
                   ('item-service-export-a', 'SVC-EXP-A', '导出正常物品', 'unit-piece', 2, 1, 1),
                   ('item-service-export-b', 'SVC-EXP-B', '导出停用物品', 'unit-piece', 3, 0, 0)",
                [],
            )?;
            conn.execute(
                "INSERT INTO stock_balances (
                   id, item_id, quantity, amount, average_price, last_inbound_price
                 ) VALUES
                   ('balance-service-export-a', 'item-service-export-a', 5, 10, 2, 2.5),
                   ('balance-service-export-b', 'item-service-export-b', -1, -3, 3, 3)",
                [],
            )?;
            Ok(())
        })
        .unwrap();
    set_admin_user(&state);

    let result = export_stock_balances(&state).unwrap();

    assert_eq!(result.row_count, 2);
    assert!(result.path.contains("Aster-库存台账-"));
    assert!(std::path::Path::new(&result.path).exists());
    let mut workbook = calamine::open_workbook_auto(&result.path).unwrap();
    let range = workbook.worksheet_range("库存台账").unwrap();
    assert_eq!(range.get((0, 0)).unwrap().to_string(), "物品编码");
    assert_eq!(range.get((0, 12)).unwrap().to_string(), "物品状态");
    assert_eq!(range.get((1, 0)).unwrap().to_string(), "SVC-EXP-A");
    assert_eq!(range.get((1, 6)).unwrap().to_string(), "5");
    assert_eq!(range.get((2, 11)).unwrap().to_string(), "负库存");
    assert_eq!(range.get((2, 12)).unwrap().to_string(), "停用");
    let audit: (i64, String) = state
        .db
        .with_conn(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*), MAX(operator) FROM audit_logs
                 WHERE action = 'export_stock_balances'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?)
        })
        .unwrap();
    assert_eq!(audit.0, 1);
    assert_eq!(audit.1, "admin-export");
}

#[test]
fn submit_stock_document_checks_business_validation_after_permission() {
    let state = test_state();
    set_warehouse_user(&state);
    let error = submit_stock_document(&state, sample_request()).unwrap_err();
    assert!(error.to_string().contains("单据行缺少物品"));
}

#[test]
fn normalize_business_datetime_accepts_datetime_local_and_rejects_future() {
    let mut value = "2026-06-30T14:25".to_string();
    normalize_business_datetime(&mut value, "业务日期").unwrap();
    assert_eq!(value, "2026-06-30 14:25:00");

    let mut future = (chrono::Local::now().naive_local() + chrono::Duration::minutes(1))
        .format("%Y-%m-%dT%H:%M")
        .to_string();
    let error = normalize_business_datetime(&mut future, "业务日期").unwrap_err();
    assert!(error.to_string().contains("不能晚于当前时间"));
}

#[test]
fn submit_stock_document_rejects_negative_manual_amount() {
    let state = test_state();
    set_warehouse_user(&state);
    let mut request = sample_request();
    request.lines[0].item_id = "item-1".to_string();
    request.lines[0].amount = Some(-1.0);

    let error = submit_stock_document(&state, request).unwrap_err();
    assert!(error.to_string().contains("金额不能小于 0"));
}

#[test]
fn validate_adjustment_enforces_type_direction_semantics() {
    let mut request = SubmitAdjustmentRequest {
        business_date: "2026-06-30".to_string(),
        adjustment_type: "gain".to_string(),
        handler: Some("tester".to_string()),
        reason: "盘盈".to_string(),
        lines: vec![SubmitAdjustmentLine {
            item_id: "item-1".to_string(),
            direction: "out".to_string(),
            quantity: 1.0,
            unit_price: 1.0,
            amount: None,
            remark: None,
        }],
    };

    let gain_error = validate_adjustment(&request).unwrap_err();
    assert!(gain_error.to_string().contains("盘盈调整只能增加库存"));

    request.adjustment_type = "loss".to_string();
    request.lines[0].direction = "in".to_string();
    let loss_error = validate_adjustment(&request).unwrap_err();
    assert!(loss_error.to_string().contains("盘亏调整只能减少库存"));

    request.adjustment_type = "damage".to_string();
    let damage_error = validate_adjustment(&request).unwrap_err();
    assert!(damage_error.to_string().contains("损耗调整只能减少库存"));

    request.adjustment_type = "correction".to_string();
    validate_adjustment(&request).expect("correction supports either direction");
}

#[test]
fn department_viewer_stock_lists_are_scoped_to_bound_department() {
    let state = test_state();
    state
            .db
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO master_items (id, code, name, unit_id, default_price)
                     VALUES ('item-scope-test', 'SCOPE-001', '范围测试物品', 'unit-piece', 1)",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO stock_documents (
                       id, document_no, document_type, business_date, department_id, department_name, status
                     )
                     VALUES
                       ('doc-admin-scope', 'OUT-SCOPE-ADMIN', 'outbound', '2026-06-30', 'dept-admin-office', '行政办', 'confirmed'),
                       ('doc-restaurant-scope', 'OUT-SCOPE-REST', 'outbound', '2026-06-30', 'dept-restaurant', '餐饮', 'confirmed')",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO stock_document_lines (id, document_id, item_id, quantity, unit_price, amount)
                     VALUES
                       ('line-admin-scope', 'doc-admin-scope', 'item-scope-test', 1, 1, 1),
                       ('line-restaurant-scope', 'doc-restaurant-scope', 'item-scope-test', 1, 1, 1)",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO stock_movements (
                       id, movement_date, item_id, direction, quantity, unit_price, amount,
                       document_id, department_id, department_name, movement_type
                     )
                     VALUES
                       ('mov-admin-scope', '2026-06-30', 'item-scope-test', 'out', 1, 1, 1, 'doc-admin-scope', 'dept-admin-office', '行政办', 'outbound'),
                       ('mov-restaurant-scope', '2026-06-30', 'item-scope-test', 'out', 1, 1, 1, 'doc-restaurant-scope', 'dept-restaurant', '餐饮', 'outbound')",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
    set_department_viewer(&state, "dept-admin-office");

    let docs = list_stock_documents(
        &state,
        StockDocumentQuery {
            department_id: Some("dept-restaurant".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    let movements = list_stock_movements(
        &state,
        StockMovementQuery {
            department_id: Some("dept-restaurant".to_string()),
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].department_id.as_deref(), Some("dept-admin-office"));
    assert_eq!(movements.len(), 1);
    assert_eq!(movements[0].department_name.as_deref(), Some("行政办"));
}
