use std::sync::{Arc, Mutex};

use zip::ZipArchive;

use super::*;
use crate::domain::reports::{
    CategoryConsumptionRow, DepartmentIssueDetailRow, DepartmentIssueSummaryRow, InboundDetailRow,
    ItemConsumptionRow, MonthlyInventoryRow, SalesProfitRow, StockBalanceReportRow,
    StockWarningRow, StocktakeDifferenceReportRow,
};
use crate::domain::users::{CurrentUser, Role};
use crate::{app::paths::AppPaths, db::connection::Db, db::repository};

fn report_test_state() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().expect("temp dir");
    let paths = AppPaths {
        data_dir: dir.path().to_path_buf(),
        database_path: dir.path().join("aster.sqlite"),
        backup_dir: dir.path().join("backups"),
        export_dir: dir.path().join("exports"),
        import_report_dir: dir.path().join("import-reports"),
    };
    std::fs::create_dir_all(&paths.backup_dir).unwrap();
    std::fs::create_dir_all(&paths.export_dir).unwrap();
    std::fs::create_dir_all(&paths.import_report_dir).unwrap();
    let user = CurrentUser {
        id: "user-report".to_string(),
        username: "report".to_string(),
        display_name: "报表用户".to_string(),
        department_id: None,
        department_name: None,
        roles: vec![Role {
            id: "role-report".to_string(),
            code: "admin".to_string(),
            name: "管理员".to_string(),
        }],
        permissions: vec!["view_reports".to_string()],
    };
    let state = AppState {
        db: Db::initialize(&paths).unwrap(),
        paths,
        session: Arc::new(Mutex::new(None)),
        host_service: Arc::new(Mutex::new(
            crate::services::host_service::HostServiceRuntime::default(),
        )),
    };
    crate::services::test_support::install_session(&state, user).unwrap();
    (dir, state)
}

