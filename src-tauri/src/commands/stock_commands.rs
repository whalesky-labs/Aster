use tauri::State;

use crate::app::state::AppState;
use crate::domain::stock::{
    ConfirmStockDocumentDraftRequest, SaveStockDocumentDraftRequest, StockBalanceQuery,
    StockBalanceRow, StockDocument, StockDocumentDetail, StockDocumentQuery, StockMovementQuery,
    StockMovementRow, SubmitAdjustmentRequest, SubmitStockDocumentRequest,
    VoidStockDocumentRequest,
};
use crate::error::AppResult;
use crate::services::stock_service;

#[tauri::command]
pub fn submit_stock_document(
    request: SubmitStockDocumentRequest,
    state: State<'_, AppState>,
) -> AppResult<StockDocumentDetail> {
    stock_service::submit_stock_document(&state, request)
}

#[tauri::command]
pub fn save_stock_document_draft(
    request: SaveStockDocumentDraftRequest,
    state: State<'_, AppState>,
) -> AppResult<StockDocumentDetail> {
    stock_service::save_stock_document_draft(&state, request)
}

#[tauri::command]
pub fn confirm_stock_document_draft(
    request: ConfirmStockDocumentDraftRequest,
    state: State<'_, AppState>,
) -> AppResult<StockDocumentDetail> {
    stock_service::confirm_stock_document_draft(&state, request)
}

#[tauri::command]
pub fn submit_adjustment(
    request: SubmitAdjustmentRequest,
    state: State<'_, AppState>,
) -> AppResult<StockDocumentDetail> {
    stock_service::submit_adjustment(&state, request)
}

#[tauri::command]
pub fn void_stock_document(
    request: VoidStockDocumentRequest,
    state: State<'_, AppState>,
) -> AppResult<StockDocumentDetail> {
    stock_service::void_stock_document(&state, request)
}

#[tauri::command]
pub fn list_stock_documents(
    document_type: Option<String>,
    query: Option<StockDocumentQuery>,
    state: State<'_, AppState>,
) -> AppResult<Vec<StockDocument>> {
    let mut query = query.unwrap_or_default();
    if query.document_type.is_none() {
        query.document_type = document_type;
    }
    stock_service::list_stock_documents(&state, query)
}

#[tauri::command]
pub fn get_stock_document_detail(
    document_id: String,
    state: State<'_, AppState>,
) -> AppResult<StockDocumentDetail> {
    stock_service::get_stock_document_detail(&state, document_id)
}

#[tauri::command]
pub fn list_stock_balances(
    search: Option<String>,
    query: Option<StockBalanceQuery>,
    state: State<'_, AppState>,
) -> AppResult<Vec<StockBalanceRow>> {
    let mut query = query.unwrap_or_default();
    if query.search.is_none() {
        query.search = search;
    }
    stock_service::list_stock_balances(&state, query)
}

#[tauri::command]
pub fn list_stock_movements(
    search: Option<String>,
    query: Option<StockMovementQuery>,
    state: State<'_, AppState>,
) -> AppResult<Vec<StockMovementRow>> {
    let mut query = query.unwrap_or_default();
    if query.search.is_none() {
        query.search = search;
    }
    stock_service::list_stock_movements(&state, query)
}
