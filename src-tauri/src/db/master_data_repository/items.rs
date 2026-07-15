pub fn save_item(conn: &Connection, request: SaveItemRequest) -> AppResult<Item> {
    let id = request.id.unwrap_or_else(new_id);
    let is_update = record_exists(conn, "master_items", &id)?;
    let code = match request
        .code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(code) => code.to_string(),
        None if is_update => {
            return Err(AppError::Validation("物品编码不能为空".to_string()));
        }
        None => next_item_code(conn)?,
    };
    let category_id = blank_to_none(request.category_id);
    let unit_id = blank_to_none(request.unit_id);
    let supplier_id = blank_to_none(request.supplier_id);
    require_enabled_reference(conn, "categories", category_id.as_deref(), "分类")?;
    require_enabled_reference(conn, "units", unit_id.as_deref(), "单位")?;
    require_enabled_reference(conn, "suppliers", supplier_id.as_deref(), "默认供应商")?;
    if is_update {
        require_current_version(
            conn,
            "master_items",
            &id,
            request.expected_updated_at.as_deref(),
        )?;
        conn.execute(
            "UPDATE master_items
             SET code = ?1, barcode = ?2, name = ?3, category_id = ?4, spec = ?5,
                 unit_id = ?6, default_price = ?7, sale_price = ?8, supplier_id = ?9,
                 warning_quantity = ?10, enabled = ?11, remark = ?12,
                 updated_at = ?13
             WHERE id = ?14",
            params![
                code,
                blank_to_none(request.barcode),
                request.name.trim(),
                category_id,
                blank_to_none(request.spec),
                unit_id,
                request.default_price,
                request.sale_price,
                supplier_id,
                request.warning_quantity,
                bool_to_i64(request.enabled),
                blank_to_none(request.remark),
                now_timestamp(),
                id
            ],
        )?;
    } else {
        conn.execute(
            "INSERT INTO master_items (
               id, code, barcode, name, category_id, spec, unit_id, default_price,
               sale_price, supplier_id, warning_quantity, enabled, remark
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                id,
                code,
                blank_to_none(request.barcode),
                request.name.trim(),
                category_id,
                blank_to_none(request.spec),
                unit_id,
                request.default_price,
                request.sale_price,
                supplier_id,
                request.warning_quantity,
                bool_to_i64(request.enabled),
                blank_to_none(request.remark)
            ],
        )?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO stock_balances (id, item_id) VALUES (?1, ?2)",
        params![new_id(), id],
    )?;
    get_item(conn, &id)
}

fn next_item_code(conn: &Connection) -> AppResult<String> {
    let mut index: i64 = conn.query_row("SELECT COUNT(*) + 1 FROM master_items", [], |row| {
        row.get(0)
    })?;
    loop {
        let code = format!("HC-{index:04}");
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM master_items WHERE code = ?1",
            params![code],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Ok(code);
        }
        index += 1;
    }
}

pub fn set_item_enabled(
    conn: &Connection,
    id: &str,
    enabled: bool,
    expected_updated_at: Option<&str>,
) -> AppResult<()> {
    require_current_version(conn, "master_items", id, expected_updated_at)?;
    ensure_changed(conn.execute(
        "UPDATE master_items SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
        params![bool_to_i64(enabled), now_timestamp(), id],
    )?)?;
    Ok(())
}

pub fn list_budget_rules(
    conn: &Connection,
    period_month: Option<String>,
) -> AppResult<Vec<BudgetRule>> {
    pagination::collect_all(|cursor| list_budget_rules_page(conn, period_month.clone(), cursor))
}

pub fn list_budget_rules_page(
    conn: &Connection,
    period_month: Option<String>,
    cursor: Option<&str>,
) -> AppResult<Page<BudgetRule>> {
    paginated_master_data_repository::budget_rules(conn, period_month, cursor)
}

