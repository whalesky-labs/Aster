use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::domain::stock::{
    ConfirmStockDocumentDraftRequest, SaveStockDocumentDraftRequest, StockBalanceQuery,
    StockBalanceRow, StockDocument, StockDocumentDetail, StockDocumentLine, StockDocumentQuery,
    StockMovementQuery, StockMovementRow, SubmitAdjustmentRequest, SubmitStockDocumentLine,
    SubmitStockDocumentRequest, VoidStockDocumentRequest,
};
use crate::error::{AppError, AppResult};

const STOCK_LIST_LIMIT: i64 = 2_000;

pub fn submit_stock_document(
    conn: &mut Connection,
    request: SubmitStockDocumentRequest,
    allow_negative_stock: bool,
) -> AppResult<StockDocumentDetail> {
    let tx = conn.transaction()?;
    let document_id = new_id();
    let document_no = next_document_no(&tx, &request.document_type, &request.business_date)?;
    insert_confirmed_document(
        &tx,
        &document_id,
        &document_no,
        request,
        allow_negative_stock,
    )?;
    tx.commit()?;
    get_document_detail(conn, &document_id)
}

pub fn save_stock_document_draft(
    conn: &mut Connection,
    request: SaveStockDocumentDraftRequest,
) -> AppResult<StockDocumentDetail> {
    let tx = conn.transaction()?;
    let document_id = request.document_id.clone().unwrap_or_else(new_id);
    let department_id = blank_to_none(request.department_id.clone());
    let supplier_id = blank_to_none(request.supplier_id.clone());
    let outbound_kind =
        normalized_outbound_kind(&request.document_type, request.outbound_kind.as_deref())?;
    let department_name = snapshot_department_name(&tx, department_id.as_deref())?;
    let supplier_name = snapshot_supplier_name(&tx, supplier_id.as_deref())?;
    let existing = tx
        .query_row(
            "SELECT document_no, status, document_type FROM stock_documents WHERE id = ?1",
            params![document_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;
    let document_no = if let Some((document_no, status, existing_type)) = existing {
        if status != "draft" {
            return Err(AppError::Validation("只能编辑草稿单据".to_string()));
        }
        if existing_type != request.document_type {
            return Err(AppError::Validation("草稿单据类型不能变更".to_string()));
        }
        tx.execute(
            "UPDATE stock_documents
             SET business_date = ?1, department_id = ?2, department_name = ?3,
                 supplier_id = ?4, supplier_name = ?5,
                 handler = ?6, purpose = ?7, approval_request_id = ?8, remark = ?9,
                 outbound_kind = ?10
             WHERE id = ?11",
            params![
                request.business_date,
                department_id,
                department_name,
                supplier_id,
                supplier_name,
                blank_to_none(request.handler.clone()),
                blank_to_none(request.purpose.clone()),
                blank_to_none(request.approval_request_id.clone()),
                blank_to_none(request.remark.clone()),
                outbound_kind,
                document_id
            ],
        )?;
        tx.execute(
            "DELETE FROM stock_document_lines WHERE document_id = ?1",
            params![document_id],
        )?;
        document_no
    } else {
        let document_no = next_document_no(&tx, &request.document_type, &request.business_date)?;
        tx.execute(
            "INSERT INTO stock_documents (
               id, document_no, document_type, outbound_kind, business_date, department_id,
               department_name, supplier_id, supplier_name, handler, purpose,
               approval_request_id, status, remark
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'draft', ?13)",
            params![
                document_id,
                document_no,
                request.document_type,
                outbound_kind,
                request.business_date,
                department_id,
                department_name,
                supplier_id,
                supplier_name,
                blank_to_none(request.handler.clone()),
                blank_to_none(request.purpose.clone()),
                blank_to_none(request.approval_request_id.clone()),
                blank_to_none(request.remark.clone())
            ],
        )?;
        document_no
    };

    for line in request.lines {
        let amount = line_amount(&line);
        tx.execute(
            "INSERT INTO stock_document_lines (id, document_id, item_id, quantity, unit_price, amount, remark)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                new_id(),
                document_id,
                line.item_id,
                line.quantity,
                line.unit_price,
                amount,
                blank_to_none(line.remark)
            ],
        )?;
    }

    tx.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, 'save_stock_document_draft', 'stock_document', ?2, ?3, ?4)",
        params![
            new_id(),
            document_id,
            document_no,
            blank_to_none(request.handler).unwrap_or_else(|| "system".to_string())
        ],
    )?;
    tx.commit()?;
    get_document_detail(conn, &document_id)
}

