use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::db::{paginated_master_data_repository, pagination};
use crate::domain::master_data::{
    BudgetRule, Category, Department, Item, SaveBudgetRuleRequest, SaveCategoryRequest,
    SaveDepartmentRequest, SaveItemRequest, SaveSupplierRequest, SaveUnitRequest, Supplier,
    SupplierPurchaseRecord, Unit,
};
use crate::domain::pagination::Page;
use crate::error::{AppError, AppResult};

pub fn list_categories(conn: &Connection) -> AppResult<Vec<Category>> {
    pagination::collect_all(|cursor| list_categories_page(conn, cursor))
}

pub fn list_categories_page(conn: &Connection, cursor: Option<&str>) -> AppResult<Page<Category>> {
    paginated_master_data_repository::categories(conn, cursor)
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
    pagination::collect_all(|cursor| list_units_page(conn, cursor))
}

pub fn list_units_page(conn: &Connection, cursor: Option<&str>) -> AppResult<Page<Unit>> {
    paginated_master_data_repository::units(conn, cursor)
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
    pagination::collect_all(|cursor| list_departments_page(conn, cursor))
}

pub fn list_departments_page(
    conn: &Connection,
    cursor: Option<&str>,
) -> AppResult<Page<Department>> {
    paginated_master_data_repository::departments(conn, cursor)
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
    pagination::collect_all(|cursor| list_suppliers_page(conn, cursor))
}

pub fn list_suppliers_page(conn: &Connection, cursor: Option<&str>) -> AppResult<Page<Supplier>> {
    paginated_master_data_repository::suppliers(conn, cursor)
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
    pagination::collect_all(|cursor| list_supplier_purchase_records_page(conn, supplier_id, cursor))
}

pub fn list_supplier_purchase_records_page(
    conn: &Connection,
    supplier_id: &str,
    cursor: Option<&str>,
) -> AppResult<Page<SupplierPurchaseRecord>> {
    paginated_master_data_repository::supplier_purchases(conn, supplier_id, cursor)
}

pub fn list_items(
    conn: &Connection,
    search: Option<String>,
    supplier_id: Option<String>,
) -> AppResult<Vec<Item>> {
    pagination::collect_all(|cursor| {
        list_items_page(conn, search.clone(), supplier_id.clone(), cursor)
    })
}

pub fn list_items_page(
    conn: &Connection,
    search: Option<String>,
    supplier_id: Option<String>,
    cursor: Option<&str>,
) -> AppResult<Page<Item>> {
    paginated_master_data_repository::items(conn, search, supplier_id, cursor)
}

include!("master_data_repository/items.rs");

#[cfg(test)]
#[path = "master_data_repository/tests.rs"]
mod tests;
