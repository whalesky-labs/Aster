use rusqlite::Connection;

use crate::db::migrations;

use super::*;

#[test]
fn save_category_supports_large_and_small_categories_only() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();

    let parent = save_category(
        &conn,
        SaveCategoryRequest {
            id: Some("category-food".to_string()),
            expected_updated_at: None,
            parent_id: None,
            name: "食品".to_string(),
            enabled: true,
            sort_order: 1,
        },
    )
    .unwrap();
    let child = save_category(
        &conn,
        SaveCategoryRequest {
            id: Some("category-rice".to_string()),
            expected_updated_at: None,
            parent_id: Some(parent.id.clone()),
            name: "米面粮油".to_string(),
            enabled: true,
            sort_order: 2,
        },
    )
    .unwrap();

    assert_eq!(child.parent_id.as_deref(), Some("category-food"));
    let categories = list_categories(&conn).unwrap();
    assert_eq!(categories.len(), 2);
    assert_eq!(
        categories
            .iter()
            .find(|category| category.id == "category-rice")
            .and_then(|category| category.parent_id.as_deref()),
        Some("category-food")
    );

    let grandchild = save_category(
        &conn,
        SaveCategoryRequest {
            id: Some("category-rice-imported".to_string()),
            expected_updated_at: None,
            parent_id: Some(child.id),
            name: "进口米".to_string(),
            enabled: true,
            sort_order: 3,
        },
    );
    assert!(grandchild.is_err());
}

#[test]
fn list_supplier_purchase_records_filters_inbound_movements_by_supplier() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO suppliers (id, name) VALUES ('supplier-a', '供应商A')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-a', 'A-001', '采购物品', 'unit-piece', 10)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO stock_documents (
               id, document_no, document_type, business_date, supplier_id, status
             )
             VALUES (
               'doc-a', 'IN-20260630-0001', 'inbound', '2026-06-30', 'supplier-a', 'confirmed'
             )",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               document_id, supplier_id, movement_type
             )
             VALUES (
               'mov-a', '2026-06-30', 'item-a', 'in', 4, 10, 40,
               'doc-a', 'supplier-a', 'inbound'
             )",
        [],
    )
    .unwrap();

    let records = list_supplier_purchase_records(&conn, "supplier-a").unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].document_no.as_deref(), Some("IN-20260630-0001"));
    assert_eq!(records[0].item_name, "采购物品");
    assert_eq!(records[0].amount, 40.0);

    let empty = list_supplier_purchase_records(&conn, "supplier-missing").unwrap();
    assert!(empty.is_empty());
}

#[test]
fn list_items_supports_more_than_one_thousand_items() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();

    for index in 0..1005 {
        conn.execute(
            "INSERT INTO master_items (id, code, barcode, name, unit_id, default_price)
                 VALUES (?1, ?2, ?3, ?4, 'unit-piece', 1)",
            params![
                format!("item-bulk-{index:04}"),
                format!("BULK-{index:04}"),
                format!("690000{index:04}"),
                format!("批量物品 {index:04}")
            ],
        )
        .unwrap();
    }

    let items = list_items(&conn, None, None).unwrap();
    assert_eq!(items.len(), 1005);
    assert_eq!(items[0].code, "BULK-0000");
    assert_eq!(items[1004].code, "BULK-1004");

    let first = list_items_page(&conn, None, None, None).unwrap();
    assert_eq!(first.items.len(), 500);
    assert_eq!(first.items[499].code, "BULK-0499");
    let second = list_items_page(&conn, None, None, first.next_cursor.as_deref()).unwrap();
    assert_eq!(second.items.len(), 500);
    assert_eq!(second.items[0].code, "BULK-0500");
    let third = list_items_page(&conn, None, None, second.next_cursor.as_deref()).unwrap();
    assert_eq!(third.items.len(), 5);
    assert_eq!(third.items[4].code, "BULK-1004");
    assert!(third.next_cursor.is_none());

    let searched = list_items(&conn, Some("BULK-1004".to_string()), None).unwrap();
    assert_eq!(searched.len(), 1);
    assert_eq!(searched[0].barcode.as_deref(), Some("6900001004"));
}

