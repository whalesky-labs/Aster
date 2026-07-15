use rusqlite::Connection;

use crate::db::migrations;

use super::*;

#[test]
fn get_report_bundle_filters_department_summary_and_details_by_department_scope() {
    let conn = Connection::open_in_memory().unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price, enabled)
             VALUES ('item-a', 'A', '测试物品', 'unit-piece', 1, 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO stock_balances (id, item_id, quantity, amount)
             VALUES ('balance-a', 'item-a', 10, 10)",
        [],
    )
    .unwrap();
    conn.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               department_id, movement_type
             )
             VALUES
               ('mov-admin', '2026-06-10', 'item-a', 'out', 2, 1, 2, 'dept-admin-office', 'outbound'),
               ('mov-food', '2026-06-11', 'item-a', 'out', 3, 1, 3, 'dept-restaurant', 'outbound')",
            [],
        )
        .unwrap();

    let bundle = get_report_bundle(
        &conn,
        &ReportQuery {
            month: "2026-06".to_string(),
            start_date: None,
            end_date: None,
            department_id: Some("dept-admin-office".to_string()),
            category_id: None,
            item_id: None,
            supplier_id: None,
        },
    )
    .unwrap();

    assert_eq!(bundle.department_summary.len(), 1);
    assert_eq!(
        bundle.department_summary[0].department_id,
        "dept-admin-office"
    );
    assert_eq!(bundle.department_summary[0].quantity, 2.0);
    assert_eq!(bundle.department_details.len(), 1);
    assert_eq!(bundle.department_details[0].department_name, "行政办");
    assert_eq!(bundle.department_details[0].quantity, 2.0);
    assert_eq!(bundle.stock_balances.len(), 1);
    assert_eq!(bundle.stock_balances[0].item_code, "A");
    assert_eq!(bundle.stock_balances[0].quantity, 10.0);
}

