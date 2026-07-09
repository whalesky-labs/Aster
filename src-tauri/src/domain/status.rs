use serde::{Deserialize, Serialize};

use crate::domain::runtime::RuntimeConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardMetrics {
    pub item_count: i64,
    pub department_count: i64,
    pub supplier_count: i64,
    pub current_stock_amount: f64,
    pub low_stock_count: i64,
    pub negative_stock_count: i64,
    pub this_month_inbound_amount: f64,
    pub this_month_outbound_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentOperation {
    pub id: String,
    pub occurred_at: String,
    pub business_type: String,
    pub item_name: String,
    pub quantity: f64,
    pub department_name: Option<String>,
    pub supplier_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogRow {
    pub id: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    pub summary: String,
    pub operator: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthStatus {
    pub database_ok: bool,
    pub stock_balance_consistency_ok: bool,
    pub stock_balance_issue_count: i64,
    pub latest_backup_at: Option<String>,
    pub latest_interval_backup_at: Option<String>,
    pub auto_backup_enabled: bool,
    pub interval_backup_enabled: bool,
    pub interval_backup_hours: i64,
    pub second_backup_ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSettings {
    pub hotel_name: String,
    pub current_period: String,
    pub default_month: String,
    pub allow_negative_stock: bool,
    pub quantity_decimals: i64,
    pub amount_decimals: i64,
    pub default_export_dir: String,
    pub default_backup_dir: String,
    pub auto_backup_enabled: bool,
    pub interval_backup_enabled: bool,
    pub interval_backup_hours: i64,
    pub smtp_enabled: bool,
    pub smtp_host: String,
    pub smtp_port: i64,
    pub smtp_username: String,
    pub smtp_from_email: String,
    pub smtp_from_name: String,
    pub smtp_password_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyCandidate {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSystemSettingsRequest {
    pub hotel_name: String,
    pub current_period: String,
    pub default_month: String,
    pub allow_negative_stock: bool,
    pub quantity_decimals: i64,
    pub amount_decimals: i64,
    pub default_export_dir: String,
    pub default_backup_dir: String,
    pub auto_backup_enabled: bool,
    pub interval_backup_enabled: bool,
    pub interval_backup_hours: i64,
    pub smtp_enabled: bool,
    pub smtp_host: String,
    pub smtp_port: i64,
    pub smtp_username: String,
    pub smtp_password: Option<String>,
    pub smtp_from_email: String,
    pub smtp_from_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub app_name: String,
    pub app_version: String,
    pub schema_version: i64,
    pub runtime: RuntimeConfig,
    pub latest_movement_month: Option<String>,
    pub metrics: DashboardMetrics,
    pub recent_operations: Vec<RecentOperation>,
    pub health: HealthStatus,
}
