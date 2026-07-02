use rusqlite::params;
use rusqlite::{Connection, OptionalExtension};

use crate::app::state::AppState;
use crate::db::approval_repository;
use crate::domain::approvals::{ApprovalRequest, CreateApprovalRequest, DecideApprovalRequest};
use crate::domain::runtime::RuntimeMode;
use crate::error::{AppError, AppResult};

pub fn list_approval_requests(state: &AppState) -> AppResult<Vec<ApprovalRequest>> {
    crate::services::user_service::require_admin(state)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_approval_requests(state);
    }
    state
        .db
        .with_conn(approval_repository::list_approval_requests)
}

pub fn create_approval_request(
    state: &AppState,
    request: CreateApprovalRequest,
) -> AppResult<ApprovalRequest> {
    let current = crate::services::user_service::require_permission(state, "write_stock")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_create_approval_request(state, request);
    }
    state.db.with_conn(|conn| {
        create_approval_request_on_conn(conn, request, Some(current.id), &current.username)
    })
}

pub fn decide_approval_request(
    state: &AppState,
    request: DecideApprovalRequest,
) -> AppResult<ApprovalRequest> {
    let current = crate::services::user_service::require_admin(state)?;
    if request.approval_id.trim().is_empty() {
        return Err(AppError::Validation("审批单不能为空".to_string()));
    }
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_decide_approval_request(state, request);
    }
    state.db.with_conn(|conn| {
        decide_approval_request_on_conn(conn, request, Some(current.id), &current.username)
    })
}

fn runtime_mode(state: &AppState) -> AppResult<RuntimeMode> {
    Ok(crate::services::status_service::get_runtime_config(state)?.mode)
}

pub(crate) fn create_approval_request_on_conn(
    conn: &Connection,
    request: CreateApprovalRequest,
    requested_by: Option<String>,
    audit_operator: &str,
) -> AppResult<ApprovalRequest> {
    validate_create_request_on_conn(conn, &request)?;
    let approval = approval_repository::create_approval_request(conn, request, requested_by)?;
    conn.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, 'create_approval_request', 'approval', ?2, ?3, ?4)",
        params![
            uuid::Uuid::new_v4().to_string(),
            approval.id,
            approval.reason.clone().unwrap_or_default(),
            audit_operator
        ],
    )?;
    Ok(approval)
}

pub(crate) fn decide_approval_request_on_conn(
    conn: &Connection,
    request: DecideApprovalRequest,
    decided_by: Option<String>,
    audit_operator: &str,
) -> AppResult<ApprovalRequest> {
    if request.approval_id.trim().is_empty() {
        return Err(AppError::Validation("审批单不能为空".to_string()));
    }
    let status = if request.approve {
        "approved"
    } else {
        "rejected"
    };
    let approval = approval_repository::decide_approval_request(
        conn,
        &request.approval_id,
        status,
        decided_by,
        request.decision_note,
    )?;
    conn.execute(
        "INSERT INTO audit_logs (id, action, entity_type, entity_id, summary, operator)
         VALUES (?1, 'decide_approval_request', 'approval', ?2, ?3, ?4)",
        params![
            uuid::Uuid::new_v4().to_string(),
            approval.id,
            status,
            audit_operator
        ],
    )?;
    Ok(approval)
}

pub(crate) fn validate_create_request_on_conn(
    conn: &Connection,
    request: &CreateApprovalRequest,
) -> AppResult<()> {
    if request.entity_type.trim().is_empty() {
        return Err(AppError::Validation("审批类型不能为空".to_string()));
    }
    if request.entity_id.trim().is_empty() {
        return Err(AppError::Validation("审批对象不能为空".to_string()));
    }
    if request.reason.trim().is_empty() {
        return Err(AppError::Validation("申请原因不能为空".to_string()));
    }
    match request.entity_type.trim() {
        "budget_override" => validate_budget_override_entity(conn, request.entity_id.trim()),
        other => Err(AppError::Validation(format!("不支持的审批类型：{other}"))),
    }
}

