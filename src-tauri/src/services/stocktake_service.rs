use rust_xlsxwriter::{Format, Workbook};

use crate::app::state::AppState;
use crate::db::stocktake_repository;
use crate::domain::runtime::RuntimeMode;
use crate::domain::stocktake::{
    ConfirmStocktakeRequest, CreateStocktakeRequest, ExportStocktakeSheetRequest,
    ExportStocktakeSheetResult, StocktakeDetail, StocktakeDocument, UpdateStocktakeCountsRequest,
};
use crate::error::{AppError, AppResult};

pub fn create_stocktake(
    state: &AppState,
    mut request: CreateStocktakeRequest,
) -> AppResult<StocktakeDetail> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    crate::services::stock_service::normalize_business_datetime(
        &mut request.business_date,
        "盘点日期",
    )?;
    validate_create_request(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_create_stocktake(state, request);
    }
    state
        .db
        .with_conn_mut(|conn| stocktake_repository::create_stocktake(conn, request))
}

pub fn list_stocktakes(state: &AppState) -> AppResult<Vec<StocktakeDocument>> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_list_stocktakes(state);
    }
    state.db.with_conn(stocktake_repository::list_stocktakes)
}

pub fn get_stocktake_detail(state: &AppState, stocktake_id: String) -> AppResult<StocktakeDetail> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_get_stocktake_detail(state, stocktake_id);
    }
    state
        .db
        .with_conn(|conn| stocktake_repository::get_stocktake_detail(conn, &stocktake_id))
}

pub fn update_stocktake_counts(
    state: &AppState,
    request: UpdateStocktakeCountsRequest,
) -> AppResult<StocktakeDetail> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    validate_update_counts(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_update_stocktake_counts(state, request);
    }
    state
        .db
        .with_conn(|conn| stocktake_repository::update_stocktake_counts(conn, request))
}

pub fn confirm_stocktake(
    state: &AppState,
    request: ConfirmStocktakeRequest,
) -> AppResult<StocktakeDetail> {
    crate::services::user_service::require_permission(state, "write_stock")?;
    validate_confirm_request(&request)?;
    if runtime_mode(state)? == RuntimeMode::Client {
        return crate::services::host_service::remote_confirm_stocktake(state, request);
    }
    state
        .db
        .with_conn_mut(|conn| stocktake_repository::confirm_stocktake(conn, request))
}

pub fn export_stocktake_sheet(
    state: &AppState,
    request: ExportStocktakeSheetRequest,
) -> AppResult<ExportStocktakeSheetResult> {
    crate::services::user_service::require_permission(state, "view_reports")?;
    if request.stocktake_id.trim().is_empty() {
        return Err(AppError::Validation("盘点单不能为空".to_string()));
    }
    let detail = get_stocktake_detail(state, request.stocktake_id)?;
    let file_name = format!(
        "Aster-盘点表-{}-{}.xlsx",
        safe_file_part(&detail.document.business_date),
        safe_file_part(&detail.document.document_no)
    );
    let export_dir = crate::services::status_service::effective_export_dir(state)?;
    std::fs::create_dir_all(&export_dir)?;
    let path = export_dir.join(file_name);
    write_stocktake_workbook(&detail, &path)?;
    Ok(ExportStocktakeSheetResult {
        path: path.display().to_string(),
    })
}

pub(crate) fn validate_create_request(request: &CreateStocktakeRequest) -> AppResult<()> {
    crate::services::stock_service::validate_business_datetime(&request.business_date, "盘点日期")?;
    crate::application::write_limits::validate_line_count(request.item_ids.len(), "盘点范围")?;
    match request.scope_type.as_str() {
        "all" => Ok(()),
        "category" => {
            if request
                .category_id
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
            {
                Err(AppError::Validation("分类盘点必须选择分类".to_string()))
            } else {
                Ok(())
            }
        }
        "custom" => {
            if request.item_ids.is_empty() {
                Err(AppError::Validation("自定义盘点必须选择物品".to_string()))
            } else {
                Ok(())
            }
        }
        other => Err(AppError::Validation(format!("不支持的盘点范围：{other}"))),
    }
}

