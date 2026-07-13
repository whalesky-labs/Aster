mod app;
mod application;
mod commands;
mod db;
mod domain;
mod error;
mod infrastructure;
mod services;

use app::state::AppState;
use commands::app_commands::{
    get_app_status, get_runtime_config, get_system_proxy_candidates, get_system_settings,
    list_audit_logs, prepare_update_settings_snapshot, save_system_settings, set_runtime_mode,
};
use commands::approval_commands::{
    create_approval_request, decide_approval_request, list_approval_requests,
};
use commands::backup_commands::{
    create_backup, list_backup_records, preview_restore_backup, restore_backup,
    set_second_backup_dir,
};
use commands::host_commands::{
    discover_hosts, get_host_service_status, list_client_connections, pair_with_host,
    remove_client_connection, save_client_config, start_host_service, test_host_connection,
};
use commands::import_commands::{export_import_template, preview_excel_import, run_excel_import};
use commands::master_data_commands::{
    export_items, list_budget_rules, list_categories, list_departments, list_items,
    list_supplier_purchase_records, list_suppliers, list_units, save_budget_rule, save_category,
    save_department, save_item, save_supplier, save_unit, set_budget_rule_enabled,
    set_category_enabled, set_department_enabled, set_item_enabled, set_supplier_enabled,
    set_unit_enabled,
};
use commands::report_commands::{export_monthly_report, get_report_bundle};
use commands::stock_commands::{
    confirm_stock_document_draft, get_stock_document_detail, list_stock_balances,
    list_stock_batches, list_stock_documents, list_stock_movements, save_stock_document_draft,
    submit_adjustment, submit_stock_document, void_stock_document,
};
use commands::stocktake_commands::{
    confirm_stocktake, create_stocktake, export_stocktake_sheet, get_stocktake_detail,
    list_stocktakes, update_stocktake_counts,
};
use commands::user_commands::{
    change_password, delete_login_credential, get_current_user, get_password_change_required,
    list_roles, list_user_accounts, load_saved_credential, login, logout,
    request_password_reset_code, reset_password_with_code, save_login_credential,
    save_user_account, set_user_account_enabled,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let app_state = AppState::initialize().expect("failed to initialize Aster application state");
    services::status_service::restore_update_settings_snapshot_if_needed(&app_state)
        .expect("failed to restore update settings snapshot");
    services::user_service::ensure_default_admin(&app_state)
        .expect("failed to initialize default admin user");
    let _ = services::backup_service::run_startup_backup_if_needed(&app_state);
    services::backup_service::start_interval_backup_worker(&app_state);
    services::host_service::ensure_host_service_for_mode(&app_state, env!("CARGO_PKG_VERSION"))
        .expect("failed to initialize host service");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            get_runtime_config,
            get_system_proxy_candidates,
            get_system_settings,
            list_audit_logs,
            prepare_update_settings_snapshot,
            save_system_settings,
            set_runtime_mode,
            list_approval_requests,
            create_approval_request,
            decide_approval_request,
            list_categories,
            save_category,
            set_category_enabled,
            list_units,
            save_unit,
            set_unit_enabled,
            list_departments,
            save_department,
            set_department_enabled,
            list_suppliers,
            list_supplier_purchase_records,
            save_supplier,
            set_supplier_enabled,
            list_items,
            export_items,
            save_item,
            set_item_enabled,
            list_budget_rules,
            save_budget_rule,
            set_budget_rule_enabled,
            submit_stock_document,
            save_stock_document_draft,
            confirm_stock_document_draft,
            submit_adjustment,
            void_stock_document,
            get_stock_document_detail,
            list_stock_documents,
            list_stock_balances,
            list_stock_batches,
            list_stock_movements,
            get_report_bundle,
            export_monthly_report,
            export_import_template,
            preview_excel_import,
            run_excel_import,
            create_backup,
            list_backup_records,
            set_second_backup_dir,
            preview_restore_backup,
            restore_backup,
            create_stocktake,
            list_stocktakes,
            get_stocktake_detail,
            update_stocktake_counts,
            confirm_stocktake,
            export_stocktake_sheet,
            login,
            logout,
            get_current_user,
            get_password_change_required,
            load_saved_credential,
            save_login_credential,
            delete_login_credential,
            list_user_accounts,
            list_roles,
            save_user_account,
            set_user_account_enabled,
            change_password,
            request_password_reset_code,
            reset_password_with_code,
            start_host_service,
            get_host_service_status,
            list_client_connections,
            remove_client_connection,
            save_client_config,
            test_host_connection,
            discover_hosts,
            pair_with_host
        ])
        .run(tauri::generate_context!())
        .expect("error while running Aster application");
}
