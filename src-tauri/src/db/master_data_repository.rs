use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::domain::master_data::{
    BudgetRule, Category, Department, Item, SaveBudgetRuleRequest, SaveCategoryRequest,
    SaveDepartmentRequest, SaveItemRequest, SaveSupplierRequest, SaveUnitRequest, Supplier,
    SupplierPurchaseRecord, Unit,
};
use crate::error::{AppError, AppResult};

const ITEM_LIST_LIMIT: i64 = 2_000;

pub fn list_categories(conn: &Connection) -> AppResult<Vec<Category>> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_id, name, enabled, sort_order, created_at, updated_at
         FROM categories
         ORDER BY enabled DESC, sort_order ASC, name ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Category {
            id: row.get(0)?,
            parent_id: row.get(1)?,
            name: row.get(2)?,
            enabled: row.get::<_, i64>(3)? == 1,
            sort_order: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    })?;
    collect_rows(rows)
}

pub fn save_category(conn: &Connection, request: SaveCategoryRequest) -> AppResult<Category> {
    let id = request.id.unwrap_or_else(new_id);
    let is_update = record_exists(conn, "categories", &id)?;
    let parent_id = blank_to_none(request.parent_id);
    if let Some(parent_id) = &parent_id {
        let parent_parent_id: Option<Option<String>> = conn
            .query_row(
                "SELECT parent_id FROM categories WHERE id = ?1",
                params![parent_id],
                |row| row.get(0),
            )
            .optional()?;
        match parent_parent_id {
            Some(None) => {}
            Some(Some(_)) => {
                return Err(crate::error::AppError::Validation(
                    "上级分类必须是大类".to_string(),
                ));
            }
            None => {
                return Err(crate::error::AppError::Validation(
                    "上级分类不存在".to_string(),
                ));
            }
        }
    }
    if is_update {
        require_current_version(
            conn,
            "categories",
            &id,
            request.expected_updated_at.as_deref(),
        )?;
        conn.execute(
            "UPDATE categories
             SET parent_id = ?1, name = ?2, enabled = ?3, sort_order = ?4, updated_at = ?5
             WHERE id = ?6",
            params![
                parent_id,
                request.name.trim(),
                bool_to_i64(request.enabled),
                request.sort_order,
                now_timestamp(),
                id
            ],
        )?;
    } else {
        conn.execute(
            "INSERT INTO categories (id, parent_id, name, enabled, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                id,
                parent_id,
                request.name.trim(),
                bool_to_i64(request.enabled),
                request.sort_order
            ],
        )?;
    }
    get_category(conn, &id)
}

pub fn set_category_enabled(
    conn: &Connection,
    id: &str,
    enabled: bool,
    expected_updated_at: Option<&str>,
) -> AppResult<()> {
    require_current_version(conn, "categories", id, expected_updated_at)?;
    ensure_changed(conn.execute(
        "UPDATE categories SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
        params![bool_to_i64(enabled), now_timestamp(), id],
    )?)?;
    Ok(())
}

pub fn list_units(conn: &Connection) -> AppResult<Vec<Unit>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, enabled, sort_order, created_at, updated_at
         FROM units
         ORDER BY enabled DESC, sort_order ASC, name ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Unit {
            id: row.get(0)?,
            name: row.get(1)?,
            enabled: row.get::<_, i64>(2)? == 1,
            sort_order: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;
    collect_rows(rows)
}

pub fn save_unit(conn: &Connection, request: SaveUnitRequest) -> AppResult<Unit> {
    let id = request.id.unwrap_or_else(new_id);
    let is_update = record_exists(conn, "units", &id)?;
    if is_update {
        require_current_version(conn, "units", &id, request.expected_updated_at.as_deref())?;
        conn.execute(
            "UPDATE units
             SET name = ?1, enabled = ?2, sort_order = ?3, updated_at = ?4
             WHERE id = ?5",
            params![
                request.name.trim(),
                bool_to_i64(request.enabled),
                request.sort_order,
                now_timestamp(),
                id
            ],
        )?;
    } else {
        conn.execute(
            "INSERT INTO units (id, name, enabled, sort_order)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                id,
                request.name.trim(),
                bool_to_i64(request.enabled),
                request.sort_order
            ],
        )?;
    }
    get_unit(conn, &id)
}

