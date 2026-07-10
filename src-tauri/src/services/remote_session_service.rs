use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::db::connection::Db;
use crate::domain::users::CurrentUser;
use crate::domain::users::LoginRequest;
use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub user: CurrentUser,
    pub session_token: String,
}

pub fn issue(db: &Db, device_token: &str, user: CurrentUser) -> AppResult<LoginResponse> {
    let session_token = uuid::Uuid::new_v4().to_string();
    db.with_conn(|conn| {
        crate::db::session_repository::create(
            conn,
            &token_hash(&session_token),
            &token_hash(device_token),
            &user.id,
            chrono::Utc::now().timestamp(),
        )
    })?;
    Ok(LoginResponse {
        user,
        session_token,
    })
}

pub fn handle_login(db: &Db, raw_request: &str, body: &str) -> AppResult<LoginResponse> {
    let request: LoginRequest = serde_json::from_str(body)
        .map_err(|error| AppError::Validation(format!("登录请求解析失败：{error}")))?;
    let device_token = header(raw_request, "X-Aster-Client-Token")
        .ok_or_else(|| AppError::Validation("登录请求缺少设备凭据".to_string()))?;
    let user = db.with_conn(|conn| crate::services::user_service::login_on_conn(conn, request))?;
    issue(db, &device_token, user)
}

pub fn handle_logout(db: &Db, raw_request: &str) -> AppResult<()> {
    let session_token = header(raw_request, "X-Aster-Session-Token")
        .ok_or_else(|| AppError::Unauthorized("退出请求缺少用户会话".to_string()))?;
    db.with_conn(|conn| {
        crate::db::session_repository::revoke(
            conn,
            &token_hash(&session_token),
            chrono::Utc::now().timestamp(),
        )
    })
}

pub fn current_user(request: &str, conn: &rusqlite::Connection) -> AppResult<CurrentUser> {
    crate::db::session_repository::cleanup(conn, chrono::Utc::now().timestamp())?;
    let device_token = header(request, "X-Aster-Client-Token")
        .ok_or_else(|| AppError::Unauthorized("远程请求缺少设备凭据".to_string()))?;
    let session_token = header(request, "X-Aster-Session-Token")
        .ok_or_else(|| AppError::Unauthorized("远程请求缺少用户会话".to_string()))?;
    let user_id = crate::db::session_repository::authenticate(
        conn,
        &token_hash(&session_token),
        &token_hash(&device_token),
        chrono::Utc::now().timestamp(),
    )?;
    let Some((user, _)) = crate::db::user_repository::find_user_by_id(conn, &user_id)? else {
        return Err(AppError::Unauthorized("远程当前用户不存在".to_string()));
    };
    if !user.enabled {
        return Err(AppError::Unauthorized("远程当前用户已停用".to_string()));
    }
    Ok(crate::services::user_service::to_current_user(user))
}

pub fn token_hash(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn header(request: &str, key: &str) -> Option<String> {
    request.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        (name.eq_ignore_ascii_case(key) && !value.trim().is_empty())
            .then(|| value.trim().to_string())
    })
}
