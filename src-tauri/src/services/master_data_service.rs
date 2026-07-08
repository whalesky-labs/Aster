use std::fs;

use chrono::Local;
use rusqlite::params;
use rust_xlsxwriter::{Format, Workbook, XlsxError};

use crate::app::state::AppState;
use crate::db::master_data_repository;
use crate::domain::imports::ExportItemsResult;
use crate::domain::master_data::{
    BudgetRule, Category, Department, Item, SaveBudgetRuleRequest, SaveCategoryRequest,
    SaveDepartmentRequest, SaveItemRequest, SaveSupplierRequest, SaveUnitRequest, Supplier, Unit,
};
use crate::domain::runtime::RuntimeMode;
use crate::error::{AppError, AppResult};

pub fn list_categories(state: &AppState) -> AppResult<Vec<Category>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_categories(state);
    }
    state.db.with_conn(master_data_repository::list_categories)
}

pub fn save_category(state: &AppState, request: SaveCategoryRequest) -> AppResult<Category> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    validate_category(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_save_category(state, request);
    }
    state.db.with_conn(|conn| {
        let category = master_data_repository::save_category(conn, request)?;
        write_audit(
            conn,
            "save_category",
            "category",
            &category.id,
            &category.name,
        )?;
        Ok(category)
    })
}

pub fn set_category_enabled(
    state: &AppState,
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
) -> AppResult<()> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_set_category_enabled(
            state,
            id,
            enabled,
            expected_updated_at,
        );
    }
    state.db.with_conn(|conn| {
        master_data_repository::set_category_enabled(
            conn,
            &id,
            enabled,
            expected_updated_at.as_deref(),
        )?;
        write_audit(
            conn,
            "set_category_enabled",
            "category",
            &id,
            &enabled.to_string(),
        )
    })
}

pub fn list_units(state: &AppState) -> AppResult<Vec<Unit>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_units(state);
    }
    state.db.with_conn(master_data_repository::list_units)
}

pub fn save_unit(state: &AppState, request: SaveUnitRequest) -> AppResult<Unit> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    validate_unit(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_save_unit(state, request);
    }
    state.db.with_conn(|conn| {
        let unit = master_data_repository::save_unit(conn, request)?;
        write_audit(conn, "save_unit", "unit", &unit.id, &unit.name)?;
        Ok(unit)
    })
}

pub fn set_unit_enabled(
    state: &AppState,
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
) -> AppResult<()> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_set_unit_enabled(
            state,
            id,
            enabled,
            expected_updated_at,
        );
    }
    state.db.with_conn(|conn| {
        master_data_repository::set_unit_enabled(
            conn,
            &id,
            enabled,
            expected_updated_at.as_deref(),
        )?;
        write_audit(conn, "set_unit_enabled", "unit", &id, &enabled.to_string())
    })
}

pub fn list_departments(state: &AppState) -> AppResult<Vec<Department>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_departments(state);
    }
    state.db.with_conn(master_data_repository::list_departments)
}

pub fn save_department(state: &AppState, request: SaveDepartmentRequest) -> AppResult<Department> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    validate_department(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_save_department(state, request);
    }
    state.db.with_conn(|conn| {
        let department = master_data_repository::save_department(conn, request)?;
        write_audit(
            conn,
            "save_department",
            "department",
            &department.id,
            &department.name,
        )?;
        Ok(department)
    })
}

pub fn set_department_enabled(
    state: &AppState,
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
) -> AppResult<()> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_set_department_enabled(
            state,
            id,
            enabled,
            expected_updated_at,
        );
    }
    state.db.with_conn(|conn| {
        master_data_repository::set_department_enabled(
            conn,
            &id,
            enabled,
            expected_updated_at.as_deref(),
        )?;
        write_audit(
            conn,
            "set_department_enabled",
            "department",
            &id,
            &enabled.to_string(),
        )
    })
}

pub fn list_suppliers(state: &AppState) -> AppResult<Vec<Supplier>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_suppliers(state);
    }
    state.db.with_conn(master_data_repository::list_suppliers)
}

