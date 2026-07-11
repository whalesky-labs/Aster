use super::super::*;

pub(crate) fn handle_operation_routes<S: Read + Write>(
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
        ("GET", path) if http_transport::route_matches(path, "/api/stocktakes") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "view_reports")?;
                stocktake_repository::list_stocktakes_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/stocktakes") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let mut request: CreateStocktakeRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("盘点创建请求解析失败：{error}")))?;
            crate::services::stock_service::normalize_business_datetime(
                &mut request.business_date,
                "盘点日期",
            )?;
            crate::services::stocktake_service::validate_create_request(&request)?;
            let response = db.with_conn_mut(|conn| {
                require_remote_permission(auth_request, conn, "write_stock")?;
                stocktake_repository::create_stocktake(conn, request)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/stocktake") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let stocktake_id = query_param(path, "stocktakeId")
                .ok_or_else(|| AppError::Validation("盘点单不能为空".to_string()))?;
            let response = db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "view_reports")?;
                stocktake_repository::get_stocktake_detail(conn, &stocktake_id)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/stocktake/counts") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: UpdateStocktakeCountsRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("盘点录入请求解析失败：{error}")))?;
            crate::services::stocktake_service::validate_update_counts(&request)?;
            let response = db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "write_stock")?;
                stocktake_repository::update_stocktake_counts(conn, request)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/stocktake/confirm") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: ConfirmStocktakeRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("盘点确认请求解析失败：{error}")))?;
            crate::services::stocktake_service::validate_confirm_request(&request)?;
            let response = db.with_conn_mut(|conn| {
                require_remote_permission(auth_request, conn, "write_stock")?;
                stocktake_repository::confirm_stocktake(conn, request)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/approvals") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_admin(auth_request, conn)?;
                approval_repository::list_approval_requests_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/approval") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: CreateApprovalRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("审批请求解析失败：{error}")))?;
            let response = db.with_conn(|conn| {
                let current = require_remote_permission(auth_request, conn, "write_stock")?;
                crate::services::approval_service::create_approval_request_on_conn(
                    conn,
                    request,
                    Some(current.id),
                    "client",
                )
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/approval/decision") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: DecideApprovalRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("审批决定解析失败：{error}")))?;
            let response = db.with_conn(|conn| {
                let current = require_remote_admin(auth_request, conn)?;
                crate::services::approval_service::decide_approval_request_on_conn(
                    conn,
                    request,
                    Some(current.id),
                    "client",
                )
            })?;
            write_json(stream, 200, &response)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