#[test]
fn item_consumption_ranking_exports_all_consumed_items() {
    let conn = Connection::open_in_memory().unwrap();
    migrations::run(&conn).unwrap();

    for index in 0..120 {
        conn.execute(
            "INSERT INTO master_items (id, code, name, unit_id, default_price, enabled)
                 VALUES (?1, ?2, ?3, 'unit-piece', 1, 1)",
            rusqlite::params![
                format!("item-rank-{index:03}"),
                format!("RANK-{index:03}"),
                format!("排行物品 {index:03}")
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_movements (
                   id, movement_date, item_id, direction, quantity, unit_price, amount,
                   department_id, movement_type
                 )
                 VALUES (?1, '2026-06-15', ?2, 'out', 1, 1, ?3, 'dept-admin-office', 'outbound')",
            rusqlite::params![
                format!("mov-rank-{index:03}"),
                format!("item-rank-{index:03}"),
                (index + 1) as f64
            ],
        )
        .unwrap();
    }

    let bundle = get_report_bundle(
        &conn,
        &ReportQuery {
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

    assert_eq!(bundle.item_consumption_ranking.len(), 120);
    assert_eq!(bundle.item_consumption_ranking[0].item_code, "RANK-119");
    assert_eq!(bundle.item_consumption_ranking[119].item_code, "RANK-000");
}

#[test]
fn report_details_use_movement_party_name_snapshots_after_rename() {
    let conn = Connection::open_in_memory().unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO departments (id, code, name)
             VALUES ('dept-report-snapshot', 'RS', '新部门名称')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO suppliers (id, name)
             VALUES ('supplier-report-snapshot', '新供应商名称')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price, enabled)
             VALUES ('item-report-snapshot', 'RPT-SNP', '报表快照物品', 'unit-piece', 10, 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               department_id, department_name, supplier_id, supplier_name, movement_type
             )
             VALUES
               ('mov-report-out', '2026-06-10', 'item-report-snapshot', 'out', 1, 10, 10,
                'dept-report-snapshot', '旧部门名称', NULL, NULL, 'outbound'),
               ('mov-report-in', '2026-06-11', 'item-report-snapshot', 'in', 2, 10, 20,
                NULL, NULL, 'supplier-report-snapshot', '旧供应商名称', 'inbound')",
        [],
    )
    .unwrap();

    let bundle = get_report_bundle(
        &conn,
        &ReportQuery {
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

    assert_eq!(bundle.department_details[0].department_name, "旧部门名称");
    assert_eq!(bundle.inbound_details[0].supplier_name, "旧供应商名称");
}

#[test]
fn get_report_bundle_filters_movement_reports_by_date_range() {
    let conn = Connection::open_in_memory().unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO categories (id, name, enabled) VALUES ('cat-date', '日期分类', 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, category_id, unit_id, default_price, enabled)
             VALUES ('item-date', 'DATE-001', '日期筛选物品', 'cat-date', 'unit-piece', 10, 1)",
        [],
    )
    .unwrap();
    conn.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               department_id, supplier_id, movement_type
             )
             VALUES
               ('mov-date-in-before', '2026-06-05', 'item-date', 'in', 5, 10, 50, NULL, NULL, 'inbound'),
               ('mov-date-in-inside', '2026-06-15', 'item-date', 'in', 7, 10, 70, NULL, NULL, 'inbound'),
               ('mov-date-out-before', '2026-06-06', 'item-date', 'out', 2, 10, 20, 'dept-admin-office', NULL, 'outbound'),
               ('mov-date-out-inside', '2026-06-16', 'item-date', 'out', 3, 10, 30, 'dept-admin-office', NULL, 'outbound'),
               ('mov-date-out-after', '2026-06-25', 'item-date', 'out', 4, 10, 40, 'dept-admin-office', NULL, 'outbound')",
            [],
        )
        .unwrap();

    let bundle = get_report_bundle(
        &conn,
        &ReportQuery {
            month: "2026-06".to_string(),
            start_date: Some("2026-06-10".to_string()),
            end_date: Some("2026-06-20".to_string()),
            department_id: None,
            category_id: None,
            item_id: None,
            supplier_id: None,
        },
    )
    .unwrap();

    assert_eq!(bundle.monthly_inventory.len(), 1);
    assert_eq!(bundle.monthly_inventory[0].inbound_quantity, 7.0);
    assert_eq!(bundle.monthly_inventory[0].outbound_quantity, 3.0);
    assert_eq!(bundle.department_details.len(), 1);
    assert_eq!(bundle.department_details[0].movement_date, "2026-06-16");
    assert_eq!(bundle.inbound_details.len(), 1);
    assert_eq!(bundle.inbound_details[0].movement_date, "2026-06-15");
    assert_eq!(bundle.department_summary[0].quantity, 3.0);
    assert_eq!(bundle.category_consumption[0].quantity, 3.0);
    assert_eq!(bundle.item_consumption_ranking[0].quantity, 3.0);
}

