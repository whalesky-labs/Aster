use crate::app::state::AppState;
use crate::domain::stock::{
    ConfirmStockDocumentDraftRequest, SaveStockDocumentDraftRequest, StockBalanceQuery,
    StockBalanceRow, StockBatchRow, StockDocument, StockDocumentQuery, StockMovementQuery,
    StockMovementRow, SubmitAdjustmentRequest, SubmitStockDocumentRequest,
    VoidStockDocumentRequest,
};
use crate::error::AppResult;

use super::{client_runtime_config, collect_remote_pages, http_get_json, http_post_json};
use crate::infrastructure::http_transport::{page_path, push_query_param, url_encode};
pub fn remote_list_stock_documents(
    state: &AppState,
    query: StockDocumentQuery,
) -> AppResult<Vec<StockDocument>> {
    let config = client_runtime_config(state)?;
    let mut params = Vec::new();
    push_query_param(&mut params, "documentType", query.document_type);
    push_query_param(&mut params, "outboundKind", query.outbound_kind);
    push_query_param(&mut params, "month", query.month);
    push_query_param(&mut params, "departmentId", query.department_id);
    push_query_param(&mut params, "supplierId", query.supplier_id);
    push_query_param(&mut params, "itemId", query.item_id);
    push_query_param(&mut params, "handler", query.handler);
    push_query_param(&mut params, "search", query.search);
    let path = if params.is_empty() {
        "/api/stock/documents".to_string()
    } else {
        format!("/api/stock/documents?{}", params.join("&"))
    };
    crate::application::remote_pagination::collect_all(|cursor| {
        http_get_json(&config, &page_path(&path, cursor))
    })
}

pub fn remote_get_stock_document_detail(
    state: &AppState,
    document_id: String,
) -> AppResult<crate::domain::stock::StockDocumentDetail> {
    let config = client_runtime_config(state)?;
    http_get_json(
        &config,
        &format!(
            "/api/stock/document?documentId={}",
            url_encode(&document_id)
        ),
    )
}

pub fn remote_list_stock_balances(
    state: &AppState,
    query: StockBalanceQuery,
) -> AppResult<Vec<StockBalanceRow>> {
    let config = client_runtime_config(state)?;
    let mut params = Vec::new();
    push_query_param(&mut params, "search", query.search);
    push_query_param(&mut params, "categoryId", query.category_id);
    push_query_param(&mut params, "itemId", query.item_id);
    push_query_param(&mut params, "stockStatus", query.stock_status);
    let path = if params.is_empty() {
        "/api/stock/balances".to_string()
    } else {
        format!("/api/stock/balances?{}", params.join("&"))
    };
    collect_remote_pages(&config, &path)
}

pub fn remote_list_stock_batches(
    state: &AppState,
    item_id: String,
) -> AppResult<Vec<StockBatchRow>> {
    let config = client_runtime_config(state)?;
    collect_remote_pages(
        &config,
        &format!("/api/stock/batches?itemId={}", url_encode(&item_id)),
    )
}

pub fn remote_list_stock_movements(
    state: &AppState,
    query: StockMovementQuery,
) -> AppResult<Vec<StockMovementRow>> {
    let config = client_runtime_config(state)?;
    let mut params = Vec::new();
    push_query_param(&mut params, "search", query.search);
    push_query_param(&mut params, "itemId", query.item_id);
    push_query_param(&mut params, "departmentId", query.department_id);
    push_query_param(&mut params, "direction", query.direction);
    push_query_param(&mut params, "movementType", query.movement_type);
    let path = if params.is_empty() {
        "/api/stock/movements".to_string()
    } else {
        format!("/api/stock/movements?{}", params.join("&"))
    };
    crate::application::remote_pagination::collect_all(|cursor| {
        http_get_json(&config, &page_path(&path, cursor))
    })
}

pub fn remote_submit_stock_document(
    state: &AppState,
    request: SubmitStockDocumentRequest,
) -> AppResult<crate::domain::stock::StockDocumentDetail> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/stock/document", &request)
}

pub fn remote_save_stock_document_draft(
    state: &AppState,
    request: SaveStockDocumentDraftRequest,
) -> AppResult<crate::domain::stock::StockDocumentDetail> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/stock/document/draft", &request)
}

pub fn remote_confirm_stock_document_draft(
    state: &AppState,
    request: ConfirmStockDocumentDraftRequest,
) -> AppResult<crate::domain::stock::StockDocumentDetail> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/stock/document/draft/confirm", &request)
}

pub fn remote_submit_adjustment(
    state: &AppState,
    request: SubmitAdjustmentRequest,
) -> AppResult<crate::domain::stock::StockDocumentDetail> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/stock/adjustment", &request)
}

pub fn remote_void_stock_document(
    state: &AppState,
    request: VoidStockDocumentRequest,
) -> AppResult<crate::domain::stock::StockDocumentDetail> {
    let config = client_runtime_config(state)?;
    http_post_json(&config, "/api/stock/void", &request)
}