pub fn confirm_stock_document_draft(
    conn: &mut Connection,
    request: ConfirmStockDocumentDraftRequest,
    allow_negative_stock: bool,
) -> AppResult<StockDocumentDetail> {
    let tx = conn.transaction()?;
    let (
        document_no,
        document_type,
        outbound_kind,
        business_date,
        department_id,
        department_name,
        supplier_id,
        supplier_name,
        handler,
        purpose,
        remark,
        status,
        saved_approval_request_id,
    ) = tx
        .query_row(
            "SELECT document_no, document_type, outbound_kind, business_date, department_id, department_name,
                    supplier_id, supplier_name, handler, purpose, remark, status,
                    approval_request_id
             FROM stock_documents WHERE id = ?1",
            params![request.document_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, Option<String>>(12)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| AppError::Validation("草稿单据不存在".to_string()))?;
    if status != "draft" {
        return Err(AppError::Validation("只能确认草稿单据".to_string()));
    }
    let lines = load_document_lines_for_submit(&tx, &request.document_id)?;
    let final_approval_request_id = request.approval_request_id.or(saved_approval_request_id);
    let submit_request = SubmitStockDocumentRequest {
        document_type,
        outbound_kind,
        business_date,
        department_id,
        supplier_id,
        handler,
        purpose,
        remark,
        approval_request_id: final_approval_request_id.clone(),
        lines,
    };
    crate::services::stock_service::validate_document(&submit_request)?;
    apply_confirmed_document_effects(
        &tx,
        &request.document_id,
        &document_no,
        submit_request,
        SnapshotNames {
            department_name,
            supplier_name,
        },
        allow_negative_stock,
    )?;
    tx.execute(
        "UPDATE stock_documents
         SET status = 'confirmed',
             approval_request_id = ?1,
             confirmed_at = CURRENT_TIMESTAMP
         WHERE id = ?2",
        params![
            blank_to_none(final_approval_request_id),
            request.document_id
        ],
    )?;
    tx.commit()?;
    get_document_detail(conn, &request.document_id)
}

pub fn submit_adjustment(
    conn: &mut Connection,
    request: SubmitAdjustmentRequest,
) -> AppResult<StockDocumentDetail> {
    let tx = conn.transaction()?;
    let document_id = new_id();
    let document_no = next_document_no(&tx, "adjustment", &request.business_date)?;

    tx.execute(
        "INSERT INTO stock_documents (
           id, document_no, document_type, business_date, handler, purpose,
           approval_request_id, status, remark, confirmed_at
         )
         VALUES (?1, ?2, 'adjustment', ?3, ?4, ?5, NULL, 'confirmed', ?6, CURRENT_TIMESTAMP)",
        params![
            document_id,
            document_no,
            request.business_date,
            blank_to_none(request.handler.clone()),
            request.adjustment_type,
            request.reason
        ],
    )?;

    for line in request.lines {
        let item = get_item_for_stock(&tx, &line.item_id)?;
        let amount = adjustment_line_amount(&line);
        let line_id = new_id();
        tx.execute(
            "INSERT INTO stock_document_lines (id, document_id, item_id, quantity, unit_price, amount, remark)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                line_id,
                document_id,
                line.item_id,
                line.quantity,
                line.unit_price,
                amount,
                blank_to_none(line.remark.clone())
            ],
        )?;
        apply_balance(
            &tx,
            &line.item_id,
            &line.direction,
            line.quantity,
            line.unit_price,
            amount,
            item.default_price,
            true,
        )?;
        tx.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               document_id, document_line_id, movement_type, operator, remark
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'adjustment', ?10, ?11)",
            params![
                new_id(),
                request.business_date,
                line.item_id,
                line.direction,
                line.quantity,
                line.unit_price,
                amount,
                document_id,
                line_id,
                blank_to_none(request.handler.clone()).unwrap_or_else(|| "system".to_string()),
                line.remark
                    .clone()
                    .or_else(|| Some(format!("{}：{}", request.adjustment_type, request.reason)))
            ],
        )?;
    }

    tx.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, 'submit_adjustment', 'stock_document', ?2, ?3, ?4)",
        params![
            new_id(),
            document_id,
            document_no,
            blank_to_none(request.handler).unwrap_or_else(|| "system".to_string())
        ],
    )?;

    tx.commit()?;
    get_document_detail(conn, &document_id)
}

