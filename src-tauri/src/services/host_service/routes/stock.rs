use super::super::*;

pub(crate) fn handle_stock_routes<S: Read + Write>(
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
        ("GET", path) if http_transport::route_matches(path, "/api/stock/documents") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let query = StockDocumentQuery {
                document_type: query_param(path, "documentType"),
                outbound_kind: query_param(path, "outboundKind"),
                month: query_param(path, "month"),
                department_id: query_param(path, "departmentId"),
                supplier_id: query_param(path, "supplierId"),
                item_id: query_param(path, "itemId"),
                handler: query_param(path, "handler"),
                search: query_param(path, "search"),
            };
            let cursor = query_param(path, "cursor");
            let response: Page<StockDocument> = db.with_conn(|conn| {
                let current = require_remote_permission(auth_request, conn, "view_reports")?;
                let mut query = query;
                query.department_id = remote_department_scope(&current)?.or(query.department_id);
                paginated_stock_repository::list_documents_page(conn, query, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/stock/document") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let document_id = query_param(path, "documentId")
                .ok_or_else(|| AppError::Validation("缺少单据 ID".to_string()))?;
            let response = db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "view_reports")?;
                stock_repository::get_stock_document_detail(conn, &document_id)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/stock/balances") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let query = StockBalanceQuery {
                search: query_param(path, "search"),
                category_id: query_param(path, "categoryId"),
                item_id: query_param(path, "itemId"),
                stock_status: query_param(path, "stockStatus"),
            };
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "view_reports")?;
                stock_repository::list_stock_balances_page(conn, query, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/stock/batches") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let item_id = query_param(path, "itemId")
                .ok_or_else(|| AppError::Validation("缺少物品 ID".to_string()))?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_permission(auth_request, conn, "view_reports")?;
                stock_repository::list_stock_batches_page(conn, &item_id, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/stock/movements") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let query = StockMovementQuery {
                search: query_param(path, "search"),
                item_id: query_param(path, "itemId"),
                department_id: query_param(path, "departmentId"),
                direction: query_param(path, "direction"),
                movement_type: query_param(path, "movementType"),
            };
            let cursor = query_param(path, "cursor");
            let response: Page<StockMovementRow> = db.with_conn(|conn| {
                let current = require_remote_permission(auth_request, conn, "view_reports")?;
                let mut query = query;
                query.department_id = remote_department_scope(&current)?.or(query.department_id);
                paginated_stock_repository::list_movements_page(conn, query, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/stock/document") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let mut request: SubmitStockDocumentRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("单据请求解析失败：{error}")))?;
            crate::services::stock_service::normalize_business_datetime(
                &mut request.business_date,
                "业务日期",
            )?;
            crate::services::stock_service::validate_document(&request)?;
            let response = db.with_conn_mut(|conn| {
                require_remote_permission(auth_request, conn, "write_stock")?;
                let allow_negative_stock =
                    crate::db::repository::get_setting(conn, "allow_negative_stock")?
                        .map(|value| value == "true")
                        .unwrap_or(false);
                stock_repository::submit_stock_document(conn, request, allow_negative_stock)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/stock/document/draft") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let mut request: SaveStockDocumentDraftRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("草稿请求解析失败：{error}")))?;
            crate::services::stock_service::normalize_business_datetime(
                &mut request.business_date,
                "业务日期",
            )?;
            crate::services::stock_service::validate_draft_document(&request)?;
            let response = db.with_conn_mut(|conn| {
                require_remote_permission(auth_request, conn, "write_stock")?;
                stock_repository::save_stock_document_draft(conn, request)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/stock/document/draft/confirm") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: ConfirmStockDocumentDraftRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("草稿确认请求解析失败：{error}")))?;
            crate::services::stock_service::validate_confirm_draft(&request)?;
            let response = db.with_conn_mut(|conn| {
                require_remote_permission(auth_request, conn, "write_stock")?;
                let allow_negative_stock =
                    crate::db::repository::get_setting(conn, "allow_negative_stock")?
                        .map(|value| value == "true")
                        .unwrap_or(false);
                stock_repository::confirm_stock_document_draft(conn, request, allow_negative_stock)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/stock/adjustment") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let mut request: SubmitAdjustmentRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("调整请求解析失败：{error}")))?;
            crate::services::stock_service::normalize_business_datetime(
                &mut request.business_date,
                "调整日期",
            )?;
            crate::services::stock_service::validate_adjustment(&request)?;
            let response = db.with_conn_mut(|conn| {
                require_remote_permission(auth_request, conn, "write_stock")?;
                stock_repository::submit_adjustment(conn, request)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/stock/void") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: VoidStockDocumentRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("作废请求解析失败：{error}")))?;
            crate::services::stock_service::validate_void_document(&request)?;
            let response = db.with_conn_mut(|conn| {
                require_remote_permission(auth_request, conn, "write_stock")?;
                stock_repository::void_stock_document(conn, request)
            })?;
            write_json(stream, 200, &response)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
