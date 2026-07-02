use crate::app::state::AppState;
use crate::domain::runtime::RuntimeMode;
use crate::error::{AppError, AppResult};

pub fn require_local_primary_database(state: &AppState, action: &str) -> AppResult<()> {
    let mode = crate::services::status_service::get_runtime_config(state)?.mode;
    if mode == RuntimeMode::Client {
        return Err(AppError::Validation(format!(
            "{action}只能在单机模式或主机本机执行，客户端模式不能操作正式数据库"
        )));
    }
    Ok(())
}

pub fn require_dangerous_local_operation(state: &AppState, action: &str) -> AppResult<()> {
    crate::services::user_service::require_admin(state)?;
    require_local_primary_database(state, action)
}