pub fn void_stock_document(
    conn: &mut Connection,
    request: VoidStockDocumentRequest,
) -> AppResult<StockDocumentDetail> {
    let tx = conn.transaction()?;
    let document = tx
        .query_row(
            "SELECT id, document_no, document_type, business_date, status
             FROM stock_documents WHERE id = ?1",
            params![request.document_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| AppError::Validation("单据不存在".to_string()))?;
    let (document_id, document_no, document_type, business_date, status) = document;
    if status != "confirmed" {
        return Err(AppError::Validation("只能作废已确认单据".to_string()));
    }
    if !matches!(
        document_type.as_str(),
        "inbound" | "outbound" | "adjustment" | "stocktake"
    ) {
        return Err(AppError::Validation(
            "当前单据类型暂不支持作废冲正".to_string(),
        ));
    }

    let movements = load_document_movements(&tx, &document_id)?;
    for movement in movements {
        let reverse_direction = if movement.direction == "in" {
            "out"
        } else {
            "in"
        };
        apply_balance(
            &tx,
            &movement.item_id,
            reverse_direction,
            movement.quantity,
            movement.unit_price,
            movement.amount,
            movement.unit_price,
            true,
        )?;
        tx.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               document_id, document_line_id, department_id, department_name,
               supplier_id, supplier_name, movement_type,
               operator, remark
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?10, ?11, ?12, 'reversal', ?13, ?14)",
            params![
                new_id(),
                business_date,
                movement.item_id,
                reverse_direction,
                movement.quantity,
                movement.unit_price,
                movement.amount,
                document_id,
                movement.department_id,
                movement.department_name,
                movement.supplier_id,
                movement.supplier_name,
                blank_to_none(request.handler.clone()).unwrap_or_else(|| "system".to_string()),
                format!("作废冲正 {}：{}", document_no, request.reason)
            ],
        )?;
    }

    tx.execute(
        "UPDATE stock_documents
         SET status = 'voided', voided_at = CURRENT_TIMESTAMP,
             remark = CASE
               WHEN COALESCE(remark, '') = '' THEN ?1
               ELSE remark || '；作废原因：' || ?1
             END
         WHERE id = ?2",
        params![request.reason, document_id],
    )?;
    if document_type == "stocktake" {
        tx.execute(
            "UPDATE stocktake_documents
             SET status = 'voided', updated_at = CURRENT_TIMESTAMP
             WHERE document_id = ?1",
            params![document_id],
        )?;
    }
    tx.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, 'void_stock_document', 'stock_document', ?2, ?3, ?4)",
        params![
            new_id(),
            document_id,
            format!("作废单据 {document_no}：{}", request.reason),
            blank_to_none(request.handler).unwrap_or_else(|| "system".to_string())
        ],
    )?;

    tx.commit()?;
    get_document_detail(conn, &document_id)
}

pub fn list_stock_documents(
    conn: &Connection,
    query: StockDocumentQuery,
) -> AppResult<Vec<StockDocument>> {
    let document_type = blank_to_none(query.document_type);
    let outbound_kind = blank_to_none(query.outbound_kind);
    let month = blank_to_none(query.month);
    let department_id = blank_to_none(query.department_id);
    let supplier_id = blank_to_none(query.supplier_id);
    let item_id = blank_to_none(query.item_id);
    let search = blank_to_none(query.search);
    let search_like = search.as_ref().map(|value| format!("%{}%", value.trim()));
    let mut stmt = conn.prepare(
        "SELECT d.id, d.document_no, d.document_type, d.outbound_kind, d.business_date,
                d.department_id, COALESCE(d.department_name, dep.name),
                d.supplier_id, COALESCE(d.supplier_name, sup.name),
                d.handler, d.purpose, d.approval_request_id, d.status, d.remark,
                COALESCE(SUM(l.quantity), 0), COALESCE(SUM(l.amount), 0),
                d.created_at, d.confirmed_at
         FROM stock_documents d
         LEFT JOIN departments dep ON dep.id = d.department_id
         LEFT JOIN suppliers sup ON sup.id = d.supplier_id
         LEFT JOIN stock_document_lines l ON l.document_id = d.id
         WHERE (?1 IS NULL OR d.document_type = ?1)
           AND (?2 IS NULL OR d.outbound_kind = ?2)
           AND (?3 IS NULL OR strftime('%Y-%m', d.business_date) = ?3)
           AND (?4 IS NULL OR d.department_id = ?4)
           AND (?5 IS NULL OR d.supplier_id = ?5)
           AND (?6 IS NULL OR EXISTS (
             SELECT 1 FROM stock_document_lines line_filter
             WHERE line_filter.document_id = d.id
               AND line_filter.item_id = ?6
           ))
           AND (?7 IS NULL OR d.document_no LIKE ?7 OR COALESCE(d.handler, '') LIKE ?7 OR COALESCE(d.purpose, '') LIKE ?7 OR COALESCE(d.remark, '') LIKE ?7 OR COALESCE(d.department_name, '') LIKE ?7 OR COALESCE(d.supplier_name, '') LIKE ?7)
         GROUP BY d.id
         ORDER BY d.business_date DESC, d.created_at DESC
         LIMIT ?8",
    )?;
    let rows = stmt.query_map(
        params![
            document_type,
            outbound_kind,
            month,
            department_id,
            supplier_id,
            item_id,
            search_like,
            STOCK_LIST_LIMIT
        ],
        map_document,
    )?;
    collect_rows(rows)
}