#[test]
fn list_items_filters_by_supplier_without_affecting_unfiltered_results() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO suppliers (id, name) VALUES
           ('supplier-filter-a', '筛选供应商 A'),
           ('supplier-filter-b', '筛选供应商 B')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, supplier_id, unit_id, default_price) VALUES
           ('item-supplier-a', 'SUP-A', '供应商 A 物品', 'supplier-filter-a', 'unit-piece', 1),
           ('item-supplier-b', 'SUP-B', '供应商 B 物品', 'supplier-filter-b', 'unit-piece', 1),
           ('item-no-supplier', 'SUP-NONE', '无供应商物品', NULL, 'unit-piece', 1)",
        [],
    )
    .unwrap();

    let supplier_a = list_items(&conn, None, Some("supplier-filter-a".to_string())).unwrap();
    assert_eq!(supplier_a.len(), 1);
    assert_eq!(supplier_a[0].code, "SUP-A");

    let all = list_items_page(&conn, None, None, None).unwrap();
    let supplier_b =
        list_items_page(&conn, None, Some("supplier-filter-b".to_string()), None).unwrap();
    assert_eq!(all.items.len(), 3);
    assert_eq!(supplier_b.items.len(), 1);
    assert_eq!(supplier_b.items[0].code, "SUP-B");
}

#[test]
fn save_item_requires_enabled_category_unit_and_supplier_references() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO categories (id, name, enabled)
             VALUES ('cat-disabled', '停用分类', 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO units (id, name, enabled)
             VALUES ('unit-disabled', '停用单位', 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO suppliers (id, name, enabled)
             VALUES ('supplier-disabled', '停用供应商', 0)",
        [],
    )
    .unwrap();

    let category_error = save_item(
        &conn,
        SaveItemRequest {
            id: Some("item-disabled-category".to_string()),
            expected_updated_at: None,
            code: Some("REF-001".to_string()),
            barcode: None,
            name: "停用分类物品".to_string(),
            category_id: Some("cat-disabled".to_string()),
            spec: None,
            unit_id: Some("unit-piece".to_string()),
            default_price: 1.0,
            sale_price: 0.0,
            supplier_id: None,
            warning_quantity: 0.0,
            enabled: true,
            remark: None,
        },
    )
    .unwrap_err();
    assert!(category_error.to_string().contains("分类已停用"));

    let unit_error = save_item(
        &conn,
        SaveItemRequest {
            id: Some("item-disabled-unit".to_string()),
            expected_updated_at: None,
            code: Some("REF-002".to_string()),
            barcode: None,
            name: "停用单位物品".to_string(),
            category_id: None,
            spec: None,
            unit_id: Some("unit-disabled".to_string()),
            default_price: 1.0,
            sale_price: 0.0,
            supplier_id: None,
            warning_quantity: 0.0,
            enabled: true,
            remark: None,
        },
    )
    .unwrap_err();
    assert!(unit_error.to_string().contains("单位已停用"));

    let supplier_error = save_item(
        &conn,
        SaveItemRequest {
            id: Some("item-disabled-supplier".to_string()),
            expected_updated_at: None,
            code: Some("REF-003".to_string()),
            barcode: None,
            name: "停用供应商物品".to_string(),
            category_id: None,
            spec: None,
            unit_id: Some("unit-piece".to_string()),
            default_price: 1.0,
            sale_price: 0.0,
            supplier_id: Some("supplier-disabled".to_string()),
            warning_quantity: 0.0,
            enabled: true,
            remark: None,
        },
    )
    .unwrap_err();
    assert!(supplier_error.to_string().contains("默认供应商已停用"));
}

#[test]
fn save_item_generates_code_for_new_item_when_blank() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-existing', 'HC-0001', '已有物品', 'unit-piece', 1)",
        [],
    )
    .unwrap();

    let item = save_item(
        &conn,
        SaveItemRequest {
            id: None,
            expected_updated_at: None,
            code: None,
            barcode: None,
            name: "自动编码物品".to_string(),
            category_id: None,
            spec: None,
            unit_id: Some("unit-piece".to_string()),
            default_price: 1.0,
            sale_price: 0.0,
            supplier_id: None,
            warning_quantity: 0.0,
            enabled: true,
            remark: None,
        },
    )
    .unwrap();

    assert_eq!(item.code, "HC-0002");
}

#[test]
fn save_budget_rule_requires_enabled_department_and_category() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO departments (id, code, name, enabled)
             VALUES ('dept-budget-disabled', 'BDIS', '停用预算部门', 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO categories (id, name, enabled)
             VALUES ('cat-budget-disabled', '停用预算分类', 0)",
        [],
    )
    .unwrap();

    let department_error = save_budget_rule(
        &conn,
        SaveBudgetRuleRequest {
            id: Some("budget-disabled-dept".to_string()),
            expected_updated_at: None,
            department_id: "dept-budget-disabled".to_string(),
            category_id: Some("cat-budget-disabled".to_string()),
            period_month: "2026-06".to_string(),
            amount_limit: 100.0,
            enabled: true,
        },
    )
    .unwrap_err();
    assert!(department_error.to_string().contains("部门已停用"));

    let category_error = save_budget_rule(
        &conn,
        SaveBudgetRuleRequest {
            id: Some("budget-disabled-cat".to_string()),
            expected_updated_at: None,
            department_id: "dept-admin-office".to_string(),
            category_id: Some("cat-budget-disabled".to_string()),
            period_month: "2026-06".to_string(),
            amount_limit: 100.0,
            enabled: true,
        },
    )
    .unwrap_err();
    assert!(category_error.to_string().contains("分类已停用"));
}