#[test]
fn get_report_bundle_filters_category_item_and_supplier() {
    let conn = Connection::open_in_memory().unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO categories (id, name, enabled) VALUES ('cat-room', '客房耗材', 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO suppliers (id, name, enabled) VALUES ('supplier-a', '供应商 A', 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO suppliers (id, name, enabled) VALUES ('supplier-b', '供应商 B', 1)",
        [],
    )
    .unwrap();
    conn.execute(
            "INSERT INTO master_items (id, code, name, category_id, unit_id, default_price, supplier_id, warning_quantity, enabled)
             VALUES
               ('item-a', 'A', '筛选物品 A', 'cat-room', 'unit-piece', 1, 'supplier-a', 5, 1),
               ('item-b', 'B', '筛选物品 B', NULL, 'unit-piece', 1, 'supplier-b', 5, 1)",
            [],
        )
        .unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO stock_balances (id, item_id, quantity, amount)
             VALUES
               ('balance-a', 'item-a', 4, 4),
               ('balance-b', 'item-b', 4, 4)",
        [],
    )
    .unwrap();
    conn.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               department_id, supplier_id, movement_type
             )
             VALUES
               ('mov-in-a', '2026-06-03', 'item-a', 'in', 8, 1, 8, NULL, 'supplier-a', 'inbound'),
               ('mov-in-b', '2026-06-04', 'item-b', 'in', 9, 1, 9, NULL, 'supplier-b', 'inbound'),
               ('mov-out-a', '2026-06-05', 'item-a', 'out', 2, 1, 2, 'dept-admin-office', NULL, 'outbound'),
               ('mov-out-b', '2026-06-06', 'item-b', 'out', 3, 1, 3, 'dept-restaurant', NULL, 'outbound')",
            [],
        )
        .unwrap();

    let category_bundle = get_report_bundle(
        &conn,
        &ReportQuery {
            month: "2026-06".to_string(),
            start_date: None,
            end_date: None,
            department_id: None,
            category_id: Some("cat-room".to_string()),
            item_id: None,
            supplier_id: None,
        },
    )
    .unwrap();
    assert_eq!(category_bundle.monthly_inventory.len(), 1);
    assert_eq!(category_bundle.monthly_inventory[0].item_code, "A");
    assert_eq!(category_bundle.item_consumption_ranking.len(), 1);
    assert_eq!(category_bundle.stock_balances.len(), 1);
    assert_eq!(category_bundle.stock_warnings.len(), 1);

    let item_bundle = get_report_bundle(
        &conn,
        &ReportQuery {
            month: "2026-06".to_string(),
            start_date: None,
            end_date: None,
            department_id: None,
            category_id: None,
            item_id: Some("item-b".to_string()),
            supplier_id: None,
        },
    )
    .unwrap();
    assert_eq!(item_bundle.department_details.len(), 1);
    assert_eq!(item_bundle.department_details[0].item_code, "B");
    assert_eq!(item_bundle.category_consumption[0].category_name, "未分类");

    let supplier_bundle = get_report_bundle(
        &conn,
        &ReportQuery {
            month: "2026-06".to_string(),
            start_date: None,
            end_date: None,
            department_id: None,
            category_id: None,
            item_id: None,
            supplier_id: Some("supplier-a".to_string()),
        },
    )
    .unwrap();
    assert_eq!(supplier_bundle.inbound_details.len(), 1);
    assert_eq!(supplier_bundle.inbound_details[0].supplier_name, "供应商 A");
    assert_eq!(supplier_bundle.monthly_inventory.len(), 1);
    assert_eq!(supplier_bundle.monthly_inventory[0].item_code, "A");
}