pub fn list_stock_balances(
    conn: &Connection,
    query: StockBalanceQuery,
) -> AppResult<Vec<StockBalanceRow>> {
    let search = query.search.unwrap_or_default();
    let like = format!("%{}%", search.trim());
    let category_id = blank_to_none(query.category_id);
    let item_id = blank_to_none(query.item_id);
    let stock_status = blank_to_none(query.stock_status);
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
             OR (?4 = 'low' AND COALESCE(b.quantity, 0) >= 0 AND i.warning_quantity > 0 AND COALESCE(b.quantity, 0) <= i.warning_quantity)
             OR (?4 = 'normal' AND COALESCE(b.quantity, 0) >= 0 AND (i.warning_quantity <= 0 OR COALESCE(b.quantity, 0) > i.warning_quantity))
           )
         ORDER BY i.enabled DESC, i.code ASC
         LIMIT ?5",
    )?;
    let rows = stmt.query_map(
        params![like, category_id, item_id, stock_status, STOCK_LIST_LIMIT],
        |row| {
            let quantity: f64 = row.get(5)?;
            let warning_quantity: f64 = row.get(9)?;
            let stock_status = if quantity < 0.0 {
                "negative"
            } else if warning_quantity > 0.0 && quantity <= warning_quantity {
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
    collect_rows(rows)
}

pub fn list_stock_movements(
    conn: &Connection,
    query: StockMovementQuery,
) -> AppResult<Vec<StockMovementRow>> {
    let search = query.search.unwrap_or_default();
    let like = format!("%{}%", search.trim());
    let item_id = blank_to_none(query.item_id);
    let department_id = blank_to_none(query.department_id);
    let direction = blank_to_none(query.direction);
    let movement_type = blank_to_none(query.movement_type);
    let mut stmt = conn.prepare(
        "SELECT m.id, m.movement_date, i.code, i.name, m.direction,
                m.quantity, m.unit_price, m.amount, d.document_no,
                COALESCE(m.department_name, dep.name),
                COALESCE(m.supplier_name, sup.name),
                m.movement_type, m.operator, m.remark, m.created_at
         FROM stock_movements m
         JOIN master_items i ON i.id = m.item_id
         LEFT JOIN stock_documents d ON d.id = m.document_id
         LEFT JOIN departments dep ON dep.id = m.department_id
         LEFT JOIN suppliers sup ON sup.id = m.supplier_id
         WHERE (?1 = '%%' OR i.code LIKE ?1 OR i.name LIKE ?1 OR COALESCE(d.document_no, '') LIKE ?1 OR COALESCE(m.operator, '') LIKE ?1 OR COALESCE(m.remark, '') LIKE ?1)
           AND (?2 IS NULL OR m.item_id = ?2)
           AND (?3 IS NULL OR m.department_id = ?3)
           AND (?4 IS NULL OR m.direction = ?4)
           AND (?5 IS NULL OR m.movement_type = ?5)
         ORDER BY m.movement_date DESC, m.created_at DESC
         LIMIT ?6",
    )?;
    let rows = stmt.query_map(
        params![
            like,
            item_id,
            department_id,
            direction,
            movement_type,
            STOCK_LIST_LIMIT
        ],
        |row| {
            Ok(StockMovementRow {
                id: row.get(0)?,
                movement_date: row.get(1)?,
                item_code: row.get(2)?,
                item_name: row.get(3)?,
                direction: row.get(4)?,
                quantity: row.get(5)?,
                unit_price: row.get(6)?,
                amount: row.get(7)?,
                document_no: row.get(8)?,
                department_name: row.get(9)?,
                supplier_name: row.get(10)?,
                movement_type: row.get(11)?,
                operator: row.get(12)?,
                remark: row.get(13)?,
                created_at: row.get(14)?,
            })
        },
    )?;
    collect_rows(rows)
}

fn get_document_detail(conn: &Connection, id: &str) -> AppResult<StockDocumentDetail> {
    let document = conn.query_row(
        "SELECT d.id, d.document_no, d.document_type, d.outbound_kind, d.business_date,
                d.department_id, COALESCE(d.department_name, dep.name),
                d.supplier_id, COALESCE(d.supplier_name, sup.name),
                d.handler, d.purpose, d.approval_request_id, d.status, d.remark,
                COALESCE(SUM(l.quantity), 0), COALESCE(SUM(l.amount), 0),
                d.created_at, d.confirmed_at
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
                l.quantity, l.unit_price, l.amount, l.remark
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
            remark: row.get(9)?,
        })
    })?;

    Ok(StockDocumentDetail {
        document,
        lines: collect_rows(rows)?,
    })
}