pub fn save_supplier(state: &AppState, request: SaveSupplierRequest) -> AppResult<Supplier> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    validate_supplier(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_save_supplier(state, request);
    }
    state.db.with_conn(|conn| {
        let supplier = master_data_repository::save_supplier(conn, request)?;
        write_audit(
            conn,
            "save_supplier",
            "supplier",
            &supplier.id,
            &supplier.name,
        )?;
        Ok(supplier)
    })
}

pub fn set_supplier_enabled(
    state: &AppState,
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
) -> AppResult<()> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_set_supplier_enabled(
            state,
            id,
            enabled,
            expected_updated_at,
        );
    }
    state.db.with_conn(|conn| {
        master_data_repository::set_supplier_enabled(
            conn,
            &id,
            enabled,
            expected_updated_at.as_deref(),
        )?;
        write_audit(
            conn,
            "set_supplier_enabled",
            "supplier",
            &id,
            &enabled.to_string(),
        )
    })
}

pub fn list_supplier_purchase_records(
    state: &AppState,
    supplier_id: String,
) -> AppResult<Vec<crate::domain::master_data::SupplierPurchaseRecord>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    if supplier_id.trim().is_empty() {
        return Err(AppError::Validation("供应商不能为空".to_string()));
    }
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_supplier_purchase_records(
            state,
            supplier_id,
        );
    }
    state.db.with_conn(|conn| {
        master_data_repository::list_supplier_purchase_records(conn, &supplier_id)
    })
}

pub fn list_items(state: &AppState, search: Option<String>) -> AppResult<Vec<Item>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_items(state, search);
    }
    state
        .db
        .with_conn(|conn| master_data_repository::list_items(conn, search))
}

pub fn export_items(state: &AppState, search: Option<String>) -> AppResult<ExportItemsResult> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    let export_dir = crate::services::status_service::effective_export_dir(state)?;
    fs::create_dir_all(&export_dir)?;
    let file_name = format!(
        "Aster-物品档案-{}.xlsx",
        Local::now().format("%Y%m%d%H%M%S")
    );
    let path = export_dir.join(file_name);
    let items = if runtime_mode(state)? == RuntimeMode::Client {
        crate::services::host_service::remote_list_items(state, search)?
    } else {
        state
            .db
            .with_conn(|conn| master_data_repository::list_items(conn, search))?
    };
    write_items_workbook(&path, &items)?;
    if runtime_mode(state)? != RuntimeMode::Client {
        state.db.with_conn(|conn| {
            write_audit(
                conn,
                "export_items",
                "item",
                "items",
                &format!("导出物品档案：{} 种", items.len()),
            )
        })?;
    }
    Ok(ExportItemsResult {
        path: path.display().to_string(),
    })
}

pub fn save_item(state: &AppState, request: SaveItemRequest) -> AppResult<Item> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    validate_item(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_save_item(state, request);
    }
    state.db.with_conn(|conn| {
        let item = master_data_repository::save_item(conn, request)?;
        write_audit(conn, "save_item", "item", &item.id, &item.name)?;
        Ok(item)
    })
}

