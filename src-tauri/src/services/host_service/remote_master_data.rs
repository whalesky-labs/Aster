use crate::app::state::AppState;
use crate::domain::master_data::{
    BudgetRule, Category, Department, Item, SaveBudgetRuleRequest, SaveCategoryRequest,
    SaveDepartmentRequest, SaveItemRequest, SaveSupplierRequest, SaveUnitRequest, Supplier, Unit,
};
use crate::domain::pagination::Page;
use crate::error::AppResult;

use super::{client_runtime_config, collect_remote_pages, http_post_json, SetEnabledRequest};
use crate::infrastructure::http_transport::{push_query_param, url_encode};
pub fn remote_list_categories(state: &AppState) -> AppResult<Vec<Category>> {
    let config = client_runtime_config(state)?;
    collect_remote_pages(&config, "/api/master/categories")
}

pub fn remote_save_category(state: &AppState, request: SaveCategoryRequest) -> AppResult<Category> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/master/category", &request)
}

pub fn remote_set_category_enabled(
    state: &AppState,
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
) -> AppResult<()> {
    let config = client_runtime_config(state)?;
    http_post_json(
        &config,
        "/api/master/category/enabled",
        &SetEnabledRequest {
            id,
            enabled,
            expected_updated_at,
        },
    )
}

pub fn remote_list_units(state: &AppState) -> AppResult<Vec<Unit>> {
    let config = client_runtime_config(state)?;
    collect_remote_pages(&config, "/api/master/units")
}

pub fn remote_save_unit(state: &AppState, request: SaveUnitRequest) -> AppResult<Unit> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/master/unit", &request)
}

pub fn remote_set_unit_enabled(
    state: &AppState,
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
) -> AppResult<()> {
    let config = client_runtime_config(state)?;
    http_post_json(
        &config,
        "/api/master/unit/enabled",
        &SetEnabledRequest {
            id,
            enabled,
            expected_updated_at,
        },
    )
}

pub fn remote_list_departments(state: &AppState) -> AppResult<Vec<Department>> {
    let config = client_runtime_config(state)?;
    collect_remote_pages(&config, "/api/master/departments")
}

pub fn remote_save_department(
    state: &AppState,
    request: SaveDepartmentRequest,
) -> AppResult<Department> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/master/department", &request)
}

pub fn remote_set_department_enabled(
    state: &AppState,
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
) -> AppResult<()> {
    let config = client_runtime_config(state)?;
    http_post_json(
        &config,
        "/api/master/department/enabled",
        &SetEnabledRequest {
            id,
            enabled,
            expected_updated_at,
        },
    )
}

pub fn remote_list_suppliers(state: &AppState) -> AppResult<Vec<Supplier>> {
    let config = client_runtime_config(state)?;
    collect_remote_pages(&config, "/api/master/suppliers")
}

pub fn remote_list_supplier_purchase_records(
    state: &AppState,
    supplier_id: String,
) -> AppResult<Vec<crate::domain::master_data::SupplierPurchaseRecord>> {
    let config = client_runtime_config(state)?;
    collect_remote_pages(
        &config,
        &format!(
            "/api/master/supplier/purchases?supplierId={}",
            url_encode(&supplier_id)
        ),
    )
}

pub fn remote_save_supplier(state: &AppState, request: SaveSupplierRequest) -> AppResult<Supplier> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/master/supplier", &request)
}

pub fn remote_set_supplier_enabled(
    state: &AppState,
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
) -> AppResult<()> {
    let config = client_runtime_config(state)?;
    http_post_json(
        &config,
        "/api/master/supplier/enabled",
        &SetEnabledRequest {
            id,
            enabled,
            expected_updated_at,
        },
    )
}

pub fn remote_list_items(
    state: &AppState,
    search: Option<String>,
    supplier_id: Option<String>,
) -> AppResult<Vec<Item>> {
    let config = client_runtime_config(state)?;
    let mut params = Vec::new();
    push_query_param(&mut params, "search", search);
    push_query_param(&mut params, "supplierId", supplier_id);
    let path = if params.is_empty() {
        "/api/master/items".to_string()
    } else {
        format!("/api/master/items?{}", params.join("&"))
    };
    collect_remote_pages(&config, &path)
}

pub fn remote_list_items_page(
    state: &AppState,
    search: Option<String>,
    supplier_id: Option<String>,
    cursor: Option<String>,
) -> AppResult<Page<Item>> {
    let config = client_runtime_config(state)?;
    let mut params = Vec::new();
    push_query_param(&mut params, "search", search);
    push_query_param(&mut params, "supplierId", supplier_id);
    let path = if params.is_empty() {
        "/api/master/items".to_string()
    } else {
        format!("/api/master/items?{}", params.join("&"))
    };
    super::http_get_json(
        &config,
        &crate::infrastructure::http_transport::page_path(&path, cursor.as_deref()),
    )
}

pub fn remote_save_item(state: &AppState, request: SaveItemRequest) -> AppResult<Item> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/master/item", &request)
}

pub fn remote_set_item_enabled(
    state: &AppState,
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
) -> AppResult<()> {
    let config = client_runtime_config(state)?;
    http_post_json(
        &config,
        "/api/master/item/enabled",
        &SetEnabledRequest {
            id,
            enabled,
            expected_updated_at,
        },
    )
}

pub fn remote_list_budget_rules(
    state: &AppState,
    period_month: Option<String>,
) -> AppResult<Vec<BudgetRule>> {
    let config = client_runtime_config(state)?;
    let path = if let Some(period_month) = period_month {
        format!(
            "/api/master/budget-rules?month={}",
            url_encode(&period_month)
        )
    } else {
        "/api/master/budget-rules".to_string()
    };
    collect_remote_pages(&config, &path)
}

pub fn remote_save_budget_rule(
    state: &AppState,
    request: SaveBudgetRuleRequest,
) -> AppResult<BudgetRule> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/master/budget-rule", &request)
}

pub fn remote_set_budget_rule_enabled(
    state: &AppState,
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
) -> AppResult<()> {
    let config = client_runtime_config(state)?;
    http_post_json(
        &config,
        "/api/master/budget-rule/enabled",
        &SetEnabledRequest {
            id,
            enabled,
            expected_updated_at,
        },
    )
}
