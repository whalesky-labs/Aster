use rusqlite::{params, Connection};

use crate::db::pagination::{self, FETCH_SIZE};
use crate::domain::backups::BackupRecord;
use crate::domain::pagination::Page;
use crate::error::AppResult;

pub fn list_backup_records(conn: &Connection) -> AppResult<Vec<BackupRecord>> {
    pagination::collect_all(|cursor| list_backup_records_page(conn, cursor))
}

pub fn list_backup_records_page(
    conn: &Connection,
    cursor: Option<&str>,
) -> AppResult<Page<BackupRecord>> {
    let offset = pagination::offset(conn, "backups", cursor)?;
    let mut stmt = conn.prepare(
        "SELECT id, backup_file, backup_type, app_version, schema_version,
                host_name, os, database_size, sha256, status, error_message, created_at
         FROM backup_jobs
         ORDER BY created_at DESC, id DESC
         LIMIT ?1 OFFSET ?2",
    )?;
    let rows = stmt.query_map(params![FETCH_SIZE, offset], |row| {
        Ok(BackupRecord {
            id: row.get(0)?,
            backup_file: row.get(1)?,
            backup_type: row.get(2)?,
            app_version: row.get(3)?,
            schema_version: row.get(4)?,
            host_name: row.get(5)?,
            os: row.get(6)?,
            database_size: row.get(7)?,
            sha256: row.get(8)?,
            status: row.get(9)?,
            error_message: row.get(10)?,
            created_at: row.get(11)?,
        })
    })?;

    let mut output = Vec::new();
    for row in rows {
        output.push(row?);
    }
    pagination::page(conn, "backups", offset, output)
}

pub fn list_auto_backup_records(conn: &Connection) -> AppResult<Vec<BackupRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, backup_file, backup_type, app_version, schema_version,
                host_name, os, database_size, sha256, status, error_message, created_at
         FROM backup_jobs
         WHERE status = 'success'
           AND backup_type IN ('auto_startup', 'auto_interval')
         ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(BackupRecord {
            id: row.get(0)?,
            backup_file: row.get(1)?,
            backup_type: row.get(2)?,
            app_version: row.get(3)?,
            schema_version: row.get(4)?,
            host_name: row.get(5)?,
            os: row.get(6)?,
            database_size: row.get(7)?,
            sha256: row.get(8)?,
            status: row.get(9)?,
            error_message: row.get(10)?,
            created_at: row.get(11)?,
        })
    })?;

    let mut output = Vec::new();
    for row in rows {
        output.push(row?);
    }
    Ok(output)
}

pub struct NewBackupRecord<'a> {
    pub id: &'a str,
    pub backup_file: &'a str,
    pub backup_type: &'a str,
    pub app_version: &'a str,
    pub schema_version: i64,
    pub host_name: &'a str,
    pub os: &'a str,
    pub database_size: u64,
    pub sha256: &'a str,
    pub status: &'a str,
    pub error_message: Option<&'a str>,
}

pub struct SuccessfulBackupRecord<'a> {
    pub id: &'a str,
    pub backup_file: &'a str,
    pub backup_type: &'a str,
    pub app_version: &'a str,
    pub schema_version: i64,
    pub host_name: &'a str,
    pub os: &'a str,
    pub database_size: u64,
    pub sha256: &'a str,
}

pub fn insert_successful_backup(
    conn: &Connection,
    record: SuccessfulBackupRecord<'_>,
) -> AppResult<()> {
    insert_backup_record(
        conn,
        NewBackupRecord {
            id: record.id,
            backup_file: record.backup_file,
            backup_type: record.backup_type,
            app_version: record.app_version,
            schema_version: record.schema_version,
            host_name: record.host_name,
            os: record.os,
            database_size: record.database_size,
            sha256: record.sha256,
            status: "success",
            error_message: None,
        },
    )
}

pub fn insert_backup_record(conn: &Connection, record: NewBackupRecord<'_>) -> AppResult<()> {
    conn.execute(
        "INSERT INTO backup_jobs (
           id, backup_file, backup_type, app_version, schema_version,
           host_name, os, database_size, sha256, status, error_message
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            record.id,
            record.backup_file,
            record.backup_type,
            record.app_version,
            record.schema_version,
            record.host_name,
            record.os,
            record.database_size as i64,
            record.sha256,
            record.status,
            record.error_message
        ],
    )?;
    Ok(())
}

pub fn latest_successful_backup_at(
    conn: &Connection,
    backup_type: &str,
) -> AppResult<Option<String>> {
    use rusqlite::OptionalExtension;

    Ok(conn
        .query_row(
            "SELECT created_at
             FROM backup_jobs
             WHERE backup_type = ?1
               AND status = 'success'
             ORDER BY created_at DESC
             LIMIT 1",
            params![backup_type],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn delete_backup_record(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM backup_jobs WHERE id = ?1", params![id])?;
    Ok(())
}