fn validate_budget_override_entity(conn: &Connection, entity_id: &str) -> AppResult<()> {
    let (department_id, period_month) = entity_id
        .split_once(':')
        .ok_or_else(|| AppError::Validation("超预算审批对象必须是 部门ID:YYYY-MM".to_string()))?;
    if department_id.trim().is_empty() {
        return Err(AppError::Validation("超预算审批部门不能为空".to_string()));
    }
    if !valid_period_month(period_month.trim()) {
        return Err(AppError::Validation(
            "超预算审批月份必须是 YYYY-MM".to_string(),
        ));
    }
    let department = conn
        .query_row(
            "SELECT name, enabled FROM departments WHERE id = ?1",
            params![department_id.trim()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? == 1)),
        )
        .optional()?
        .ok_or_else(|| AppError::Validation("超预算审批部门不存在".to_string()))?;
    if !department.1 {
        return Err(AppError::Validation(format!(
            "超预算审批部门已停用：{}",
            department.0
        )));
    }
    Ok(())
}

fn valid_period_month(value: &str) -> bool {
    if value.len() != 7 {
        return false;
    }
    let Some((year, month)) = value.split_once('-') else {
        return false;
    };
    year.len() == 4
        && year.chars().all(|item| item.is_ascii_digit())
        && month
            .parse::<u32>()
            .is_ok_and(|month| (1..=12).contains(&month))
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::db::migrations;

    use super::*;

    #[test]
    fn create_approval_request_validates_supported_budget_entity() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();

        validate_create_request_on_conn(
            &conn,
            &CreateApprovalRequest {
                entity_type: "budget_override".to_string(),
                entity_id: "dept-admin-office:2026-06".to_string(),
                reason: "部门超预算领用".to_string(),
            },
        )
        .unwrap();

        let type_error = validate_create_request_on_conn(
            &conn,
            &CreateApprovalRequest {
                entity_type: "unknown".to_string(),
                entity_id: "dept-admin-office:2026-06".to_string(),
                reason: "测试".to_string(),
            },
        )
        .unwrap_err();
        assert!(type_error.to_string().contains("不支持的审批类型"));

        let month_error = validate_create_request_on_conn(
            &conn,
            &CreateApprovalRequest {
                entity_type: "budget_override".to_string(),
                entity_id: "dept-admin-office:2026-13".to_string(),
                reason: "测试".to_string(),
            },
        )
        .unwrap_err();
        assert!(month_error.to_string().contains("YYYY-MM"));

        conn.execute(
            "INSERT INTO departments (id, code, name, enabled)
             VALUES ('dept-disabled-approval', 'DAP', '停用审批部门', 0)",
            [],
        )
        .unwrap();
        let department_error = validate_create_request_on_conn(
            &conn,
            &CreateApprovalRequest {
                entity_type: "budget_override".to_string(),
                entity_id: "dept-disabled-approval:2026-06".to_string(),
                reason: "测试".to_string(),
            },
        )
        .unwrap_err();
        assert!(department_error.to_string().contains("已停用"));
    }

    #[test]
    fn create_and_decide_approval_request_bind_user_and_audit_operator() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&conn).unwrap();
        conn.execute(
            "INSERT INTO users (id, username, display_name, enabled)
             VALUES ('user-warehouse-test', 'warehouse-approval-service-test', '审批仓库员', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, display_name, enabled)
             VALUES ('user-admin-test', 'admin-approval-service-test', '审批管理员', 1)",
            [],
        )
        .unwrap();

        let approval = create_approval_request_on_conn(
            &conn,
            CreateApprovalRequest {
                entity_type: "budget_override".to_string(),
                entity_id: "dept-admin-office:2026-06".to_string(),
                reason: "部门超预算领用".to_string(),
            },
            Some("user-warehouse-test".to_string()),
            "warehouse-test",
        )
        .unwrap();
        let decided = decide_approval_request_on_conn(
            &conn,
            DecideApprovalRequest {
                approval_id: approval.id.clone(),
                approve: true,
                decision_note: Some("通过".to_string()),
            },
            Some("user-admin-test".to_string()),
            "admin-test",
        )
        .unwrap();

        assert_eq!(decided.status, "approved");
        assert_eq!(decided.requested_by.as_deref(), Some("user-warehouse-test"));
        assert_eq!(decided.decided_by.as_deref(), Some("user-admin-test"));
        let audit_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_logs
                 WHERE operator IN ('warehouse-test', 'admin-test')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 2);
    }
}
