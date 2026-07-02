use tauri::State;

use crate::app::state::AppState;
use crate::domain::approvals::{ApprovalRequest, CreateApprovalRequest, DecideApprovalRequest};
use crate::error::AppResult;
use crate::services::approval_service;

#[tauri::command]
pub fn list_approval_requests(state: State<'_, AppState>) -> AppResult<Vec<ApprovalRequest>> {
    approval_service::list_approval_requests(&state)
}

#[tauri::command]
pub fn create_approval_request(
    request: CreateApprovalRequest,
    state: State<'_, AppState>,
) -> AppResult<ApprovalRequest> {
    approval_service::create_approval_request(&state, request)
}

#[tauri::command]
pub fn decide_approval_request(
    request: DecideApprovalRequest,
    state: State<'_, AppState>,
) -> AppResult<ApprovalRequest> {
    approval_service::decide_approval_request(&state, request)
}
