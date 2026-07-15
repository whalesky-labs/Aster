use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::db::stock_repository::{
    create_batch_in_movement, create_batch_out_movements, BatchInMovementInput,
    BatchOutMovementInput,
};
use crate::db::{paginated_stocktake_repository, pagination};
use crate::domain::pagination::Page;
use crate::domain::stocktake::{
    ConfirmStocktakeRequest, CreateStocktakeRequest, StocktakeDetail, StocktakeDocument,
    StocktakeLine, UpdateStocktakeCountsRequest,
};
use crate::error::{AppError, AppResult};

pub fn create_stocktake(
    conn: &mut Connection,
    request: CreateStocktakeRequest,
) -> AppResult<StocktakeDetail> {
    let tx = conn.transaction()?;
    let document_id = new_id();
    let stocktake_id = new_id();
    let document_no = next_stocktake_no(&tx, &request.business_date)?;
    let scope_type = request.scope_type.trim();

    tx.execute(
        "INSERT INTO stock_documents (
           id, document_no, document_type, business_date, handler, purpose, status, remark
         )
         VALUES (?1, ?2, 'stocktake', ?3, ?4, ?5, 'draft', ?6)",
        params![
            document_id,
            document_no,
            request.business_date,
            blank_to_none(request.handler.clone()),
            scope_type,
            blank_to_none(request.remark.clone())
        ],
    )?;
    tx.execute(
        "INSERT INTO stocktake_documents (id, document_id, scope_type, status)
         VALUES (?1, ?2, ?3, 'counting')",
        params![stocktake_id, document_id, scope_type],
    )?;

    let items = load_items_for_scope(
        &tx,
        scope_type,
        request.category_id.as_deref(),
        &request.item_ids,
    )?;
    if items.is_empty() {
        return Err(AppError::Validation("盘点范围内没有可盘点物品".to_string()));
    }

    for item in items {
        tx.execute(
            "INSERT INTO stocktake_lines (
               id, stocktake_id, item_id, book_quantity, counted_quantity, difference_quantity
             )
             VALUES (?1, ?2, ?3, ?4, NULL, 0)",
            params![new_id(), stocktake_id, item.item_id, item.book_quantity],
        )?;
    }

    tx.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, 'create_stocktake', 'stocktake', ?2, ?3, ?4)",
        params![
            new_id(),
            stocktake_id,
            document_no,
            blank_to_none(request.handler).unwrap_or_else(|| "system".to_string())
        ],
    )?;

    tx.commit()?;
    get_stocktake_detail(conn, &stocktake_id)
}

pub fn list_stocktakes(conn: &Connection) -> AppResult<Vec<StocktakeDocument>> {
    pagination::collect_all(|cursor| list_stocktakes_page(conn, cursor))
}

pub fn list_stocktakes_page(
    conn: &Connection,
    cursor: Option<&str>,
) -> AppResult<Page<StocktakeDocument>> {
    paginated_stocktake_repository::list_page(conn, cursor)
}

pub fn get_stocktake_detail(conn: &Connection, stocktake_id: &str) -> AppResult<StocktakeDetail> {
    let document = get_stocktake_document(conn, stocktake_id)?;
    let mut stmt = conn.prepare(
        "SELECT l.id, l.stocktake_id, l.item_id, i.code, i.name, i.spec, u.name,
                l.book_quantity, l.counted_quantity, l.difference_quantity,
                COALESCE(b.average_price, i.default_price, 0),
                l.difference_quantity * COALESCE(b.average_price, i.default_price, 0),
                l.remark
         FROM stocktake_lines l
         JOIN master_items i ON i.id = l.item_id
         LEFT JOIN units u ON u.id = i.unit_id
         LEFT JOIN stock_balances b ON b.item_id = l.item_id
         WHERE l.stocktake_id = ?1
         ORDER BY i.code ASC",
    )?;
    let rows = stmt.query_map(params![stocktake_id], |row| {
        Ok(StocktakeLine {
            id: row.get(0)?,
            stocktake_id: row.get(1)?,
            item_id: row.get(2)?,
            item_code: row.get(3)?,
            item_name: row.get(4)?,
            spec: row.get(5)?,
            unit_name: row.get(6)?,
            book_quantity: row.get(7)?,
            counted_quantity: row.get(8)?,
            difference_quantity: row.get(9)?,
            average_price: row.get(10)?,
            difference_amount: row.get(11)?,
            remark: row.get(12)?,
        })
    })?;
    Ok(StocktakeDetail {
        document,
        lines: collect_rows(rows)?,
    })
}

pub fn update_stocktake_counts(
    conn: &Connection,
    request: UpdateStocktakeCountsRequest,
) -> AppResult<StocktakeDetail> {
    let status = stocktake_status(conn, &request.stocktake_id)?;
    if status == "confirmed" {
        return Err(AppError::Validation("已确认的盘点单不能修改".to_string()));
    }
    if status == "voided" {
        return Err(AppError::Validation("已作废的盘点单不能修改".to_string()));
    }

    for line in request.lines {
        if let Some(quantity) = line.counted_quantity {
            if quantity < 0.0 {
                return Err(AppError::Validation("实盘数量不能小于 0".to_string()));
            }
        }
        let affected = conn.execute(
            "UPDATE stocktake_lines
             SET counted_quantity = ?1,
                 difference_quantity = CASE
                   WHEN ?1 IS NULL THEN 0
                   ELSE ?1 - book_quantity
                 END,
                 remark = ?2,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?3 AND stocktake_id = ?4",
            params![
                line.counted_quantity,
                blank_to_none(line.remark),
                line.line_id,
                request.stocktake_id
            ],
        )?;
        if affected == 0 {
            return Err(AppError::Validation(format!(
                "盘点明细不存在或不属于当前盘点单：{}",
                line.line_id
            )));
        }
    }

    get_stocktake_detail(conn, &request.stocktake_id)
}

include!("stocktake_repository/confirmation.rs");

#[cfg(test)]
#[path = "stocktake_repository/tests.rs"]
mod tests;