pub fn save_budget_rule(
    conn: &Connection,
    request: SaveBudgetRuleRequest,
) -> AppResult<BudgetRule> {
    let id = request.id.unwrap_or_else(new_id);
    let is_update = record_exists(conn, "budget_rules", &id)?;
    require_enabled_reference(
        conn,
        "departments",
        Some(request.department_id.as_str()),
        "部门",
    )?;
    let category_id = blank_to_none(request.category_id.clone());
    require_enabled_reference(conn, "categories", category_id.as_deref(), "分类")?;
    if is_update {
        require_current_version(
            conn,
            "budget_rules",
            &id,
            request.expected_updated_at.as_deref(),
        )?;
        conn.execute(
            "UPDATE budget_rules
             SET department_id = ?1, category_id = ?2, period_month = ?3,
                 amount_limit = ?4, enabled = ?5, updated_at = ?6
             WHERE id = ?7",
            params![
                request.department_id,
                category_id,
                request.period_month,
                request.amount_limit,
                bool_to_i64(request.enabled),
                now_timestamp(),
                id
            ],
        )?;
    } else {
        conn.execute(
            "INSERT INTO budget_rules (
               id, department_id, category_id, period_month, amount_limit, enabled
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                request.department_id,
                category_id,
                request.period_month,
                request.amount_limit,
                bool_to_i64(request.enabled)
            ],
        )?;
    }
    get_budget_rule(conn, &id)
}

pub fn set_budget_rule_enabled(
    conn: &Connection,
    id: &str,
    enabled: bool,
    expected_updated_at: Option<&str>,
) -> AppResult<()> {
    require_current_version(conn, "budget_rules", id, expected_updated_at)?;
    ensure_changed(conn.execute(
        "UPDATE budget_rules SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
        params![bool_to_i64(enabled), now_timestamp(), id],
    )?)?;
    Ok(())
}