pub fn set_unit_enabled(
    conn: &Connection,
    id: &str,
    enabled: bool,
    expected_updated_at: Option<&str>,
) -> AppResult<()> {
    require_current_version(conn, "units", id, expected_updated_at)?;
    ensure_changed(conn.execute(
        "UPDATE units SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
        params![bool_to_i64(enabled), now_timestamp(), id],
    )?)?;
    Ok(())
}

pub fn list_departments(conn: &Connection) -> AppResult<Vec<Department>> {
    let mut stmt = conn.prepare(
        "SELECT id, code, name, manager, enabled, sort_order, remark, created_at, updated_at
         FROM departments
         ORDER BY enabled DESC, sort_order ASC, code ASC",
    )?;
    let rows = stmt.query_map([], |row| {
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
    })?;
    collect_rows(rows)
}

pub fn save_department(conn: &Connection, request: SaveDepartmentRequest) -> AppResult<Department> {
    let id = request.id.unwrap_or_else(new_id);
    let is_update = record_exists(conn, "departments", &id)?;
    if is_update {
        require_current_version(
            conn,
            "departments",
            &id,
            request.expected_updated_at.as_deref(),
        )?;
        conn.execute(
            "UPDATE departments
             SET code = ?1, name = ?2, manager = ?3, enabled = ?4, sort_order = ?5,
                 remark = ?6, updated_at = ?7
             WHERE id = ?8",
            params![
                request.code.trim(),
                request.name.trim(),
                blank_to_none(request.manager),
                bool_to_i64(request.enabled),
                request.sort_order,
                blank_to_none(request.remark),
                now_timestamp(),
                id
            ],
        )?;
    } else {
        conn.execute(
            "INSERT INTO departments (id, code, name, manager, enabled, sort_order, remark)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                request.code.trim(),
                request.name.trim(),
                blank_to_none(request.manager),
                bool_to_i64(request.enabled),
                request.sort_order,
                blank_to_none(request.remark)
            ],
        )?;
    }
    get_department(conn, &id)
}

pub fn set_department_enabled(
    conn: &Connection,
    id: &str,
    enabled: bool,
    expected_updated_at: Option<&str>,
) -> AppResult<()> {
    require_current_version(conn, "departments", id, expected_updated_at)?;
    ensure_changed(conn.execute(
        "UPDATE departments SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
        params![bool_to_i64(enabled), now_timestamp(), id],
    )?)?;
    Ok(())
}

pub fn list_suppliers(conn: &Connection) -> AppResult<Vec<Supplier>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, contact, phone, address, enabled, remark, created_at, updated_at
         FROM suppliers
         ORDER BY enabled DESC, name ASC",
    )?;
    let rows = stmt.query_map([], |row| {
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
    })?;
    collect_rows(rows)
}

pub fn save_supplier(conn: &Connection, request: SaveSupplierRequest) -> AppResult<Supplier> {
    let id = request.id.unwrap_or_else(new_id);
    let is_update = record_exists(conn, "suppliers", &id)?;
    if is_update {
        require_current_version(
            conn,
            "suppliers",
            &id,
            request.expected_updated_at.as_deref(),
        )?;
        conn.execute(
            "UPDATE suppliers
             SET name = ?1, contact = ?2, phone = ?3, address = ?4, enabled = ?5,
                 remark = ?6, updated_at = ?7
             WHERE id = ?8",
            params![
                request.name.trim(),
                blank_to_none(request.contact),
                blank_to_none(request.phone),
                blank_to_none(request.address),
                bool_to_i64(request.enabled),
                blank_to_none(request.remark),
                now_timestamp(),
                id
            ],
        )?;
    } else {
        conn.execute(
            "INSERT INTO suppliers (id, name, contact, phone, address, enabled, remark)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                request.name.trim(),
                blank_to_none(request.contact),
                blank_to_none(request.phone),
                blank_to_none(request.address),
                bool_to_i64(request.enabled),
                blank_to_none(request.remark)
            ],
        )?;
    }
    get_supplier(conn, &id)
}

