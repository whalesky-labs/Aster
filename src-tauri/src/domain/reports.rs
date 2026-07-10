use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportQuery {
    pub month: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub department_id: Option<String>,
    pub category_id: Option<String>,
    pub item_id: Option<String>,
    pub supplier_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyInventoryRow {
    pub item_id: String,
    pub item_code: String,
    pub item_name: String,
    pub spec: Option<String>,
    pub unit_name: Option<String>,
    pub inbound_quantity: f64,
    pub inbound_amount: f64,
    pub outbound_quantity: f64,
    pub outbound_amount: f64,
    pub ending_quantity: f64,
    pub ending_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepartmentIssueSummaryRow {
    pub department_id: String,
    pub department_name: String,
    pub quantity: f64,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepartmentIssueDetailRow {
    pub movement_date: String,
    pub department_name: String,
    pub outbound_kind: Option<String>,
    pub item_code: String,
    pub item_name: String,
    pub spec: Option<String>,
    pub unit_name: Option<String>,
    pub quantity: f64,
    pub unit_price: f64,
    pub amount: f64,
    pub sale_unit_price: Option<f64>,
    pub sale_amount: Option<f64>,
    pub cost_unit_price: f64,
    pub cost_amount: f64,
    pub gross_profit: Option<f64>,
    pub gross_margin: Option<f64>,
    pub document_no: Option<String>,
    pub purpose: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SalesProfitRow {
    pub movement_date: String,
    pub item_code: String,
    pub item_name: String,
    pub spec: Option<String>,
    pub unit_name: Option<String>,
    pub quantity: f64,
    pub sale_unit_price: f64,
    pub sale_amount: f64,
    pub cost_unit_price: f64,
    pub cost_amount: f64,
    pub gross_profit: f64,
    pub gross_margin: Option<f64>,
    pub negative_profit: bool,
    pub document_no: Option<String>,
    pub purpose: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryConsumptionRow {
    pub category_id: Option<String>,
    pub category_name: String,
    pub quantity: f64,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemConsumptionRow {
    pub item_id: String,
    pub item_code: String,
    pub item_name: String,
    pub spec: Option<String>,
    pub unit_name: Option<String>,
    pub quantity: f64,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboundDetailRow {
    pub movement_date: String,
    pub supplier_name: String,
    pub item_code: String,
    pub item_name: String,
    pub spec: Option<String>,
    pub unit_name: Option<String>,
    pub quantity: f64,
    pub unit_price: f64,
    pub amount: f64,
    pub document_no: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockWarningRow {
    pub item_id: String,
    pub item_code: String,
    pub item_name: String,
    pub spec: Option<String>,
    pub unit_name: Option<String>,
    pub quantity: f64,
    pub warning_quantity: f64,
    pub shortage_quantity: f64,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockBalanceReportRow {
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
pub struct StocktakeDifferenceReportRow {
    pub business_date: String,
    pub document_no: String,
    pub scope_type: String,
    pub status: String,
    pub item_code: String,
    pub item_name: String,
    pub spec: Option<String>,
    pub unit_name: Option<String>,
    pub book_quantity: f64,
    pub counted_quantity: f64,
    pub difference_quantity: f64,
    pub average_price: f64,
    pub difference_amount: f64,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportBundle {
    pub month: String,
    pub monthly_inventory: Vec<MonthlyInventoryRow>,
    pub department_summary: Vec<DepartmentIssueSummaryRow>,
    pub department_details: Vec<DepartmentIssueDetailRow>,
    pub category_consumption: Vec<CategoryConsumptionRow>,
    pub item_consumption_ranking: Vec<ItemConsumptionRow>,
    pub inbound_details: Vec<InboundDetailRow>,
    pub outbound_details: Vec<DepartmentIssueDetailRow>,
    pub sales_profit: Vec<SalesProfitRow>,
    pub stock_balances: Vec<StockBalanceReportRow>,
    pub stock_warnings: Vec<StockWarningRow>,
    pub stocktake_differences: Vec<StocktakeDifferenceReportRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportBundlePage {
    pub section: String,
    pub bundle: ReportBundle,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReportResult {
    pub path: String,
}
