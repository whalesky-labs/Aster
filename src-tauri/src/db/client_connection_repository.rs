use rusqlite::{params, Connection};

use crate::db::pagination::{self, FETCH_SIZE};
use crate::domain::pagination::Page;
use crate::domain::runtime::ClientConnectionInfo;
use crate::error::AppResult;

pub fn list(conn: &Connection) -> AppResult<Vec<ClientConnectionInfo>> {
    pagination::collect_all(|cursor| list_page(conn, cursor))
}

pub fn list_page(conn: &Connection, cursor: Option<&str>) -> AppResult<Page<ClientConnectionInfo>> {
    let offset = pagination::clients_offset(conn, "clients", cursor)?;
    let mut statement = conn.prepare(
        "SELECT id, client_name, client_device_id, COALESCE(client_ip, ''),
                COALESCE(app_version, ''), status, last_seen_at
         FROM client_connections ORDER BY last_seen_at DESC, updated_at DESC, id DESC
         LIMIT ?1 OFFSET ?2",
    )?;
    let rows = statement.query_map(params![FETCH_SIZE, offset], |row| {
        Ok(ClientConnectionInfo {
            id: row.get(0)?,
            client_name: row.get(1)?,
            client_device_id: row.get(2)?,
            client_ip: row.get(3)?,
            app_version: row.get(4)?,
            status: row.get(5)?,
            last_seen_at: row.get(6)?,
        })
    })?;
    let clients = rows.collect::<Result<Vec<_>, _>>()?;
    pagination::clients_page(conn, "clients", offset, clients)
}