#[test]
fn write_report_workbook_creates_openable_xlsx_package() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("report.xlsx");
    let bundle = ReportBundle {
        month: "2026-06".to_string(),
        monthly_inventory: vec![MonthlyInventoryRow {
            item_id: "item-1".to_string(),
            item_code: "IT-001".to_string(),
            item_name: "测试物品".to_string(),
            spec: Some("标准".to_string()),
            unit_name: Some("件".to_string()),
            inbound_quantity: 10.0,
            inbound_amount: 120.0,
            outbound_quantity: 4.0,
            outbound_amount: 48.0,
            ending_quantity: 6.0,
            ending_amount: 72.0,
        }],
        department_summary: vec![DepartmentIssueSummaryRow {
            department_id: "dept-admin-office".to_string(),
            department_name: "行政办".to_string(),
            quantity: 4.0,
            amount: 48.0,
        }],
        department_details: vec![DepartmentIssueDetailRow {
            movement_date: "2026-06-30".to_string(),
            department_name: "行政办".to_string(),
            outbound_kind: Some("internal".to_string()),
            item_code: "IT-001".to_string(),
            item_name: "测试物品".to_string(),
            spec: Some("标准".to_string()),
            unit_name: Some("件".to_string()),
            quantity: 4.0,
            unit_price: 12.0,
            amount: 48.0,
            sale_unit_price: None,
            sale_amount: None,
            cost_unit_price: 12.0,
            cost_amount: 48.0,
            gross_profit: None,
            gross_margin: None,
            document_no: Some("OUT-20260630-0001".to_string()),
            purpose: Some("测试".to_string()),
            remark: None,
        }],
        category_consumption: vec![CategoryConsumptionRow {
            category_id: Some("cat-1".to_string()),
            category_name: "客耗".to_string(),
            quantity: 4.0,
            amount: 48.0,
        }],
        item_consumption_ranking: vec![ItemConsumptionRow {
            item_id: "item-1".to_string(),
            item_code: "IT-001".to_string(),
            item_name: "测试物品".to_string(),
            spec: Some("标准".to_string()),
            unit_name: Some("件".to_string()),
            quantity: 4.0,
            amount: 48.0,
        }],
        inbound_details: vec![InboundDetailRow {
            movement_date: "2026-06-01".to_string(),
            supplier_name: "测试供应商".to_string(),
            item_code: "IT-001".to_string(),
            item_name: "测试物品".to_string(),
            spec: Some("标准".to_string()),
            unit_name: Some("件".to_string()),
            quantity: 10.0,
            unit_price: 12.0,
            amount: 120.0,
            document_no: Some("IN-20260601-0001".to_string()),
            remark: None,
        }],
        outbound_details: vec![DepartmentIssueDetailRow {
            movement_date: "2026-06-30".to_string(),
            department_name: "行政办".to_string(),
            outbound_kind: Some("internal".to_string()),
            item_code: "IT-001".to_string(),
            item_name: "测试物品".to_string(),
            spec: Some("标准".to_string()),
            unit_name: Some("件".to_string()),
            quantity: 4.0,
            unit_price: 12.0,
            amount: 48.0,
            sale_unit_price: None,
            sale_amount: None,
            cost_unit_price: 12.0,
            cost_amount: 48.0,
            gross_profit: None,
            gross_margin: None,
            document_no: Some("OUT-20260630-0001".to_string()),
            purpose: Some("测试".to_string()),
            remark: None,
        }],
        sales_profit: vec![SalesProfitRow {
            movement_date: "2026-06-30".to_string(),
            item_code: "IT-001".to_string(),
            item_name: "测试物品".to_string(),
            spec: Some("标准".to_string()),
            unit_name: Some("件".to_string()),
            quantity: 2.0,
            sale_unit_price: 20.0,
            sale_amount: 40.0,
            cost_unit_price: 12.0,
            cost_amount: 24.0,
            gross_profit: 16.0,
            gross_margin: Some(0.4),
            negative_profit: false,
            document_no: Some("OUT-20260630-0002".to_string()),
            purpose: Some("客销".to_string()),
            remark: None,
        }],
        stock_balances: vec![StockBalanceReportRow {
            item_id: "item-1".to_string(),
            item_code: "IT-001".to_string(),
            item_name: "测试物品".to_string(),
            spec: Some("标准".to_string()),
            unit_name: Some("件".to_string()),
            quantity: 6.0,
            amount: 72.0,
            average_price: 12.0,
            last_inbound_price: 12.0,
            warning_quantity: 8.0,
            stock_status: "low".to_string(),
        }],
        stock_warnings: vec![StockWarningRow {
            item_id: "item-1".to_string(),
            item_code: "IT-001".to_string(),
            item_name: "测试物品".to_string(),
            spec: Some("标准".to_string()),
            unit_name: Some("件".to_string()),
            quantity: 6.0,
            warning_quantity: 8.0,
            shortage_quantity: 2.0,
            amount: 72.0,
        }],
        stocktake_differences: vec![StocktakeDifferenceReportRow {
            business_date: "2026-06-30".to_string(),
            document_no: "ST-20260630-0001".to_string(),
            scope_type: "all".to_string(),
            status: "confirmed".to_string(),
            item_code: "IT-001".to_string(),
            item_name: "测试物品".to_string(),
            spec: Some("标准".to_string()),
            unit_name: Some("件".to_string()),
            book_quantity: 6.0,
            counted_quantity: 5.0,
            difference_quantity: -1.0,
            average_price: 12.0,
            difference_amount: -12.0,
            remark: Some("测试差异".to_string()),
        }],
    };

    write_report_workbook(&bundle, &path).expect("write workbook");

    let file = std::fs::File::open(&path).expect("open workbook");
    let mut archive = ZipArchive::new(file).expect("xlsx zip");
    let mut workbook_xml = String::new();
    {
        use std::io::Read;
        let mut workbook_entry = archive
            .by_name("xl/workbook.xml")
            .expect("workbook xml exists");
        workbook_entry
            .read_to_string(&mut workbook_xml)
            .expect("read workbook xml");
    }
    assert!(workbook_xml.contains("库存余额"));
    assert!(workbook_xml.contains("盘点差异"));
    archive
        .by_name("xl/worksheets/sheet1.xml")
        .expect("first sheet exists");
}

#[test]
fn export_monthly_report_uses_default_export_dir_setting() {
    let (dir, state) = report_test_state();
    let custom_export_dir = dir.path().join("custom-exports");
    state
        .db
        .with_conn(|conn| {
            repository::set_setting(
                conn,
                "default_export_dir",
                &custom_export_dir.display().to_string(),
            )
        })
        .unwrap();

    let result = export_monthly_report(
        &state,
        ReportQuery {
            month: "2026-06".to_string(),
            start_date: None,
            end_date: None,
            department_id: None,
            category_id: None,
            item_id: None,
            supplier_id: None,
        },
    )
    .unwrap();

    assert!(std::path::Path::new(&result.path).starts_with(&custom_export_dir));
    assert!(std::path::Path::new(&result.path).exists());
}
