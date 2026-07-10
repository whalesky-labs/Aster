use crate::app::state::AppState;
use crate::application::write_limits::validate_line_count;
use crate::db::stock_repository;
use crate::domain::runtime::RuntimeMode;
use crate::domain::stock::{
    ConfirmStockDocumentDraftRequest, SaveStockDocumentDraftRequest, StockBalanceQuery,
    StockBalanceRow, StockBatchRow, StockDocument, StockDocumentDetail, StockDocumentQuery,
    StockMovementQuery, StockMovementRow, SubmitAdjustmentRequest, SubmitStockDocumentRequest,
    VoidStockDocumentRequest,
};
use crate::error::{AppError, AppResult};

pub(crate) use crate::domain::business_datetime::{
    normalize as normalize_business_datetime, validate as validate_business_datetime,
};

pub fn submit_stock_document(
    state: &AppState,
    mut request: SubmitStockDocumentRequest,
) -> AppResult<StockDocumentDetail> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    normalize_business_datetime(&mut request.business_date, "业务日期")?;
    validate_document(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_submit_stock_document(state, request);
    }
    let allow_negative_stock = crate::services::status_service::allow_negative_stock(state)?;
    state.db.with_conn_mut(|conn| {
        stock_repository::submit_stock_document(conn, request, allow_negative_stock)
    })
}

pub fn save_stock_document_draft(
    state: &AppState,
    mut request: SaveStockDocumentDraftRequest,
) -> AppResult<StockDocumentDetail> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    normalize_business_datetime(&mut request.business_date, "业务日期")?;
    validate_draft_document(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_save_stock_document_draft(state, request);
    }
    state
        .db
        .with_conn_mut(|conn| stock_repository::save_stock_document_draft(conn, request))
}

pub fn confirm_stock_document_draft(
    state: &AppState,
    request: ConfirmStockDocumentDraftRequest,
) -> AppResult<StockDocumentDetail> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    validate_confirm_draft(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_confirm_stock_document_draft(state, request);
    }
    let allow_negative_stock = crate::services::status_service::allow_negative_stock(state)?;
    state.db.with_conn_mut(|conn| {
        stock_repository::confirm_stock_document_draft(conn, request, allow_negative_stock)
    })
}

pub fn submit_adjustment(
    state: &AppState,
    mut request: SubmitAdjustmentRequest,
) -> AppResult<StockDocumentDetail> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    normalize_business_datetime(&mut request.business_date, "调整日期")?;
    validate_adjustment(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_submit_adjustment(state, request);
    }
    state
        .db
        .with_conn_mut(|conn| stock_repository::submit_adjustment(conn, request))
}

pub fn void_stock_document(
    state: &AppState,
    request: VoidStockDocumentRequest,
) -> AppResult<StockDocumentDetail> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    validate_void_document(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_void_stock_document(state, request);
    }
    state
        .db
        .with_conn_mut(|conn| stock_repository::void_stock_document(conn, request))
}

pub fn list_stock_documents(
    state: &AppState,
    mut query: StockDocumentQuery,
) -> AppResult<Vec<StockDocument>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    query.department_id =
        crate::services::user_service::current_department_scope(state)?.or(query.department_id);
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_stock_documents(state, query);
    }
    state
        .db
        .with_conn(|conn| stock_repository::list_stock_documents(conn, query))
}

pub fn get_stock_document_detail(
    state: &AppState,
    document_id: String,
) -> AppResult<StockDocumentDetail> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_get_stock_document_detail(state, document_id);
    }
    state
        .db
        .with_conn(|conn| stock_repository::get_stock_document_detail(conn, &document_id))
}

pub fn list_stock_balances(
    state: &AppState,
    query: StockBalanceQuery,
) -> AppResult<Vec<StockBalanceRow>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_stock_balances(state, query);
    }
    state
        .db
        .with_conn(|conn| stock_repository::list_stock_balances(conn, query))
}

pub fn list_stock_batches(state: &AppState, item_id: String) -> AppResult<Vec<StockBatchRow>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_stock_batches(state, item_id);
    }
    state
        .db
        .with_conn(|conn| stock_repository::list_stock_batches(conn, &item_id))
}