#[test]
fn save_budget_rule_allows_department_month_total_without_category() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO categories (id, name, enabled) VALUES ('cat-budget-total', '预算分类', 1)",
        [],
    )
    .unwrap();
    conn.execute(
            "INSERT INTO master_items (id, code, name, category_id, unit_id, default_price)
             VALUES ('item-budget-total', 'BT-001', '预算物品', 'cat-budget-total', 'unit-piece', 10)",
            [],
        )
        .unwrap();
    conn.execute(
        "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               department_id, movement_type
             )
             VALUES (
               'mov-total-used', '2026-06-10', 'item-budget-total', 'out', 3, 10, 30,
               'dept-admin-office', 'outbound'
             )",
        [],
    )
    .unwrap();

    let rule = save_budget_rule(
        &conn,
        SaveBudgetRuleRequest {
            id: Some("budget-total".to_string()),
            expected_updated_at: None,
            department_id: "dept-admin-office".to_string(),
            category_id: None,
            period_month: "2026-06".to_string(),
            amount_limit: 100.0,
            enabled: true,
        },
    )
    .unwrap();

    assert_eq!(rule.category_id, None);
    assert_eq!(rule.category_name, "全部分类");
    assert_eq!(rule.used_amount, 30.0);
}

#[test]
fn set_enabled_requires_existing_master_data_record() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();

    let error = set_unit_enabled(&conn, "missing-unit", false, Some("any-version")).unwrap_err();

    assert!(error.to_string().contains("要编辑的记录不存在"));
}

#[test]
fn save_item_requires_matching_updated_at_for_existing_records() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();

    let item = save_item(
        &conn,
        SaveItemRequest {
            id: Some("item-lock".to_string()),
            expected_updated_at: None,
            code: Some("LOCK-001".to_string()),
            barcode: None,
            name: "乐观锁物品".to_string(),
            category_id: None,
            spec: None,
            unit_id: Some("unit-piece".to_string()),
            default_price: 1.0,
            sale_price: 0.0,
            supplier_id: None,
            warning_quantity: 0.0,
            enabled: true,
            remark: None,
        },
    )
    .unwrap();

    let missing_version = save_item(
        &conn,
        SaveItemRequest {
            id: Some(item.id.clone()),
            expected_updated_at: None,
            code: Some("LOCK-001".to_string()),
            barcode: None,
            name: "缺版本覆盖".to_string(),
            category_id: None,
            spec: None,
            unit_id: Some("unit-piece".to_string()),
            default_price: 1.0,
            sale_price: 0.0,
            supplier_id: None,
            warning_quantity: 0.0,
            enabled: true,
            remark: None,
        },
    )
    .unwrap_err();
    assert!(missing_version.to_string().contains("缺少版本信息"));

    let updated = save_item(
        &conn,
        SaveItemRequest {
            id: Some(item.id.clone()),
            expected_updated_at: Some(item.updated_at.clone()),
            code: Some("LOCK-001".to_string()),
            barcode: None,
            name: "已更新物品".to_string(),
            category_id: None,
            spec: None,
            unit_id: Some("unit-piece".to_string()),
            default_price: 2.0,
            sale_price: 0.0,
            supplier_id: None,
            warning_quantity: 0.0,
            enabled: true,
            remark: None,
        },
    )
    .unwrap();
    assert_eq!(updated.name, "已更新物品");

    let stale = save_item(
        &conn,
        SaveItemRequest {
            id: Some(item.id.clone()),
            expected_updated_at: Some(item.updated_at.clone()),
            code: Some("LOCK-001".to_string()),
            barcode: None,
            name: "旧页面覆盖".to_string(),
            category_id: None,
            spec: None,
            unit_id: Some("unit-piece".to_string()),
            default_price: 3.0,
            sale_price: 0.0,
            supplier_id: None,
            warning_quantity: 0.0,
            enabled: true,
            remark: None,
        },
    )
    .unwrap_err();
    assert!(stale.to_string().contains("已被其他客户端修改"));

    let missing_toggle_version = set_item_enabled(&conn, &updated.id, false, None).unwrap_err();
    assert!(missing_toggle_version.to_string().contains("缺少版本信息"));

    set_item_enabled(&conn, &updated.id, false, Some(&updated.updated_at)).unwrap();
    let toggled = get_item(&conn, &updated.id).unwrap();
    assert!(!toggled.enabled);

    let stale_toggle =
        set_item_enabled(&conn, &updated.id, true, Some(&updated.updated_at)).unwrap_err();
    assert!(stale_toggle.to_string().contains("已被其他客户端修改"));
}
