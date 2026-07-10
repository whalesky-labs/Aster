use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashSet;
use std::sync::LazyLock;
use uuid::Uuid;

use crate::domain::pagination::Page;
use crate::error::{AppError, AppResult};

pub const PAGE_SIZE: i64 = 500;
pub const FETCH_SIZE: i64 = PAGE_SIZE + 1;

static PROCESS_ID: LazyLock<String> = LazyLock::new(|| Uuid::new_v4().to_string());
static CURSOR_SECRET: LazyLock<String> =
    LazyLock::new(|| format!("{}{}", Uuid::new_v4(), Uuid::new_v4()));

pub fn query(base: &str, offset: Option<i64>) -> String {
    match offset {
        Some(offset) => format!("{base} LIMIT {FETCH_SIZE} OFFSET {offset}"),
        None => base.to_string(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OffsetCursor {
    scope: String,
    revision: Revision,
    offset: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Revision {
    process_id: String,
    database_epoch: String,
    business_revision: i64,
}

pub(crate) fn revision(conn: &rusqlite::Connection) -> AppResult<Revision> {
    revision_for(conn, "revision")
}

fn revision_for(conn: &rusqlite::Connection, column: &str) -> AppResult<Revision> {
    let sql = format!("SELECT database_epoch, {column} FROM pagination_revision WHERE id = 1");
    let (database_epoch, business_revision) =
        conn.query_row(&sql, [], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(Revision {
        process_id: PROCESS_ID.clone(),
        database_epoch,
        business_revision,
    })
}

pub fn offset(conn: &rusqlite::Connection, scope: &str, cursor: Option<&str>) -> AppResult<i64> {
    offset_for(conn, scope, cursor, "revision")
}

pub(crate) fn clients_offset(
    conn: &rusqlite::Connection,
    scope: &str,
    cursor: Option<&str>,
) -> AppResult<i64> {
    offset_for(conn, scope, cursor, "clients_revision")
}

fn offset_for(
    conn: &rusqlite::Connection,
    scope: &str,
    cursor: Option<&str>,
    revision_column: &str,
) -> AppResult<i64> {
    let Some(cursor) = cursor else {
        return Ok(0);
    };
    let cursor: OffsetCursor = decode_cursor(cursor)?;
    if cursor.scope != scope
        || cursor.revision != revision_for(conn, revision_column)?
        || cursor.offset < 0
    {
        return Err(invalid_cursor());
    }
    Ok(cursor.offset)
}

pub fn page<T>(
    conn: &rusqlite::Connection,
    scope: &str,
    offset: i64,
    items: Vec<T>,
) -> AppResult<Page<T>> {
    page_for(conn, scope, offset, items, "revision")
}

pub(crate) fn clients_page<T>(
    conn: &rusqlite::Connection,
    scope: &str,
    offset: i64,
    items: Vec<T>,
) -> AppResult<Page<T>> {
    page_for(conn, scope, offset, items, "clients_revision")
}

fn page_for<T>(
    conn: &rusqlite::Connection,
    scope: &str,
    offset: i64,
    mut items: Vec<T>,
    revision_column: &str,
) -> AppResult<Page<T>> {
    let has_more = items.len() > PAGE_SIZE as usize;
    if has_more {
        items.truncate(PAGE_SIZE as usize);
    }
    let next_cursor = has_more
        .then(|| {
            encode(
                scope,
                revision_for(conn, revision_column)?,
                offset + PAGE_SIZE,
            )
        })
        .transpose()?;
    Ok(Page { items, next_cursor })
}

pub fn collect_all<T>(
    mut fetch: impl FnMut(Option<&str>) -> AppResult<Page<T>>,
) -> AppResult<Vec<T>> {
    let mut output = Vec::new();
    let mut cursor = None;
    let mut seen = HashSet::new();
    loop {
        let page = fetch(cursor.as_deref())?;
        output.extend(page.items);
        let Some(next) = page.next_cursor else {
            return Ok(output);
        };
        if !seen.insert(next.clone()) {
            return Err(AppError::Validation("数据库返回了重复分页游标".to_string()));
        }
        cursor = Some(next);
    }
}

fn encode(scope: &str, revision: Revision, offset: i64) -> AppResult<String> {
    encode_cursor(&OffsetCursor {
        scope: scope.to_string(),
        revision,
        offset,
    })
}

pub(crate) fn encode_cursor<T: Serialize>(cursor: &T) -> AppResult<String> {
    let payload = serde_json::to_vec(cursor)
        .map_err(|error| AppError::Validation(format!("分页游标序列化失败：{error}")))?;
    let signature = signature(&payload)?;
    Ok(format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(payload),
        URL_SAFE_NO_PAD.encode(signature)
    ))
}

pub(crate) fn decode_cursor<T: DeserializeOwned>(cursor: &str) -> AppResult<T> {
    let (payload, signature) = cursor.split_once('.').ok_or_else(invalid_cursor)?;
    let payload = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| invalid_cursor())?;
    let provided = URL_SAFE_NO_PAD
        .decode(signature)
        .map_err(|_| invalid_cursor())?;
    let mut mac =
        Hmac::<Sha256>::new_from_slice(CURSOR_SECRET.as_bytes()).map_err(|_| invalid_cursor())?;
    mac.update(&payload);
    if mac.verify_slice(&provided).is_err() {
        return Err(invalid_cursor());
    }
    serde_json::from_slice(&payload).map_err(|_| invalid_cursor())
}

fn signature(payload: &[u8]) -> AppResult<Vec<u8>> {
    let mut mac = Hmac::<Sha256>::new_from_slice(CURSOR_SECRET.as_bytes())
        .map_err(|_| AppError::Validation("分页游标签名初始化失败".to_string()))?;
    mac.update(payload);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn invalid_cursor() -> AppError {
    AppError::Validation("分页游标无效或不属于当前查询".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_cursor_round_trips_and_is_scope_bound() {
        let conn = rusqlite::Connection::open_in_memory().expect("database");
        crate::db::migrations::run(&conn).expect("migrations");
        let page = page(&conn, "items", 0, (0..501).collect::<Vec<_>>()).expect("page");
        assert_eq!(page.items.len(), 500);
        let cursor = page.next_cursor.expect("cursor");
        assert_eq!(offset(&conn, "items", Some(&cursor)).expect("offset"), 500);
        assert!(offset(&conn, "users", Some(&cursor)).is_err());
    }

    #[test]
    fn page_cursor_is_rejected_after_database_changes() {
        let conn = rusqlite::Connection::open_in_memory().expect("database");
        crate::db::migrations::run(&conn).expect("migrations");
        let page = page(&conn, "items", 0, (0..501).collect::<Vec<_>>()).expect("page");
        let cursor = page.next_cursor.expect("cursor");
        conn.execute(
            "INSERT INTO master_items (id, code, name, unit_id, default_price) VALUES ('changed', 'CHANGED', 'Changed', 'unit-piece', 1)",
            [],
        )
            .expect("insert");
        assert!(offset(&conn, "items", Some(&cursor)).is_err());
    }

    #[test]
    fn client_heartbeat_does_not_invalidate_business_cursor() {
        let conn = rusqlite::Connection::open_in_memory().expect("database");
        crate::db::migrations::run(&conn).expect("migrations");
        let page = page(&conn, "items", 0, (0..501).collect::<Vec<_>>()).expect("page");
        let cursor = page.next_cursor.expect("cursor");
        conn.execute(
            "INSERT INTO client_connections (id, client_name, client_device_id) VALUES ('client', 'Client', 'device')",
            [],
        )
        .expect("client");
        assert_eq!(offset(&conn, "items", Some(&cursor)).expect("offset"), 500);
    }

    #[test]
    fn modified_cursor_is_rejected() {
        let conn = rusqlite::Connection::open_in_memory().expect("database");
        crate::db::migrations::run(&conn).expect("migrations");
        let page = page(&conn, "items", 0, (0..501).collect::<Vec<_>>()).expect("page");
        let mut cursor = page.next_cursor.expect("cursor").into_bytes();
        cursor[0] = if cursor[0] == b'A' { b'B' } else { b'A' };
        let cursor = String::from_utf8(cursor).expect("cursor text");
        assert!(offset(&conn, "items", Some(&cursor)).is_err());
    }
}
