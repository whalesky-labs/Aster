pub(crate) fn category_consumption(
    conn: &Connection,
    filters: &ReportFilters<'_>,
    offset: Option<i64>,
) -> AppResult<Vec<CategoryConsumptionRow>> {
    let mut stmt = conn.prepare(&report_sql(
        "SELECT c.id, COALESCE(c.name, '未分类'),
                COALESCE(SUM(m.quantity), 0),
                COALESCE(SUM(m.amount), 0)
         FROM stock_movements m
         JOIN master_items i ON i.id = m.item_id
         LEFT JOIN categories c ON c.id = i.category_id
         WHERE m.direction = 'out'
           AND m.movement_date >= ?1 || '-01'
           AND m.movement_date < date(?1 || '-01', '+1 month')
           AND (?2 IS NULL OR m.movement_date >= ?2)
           AND (?3 IS NULL OR m.movement_date <= ?3)
           AND (?4 IS NULL OR m.department_id = ?4)
           AND (?5 IS NULL OR i.category_id = ?5)
           AND (?6 IS NULL OR i.id = ?6)
         GROUP BY c.id, c.name
         ORDER BY COALESCE(SUM(m.amount), 0) DESC, COALESCE(c.name, '未分类') ASC, c.id ASC",
        offset,
    ))?;
    let rows = stmt.query_map(
        params![
            filters.month,
            filters.start_date.as_deref(),
            filters.end_date.as_deref(),
            filters.department_id,
            filters.category_id,
            filters.item_id
        ],
        |row| {
            Ok(CategoryConsumptionRow {
                category_id: row.get(0)?,
                category_name: row.get(1)?,
                quantity: row.get(2)?,
                amount: row.get(3)?,
            })
        },
    )?;
    collect_rows(rows)
}

pub(crate) fn item_consumption_ranking(
    conn: &Connection,
    filters: &ReportFilters<'_>,
    offset: Option<i64>,
) -> AppResult<Vec<ItemConsumptionRow>> {
    let mut stmt = conn.prepare(&report_sql(
        "SELECT i.id, i.code, i.name, i.spec, u.name,
                COALESCE(SUM(m.quantity), 0),
                COALESCE(SUM(m.amount), 0)
         FROM stock_movements m
         JOIN master_items i ON i.id = m.item_id
         LEFT JOIN units u ON u.id = i.unit_id
         WHERE m.direction = 'out'
           AND m.movement_date >= ?1 || '-01'
           AND m.movement_date < date(?1 || '-01', '+1 month')
           AND (?2 IS NULL OR m.movement_date >= ?2)
           AND (?3 IS NULL OR m.movement_date <= ?3)
           AND (?4 IS NULL OR m.department_id = ?4)
           AND (?5 IS NULL OR i.category_id = ?5)
           AND (?6 IS NULL OR i.id = ?6)
         GROUP BY i.id
         ORDER BY COALESCE(SUM(m.amount), 0) DESC, COALESCE(SUM(m.quantity), 0) DESC, i.code ASC, i.id ASC",
        offset,
    ))?;
    let rows = stmt.query_map(
        params![
            filters.month,
            filters.start_date.as_deref(),
            filters.end_date.as_deref(),
            filters.department_id,
            filters.category_id,
            filters.item_id
        ],
        |row| {
            Ok(ItemConsumptionRow {
                item_id: row.get(0)?,
                item_code: row.get(1)?,
                item_name: row.get(2)?,
                spec: row.get(3)?,
                unit_name: row.get(4)?,
                quantity: row.get(5)?,
                amount: row.get(6)?,
            })
        },
    )?;
    collect_rows(rows)
}

pub(crate) fn inbound_details(
    conn: &Connection,
    filters: &ReportFilters<'_>,
    offset: Option<i64>,
) -> AppResult<Vec<InboundDetailRow>> {
    let mut stmt = conn.prepare(&report_sql(
        "SELECT m.movement_date, COALESCE(m.supplier_name, s.name), i.code, i.name, i.spec, u.name,
                m.quantity, m.unit_price, m.amount, doc.document_no, m.remark
         FROM stock_movements m
         JOIN master_items i ON i.id = m.item_id
         LEFT JOIN units u ON u.id = i.unit_id
         LEFT JOIN suppliers s ON s.id = m.supplier_id
         LEFT JOIN stock_documents doc ON doc.id = m.document_id
         WHERE m.direction = 'in'
           AND m.movement_date >= ?1 || '-01'
           AND m.movement_date < date(?1 || '-01', '+1 month')
           AND (?2 IS NULL OR m.movement_date >= ?2)
           AND (?3 IS NULL OR m.movement_date <= ?3)
           AND (?4 IS NULL OR i.category_id = ?4)
           AND (?5 IS NULL OR i.id = ?5)
           AND (?6 IS NULL OR m.supplier_id = ?6)
         ORDER BY m.movement_date ASC, s.name ASC, i.code ASC, m.created_at ASC, m.id ASC",
        offset,
    ))?;
    let rows = stmt.query_map(
        params![
            filters.month,
            filters.start_date.as_deref(),
            filters.end_date.as_deref(),
            filters.category_id,
            filters.item_id,
            filters.supplier_id
        ],
        |row| {
            Ok(InboundDetailRow {
                movement_date: row.get(0)?,
                supplier_name: row
                    .get::<_, Option<String>>(1)?
                    .unwrap_or_else(|| "未指定".to_string()),
                item_code: row.get(2)?,
                item_name: row.get(3)?,
                spec: row.get(4)?,
                unit_name: row.get(5)?,
                quantity: row.get(6)?,
                unit_price: row.get(7)?,
                amount: row.get(8)?,
                document_no: row.get(9)?,
                remark: row.get(10)?,
            })
        },
    )?;
    collect_rows(rows)
}