pub fn list_stock_movements(
    state: &AppState,
    mut query: StockMovementQuery,
) -> AppResult<Vec<StockMovementRow>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    query.department_id =
        crate::services::user_service::current_department_scope(state)?.or(query.department_id);
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_stock_movements(state, query);
    }
    state
        .db
        .with_conn(|conn| stock_repository::list_stock_movements(conn, query))
}

fn runtime_mode(state: &AppState) -> AppResult<RuntimeMode> {
    Ok(crate::services::status_service::get_runtime_config(state)?.mode)
}

pub(crate) fn validate_document(request: &SubmitStockDocumentRequest) -> AppResult<()> {
    if request.document_type != "inbound" && request.document_type != "outbound" {
        return Err(AppError::Validation("单据类型必须是入库或出库".to_string()));
    }
    validate_business_datetime(&request.business_date, "业务日期")?;
    let outbound_kind = request
        .outbound_kind
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("internal");
    if request.document_type == "outbound" && !matches!(outbound_kind, "internal" | "guest_sale") {
        return Err(AppError::Validation(
            "出库类型必须是内部领用或客人销售".to_string(),
        ));
    }
    if request.document_type == "outbound"
        && outbound_kind == "internal"
        && request
            .department_id
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        return Err(AppError::Validation("出库/领用必须选择部门".to_string()));
    }
    if request.lines.is_empty() {
        return Err(AppError::Validation("单据至少需要一行物品".to_string()));
    }
    validate_line_count(request.lines.len(), "单据")?;
    for line in &request.lines {
        if line.item_id.trim().is_empty() {
            return Err(AppError::Validation("单据行缺少物品".to_string()));
        }
        if line.quantity <= 0.0 {
            return Err(AppError::Validation("数量必须大于 0".to_string()));
        }
        if line.unit_price < 0.0 {
            return Err(AppError::Validation("单价不能小于 0".to_string()));
        }
        if line.amount.is_some_and(|amount| amount < 0.0) {
            return Err(AppError::Validation("金额不能小于 0".to_string()));
        }
    }
    Ok(())
}

pub(crate) fn validate_draft_document(request: &SaveStockDocumentDraftRequest) -> AppResult<()> {
    validate_document(&SubmitStockDocumentRequest {
        document_type: request.document_type.clone(),
        outbound_kind: request.outbound_kind.clone(),
        business_date: request.business_date.clone(),
        department_id: request.department_id.clone(),
        supplier_id: request.supplier_id.clone(),
        handler: request.handler.clone(),
        purpose: request.purpose.clone(),
        remark: request.remark.clone(),
        approval_request_id: request.approval_request_id.clone(),
        lines: request.lines.clone(),
    })
}

pub(crate) fn validate_confirm_draft(request: &ConfirmStockDocumentDraftRequest) -> AppResult<()> {
    if request.document_id.trim().is_empty() {
        return Err(AppError::Validation("草稿单据不能为空".to_string()));
    }
    Ok(())
}

pub(crate) fn validate_adjustment(request: &SubmitAdjustmentRequest) -> AppResult<()> {
    validate_business_datetime(&request.business_date, "调整日期")?;
    match request.adjustment_type.as_str() {
        "gain" | "loss" | "damage" | "correction" => {}
        other => return Err(AppError::Validation(format!("不支持的调整类型：{other}"))),
    }
    if request.reason.trim().is_empty() {
        return Err(AppError::Validation("调整原因不能为空".to_string()));
    }
    if request.lines.is_empty() {
        return Err(AppError::Validation("调整单至少需要一行物品".to_string()));
    }
    validate_line_count(request.lines.len(), "调整单")?;
    for line in &request.lines {
        if line.item_id.trim().is_empty() {
            return Err(AppError::Validation("调整行缺少物品".to_string()));
        }
        if line.direction != "in" && line.direction != "out" {
            return Err(AppError::Validation("调整方向必须是增加或减少".to_string()));
        }
        match (request.adjustment_type.as_str(), line.direction.as_str()) {
            ("gain", "out") => {
                return Err(AppError::Validation("盘盈调整只能增加库存".to_string()));
            }
            ("loss", "in") => {
                return Err(AppError::Validation("盘亏调整只能减少库存".to_string()));
            }
            ("damage", "in") => {
                return Err(AppError::Validation("损耗调整只能减少库存".to_string()));
            }
            _ => {}
        }
        if line.quantity <= 0.0 {
            return Err(AppError::Validation("调整数量必须大于 0".to_string()));
        }
        if line.unit_price < 0.0 {
            return Err(AppError::Validation("调整单价不能小于 0".to_string()));
        }
        if line.amount.is_some_and(|amount| amount < 0.0) {
            return Err(AppError::Validation("调整金额不能小于 0".to_string()));
        }
    }
    Ok(())
}

