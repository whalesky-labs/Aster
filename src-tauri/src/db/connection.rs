use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::app::paths::AppPaths;
use crate::db::migrations;
use crate::error::AppResult;

#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    pub fn initialize(paths: &AppPaths) -> AppResult<Self> {
        let conn = open_configured_connection(paths)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn clone_handle(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
        }
    }

    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let conn = self.conn.lock().expect("database mutex poisoned");
        f(&conn)
    }

    pub fn with_conn_mut<T>(
        &self,
        f: impl FnOnce(&mut Connection) -> AppResult<T>,
    ) -> AppResult<T> {
        let mut conn = self.conn.lock().expect("database mutex poisoned");
        f(&mut conn)
    }

    pub fn replace_database(
        &self,
        paths: &AppPaths,
        replacement_path: &std::path::Path,
    ) -> AppResult<()> {
        let mut conn = self.conn.lock().expect("database mutex poisoned");
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;

        let placeholder = Connection::open_in_memory()?;
        let old_conn = std::mem::replace(&mut *conn, placeholder);
        match old_conn.close() {
            Ok(()) => {}
            Err((_conn, error)) => return Err(error.into()),
        }

        std::fs::copy(replacement_path, &paths.database_path)?;
        let reopened = open_configured_connection(paths)?;
        *conn = reopened;
        Ok(())
    }
}

fn open_configured_connection(paths: &AppPaths) -> AppResult<Connection> {
    let conn = Connection::open(&paths.database_path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    migrations::run(&conn)?;
    Ok(conn)
}
