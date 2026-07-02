use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeMode {
    Standalone,
    Host,
    Client,
}

impl RuntimeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            RuntimeMode::Standalone => "standalone",
            RuntimeMode::Host => "host",
            RuntimeMode::Client => "client",
        }
    }

    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "standalone" => Ok(RuntimeMode::Standalone),
            "host" => Ok(RuntimeMode::Host),
            "client" => Ok(RuntimeMode::Client),
            other => Err(AppError::InvalidRuntimeMode(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfig {
    pub mode: RuntimeMode,
    pub host_address: Option<String>,
    pub host_port: u16,
    pub client_token: Option<String>,
    pub client_device_id: String,
    pub data_dir: String,
    pub database_path: String,
    pub backup_dir: String,
    pub export_dir: String,
    pub import_report_dir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostServiceStatus {
    pub running: bool,
    pub bind_address: String,
    pub port: u16,
    pub pair_code: Option<String>,
    pub client_count: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientConnectionInfo {
    pub id: String,
    pub client_name: String,
    pub client_device_id: String,
    pub client_ip: String,
    pub app_version: String,
    pub status: String,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostConnectionTestRequest {
    pub host_address: String,
    pub host_port: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostConnectionTestResult {
    pub ok: bool,
    pub message: String,
    pub app_name: Option<String>,
    pub app_version: Option<String>,
    pub schema_version: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostDiscoveryResult {
    pub host_address: String,
    pub host_port: u16,
    pub app_name: String,
    pub app_version: String,
    pub schema_version: i64,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveClientConfigRequest {
    pub host_address: String,
    pub host_port: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairWithHostRequest {
    pub pair_code: String,
    pub client_name: String,
    pub client_device_id: String,
}
