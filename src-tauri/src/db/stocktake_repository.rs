use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::db::stock_repository::{
    create_batch_in_movement, create_batch_out_movements, BatchInMovementInput,
    BatchOutMovementInput,
};
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
    let mut stmt = conn.prepare(
        "SELECT st.id, st.document_id, d.document_no, d.business_date, st.scope_type,
                st.status, d.handler, d.remark,
                COUNT(l.id),
                SUM(CASE WHEN l.counted_quantity IS NOT NULL THEN 1 ELSE 0 END),
                SUM(CASE WHEN ABS(l.difference_quantity) > 0.000001 THEN 1 ELSE 0 END),
                COALESCE(SUM(CASE WHEN l.difference_quantity > 0 THEN l.difference_quantity * COALESCE(b.average_price, i.default_price, 0) ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN l.difference_quantity < 0 THEN ABS(l.difference_quantity) * COALESCE(b.average_price, i.default_price, 0) ELSE 0 END), 0),
                st.created_at, d.confirmed_at
         FROM stocktake_documents st
         JOIN stock_documents d ON d.id = st.document_id
         LEFT JOIN stocktake_lines l ON l.stocktake_id = st.id
         LEFT JOIN master_items i ON i.id = l.item_id
         LEFT JOIN stock_balances b ON b.item_id = l.item_id
         GROUP BY st.id
         ORDER BY st.created_at DESC
         LIMIT 100",
    )?;
    let rows = stmt.query_map([], map_stocktake_document)?;
    collect_rows(rows)
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

pub fn confirm_stocktake(
    conn: &mut Connection,
    request: ConfirmStocktakeRequest,
) -> AppResult<StocktakeDetail> {
    let tx = conn.transaction()?;
    let status = stocktake_status(&tx, &request.stocktake_id)?;
    if status == "confirmed" {
        return Err(AppError::Validation("盘点单已经确认".to_string()));
    }
    if status == "voided" {
        return Err(AppError::Validation("已作废的盘点单不能确认".to_string()));
    }

    let (document_id, business_date, document_no) = tx.query_row(
        "SELECT d.id, d.business_date, d.document_no
         FROM stocktake_documents st
         JOIN stock_documents d ON d.id = st.document_id
         WHERE st.id = ?1",
        params![request.stocktake_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        },
    )?;

    let uncounted_count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM stocktake_lines
         WHERE stocktake_id = ?1 AND counted_quantity IS NULL",
        params![request.stocktake_id],
        |row| row.get(0),
    )?;
    if uncounted_count > 0 {
        return Err(AppError::Validation(format!(
            "还有 {uncounted_count} 行没有录入实盘数量"
        )));
    }

    let lines = load_stocktake_adjustment_lines(&tx, &request.stocktake_id)?;
    for line in lines {
        let quantity = line.difference_quantity.abs();
        if quantity <= 0.0 {
            continue;
        }
        let direction = if line.difference_quantity > 0.0 {
            "in"
        } else {
            "out"
        };
        let movement_type = if line.difference_quantity > 0.0 {
            "stocktake_gain"
        } else {
            "stocktake_loss"
        };
        let unit_price = if line.average_price > 0.0 {
            line.average_price
        } else {
            line.default_price
        };
        let amount = round_money(quantity * unit_price);
        let line_id = new_id();
        tx.execute(
            "INSERT INTO stock_document_lines (id, document_id, item_id, quantity, unit_price, amount, remark)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                line_id,
                document_id,
                line.item_id,
                quantity,
                unit_price,
                amount,
                request.remark.clone().or(line.remark.clone())
            ],
        )?;
        let remark = request.remark.clone().or(line.remark);
        let operator =
            blank_to_none(request.handler.clone()).unwrap_or_else(|| "system".to_string());
        if direction == "in" {
            create_batch_in_movement(
                &tx,
                BatchInMovementInput {
                    document_id: &document_id,
                    document_line_id: &line_id,
                    document_no: &document_no,
                    item_id: &line.item_id,
                    business_date: &business_date,
                    quantity,
                    unit_price,
                    amount,
                    supplier_id: None,
                    supplier_name: None,
                    movement_type,
                    operator,
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
                    business_date: &business_date,
                    quantity,
                    department_id: None,
                    department_name: None,
                    supplier_id: None,
                    supplier_name: None,
                    movement_type,
                    operator,
                    remark,
                    allow_negative_stock: false,
                    fallback_unit_price: unit_price,
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
        "UPDATE stocktake_documents
         SET status = 'confirmed', updated_at = CURRENT_TIMESTAMP
         WHERE id = ?1",
        params![request.stocktake_id],
    )?;
    tx.execute(
        "UPDATE stock_documents
         SET status = 'confirmed',
             handler = COALESCE(?1, handler),
             remark = COALESCE(?2, remark),
             confirmed_at = CURRENT_TIMESTAMP
         WHERE id = ?3",
        params![
            blank_to_none(request.handler.clone()),
            blank_to_none(request.remark.clone()),
            document_id
        ],
    )?;
    tx.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, 'confirm_stocktake', 'stocktake', ?2, ?3, ?4)",
        params![
            new_id(),
            request.stocktake_id,
            document_no,
            blank_to_none(request.handler).unwrap_or_else(|| "system".to_string())
        ],
    )?;

    tx.commit()?;
    get_stocktake_detail(conn, &request.stocktake_id)
}

