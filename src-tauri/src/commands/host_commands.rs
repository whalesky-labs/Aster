use tauri::{AppHandle, State};

use crate::app::state::AppState;
use crate::domain::runtime::{
    ClientConnectionInfo, HostConnectionTestRequest, HostConnectionTestResult, HostDiscoveryResult,
    HostServiceStatus, PairWithHostRequest, RuntimeConfig, SaveClientConfigRequest,
};
use crate::error::AppResult;
use crate::services::host_service;

#[tauri::command]
pub fn start_host_service(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<HostServiceStatus> {
    let version = app.package_info().version.to_string();
    host_service::start_host_service(&state, &version)
}

#[tauri::command]
pub fn get_host_service_status(state: State<'_, AppState>) -> AppResult<HostServiceStatus> {
    Ok(host_service::get_host_service_status(&state))
}

#[tauri::command]
pub fn list_client_connections(state: State<'_, AppState>) -> AppResult<Vec<ClientConnectionInfo>> {
    host_service::list_client_connections(&state)
}

#[tauri::command]
pub fn save_client_config(
    state: State<'_, AppState>,
    request: SaveClientConfigRequest,
) -> AppResult<RuntimeConfig> {
    host_service::save_client_config(&state, request)
}

#[tauri::command]
pub fn test_host_connection(
    request: HostConnectionTestRequest,
) -> AppResult<HostConnectionTestResult> {
    host_service::test_host_connection(request)
}

#[tauri::command]
pub fn discover_hosts(host_port: u16) -> AppResult<Vec<HostDiscoveryResult>> {
    host_service::discover_hosts(host_port)
}

#[tauri::command]
pub fn pair_with_host(
    app: AppHandle,
    state: State<'_, AppState>,
    request: PairWithHostRequest,
) -> AppResult<RuntimeConfig> {
    let version = app.package_info().version.to_string();
    host_service::pair_with_host(
        &state,
        request.pair_code,
        request.client_name,
        request.client_device_id,
        version,
    )
}
