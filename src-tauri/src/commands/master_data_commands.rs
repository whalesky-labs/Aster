use tauri::State;

use crate::app::state::AppState;
use crate::domain::imports::ExportItemsResult;
use crate::domain::master_data::{
    BudgetRule, Category, Department, Item, SaveBudgetRuleRequest, SaveCategoryRequest,
    SaveDepartmentRequest, SaveItemRequest, SaveSupplierRequest, SaveUnitRequest, Supplier,
    SupplierPurchaseRecord, Unit,
};
use crate::error::AppResult;
use crate::services::master_data_service;

#[tauri::command]
pub fn list_categories(state: State<'_, AppState>) -> AppResult<Vec<Category>> {
    master_data_service::list_categories(&state)
}

#[tauri::command]
pub fn save_category(
    request: SaveCategoryRequest,
    state: State<'_, AppState>,
) -> AppResult<Category> {
    master_data_service::save_category(&state, request)
}

#[tauri::command]
pub fn set_category_enabled(
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    master_data_service::set_category_enabled(&state, id, enabled, expected_updated_at)
}

#[tauri::command]
pub fn list_units(state: State<'_, AppState>) -> AppResult<Vec<Unit>> {
    master_data_service::list_units(&state)
}

#[tauri::command]
pub fn save_unit(request: SaveUnitRequest, state: State<'_, AppState>) -> AppResult<Unit> {
    master_data_service::save_unit(&state, request)
}

#[tauri::command]
pub fn set_unit_enabled(
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    master_data_service::set_unit_enabled(&state, id, enabled, expected_updated_at)
}

#[tauri::command]
pub fn list_departments(state: State<'_, AppState>) -> AppResult<Vec<Department>> {
    master_data_service::list_departments(&state)
}

#[tauri::command]
pub fn save_department(
    request: SaveDepartmentRequest,
    state: State<'_, AppState>,
) -> AppResult<Department> {
    master_data_service::save_department(&state, request)
}

#[tauri::command]
pub fn set_department_enabled(
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    master_data_service::set_department_enabled(&state, id, enabled, expected_updated_at)
}

#[tauri::command]
pub fn list_suppliers(state: State<'_, AppState>) -> AppResult<Vec<Supplier>> {
    master_data_service::list_suppliers(&state)
}

#[tauri::command]
pub fn save_supplier(
    request: SaveSupplierRequest,
    state: State<'_, AppState>,
) -> AppResult<Supplier> {
    master_data_service::save_supplier(&state, request)
}

#[tauri::command]
pub fn set_supplier_enabled(
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    master_data_service::set_supplier_enabled(&state, id, enabled, expected_updated_at)
}

#[tauri::command]
pub fn list_supplier_purchase_records(
    supplier_id: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<SupplierPurchaseRecord>> {
    master_data_service::list_supplier_purchase_records(&state, supplier_id)
}

#[tauri::command]
pub fn list_items(
    search: Option<String>,
    supplier_id: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<Vec<Item>> {
    master_data_service::list_items(&state, search, supplier_id)
}

#[tauri::command]
pub fn list_items_page(
    search: Option<String>,
    supplier_id: Option<String>,
    cursor: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::pagination::Page<Item>> {
    master_data_service::list_items_page(&state, search, supplier_id, cursor)
}

#[tauri::command]
pub fn export_items(
    search: Option<String>,
    supplier_id: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<ExportItemsResult> {
    master_data_service::export_items(&state, search, supplier_id)
}

#[tauri::command]
pub fn save_item(request: SaveItemRequest, state: State<'_, AppState>) -> AppResult<Item> {
    master_data_service::save_item(&state, request)
}

#[tauri::command]
pub fn set_item_enabled(
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    master_data_service::set_item_enabled(&state, id, enabled, expected_updated_at)
}

#[tauri::command]
pub fn list_budget_rules(
    period_month: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<Vec<BudgetRule>> {
    master_data_service::list_budget_rules(&state, period_month)
}

#[tauri::command]
pub fn save_budget_rule(
    request: SaveBudgetRuleRequest,
    state: State<'_, AppState>,
) -> AppResult<BudgetRule> {
    master_data_service::save_budget_rule(&state, request)
}

#[tauri::command]
pub fn set_budget_rule_enabled(
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    master_data_service::set_budget_rule_enabled(&state, id, enabled, expected_updated_at)
}