fn get_stocktake_document(conn: &Connection, stocktake_id: &str) -> AppResult<StocktakeDocument> {
    conn.query_row(
        "SELECT st.id, st.document_id, d.document_no, d.business_date, st.scope_type,
                st.status, d.handler, d.remark,
                COUNT(l.id),
                SUM(CASE WHEN l.counted_quantity IS NOT NULL THEN 1 ELSE 0 END),
                SUM(CASE WHEN ABS(l.difference_quantity) > 0.000001 THEN 1 ELSE 0 END),
                COALESCE(SUM(CASE WHEN l.difference_quantity > 0 THEN l.difference_quantity * COALESCE(b.average_price, i.default_price, 0) ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN l.difference_quantity < 0 THEN ABS(l.difference_quantity) * COALESCE(b.average_price, i.default_price, 0) ELSE 0 END), 0),
                st.created_at, d.confirmed_at
         FROM stocktake_documents st
         JOIN stock_documents d ON d.id = st.document_id
         LEFT JOIN stocktake_lines l ON l.stocktake_id = st.id
         LEFT JOIN master_items i ON i.id = l.item_id
         LEFT JOIN stock_balances b ON b.item_id = l.item_id
         WHERE st.id = ?1
         GROUP BY st.id",
        params![stocktake_id],
        map_stocktake_document,
    )
    .optional()?
    .ok_or_else(|| AppError::Validation("盘点单不存在".to_string()))
}

fn load_items_for_scope(
    conn: &Connection,
    scope_type: &str,
    category_id: Option<&str>,
    item_ids: &[String],
) -> AppResult<Vec<ItemSnapshot>> {
    let mut sql = String::from(
        "SELECT i.id, COALESCE(b.quantity, 0)
         FROM master_items i
         LEFT JOIN stock_balances b ON b.item_id = i.id
         WHERE i.enabled = 1",
    );
    match scope_type {
        "all" => {}
        "category" => {
            let Some(category_id) = category_id.map(str::trim).filter(|value| !value.is_empty())
            else {
                return Err(AppError::Validation("分类盘点必须选择分类".to_string()));
            };
            require_enabled_category(conn, category_id)?;
            sql.push_str(" AND i.category_id = ?1");
        }
        "custom" => {
            if item_ids.is_empty() {
                return Err(AppError::Validation("自定义盘点必须选择物品".to_string()));
            }
            require_enabled_items(conn, item_ids)?;
            let quoted = item_ids
                .iter()
                .map(|id| format!("'{}'", id.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(",");
            sql.push_str(&format!(" AND i.id IN ({quoted})"));
        }
        other => return Err(AppError::Validation(format!("不支持的盘点范围：{other}"))),
    }
    sql.push_str(" ORDER BY i.code ASC");

    let mut stmt = conn.prepare(&sql)?;
    if scope_type == "category" {
        let rows = stmt.query_map(params![category_id], |row| {
            Ok(ItemSnapshot {
                item_id: row.get(0)?,
                book_quantity: row.get(1)?,
            })
        })?;
        collect_rows(rows)
    } else {
        let rows = stmt.query_map([], |row| {
            Ok(ItemSnapshot {
                item_id: row.get(0)?,
                book_quantity: row.get(1)?,
            })
        })?;
        collect_rows(rows)
    }
}

fn require_enabled_category(conn: &Connection, category_id: &str) -> AppResult<()> {
    let category = conn
        .query_row(
            "SELECT name, enabled FROM categories WHERE id = ?1",
            params![category_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? == 1)),
        )
        .optional()?
        .ok_or_else(|| AppError::Validation("盘点分类不存在".to_string()))?;
    if !category.1 {
        return Err(AppError::Validation(format!(
            "盘点分类已停用：{}",
            category.0
        )));
    }
    Ok(())
}

fn require_enabled_items(conn: &Connection, item_ids: &[String]) -> AppResult<()> {
    for item_id in item_ids {
        let item = conn
            .query_row(
                "SELECT name, enabled FROM master_items WHERE id = ?1",
                params![item_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? == 1)),
            )
            .optional()?
            .ok_or_else(|| AppError::Validation(format!("盘点物品不存在：{item_id}")))?;
        if !item.1 {
            return Err(AppError::Validation(format!("盘点物品已停用：{}", item.0)));
        }
    }
    Ok(())
}

