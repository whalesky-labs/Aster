use super::*;
#[test]
fn list_stock_documents_filters_by_month_party_item_and_search() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO suppliers (id, name) VALUES ('supplier-filter', '筛选供应商')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES
               ('item-filter-a', 'FIL-A', '筛选物品A', 'unit-piece', 10),
               ('item-filter-b', 'FIL-B', '筛选物品B', 'unit-piece', 10)",
        [],
    )
    .unwrap();

    let inbound = submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "inbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-10".to_string(),
            department_id: None,
            supplier_id: Some("supplier-filter".to_string()),
            handler: Some("采购筛选".to_string()),
            purpose: None,
            remark: Some("六月采购".to_string()),
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-filter-a".to_string(),
                quantity: 5.0,
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
            document_type: "inbound".to_string(),
            outbound_kind: None,
            business_date: "2026-07-10".to_string(),
            department_id: None,
            supplier_id: None,
            handler: Some("七月采购".to_string()),
            purpose: None,
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-filter-b".to_string(),
                quantity: 3.0,
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
            business_date: "2026-06-11".to_string(),
            department_id: Some("dept-admin-office".to_string()),
            supplier_id: None,
            handler: Some("领用筛选".to_string()),
            purpose: Some("部门筛选".to_string()),
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-filter-a".to_string(),
                quantity: 1.0,
                unit_price: 10.0,
                amount: None,
                remark: None,
            }],
        },
        false,
    )
    .unwrap();

    let june_inbound = list_stock_documents(
        &conn,
        StockDocumentQuery {
            document_type: Some("inbound".to_string()),
            month: Some("2026-06".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(june_inbound.len(), 1);
    assert_eq!(june_inbound[0].id, inbound.document.id);
    assert_eq!(
        june_inbound[0].item_summary.as_deref(),
        Some("FIL-A · 筛选物品A")
    );

    let supplier_docs = list_stock_documents(
        &conn,
        StockDocumentQuery {
            supplier_id: Some("supplier-filter".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(supplier_docs.len(), 1);
    assert_eq!(
        supplier_docs[0].supplier_name.as_deref(),
        Some("筛选供应商")
    );

    let department_docs = list_stock_documents(
        &conn,
        StockDocumentQuery {
            department_id: Some("dept-admin-office".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(department_docs.len(), 1);
    assert_eq!(department_docs[0].document_type, "outbound");

    let item_docs = list_stock_documents(
        &conn,
        StockDocumentQuery {
            item_id: Some("item-filter-a".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(item_docs.len(), 2);

    let search_docs = list_stock_documents(
        &conn,
        StockDocumentQuery {
            search: Some("六月采购".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(search_docs.len(), 1);
    assert_eq!(search_docs[0].document_type, "inbound");

    let item_search_docs = list_stock_documents(
        &conn,
        StockDocumentQuery {
            search: Some("筛选物品A".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(item_search_docs.len(), 2);

    let handler_docs = list_stock_documents(
        &conn,
        StockDocumentQuery {
            handler: Some("采购筛选".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(handler_docs.len(), 1);
    assert_eq!(handler_docs[0].id, inbound.document.id);

    let detail = get_stock_document_detail(&conn, &inbound.document.id).unwrap();
    assert_eq!(detail.lines.len(), 1);
    assert_eq!(detail.lines[0].item_name, "筛选物品A");
    assert_eq!(detail.lines[0].quantity, 5.0);
}

#[test]
fn stock_balance_and_movement_lists_support_more_than_one_thousand_rows() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();

    for index in 0..1005 {
        conn.execute(
            "INSERT INTO master_items (id, code, name, unit_id, default_price)
                 VALUES (?1, ?2, ?3, 'unit-piece', 1)",
            params![
                format!("item-stock-{index:04}"),
                format!("STK-{index:04}"),
                format!("库存物品 {index:04}")
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_balances (id, item_id, quantity, amount, average_price)
                 VALUES (?1, ?2, 1, 1, 1)",
            params![
                format!("balance-stock-{index:04}"),
                format!("item-stock-{index:04}")
            ],
        )
        .unwrap();
        conn.execute(
                "INSERT INTO stock_movements (
                   id, movement_date, item_id, direction, quantity, unit_price, amount, movement_type, created_at
                 )
                 VALUES (?1, '2026-06-30', ?2, 'in', 1, 1, 1, 'opening', ?3)",
                params![
                    format!("movement-stock-{index:04}"),
                    format!("item-stock-{index:04}"),
                    format!("2026-06-30T10:{:02}:00+08:00", index % 60)
                ],
            )
            .unwrap();
    }

    let balances = list_stock_balances(&conn, StockBalanceQuery::default()).unwrap();
    assert_eq!(balances.len(), 1005);
    assert_eq!(balances[0].item_code, "STK-0000");
    assert_eq!(balances[1004].item_code, "STK-1004");

    let movements = list_stock_movements(&conn, StockMovementQuery::default()).unwrap();
    assert_eq!(movements.len(), 1005);
    assert!(movements.iter().any(|row| row.item_code == "STK-0000"));
    assert!(movements.iter().any(|row| row.item_code == "STK-1004"));
}

#[test]
fn stock_balance_and_movement_lists_support_structured_filters() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO categories (id, name, enabled) VALUES ('cat-stock-filter', '库存筛选分类', 1)",
        [],
    )
    .unwrap();
    conn.execute(
            "INSERT INTO master_items (id, code, name, category_id, unit_id, default_price, warning_quantity)
             VALUES
               ('item-normal', 'NORM', '正常库存', 'cat-stock-filter', 'unit-piece', 1, 3),
               ('item-low', 'LOW', '低库存', 'cat-stock-filter', 'unit-piece', 1, 5),
               ('item-zero-low', 'ZERO', '零库存预警', NULL, 'unit-piece', 1, 0),
               ('item-negative', 'NEG', '负库存', NULL, 'unit-piece', 1, 0)",
            [],
        )
        .unwrap();
    conn.execute(
        "INSERT INTO stock_balances (id, item_id, quantity, amount, average_price)
             VALUES
               ('balance-normal', 'item-normal', 10, 10, 1),
               ('balance-low', 'item-low', 4, 4, 1),
               ('balance-zero-low', 'item-zero-low', 0, 0, 0),
               ('balance-negative', 'item-negative', -1, -1, 1)",
        [],
    )
    .unwrap();
    conn.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount, movement_type, operator, remark, created_at
             )
             VALUES
               ('mov-normal-in', '2026-06-01', 'item-normal', 'in', 10, 1, 10, 'inbound', 'alice', '正常入库', '2026-06-01T10:00:00+08:00'),
               ('mov-low-out', '2026-06-02', 'item-low', 'out', 1, 1, 1, 'outbound', 'bob', '低库存领用', '2026-06-02T10:00:00+08:00'),
               ('mov-negative-out', '2026-06-03', 'item-negative', 'out', 1, 1, 1, 'reversal', 'carol', '冲正测试', '2026-06-03T10:00:00+08:00')",
            [],
        )
        .unwrap();

    let category_balances = list_stock_balances(
        &conn,
        StockBalanceQuery {
            category_id: Some("cat-stock-filter".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(category_balances.len(), 2);
    assert!(category_balances.iter().all(|row| row.item_code != "NEG"));

    let low_balances = list_stock_balances(
        &conn,
        StockBalanceQuery {
            stock_status: Some("low".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(low_balances.len(), 2);
    assert!(low_balances.iter().any(|row| row.item_code == "LOW"));
    assert!(low_balances.iter().any(|row| row.item_code == "ZERO"));
    assert!(low_balances.iter().all(|row| row.item_code != "NEG"));

    let item_movements = list_stock_movements(
        &conn,
        StockMovementQuery {
            item_id: Some("item-low".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(item_movements.len(), 1);
    assert_eq!(item_movements[0].item_code, "LOW");
    assert_eq!(item_movements[0].operator.as_deref(), Some("bob"));
    assert_eq!(item_movements[0].remark.as_deref(), Some("低库存领用"));

    let outbound_movements = list_stock_movements(
        &conn,
        StockMovementQuery {
            direction: Some("out".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(outbound_movements.len(), 2);
    assert!(outbound_movements.iter().all(|row| row.direction == "out"));

    let reversal_movements = list_stock_movements(
        &conn,
        StockMovementQuery {
            movement_type: Some("reversal".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(reversal_movements.len(), 1);
    assert_eq!(reversal_movements[0].item_code, "NEG");

    let operator_search = list_stock_movements(
        &conn,
        StockMovementQuery {
            search: Some("alice".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(operator_search.len(), 1);
    assert_eq!(operator_search[0].movement_type, "inbound");
}

#[test]
fn submit_outbound_rejects_when_budget_limit_would_be_exceeded() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO categories (id, name, enabled) VALUES ('cat-budget', '预算分类', 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, category_id, unit_id, default_price)
             VALUES ('item-budget', 'BUD-001', '预算物品', 'cat-budget', 'unit-piece', 10)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO stock_balances (id, item_id, quantity, amount, average_price)
             VALUES ('balance-budget', 'item-budget', 20, 200, 10)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO budget_rules (
               id, department_id, category_id, period_month, amount_limit, enabled
             )
             VALUES ('budget-1', 'dept-admin-office', 'cat-budget', '2026-06', 100, 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               department_id, movement_type
             )
             VALUES (
               'mov-used', '2026-06-10', 'item-budget', 'out', 6, 10, 60,
               'dept-admin-office', 'outbound'
             )",
        [],
    )
    .unwrap();

    let error = submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "outbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-30".to_string(),
            department_id: Some("dept-admin-office".to_string()),
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: Some("预算测试".to_string()),
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-budget".to_string(),
                quantity: 5.0,
                unit_price: 10.0,
                amount: None,
                remark: None,
            }],
        },
        false,
    )
    .unwrap_err();

    assert!(error.to_string().contains("超出预算"));
    let document_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stock_documents WHERE document_type = 'outbound'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(document_count, 0);
}

#[test]
fn submit_outbound_rejects_when_department_budget_total_would_be_exceeded() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO categories (id, name, enabled) VALUES ('cat-dept-budget', '部门预算分类', 1)",
        [],
    )
    .unwrap();
    conn.execute(
            "INSERT INTO master_items (id, code, name, category_id, unit_id, default_price)
             VALUES ('item-dept-budget', 'DBUD-001', '部门预算物品', 'cat-dept-budget', 'unit-piece', 10)",
            [],
        )
        .unwrap();
    conn.execute(
        "INSERT INTO stock_balances (id, item_id, quantity, amount, average_price)
             VALUES ('balance-dept-budget', 'item-dept-budget', 20, 200, 10)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO budget_rules (
               id, department_id, category_id, period_month, amount_limit, enabled
             )
             VALUES ('budget-dept-total', 'dept-admin-office', NULL, '2026-06', 100, 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               department_id, movement_type
             )
             VALUES (
               'mov-dept-used', '2026-06-10', 'item-dept-budget', 'out', 8, 10, 80,
               'dept-admin-office', 'outbound'
             )",
        [],
    )
    .unwrap();

    let error = submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "outbound".to_string(),
            outbound_kind: Some("internal".to_string()),
            business_date: "2026-06-30".to_string(),
            department_id: Some("dept-admin-office".to_string()),
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: Some("部门总预算测试".to_string()),
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-dept-budget".to_string(),
                quantity: 3.0,
                unit_price: 10.0,
                amount: None,
                remark: None,
            }],
        },
        false,
    )
    .unwrap_err();

    assert!(error.to_string().contains("部门总预算"));
}
