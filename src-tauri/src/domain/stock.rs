use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitStockDocumentRequest {
    pub document_type: String,
    pub outbound_kind: Option<String>,
    pub business_date: String,
    pub department_id: Option<String>,
    pub supplier_id: Option<String>,
    pub handler: Option<String>,
    pub purpose: Option<String>,
    pub remark: Option<String>,
    pub approval_request_id: Option<String>,
    pub lines: Vec<SubmitStockDocumentLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveStockDocumentDraftRequest {
    pub document_id: Option<String>,
    pub document_type: String,
    pub outbound_kind: Option<String>,
    pub business_date: String,
    pub department_id: Option<String>,
    pub supplier_id: Option<String>,
    pub handler: Option<String>,
    pub purpose: Option<String>,
    pub remark: Option<String>,
    pub approval_request_id: Option<String>,
    pub lines: Vec<SubmitStockDocumentLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmStockDocumentDraftRequest {
    pub document_id: String,
    pub approval_request_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitStockDocumentLine {
    pub item_id: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub amount: Option<f64>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitAdjustmentRequest {
    pub business_date: String,
    pub adjustment_type: String,
    pub handler: Option<String>,
    pub reason: String,
    pub lines: Vec<SubmitAdjustmentLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitAdjustmentLine {
    pub item_id: String,
    pub direction: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub amount: Option<f64>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoidStockDocumentRequest {
    pub document_id: String,
    pub reason: String,
    pub handler: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockDocumentQuery {
    pub document_type: Option<String>,
    pub outbound_kind: Option<String>,
    pub month: Option<String>,
    pub department_id: Option<String>,
    pub supplier_id: Option<String>,
    pub item_id: Option<String>,
    pub handler: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockBalanceQuery {
    pub search: Option<String>,
    pub category_id: Option<String>,
    pub item_id: Option<String>,
    pub stock_status: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockMovementQuery {
    pub search: Option<String>,
    pub item_id: Option<String>,
    pub department_id: Option<String>,
    pub direction: Option<String>,
    pub movement_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockBatchRow {
    pub id: String,
    pub item_id: String,
    pub item_code: String,
    pub item_name: String,
    pub batch_no: String,
    pub inbound_date: String,
    pub supplier_name: Option<String>,
    pub original_quantity: f64,
    pub remaining_quantity: f64,
    pub unit_price: f64,
    pub original_amount: f64,
    pub remaining_amount: f64,
    pub status: String,
    pub source_document_no: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockDocument {
    pub id: String,
    pub document_no: String,
    pub document_type: String,
    pub outbound_kind: Option<String>,
    pub business_date: String,
    pub department_id: Option<String>,
    pub department_name: Option<String>,
    pub supplier_id: Option<String>,
    pub supplier_name: Option<String>,
    pub handler: Option<String>,
    pub purpose: Option<String>,
    pub approval_request_id: Option<String>,
    pub status: String,
    pub remark: Option<String>,
    pub total_quantity: f64,
    pub total_amount: f64,
    pub total_purchase_amount: f64,
    pub total_sale_amount: f64,
    pub total_cost_amount: f64,
    pub total_gross_profit: f64,
    pub item_summary: Option<String>,
    pub created_at: String,
    pub confirmed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockDocumentDetail {
    pub document: StockDocument,
    pub lines: Vec<StockDocumentLine>,
    pub batch_lines: Vec<StockDocumentBatchLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockDocumentLine {
    pub id: String,
    pub item_id: String,
    pub item_code: String,
    pub item_name: String,
    pub spec: Option<String>,
    pub unit_name: Option<String>,
    pub quantity: f64,
    pub unit_price: f64,
    pub amount: f64,
    pub purchase_unit_price: Option<f64>,
    pub purchase_amount: Option<f64>,
    pub sale_unit_price: Option<f64>,
    pub sale_amount: Option<f64>,
    pub cost_unit_price: Option<f64>,
    pub cost_amount: Option<f64>,
    pub gross_profit: Option<f64>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockDocumentBatchLine {
    pub id: String,
    pub item_id: String,
    pub item_code: String,
    pub item_name: String,
    pub batch_id: String,
    pub batch_no: String,
    pub inbound_date: String,
    pub supplier_name: Option<String>,
    pub direction: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub amount: f64,
    pub movement_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockBalanceRow {
    pub item_id: String,
    pub item_code: String,
    pub item_name: String,
    pub spec: Option<String>,
    pub unit_name: Option<String>,
    pub quantity: f64,
    pub amount: f64,
    pub average_price: f64,
    pub last_inbound_price: f64,
    pub warning_quantity: f64,
    pub stock_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockMovementRow {
    pub id: String,
    pub movement_date: String,
    pub item_code: String,
    pub item_name: String,
    pub direction: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub amount: f64,
    pub document_no: Option<String>,
    pub department_name: Option<String>,
    pub supplier_name: Option<String>,
    pub movement_type: String,
    pub operator: Option<String>,
    pub remark: Option<String>,
    pub created_at: String,
}
