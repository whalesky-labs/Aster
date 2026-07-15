use rusqlite::Connection;

use super::*;
use crate::db::migrations;

#[test]
fn confirm_stocktake_writes_gain_loss_movements_and_updates_balance() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-1', 'IT-001', '测试物品', 'unit-piece', 10)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO stock_balances (id, item_id, quantity, amount, average_price)
             VALUES ('balance-1', 'item-1', 10, 100, 10)",
        [],
    )
    .unwrap();

    let created = create_stocktake(
        &mut conn,
        CreateStocktakeRequest {
            business_date: "2026-06-30".to_string(),
            scope_type: "all".to_string(),
            category_id: None,
            item_ids: vec![],
            handler: Some("tester".to_string()),
            remark: None,
        },
    )
    .unwrap();
    let line_id = created.lines[0].id.clone();
    update_stocktake_counts(
        &conn,
        UpdateStocktakeCountsRequest {
            stocktake_id: created.document.id.clone(),
            lines: vec![crate::domain::stocktake::UpdateStocktakeLineRequest {
                line_id,
                counted_quantity: Some(12.0),
                remark: Some("盘盈测试".to_string()),
            }],
        },
    )
    .unwrap();
    confirm_stocktake(
        &mut conn,
        ConfirmStocktakeRequest {
            stocktake_id: created.document.id,
            handler: Some("tester".to_string()),
            remark: Some("确认盘点".to_string()),
        },
    )
    .unwrap();

    let balance: (f64, f64) = conn
        .query_row(
            "SELECT quantity, amount FROM stock_balances WHERE item_id = 'item-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(balance, (12.0, 120.0));

    let movement: (String, String, f64) = conn
        .query_row(
            "SELECT direction, movement_type, quantity FROM stock_movements",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        movement,
        ("in".to_string(), "stocktake_gain".to_string(), 2.0)
    );
}

#[test]
fn create_stocktake_rejects_disabled_category_and_items() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO categories (id, name, enabled)
             VALUES ('cat-stocktake-disabled', '停用盘点分类', 0)",
        [],
    )
    .unwrap();
    conn.execute(
            "INSERT INTO master_items (id, code, name, category_id, unit_id, default_price, enabled)
             VALUES ('item-stocktake-disabled', 'STD-001', '停用盘点物品', 'cat-stocktake-disabled', 'unit-piece', 10, 0)",
            [],
        )
        .unwrap();

    let category_error = create_stocktake(
        &mut conn,
        CreateStocktakeRequest {
            business_date: "2026-06-30".to_string(),
            scope_type: "category".to_string(),
            category_id: Some("cat-stocktake-disabled".to_string()),
            item_ids: vec![],
            handler: Some("tester".to_string()),
            remark: None,
        },
    )
    .unwrap_err();
    assert!(category_error.to_string().contains("盘点分类已停用"));

    let item_error = create_stocktake(
        &mut conn,
        CreateStocktakeRequest {
            business_date: "2026-06-30".to_string(),
            scope_type: "custom".to_string(),
            category_id: None,
            item_ids: vec!["item-stocktake-disabled".to_string()],
            handler: Some("tester".to_string()),
            remark: None,
        },
    )
    .unwrap_err();
    assert!(item_error.to_string().contains("盘点物品已停用"));
}

#[test]
fn update_stocktake_counts_rejects_unknown_or_foreign_line() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-count-1', 'CNT-001', '盘点物品一', 'unit-piece', 10),
                    ('item-count-2', 'CNT-002', '盘点物品二', 'unit-piece', 20)",
        [],
    )
    .unwrap();

    let first = create_stocktake(
        &mut conn,
        CreateStocktakeRequest {
            business_date: "2026-06-30".to_string(),
            scope_type: "custom".to_string(),
            category_id: None,
            item_ids: vec!["item-count-1".to_string()],
            handler: Some("tester".to_string()),
            remark: None,
        },
    )
    .unwrap();
    let second = create_stocktake(
        &mut conn,
        CreateStocktakeRequest {
            business_date: "2026-06-30".to_string(),
            scope_type: "custom".to_string(),
            category_id: None,
            item_ids: vec!["item-count-2".to_string()],
            handler: Some("tester".to_string()),
            remark: None,
        },
    )
    .unwrap();

    let foreign_error = update_stocktake_counts(
        &conn,
        UpdateStocktakeCountsRequest {
            stocktake_id: first.document.id.clone(),
            lines: vec![crate::domain::stocktake::UpdateStocktakeLineRequest {
                line_id: second.lines[0].id.clone(),
                counted_quantity: Some(1.0),
                remark: None,
            }],
        },
    )
    .unwrap_err();
    assert!(foreign_error.to_string().contains("不属于当前盘点单"));

    let missing_error = update_stocktake_counts(
        &conn,
        UpdateStocktakeCountsRequest {
            stocktake_id: first.document.id,
            lines: vec![crate::domain::stocktake::UpdateStocktakeLineRequest {
                line_id: "missing-line".to_string(),
                counted_quantity: Some(1.0),
                remark: None,
            }],
        },
    )
    .unwrap_err();
    assert!(missing_error.to_string().contains("盘点明细不存在"));
}