pub(crate) fn validate_update_counts(request: &UpdateStocktakeCountsRequest) -> AppResult<()> {
    if request.stocktake_id.trim().is_empty() {
        return Err(AppError::Validation("盘点单不能为空".to_string()));
    }
    if request.lines.is_empty() {
        return Err(AppError::Validation("至少需要提交一行盘点数量".to_string()));
    }
    crate::application::write_limits::validate_line_count(request.lines.len(), "盘点录入")?;
    Ok(())
}

pub(crate) fn validate_confirm_request(request: &ConfirmStocktakeRequest) -> AppResult<()> {
    if request.stocktake_id.trim().is_empty() {
        return Err(AppError::Validation("盘点单不能为空".to_string()));
    }
    Ok(())
}

fn write_stocktake_workbook(detail: &StocktakeDetail, path: &std::path::Path) -> AppResult<()> {
    let mut workbook = Workbook::new();
    let header = Format::new().set_bold();
    let number = Format::new().set_num_format("0.00");
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("盘点表").map_err(xlsx_error)?;
    worksheet
        .write_string_with_format(0, 0, "盘点单号", &header)
        .map_err(xlsx_error)?;
    worksheet
        .write_string(0, 1, &detail.document.document_no)
        .map_err(xlsx_error)?;
    worksheet
        .write_string_with_format(0, 2, "盘点日期", &header)
        .map_err(xlsx_error)?;
    worksheet
        .write_string(0, 3, &detail.document.business_date)
        .map_err(xlsx_error)?;
    worksheet
        .write_string_with_format(2, 0, "编码", &header)
        .map_err(xlsx_error)?;
    worksheet
        .write_string_with_format(2, 1, "物品", &header)
        .map_err(xlsx_error)?;
    worksheet
        .write_string_with_format(2, 2, "规格", &header)
        .map_err(xlsx_error)?;
    worksheet
        .write_string_with_format(2, 3, "单位", &header)
        .map_err(xlsx_error)?;
    worksheet
        .write_string_with_format(2, 4, "账面数量", &header)
        .map_err(xlsx_error)?;
    worksheet
        .write_string_with_format(2, 5, "实盘数量", &header)
        .map_err(xlsx_error)?;
    worksheet
        .write_string_with_format(2, 6, "差异数量", &header)
        .map_err(xlsx_error)?;
    worksheet
        .write_string_with_format(2, 7, "备注", &header)
        .map_err(xlsx_error)?;

    for (index, line) in detail.lines.iter().enumerate() {
        let row = index as u32 + 3;
        worksheet
            .write_string(row, 0, &line.item_code)
            .map_err(xlsx_error)?;
        worksheet
            .write_string(row, 1, &line.item_name)
            .map_err(xlsx_error)?;
        worksheet
            .write_string(row, 2, line.spec.as_deref().unwrap_or(""))
            .map_err(xlsx_error)?;
        worksheet
            .write_string(row, 3, line.unit_name.as_deref().unwrap_or(""))
            .map_err(xlsx_error)?;
        worksheet
            .write_number_with_format(row, 4, line.book_quantity, &number)
            .map_err(xlsx_error)?;
        if let Some(counted) = line.counted_quantity {
            worksheet
                .write_number_with_format(row, 5, counted, &number)
                .map_err(xlsx_error)?;
        }
        worksheet
            .write_number_with_format(row, 6, line.difference_quantity, &number)
            .map_err(xlsx_error)?;
        worksheet
            .write_string(row, 7, line.remark.as_deref().unwrap_or(""))
            .map_err(xlsx_error)?;
    }
    worksheet.set_column_width(0, 16).map_err(xlsx_error)?;
    worksheet.set_column_width(1, 24).map_err(xlsx_error)?;
    worksheet.set_column_width(2, 18).map_err(xlsx_error)?;
    worksheet.set_column_width(7, 28).map_err(xlsx_error)?;
    workbook.save(path).map_err(xlsx_error)?;
    Ok(())
}

fn runtime_mode(state: &AppState) -> AppResult<RuntimeMode> {
    Ok(crate::services::status_service::get_runtime_config(state)?.mode)
}