fn get_category(conn: &Connection, id: &str) -> AppResult<Category> {
    Ok(conn.query_row(
        "SELECT id, parent_id, name, enabled, sort_order, created_at, updated_at
         FROM categories WHERE id = ?1",
        params![id],
        |row| {
            Ok(Category {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                name: row.get(2)?,
                enabled: row.get::<_, i64>(3)? == 1,
                sort_order: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        },
    )?)
}

fn get_unit(conn: &Connection, id: &str) -> AppResult<Unit> {
    Ok(conn.query_row(
        "SELECT id, name, enabled, sort_order, created_at, updated_at
         FROM units WHERE id = ?1",
        params![id],
        |row| {
            Ok(Unit {
                id: row.get(0)?,
                name: row.get(1)?,
                enabled: row.get::<_, i64>(2)? == 1,
                sort_order: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        },
    )?)
}

fn get_department(conn: &Connection, id: &str) -> AppResult<Department> {
    Ok(conn.query_row(
        "SELECT id, code, name, manager, enabled, sort_order, remark, created_at, updated_at
         FROM departments WHERE id = ?1",
        params![id],
        |row| {
            Ok(Department {
                id: row.get(0)?,
                code: row.get(1)?,
                name: row.get(2)?,
                manager: row.get(3)?,
                enabled: row.get::<_, i64>(4)? == 1,
                sort_order: row.get(5)?,
                remark: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        },
    )?)
}

fn get_supplier(conn: &Connection, id: &str) -> AppResult<Supplier> {
    Ok(conn.query_row(
        "SELECT id, name, contact, phone, address, enabled, remark, created_at, updated_at
         FROM suppliers WHERE id = ?1",
        params![id],
        |row| {
            Ok(Supplier {
                id: row.get(0)?,
                name: row.get(1)?,
                contact: row.get(2)?,
                phone: row.get(3)?,
                address: row.get(4)?,
                enabled: row.get::<_, i64>(5)? == 1,
                remark: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        },
    )?)
}

fn get_item(conn: &Connection, id: &str) -> AppResult<Item> {
    Ok(conn.query_row(
        "SELECT i.id, i.code, i.barcode, i.name, i.category_id, c.name, i.spec, i.unit_id, u.name,
                i.default_price, i.sale_price, i.supplier_id, s.name, i.warning_quantity,
                i.enabled, i.remark, i.created_at, i.updated_at
         FROM master_items i
         LEFT JOIN categories c ON c.id = i.category_id
         LEFT JOIN units u ON u.id = i.unit_id
         LEFT JOIN suppliers s ON s.id = i.supplier_id
         WHERE i.id = ?1",
        params![id],
        |row| {
            Ok(Item {
                id: row.get(0)?,
                code: row.get(1)?,
                barcode: row.get(2)?,
                name: row.get(3)?,
                category_id: row.get(4)?,
                category_name: row.get(5)?,
                spec: row.get(6)?,
                unit_id: row.get(7)?,
                unit_name: row.get(8)?,
                default_price: row.get(9)?,
                sale_price: row.get(10)?,
                supplier_id: row.get(11)?,
                supplier_name: row.get(12)?,
                warning_quantity: row.get(13)?,
                enabled: row.get::<_, i64>(14)? == 1,
                remark: row.get(15)?,
                created_at: row.get(16)?,
                updated_at: row.get(17)?,
            })
        },
    )?)
}

fn get_budget_rule(conn: &Connection, id: &str) -> AppResult<BudgetRule> {
    Ok(conn.query_row(
        "SELECT b.id, b.department_id, d.name, b.category_id, COALESCE(c.name, '全部分类'), b.period_month,
                b.amount_limit,
                COALESCE((
                  SELECT SUM(m.amount)
                  FROM stock_movements m
                  JOIN master_items i ON i.id = m.item_id
                  WHERE m.direction = 'out'
                    AND m.department_id = b.department_id
                    AND (b.category_id IS NULL OR i.category_id = b.category_id)
                    AND m.movement_date >= b.period_month || '-01'
                    AND m.movement_date < date(b.period_month || '-01', '+1 month')
                ), 0) AS used_amount,
                b.enabled, b.created_at, b.updated_at
         FROM budget_rules b
         JOIN departments d ON d.id = b.department_id
         LEFT JOIN categories c ON c.id = b.category_id
         WHERE b.id = ?1",
        params![id],
        |row| {
            Ok(BudgetRule {
                id: row.get(0)?,
                department_id: row.get(1)?,
                department_name: row.get(2)?,
                category_id: row.get(3)?,
                category_name: row.get(4)?,
                period_month: row.get(5)?,
                amount_limit: row.get(6)?,
                used_amount: row.get(7)?,
                enabled: row.get::<_, i64>(8)? == 1,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        },
    )?)
}

fn require_current_version(
    conn: &Connection,
    table: &str,
    id: &str,
    expected_updated_at: Option<&str>,
) -> AppResult<()> {
    let expected = expected_updated_at
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Validation("编辑记录缺少版本信息，请刷新后重试".to_string()))?;
    let sql = format!("SELECT updated_at FROM {table} WHERE id = ?1");
    let current: Option<String> = conn
        .query_row(&sql, params![id], |row| row.get(0))
        .optional()?;
    match current {
        Some(current) if current == expected => Ok(()),
        Some(_) => Err(AppError::Validation(
            "记录已被其他客户端修改，请刷新后重试".to_string(),
        )),
        None => Err(AppError::Validation("要编辑的记录不存在".to_string())),
    }
}

fn record_exists(conn: &Connection, table: &str, id: &str) -> AppResult<bool> {
    let sql = format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id = ?1)");
    Ok(conn.query_row(&sql, params![id], |row| row.get::<_, bool>(0))?)
}

fn ensure_changed(changed: usize) -> AppResult<()> {
    if changed == 0 {
        Err(AppError::Validation("要编辑的记录不存在".to_string()))
    } else {
        Ok(())
    }
}

fn require_enabled_reference(
    conn: &Connection,
    table: &str,
    id: Option<&str>,
    label: &str,
) -> AppResult<()> {
    let Some(id) = id else {
        return Ok(());
    };
    let sql = match table {
        "categories" => "SELECT name, enabled FROM categories WHERE id = ?1",
        "units" => "SELECT name, enabled FROM units WHERE id = ?1",
        "departments" => "SELECT name, enabled FROM departments WHERE id = ?1",
        "suppliers" => "SELECT name, enabled FROM suppliers WHERE id = ?1",
        _ => return Err(AppError::Validation("不支持的关联资料类型".to_string())),
    };
    let Some((name, enabled)) = conn
        .query_row(sql, params![id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? == 1))
        })
        .optional()?
    else {
        return Err(AppError::Validation(format!("{label}不存在")));
    };
    if !enabled {
        return Err(AppError::Validation(format!("{label}已停用：{name}")));
    }
    Ok(())
}

fn now_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn bool_to_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
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
