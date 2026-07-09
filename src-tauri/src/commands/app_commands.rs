use tauri::State;

use crate::app::state::AppState;
use crate::domain::runtime::{RuntimeConfig, RuntimeMode};
use crate::domain::status::{
    AppStatus, AuditLogRow, ProxyCandidate, SaveSystemSettingsRequest, SystemSettings,
};
use crate::error::AppResult;
use crate::services::status_service;

#[tauri::command]
pub fn get_runtime_config(state: State<'_, AppState>) -> AppResult<RuntimeConfig> {
    status_service::get_runtime_config(&state)
}

#[tauri::command]
pub fn set_runtime_mode(mode: String, state: State<'_, AppState>) -> AppResult<RuntimeConfig> {
    let mode = RuntimeMode::parse(&mode)?;
    status_service::set_runtime_mode(&state, mode)
}

#[tauri::command]
pub fn get_app_status(app: tauri::AppHandle, state: State<'_, AppState>) -> AppResult<AppStatus> {
    let version = app.package_info().version.to_string();
    status_service::get_app_status(&state, &version)
}

#[tauri::command]
pub fn get_system_settings(state: State<'_, AppState>) -> AppResult<SystemSettings> {
    status_service::get_system_settings(&state)
}

#[tauri::command]
pub fn list_audit_logs(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> AppResult<Vec<AuditLogRow>> {
    status_service::list_audit_logs(&state, limit)
}

#[tauri::command]
pub fn save_system_settings(
    state: State<'_, AppState>,
    request: SaveSystemSettingsRequest,
) -> AppResult<SystemSettings> {
    status_service::save_system_settings(&state, request)
}

#[tauri::command]
pub fn prepare_update_settings_snapshot(state: State<'_, AppState>) -> AppResult<()> {
    status_service::prepare_update_settings_snapshot(&state)
}

#[tauri::command]
pub fn get_system_proxy_candidates() -> Vec<ProxyCandidate> {
    let mut candidates = Vec::new();
    push_env_proxy_candidates(&mut candidates);
    push_platform_proxy_candidates(&mut candidates);
    push_common_local_proxy_candidates(&mut candidates);
    dedupe_proxy_candidates(candidates)
}

fn push_proxy_candidate(candidates: &mut Vec<ProxyCandidate>, label: &str, url: String) {
    let normalized = normalize_proxy_url(&url);
    if normalized.is_empty() {
        return;
    }
    candidates.push(ProxyCandidate {
        label: label.to_string(),
        url: normalized,
    });
}

fn normalize_proxy_url(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("socks5://")
        || trimmed.starts_with("socks5h://")
    {
        return trimmed.to_string();
    }
    format!("http://{trimmed}")
}

fn push_env_proxy_candidates(candidates: &mut Vec<ProxyCandidate>) {
    for key in [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        if let Ok(value) = std::env::var(key) {
            push_proxy_candidate(candidates, key, value);
        }
    }
}

#[cfg(target_os = "macos")]
fn push_platform_proxy_candidates(candidates: &mut Vec<ProxyCandidate>) {
    use std::process::Command;

    let output = Command::new("scutil").arg("--proxy").output();
    let Ok(output) = output else {
        return;
    };
    if !output.status.success() {
        return;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let https_enabled = proxy_flag(&text, "HTTPSEnable");
    let https_host = proxy_value(&text, "HTTPSProxy");
    let https_port = proxy_value(&text, "HTTPSPort");
    if https_enabled && !https_host.is_empty() && !https_port.is_empty() {
        push_proxy_candidate(
            candidates,
            "macOS HTTPS 系统代理",
            format!("http://{https_host}:{https_port}"),
        );
    }

    let http_enabled = proxy_flag(&text, "HTTPEnable");
    let http_host = proxy_value(&text, "HTTPProxy");
    let http_port = proxy_value(&text, "HTTPPort");
    if http_enabled && !http_host.is_empty() && !http_port.is_empty() {
        push_proxy_candidate(
            candidates,
            "macOS HTTP 系统代理",
            format!("http://{http_host}:{http_port}"),
        );
    }

    let socks_enabled = proxy_flag(&text, "SOCKSEnable");
    let socks_host = proxy_value(&text, "SOCKSProxy");
    let socks_port = proxy_value(&text, "SOCKSPort");
    if socks_enabled && !socks_host.is_empty() && !socks_port.is_empty() {
        push_proxy_candidate(
            candidates,
            "macOS SOCKS 系统代理",
            format!("socks5://{socks_host}:{socks_port}"),
        );
    }
}

#[cfg(target_os = "macos")]
fn proxy_flag(text: &str, key: &str) -> bool {
    proxy_value(text, key) == "1"
}

#[cfg(target_os = "macos")]
fn proxy_value(text: &str, key: &str) -> String {
    let Some(line) = text.lines().find(|line| line.trim_start().starts_with(key)) else {
        return String::new();
    };
    line.split(':').nth(1).unwrap_or("").trim().to_string()
}

#[cfg(target_os = "windows")]
fn push_platform_proxy_candidates(candidates: &mut Vec<ProxyCandidate>) {
    use std::process::Command;

    let output = Command::new("netsh")
        .args(["winhttp", "show", "proxy"])
        .output();
    let Ok(output) = output else {
        return;
    };
    if !output.status.success() {
        return;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for token in text.split_whitespace() {
        if let Some(proxy) = token.strip_prefix("http=") {
            push_proxy_candidate(candidates, "Windows WinHTTP HTTP 代理", proxy.to_string());
        } else if let Some(proxy) = token.strip_prefix("https=") {
            push_proxy_candidate(candidates, "Windows WinHTTP HTTPS 代理", proxy.to_string());
        } else if token.contains(':')
            && !token.contains('\\')
            && token.chars().any(|char| char.is_ascii_digit())
        {
            push_proxy_candidate(candidates, "Windows WinHTTP 代理", token.to_string());
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn push_platform_proxy_candidates(_candidates: &mut Vec<ProxyCandidate>) {}

fn push_common_local_proxy_candidates(candidates: &mut Vec<ProxyCandidate>) {
    for port in [7897, 7890, 7899, 1080, 10808, 20171] {
        push_proxy_candidate(
            candidates,
            "常见本地代理",
            format!("http://127.0.0.1:{port}"),
        );
    }
}

fn dedupe_proxy_candidates(candidates: Vec<ProxyCandidate>) -> Vec<ProxyCandidate> {
    let mut seen = std::collections::HashSet::new();
    candidates
        .into_iter()
        .filter(|candidate| seen.insert(candidate.url.clone()))
        .collect()
}
