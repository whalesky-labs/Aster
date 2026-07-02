use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreviewRequest {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunImportRequest {
    pub path: String,
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub source_file: String,
    pub sheet_count: usize,
    pub row_count: usize,
    pub item_count: usize,
    pub new_item_count: usize,
    pub existing_item_count: usize,
    pub opening_quantity: f64,
    pub opening_amount: f64,
    pub inbound_quantity: f64,
    pub inbound_amount: f64,
    pub outbound_quantity: f64,
    pub outbound_amount: f64,
    pub document_count: usize,
    pub warnings: Vec<ImportMessage>,
    pub errors: Vec<ImportMessage>,
    pub items: Vec<ImportItemPreview>,
    pub months: Vec<ImportMonthPreview>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportItemPreview {
    pub name: String,
    pub category_name: Option<String>,
    pub spec: Option<String>,
    pub unit_name: Option<String>,
    pub default_price: f64,
    pub opening_quantity: f64,
    pub inbound_quantity: f64,
    pub outbound_quantity: f64,
    pub existing: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportMonthPreview {
    pub month: String,
    pub row_count: usize,
    pub opening_quantity: f64,
    pub inbound_quantity: f64,
    pub outbound_quantity: f64,
    pub outbound_amount: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportMessage {
    pub level: String,
    pub sheet: String,
    pub row: usize,
    pub column: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub job_id: String,
    pub source_file: String,
    pub imported_items: usize,
    pub matched_items: usize,
    pub document_count: usize,
    pub movement_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
    pub report_path: Option<String>,
    pub source_copy_path: Option<String>,
}