fn insert_confirmed_document(
    tx: &Connection,
    document_id: &str,
    document_no: &str,
    request: SubmitStockDocumentRequest,
    allow_negative_stock: bool,
) -> AppResult<()> {
    let department_id = blank_to_none(request.department_id.clone());
    let supplier_id = blank_to_none(request.supplier_id.clone());
    let outbound_kind =
        normalized_outbound_kind(&request.document_type, request.outbound_kind.as_deref())?;
    let department_name = snapshot_department_name(tx, department_id.as_deref())?;
    let supplier_name = snapshot_supplier_name(tx, supplier_id.as_deref())?;
    tx.execute(
        "INSERT INTO stock_documents (
           id, document_no, document_type, outbound_kind, business_date, department_id,
           department_name, supplier_id, supplier_name, handler, purpose,
           approval_request_id, status, remark, confirmed_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'confirmed', ?13, CURRENT_TIMESTAMP)",
        params![
            document_id,
            document_no,
            request.document_type,
            outbound_kind,
            request.business_date,
            department_id,
            department_name,
            supplier_id,
            supplier_name,
            blank_to_none(request.handler.clone()),
            blank_to_none(request.purpose.clone()),
            blank_to_none(request.approval_request_id.clone()),
            blank_to_none(request.remark.clone())
        ],
    )?;
    for line in &request.lines {
        let amount = line_amount(line);
        tx.execute(
            "INSERT INTO stock_document_lines (id, document_id, item_id, quantity, unit_price, amount, remark)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                new_id(),
                document_id,
                line.item_id,
                line.quantity,
                line.unit_price,
                amount,
                blank_to_none(line.remark.clone())
            ],
        )?;
    }
    apply_confirmed_document_effects(
        tx,
        document_id,
        document_no,
        request,
        SnapshotNames {
            department_name,
            supplier_name,
        },
        allow_negative_stock,
    )
}

fn apply_confirmed_document_effects(
    tx: &Connection,
    document_id: &str,
    document_no: &str,
    request: SubmitStockDocumentRequest,
    snapshot_names: SnapshotNames,
    allow_negative_stock: bool,
) -> AppResult<()> {
    validate_enabled_parties_for_document(tx, &request)?;
    enforce_budget_limits(tx, &request)?;
    let lines = load_document_lines_for_confirm(tx, document_id)?;
    let department_id = blank_to_none(request.department_id.clone());
    let supplier_id = blank_to_none(request.supplier_id.clone());
    for line in lines {
        let item = get_item_for_stock(tx, &line.item_id)?;
        let (direction, movement_type) = if request.document_type == "inbound" {
            ("in", "inbound")
        } else {
            ("out", "outbound")
        };
        apply_balance(
            tx,
            &line.item_id,
            direction,
            line.quantity,
            line.unit_price,
            line.amount,
            item.default_price,
            allow_negative_stock,
        )?;
        tx.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               document_id, document_line_id, department_id, department_name,
               supplier_id, supplier_name, movement_type,
               operator, remark
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                new_id(),
                request.business_date,
                line.item_id,
                direction,
                line.quantity,
                line.unit_price,
                line.amount,
                document_id,
                line.line_id,
                department_id,
                snapshot_names.department_name.clone(),
                supplier_id,
                snapshot_names.supplier_name.clone(),
                movement_type,
                blank_to_none(request.handler.clone()).unwrap_or_else(|| "system".to_string()),
                blank_to_none(line.remark)
            ],
        )?;
    }
    tx.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, ?2, 'stock_document', ?3, ?4, ?5)",
        params![
            new_id(),
            "submit_stock_document",
            document_id,
            document_no,
            blank_to_none(request.handler).unwrap_or_else(|| "system".to_string())
        ],
    )?;
    Ok(())
}

fn load_document_lines_for_submit(
    conn: &Connection,
    document_id: &str,
) -> AppResult<Vec<SubmitStockDocumentLine>> {
    let mut stmt = conn.prepare(
        "SELECT item_id, quantity, unit_price, amount, remark
         FROM stock_document_lines
         WHERE document_id = ?1
         ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![document_id], |row| {
        Ok(SubmitStockDocumentLine {
            item_id: row.get(0)?,
            quantity: row.get(1)?,
            unit_price: row.get(2)?,
            amount: row.get(3)?,
            remark: row.get(4)?,
        })
    })?;
    collect_rows(rows)
}

fn load_document_lines_for_confirm(
    conn: &Connection,
    document_id: &str,
) -> AppResult<Vec<DocumentLineForConfirm>> {
    let mut stmt = conn.prepare(
        "SELECT id, item_id, quantity, unit_price, amount, remark
         FROM stock_document_lines
         WHERE document_id = ?1
         ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![document_id], |row| {
        Ok(DocumentLineForConfirm {
            line_id: row.get(0)?,
            item_id: row.get(1)?,
            quantity: row.get(2)?,
            unit_price: row.get(3)?,
            amount: row.get(4)?,
            remark: row.get(5)?,
        })
    })?;
    collect_rows(rows)
}

