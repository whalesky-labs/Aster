use super::*;
#[test]
fn guest_sale_outbound_skips_department_budget_and_reduces_stock() {
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
               'mov-used', '2026-06-10', 'item-budget', 'out', 10, 10, 100,
               'dept-admin-office', 'outbound'
             )",
        [],
    )
    .unwrap();

    let detail = submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "outbound".to_string(),
            outbound_kind: Some("guest_sale".to_string()),
            business_date: "2026-06-30".to_string(),
            department_id: None,
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: Some("客人购买".to_string()),
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
    .unwrap();

    assert_eq!(detail.document.outbound_kind.as_deref(), Some("guest_sale"));
    assert_eq!(detail.document.department_id, None);
    let quantity: f64 = conn
        .query_row(
            "SELECT quantity FROM stock_balances WHERE item_id = 'item-budget'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(quantity, 15.0);
}

#[test]
fn approved_budget_override_allows_over_budget_outbound() {
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
    conn.execute(
        "INSERT INTO approval_requests (
               id, entity_type, entity_id, status, reason
             )
             VALUES (
               'approval-1', 'budget_override', 'dept-admin-office:2026-06',
               'approved', '超预算领用'
             )",
        [],
    )
    .unwrap();

    let detail = submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "outbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-30".to_string(),
            department_id: Some("dept-admin-office".to_string()),
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: Some("预算审批测试".to_string()),
            remark: None,
            approval_request_id: Some("approval-1".to_string()),
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
    .unwrap();

    assert_eq!(detail.document.document_type, "outbound");
    assert_eq!(
        detail.document.approval_request_id.as_deref(),
        Some("approval-1")
    );
    let documents = list_stock_documents(
        &conn,
        StockDocumentQuery {
            document_type: Some("outbound".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(documents.len(), 1);
    assert_eq!(
        documents[0].approval_request_id.as_deref(),
        Some("approval-1")
    );
}

#[test]
fn allow_negative_stock_setting_allows_outbound_below_zero() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-negative', 'NEG-001', '负库存测试', 'unit-piece', 10)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO stock_balances (id, item_id, quantity, amount, average_price)
             VALUES ('balance-negative', 'item-negative', 1, 10, 10)",
        [],
    )
    .unwrap();

    let rejected = submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "outbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-30".to_string(),
            department_id: Some("dept-admin-office".to_string()),
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: Some("负库存测试".to_string()),
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-negative".to_string(),
                quantity: 2.0,
                unit_price: 10.0,
                amount: None,
                remark: None,
            }],
        },
        false,
    )
    .unwrap_err();
    assert!(rejected.to_string().contains("库存不足"));

    submit_stock_document(
        &mut conn,
        SubmitStockDocumentRequest {
            document_type: "outbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-30".to_string(),
            department_id: Some("dept-admin-office".to_string()),
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: Some("负库存测试".to_string()),
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-negative".to_string(),
                quantity: 2.0,
                unit_price: 10.0,
                amount: None,
                remark: None,
            }],
        },
        true,
    )
    .unwrap();

    let quantity: f64 = conn
        .query_row(
            "SELECT quantity FROM stock_balances WHERE item_id = 'item-negative'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(quantity, -1.0);
}

#[test]
fn save_and_confirm_draft_updates_inventory_only_on_confirm() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-draft', 'DR-001', '草稿物品', 'unit-piece', 10)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO stock_balances (id, item_id) VALUES ('balance-draft', 'item-draft')",
        [],
    )
    .unwrap();

    let draft = save_stock_document_draft(
        &mut conn,
        SaveStockDocumentDraftRequest {
            document_id: None,
            document_type: "inbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-30".to_string(),
            department_id: None,
            supplier_id: None,
            handler: Some("tester".to_string()),
            purpose: None,
            remark: Some("先保存".to_string()),
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: "item-draft".to_string(),
                quantity: 5.0,
                unit_price: 10.0,
                amount: None,
                remark: None,
            }],
        },
    )
    .unwrap();

    assert_eq!(draft.document.status, "draft");
    let movement_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM stock_movements", [], |row| row.get(0))
        .unwrap();
    assert_eq!(movement_count, 0);
    let quantity_before_confirm: f64 = conn
        .query_row(
            "SELECT quantity FROM stock_balances WHERE item_id = 'item-draft'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(quantity_before_confirm, 0.0);

    let confirmed = confirm_stock_document_draft(
        &mut conn,
        ConfirmStockDocumentDraftRequest {
            document_id: draft.document.id,
            approval_request_id: None,
        },
        false,
    )
    .unwrap();

    assert_eq!(confirmed.document.status, "confirmed");
    let movement_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM stock_movements", [], |row| row.get(0))
        .unwrap();
    assert_eq!(movement_count, 1);
    let quantity_after_confirm: f64 = conn
        .query_row(
            "SELECT quantity FROM stock_balances WHERE item_id = 'item-draft'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(quantity_after_confirm, 5.0);
}

#[test]
fn confirm_draft_revalidates_persisted_business_rules() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-invalid-draft', 'DR-INVALID', '异常草稿物品', 'unit-piece', 10)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO stock_balances (id, item_id, quantity, amount, average_price)
             VALUES ('balance-invalid-draft', 'item-invalid-draft', 5, 50, 10)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO stock_documents (
               id, document_no, document_type, business_date, status
             )
             VALUES ('doc-invalid-draft', 'OUT-INVALID-DRAFT', 'outbound', '2026-06-30', 'draft')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO stock_document_lines (id, document_id, item_id, quantity, unit_price, amount)
             VALUES ('line-invalid-draft', 'doc-invalid-draft', 'item-invalid-draft', 1, 10, 10)",
        [],
    )
    .unwrap();

    let error = confirm_stock_document_draft(
        &mut conn,
        ConfirmStockDocumentDraftRequest {
            document_id: "doc-invalid-draft".to_string(),
            approval_request_id: None,
        },
        false,
    )
    .unwrap_err();

    assert!(error.to_string().contains("出库/领用必须选择部门"));
    let movement_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM stock_movements", [], |row| row.get(0))
        .unwrap();
    assert_eq!(movement_count, 0);
    let status: String = conn
        .query_row(
            "SELECT status FROM stock_documents WHERE id = 'doc-invalid-draft'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status, "draft");
}

#[test]
fn submit_adjustment_and_void_document_write_inventory_movements() {
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

    let detail = submit_adjustment(
        &mut conn,
        SubmitAdjustmentRequest {
            business_date: "2026-06-30".to_string(),
            adjustment_type: "damage".to_string(),
            handler: Some("tester".to_string()),
            reason: "损耗处理".to_string(),
            lines: vec![SubmitAdjustmentLine {
                item_id: "item-1".to_string(),
                direction: "out".to_string(),
                quantity: 2.0,
                unit_price: 10.0,
                amount: None,
                remark: None,
            }],
        },
    )
    .unwrap();
    let balance: f64 = conn
        .query_row(
            "SELECT quantity FROM stock_balances WHERE item_id = 'item-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(balance, 8.0);

    void_stock_document(
        &mut conn,
        VoidStockDocumentRequest {
            document_id: detail.document.id,
            reason: "录入错误".to_string(),
            handler: Some("tester".to_string()),
        },
    )
    .unwrap();

    let restored_balance: f64 = conn
        .query_row(
            "SELECT quantity FROM stock_balances WHERE item_id = 'item-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(restored_balance, 10.0);
    let reversal_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE movement_type = 'reversal'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(reversal_count, 1);
    let voided_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stock_documents WHERE status = 'voided'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(voided_count, 1);
}
