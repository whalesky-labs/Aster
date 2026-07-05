use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub enabled: bool,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveCategoryRequest {
    pub id: Option<String>,
    pub expected_updated_at: Option<String>,
    pub parent_id: Option<String>,
    pub name: String,
    pub enabled: bool,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unit {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveUnitRequest {
    pub id: Option<String>,
    pub expected_updated_at: Option<String>,
    pub name: String,
    pub enabled: bool,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Department {
    pub id: String,
    pub code: String,
    pub name: String,
    pub manager: Option<String>,
    pub enabled: bool,
    pub sort_order: i64,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDepartmentRequest {
    pub id: Option<String>,
    pub expected_updated_at: Option<String>,
    pub code: String,
    pub name: String,
    pub manager: Option<String>,
    pub enabled: bool,
    pub sort_order: i64,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Supplier {
    pub id: String,
    pub name: String,
    pub contact: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub enabled: bool,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierPurchaseRecord {
    pub movement_date: String,
    pub document_no: Option<String>,
    pub item_code: String,
    pub item_name: String,
    pub spec: Option<String>,
    pub unit_name: Option<String>,
    pub quantity: f64,
    pub unit_price: f64,
    pub amount: f64,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSupplierRequest {
    pub id: Option<String>,
    pub expected_updated_at: Option<String>,
    pub name: String,
    pub contact: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub enabled: bool,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub code: String,
    pub barcode: Option<String>,
    pub name: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub spec: Option<String>,
    pub unit_id: Option<String>,
    pub unit_name: Option<String>,
    pub default_price: f64,
    pub supplier_id: Option<String>,
    pub supplier_name: Option<String>,
    pub warning_quantity: f64,
    pub enabled: bool,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveItemRequest {
    pub id: Option<String>,
    pub expected_updated_at: Option<String>,
    pub code: Option<String>,
    pub barcode: Option<String>,
    pub name: String,
    pub category_id: Option<String>,
    pub spec: Option<String>,
    pub unit_id: Option<String>,
    pub default_price: f64,
    pub supplier_id: Option<String>,
    pub warning_quantity: f64,
    pub enabled: bool,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetRule {
    pub id: String,
    pub department_id: String,
    pub department_name: String,
    pub category_id: Option<String>,
    pub category_name: String,
    pub period_month: String,
    pub amount_limit: f64,
    pub used_amount: f64,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveBudgetRuleRequest {
    pub id: Option<String>,
    pub expected_updated_at: Option<String>,
    pub department_id: String,
    pub category_id: Option<String>,
    pub period_month: String,
    pub amount_limit: f64,
    pub enabled: bool,
}