fn apply_balance(
    conn: &Connection,
    item_id: &str,
    direction: &str,
    quantity: f64,
    unit_price: f64,
    amount: f64,
    default_price: f64,
    allow_negative_stock: bool,
) -> AppResult<()> {
    let existing = conn
        .query_row(
            "SELECT quantity, amount, average_price, last_inbound_price
             FROM stock_balances WHERE item_id = ?1",
            params![item_id],
            |row| {
                Ok((
                    row.get::<_, f64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, f64>(3)?,
                ))
            },
        )
        .optional()?
        .unwrap_or((0.0, 0.0, default_price, 0.0));

    let (old_qty, old_amount, old_avg_price, old_last_price) = existing;
    let (new_qty, new_amount, new_avg_price, new_last_price) = if direction == "in" {
        let next_qty = old_qty + quantity;
        let next_amount = old_amount + amount;
        let next_avg = if next_qty.abs() < f64::EPSILON {
            0.0
        } else {
            round_price(next_amount / next_qty)
        };
        (next_qty, round_money(next_amount), next_avg, unit_price)
    } else {
        if !allow_negative_stock && old_qty + f64::EPSILON < quantity {
            return Err(AppError::Validation(format!(
                "库存不足：当前库存 {old_qty}，出库数量 {quantity}"
            )));
        }
        let price = if old_avg_price > 0.0 {
            old_avg_price
        } else {
            unit_price
        };
        let out_amount = if amount > 0.0 {
            amount
        } else {
            round_money(quantity * price)
        };
        let next_qty = old_qty - quantity;
        let next_amount = if allow_negative_stock {
            old_amount - out_amount
        } else {
            (old_amount - out_amount).max(0.0)
        };
        let next_avg = if next_qty.abs() < f64::EPSILON {
            0.0
        } else {
            round_price(next_amount / next_qty)
        };
        (next_qty, round_money(next_amount), next_avg, old_last_price)
    };

    conn.execute(
        "INSERT INTO stock_balances (
           id, item_id, quantity, amount, average_price, last_inbound_price, updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP)
         ON CONFLICT(item_id) DO UPDATE SET
           quantity = excluded.quantity,
           amount = excluded.amount,
           average_price = excluded.average_price,
           last_inbound_price = excluded.last_inbound_price,
           updated_at = CURRENT_TIMESTAMP",
        params![
            new_id(),
            item_id,
            new_qty,
            new_amount,
            new_avg_price,
            new_last_price
        ],
    )?;

    Ok(())
}

fn load_document_movements(
    conn: &Connection,
    document_id: &str,
) -> AppResult<Vec<DocumentMovement>> {
    let mut stmt = conn.prepare(
        "SELECT item_id, direction, quantity, unit_price, amount,
                department_id, department_name, supplier_id, supplier_name
         FROM stock_movements
         WHERE document_id = ?1 AND movement_type != 'reversal'
         ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![document_id], |row| {
        Ok(DocumentMovement {
            item_id: row.get(0)?,
            direction: row.get(1)?,
            quantity: row.get(2)?,
            unit_price: row.get(3)?,
            amount: row.get(4)?,
            department_id: row.get(5)?,
            department_name: row.get(6)?,
            supplier_id: row.get(7)?,
            supplier_name: row.get(8)?,
        })
    })?;
    collect_rows(rows)
}

fn get_item_for_stock(conn: &Connection, item_id: &str) -> AppResult<ItemForStock> {
    conn.query_row(
        "SELECT i.id, i.default_price, i.enabled, i.category_id, c.name
         FROM master_items i
         LEFT JOIN categories c ON c.id = i.category_id
         WHERE i.id = ?1",
        params![item_id],
        |row| {
            Ok(ItemForStock {
                id: row.get(0)?,
                default_price: row.get(1)?,
                enabled: row.get::<_, i64>(2)? == 1,
                category_id: row.get(3)?,
                category_name: row.get(4)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| AppError::Validation("物品不存在".to_string()))
    .and_then(|item| {
        if item.enabled {
            Ok(item)
        } else {
            Err(AppError::Validation(format!("物品已停用：{}", item.id)))
        }
    })
}

fn validate_enabled_parties_for_document(
    conn: &Connection,
    request: &SubmitStockDocumentRequest,
) -> AppResult<()> {
    if request.document_type == "outbound" {
        let Some(department_id) = request
            .department_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(());
        };
        let department = conn
            .query_row(
                "SELECT name, enabled FROM departments WHERE id = ?1",
                params![department_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? == 1)),
            )
            .optional()?
            .ok_or_else(|| AppError::Validation("领用部门不存在".to_string()))?;
        if !department.1 {
            return Err(AppError::Validation(format!(
                "领用部门已停用：{}",
                department.0
            )));
        }
    }

    if request.document_type == "inbound" {
        let Some(supplier_id) = request
            .supplier_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(());
        };
        let supplier = conn
            .query_row(
                "SELECT name, enabled FROM suppliers WHERE id = ?1",
                params![supplier_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? == 1)),
            )
            .optional()?
            .ok_or_else(|| AppError::Validation("供应商不存在".to_string()))?;
        if !supplier.1 {
            return Err(AppError::Validation(format!(
                "供应商已停用：{}",
                supplier.0
            )));
        }
    }

    Ok(())
}

