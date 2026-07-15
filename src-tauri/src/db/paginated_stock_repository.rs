use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::domain::pagination::Page;
use crate::domain::stock::{
    StockDocument, StockDocumentQuery, StockMovementQuery, StockMovementRow,
};
use crate::error::{AppError, AppResult};

const PAGE_SIZE: i64 = 500;

#[derive(Debug, Serialize, Deserialize)]
struct Cursor {
    scope: String,
    revision: crate::db::pagination::Revision,
    sort_date: String,
    created_at: String,
    id: String,
}

struct Position {
    sort_date: String,
    created_at: String,
    id: String,
}

pub fn list_documents_page(
    conn: &Connection,
    query: StockDocumentQuery,
    cursor: Option<&str>,
) -> AppResult<Page<StockDocument>> {
    let document_type = normalize(query.document_type);
    let outbound_kind = normalize(query.outbound_kind);
    let month = normalize(query.month);
    let department_id = normalize(query.department_id);
    let supplier_id = normalize(query.supplier_id);
    let item_id = normalize(query.item_id);
    let handler = normalize(query.handler);
    let search_like = normalize(query.search).map(|value| format!("%{value}%"));
    let scope = query_scope(
        "stock-documents",
        &(
            &document_type,
            &outbound_kind,
            &month,
            &department_id,
            &supplier_id,
            &item_id,
            &handler,
            &search_like,
        ),
    )?;
    let cursor = decode_cursor(conn, &scope, cursor)?;
    let mut statement = conn.prepare(DOCUMENTS_SQL)?;
    let rows = statement.query_map(
        params![
            document_type,
            outbound_kind,
            month,
            department_id,
            supplier_id,
            item_id,
            handler,
            search_like,
            cursor.as_ref().map(|value| value.sort_date.as_str()),
            cursor.as_ref().map(|value| value.created_at.as_str()),
            cursor.as_ref().map(|value| value.id.as_str()),
            PAGE_SIZE + 1,
        ],
        crate::db::stock_repository::map_document,
    )?;
    let mut items = collect(rows)?;
    finish_page(conn, &scope, &mut items, |item| Position {
        sort_date: item.business_date.clone(),
        created_at: item.created_at.clone(),
        id: item.id.clone(),
    })
}