pub fn set_supplier_enabled(
    conn: &Connection,
    id: &str,
    enabled: bool,
    expected_updated_at: Option<&str>,
) -> AppResult<()> {
    require_current_version(conn, "suppliers", id, expected_updated_at)?;
    ensure_changed(conn.execute(
        "UPDATE suppliers SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
        params![bool_to_i64(enabled), now_timestamp(), id],
    )?)?;
    Ok(())
}

pub fn list_supplier_purchase_records(
    conn: &Connection,
    supplier_id: &str,
) -> AppResult<Vec<SupplierPurchaseRecord>> {
    let mut stmt = conn.prepare(
        "SELECT m.movement_date, d.document_no, i.code, i.name, i.spec, u.name,
                m.quantity, m.unit_price, m.amount, m.remark
         FROM stock_movements m
         JOIN master_items i ON i.id = m.item_id
         LEFT JOIN units u ON u.id = i.unit_id
         LEFT JOIN stock_documents d ON d.id = m.document_id
         WHERE m.direction = 'in'
           AND m.supplier_id = ?1
         ORDER BY m.movement_date DESC, m.created_at DESC
         LIMIT 200",
    )?;
    let rows = stmt.query_map(params![supplier_id], |row| {
        Ok(SupplierPurchaseRecord {
            movement_date: row.get(0)?,
            document_no: row.get(1)?,
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
    collect_rows(rows)
}

pub fn list_items(conn: &Connection, search: Option<String>) -> AppResult<Vec<Item>> {
    let search = search.unwrap_or_default();
    let like = format!("%{}%", search.trim());
    let mut stmt = conn.prepare(
        "SELECT i.id, i.code, i.barcode, i.name, i.category_id, c.name, i.spec, i.unit_id, u.name,
                i.default_price, i.sale_price, i.supplier_id, s.name, i.warning_quantity,
                i.enabled, i.remark, i.created_at, i.updated_at
         FROM master_items i
         LEFT JOIN categories c ON c.id = i.category_id
         LEFT JOIN units u ON u.id = i.unit_id
         LEFT JOIN suppliers s ON s.id = i.supplier_id
         WHERE (?1 = '%%'
            OR i.code LIKE ?1
            OR COALESCE(i.barcode, '') LIKE ?1
            OR i.name LIKE ?1
            OR COALESCE(i.spec, '') LIKE ?1)
         ORDER BY i.enabled DESC, i.code ASC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![like, ITEM_LIST_LIMIT], |row| {
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
    })?;
    collect_rows(rows)
}

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
    let mut stmt = conn.prepare(
        "SELECT b.id, b.department_id, d.name, b.category_id, COALESCE(c.name, '全部分类'), b.period_month,
                b.amount_limit,
                COALESCE((
                  SELECT SUM(m.amount)
                  FROM stock_movements m
                  JOIN master_items i ON i.id = m.item_id
                  WHERE m.direction = 'out'
                    AND m.department_id = b.department_id
                    AND (b.category_id IS NULL OR i.category_id = b.category_id)
                    AND strftime('%Y-%m', m.movement_date) = b.period_month
                ), 0) AS used_amount,
                b.enabled, b.created_at, b.updated_at
         FROM budget_rules b
         JOIN departments d ON d.id = b.department_id
         LEFT JOIN categories c ON c.id = b.category_id
         WHERE (?1 IS NULL OR b.period_month = ?1)
         ORDER BY b.period_month DESC, d.sort_order ASC, b.category_id IS NOT NULL ASC, c.sort_order ASC, c.name ASC",
    )?;
    let rows = stmt.query_map(params![period_month], |row| {
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
    })?;
    collect_rows(rows)
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
                    AND strftime('%Y-%m', m.movement_date) = b.period_month
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

fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> AppResult<Vec<T>> {
    let mut output = Vec::new();
    for row in rows {
        output.push(row?);
    }
    Ok(output)
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

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::db::migrations;

    use super::*;

    #[test]
    fn save_category_supports_large_and_small_categories_only() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();

        let parent = save_category(
            &conn,
            SaveCategoryRequest {
                id: Some("category-food".to_string()),
                expected_updated_at: None,
                parent_id: None,
                name: "食品".to_string(),
                enabled: true,
                sort_order: 1,
            },
        )
        .unwrap();
        let child = save_category(
            &conn,
            SaveCategoryRequest {
                id: Some("category-rice".to_string()),
                expected_updated_at: None,
                parent_id: Some(parent.id.clone()),
                name: "米面粮油".to_string(),
                enabled: true,
                sort_order: 2,
            },
        )
        .unwrap();

        assert_eq!(child.parent_id.as_deref(), Some("category-food"));
        let categories = list_categories(&conn).unwrap();
        assert_eq!(categories.len(), 2);
        assert_eq!(
            categories
                .iter()
                .find(|category| category.id == "category-rice")
                .and_then(|category| category.parent_id.as_deref()),
            Some("category-food")
        );

        let grandchild = save_category(
            &conn,
            SaveCategoryRequest {
                id: Some("category-rice-imported".to_string()),
                expected_updated_at: None,
                parent_id: Some(child.id),
                name: "进口米".to_string(),
                enabled: true,
                sort_order: 3,
            },
        );
        assert!(grandchild.is_err());
    }

    #[test]
    fn list_supplier_purchase_records_filters_inbound_movements_by_supplier() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO suppliers (id, name) VALUES ('supplier-a', '供应商A')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-a', 'A-001', '采购物品', 'unit-piece', 10)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_documents (
               id, document_no, document_type, business_date, supplier_id, status
             )
             VALUES (
               'doc-a', 'IN-20260630-0001', 'inbound', '2026-06-30', 'supplier-a', 'confirmed'
             )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               document_id, supplier_id, movement_type
             )
             VALUES (
               'mov-a', '2026-06-30', 'item-a', 'in', 4, 10, 40,
               'doc-a', 'supplier-a', 'inbound'
             )",
            [],
        )
        .unwrap();

        let records = list_supplier_purchase_records(&conn, "supplier-a").unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].document_no.as_deref(), Some("IN-20260630-0001"));
        assert_eq!(records[0].item_name, "采购物品");
        assert_eq!(records[0].amount, 40.0);

        let empty = list_supplier_purchase_records(&conn, "supplier-missing").unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn list_items_supports_more_than_one_thousand_items() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();

        for index in 0..1005 {
            conn.execute(
                "INSERT INTO master_items (id, code, barcode, name, unit_id, default_price)
                 VALUES (?1, ?2, ?3, ?4, 'unit-piece', 1)",
                params![
                    format!("item-bulk-{index:04}"),
                    format!("BULK-{index:04}"),
                    format!("690000{index:04}"),
                    format!("批量物品 {index:04}")
                ],
            )
            .unwrap();
        }

        let items = list_items(&conn, None).unwrap();
        assert_eq!(items.len(), 1005);
        assert_eq!(items[0].code, "BULK-0000");
        assert_eq!(items[1004].code, "BULK-1004");

        let searched = list_items(&conn, Some("BULK-1004".to_string())).unwrap();
        assert_eq!(searched.len(), 1);
        assert_eq!(searched[0].barcode.as_deref(), Some("6900001004"));
    }

    #[test]
    fn save_item_requires_enabled_category_unit_and_supplier_references() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO categories (id, name, enabled)
             VALUES ('cat-disabled', '停用分类', 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO units (id, name, enabled)
             VALUES ('unit-disabled', '停用单位', 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO suppliers (id, name, enabled)
             VALUES ('supplier-disabled', '停用供应商', 0)",
            [],
        )
        .unwrap();

        let category_error = save_item(
            &conn,
            SaveItemRequest {
                id: Some("item-disabled-category".to_string()),
                expected_updated_at: None,
                code: Some("REF-001".to_string()),
                barcode: None,
                name: "停用分类物品".to_string(),
                category_id: Some("cat-disabled".to_string()),
                spec: None,
                unit_id: Some("unit-piece".to_string()),
                default_price: 1.0,
                sale_price: 0.0,
                supplier_id: None,
                warning_quantity: 0.0,
                enabled: true,
                remark: None,
            },
        )
        .unwrap_err();
        assert!(category_error.to_string().contains("分类已停用"));

        let unit_error = save_item(
            &conn,
            SaveItemRequest {
                id: Some("item-disabled-unit".to_string()),
                expected_updated_at: None,
                code: Some("REF-002".to_string()),
                barcode: None,
                name: "停用单位物品".to_string(),
                category_id: None,
                spec: None,
                unit_id: Some("unit-disabled".to_string()),
                default_price: 1.0,
                sale_price: 0.0,
                supplier_id: None,
                warning_quantity: 0.0,
                enabled: true,
                remark: None,
            },
        )
        .unwrap_err();
        assert!(unit_error.to_string().contains("单位已停用"));

        let supplier_error = save_item(
            &conn,
            SaveItemRequest {
                id: Some("item-disabled-supplier".to_string()),
                expected_updated_at: None,
                code: Some("REF-003".to_string()),
                barcode: None,
                name: "停用供应商物品".to_string(),
                category_id: None,
                spec: None,
                unit_id: Some("unit-piece".to_string()),
                default_price: 1.0,
                sale_price: 0.0,
                supplier_id: Some("supplier-disabled".to_string()),
                warning_quantity: 0.0,
                enabled: true,
                remark: None,
            },
        )
        .unwrap_err();
        assert!(supplier_error.to_string().contains("默认供应商已停用"));
    }

    #[test]
    fn save_item_generates_code_for_new_item_when_blank() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO master_items (id, code, name, unit_id, default_price)
             VALUES ('item-existing', 'HC-0001', '已有物品', 'unit-piece', 1)",
            [],
        )
        .unwrap();

        let item = save_item(
            &conn,
            SaveItemRequest {
                id: None,
                expected_updated_at: None,
                code: None,
                barcode: None,
                name: "自动编码物品".to_string(),
                category_id: None,
                spec: None,
                unit_id: Some("unit-piece".to_string()),
                default_price: 1.0,
                sale_price: 0.0,
                supplier_id: None,
                warning_quantity: 0.0,
                enabled: true,
                remark: None,
            },
        )
        .unwrap();

        assert_eq!(item.code, "HC-0002");
    }

    #[test]
    fn save_budget_rule_requires_enabled_department_and_category() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO departments (id, code, name, enabled)
             VALUES ('dept-budget-disabled', 'BDIS', '停用预算部门', 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO categories (id, name, enabled)
             VALUES ('cat-budget-disabled', '停用预算分类', 0)",
            [],
        )
        .unwrap();

        let department_error = save_budget_rule(
            &conn,
            SaveBudgetRuleRequest {
                id: Some("budget-disabled-dept".to_string()),
                expected_updated_at: None,
                department_id: "dept-budget-disabled".to_string(),
                category_id: Some("cat-budget-disabled".to_string()),
                period_month: "2026-06".to_string(),
                amount_limit: 100.0,
                enabled: true,
            },
        )
        .unwrap_err();
        assert!(department_error.to_string().contains("部门已停用"));

        let category_error = save_budget_rule(
            &conn,
            SaveBudgetRuleRequest {
                id: Some("budget-disabled-cat".to_string()),
                expected_updated_at: None,
                department_id: "dept-admin-office".to_string(),
                category_id: Some("cat-budget-disabled".to_string()),
                period_month: "2026-06".to_string(),
                amount_limit: 100.0,
                enabled: true,
            },
        )
        .unwrap_err();
        assert!(category_error.to_string().contains("分类已停用"));
    }

    #[test]
    fn save_budget_rule_allows_department_month_total_without_category() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO categories (id, name, enabled) VALUES ('cat-budget-total', '预算分类', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO master_items (id, code, name, category_id, unit_id, default_price)
             VALUES ('item-budget-total', 'BT-001', '预算物品', 'cat-budget-total', 'unit-piece', 10)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_movements (
               id, movement_date, item_id, direction, quantity, unit_price, amount,
               department_id, movement_type
             )
             VALUES (
               'mov-total-used', '2026-06-10', 'item-budget-total', 'out', 3, 10, 30,
               'dept-admin-office', 'outbound'
             )",
            [],
        )
        .unwrap();

        let rule = save_budget_rule(
            &conn,
            SaveBudgetRuleRequest {
                id: Some("budget-total".to_string()),
                expected_updated_at: None,
                department_id: "dept-admin-office".to_string(),
                category_id: None,
                period_month: "2026-06".to_string(),
                amount_limit: 100.0,
                enabled: true,
            },
        )
        .unwrap();

        assert_eq!(rule.category_id, None);
        assert_eq!(rule.category_name, "全部分类");
        assert_eq!(rule.used_amount, 30.0);
    }

    #[test]
    fn set_enabled_requires_existing_master_data_record() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();

        let error =
            set_unit_enabled(&conn, "missing-unit", false, Some("any-version")).unwrap_err();

        assert!(error.to_string().contains("要编辑的记录不存在"));
    }

    #[test]
    fn save_item_requires_matching_updated_at_for_existing_records() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();

        let item = save_item(
            &conn,
            SaveItemRequest {
                id: Some("item-lock".to_string()),
                expected_updated_at: None,
                code: Some("LOCK-001".to_string()),
                barcode: None,
                name: "乐观锁物品".to_string(),
                category_id: None,
                spec: None,
                unit_id: Some("unit-piece".to_string()),
                default_price: 1.0,
                sale_price: 0.0,
                supplier_id: None,
                warning_quantity: 0.0,
                enabled: true,
                remark: None,
            },
        )
        .unwrap();

        let missing_version = save_item(
            &conn,
            SaveItemRequest {
                id: Some(item.id.clone()),
                expected_updated_at: None,
                code: Some("LOCK-001".to_string()),
                barcode: None,
                name: "缺版本覆盖".to_string(),
                category_id: None,
                spec: None,
                unit_id: Some("unit-piece".to_string()),
                default_price: 1.0,
                sale_price: 0.0,
                supplier_id: None,
                warning_quantity: 0.0,
                enabled: true,
                remark: None,
            },
        )
        .unwrap_err();
        assert!(missing_version.to_string().contains("缺少版本信息"));

        let updated = save_item(
            &conn,
            SaveItemRequest {
                id: Some(item.id.clone()),
                expected_updated_at: Some(item.updated_at.clone()),
                code: Some("LOCK-001".to_string()),
                barcode: None,
                name: "已更新物品".to_string(),
                category_id: None,
                spec: None,
                unit_id: Some("unit-piece".to_string()),
                default_price: 2.0,
                sale_price: 0.0,
                supplier_id: None,
                warning_quantity: 0.0,
                enabled: true,
                remark: None,
            },
        )
        .unwrap();
        assert_eq!(updated.name, "已更新物品");

        let stale = save_item(
            &conn,
            SaveItemRequest {
                id: Some(item.id.clone()),
                expected_updated_at: Some(item.updated_at.clone()),
                code: Some("LOCK-001".to_string()),
                barcode: None,
                name: "旧页面覆盖".to_string(),
                category_id: None,
                spec: None,
                unit_id: Some("unit-piece".to_string()),
                default_price: 3.0,
                sale_price: 0.0,
                supplier_id: None,
                warning_quantity: 0.0,
                enabled: true,
                remark: None,
            },
        )
        .unwrap_err();
        assert!(stale.to_string().contains("已被其他客户端修改"));

        let missing_toggle_version = set_item_enabled(&conn, &updated.id, false, None).unwrap_err();
        assert!(missing_toggle_version.to_string().contains("缺少版本信息"));

        set_item_enabled(&conn, &updated.id, false, Some(&updated.updated_at)).unwrap();
        let toggled = get_item(&conn, &updated.id).unwrap();
        assert!(!toggled.enabled);

        let stale_toggle =
            set_item_enabled(&conn, &updated.id, true, Some(&updated.updated_at)).unwrap_err();
        assert!(stale_toggle.to_string().contains("已被其他客户端修改"));
    }
}
