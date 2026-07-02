use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBackupRequest {
    pub backup_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreBackupRequest {
    pub backup_file: String,
    pub confirmation: String,
    pub validation_token: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetSecondBackupDirRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupMetadata {
    pub app_name: String,
    pub app_version: String,
    pub schema_version: i64,
    pub created_at: String,
    pub backup_type: String,
    pub database_file: String,
    pub database_size: u64,
    pub database_sha256: String,
    pub source_os: String,
    pub source_host_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRecord {
    pub id: String,
    pub backup_file: String,
    pub backup_type: String,
    pub app_version: String,
    pub schema_version: i64,
    pub host_name: Option<String>,
    pub os: Option<String>,
    pub database_size: i64,
    pub sha256: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    pub backup_file: String,
    pub backup_type: String,
    pub created_at: String,
    pub schema_version: i64,
    pub source_host_name: String,
    pub source_os: String,
    pub database_size: u64,
    pub database_sha256: String,
    pub second_backup_file: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePreview {
    pub backup_file: String,
    pub metadata: BackupMetadata,
    pub valid: bool,
    pub message: String,
    pub validation_token: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub restored_from: String,
    pub protected_backup_file: String,
    pub schema_version: i64,
    pub integrity: String,
}
