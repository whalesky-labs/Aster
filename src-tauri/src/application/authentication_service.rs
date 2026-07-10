use rusqlite::params;

use crate::app::state::AppState;
use crate::db::user_repository;
use crate::domain::users::{CurrentUser, LoginRequest};
use crate::error::{AppError, AppResult};

const DEFAULT_ADMIN_PASSWORD: &str = "admin123";

pub fn ensure_default_admin(state: &AppState) -> AppResult<()> {
    let hash = crate::domain::passwords::hash(DEFAULT_ADMIN_PASSWORD)?;
    state
        .db
        .with_conn(|conn| user_repository::ensure_default_admin(conn, &hash))?;
    let admin = state
        .db
        .with_conn(|conn| user_repository::find_user_by_username(conn, "admin"))?;
    if let Some((admin, current_hash)) = admin {
        let uses_default = current_hash
            .as_deref()
            .is_some_and(|hash| crate::domain::passwords::verify(DEFAULT_ADMIN_PASSWORD, hash));
        state.db.with_conn(|conn| {
            user_repository::set_must_change_password(conn, &admin.id, uses_default)
        })?;
    }
    Ok(())
}

pub fn password_change_required(state: &AppState) -> AppResult<bool> {
    let current = crate::services::user_service::current_user(state)?
        .ok_or_else(|| AppError::Validation("请先登录".to_string()))?;
    state
        .db
        .with_conn(|conn| user_repository::must_change_password(conn, &current.id))
}

pub fn require_password_changed(state: &AppState, current: &CurrentUser) -> AppResult<()> {
    if state
        .db
        .with_conn(|conn| user_repository::must_change_password(conn, &current.id))?
    {
        Err(AppError::Validation(
            "当前账户必须先修改默认密码".to_string(),
        ))
    } else {
        Ok(())
    }
}

pub fn login_locally(state: &AppState, request: LoginRequest) -> AppResult<CurrentUser> {
    let username = request.username.trim().to_string();
    if username.is_empty() || request.password.is_empty() {
        return Err(AppError::Validation("用户名和密码不能为空".to_string()));
    }
    let candidate = state
        .db
        .with_conn(|conn| user_repository::find_user_by_username(conn, &username))?;
    let Some((user, hash)) = candidate else {
        return Err(AppError::Validation("用户名或密码错误".to_string()));
    };
    if !user.enabled {
        return Err(AppError::Validation("用户已停用".to_string()));
    }
    let Some(hash) = hash else {
        return Err(AppError::Validation("用户未设置密码".to_string()));
    };
    if !crate::domain::passwords::verify(&request.password, &hash) {
        return Err(AppError::Validation("用户名或密码错误".to_string()));
    }
    let current = crate::services::user_service::to_current_user(user);
    state.db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
             VALUES (?1, 'login', 'user', ?2, ?3, ?4)",
            params![
                uuid::Uuid::new_v4().to_string(),
                current.id,
                format!("用户登录：{}", current.username),
                current.username
            ],
        )?;
        Ok(())
    })?;
    Ok(current)
}
