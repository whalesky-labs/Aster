use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateStocktakeRequest {
    pub business_date: String,
    pub scope_type: String,
    pub category_id: Option<String>,
    pub item_ids: Vec<String>,
    pub handler: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStocktakeCountsRequest {
    pub stocktake_id: String,
    pub lines: Vec<UpdateStocktakeLineRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStocktakeLineRequest {
    pub line_id: String,
    pub counted_quantity: Option<f64>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmStocktakeRequest {
    pub stocktake_id: String,
    pub handler: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportStocktakeSheetRequest {
    pub stocktake_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportStocktakeSheetResult {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StocktakeDocument {
    pub id: String,
    pub document_id: String,
    pub document_no: String,
    pub business_date: String,
    pub scope_type: String,
    pub status: String,
    pub handler: Option<String>,
    pub remark: Option<String>,
    pub line_count: i64,
    pub counted_count: i64,
    pub difference_count: i64,
    pub gain_amount: f64,
    pub loss_amount: f64,
    pub created_at: String,
    pub confirmed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StocktakeDetail {
    pub document: StocktakeDocument,
    pub lines: Vec<StocktakeLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StocktakeLine {
    pub id: String,
    pub stocktake_id: String,
    pub item_id: String,
    pub item_code: String,
    pub item_name: String,
    pub spec: Option<String>,
    pub unit_name: Option<String>,
    pub book_quantity: f64,
    pub counted_quantity: Option<f64>,
    pub difference_quantity: f64,
    pub average_price: f64,
    pub difference_amount: f64,
    pub remark: Option<String>,
}
