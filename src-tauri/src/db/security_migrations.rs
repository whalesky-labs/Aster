use rusqlite::Connection;

use crate::error::AppResult;

pub fn run(conn: &Connection) -> AppResult<()> {
    if !column_exists(conn, "users", "must_change_password")? {
        conn.execute(
            "ALTER TABLE users ADD COLUMN must_change_password INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS user_sessions (
           token_hash TEXT PRIMARY KEY,
           device_token_hash TEXT NOT NULL,
           user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
           created_at_unix INTEGER NOT NULL,
           last_seen_at_unix INTEGER NOT NULL,
           expires_at_unix INTEGER NOT NULL,
           revoked_at_unix INTEGER
         );
         CREATE INDEX IF NOT EXISTS idx_user_sessions_user_id
           ON user_sessions(user_id);",
    )?;
    migrate_secure_transport_v2(conn)?;
    Ok(())
}

fn migrate_secure_transport_v2(conn: &Connection) -> AppResult<()> {
    let settings_exist: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'app_settings')",
        [],
        |row| row.get(0),
    )?;
    if !settings_exist {
        return Ok(());
    }
    let version = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = 'security_protocol_version'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok();
    if version.as_deref() == Some("2") {
        return Ok(());
    }
    conn.execute_batch(
        "BEGIN IMMEDIATE;
         DELETE FROM app_settings
           WHERE key IN ('client_token', 'host_certificate_fingerprint');
         DELETE FROM user_sessions;
         INSERT INTO app_settings (key, value, updated_at)
           VALUES ('security_protocol_version', '2', CURRENT_TIMESTAMP)
           ON CONFLICT(key) DO UPDATE
             SET value = excluded.value, updated_at = CURRENT_TIMESTAMP;
         COMMIT;",
    )?;
    Ok(())
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> AppResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2)",
        rusqlite::params![table, column],
        |row| row.get(0),
    )?)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    #[test]
    fn v2_migration_clears_old_remote_credentials_but_keeps_host_address() {
        let connection = Connection::open_in_memory().expect("open database");
        connection
            .execute_batch(
                "CREATE TABLE users (id TEXT PRIMARY KEY, must_change_password INTEGER NOT NULL DEFAULT 0);
                 CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
                 INSERT INTO app_settings (key, value) VALUES
                   ('host_address', '192.168.1.10'),
                   ('client_token', 'old-token'),
                   ('host_certificate_fingerprint', 'old-pin');",
            )
            .expect("create schema");
        super::run(&connection).expect("run migration");
        let address: String = connection
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'host_address'",
                [],
                |row| row.get(0),
            )
            .expect("host address");
        let credential_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM app_settings WHERE key IN ('client_token', 'host_certificate_fingerprint')",
                [],
                |row| row.get(0),
            )
            .expect("credential count");
        assert_eq!(address, "192.168.1.10");
        assert_eq!(credential_count, 0);
    }
}
