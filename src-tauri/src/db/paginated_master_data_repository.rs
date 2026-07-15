use rusqlite::{params, Connection};

use crate::db::pagination::{self, FETCH_SIZE};
use crate::domain::master_data::{
    BudgetRule, Category, Department, Item, Supplier, SupplierPurchaseRecord, Unit,
};
use crate::domain::pagination::Page;
use crate::error::AppResult;

pub fn categories(conn: &Connection, cursor: Option<&str>) -> AppResult<Page<Category>> {
    let offset = pagination::offset(conn, "categories", cursor)?;
    let mut statement = conn.prepare(
        "SELECT id, parent_id, name, enabled, sort_order, created_at, updated_at FROM categories
         ORDER BY enabled DESC, sort_order ASC, name ASC, id ASC LIMIT ?1 OFFSET ?2",
    )?;
    let rows = statement.query_map(params![FETCH_SIZE, offset], |row| {
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
    pagination::page(conn, "categories", offset, collect(rows)?)
}

pub fn units(conn: &Connection, cursor: Option<&str>) -> AppResult<Page<Unit>> {
    let offset = pagination::offset(conn, "units", cursor)?;
    let mut statement = conn.prepare(
        "SELECT id, name, enabled, sort_order, created_at, updated_at FROM units
         ORDER BY enabled DESC, sort_order ASC, name ASC, id ASC LIMIT ?1 OFFSET ?2",
    )?;
    let rows = statement.query_map(params![FETCH_SIZE, offset], |row| {
        Ok(Unit {
            id: row.get(0)?,
            name: row.get(1)?,
            enabled: row.get::<_, i64>(2)? == 1,
            sort_order: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;
    pagination::page(conn, "units", offset, collect(rows)?)
}

pub fn departments(conn: &Connection, cursor: Option<&str>) -> AppResult<Page<Department>> {
    let offset = pagination::offset(conn, "departments", cursor)?;
    let mut statement = conn.prepare(
        "SELECT id, code, name, manager, enabled, sort_order, remark, created_at, updated_at
         FROM departments ORDER BY enabled DESC, sort_order ASC, code ASC, id ASC
         LIMIT ?1 OFFSET ?2",
    )?;
    let rows = statement.query_map(params![FETCH_SIZE, offset], |row| {
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
    pagination::page(conn, "departments", offset, collect(rows)?)
}

pub fn suppliers(conn: &Connection, cursor: Option<&str>) -> AppResult<Page<Supplier>> {
    let offset = pagination::offset(conn, "suppliers", cursor)?;
    let mut statement = conn.prepare(
        "SELECT id, name, contact, phone, address, enabled, remark, created_at, updated_at
         FROM suppliers ORDER BY enabled DESC, name ASC, id ASC LIMIT ?1 OFFSET ?2",
    )?;
    let rows = statement.query_map(params![FETCH_SIZE, offset], |row| {
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
    pagination::page(conn, "suppliers", offset, collect(rows)?)
}

pub fn supplier_purchases(
    conn: &Connection,
    supplier_id: &str,
    cursor: Option<&str>,
) -> AppResult<Page<SupplierPurchaseRecord>> {
    let scope = format!("supplier-purchases:{supplier_id}");
    let offset = pagination::offset(conn, &scope, cursor)?;
    let mut statement = conn.prepare(
        "SELECT m.movement_date, d.document_no, i.code, i.name, i.spec, u.name,
                m.quantity, m.unit_price, m.amount, m.remark
         FROM stock_movements m JOIN master_items i ON i.id = m.item_id
         LEFT JOIN units u ON u.id = i.unit_id LEFT JOIN stock_documents d ON d.id = m.document_id
         WHERE m.direction = 'in' AND m.supplier_id = ?1
         ORDER BY m.movement_date DESC, m.created_at DESC, m.id DESC LIMIT ?2 OFFSET ?3",
    )?;
    let rows = statement.query_map(params![supplier_id, FETCH_SIZE, offset], |row| {
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
    pagination::page(conn, &scope, offset, collect(rows)?)
}

pub fn items(
    conn: &Connection,
    search: Option<String>,
    supplier_id: Option<String>,
    cursor: Option<&str>,
) -> AppResult<Page<Item>> {
    let search = search.unwrap_or_default();
    let like = format!("%{}%", search.trim());
    let supplier_id = supplier_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let scope = format!(
        "items:{}:{}",
        search.trim().to_lowercase(),
        supplier_id.as_deref().unwrap_or_default()
    );
    let offset = pagination::offset(conn, &scope, cursor)?;
    let mut statement = conn.prepare(
        "SELECT i.id, i.code, i.barcode, i.name, i.category_id, c.name, i.spec, i.unit_id, u.name,
                i.default_price, i.sale_price, i.supplier_id, s.name, i.warning_quantity,
                i.enabled, i.remark, i.created_at, i.updated_at
         FROM master_items i LEFT JOIN categories c ON c.id = i.category_id
         LEFT JOIN units u ON u.id = i.unit_id LEFT JOIN suppliers s ON s.id = i.supplier_id
         WHERE (?1 = '%%' OR i.code LIKE ?1 OR COALESCE(i.barcode, '') LIKE ?1
            OR i.name LIKE ?1 OR COALESCE(i.spec, '') LIKE ?1)
           AND (?2 IS NULL OR i.supplier_id = ?2)
         ORDER BY i.enabled DESC, i.code ASC, i.id ASC LIMIT ?3 OFFSET ?4",
    )?;
    let rows = statement.query_map(params![like, supplier_id, FETCH_SIZE, offset], |row| {
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
    pagination::page(conn, &scope, offset, collect(rows)?)
}

pub fn budget_rules(
    conn: &Connection,
    period_month: Option<String>,
    cursor: Option<&str>,
) -> AppResult<Page<BudgetRule>> {
    let scope = format!(
        "budget-rules:{}",
        period_month.as_deref().unwrap_or_default()
    );
    let offset = pagination::offset(conn, &scope, cursor)?;
    let mut statement = conn.prepare(
        "SELECT b.id, b.department_id, d.name, b.category_id, COALESCE(c.name, '全部分类'),
                b.period_month, b.amount_limit, COALESCE((SELECT SUM(m.amount)
                  FROM stock_movements m JOIN master_items i ON i.id = m.item_id
                  WHERE m.direction = 'out' AND m.department_id = b.department_id
                    AND (b.category_id IS NULL OR i.category_id = b.category_id)
                    AND m.movement_date >= b.period_month || '-01'
                    AND m.movement_date < date(b.period_month || '-01', '+1 month')), 0),
                b.enabled, b.created_at, b.updated_at
         FROM budget_rules b JOIN departments d ON d.id = b.department_id
         LEFT JOIN categories c ON c.id = b.category_id WHERE (?1 IS NULL OR b.period_month = ?1)
         ORDER BY b.period_month DESC, d.sort_order ASC, b.category_id IS NOT NULL ASC,
                  c.sort_order ASC, c.name ASC, b.id ASC LIMIT ?2 OFFSET ?3",
    )?;
    let rows = statement.query_map(params![period_month, FETCH_SIZE, offset], |row| {
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
    pagination::page(conn, &scope, offset, collect(rows)?)
}

fn collect<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> AppResult<Vec<T>> {
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}