pub(crate) fn validate_void_document(request: &VoidStockDocumentRequest) -> AppResult<()> {
    if request.document_id.trim().is_empty() {
        return Err(AppError::Validation("单据不能为空".to_string()));
    }
    if request.reason.trim().is_empty() {
        return Err(AppError::Validation("作废原因不能为空".to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::app::paths::AppPaths;
    use crate::app::state::AppState;
    use crate::db::connection::Db;
    use crate::domain::stock::{
        SubmitAdjustmentLine, SubmitAdjustmentRequest, SubmitStockDocumentLine,
        SubmitStockDocumentRequest,
    };
    use crate::domain::users::{CurrentUser, Role};

    use super::*;

    fn test_state() -> AppState {
        let dir = tempfile::tempdir().expect("temp dir").keep();
        let paths = AppPaths {
            data_dir: dir.to_path_buf(),
            database_path: dir.join("aster.sqlite"),
            backup_dir: dir.join("backups"),
            export_dir: dir.join("exports"),
            import_report_dir: dir.join("import-reports"),
        };
        std::fs::create_dir_all(&paths.backup_dir).unwrap();
        std::fs::create_dir_all(&paths.export_dir).unwrap();
        std::fs::create_dir_all(&paths.import_report_dir).unwrap();
        AppState {
            db: Db::initialize(&paths).unwrap(),
            paths,
            session: Arc::new(Mutex::new(None)),
            host_service: Arc::new(Mutex::new(
                crate::services::host_service::HostServiceRuntime::default(),
            )),
        }
    }

    fn sample_request() -> SubmitStockDocumentRequest {
        SubmitStockDocumentRequest {
            document_type: "inbound".to_string(),
            outbound_kind: None,
            business_date: "2026-06-30".to_string(),
            department_id: None,
            supplier_id: None,
            handler: None,
            purpose: None,
            remark: None,
            approval_request_id: None,
            lines: vec![SubmitStockDocumentLine {
                item_id: String::new(),
                quantity: 1.0,
                unit_price: 1.0,
                amount: None,
                remark: None,
            }],
        }
    }

    fn set_warehouse_user(state: &AppState) {
        let user = CurrentUser {
            id: "user-warehouse".to_string(),
            username: "warehouse".to_string(),
            display_name: "仓库员".to_string(),
            department_id: None,
            department_name: None,
            roles: vec![Role {
                id: "role-warehouse".to_string(),
                code: "warehouse".to_string(),
                name: "仓库员".to_string(),
            }],
            permissions: vec!["write_stock".to_string(), "view_reports".to_string()],
        };
        crate::services::test_support::install_session(state, user).unwrap();
    }

    fn set_department_viewer(state: &AppState, department_id: &str) {
        let user = CurrentUser {
            id: "user-department-viewer".to_string(),
            username: "dept-viewer".to_string(),
            display_name: "部门查看员".to_string(),
            department_id: Some(department_id.to_string()),
            department_name: Some("绑定部门".to_string()),
            roles: vec![Role {
                id: "role-department-viewer".to_string(),
                code: "department_viewer".to_string(),
                name: "部门查看员".to_string(),
            }],
            permissions: vec!["view_reports".to_string()],
        };
        crate::services::test_support::install_session(state, user).unwrap();
    }

    #[test]
    fn submit_stock_document_requires_write_stock_permission() {
        let state = test_state();
        let error = submit_stock_document(&state, sample_request()).unwrap_err();
        assert!(error.to_string().contains("请先登录"));
    }

    #[test]
    fn list_stock_balances_requires_view_reports_permission() {
        let state = test_state();
        let error = list_stock_balances(&state, StockBalanceQuery::default()).unwrap_err();
        assert!(error.to_string().contains("请先登录"));
    }

    #[test]
    fn submit_stock_document_checks_business_validation_after_permission() {
        let state = test_state();
        set_warehouse_user(&state);
        let error = submit_stock_document(&state, sample_request()).unwrap_err();
        assert!(error.to_string().contains("单据行缺少物品"));
    }

    #[test]
    fn normalize_business_datetime_accepts_datetime_local_and_rejects_future() {
        let mut value = "2026-06-30T14:25".to_string();
        normalize_business_datetime(&mut value, "业务日期").unwrap();
        assert_eq!(value, "2026-06-30 14:25:00");

        let mut future = (chrono::Local::now().naive_local() + chrono::Duration::minutes(1))
            .format("%Y-%m-%dT%H:%M")
            .to_string();
        let error = normalize_business_datetime(&mut future, "业务日期").unwrap_err();
        assert!(error.to_string().contains("不能晚于当前时间"));
    }

    #[test]
    fn submit_stock_document_rejects_negative_manual_amount() {
        let state = test_state();
        set_warehouse_user(&state);
        let mut request = sample_request();
        request.lines[0].item_id = "item-1".to_string();
        request.lines[0].amount = Some(-1.0);

        let error = submit_stock_document(&state, request).unwrap_err();
        assert!(error.to_string().contains("金额不能小于 0"));
    }

    #[test]
    fn validate_adjustment_enforces_type_direction_semantics() {
        let mut request = SubmitAdjustmentRequest {
            business_date: "2026-06-30".to_string(),
            adjustment_type: "gain".to_string(),
            handler: Some("tester".to_string()),
            reason: "盘盈".to_string(),
            lines: vec![SubmitAdjustmentLine {
                item_id: "item-1".to_string(),
                direction: "out".to_string(),
                quantity: 1.0,
                unit_price: 1.0,
                amount: None,
                remark: None,
            }],
        };

        let gain_error = validate_adjustment(&request).unwrap_err();
        assert!(gain_error.to_string().contains("盘盈调整只能增加库存"));

        request.adjustment_type = "loss".to_string();
        request.lines[0].direction = "in".to_string();
        let loss_error = validate_adjustment(&request).unwrap_err();
        assert!(loss_error.to_string().contains("盘亏调整只能减少库存"));

        request.adjustment_type = "damage".to_string();
        let damage_error = validate_adjustment(&request).unwrap_err();
        assert!(damage_error.to_string().contains("损耗调整只能减少库存"));

        request.adjustment_type = "correction".to_string();
        validate_adjustment(&request).expect("correction supports either direction");
    }

    #[test]
    fn department_viewer_stock_lists_are_scoped_to_bound_department() {
        let state = test_state();
        state
            .db
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO master_items (id, code, name, unit_id, default_price)
                     VALUES ('item-scope-test', 'SCOPE-001', '范围测试物品', 'unit-piece', 1)",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO stock_documents (
                       id, document_no, document_type, business_date, department_id, department_name, status
                     )
                     VALUES
                       ('doc-admin-scope', 'OUT-SCOPE-ADMIN', 'outbound', '2026-06-30', 'dept-admin-office', '行政办', 'confirmed'),
                       ('doc-restaurant-scope', 'OUT-SCOPE-REST', 'outbound', '2026-06-30', 'dept-restaurant', '餐饮', 'confirmed')",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO stock_document_lines (id, document_id, item_id, quantity, unit_price, amount)
                     VALUES
                       ('line-admin-scope', 'doc-admin-scope', 'item-scope-test', 1, 1, 1),
                       ('line-restaurant-scope', 'doc-restaurant-scope', 'item-scope-test', 1, 1, 1)",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO stock_movements (
                       id, movement_date, item_id, direction, quantity, unit_price, amount,
                       document_id, department_id, department_name, movement_type
                     )
                     VALUES
                       ('mov-admin-scope', '2026-06-30', 'item-scope-test', 'out', 1, 1, 1, 'doc-admin-scope', 'dept-admin-office', '行政办', 'outbound'),
                       ('mov-restaurant-scope', '2026-06-30', 'item-scope-test', 'out', 1, 1, 1, 'doc-restaurant-scope', 'dept-restaurant', '餐饮', 'outbound')",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        set_department_viewer(&state, "dept-admin-office");

        let docs = list_stock_documents(
            &state,
            StockDocumentQuery {
                department_id: Some("dept-restaurant".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        let movements = list_stock_movements(
            &state,
            StockMovementQuery {
                department_id: Some("dept-restaurant".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].department_id.as_deref(), Some("dept-admin-office"));
        assert_eq!(movements.len(), 1);
        assert_eq!(movements[0].department_name.as_deref(), Some("行政办"));
    }
}
