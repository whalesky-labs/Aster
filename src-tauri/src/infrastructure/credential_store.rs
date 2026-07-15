use keyring::{Entry, Error as KeyringError};

use crate::application::ports::CredentialStore;
use crate::error::{AppError, AppResult};

const SERVICE_PREFIX: &str = "com.aster.inventory.secure";
const LEGACY_SERVICE_PREFIX: &str = "com.aster.inventory.login";

#[derive(Default)]
pub struct SystemCredentialStore;

impl CredentialStore for SystemCredentialStore {
    fn load_password(&self, scope: &str, username: &str) -> AppResult<Option<String>> {
        let current = entry(SERVICE_PREFIX, scope, username)?;
        match current.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(KeyringError::NoEntry) => {
                let legacy = entry(LEGACY_SERVICE_PREFIX, scope, username)?;
                match legacy.get_password() {
                    Ok(password) => {
                        current
                            .set_password(&password)
                            .map_err(|error| credential_error("迁移", error))?;
                        let _ = legacy.delete_credential();
                        Ok(Some(password))
                    }
                    Err(KeyringError::NoEntry) => Ok(None),
                    Err(error) => Err(credential_error("读取旧版", error)),
                }
            }
            Err(error) => Err(credential_error("读取", error)),
        }
    }

    fn save_password(&self, scope: &str, username: &str, password: &str) -> AppResult<()> {
        entry(SERVICE_PREFIX, scope, username)?
            .set_password(password)
            .map_err(|error| credential_error("保存", error))?;
        if let Ok(legacy) = entry(LEGACY_SERVICE_PREFIX, scope, username) {
            let _ = legacy.delete_credential();
        }
        Ok(())
    }

    fn delete_password(&self, scope: &str, username: &str) -> AppResult<()> {
        for prefix in [SERVICE_PREFIX, LEGACY_SERVICE_PREFIX] {
            match entry(prefix, scope, username)?.delete_credential() {
                Ok(()) | Err(KeyringError::NoEntry) => {}
                Err(error) => return Err(credential_error("删除", error)),
            }
        }
        Ok(())
    }
}

fn entry(prefix: &str, scope: &str, username: &str) -> AppResult<Entry> {
    let scope = scope.trim();
    let username = username.trim().to_lowercase();
    if scope.is_empty() || username.is_empty() {
        return Err(AppError::Validation(
            "安全凭据范围和用户名不能为空".to_string(),
        ));
    }
    Entry::new(&format!("{prefix}.{scope}"), &username)
        .map_err(|error| credential_error("初始化", error))
}

fn credential_error(action: &str, error: KeyringError) -> AppError {
    AppError::Validation(format!("{action}系统安全凭据失败：{error}"))
}
