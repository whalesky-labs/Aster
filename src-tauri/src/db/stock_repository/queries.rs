pub fn list_stock_documents(
    conn: &Connection,
    query: StockDocumentQuery,
) -> AppResult<Vec<StockDocument>> {
    pagination::collect_all(|cursor| {
        crate::db::paginated_stock_repository::list_documents_page(conn, query.clone(), cursor)
    })
}

pub fn list_stock_balances(
    conn: &Connection,
    query: StockBalanceQuery,
) -> AppResult<Vec<StockBalanceRow>> {
    pagination::collect_all(|cursor| list_stock_balances_page(conn, query.clone(), cursor))
}

pub fn list_stock_balances_page(
    conn: &Connection,
    query: StockBalanceQuery,
    cursor: Option<&str>,
) -> AppResult<Page<StockBalanceRow>> {
    let search = query.search.unwrap_or_default();
    let like = format!("%{}%", search.trim());
    let category_id = blank_to_none(query.category_id);
    let item_id = blank_to_none(query.item_id);
    let stock_status = blank_to_none(query.stock_status);
    let scope = format!(
        "stock-balances:{}:{}:{}:{}",
        search.trim().to_lowercase(),
        category_id.as_deref().unwrap_or_default(),
        item_id.as_deref().unwrap_or_default(),
        stock_status.as_deref().unwrap_or_default()
    );
    let offset = pagination::offset(conn, &scope, cursor)?;
    let mut stmt = conn.prepare(
        "SELECT i.id, i.code, i.name, i.spec, u.name,
                COALESCE(b.quantity, 0), COALESCE(b.amount, 0),
                COALESCE(b.average_price, 0), COALESCE(b.last_inbound_price, 0),
                i.warning_quantity
         FROM master_items i
         LEFT JOIN units u ON u.id = i.unit_id
         LEFT JOIN stock_balances b ON b.item_id = i.id
         WHERE (?1 = '%%' OR i.code LIKE ?1 OR i.name LIKE ?1 OR COALESCE(i.spec, '') LIKE ?1)
           AND (?2 IS NULL OR i.category_id = ?2)
           AND (?3 IS NULL OR i.id = ?3)
           AND (
             ?4 IS NULL
             OR (?4 = 'negative' AND COALESCE(b.quantity, 0) < 0)
             OR (?4 = 'low' AND COALESCE(b.quantity, 0) >= 0 AND COALESCE(b.quantity, 0) <= i.warning_quantity)
             OR (?4 = 'normal' AND COALESCE(b.quantity, 0) >= 0 AND COALESCE(b.quantity, 0) > i.warning_quantity)
           )
         ORDER BY i.enabled DESC, i.code ASC, i.id ASC
         LIMIT ?5 OFFSET ?6",
    )?;
    let rows = stmt.query_map(
        params![like, category_id, item_id, stock_status, FETCH_SIZE, offset],
        |row| {
            let quantity: f64 = row.get(5)?;
            let warning_quantity: f64 = row.get(9)?;
            let stock_status = if quantity < 0.0 {
                "negative"
            } else if quantity <= warning_quantity {
                "low"
            } else {
                "normal"
            };
            Ok(StockBalanceRow {
                item_id: row.get(0)?,
                item_code: row.get(1)?,
                item_name: row.get(2)?,
                spec: row.get(3)?,
                unit_name: row.get(4)?,
                quantity,
                amount: row.get(6)?,
                average_price: row.get(7)?,
                last_inbound_price: row.get(8)?,
                warning_quantity,
                stock_status: stock_status.to_string(),
            })
        },
    )?;
    pagination::page(conn, &scope, offset, collect_rows(rows)?)
}

pub fn list_stock_batches(conn: &Connection, item_id: &str) -> AppResult<Vec<StockBatchRow>> {
    pagination::collect_all(|cursor| list_stock_batches_page(conn, item_id, cursor))
}

pub fn list_stock_batches_page(
    conn: &Connection,
    item_id: &str,
    cursor: Option<&str>,
) -> AppResult<Page<StockBatchRow>> {
    let item_id = item_id.trim();
    if item_id.is_empty() {
        return Err(AppError::Validation("物品 ID 不能为空".to_string()));
    }
    ensure_opening_batch_from_balance(conn, item_id)?;
    let scope = format!("stock-batches:{item_id}");
    let offset = pagination::offset(conn, &scope, cursor)?;
    let mut stmt = conn.prepare(
        "SELECT b.id, b.item_id, i.code, i.name, b.batch_no, b.inbound_date,
                b.supplier_name, b.original_quantity, b.remaining_quantity,
                b.unit_price, b.original_amount, b.remaining_amount, b.status,
                d.document_no, b.created_at, b.updated_at
         FROM stock_batches b
         JOIN master_items i ON i.id = b.item_id
         LEFT JOIN stock_documents d ON d.id = b.source_document_id
         WHERE b.item_id = ?1
         ORDER BY b.inbound_date ASC, b.created_at ASC, b.batch_no ASC, b.id ASC
         LIMIT ?2 OFFSET ?3",
    )?;
    let rows = stmt.query_map(params![item_id, FETCH_SIZE, offset], |row| {
        Ok(StockBatchRow {
            id: row.get(0)?,
            item_id: row.get(1)?,
            item_code: row.get(2)?,
            item_name: row.get(3)?,
            batch_no: row.get(4)?,
            inbound_date: row.get(5)?,
            supplier_name: row.get(6)?,
            original_quantity: row.get(7)?,
            remaining_quantity: row.get(8)?,
            unit_price: row.get(9)?,
            original_amount: row.get(10)?,
            remaining_amount: row.get(11)?,
            status: row.get(12)?,
            source_document_no: row.get(13)?,
            created_at: row.get(14)?,
            updated_at: row.get(15)?,
        })
    })?;
    pagination::page(conn, &scope, offset, collect_rows(rows)?)
}

