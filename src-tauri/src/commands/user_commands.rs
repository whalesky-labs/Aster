use tauri::State;

use crate::app::state::AppState;
use crate::domain::users::{
    ChangePasswordRequest, CurrentUser, LoginRequest, Role, SaveUserRequest, SetUserEnabledRequest,
    UserAccount,
};
use crate::error::AppResult;
use crate::services::user_service;

#[tauri::command]
pub async fn login(state: State<'_, AppState>, request: LoginRequest) -> AppResult<CurrentUser> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || user_service::login(&state, request))
        .await
        .map_err(|err| crate::error::AppError::Validation(format!("登录任务异常：{err}")))?
}

#[tauri::command]
pub fn logout(state: State<'_, AppState>) -> AppResult<()> {
    user_service::logout(&state)
}

#[tauri::command]
pub fn get_current_user(state: State<'_, AppState>) -> AppResult<Option<CurrentUser>> {
    user_service::current_user(&state)
}

#[tauri::command]
pub fn list_user_accounts(state: State<'_, AppState>) -> AppResult<Vec<UserAccount>> {
    user_service::list_users(&state)
}

#[tauri::command]
pub fn list_roles(state: State<'_, AppState>) -> AppResult<Vec<Role>> {
    user_service::list_roles(&state)
}

#[tauri::command]
pub fn save_user_account(
    state: State<'_, AppState>,
    request: SaveUserRequest,
) -> AppResult<UserAccount> {
    user_service::save_user(&state, request)
}

#[tauri::command]
pub fn set_user_account_enabled(
    state: State<'_, AppState>,
    request: SetUserEnabledRequest,
) -> AppResult<()> {
    user_service::set_user_enabled(&state, request)
}

#[tauri::command]
pub fn change_password(
    state: State<'_, AppState>,
    request: ChangePasswordRequest,
) -> AppResult<()> {
    user_service::change_password(&state, request)
}