fn enforce_budget_limits(conn: &Connection, request: &SubmitStockDocumentRequest) -> AppResult<()> {
    if request.document_type != "outbound" {
        return Ok(());
    }
    if normalized_outbound_kind(&request.document_type, request.outbound_kind.as_deref())?
        == Some("guest_sale".to_string())
    {
        return Ok(());
    }
    let Some(department_id) = request
        .department_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    let period_month = request.business_date.chars().take(7).collect::<String>();
    let document_amount = round_money(request.lines.iter().map(line_amount).sum());
    if let Some((rule_id, amount_limit, used_amount)) =
        active_department_budget(conn, department_id, &period_month)?
    {
        if used_amount + document_amount > amount_limit
            && !approval_allows_budget_override(
                conn,
                request.approval_request_id.as_deref(),
                department_id,
                &period_month,
            )?
        {
            return Err(AppError::Validation(format!(
                "超出预算：{} 部门总预算已用 {:.2}，本单 {:.2}，预算 {:.2}（规则 {}），请先提交并通过审批",
                period_month, used_amount, document_amount, amount_limit, rule_id
            )));
        }
    }

    let mut category_amounts: HashMap<String, (String, f64)> = HashMap::new();
    for line in &request.lines {
        let item = get_item_for_stock(conn, &line.item_id)?;
        let Some(category_id) = item.category_id else {
            continue;
        };
        let amount = line_amount(line);
        let entry = category_amounts.entry(category_id).or_insert((
            item.category_name.unwrap_or_else(|| "未分类".to_string()),
            0.0,
        ));
        entry.1 = round_money(entry.1 + amount);
    }

    for (category_id, (category_name, current_amount)) in category_amounts {
        let Some((rule_id, amount_limit, used_amount)) =
            active_budget_for_category(conn, department_id, &category_id, &period_month)?
        else {
            continue;
        };
        if used_amount + current_amount > amount_limit {
            if approval_allows_budget_override(
                conn,
                request.approval_request_id.as_deref(),
                department_id,
                &period_month,
            )? {
                continue;
            }
            return Err(AppError::Validation(format!(
                "超出预算：{} {} 已用 {:.2}，本单 {:.2}，预算 {:.2}（规则 {}），请先提交并通过审批",
                period_month, category_name, used_amount, current_amount, amount_limit, rule_id
            )));
        }
    }
    Ok(())
}