pub fn list_stock_movements(
    conn: &Connection,
    query: StockMovementQuery,
) -> AppResult<Vec<StockMovementRow>> {
    pagination::collect_all(|cursor| {
        crate::db::paginated_stock_repository::list_movements_page(conn, query.clone(), cursor)
    })
}

pub fn get_stock_document_detail(conn: &Connection, id: &str) -> AppResult<StockDocumentDetail> {
    let document = conn.query_row(
        "SELECT d.id, d.document_no, d.document_type, d.outbound_kind, d.business_date,
                d.department_id, COALESCE(d.department_name, dep.name),
                d.supplier_id, COALESCE(d.supplier_name, sup.name),
                d.handler, d.purpose, d.approval_request_id, d.status, d.remark,
                COALESCE(SUM(l.quantity), 0),
                COALESCE(SUM(CASE
                  WHEN d.document_type = 'inbound' THEN COALESCE(l.purchase_amount, l.amount)
                  WHEN d.document_type = 'outbound' AND d.outbound_kind = 'guest_sale' THEN COALESCE(l.sale_amount, l.amount)
                  ELSE COALESCE(l.cost_amount, l.amount)
                END), 0),
                COALESCE(SUM(COALESCE(l.purchase_amount, 0)), 0),
                COALESCE(SUM(COALESCE(l.sale_amount, 0)), 0),
                COALESCE(SUM(COALESCE(l.cost_amount, CASE WHEN d.document_type != 'inbound' THEN l.amount ELSE 0 END)), 0),
                COALESCE(SUM(COALESCE(l.sale_amount, 0) - COALESCE(l.cost_amount, 0)), 0),
                NULL, d.created_at, d.confirmed_at
         FROM stock_documents d
         LEFT JOIN departments dep ON dep.id = d.department_id
         LEFT JOIN suppliers sup ON sup.id = d.supplier_id
         LEFT JOIN stock_document_lines l ON l.document_id = d.id
         WHERE d.id = ?1
         GROUP BY d.id",
        params![id],
        map_document,
    )?;

    let mut stmt = conn.prepare(
        "SELECT l.id, l.item_id, i.code, i.name, i.spec, u.name,
                l.quantity, l.unit_price, l.amount,
                l.purchase_unit_price, l.purchase_amount,
                l.sale_unit_price, l.sale_amount,
                l.cost_unit_price, l.cost_amount,
                CASE
                  WHEN l.sale_amount IS NULL THEN NULL
                  ELSE COALESCE(l.sale_amount, 0) - COALESCE(l.cost_amount, 0)
                END,
                l.remark
         FROM stock_document_lines l
         JOIN master_items i ON i.id = l.item_id
         LEFT JOIN units u ON u.id = i.unit_id
         WHERE l.document_id = ?1
         ORDER BY l.created_at ASC",
    )?;
    let rows = stmt.query_map(params![id], |row| {
        Ok(StockDocumentLine {
            id: row.get(0)?,
            item_id: row.get(1)?,
            item_code: row.get(2)?,
            item_name: row.get(3)?,
            spec: row.get(4)?,
            unit_name: row.get(5)?,
            quantity: row.get(6)?,
            unit_price: row.get(7)?,
            amount: row.get(8)?,
            purchase_unit_price: row.get(9)?,
            purchase_amount: row.get(10)?,
            sale_unit_price: row.get(11)?,
            sale_amount: row.get(12)?,
            cost_unit_price: row.get(13)?,
            cost_amount: row.get(14)?,
            gross_profit: row.get(15)?,
            remark: row.get(16)?,
        })
    })?;

    let lines = collect_rows(rows)?;
    let mut batch_stmt = conn.prepare(
        "SELECT bm.id, bm.document_line_id, i.id, i.code, i.name,
                b.id, b.batch_no, b.inbound_date, b.supplier_name,
                bm.direction, bm.quantity, bm.unit_price, bm.amount,
                bm.movement_type, bm.created_at
         FROM stock_batch_movements bm
         JOIN stock_batches b ON b.id = bm.batch_id
         JOIN master_items i ON i.id = b.item_id
         WHERE bm.document_id = ?1
         ORDER BY bm.created_at ASC, i.code ASC, b.inbound_date ASC",
    )?;
    let batch_rows = batch_stmt.query_map(params![id], |row| {
        Ok(StockDocumentBatchLine {
            id: row.get(0)?,
            item_id: row.get(2)?,
            item_code: row.get(3)?,
            item_name: row.get(4)?,
            batch_id: row.get(5)?,
            batch_no: row.get(6)?,
            inbound_date: row.get(7)?,
            supplier_name: row.get(8)?,
            direction: row.get(9)?,
            quantity: row.get(10)?,
            unit_price: row.get(11)?,
            amount: row.get(12)?,
            movement_type: row.get(13)?,
            created_at: row.get(14)?,
        })
    })?;

    Ok(StockDocumentDetail {
        document,
        lines,
        batch_lines: collect_rows(batch_rows)?,
    })
}

include!("persistence.rs");
include!("movements.rs");
include!("fifo.rs");
include!("validation.rs");
include!("helpers.rs");
