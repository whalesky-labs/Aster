use crate::app::state::AppState;
use crate::domain::users::CurrentUser;
use crate::error::{AppError, AppResult};

pub fn install_session(state: &AppState, user: CurrentUser) -> AppResult<()> {
    state.db.with_conn(|connection| {
        connection.execute(
            "INSERT INTO users (
                   id, username, display_name, department_id, enabled, must_change_password
                 ) VALUES (?1, ?2, ?3, ?4, 1, 0)
                 ON CONFLICT(id) DO UPDATE SET
                   username = excluded.username,
                   display_name = excluded.display_name,
                   department_id = excluded.department_id,
                   enabled = 1,
                   must_change_password = 0",
            rusqlite::params![
                user.id,
                user.username,
                user.display_name,
                user.department_id
            ],
        )?;
        connection.execute("DELETE FROM user_roles WHERE user_id = ?1", [&user.id])?;
        for role in &user.roles {
            connection.execute(
                "INSERT OR IGNORE INTO roles (id, code, name) VALUES (?1, ?2, ?3)",
                rusqlite::params![role.id, role.code, role.name],
            )?;
            let role_id: String = connection.query_row(
                "SELECT id FROM roles WHERE code = ?1",
                [&role.code],
                |row| row.get(0),
            )?;
            connection.execute(
                "INSERT INTO user_roles (user_id, role_id) VALUES (?1, ?2)",
                rusqlite::params![user.id, role_id],
            )?;
        }
        Ok(())
    })?;
    *state
        .session
        .lock()
        .map_err(|_| AppError::Validation("测试会话状态异常".to_string()))? = Some(user);
    Ok(())
}