fn write_items_workbook(path: &std::path::Path, items: &[Item]) -> AppResult<()> {
    let mut workbook = Workbook::new();
    let header = Format::new().set_bold().set_background_color("#DDEBF7");
    let money = Format::new().set_num_format("#,##0.00");
    let number = Format::new().set_num_format("#,##0.00");
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("物品档案").map_err(xlsx_error)?;
    let headers = [
        "物品编码",
        "条码",
        "物品名称",
        "分类",
        "规格",
        "单位",
        "参考进价",
        "参考售价",
        "供应商",
        "预警库存",
        "状态",
        "备注",
        "创建时间",
        "更新时间",
    ];
    for (col, title) in headers.iter().enumerate() {
        worksheet
            .write_string_with_format(0, col as u16, *title, &header)
            .map_err(xlsx_error)?;
    }
    for (index, item) in items.iter().enumerate() {
        let row = (index + 1) as u32;
        worksheet
            .write_string(row, 0, &item.code)
            .map_err(xlsx_error)?;
        write_optional_string(worksheet, row, 1, item.barcode.as_deref())?;
        worksheet
            .write_string(row, 2, &item.name)
            .map_err(xlsx_error)?;
        write_optional_string(worksheet, row, 3, item.category_name.as_deref())?;
        write_optional_string(worksheet, row, 4, item.spec.as_deref())?;
        write_optional_string(worksheet, row, 5, item.unit_name.as_deref())?;
        worksheet
            .write_number_with_format(row, 6, item.default_price, &money)
            .map_err(xlsx_error)?;
        worksheet
            .write_number_with_format(row, 7, item.sale_price, &money)
            .map_err(xlsx_error)?;
        write_optional_string(worksheet, row, 8, item.supplier_name.as_deref())?;
        worksheet
            .write_number_with_format(row, 9, item.warning_quantity, &number)
            .map_err(xlsx_error)?;
        worksheet
            .write_string(row, 10, if item.enabled { "启用" } else { "停用" })
            .map_err(xlsx_error)?;
        write_optional_string(worksheet, row, 11, item.remark.as_deref())?;
        worksheet
            .write_string(row, 12, &item.created_at)
            .map_err(xlsx_error)?;
        worksheet
            .write_string(row, 13, &item.updated_at)
            .map_err(xlsx_error)?;
    }
    for (col, width) in [16, 18, 24, 16, 18, 10, 12, 12, 18, 12, 10, 28, 22, 22]
        .iter()
        .enumerate()
    {
        worksheet
            .set_column_width(col as u16, *width)
            .map_err(xlsx_error)?;
    }
    workbook.save(path).map_err(|error| {
        AppError::Validation(format!("物品档案导出失败：{error}"))
    })
}

fn write_optional_string(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    col: u16,
    value: Option<&str>,
) -> AppResult<()> {
    worksheet
        .write_string(row, col, value.unwrap_or(""))
        .map(|_| ())
        .map_err(xlsx_error)
}

fn xlsx_error(error: XlsxError) -> AppError {
    AppError::Validation(format!("物品档案导出失败：{error}"))
}

pub(crate) fn validate_category(request: &SaveCategoryRequest) -> AppResult<()> {
    require_text("分类名称", &request.name)?;
    if request
        .id
        .as_deref()
        .zip(request.parent_id.as_deref())
        .is_some_and(|(id, parent_id)| id == parent_id)
    {
        return Err(AppError::Validation("上级分类不能选择自己".to_string()));
    }
    Ok(())
}

pub(crate) fn validate_unit(request: &SaveUnitRequest) -> AppResult<()> {
    require_text("单位名称", &request.name)
}

pub(crate) fn validate_department(request: &SaveDepartmentRequest) -> AppResult<()> {
    require_text("部门编码", &request.code)?;
    require_text("部门名称", &request.name)
}

pub(crate) fn validate_supplier(request: &SaveSupplierRequest) -> AppResult<()> {
    require_text("供应商名称", &request.name)
}

pub(crate) fn validate_item(request: &SaveItemRequest) -> AppResult<()> {
    if request.id.is_some() {
        require_text("物品编码", request.code.as_deref().unwrap_or_default())?;
    }
    require_text("物品名称", &request.name)?;
    if request.default_price < 0.0 {
        return Err(AppError::Validation("参考进价不能小于 0".to_string()));
    }
    if request.sale_price < 0.0 {
        return Err(AppError::Validation("参考售价不能小于 0".to_string()));
    }
    if request.warning_quantity < 0.0 {
        return Err(AppError::Validation("库存预警线不能小于 0".to_string()));
    }
    Ok(())
}

