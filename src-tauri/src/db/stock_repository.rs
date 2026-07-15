use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::db::pagination::{self, FETCH_SIZE};
use crate::domain::pagination::Page;
use crate::domain::stock::{
    ConfirmStockDocumentDraftRequest, SaveStockDocumentDraftRequest, StockBalanceQuery,
    StockBalanceRow, StockBatchRow, StockDocument, StockDocumentBatchLine, StockDocumentDetail,
    StockDocumentLine, StockDocumentQuery, StockMovementQuery, StockMovementRow,
    SubmitAdjustmentRequest, SubmitStockDocumentLine, SubmitStockDocumentRequest,
    VoidStockDocumentRequest,
};
use crate::error::{AppError, AppResult};

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
    get_stock_document_detail(conn, &document_id)
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
        let pricing = line_pricing(&request.document_type, outbound_kind.as_deref(), &line);
        tx.execute(
            "INSERT INTO stock_document_lines (
               id, document_id, item_id, quantity, unit_price, amount,
               purchase_unit_price, purchase_amount, sale_unit_price, sale_amount,
               cost_unit_price, cost_amount, remark
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                new_id(),
                document_id,
                line.item_id,
                line.quantity,
                pricing.unit_price,
                pricing.amount,
                pricing.purchase_unit_price,
                pricing.purchase_amount,
                pricing.sale_unit_price,
                pricing.sale_amount,
                pricing.cost_unit_price,
                pricing.cost_amount,
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
    get_stock_document_detail(conn, &document_id)
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
    get_stock_document_detail(conn, &request.document_id)
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
        let remark = line
            .remark
            .clone()
            .or_else(|| Some(format!("{}：{}", request.adjustment_type, request.reason)));
        if line.direction == "in" {
            create_batch_in_movement(
                &tx,
                BatchInMovementInput {
                    document_id: &document_id,
                    document_line_id: &line_id,
                    document_no: &document_no,
                    item_id: &line.item_id,
                    business_date: &request.business_date,
                    quantity: line.quantity,
                    unit_price: line.unit_price,
                    amount,
                    supplier_id: None,
                    supplier_name: None,
                    movement_type: "adjustment",
                    operator: blank_to_none(request.handler.clone())
                        .unwrap_or_else(|| "system".to_string()),
                    remark,
                },
            )?;
        } else {
            let (actual_unit_price, actual_amount) = create_batch_out_movements(
                &tx,
                BatchOutMovementInput {
                    document_id: &document_id,
                    document_line_id: &line_id,
                    item_id: &line.item_id,
                    business_date: &request.business_date,
                    quantity: line.quantity,
                    department_id: None,
                    department_name: None,
                    supplier_id: None,
                    supplier_name: None,
                    movement_type: "adjustment",
                    operator: blank_to_none(request.handler.clone())
                        .unwrap_or_else(|| "system".to_string()),
                    remark,
                    allow_negative_stock: false,
                    fallback_unit_price: line.unit_price,
                },
            )?;
            tx.execute(
                "UPDATE stock_document_lines
                 SET unit_price = ?1, amount = ?2
                 WHERE id = ?3",
                params![actual_unit_price, actual_amount, line_id],
            )?;
        }
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
    get_stock_document_detail(conn, &document_id)
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

    if document_has_batch_movements(&tx, &document_id)? {
        reverse_batch_document(
            &tx,
            &document_id,
            &document_no,
            &business_date,
            &request.reason,
            request.handler.clone(),
        )?;
    } else {
        let movements = load_document_movements(&tx, &document_id)?;
        for movement in movements {
            let reverse_direction = if movement.direction == "in" {
                "out"
            } else {
                "in"
            };
            crate::db::balance_repository::apply(
                &tx,
                crate::db::balance_repository::BalanceChange {
                    item_id: &movement.item_id,
                    direction: reverse_direction,
                    quantity: movement.quantity,
                    unit_price: movement.unit_price,
                    amount: movement.amount,
                    default_price: movement.unit_price,
                    allow_negative_stock: true,
                },
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
    get_stock_document_detail(conn, &document_id)
}

include!("stock_repository/queries.rs");

mod export;
pub use export::list_stock_balance_export_rows;

#[cfg(test)]
#[path = "stock_repository/tests.rs"]
mod tests;
