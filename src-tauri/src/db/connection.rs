use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[cfg(test)]
use std::sync::atomic::AtomicU64;

use rusqlite::{Connection, ErrorCode};

use crate::app::paths::AppPaths;
use crate::db::migrations;
use crate::error::AppResult;

thread_local! {
    static QUERY_CONTROL: RefCell<Option<QueryControl>> = const { RefCell::new(None) };
}

#[derive(Clone)]
struct QueryControl {
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
    #[cfg(test)]
    test_identity: u64,
}

#[cfg(test)]
static NEXT_TEST_DB_ID: AtomicU64 = AtomicU64::new(1);

impl Db {
    pub fn initialize(paths: &AppPaths) -> AppResult<Self> {
        let conn = open_configured_connection(paths)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            #[cfg(test)]
            test_identity: NEXT_TEST_DB_ID.fetch_add(1, Ordering::Relaxed),
        })
    }

    pub fn clone_handle(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
            #[cfg(test)]
            test_identity: self.test_identity,
        }
    }

    #[cfg(test)]
    pub(crate) fn test_identity(&self) -> u64 {
        self.test_identity
    }

    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let conn = self.conn.lock().expect("database mutex poisoned");
        run_with_deadline(&conn, || f(&conn))
    }

    pub fn with_conn_mut<T>(
        &self,
        f: impl FnOnce(&mut Connection) -> AppResult<T>,
    ) -> AppResult<T> {
        let mut conn = self.conn.lock().expect("database mutex poisoned");
        run_with_deadline_mut(&mut conn, f)
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

pub fn with_query_control<T>(
    duration: Duration,
    cancelled: Arc<AtomicBool>,
    operation: impl FnOnce() -> T,
) -> T {
    QUERY_CONTROL.with(|slot| {
        let previous = slot.replace(Some(QueryControl {
            deadline: Instant::now() + duration,
            cancelled,
        }));
        let result = operation();
        slot.replace(previous);
        result
    })
}

fn run_with_deadline<T>(
    conn: &Connection,
    operation: impl FnOnce() -> AppResult<T>,
) -> AppResult<T> {
    let control = install_progress_handler(conn);
    let result = operation();
    clear_progress_handler(conn, control.is_some());
    map_interrupted_result(result, control.as_ref())
}

fn run_with_deadline_mut<T>(
    conn: &mut Connection,
    operation: impl FnOnce(&mut Connection) -> AppResult<T>,
) -> AppResult<T> {
    let control = install_progress_handler(conn);
    let result = operation(conn);
    clear_progress_handler(conn, control.is_some());
    map_interrupted_result(result, control.as_ref())
}

fn install_progress_handler(conn: &Connection) -> Option<QueryControl> {
    let control = QUERY_CONTROL.with(|slot| slot.borrow().clone());
    if let Some(control) = control.clone() {
        conn.progress_handler(
            1_000,
            Some(move || {
                control.cancelled.load(Ordering::Acquire) || Instant::now() >= control.deadline
            }),
        );
    }
    control
}

fn clear_progress_handler(conn: &Connection, installed: bool) {
    if installed {
        conn.progress_handler(0, None::<fn() -> bool>);
    }
}

fn map_interrupted_result<T>(result: AppResult<T>, control: Option<&QueryControl>) -> AppResult<T> {
    match result {
        Err(crate::error::AppError::Database(rusqlite::Error::SqliteFailure(error, _)))
            if error.code == ErrorCode::OperationInterrupted =>
        {
            let message =
                if control.is_some_and(|control| control.cancelled.load(Ordering::Acquire)) {
                    "客户端连接已断开，数据库操作已安全取消"
                } else {
                    "数据库操作超过 30 秒，已安全中断"
                };
            Err(crate::error::AppError::Timeout(message.to_string()))
        }
        other => other,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_deadline_interrupts_long_running_statement() {
        let conn = Connection::open_in_memory().expect("database");
        let db = Db {
            conn: Arc::new(Mutex::new(conn)),
            test_identity: NEXT_TEST_DB_ID.fetch_add(1, Ordering::Relaxed),
        };
        let error = with_query_control(Duration::ZERO, Arc::new(AtomicBool::new(false)), || {
            db.with_conn(|conn| {
                conn.query_row(
                    "WITH RECURSIVE counter(value) AS (SELECT 1 UNION ALL SELECT value + 1 FROM counter WHERE value < 1000000) SELECT sum(value) FROM counter",
                    [],
                    |_| Ok(()),
                )?;
                Ok(())
            })
        })
        .expect_err("deadline must interrupt query");
        assert!(matches!(error, crate::error::AppError::Timeout(_)));
    }

    #[test]
    fn cancellation_token_interrupts_current_database_task() {
        let conn = Connection::open_in_memory().expect("database");
        let db = Db {
            conn: Arc::new(Mutex::new(conn)),
            test_identity: NEXT_TEST_DB_ID.fetch_add(1, Ordering::Relaxed),
        };
        let error = with_query_control(
            Duration::from_secs(30),
            Arc::new(AtomicBool::new(true)),
            || {
                db.with_conn(|conn| {
                    conn.query_row(
                        "WITH RECURSIVE counter(value) AS (SELECT 1 UNION ALL SELECT value + 1 FROM counter WHERE value < 1000000) SELECT sum(value) FROM counter",
                        [],
                        |_| Ok(()),
                    )?;
                    Ok(())
                })
            },
        )
        .expect_err("cancelled task must stop");
        assert!(error.to_string().contains("连接已断开"));
    }
}
