use super::*;
#[test]
fn outbound_consumes_fifo_batches_and_records_actual_costs() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO suppliers (id, name) VALUES ('supplier-batch', '批次供应商')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-batch', 'BAT-001', '批次物品', 'unit-piece', 1)",
        [],
    )
    .unwrap();

    submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "inbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-01".to_string(),
            department_id: None,
            supplier_id: Some("supplier-batch".to_string()),
            handler: Some("tester".to_string()),
            purpose: None,
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-batch".to_string(),
                quantity: 100.0,
                unit_price: 1.2,
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
            document_type: "inbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-02".to_string(),
            department_id: None,
            supplier_id: Some("supplier-batch".to_string()),
            handler: Some("tester".to_string()),
            purpose: None,
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-batch".to_string(),
                quantity: 50.0,
                unit_price: 1.5,
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
            outbound_kind: Some("internal".to_string()),
            business_date: "2026-06-03".to_string(),
            department_id: Some("dept-admin-office".to_string()),
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: Some("跨批次领用".to_string()),
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-batch".to_string(),
                quantity: 120.0,
                unit_price: 9.99,
                amount: None,
                remark: None,
            }],
        },
        false,
    )
    .unwrap();

    assert_eq!(outbound.document.total_amount, 150.0);
    assert_eq!(outbound.lines[0].unit_price, 1.25);
    assert_eq!(outbound.lines[0].amount, 150.0);
    assert_eq!(outbound.batch_lines.len(), 2);
    assert_eq!(outbound.batch_lines[0].quantity, 100.0);
    assert_eq!(outbound.batch_lines[0].unit_price, 1.2);
    assert_eq!(outbound.batch_lines[1].quantity, 20.0);
    assert_eq!(outbound.batch_lines[1].unit_price, 1.5);

    let balance: (f64, f64, f64) = conn
        .query_row(
            "SELECT quantity, amount, average_price
                 FROM stock_balances
                 WHERE item_id = 'item-batch'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(balance, (30.0, 45.0, 1.5));

    let batch_remaining: Vec<(f64, f64, String)> = {
        let mut stmt = conn
            .prepare(
                "SELECT remaining_quantity, remaining_amount, status
                     FROM stock_batches
                     WHERE item_id = 'item-batch'
                     ORDER BY inbound_date ASC",
            )
            .unwrap();
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    };
    assert_eq!(
        batch_remaining,
        vec![
            (0.0, 0.0, "depleted".to_string()),
            (30.0, 45.0, "available".to_string())
        ]
    );

    let batches = list_stock_batches(&conn, "item-batch").unwrap();
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].batch_no, "IN-20260601-0001-B001");
    assert_eq!(batches[0].status, "depleted");
    assert_eq!(batches[1].remaining_quantity, 30.0);
    assert_eq!(batches[1].remaining_amount, 45.0);
    assert_eq!(
        batches[1].source_document_no.as_deref(),
        Some("IN-20260602-0001")
    );
}

#[test]
fn guest_sale_records_sale_revenue_separately_from_fifo_cost() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price, sale_price)
             VALUES ('item-sale-cost', 'SALE-COST', '客销成本物品', 'unit-piece', 12, 8)",
        [],
    )
    .unwrap();

    submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "inbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-01".to_string(),
            department_id: None,
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: None,
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-sale-cost".to_string(),
                quantity: 10.0,
                unit_price: 12.0,
                amount: None,
                remark: None,
            }],
        },
        false,
    )
    .unwrap();

    let sale = submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "outbound".to_string(),
            outbound_kind: Some("guest_sale".to_string()),
            business_date: "2026-06-02".to_string(),
            department_id: None,
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: Some("客人购买".to_string()),
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-sale-cost".to_string(),
                quantity: 2.0,
                unit_price: 8.0,
                amount: None,
                remark: None,
            }],
        },
        false,
    )
    .unwrap();

    assert_eq!(sale.document.total_sale_amount, 16.0);
    assert_eq!(sale.document.total_cost_amount, 24.0);
    assert_eq!(sale.document.total_gross_profit, -8.0);
    assert_eq!(sale.lines[0].sale_amount, Some(16.0));
    assert_eq!(sale.lines[0].cost_amount, Some(24.0));
    assert_eq!(sale.lines[0].gross_profit, Some(-8.0));

    let movement_amount: f64 = conn
        .query_row(
            "SELECT amount FROM stock_movements
                 WHERE item_id = 'item-sale-cost'
                   AND direction = 'out'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(movement_amount, 24.0);
}