pub fn set_item_enabled(
    state: &AppState,
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
) -> AppResult<()> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_set_item_enabled(
            state,
            id,
            enabled,
            expected_updated_at,
        );
    }
    state.db.with_conn(|conn| {
        master_data_repository::set_item_enabled(
            conn,
            &id,
            enabled,
            expected_updated_at.as_deref(),
        )?;
        write_audit(conn, "set_item_enabled", "item", &id, &enabled.to_string())
    })
}

pub fn list_budget_rules(
    state: &AppState,
    period_month: Option<String>,
) -> AppResult<Vec<BudgetRule>> {
    crate::services::user_service::require_admin(state)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_budget_rules(state, period_month);
    }
    state
        .db
        .with_conn(|conn| master_data_repository::list_budget_rules(conn, period_month))
}

pub fn save_budget_rule(state: &AppState, request: SaveBudgetRuleRequest) -> AppResult<BudgetRule> {
    crate::services::user_service::require_admin(state)?;
    validate_budget_rule(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_save_budget_rule(state, request);
    }
    state.db.with_conn(|conn| {
        let rule = master_data_repository::save_budget_rule(conn, request)?;
        write_audit(
            conn,
            "save_budget_rule",
            "budget_rule",
            &rule.id,
            &rule.period_month,
        )?;
        Ok(rule)
    })
}

pub fn set_budget_rule_enabled(
    state: &AppState,
    id: String,
    enabled: bool,
    expected_updated_at: Option<String>,
) -> AppResult<()> {
    crate::services::user_service::require_admin(state)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_set_budget_rule_enabled(
            state,
            id,
            enabled,
            expected_updated_at,
        );
    }
    state.db.with_conn(|conn| {
        master_data_repository::set_budget_rule_enabled(
            conn,
            &id,
            enabled,
            expected_updated_at.as_deref(),
        )?;
        write_audit(
            conn,
            "set_budget_rule_enabled",
            "budget_rule",
            &id,
            &enabled.to_string(),
        )
    })
}

fn runtime_mode(state: &AppState) -> AppResult<RuntimeMode> {
    Ok(crate::services::status_service::get_runtime_config(state)?.mode)
}

pub(crate) fn validate_budget_rule(request: &SaveBudgetRuleRequest) -> AppResult<()> {
    require_text("部门", &request.department_id)?;
    require_text("预算月份", &request.period_month)?;
    if request.amount_limit < 0.0 {
        return Err(AppError::Validation("预算金额不能小于 0".to_string()));
    }
    Ok(())
}

fn require_text(label: &str, value: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        Err(AppError::Validation(format!("{label}不能为空")))
    } else {
        Ok(())
    }
}

fn write_audit(
    conn: &rusqlite::Connection,
    action: &str,
    entity_type: &str,
    entity_id: &str,
    summary: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            uuid::Uuid::new_v4().to_string(),
            action,
            entity_type,
            entity_id,
            summary,
            "system"
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::app::paths::AppPaths;
    use crate::app::state::AppState;
    use crate::db::connection::Db;

    use super::*;

    fn test_state() -> AppState {
        let dir = tempfile::tempdir().expect("temp dir").keep();
        let paths = AppPaths {
            data_dir: dir.to_path_buf(),
            database_path: dir.join("aster.sqlite"),
            backup_dir: dir.join("backups"),
            export_dir: dir.join("exports"),
            import_report_dir: dir.join("import-reports"),
        };
        std::fs::create_dir_all(&paths.backup_dir).unwrap();
        std::fs::create_dir_all(&paths.export_dir).unwrap();
        std::fs::create_dir_all(&paths.import_report_dir).unwrap();
        AppState {
            db: Db::initialize(&paths).unwrap(),
            paths,
            session: Arc::new(Mutex::new(None)),
            host_service: Arc::new(Mutex::new(
                crate::services::host_service::HostServiceRuntime::default(),
            )),
        }
    }

    #[test]
    fn list_items_requires_view_reports_permission() {
        let state = test_state();
        let error = list_items(&state, None).unwrap_err();

        assert!(error.to_string().contains("请先登录"));
    }
}
