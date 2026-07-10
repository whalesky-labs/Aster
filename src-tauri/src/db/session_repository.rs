use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{AppError, AppResult};

const IDLE_TTL_SECONDS: i64 = 8 * 60 * 60;
const ABSOLUTE_TTL_SECONDS: i64 = 24 * 60 * 60;

pub fn create(
    conn: &Connection,
    token_hash: &str,
    device_token_hash: &str,
    user_id: &str,
    now: i64,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO user_sessions (
           token_hash, device_token_hash, user_id, created_at_unix,
           last_seen_at_unix, expires_at_unix
         ) VALUES (?1, ?2, ?3, ?4, ?4, ?5)",
        params![
            token_hash,
            device_token_hash,
            user_id,
            now,
            now + ABSOLUTE_TTL_SECONDS
        ],
    )?;
    Ok(())
}

pub fn authenticate(
    conn: &Connection,
    token_hash: &str,
    device_token_hash: &str,
    now: i64,
) -> AppResult<String> {
    let session = conn
        .query_row(
            "SELECT user_id, last_seen_at_unix, expires_at_unix, revoked_at_unix
             FROM user_sessions
             WHERE token_hash = ?1 AND device_token_hash = ?2",
            params![token_hash, device_token_hash],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| AppError::Unauthorized("用户会话无效，请重新登录".to_string()))?;
    if session.3.is_some() || session.2 <= now || session.1 + IDLE_TTL_SECONDS <= now {
        return Err(AppError::Unauthorized(
            "用户会话已过期，请重新登录".to_string(),
        ));
    }
    conn.execute(
        "UPDATE user_sessions SET last_seen_at_unix = ?1 WHERE token_hash = ?2",
        params![now, token_hash],
    )?;
    Ok(session.0)
}

pub fn revoke(conn: &Connection, token_hash: &str, now: i64) -> AppResult<()> {
    conn.execute(
        "UPDATE user_sessions SET revoked_at_unix = ?1 WHERE token_hash = ?2",
        params![now, token_hash],
    )?;
    Ok(())
}

pub fn revoke_user(conn: &Connection, user_id: &str, now: i64) -> AppResult<()> {
    conn.execute(
        "UPDATE user_sessions SET revoked_at_unix = ?1
         WHERE user_id = ?2 AND revoked_at_unix IS NULL",
        params![now, user_id],
    )?;
    Ok(())
}

pub fn cleanup(conn: &Connection, now: i64) -> AppResult<()> {
    conn.execute(
        "DELETE FROM user_sessions
         WHERE expires_at_unix <= ?1
            OR last_seen_at_unix + ?2 <= ?1
            OR revoked_at_unix IS NOT NULL",
        params![now, IDLE_TTL_SECONDS],
    )?;
    Ok(())
}
