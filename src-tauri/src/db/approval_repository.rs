use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::domain::approvals::{ApprovalRequest, CreateApprovalRequest};
use crate::error::{AppError, AppResult};

pub fn list_approval_requests(conn: &Connection) -> AppResult<Vec<ApprovalRequest>> {
    let mut stmt = conn.prepare(
        "SELECT id, entity_type, entity_id, status, requested_by, decided_by,
                reason, decision_note, created_at, decided_at
         FROM approval_requests
         ORDER BY CASE status WHEN 'pending' THEN 0 ELSE 1 END, created_at DESC
         LIMIT 200",
    )?;
    let rows = stmt.query_map([], map_approval)?;
    collect_rows(rows)
}

pub fn create_approval_request(
    conn: &Connection,
    request: CreateApprovalRequest,
    requested_by: Option<String>,
) -> AppResult<ApprovalRequest> {
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO approval_requests (id, entity_type, entity_id, status, requested_by, reason)
         VALUES (?1, ?2, ?3, 'pending', ?4, ?5)",
        params![
            id,
            request.entity_type,
            request.entity_id,
            requested_by,
            request.reason.trim()
        ],
    )?;
    get_approval_request(conn, &id)
}

pub fn decide_approval_request(
    conn: &Connection,
    approval_id: &str,
    status: &str,
    decided_by: Option<String>,
    decision_note: Option<String>,
) -> AppResult<ApprovalRequest> {
    let changed = conn.execute(
        "UPDATE approval_requests
         SET status = ?1,
             decided_by = ?2,
             decision_note = ?3,
             decided_at = CURRENT_TIMESTAMP
        WHERE id = ?4 AND status = 'pending'",
        params![status, decided_by, decision_note, approval_id],
    )?;
    if changed == 0 {
        return Err(AppError::Validation(
            "审批单不存在或已处理，不能重复审批".to_string(),
        ));
    }
    get_approval_request(conn, approval_id)
}

pub fn get_approval_request(conn: &Connection, id: &str) -> AppResult<ApprovalRequest> {
    Ok(conn.query_row(
        "SELECT id, entity_type, entity_id, status, requested_by, decided_by,
                reason, decision_note, created_at, decided_at
         FROM approval_requests
         WHERE id = ?1",
        params![id],
        map_approval,
    )?)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::db::migrations;
    use crate::domain::approvals::CreateApprovalRequest;

    use super::*;

    #[test]
    fn decide_approval_request_rejects_already_processed_request() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO users (id, username, display_name, enabled)
             VALUES ('user-warehouse', 'warehouse-approval-test', '审批仓库员', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, display_name, enabled)
             VALUES ('user-admin', 'admin-approval-test', '审批管理员', 1)",
            [],
        )
        .unwrap();
        let approval = create_approval_request(
            &conn,
            CreateApprovalRequest {
                entity_type: "budget_override".to_string(),
                entity_id: "dept-admin-office:2026-06".to_string(),
                reason: "超预算领用".to_string(),
            },
            Some("user-warehouse".to_string()),
        )
        .unwrap();
        decide_approval_request(
            &conn,
            &approval.id,
            "approved",
            Some("user-admin".to_string()),
            None,
        )
        .unwrap();

        let error = decide_approval_request(
            &conn,
            &approval.id,
            "rejected",
            Some("user-admin".to_string()),
            None,
        )
        .unwrap_err();

        assert!(error.to_string().contains("不能重复审批"));
        let status: String = conn
            .query_row(
                "SELECT status FROM approval_requests WHERE id = ?1",
                params![approval.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "approved");
    }
}

fn map_approval(row: &rusqlite::Row<'_>) -> rusqlite::Result<ApprovalRequest> {
    Ok(ApprovalRequest {
        id: row.get(0)?,
        entity_type: row.get(1)?,
        entity_id: row.get(2)?,
        status: row.get(3)?,
        requested_by: row.get(4)?,
        decided_by: row.get(5)?,
        reason: row.get(6)?,
        decision_note: row.get(7)?,
        created_at: row.get(8)?,
        decided_at: row.get(9)?,
    })
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
