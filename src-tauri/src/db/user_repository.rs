use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::domain::users::{Role, UserAccount};
use crate::error::{AppError, AppResult};

pub fn list_roles(conn: &Connection) -> AppResult<Vec<Role>> {
    let mut stmt = conn.prepare("SELECT id, code, name FROM roles ORDER BY code ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(Role {
            id: row.get(0)?,
            code: row.get(1)?,
            name: row.get(2)?,
        })
    })?;
    collect_rows(rows)
}

pub fn list_users(conn: &Connection) -> AppResult<Vec<UserAccount>> {
    let mut stmt = conn.prepare(
        "SELECT u.id, u.username, u.display_name, u.email, u.department_id, d.name,
                u.enabled, u.created_at, u.updated_at
         FROM users u
         LEFT JOIN departments d ON d.id = u.department_id
         ORDER BY u.enabled DESC, u.username ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(UserAccount {
            id: row.get(0)?,
            username: row.get(1)?,
            display_name: row.get(2)?,
            email: row.get(3)?,
            department_id: row.get(4)?,
            department_name: row.get(5)?,
            enabled: row.get::<_, i64>(6)? == 1,
            roles: Vec::new(),
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })?;
    let mut users = collect_rows(rows)?;
    for user in &mut users {
        user.roles = roles_for_user(conn, &user.id)?;
    }
    Ok(users)
}

pub fn find_user_by_username(
    conn: &Connection,
    username: &str,
) -> AppResult<Option<(UserAccount, Option<String>)>> {
    let record = conn
        .query_row(
            "SELECT u.id, u.username, u.display_name, u.email, u.password_hash, u.department_id, d.name,
                    u.enabled, u.created_at, u.updated_at
             FROM users u
             LEFT JOIN departments d ON d.id = u.department_id
             WHERE u.username = ?1",
            params![username],
            |row| {
                Ok((
                    UserAccount {
                        id: row.get(0)?,
                        username: row.get(1)?,
                        display_name: row.get(2)?,
                        email: row.get(3)?,
                        department_id: row.get(5)?,
                        department_name: row.get(6)?,
                        enabled: row.get::<_, i64>(7)? == 1,
                        roles: Vec::new(),
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    },
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .optional()?;
    if let Some((mut user, hash)) = record {
        user.roles = roles_for_user(conn, &user.id)?;
        Ok(Some((user, hash)))
    } else {
        Ok(None)
    }
}

pub fn find_user_by_id(
    conn: &Connection,
    user_id: &str,
) -> AppResult<Option<(UserAccount, Option<String>)>> {
    let record = conn
        .query_row(
            "SELECT u.id, u.username, u.display_name, u.email, u.password_hash, u.department_id, d.name,
                    u.enabled, u.created_at, u.updated_at
             FROM users u
             LEFT JOIN departments d ON d.id = u.department_id
             WHERE u.id = ?1",
            params![user_id],
            |row| {
                Ok((
                    UserAccount {
                        id: row.get(0)?,
                        username: row.get(1)?,
                        display_name: row.get(2)?,
                        email: row.get(3)?,
                        department_id: row.get(5)?,
                        department_name: row.get(6)?,
                        enabled: row.get::<_, i64>(7)? == 1,
                        roles: Vec::new(),
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    },
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .optional()?;
    if let Some((mut user, hash)) = record {
        user.roles = roles_for_user(conn, &user.id)?;
        Ok(Some((user, hash)))
    } else {
        Ok(None)
    }
}

pub fn save_user(
    conn: &Connection,
    id: Option<String>,
    username: &str,
    display_name: &str,
    email: Option<String>,
    password_hash: Option<String>,
    department_id: Option<String>,
    enabled: bool,
    role_codes: &[String],
) -> AppResult<UserAccount> {
    let user_id = id.unwrap_or_else(new_id);
    let role_ids = role_ids_for_codes(conn, role_codes)?;
    if let Some(hash) = password_hash {
        conn.execute(
            "INSERT INTO users (id, username, display_name, email, password_hash, department_id, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
               username = excluded.username,
               display_name = excluded.display_name,
               email = excluded.email,
               password_hash = excluded.password_hash,
               department_id = excluded.department_id,
               enabled = excluded.enabled,
               updated_at = CURRENT_TIMESTAMP",
            params![
                user_id,
                username,
                display_name,
                blank_to_none(email.clone()),
                hash,
                blank_to_none(department_id),
                bool_to_i64(enabled)
            ],
        )?;
    } else {
        conn.execute(
            "INSERT INTO users (id, username, display_name, email, department_id, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
               username = excluded.username,
               display_name = excluded.display_name,
               email = excluded.email,
               department_id = excluded.department_id,
               enabled = excluded.enabled,
               updated_at = CURRENT_TIMESTAMP",
            params![
                user_id,
                username,
                display_name,
                blank_to_none(email.clone()),
                blank_to_none(department_id),
                bool_to_i64(enabled)
            ],
        )?;
    }
    set_user_roles(conn, &user_id, &role_ids)?;
    Ok(find_user_by_id(conn, &user_id)?
        .expect("saved user exists")
        .0)
}

pub fn set_user_enabled(conn: &Connection, user_id: &str, enabled: bool) -> AppResult<()> {
    let affected = conn.execute(
        "UPDATE users SET enabled = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![bool_to_i64(enabled), user_id],
    )?;
    if affected == 0 {
        return Err(AppError::Validation("用户不存在".to_string()));
    }
    Ok(())
}

pub fn update_password_hash(
    conn: &Connection,
    user_id: &str,
    password_hash: &str,
) -> AppResult<()> {
    conn.execute(
        "UPDATE users SET password_hash = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![password_hash, user_id],
    )?;
    Ok(())
}

pub fn create_password_reset_code(
    conn: &Connection,
    user_id: &str,
    code_hash: &str,
    expires_at: &str,
) -> AppResult<()> {
    conn.execute(
        "UPDATE password_reset_codes
         SET used_at = CURRENT_TIMESTAMP
         WHERE user_id = ?1 AND used_at IS NULL",
        params![user_id],
    )?;
    conn.execute(
        "INSERT INTO password_reset_codes (id, user_id, code_hash, expires_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![Uuid::new_v4().to_string(), user_id, code_hash, expires_at],
    )?;
    Ok(())
}

pub fn find_active_password_reset_code(
    conn: &Connection,
    username: &str,
) -> AppResult<Option<(String, String, String, String)>> {
    Ok(conn
        .query_row(
            "SELECT c.id, u.id, u.username, c.code_hash
             FROM password_reset_codes c
             JOIN users u ON u.id = c.user_id
             WHERE u.username = ?1
               AND u.enabled = 1
               AND c.used_at IS NULL
               AND datetime(c.expires_at) > datetime('now')
             ORDER BY c.created_at DESC
             LIMIT 1",
            params![username],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?)
}

pub fn mark_password_reset_code_used(conn: &Connection, code_id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE password_reset_codes SET used_at = CURRENT_TIMESTAMP WHERE id = ?1",
        params![code_id],
    )?;
    Ok(())
}

pub fn ensure_default_admin(conn: &Connection, password_hash: &str) -> AppResult<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE username = 'admin')",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute(
            "INSERT INTO users (id, username, display_name, password_hash, enabled)
             VALUES ('user-admin', 'admin', '系统管理员', ?1, 1)",
            params![password_hash],
        )?;
        let role_ids = role_ids_for_codes(conn, &["admin".to_string()])?;
        set_user_roles(conn, "user-admin", &role_ids)?;
    }
    Ok(())
}

fn roles_for_user(conn: &Connection, user_id: &str) -> AppResult<Vec<Role>> {
    let mut stmt = conn.prepare(
        "SELECT r.id, r.code, r.name
         FROM user_roles ur
         JOIN roles r ON r.id = ur.role_id
         WHERE ur.user_id = ?1
         ORDER BY r.code ASC",
    )?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok(Role {
            id: row.get(0)?,
            code: row.get(1)?,
            name: row.get(2)?,
        })
    })?;
    collect_rows(rows)
}

fn role_ids_for_codes(conn: &Connection, role_codes: &[String]) -> AppResult<Vec<String>> {
    let mut role_ids = Vec::new();
    for code in role_codes {
        let trimmed = code.trim();
        if trimmed.is_empty() {
            return Err(AppError::Validation("角色不能为空".to_string()));
        }
        let role_id = conn
            .query_row(
                "SELECT id FROM roles WHERE code = ?1",
                params![trimmed],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| AppError::Validation(format!("角色不存在：{trimmed}")))?;
        if !role_ids.iter().any(|id| id == &role_id) {
            role_ids.push(role_id);
        }
    }
    if role_ids.is_empty() {
        return Err(AppError::Validation("至少选择一个有效角色".to_string()));
    }
    Ok(role_ids)
}

fn set_user_roles(conn: &Connection, user_id: &str, role_ids: &[String]) -> AppResult<()> {
    conn.execute(
        "DELETE FROM user_roles WHERE user_id = ?1",
        params![user_id],
    )?;
    for role_id in role_ids {
        conn.execute(
            "INSERT OR IGNORE INTO user_roles (user_id, role_id) VALUES (?1, ?2)",
            params![user_id, role_id],
        )?;
    }
    Ok(())
}

fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> AppResult<Vec<T>> {
    let mut output = Vec::new();
    for row in rows {
        output.push(row?);
    }
    Ok(output)
}

fn bool_to_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn blank_to_none(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::db::migrations;

    use super::*;

    #[test]
    fn save_user_rejects_unknown_role_codes() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();

        let error = save_user(
            &conn,
            None,
            "bad-role-user",
            "错误角色用户",
            None,
            Some("hash".to_string()),
            None,
            true,
            &["warehouse".to_string(), "ghost-role".to_string()],
        )
        .unwrap_err();

        assert!(error.to_string().contains("角色不存在：ghost-role"));
        let user_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE username = 'bad-role-user'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(user_count, 0);
        let role_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM user_roles", [], |row| row.get(0))
            .unwrap();
        assert_eq!(role_count, 0);
    }

    #[test]
    fn set_user_enabled_rejects_missing_user() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();

        let error = set_user_enabled(&conn, "missing-user", false).unwrap_err();

        assert!(error.to_string().contains("用户不存在"));
    }
}