#[test]
fn stocktake_difference_report_filters_month_category_and_item() {
    let conn = Connection::open_in_memory().unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO categories (id, name, enabled) VALUES ('cat-diff', '差异分类', 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, category_id, unit_id, default_price, enabled)
             VALUES
               ('item-diff-a', 'DIFF-A', '差异物品 A', 'cat-diff', 'unit-piece', 10, 1),
               ('item-diff-b', 'DIFF-B', '差异物品 B', NULL, 'unit-piece', 20, 1),
               ('item-diff-zero', 'DIFF-ZERO', '零差异物品', 'cat-diff', 'unit-piece', 5, 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO stock_balances (id, item_id, quantity, amount, average_price)
             VALUES
               ('balance-diff-a', 'item-diff-a', 8, 80, 10),
               ('balance-diff-b', 'item-diff-b', 5, 100, 20),
               ('balance-diff-zero', 'item-diff-zero', 8, 40, 5)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO stock_documents (id, document_no, document_type, business_date, status)
             VALUES
               ('doc-diff-before', 'ST-20260605-0001', 'stocktake', '2026-06-05', 'confirmed'),
               ('doc-diff-june', 'ST-20260630-0001', 'stocktake', '2026-06-30', 'confirmed'),
               ('doc-diff-july', 'ST-20260701-0001', 'stocktake', '2026-07-01', 'confirmed'),
               ('doc-diff-draft', 'ST-20260629-0001', 'stocktake', '2026-06-29', 'draft')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO stocktake_documents (id, document_id, scope_type, status)
             VALUES
               ('stocktake-diff-before', 'doc-diff-before', 'all', 'confirmed'),
               ('stocktake-diff-june', 'doc-diff-june', 'all', 'confirmed'),
               ('stocktake-diff-july', 'doc-diff-july', 'all', 'confirmed'),
               ('stocktake-diff-draft', 'doc-diff-draft', 'all', 'counting')",
        [],
    )
    .unwrap();
    conn.execute(
            "INSERT INTO stocktake_lines (
               id, stocktake_id, item_id, book_quantity, counted_quantity, difference_quantity, remark
             )
             VALUES
               ('line-before', 'stocktake-diff-before', 'item-diff-a', 8, 7, -1, '范围外'),
               ('line-diff-a', 'stocktake-diff-june', 'item-diff-a', 8, 6, -2, '盘亏'),
               ('line-diff-b', 'stocktake-diff-june', 'item-diff-b', 5, 7, 2, '盘盈'),
               ('line-zero', 'stocktake-diff-june', 'item-diff-zero', 8, 8, 0, '无差异'),
               ('line-july', 'stocktake-diff-july', 'item-diff-a', 8, 9, 1, '下月'),
               ('line-draft', 'stocktake-diff-draft', 'item-diff-a', 8, 3, -5, '未确认')",
            [],
        )
        .unwrap();

    let bundle = get_report_bundle(
        &conn,
        &ReportQuery {
            month: "2026-06".to_string(),
            start_date: None,
            end_date: None,
            department_id: None,
            category_id: Some("cat-diff".to_string()),
            item_id: None,
            supplier_id: None,
        },
    )
    .unwrap();

    assert_eq!(bundle.stocktake_differences.len(), 2);
    let june_difference = bundle
        .stocktake_differences
        .iter()
        .find(|row| row.document_no == "ST-20260630-0001")
        .expect("june difference row");
    assert_eq!(june_difference.item_code, "DIFF-A");
    assert_eq!(june_difference.difference_quantity, -2.0);
    assert_eq!(june_difference.difference_amount, -20.0);

    let item_bundle = get_report_bundle(
        &conn,
        &ReportQuery {
            month: "2026-06".to_string(),
            start_date: None,
            end_date: None,
            department_id: None,
            category_id: None,
            item_id: Some("item-diff-b".to_string()),
            supplier_id: None,
        },
    )
    .unwrap();
    assert_eq!(item_bundle.stocktake_differences.len(), 1);
    assert_eq!(item_bundle.stocktake_differences[0].item_code, "DIFF-B");

    let date_bundle = get_report_bundle(
        &conn,
        &ReportQuery {
            month: "2026-06".to_string(),
            start_date: Some("2026-06-20".to_string()),
            end_date: Some("2026-06-30".to_string()),
            department_id: None,
            category_id: Some("cat-diff".to_string()),
            item_id: None,
            supplier_id: None,
        },
    )
    .unwrap();
    assert_eq!(date_bundle.stocktake_differences.len(), 1);
    assert_eq!(
        date_bundle.stocktake_differences[0].document_no,
        "ST-20260630-0001"
    );
}

#[test]
fn get_report_bundle_includes_guest_sale_profit_rows() {
    let conn = Connection::open_in_memory().unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price, sale_price, enabled)
             VALUES ('item-sale-report', 'SALE-RPT', '销售报表物品', 'unit-piece', 12, 8, 1)",
        [],
    )
    .unwrap();
    conn.execute(
            "INSERT INTO stock_documents (
               id, document_no, document_type, outbound_kind, business_date, status
             )
             VALUES (
               'doc-sale-report', 'OUT-20260610-0001', 'outbound', 'guest_sale', '2026-06-10', 'confirmed'
             )",
            [],
        )
        .unwrap();
    conn.execute(
        "INSERT INTO stock_document_lines (
               id, document_id, item_id, quantity, unit_price, amount,
               sale_unit_price, sale_amount, cost_unit_price, cost_amount
             )
             VALUES (
               'line-sale-report', 'doc-sale-report', 'item-sale-report', 2, 8, 16,
               8, 16, 12, 24
             )",
        [],
    )
    .unwrap();

    let bundle = get_report_bundle(
        &conn,
        &ReportQuery {
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

    assert_eq!(bundle.sales_profit.len(), 1);
    assert_eq!(bundle.sales_profit[0].sale_amount, 16.0);
    assert_eq!(bundle.sales_profit[0].cost_amount, 24.0);
    assert_eq!(bundle.sales_profit[0].gross_profit, -8.0);
    assert!(bundle.sales_profit[0].negative_profit);
}
