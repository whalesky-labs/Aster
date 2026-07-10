use keyring::{Entry, Error as KeyringError};

use crate::application::ports::CredentialStore;
use crate::error::{AppError, AppResult};

const SERVICE_PREFIX: &str = "com.aster.inventory.login";

#[derive(Default)]
pub struct SystemCredentialStore;

impl CredentialStore for SystemCredentialStore {
    fn load_password(&self, scope: &str, username: &str) -> AppResult<Option<String>> {
        match entry(scope, username)?.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(error) => Err(credential_error("读取", error)),
        }
    }

    fn save_password(&self, scope: &str, username: &str, password: &str) -> AppResult<()> {
        entry(scope, username)?
            .set_password(password)
            .map_err(|error| credential_error("保存", error))
    }

    fn delete_password(&self, scope: &str, username: &str) -> AppResult<()> {
        match entry(scope, username)?.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(error) => Err(credential_error("删除", error)),
        }
    }
}

fn entry(scope: &str, username: &str) -> AppResult<Entry> {
    let scope = scope.trim();
    let username = username.trim().to_lowercase();
    if scope.is_empty() || username.is_empty() {
        return Err(AppError::Validation(
            "安全凭据范围和用户名不能为空".to_string(),
        ));
    }
    Entry::new(&format!("{SERVICE_PREFIX}.{scope}"), &username)
        .map_err(|error| credential_error("初始化", error))
}

fn credential_error(action: &str, error: KeyringError) -> AppError {
    AppError::Validation(format!("{action}系统安全凭据失败：{error}"))
}