#[test]
fn void_outbound_restores_fifo_batch_quantities() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-void-batch', 'VB-001', '批次作废物品', 'unit-piece', 10)",
        [],
    )
    .unwrap();
    submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "inbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-01".to_string(),
            department_id: None,
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: None,
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-void-batch".to_string(),
                quantity: 10.0,
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
            business_date: "2026-06-02".to_string(),
            department_id: Some("dept-admin-office".to_string()),
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: Some("领用后作废".to_string()),
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-void-batch".to_string(),
                quantity: 4.0,
                unit_price: 10.0,
                amount: None,
                remark: None,
            }],
        },
        false,
    )
    .unwrap();

    void_stock_document(
        &mut conn,
        VoidStockDocumentRequest {
            document_id: outbound.document.id,
            reason: "出库错误".to_string(),
            handler: Some("tester".to_string()),
        },
    )
    .unwrap();

    let batch: (f64, f64, String) = conn
        .query_row(
            "SELECT remaining_quantity, remaining_amount, status
                 FROM stock_batches
                 WHERE item_id = 'item-void-batch'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(batch, (10.0, 100.0, "available".to_string()));
    let balance: (f64, f64) = conn
        .query_row(
            "SELECT quantity, amount
                 FROM stock_balances
                 WHERE item_id = 'item-void-batch'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(balance, (10.0, 100.0));
}

#[test]
fn void_inbound_after_batch_was_consumed_is_rejected() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-consumed-inbound', 'CIN-001', '已消耗入库', 'unit-piece', 10)",
        [],
    )
    .unwrap();
    let inbound = submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "inbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-01".to_string(),
            department_id: None,
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: None,
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-consumed-inbound".to_string(),
                quantity: 10.0,
                unit_price: 10.0,
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
            business_date: "2026-06-02".to_string(),
            department_id: Some("dept-admin-office".to_string()),
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: Some("消耗入库批次".to_string()),
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-consumed-inbound".to_string(),
                quantity: 1.0,
                unit_price: 10.0,
                amount: None,
                remark: None,
            }],
        },
        false,
    )
    .unwrap();

    let error = void_stock_document(
        &mut conn,
        VoidStockDocumentRequest {
            document_id: inbound.document.id,
            reason: "采购错误".to_string(),
            handler: Some("tester".to_string()),
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("入库批次已被后续出库消耗"));
}

#[test]
fn void_confirmed_stocktake_writes_reversal_and_marks_stocktake_voided() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-stocktake-void', 'STV-001', '盘点作废物品', 'unit-piece', 10)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO stock_balances (id, item_id, quantity, amount, average_price)
             VALUES ('balance-stocktake-void', 'item-stocktake-void', 10, 100, 10)",
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
    update_stocktake_counts(
        &conn,
        UpdateStocktakeCountsRequest {
            stocktake_id: created.document.id.clone(),
            lines: vec![UpdateStocktakeLineRequest {
                line_id: created.lines[0].id.clone(),
                counted_quantity: Some(12.0),
                remark: Some("盘盈".to_string()),
            }],
        },
    )
    .unwrap();
    let confirmed = confirm_stocktake(
        &mut conn,
        ConfirmStocktakeRequest {
            stocktake_id: created.document.id.clone(),
            handler: Some("tester".to_string()),
            remark: Some("确认盘点".to_string()),
        },
    )
    .unwrap();

    let confirmed_balance: f64 = conn
        .query_row(
            "SELECT quantity FROM stock_balances WHERE item_id = 'item-stocktake-void'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(confirmed_balance, 12.0);

    void_stock_document(
        &mut conn,
        VoidStockDocumentRequest {
            document_id: confirmed.document.document_id,
            reason: "盘点录入错误".to_string(),
            handler: Some("tester".to_string()),
        },
    )
    .unwrap();

    let restored_balance: f64 = conn
        .query_row(
            "SELECT quantity FROM stock_balances WHERE item_id = 'item-stocktake-void'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(restored_balance, 10.0);

    let stocktake_status: String = conn
        .query_row(
            "SELECT status FROM stocktake_documents WHERE id = ?1",
            [created.document.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stocktake_status, "voided");

    let reversal_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE movement_type = 'reversal'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(reversal_count, 1);
}
