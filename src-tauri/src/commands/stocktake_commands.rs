use tauri::State;

use crate::app::state::AppState;
use crate::domain::stocktake::{
    ConfirmStocktakeRequest, CreateStocktakeRequest, ExportStocktakeSheetRequest,
    ExportStocktakeSheetResult, StocktakeDetail, StocktakeDocument, UpdateStocktakeCountsRequest,
};
use crate::error::AppResult;
use crate::services::stocktake_service;

#[tauri::command]
pub fn create_stocktake(
    state: State<'_, AppState>,
    request: CreateStocktakeRequest,
) -> AppResult<StocktakeDetail> {
    stocktake_service::create_stocktake(&state, request)
}

#[tauri::command]
pub fn list_stocktakes(state: State<'_, AppState>) -> AppResult<Vec<StocktakeDocument>> {
    stocktake_service::list_stocktakes(&state)
}

#[tauri::command]
pub fn get_stocktake_detail(
    state: State<'_, AppState>,
    stocktake_id: String,
) -> AppResult<StocktakeDetail> {
    stocktake_service::get_stocktake_detail(&state, stocktake_id)
}

#[tauri::command]
pub fn update_stocktake_counts(
    state: State<'_, AppState>,
    request: UpdateStocktakeCountsRequest,
) -> AppResult<StocktakeDetail> {
    stocktake_service::update_stocktake_counts(&state, request)
}

#[tauri::command]
pub fn confirm_stocktake(
    state: State<'_, AppState>,
    request: ConfirmStocktakeRequest,
) -> AppResult<StocktakeDetail> {
    stocktake_service::confirm_stocktake(&state, request)
}

#[tauri::command]
pub fn export_stocktake_sheet(
    state: State<'_, AppState>,
    request: ExportStocktakeSheetRequest,
) -> AppResult<ExportStocktakeSheetResult> {
    stocktake_service::export_stocktake_sheet(&state, request)
}