fn active_department_budget(
    conn: &Connection,
    department_id: &str,
    period_month: &str,
) -> AppResult<Option<(String, f64, f64)>> {
    conn.query_row(
        "SELECT b.id, b.amount_limit,
                COALESCE((
                  SELECT SUM(m.amount)
                  FROM stock_movements m
                  WHERE m.direction = 'out'
                    AND m.department_id = b.department_id
                    AND strftime('%Y-%m', m.movement_date) = b.period_month
                ), 0)
         FROM budget_rules b
         WHERE b.enabled = 1
           AND b.department_id = ?1
           AND b.category_id IS NULL
           AND b.period_month = ?2
         ORDER BY b.updated_at DESC
         LIMIT 1",
        params![department_id, period_month],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .optional()
    .map_err(Into::into)
}

fn active_budget_for_category(
    conn: &Connection,
    department_id: &str,
    category_id: &str,
    period_month: &str,
) -> AppResult<Option<(String, f64, f64)>> {
    conn.query_row(
        "SELECT b.id, b.amount_limit,
                COALESCE((
                  SELECT SUM(m.amount)
                  FROM stock_movements m
                  JOIN master_items i ON i.id = m.item_id
                  WHERE m.direction = 'out'
                    AND m.department_id = b.department_id
                    AND i.category_id = b.category_id
                    AND strftime('%Y-%m', m.movement_date) = b.period_month
                ), 0)
         FROM budget_rules b
         WHERE b.enabled = 1
           AND b.department_id = ?1
           AND b.category_id = ?2
           AND b.period_month = ?3
         ORDER BY b.updated_at DESC
         LIMIT 1",
        params![department_id, category_id, period_month],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .optional()
    .map_err(Into::into)
}

fn approval_allows_budget_override(
    conn: &Connection,
    approval_request_id: Option<&str>,
    department_id: &str,
    period_month: &str,
) -> AppResult<bool> {
    let Some(approval_request_id) = approval_request_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(false);
    };
    let expected_entity_id = format!("{department_id}:{period_month}");
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
         FROM approval_requests
         WHERE id = ?1
           AND entity_type = 'budget_override'
           AND entity_id = ?2
           AND status = 'approved'",
        params![approval_request_id, expected_entity_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn next_document_no(
    conn: &Connection,
    document_type: &str,
    business_date: &str,
) -> AppResult<String> {
    let prefix = match document_type {
        "inbound" => "IN",
        "outbound" => "OUT",
        "adjustment" => "ADJ",
        other => return Err(AppError::Validation(format!("不支持的单据类型：{other}"))),
    };
    let date_part = business_date.replace('-', "");
    let like = format!("{prefix}-{date_part}-%");
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM stock_documents WHERE document_no LIKE ?1",
        params![like],
        |row| row.get(0),
    )?;
    Ok(format!("{prefix}-{date_part}-{:04}", count + 1))
}

fn map_document(row: &rusqlite::Row<'_>) -> rusqlite::Result<StockDocument> {
    Ok(StockDocument {
        id: row.get(0)?,
        document_no: row.get(1)?,
        document_type: row.get(2)?,
        outbound_kind: row.get(3)?,
        business_date: row.get(4)?,
        department_id: row.get(5)?,
        department_name: row.get(6)?,
        supplier_id: row.get(7)?,
        supplier_name: row.get(8)?,
        handler: row.get(9)?,
        purpose: row.get(10)?,
        approval_request_id: row.get(11)?,
        status: row.get(12)?,
        remark: row.get(13)?,
        total_quantity: row.get(14)?,
        total_amount: row.get(15)?,
        created_at: row.get(16)?,
        confirmed_at: row.get(17)?,
    })
}

fn normalized_outbound_kind(
    document_type: &str,
    outbound_kind: Option<&str>,
) -> AppResult<Option<String>> {
    if document_type != "outbound" {
        return Ok(None);
    }
    let value = outbound_kind
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("internal");
    match value {
        "internal" | "guest_sale" => Ok(Some(value.to_string())),
        other => Err(AppError::Validation(format!("不支持的出库类型：{other}"))),
    }
}

struct ItemForStock {
    id: String,
    default_price: f64,
    enabled: bool,
    category_id: Option<String>,
    category_name: Option<String>,
}

struct DocumentMovement {
    item_id: String,
    direction: String,
    quantity: f64,
    unit_price: f64,
    amount: f64,
    department_id: Option<String>,
    department_name: Option<String>,
    supplier_id: Option<String>,
    supplier_name: Option<String>,
}

struct DocumentLineForConfirm {
    line_id: String,
    item_id: String,
    quantity: f64,
    unit_price: f64,
    amount: f64,
    remark: Option<String>,
}

struct SnapshotNames {
    department_name: Option<String>,
    supplier_name: Option<String>,
}

fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> AppResult<Vec<T>> {
    let mut output = Vec::new();
    for row in rows {
        output.push(row?);
    }
    Ok(output)
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn blank_to_none(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn snapshot_department_name(
    conn: &Connection,
    department_id: Option<&str>,
) -> AppResult<Option<String>> {
    let Some(department_id) = department_id else {
        return Ok(None);
    };
    conn.query_row(
        "SELECT name FROM departments WHERE id = ?1",
        params![department_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

fn snapshot_supplier_name(
    conn: &Connection,
    supplier_id: Option<&str>,
) -> AppResult<Option<String>> {
    let Some(supplier_id) = supplier_id else {
        return Ok(None);
    };
    conn.query_row(
        "SELECT name FROM suppliers WHERE id = ?1",
        params![supplier_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

fn round_money(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round_price(value: f64) -> f64 {
    (value * 10000.0).round() / 10000.0
}

fn line_amount(line: &SubmitStockDocumentLine) -> f64 {
    effective_amount(line.quantity, line.unit_price, line.amount)
}

fn adjustment_line_amount(line: &crate::domain::stock::SubmitAdjustmentLine) -> f64 {
    effective_amount(line.quantity, line.unit_price, line.amount)
}

fn effective_amount(quantity: f64, unit_price: f64, amount: Option<f64>) -> f64 {
    round_money(
        amount
            .filter(|value| *value > 0.0)
            .unwrap_or(quantity * unit_price),
    )
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use super::*;
    use crate::db::migrations;
    use crate::db::stocktake_repository::{
        confirm_stocktake, create_stocktake, update_stocktake_counts,
    };
    use crate::domain::stock::{
        SubmitAdjustmentLine, SubmitAdjustmentRequest, SubmitStockDocumentLine,
        SubmitStockDocumentRequest, VoidStockDocumentRequest,
    };
    use crate::domain::stocktake::{
        ConfirmStocktakeRequest, CreateStocktakeRequest, UpdateStocktakeCountsRequest,
        UpdateStocktakeLineRequest,
    };

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
        assert_eq!(outbound.document.total_amount, 11.0);
        assert_eq!(outbound.lines[0].amount, 11.0);

        let balance: (f64, f64) = conn
            .query_row(
                "SELECT quantity, amount FROM stock_balances WHERE item_id = 'item-manual-amount'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(balance, (2.0, 24.0));

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
        assert_eq!(movement_amounts, vec![35.0, 11.0]);
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

        let inbound_detail = get_document_detail(&conn, &inbound.document.id).unwrap();
        let outbound_detail = get_document_detail(&conn, &outbound.document.id).unwrap();
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
               ('item-negative', 'NEG', '负库存', NULL, 'unit-piece', 1, 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_balances (id, item_id, quantity, amount, average_price)
             VALUES
               ('balance-normal', 'item-normal', 10, 10, 1),
               ('balance-low', 'item-low', 4, 4, 1),
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
        assert_eq!(low_balances.len(), 1);
        assert_eq!(low_balances[0].item_code, "LOW");

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
}
