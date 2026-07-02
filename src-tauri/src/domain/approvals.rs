use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub status: String,
    pub requested_by: Option<String>,
    pub decided_by: Option<String>,
    pub reason: Option<String>,
    pub decision_note: Option<String>,
    pub created_at: String,
    pub decided_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApprovalRequest {
    pub entity_type: String,
    pub entity_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecideApprovalRequest {
    pub approval_id: String,
    pub approve: bool,
    pub decision_note: Option<String>,
}
