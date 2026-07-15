use crate::app::state::AppState;
use crate::application::write_limits::validate_line_count;
use crate::db::stock_repository;
use crate::domain::pagination::Page;
use crate::domain::runtime::RuntimeMode;
use crate::domain::stock::{
    ConfirmStockDocumentDraftRequest, SaveStockDocumentDraftRequest, StockBalanceQuery,
    StockBalanceRow, StockBatchRow, StockDocument, StockDocumentDetail, StockDocumentQuery,
    StockMovementQuery, StockMovementRow, SubmitAdjustmentRequest, SubmitStockDocumentRequest,
    VoidStockDocumentRequest,
};
use crate::error::{AppError, AppResult};

pub(crate) use crate::domain::business_datetime::{
    normalize as normalize_business_datetime, validate as validate_business_datetime,
};

mod export;
pub use export::export_stock_balances;
pub(crate) use export::stock_balance_export_workbook_bytes;

pub fn submit_stock_document(
    state: &AppState,
    mut request: SubmitStockDocumentRequest,
) -> AppResult<StockDocumentDetail> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    normalize_business_datetime(&mut request.business_date, "业务日期")?;
    validate_document(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_submit_stock_document(state, request);
    }
    let allow_negative_stock = crate::services::status_service::allow_negative_stock(state)?;
    state.db.with_conn_mut(|conn| {
        stock_repository::submit_stock_document(conn, request, allow_negative_stock)
    })
}

pub fn save_stock_document_draft(
    state: &AppState,
    mut request: SaveStockDocumentDraftRequest,
) -> AppResult<StockDocumentDetail> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    normalize_business_datetime(&mut request.business_date, "业务日期")?;
    validate_draft_document(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_save_stock_document_draft(state, request);
    }
    state
        .db
        .with_conn_mut(|conn| stock_repository::save_stock_document_draft(conn, request))
}

pub fn confirm_stock_document_draft(
    state: &AppState,
    request: ConfirmStockDocumentDraftRequest,
) -> AppResult<StockDocumentDetail> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    validate_confirm_draft(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_confirm_stock_document_draft(state, request);
    }
    let allow_negative_stock = crate::services::status_service::allow_negative_stock(state)?;
    state.db.with_conn_mut(|conn| {
        stock_repository::confirm_stock_document_draft(conn, request, allow_negative_stock)
    })
}

pub fn submit_adjustment(
    state: &AppState,
    mut request: SubmitAdjustmentRequest,
) -> AppResult<StockDocumentDetail> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    normalize_business_datetime(&mut request.business_date, "调整日期")?;
    validate_adjustment(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_submit_adjustment(state, request);
    }
    state
        .db
        .with_conn_mut(|conn| stock_repository::submit_adjustment(conn, request))
}

pub fn void_stock_document(
    state: &AppState,
    request: VoidStockDocumentRequest,
) -> AppResult<StockDocumentDetail> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    validate_void_document(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_void_stock_document(state, request);
    }
    state
        .db
        .with_conn_mut(|conn| stock_repository::void_stock_document(conn, request))
}

pub fn list_stock_documents(
    state: &AppState,
    mut query: StockDocumentQuery,
) -> AppResult<Vec<StockDocument>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    query.department_id =
        crate::services::user_service::current_department_scope(state)?.or(query.department_id);
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_stock_documents(state, query);
    }
    state
        .db
        .with_conn(|conn| stock_repository::list_stock_documents(conn, query))
}

pub fn list_stock_documents_page(
    state: &AppState,
    mut query: StockDocumentQuery,
    cursor: Option<String>,
) -> AppResult<Page<StockDocument>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    query.department_id =
        crate::services::user_service::current_department_scope(state)?.or(query.department_id);
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_stock_documents_page(
            state, query, cursor,
        );
    }
    state.db.with_conn(|conn| {
        crate::db::paginated_stock_repository::list_documents_page(conn, query, cursor.as_deref())
    })
}

pub fn get_stock_document_detail(
    state: &AppState,
    document_id: String,
) -> AppResult<StockDocumentDetail> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_get_stock_document_detail(state, document_id);
    }
    state
        .db
        .with_conn(|conn| stock_repository::get_stock_document_detail(conn, &document_id))
}

pub fn list_stock_balances(
    state: &AppState,
    query: StockBalanceQuery,
) -> AppResult<Vec<StockBalanceRow>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_stock_balances(state, query);
    }
    state
        .db
        .with_conn(|conn| stock_repository::list_stock_balances(conn, query))
}

pub fn list_stock_balances_page(
    state: &AppState,
    query: StockBalanceQuery,
    cursor: Option<String>,
) -> AppResult<Page<StockBalanceRow>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_stock_balances_page(
            state, query, cursor,
        );
    }
    state.db.with_conn(|conn| {
        stock_repository::list_stock_balances_page(conn, query, cursor.as_deref())
    })
}

pub fn list_stock_batches(state: &AppState, item_id: String) -> AppResult<Vec<StockBatchRow>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_stock_batches(state, item_id);
    }
    state
        .db
        .with_conn(|conn| stock_repository::list_stock_batches(conn, &item_id))
}

pub fn list_stock_movements(
    state: &AppState,
    mut query: StockMovementQuery,
) -> AppResult<Vec<StockMovementRow>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    query.department_id =
        crate::services::user_service::current_department_scope(state)?.or(query.department_id);
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_stock_movements(state, query);
    }
    state
        .db
        .with_conn(|conn| stock_repository::list_stock_movements(conn, query))
}

