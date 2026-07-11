use super::*;

#[test]
fn submit_inbound_and_outbound_updates_balance_and_movements() {
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
        "INSERT INTO stock_balances (id, item_id) VALUES ('balance-1', 'item-1')",
        [],
    )
    .unwrap();

    submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "inbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-30".to_string(),
            department_id: None,
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: None,
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-1".to_string(),
                quantity: 10.0,
                unit_price: 12.0,
                amount: None,
                remark: None,
            }],
        },
        false,
    )
    .unwrap();

    submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "outbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-30".to_string(),
            department_id: Some("dept-admin-office".to_string()),
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: Some("领用测试".to_string()),
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-1".to_string(),
                quantity: 4.0,
                unit_price: 12.0,
                amount: None,
                remark: None,
            }],
        },
        false,
    )
    .unwrap();

    let balance: (f64, f64, f64) = conn
        .query_row(
            "SELECT quantity, amount, average_price FROM stock_balances WHERE item_id = 'item-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(balance, (6.0, 72.0, 12.0));

    let movement_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM stock_movements", [], |row| row.get(0))
        .unwrap();
    assert_eq!(movement_count, 2);

    let outbound_department: String = conn
        .query_row(
            "SELECT department_id FROM stock_movements WHERE direction = 'out'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(outbound_department, "dept-admin-office");

    let document_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stock_documents WHERE status = 'confirmed'",
            params![],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(document_count, 2);
}

#[test]
fn submit_stock_document_uses_manual_line_amount_when_provided() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();

    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-manual-amount', 'AMT-001', '手工金额物品', 'unit-piece', 10)",
        [],
    )
    .unwrap();

    let inbound = submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "inbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-30".to_string(),
            department_id: None,
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: None,
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-manual-amount".to_string(),
                quantity: 3.0,
                unit_price: 10.0,
                amount: Some(35.0),
                remark: None,
            }],
        },
        false,
    )
    .unwrap();
    assert_eq!(inbound.document.total_amount, 35.0);
    assert_eq!(inbound.lines[0].amount, 35.0);

    let outbound = submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "outbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-30".to_string(),
            department_id: Some("dept-admin-office".to_string()),
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: Some("领用测试".to_string()),
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-manual-amount".to_string(),
                quantity: 1.0,
                unit_price: 10.0,
                amount: Some(11.0),
                remark: None,
            }],
        },
        false,
    )
    .unwrap();
    assert_eq!(outbound.document.total_amount, 11.67);
    assert_eq!(outbound.lines[0].amount, 11.67);

    let balance: (f64, f64) = conn
        .query_row(
            "SELECT quantity, amount FROM stock_balances WHERE item_id = 'item-manual-amount'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(balance, (2.0, 23.33));

    let movement_amounts: Vec<f64> = {
        let mut stmt = conn
            .prepare(
                "SELECT amount FROM stock_movements
                     WHERE item_id = 'item-manual-amount'
                     ORDER BY direction ASC",
            )
            .unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<f64>, _>>()
            .unwrap()
    };
    assert_eq!(movement_amounts, vec![35.0, 11.67]);
}

#[test]
fn stock_documents_and_movements_keep_party_name_snapshots_after_rename() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO departments (id, code, name)
             VALUES ('dept-snapshot', 'SNAP', '旧部门名称')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO suppliers (id, name)
             VALUES ('supplier-snapshot', '旧供应商名称')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-snapshot', 'SNP-001', '快照物品', 'unit-piece', 10)",
        [],
    )
    .unwrap();

    let inbound = submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "inbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-30".to_string(),
            department_id: None,
            supplier_id: Some("supplier-snapshot".to_string()),
            handler: Some("tester".to_string()),
            purpose: None,
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-snapshot".to_string(),
                quantity: 2.0,
                unit_price: 10.0,
                amount: None,
                remark: None,
            }],
        },
        false,
    )
    .unwrap();
    let outbound = submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "outbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-30".to_string(),
            department_id: Some("dept-snapshot".to_string()),
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: Some("领用".to_string()),
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-snapshot".to_string(),
                quantity: 1.0,
                unit_price: 10.0,
                amount: None,
                remark: None,
            }],
        },
        false,
    )
    .unwrap();

    conn.execute(
        "UPDATE departments SET name = '新部门名称' WHERE id = 'dept-snapshot'",
        [],
    )
    .unwrap();
    conn.execute(
        "UPDATE suppliers SET name = '新供应商名称' WHERE id = 'supplier-snapshot'",
        [],
    )
    .unwrap();

    let inbound_detail = get_stock_document_detail(&conn, &inbound.document.id).unwrap();
    let outbound_detail = get_stock_document_detail(&conn, &outbound.document.id).unwrap();
    assert_eq!(
        inbound_detail.document.supplier_name.as_deref(),
        Some("旧供应商名称")
    );
    assert_eq!(
        outbound_detail.document.department_name.as_deref(),
        Some("旧部门名称")
    );

    let movements = list_stock_movements(&conn, StockMovementQuery::default()).unwrap();
    assert!(movements
        .iter()
        .any(|movement| movement.supplier_name.as_deref() == Some("旧供应商名称")));
    assert!(movements
        .iter()
        .any(|movement| movement.department_name.as_deref() == Some("旧部门名称")));
}

#[test]
fn submit_stock_document_rejects_disabled_department_and_supplier() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO departments (id, code, name, enabled)
             VALUES ('dept-disabled', 'DIS', '停用部门', 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO suppliers (id, name, enabled)
             VALUES ('supplier-disabled', '停用供应商', 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-disabled-party', 'DSP-001', '停用对象测试物品', 'unit-piece', 10)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO stock_balances (id, item_id, quantity, amount, average_price)
             VALUES ('balance-disabled-party', 'item-disabled-party', 10, 100, 10)",
        [],
    )
    .unwrap();

    let inbound_error = submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "inbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-30".to_string(),
            department_id: None,
            supplier_id: Some("supplier-disabled".to_string()),
            handler: Some("tester".to_string()),
            purpose: None,
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-disabled-party".to_string(),
                quantity: 1.0,
                unit_price: 10.0,
                amount: None,
                remark: None,
            }],
        },
        false,
    )
    .unwrap_err();
    assert!(inbound_error.to_string().contains("供应商已停用"));

    let outbound_error = submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "outbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-30".to_string(),
            department_id: Some("dept-disabled".to_string()),
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: Some("领用".to_string()),
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-disabled-party".to_string(),
                quantity: 1.0,
                unit_price: 10.0,
                amount: None,
                remark: None,
            }],
        },
        false,
    )
    .unwrap_err();
    assert!(outbound_error.to_string().contains("领用部门已停用"));
}