pub(crate) fn stock_warnings(
    conn: &Connection,
    filters: &ReportFilters<'_>,
    offset: Option<i64>,
) -> AppResult<Vec<StockWarningRow>> {
    let mut stmt = conn.prepare(&report_sql(
        "SELECT i.id, i.code, i.name, i.spec, u.name,
                COALESCE(b.quantity, 0),
                i.warning_quantity,
                MAX(i.warning_quantity - COALESCE(b.quantity, 0), 0),
                COALESCE(b.amount, 0)
         FROM master_items i
         LEFT JOIN units u ON u.id = i.unit_id
         LEFT JOIN stock_balances b ON b.item_id = i.id
         WHERE i.enabled = 1
           AND COALESCE(b.quantity, 0) >= 0
           AND COALESCE(b.quantity, 0) <= i.warning_quantity
           AND (?1 IS NULL OR i.category_id = ?1)
           AND (?2 IS NULL OR i.id = ?2)
           AND (?3 IS NULL OR i.supplier_id = ?3)
         ORDER BY (i.warning_quantity - COALESCE(b.quantity, 0)) DESC, i.code ASC, i.id ASC",
        offset,
    ))?;
    let rows = stmt.query_map(
        params![filters.category_id, filters.item_id, filters.supplier_id],
        |row| {
            Ok(StockWarningRow {
                item_id: row.get(0)?,
                item_code: row.get(1)?,
                item_name: row.get(2)?,
                spec: row.get(3)?,
                unit_name: row.get(4)?,
                quantity: row.get(5)?,
                warning_quantity: row.get(6)?,
                shortage_quantity: row.get(7)?,
                amount: row.get(8)?,
            })
        },
    )?;
    collect_rows(rows)
}

pub(crate) fn stock_balances(
    conn: &Connection,
    filters: &ReportFilters<'_>,
    offset: Option<i64>,
) -> AppResult<Vec<StockBalanceReportRow>> {
    let mut stmt = conn.prepare(&report_sql(
        "SELECT i.id, i.code, i.name, i.spec, u.name,
                COALESCE(b.quantity, 0),
                COALESCE(b.amount, 0),
                COALESCE(b.average_price, 0),
                COALESCE(b.last_inbound_price, 0),
                i.warning_quantity
         FROM master_items i
         LEFT JOIN units u ON u.id = i.unit_id
         LEFT JOIN stock_balances b ON b.item_id = i.id
         WHERE i.enabled = 1
           AND (?1 IS NULL OR i.category_id = ?1)
           AND (?2 IS NULL OR i.id = ?2)
           AND (?3 IS NULL OR i.supplier_id = ?3)
         ORDER BY i.code ASC, i.id ASC",
        offset,
    ))?;
    let rows = stmt.query_map(
        params![filters.category_id, filters.item_id, filters.supplier_id],
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
            Ok(StockBalanceReportRow {
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

pub(crate) fn stocktake_differences(
    conn: &Connection,
    filters: &ReportFilters<'_>,
    offset: Option<i64>,
) -> AppResult<Vec<StocktakeDifferenceReportRow>> {
    let mut stmt = conn.prepare(&report_sql(
        "SELECT d.business_date, d.document_no, st.scope_type, st.status,
                i.code, i.name, i.spec, u.name,
                l.book_quantity,
                COALESCE(l.counted_quantity, 0),
                l.difference_quantity,
                COALESCE(b.average_price, i.default_price, 0),
                l.difference_quantity * COALESCE(b.average_price, i.default_price, 0),
                l.remark
         FROM stocktake_documents st
         JOIN stock_documents d ON d.id = st.document_id
         JOIN stocktake_lines l ON l.stocktake_id = st.id
         JOIN master_items i ON i.id = l.item_id
         LEFT JOIN units u ON u.id = i.unit_id
         LEFT JOIN stock_balances b ON b.item_id = i.id
         WHERE st.status = 'confirmed'
           AND d.status = 'confirmed'
           AND d.business_date >= ?1 || '-01'
           AND d.business_date < date(?1 || '-01', '+1 month')
           AND (?2 IS NULL OR d.business_date >= ?2)
           AND (?3 IS NULL OR d.business_date <= ?3)
           AND ABS(l.difference_quantity) > 0.000001
           AND (?4 IS NULL OR i.category_id = ?4)
           AND (?5 IS NULL OR i.id = ?5)
         ORDER BY d.business_date ASC, d.document_no ASC, i.code ASC, l.id ASC",
        offset,
    ))?;
    let rows = stmt.query_map(
        params![
            filters.month,
            filters.start_date.as_deref(),
            filters.end_date.as_deref(),
            filters.category_id,
            filters.item_id
        ],
        |row| {
            Ok(StocktakeDifferenceReportRow {
                business_date: row.get(0)?,
                document_no: row.get(1)?,
                scope_type: row.get(2)?,
                status: row.get(3)?,
                item_code: row.get(4)?,
                item_name: row.get(5)?,
                spec: row.get(6)?,
                unit_name: row.get(7)?,
                book_quantity: row.get(8)?,
                counted_quantity: row.get(9)?,
                difference_quantity: row.get(10)?,
                average_price: row.get(11)?,
                difference_amount: row.get(12)?,
                remark: row.get(13)?,
            })
        },
    )?;
    collect_rows(rows)
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
