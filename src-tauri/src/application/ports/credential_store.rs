use crate::error::AppResult;

pub trait CredentialStore: Send + Sync {
    fn load_password(&self, scope: &str, username: &str) -> AppResult<Option<String>>;
    fn save_password(&self, scope: &str, username: &str, password: &str) -> AppResult<()>;
    fn delete_password(&self, scope: &str, username: &str) -> AppResult<()>;
}