fn xlsx_error(error: rust_xlsxwriter::XlsxError) -> AppError {
    AppError::Validation(format!("盘点表导出失败：{error}"))
}

fn safe_file_part(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if matches!(
                ch,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | ' '
            ) {
                '-'
            } else {
                ch
            }
        })
        .collect::<String>();
    sanitized
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::app::paths::AppPaths;
    use crate::app::state::AppState;
    use crate::db::connection::Db;
    use crate::db::repository;
    use crate::domain::users::{CurrentUser, Role};

    fn stocktake_test_state() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().expect("temp dir");
        let paths = AppPaths {
            data_dir: dir.path().to_path_buf(),
            database_path: dir.path().join("aster.sqlite"),
            backup_dir: dir.path().join("backups"),
            export_dir: dir.path().join("exports"),
            import_report_dir: dir.path().join("import-reports"),
        };
        std::fs::create_dir_all(&paths.backup_dir).unwrap();
        std::fs::create_dir_all(&paths.export_dir).unwrap();
        std::fs::create_dir_all(&paths.import_report_dir).unwrap();
        let user = CurrentUser {
            id: "user-stocktake".to_string(),
            username: "stocktake".to_string(),
            display_name: "盘点用户".to_string(),
            department_id: None,
            department_name: None,
            roles: vec![Role {
                id: "role-stocktake".to_string(),
                code: "admin".to_string(),
                name: "管理员".to_string(),
            }],
            permissions: vec!["write_stock".to_string(), "view_reports".to_string()],
        };
        let state = AppState {
            db: Db::initialize(&paths).unwrap(),
            paths,
            session: Arc::new(Mutex::new(None)),
            host_service: Arc::new(Mutex::new(
                crate::services::host_service::HostServiceRuntime::default(),
            )),
        };
        crate::services::test_support::install_session(&state, user).unwrap();
        (dir, state)
    }

    #[test]
    fn stocktake_export_file_name_is_windows_safe() {
        assert_eq!(safe_file_part("2026-06-30 09:10:11"), "2026-06-30-09-10-11");
        assert_eq!(safe_file_part("STK:2026/06|30"), "STK-2026-06-30");
    }

    #[test]
    fn export_stocktake_sheet_uses_default_export_dir_setting() {
        let (dir, state) = stocktake_test_state();
        let custom_export_dir = dir.path().join("stocktake-exports");
        state
            .db
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO master_items (id, code, name, unit_id, default_price)
                     VALUES ('item-stocktake-export', 'STE-001', '盘点导出物品', 'unit-piece', 5)",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO stock_balances (id, item_id, quantity, amount, average_price)
                     VALUES ('balance-stocktake-export', 'item-stocktake-export', 3, 15, 5)",
                    [],
                )?;
                repository::set_setting(
                    conn,
                    "default_export_dir",
                    &custom_export_dir.display().to_string(),
                )
            })
            .unwrap();
        let detail = create_stocktake(
            &state,
            CreateStocktakeRequest {
                business_date: "2026-06-30".to_string(),
                scope_type: "all".to_string(),
                category_id: None,
                item_ids: vec![],
                handler: Some("tester".to_string()),
                remark: Some("导出目录测试".to_string()),
            },
        )
        .unwrap();

        let result = export_stocktake_sheet(
            &state,
            ExportStocktakeSheetRequest {
                stocktake_id: detail.document.id,
            },
        )
        .unwrap();

        assert!(std::path::Path::new(&result.path).starts_with(&custom_export_dir));
        assert!(std::path::Path::new(&result.path).exists());
    }

    #[test]
    fn validate_create_request_requires_real_business_datetime_not_future() {
        let request = CreateStocktakeRequest {
            business_date: (chrono::Local::now().naive_local() + chrono::Duration::minutes(1))
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
            scope_type: "all".to_string(),
            category_id: None,
            item_ids: vec![],
            handler: Some("tester".to_string()),
            remark: None,
        };

        let error = validate_create_request(&request).unwrap_err();
        assert!(error.to_string().contains("盘点日期不能晚于当前时间"));
    }
}
