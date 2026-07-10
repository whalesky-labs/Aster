use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    pub id: String,
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserAccount {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub department_id: Option<String>,
    pub department_name: Option<String>,
    pub enabled: bool,
    pub roles: Vec<Role>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentUser {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub department_id: Option<String>,
    pub department_name: Option<String>,
    pub roles: Vec<Role>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedCredentialRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedCredential {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveUserRequest {
    pub id: Option<String>,
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub password: Option<String>,
    pub department_id: Option<String>,
    pub enabled: bool,
    pub role_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    pub user_id: Option<String>,
    pub old_password: Option<String>,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestPasswordResetCodeRequest {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestPasswordResetCodeResponse {
    pub masked_email: String,
    pub expires_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordWithCodeRequest {
    pub username: String,
    pub code: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetUserEnabledRequest {
    pub user_id: String,
    pub enabled: bool,
}