pub fn list_movements_page(
    conn: &Connection,
    query: StockMovementQuery,
    cursor: Option<&str>,
) -> AppResult<Page<StockMovementRow>> {
    let like = format!("%{}%", query.search.unwrap_or_default().trim());
    let item_id = normalize(query.item_id);
    let department_id = normalize(query.department_id);
    let direction = normalize(query.direction);
    let movement_type = normalize(query.movement_type);
    let scope = query_scope(
        "stock-movements",
        &(&like, &item_id, &department_id, &direction, &movement_type),
    )?;
    let cursor = decode_cursor(conn, &scope, cursor)?;
    let mut statement = conn.prepare(MOVEMENTS_SQL)?;
    let rows = statement.query_map(
        params![
            like,
            item_id,
            department_id,
            direction,
            movement_type,
            cursor.as_ref().map(|value| value.sort_date.as_str()),
            cursor.as_ref().map(|value| value.created_at.as_str()),
            cursor.as_ref().map(|value| value.id.as_str()),
            PAGE_SIZE + 1,
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
    let mut items = collect(rows)?;
    finish_page(conn, &scope, &mut items, |item| Position {
        sort_date: item.movement_date.clone(),
        created_at: item.created_at.clone(),
        id: item.id.clone(),
    })
}

fn finish_page<T>(
    conn: &Connection,
    scope: &str,
    items: &mut Vec<T>,
    position: impl FnOnce(&T) -> Position,
) -> AppResult<Page<T>> {
    let has_more = items.len() > PAGE_SIZE as usize;
    if has_more {
        items.truncate(PAGE_SIZE as usize);
    }
    let next_cursor = if has_more {
        items
            .last()
            .map(position)
            .map(|position| {
                encode_cursor(Cursor {
                    scope: scope.to_string(),
                    revision: crate::db::pagination::revision(conn)?,
                    sort_date: position.sort_date,
                    created_at: position.created_at,
                    id: position.id,
                })
            })
            .transpose()?
    } else {
        None
    };
    Ok(Page {
        items: std::mem::take(items),
        next_cursor,
    })
}

fn encode_cursor(cursor: Cursor) -> AppResult<String> {
    crate::db::pagination::encode_cursor(&cursor)
}

fn decode_cursor(
    conn: &Connection,
    scope: &str,
    cursor: Option<&str>,
) -> AppResult<Option<Cursor>> {
    let cursor: Option<Cursor> = cursor
        .filter(|value| !value.trim().is_empty())
        .map(crate::db::pagination::decode_cursor)
        .transpose()?;
    let revision = crate::db::pagination::revision(conn)?;
    if cursor
        .as_ref()
        .is_some_and(|value| value.scope != scope || value.revision != revision)
    {
        return Err(AppError::Validation(
            "分页游标无效或不属于当前查询".to_string(),
        ));
    }
    Ok(cursor)
}

fn query_scope(prefix: &str, values: &impl Serialize) -> AppResult<String> {
    let values = serde_json::to_string(values)
        .map_err(|error| AppError::Validation(format!("分页查询序列化失败：{error}")))?;
    Ok(format!("{prefix}:{values}"))
}

fn normalize(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

fn collect<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> AppResult<Vec<T>> {
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

const DOCUMENTS_SQL: &str = "WITH document_items AS (
  SELECT ranked.document_id, GROUP_CONCAT(ranked.item_label, '、') || CASE WHEN totals.item_count > 3 THEN ' 等 ' || totals.item_count || ' 项' ELSE '' END AS item_summary
  FROM (SELECT l.document_id, i.code || ' · ' || i.name AS item_label, ROW_NUMBER() OVER (PARTITION BY l.document_id ORDER BY MIN(l.created_at), i.code, i.name) AS row_number FROM stock_document_lines l JOIN master_items i ON i.id = l.item_id GROUP BY l.document_id, l.item_id) ranked
  JOIN (SELECT document_id, COUNT(DISTINCT item_id) AS item_count FROM stock_document_lines GROUP BY document_id) totals ON totals.document_id = ranked.document_id
  WHERE ranked.row_number <= 3 GROUP BY ranked.document_id)
SELECT d.id, d.document_no, d.document_type, d.outbound_kind, d.business_date, d.department_id, COALESCE(d.department_name, dep.name), d.supplier_id, COALESCE(d.supplier_name, sup.name), d.handler, d.purpose, d.approval_request_id, d.status, d.remark,
  COALESCE(SUM(l.quantity), 0), COALESCE(SUM(CASE WHEN d.document_type = 'inbound' THEN COALESCE(l.purchase_amount, l.amount) WHEN d.document_type = 'outbound' AND d.outbound_kind = 'guest_sale' THEN COALESCE(l.sale_amount, l.amount) ELSE COALESCE(l.cost_amount, l.amount) END), 0),
  COALESCE(SUM(COALESCE(l.purchase_amount, 0)), 0), COALESCE(SUM(COALESCE(l.sale_amount, 0)), 0), COALESCE(SUM(COALESCE(l.cost_amount, CASE WHEN d.document_type != 'inbound' THEN l.amount ELSE 0 END)), 0), COALESCE(SUM(COALESCE(l.sale_amount, 0) - COALESCE(l.cost_amount, 0)), 0), di.item_summary, d.created_at, d.confirmed_at
FROM stock_documents d LEFT JOIN departments dep ON dep.id = d.department_id LEFT JOIN suppliers sup ON sup.id = d.supplier_id LEFT JOIN stock_document_lines l ON l.document_id = d.id LEFT JOIN document_items di ON di.document_id = d.id
WHERE (?1 IS NULL OR d.document_type = ?1) AND (?2 IS NULL OR d.outbound_kind = ?2) AND (?3 IS NULL OR (d.business_date >= ?3 || '-01' AND d.business_date < date(?3 || '-01', '+1 month'))) AND (?4 IS NULL OR d.department_id = ?4) AND (?5 IS NULL OR d.supplier_id = ?5)
  AND (?6 IS NULL OR EXISTS (SELECT 1 FROM stock_document_lines f WHERE f.document_id = d.id AND f.item_id = ?6)) AND (?7 IS NULL OR d.handler = ?7)
  AND (?8 IS NULL OR d.document_no LIKE ?8 OR COALESCE(d.handler, '') LIKE ?8 OR COALESCE(d.purpose, '') LIKE ?8 OR COALESCE(d.remark, '') LIKE ?8 OR COALESCE(d.department_name, '') LIKE ?8 OR COALESCE(d.supplier_name, '') LIKE ?8 OR EXISTS (SELECT 1 FROM stock_document_lines sl JOIN master_items si ON si.id = sl.item_id WHERE sl.document_id = d.id AND (si.code LIKE ?8 OR si.name LIKE ?8 OR COALESCE(si.spec, '') LIKE ?8)))
  AND (?9 IS NULL OR d.business_date < ?9 OR (d.business_date = ?9 AND (d.created_at < ?10 OR (d.created_at = ?10 AND d.id < ?11))))
GROUP BY d.id ORDER BY d.business_date DESC, d.created_at DESC, d.id DESC LIMIT ?12";

const MOVEMENTS_SQL: &str = "SELECT m.id, m.movement_date, i.code, i.name, m.direction, m.quantity, m.unit_price, m.amount, d.document_no, COALESCE(m.department_name, dep.name), COALESCE(m.supplier_name, sup.name), m.movement_type, m.operator, m.remark, m.created_at
FROM stock_movements m JOIN master_items i ON i.id = m.item_id LEFT JOIN stock_documents d ON d.id = m.document_id LEFT JOIN departments dep ON dep.id = m.department_id LEFT JOIN suppliers sup ON sup.id = m.supplier_id
WHERE (?1 = '%%' OR i.code LIKE ?1 OR i.name LIKE ?1 OR COALESCE(d.document_no, '') LIKE ?1 OR COALESCE(m.operator, '') LIKE ?1 OR COALESCE(m.remark, '') LIKE ?1)
  AND (?2 IS NULL OR m.item_id = ?2) AND (?3 IS NULL OR m.department_id = ?3) AND (?4 IS NULL OR m.direction = ?4) AND (?5 IS NULL OR m.movement_type = ?5)
  AND (?6 IS NULL OR m.movement_date < ?6 OR (m.movement_date = ?6 AND (m.created_at < ?7 OR (m.created_at = ?7 AND m.id < ?8))))
ORDER BY m.movement_date DESC, m.created_at DESC, m.id DESC LIMIT ?9";

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use crate::db::migrations;
    use crate::domain::stock::StockDocumentQuery;

    use super::*;

    fn database_with_documents() -> Connection {
        let conn = Connection::open_in_memory().expect("database");
        migrations::run(&conn).expect("migrations");
        for index in 0..501 {
            conn.execute(
                "INSERT INTO stock_documents (
                   id, document_no, document_type, business_date, status, created_at
                 ) VALUES (?1, ?2, 'inbound', '2026-07-10 10:00:00', 'draft', ?3)",
                params![
                    format!("document-page-{index:03}"),
                    format!("IN-20260710-{index:04}"),
                    format!("2026-07-10 10:{:02}:{:02}", index / 60, index % 60)
                ],
            )
            .expect("document");
        }
        conn
    }

    #[test]
    fn document_cursor_is_bound_to_normalized_query() {
        let conn = database_with_documents();
        let query = StockDocumentQuery {
            document_type: Some("inbound".to_string()),
            ..Default::default()
        };
        let first = list_documents_page(&conn, query, None).expect("first page");
        let cursor = first.next_cursor.expect("cursor");
        let different_query = StockDocumentQuery {
            document_type: Some("outbound".to_string()),
            ..Default::default()
        };
        assert!(list_documents_page(&conn, different_query, Some(&cursor)).is_err());
    }

    #[test]
    fn document_cursor_is_rejected_after_business_data_changes() {
        let conn = database_with_documents();
        let first =
            list_documents_page(&conn, StockDocumentQuery::default(), None).expect("first page");
        let cursor = first.next_cursor.expect("cursor");
        conn.execute(
            "INSERT INTO stock_documents (
               id, document_no, document_type, business_date, status
             ) VALUES ('document-new', 'IN-NEW', 'inbound', '2026-07-10 11:00:00', 'draft')",
            [],
        )
        .expect("document");
        assert!(list_documents_page(&conn, StockDocumentQuery::default(), Some(&cursor)).is_err());
    }
}
