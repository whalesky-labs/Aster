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

fn enforce_budget_limits(
    conn: &Connection,
    request: &SubmitStockDocumentRequest,
    lines: &[DocumentLineForConfirm],
    outbound_costs: &HashMap<String, f64>,
) -> AppResult<()> {
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
    let document_amount = round_money(outbound_costs.values().sum());
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
    for line in lines {
        let item = get_item_for_stock(conn, &line.item_id)?;
        let Some(category_id) = item.category_id else {
            continue;
        };
        let amount = outbound_costs
            .get(&line.line_id)
            .copied()
            .unwrap_or(line.amount);
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
                    AND m.movement_date >= b.period_month || '-01'
                    AND m.movement_date < date(b.period_month || '-01', '+1 month')
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
                    AND m.movement_date >= b.period_month || '-01'
                    AND m.movement_date < date(b.period_month || '-01', '+1 month')
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
    let date_part = business_date
        .chars()
        .take(10)
        .collect::<String>()
        .replace('-', "");
    let like = format!("{prefix}-{date_part}-%");
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM stock_documents WHERE document_no LIKE ?1",
        params![like],
        |row| row.get(0),
    )?;
    Ok(format!("{prefix}-{date_part}-{:04}", count + 1))
}

fn next_batch_no(conn: &Connection, document_no: &str) -> AppResult<String> {
    let like = format!("{document_no}-B%");
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM stock_batches WHERE batch_no LIKE ?1",
        params![like],
        |row| row.get(0),
    )?;
    Ok(format!("{document_no}-B{:03}", count + 1))
}

pub(crate) fn map_document(row: &rusqlite::Row<'_>) -> rusqlite::Result<StockDocument> {
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
        total_purchase_amount: row.get(16)?,
        total_sale_amount: row.get(17)?,
        total_cost_amount: row.get(18)?,
        total_gross_profit: row.get(19)?,
        item_summary: row.get(20)?,
        created_at: row.get(21)?,
        confirmed_at: row.get(22)?,
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