fn load_stocktake_adjustment_lines(
    conn: &Connection,
    stocktake_id: &str,
) -> AppResult<Vec<StocktakeAdjustmentLine>> {
    let mut stmt = conn.prepare(
        "SELECT l.item_id, l.difference_quantity, COALESCE(b.average_price, 0),
                i.default_price, l.remark
         FROM stocktake_lines l
         JOIN master_items i ON i.id = l.item_id
         LEFT JOIN stock_balances b ON b.item_id = l.item_id
         WHERE l.stocktake_id = ?1
         ORDER BY i.code ASC",
    )?;
    let rows = stmt.query_map(params![stocktake_id], |row| {
        Ok(StocktakeAdjustmentLine {
            item_id: row.get(0)?,
            difference_quantity: row.get(1)?,
            average_price: row.get(2)?,
            default_price: row.get(3)?,
            remark: row.get(4)?,
        })
    })?;
    collect_rows(rows)
}

fn stocktake_status(conn: &Connection, stocktake_id: &str) -> AppResult<String> {
    conn.query_row(
        "SELECT status FROM stocktake_documents WHERE id = ?1",
        params![stocktake_id],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| AppError::Validation("盘点单不存在".to_string()))
}

fn next_stocktake_no(conn: &Connection, business_date: &str) -> AppResult<String> {
    let date_part = business_date
        .chars()
        .take(10)
        .collect::<String>()
        .replace('-', "");
    let like = format!("ST-{date_part}-%");
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM stock_documents WHERE document_no LIKE ?1",
        params![like],
        |row| row.get(0),
    )?;
    Ok(format!("ST-{date_part}-{:04}", count + 1))
}

fn map_stocktake_document(row: &rusqlite::Row<'_>) -> rusqlite::Result<StocktakeDocument> {
    Ok(StocktakeDocument {
        id: row.get(0)?,
        document_id: row.get(1)?,
        document_no: row.get(2)?,
        business_date: row.get(3)?,
        scope_type: row.get(4)?,
        status: row.get(5)?,
        handler: row.get(6)?,
        remark: row.get(7)?,
        line_count: row.get(8)?,
        counted_count: row.get(9)?,
        difference_count: row.get(10)?,
        gain_amount: row.get(11)?,
        loss_amount: row.get(12)?,
        created_at: row.get(13)?,
        confirmed_at: row.get(14)?,
    })
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

fn round_money(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

struct ItemSnapshot {
    item_id: String,
    book_quantity: f64,
}

struct StocktakeAdjustmentLine {
    item_id: String,
    difference_quantity: f64,
    average_price: f64,
    default_price: f64,
    remark: Option<String>,
}

#[cfg(test)]
mod tests {
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
}
