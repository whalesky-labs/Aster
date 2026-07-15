use super::super::*;

pub(crate) fn handle_master_data_routes<S: Read + Write>(
    stream: &mut S,
    context: RouteContext<'_>,
) -> AppResult<bool> {
    let RouteContext {
        runtime,
        db,
        method,
        path,
        body,
        request,
        auth_request,
        ..
    } = context;
    match (method, path) {
        ("GET", path) if http_transport::route_matches(path, "/api/master/categories") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "view_reports")?;
                master_data_repository::list_categories_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/category") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: SaveCategoryRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("分类请求解析失败：{error}")))?;
            crate::services::master_data_service::validate_category(&request)?;
            let response = db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "write_stock")?;
                let category = master_data_repository::save_category(conn, request)?;
                write_host_audit(
                    conn,
                    "save_category",
                    "category",
                    &category.id,
                    &category.name,
                )?;
                Ok(category)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/category/enabled") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: SetEnabledRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("分类状态请求解析失败：{error}")))?;
            db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "write_stock")?;
                master_data_repository::set_category_enabled(
                    conn,
                    &request.id,
                    request.enabled,
                    request.expected_updated_at.as_deref(),
                )?;
                write_host_audit(
                    conn,
                    "set_category_enabled",
                    "category",
                    &request.id,
                    &request.enabled.to_string(),
                )
            })?;
            write_json(stream, 200, &())?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/master/units") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "view_reports")?;
                master_data_repository::list_units_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/unit") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: SaveUnitRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("单位请求解析失败：{error}")))?;
            crate::services::master_data_service::validate_unit(&request)?;
            let response = db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "write_stock")?;
                let unit = master_data_repository::save_unit(conn, request)?;
                write_host_audit(conn, "save_unit", "unit", &unit.id, &unit.name)?;
                Ok(unit)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/unit/enabled") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: SetEnabledRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("单位状态请求解析失败：{error}")))?;
            db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "write_stock")?;
                master_data_repository::set_unit_enabled(
                    conn,
                    &request.id,
                    request.enabled,
                    request.expected_updated_at.as_deref(),
                )?;
                write_host_audit(
                    conn,
                    "set_unit_enabled",
                    "unit",
                    &request.id,
                    &request.enabled.to_string(),
                )
            })?;
            write_json(stream, 200, &())?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/master/departments") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "view_reports")?;
                master_data_repository::list_departments_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/department") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: SaveDepartmentRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("部门请求解析失败：{error}")))?;
            crate::services::master_data_service::validate_department(&request)?;
            let response = db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "write_stock")?;
                let department = master_data_repository::save_department(conn, request)?;
                write_host_audit(
                    conn,
                    "save_department",
                    "department",
                    &department.id,
                    &department.name,
                )?;
                Ok(department)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/department/enabled") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: SetEnabledRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("部门状态请求解析失败：{error}")))?;
            db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "write_stock")?;
                master_data_repository::set_department_enabled(
                    conn,
                    &request.id,
                    request.enabled,
                    request.expected_updated_at.as_deref(),
                )?;
                write_host_audit(
                    conn,
                    "set_department_enabled",
                    "department",
                    &request.id,
                    &request.enabled.to_string(),
                )
            })?;
            write_json(stream, 200, &())?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/master/suppliers") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "view_reports")?;
                master_data_repository::list_suppliers_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/master/supplier/purchases") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let supplier_id = query_param(path, "supplierId")
                .ok_or_else(|| AppError::Validation("缺少供应商 ID".to_string()))?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "view_reports")?;
                master_data_repository::list_supplier_purchase_records_page(
                    conn,
                    &supplier_id,
                    cursor.as_deref(),
                )
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/supplier") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: SaveSupplierRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("供应商请求解析失败：{error}")))?;
            crate::services::master_data_service::validate_supplier(&request)?;
            let response = db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "write_stock")?;
                let supplier = master_data_repository::save_supplier(conn, request)?;
                write_host_audit(
                    conn,
                    "save_supplier",
                    "supplier",
                    &supplier.id,
                    &supplier.name,
                )?;
                Ok(supplier)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/supplier/enabled") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: SetEnabledRequest = serde_json::from_str(body).map_err(|error| {
                AppError::Validation(format!("供应商状态请求解析失败：{error}"))
            })?;
            db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "write_stock")?;
                master_data_repository::set_supplier_enabled(
                    conn,
                    &request.id,
                    request.enabled,
                    request.expected_updated_at.as_deref(),
                )?;
                write_host_audit(
                    conn,
                    "set_supplier_enabled",
                    "supplier",
                    &request.id,
                    &request.enabled.to_string(),
                )
            })?;
            write_json(stream, 200, &())?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/master/items") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let search = query_param(path, "search");
            let supplier_id = query_param(path, "supplierId");
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "view_reports")?;
                master_data_repository::list_items_page(
                    conn,
                    search,
                    supplier_id,
                    cursor.as_deref(),
                )
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/item") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: SaveItemRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("物品请求解析失败：{error}")))?;
            crate::services::master_data_service::validate_item(&request)?;
            let response = db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "write_stock")?;
                let item = master_data_repository::save_item(conn, request)?;
                write_host_audit(conn, "save_item", "item", &item.id, &item.name)?;
                Ok(item)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/item/enabled") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: SetEnabledRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("物品状态请求解析失败：{error}")))?;
            db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "write_stock")?;
                master_data_repository::set_item_enabled(
                    conn,
                    &request.id,
                    request.enabled,
                    request.expected_updated_at.as_deref(),
                )?;
                write_host_audit(
                    conn,
                    "set_item_enabled",
                    "item",
                    &request.id,
                    &request.enabled.to_string(),
                )
            })?;
            write_json(stream, 200, &())?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/master/budget-rules") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let month = query_param(path, "month");
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_admin(auth_request, conn)?;
                master_data_repository::list_budget_rules_page(conn, month, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/budget-rule") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: SaveBudgetRuleRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("预算请求解析失败：{error}")))?;
            crate::services::master_data_service::validate_budget_rule(&request)?;
            let response = db.with_conn(|conn| {
                require_remote_admin(auth_request, conn)?;
                let rule = master_data_repository::save_budget_rule(conn, request)?;
                write_host_audit(
                    conn,
                    "save_budget_rule",
                    "budget_rule",
                    &rule.id,
                    &rule.period_month,
                )?;
                Ok(rule)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/master/budget-rule/enabled") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: SetEnabledRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("预算状态请求解析失败：{error}")))?;
            db.with_conn(|conn| {
                require_remote_admin(auth_request, conn)?;
                master_data_repository::set_budget_rule_enabled(
                    conn,
                    &request.id,
                    request.enabled,
                    request.expected_updated_at.as_deref(),
                )?;
                write_host_audit(
                    conn,
                    "set_budget_rule_enabled",
                    "budget_rule",
                    &request.id,
                    &request.enabled.to_string(),
                )
            })?;
            write_json(stream, 200, &())?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/reports/monthly") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let month = query_param(path, "month")
                .ok_or_else(|| AppError::Validation("报表月份不能为空".to_string()))?;
            let start_date = query_param(path, "startDate").map(|date| {
                if date.len() == 10 {
                    format!("{date} 00:00:00")
                } else {
                    date
                }
            });
            let end_date = query_param(path, "endDate").map(|date| {
                if date.len() == 10 {
                    format!("{date} 23:59:59")
                } else {
                    date
                }
            });
            let department_id = query_param(path, "departmentId");
            let category_id = query_param(path, "categoryId");
            let item_id = query_param(path, "itemId");
            let supplier_id = query_param(path, "supplierId");
            let section = query_param(path, "section")
                .ok_or_else(|| AppError::Validation("报表分页 section 不能为空".to_string()))?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                let current = require_remote_permission(auth_request, conn, "view_reports")?;
                let scoped_department_id = remote_department_scope(&current)?.or(department_id);
                report_repository::get_report_bundle_page(
                    conn,
                    &ReportQuery {
                        month,
                        start_date,
                        end_date,
                        department_id: scoped_department_id,
                        category_id,
                        item_id,
                        supplier_id,
                    },
                    &section,
                    cursor.as_deref(),
                )
            })?;
            write_json(stream, 200, &response)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