pub fn list_stock_movements_page(
    state: &AppState,
    mut query: StockMovementQuery,
    cursor: Option<String>,
) -> AppResult<Page<StockMovementRow>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    query.department_id =
        crate::services::user_service::current_department_scope(state)?.or(query.department_id);
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_stock_movements_page(
            state, query, cursor,
        );
    }
    state.db.with_conn(|conn| {
        crate::db::paginated_stock_repository::list_movements_page(conn, query, cursor.as_deref())
    })
}

fn runtime_mode(state: &AppState) -> AppResult<RuntimeMode> {
    Ok(crate::services::status_service::get_runtime_config(state)?.mode)
}

pub(crate) fn validate_document(request: &SubmitStockDocumentRequest) -> AppResult<()> {
    if request.document_type != "inbound" && request.document_type != "outbound" {
        return Err(AppError::Validation("单据类型必须是入库或出库".to_string()));
    }
    validate_business_datetime(&request.business_date, "业务日期")?;
    let outbound_kind = request
        .outbound_kind
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("internal");
    if request.document_type == "outbound" && !matches!(outbound_kind, "internal" | "guest_sale") {
        return Err(AppError::Validation(
            "出库类型必须是内部领用或客人销售".to_string(),
        ));
    }
    if request.document_type == "outbound"
        && outbound_kind == "internal"
        && request
            .department_id
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        return Err(AppError::Validation("出库/领用必须选择部门".to_string()));
    }
    if request.lines.is_empty() {
        return Err(AppError::Validation("单据至少需要一行物品".to_string()));
    }
    validate_line_count(request.lines.len(), "单据")?;
    for line in &request.lines {
        if line.item_id.trim().is_empty() {
            return Err(AppError::Validation("单据行缺少物品".to_string()));
        }
        if line.quantity <= 0.0 {
            return Err(AppError::Validation("数量必须大于 0".to_string()));
        }
        if line.unit_price < 0.0 {
            return Err(AppError::Validation("单价不能小于 0".to_string()));
        }
        if line.amount.is_some_and(|amount| amount < 0.0) {
            return Err(AppError::Validation("金额不能小于 0".to_string()));
        }
    }
    Ok(())
}

pub(crate) fn validate_draft_document(request: &SaveStockDocumentDraftRequest) -> AppResult<()> {
    validate_document(&SubmitStockDocumentRequest {
        document_type: request.document_type.clone(),
        outbound_kind: request.outbound_kind.clone(),
        business_date: request.business_date.clone(),
        department_id: request.department_id.clone(),
        supplier_id: request.supplier_id.clone(),
        handler: request.handler.clone(),
        purpose: request.purpose.clone(),
        remark: request.remark.clone(),
        approval_request_id: request.approval_request_id.clone(),
        lines: request.lines.clone(),
    })
}

pub(crate) fn validate_confirm_draft(request: &ConfirmStockDocumentDraftRequest) -> AppResult<()> {
    if request.document_id.trim().is_empty() {
        return Err(AppError::Validation("草稿单据不能为空".to_string()));
    }
    Ok(())
}

pub(crate) fn validate_adjustment(request: &SubmitAdjustmentRequest) -> AppResult<()> {
    validate_business_datetime(&request.business_date, "调整日期")?;
    match request.adjustment_type.as_str() {
        "gain" | "loss" | "damage" | "correction" => {}
        other => return Err(AppError::Validation(format!("不支持的调整类型：{other}"))),
    }
    if request.reason.trim().is_empty() {
        return Err(AppError::Validation("调整原因不能为空".to_string()));
    }
    if request.lines.is_empty() {
        return Err(AppError::Validation("调整单至少需要一行物品".to_string()));
    }
    validate_line_count(request.lines.len(), "调整单")?;
    for line in &request.lines {
        if line.item_id.trim().is_empty() {
            return Err(AppError::Validation("调整行缺少物品".to_string()));
        }
        if line.direction != "in" && line.direction != "out" {
            return Err(AppError::Validation("调整方向必须是增加或减少".to_string()));
        }
        match (request.adjustment_type.as_str(), line.direction.as_str()) {
            ("gain", "out") => {
                return Err(AppError::Validation("盘盈调整只能增加库存".to_string()));
            }
            ("loss", "in") => {
                return Err(AppError::Validation("盘亏调整只能减少库存".to_string()));
            }
            ("damage", "in") => {
                return Err(AppError::Validation("损耗调整只能减少库存".to_string()));
            }
            _ => {}
        }
        if line.quantity <= 0.0 {
            return Err(AppError::Validation("调整数量必须大于 0".to_string()));
        }
        if line.unit_price < 0.0 {
            return Err(AppError::Validation("调整单价不能小于 0".to_string()));
        }
        if line.amount.is_some_and(|amount| amount < 0.0) {
            return Err(AppError::Validation("调整金额不能小于 0".to_string()));
        }
    }
    Ok(())
}

pub(crate) fn validate_void_document(request: &VoidStockDocumentRequest) -> AppResult<()> {
    if request.document_id.trim().is_empty() {
        return Err(AppError::Validation("单据不能为空".to_string()));
    }
    if request.reason.trim().is_empty() {
        return Err(AppError::Validation("作废原因不能为空".to_string()));
    }
    Ok(())
}

#[cfg(test)]
#[path = "stock_service/tests.rs"]
mod tests;
