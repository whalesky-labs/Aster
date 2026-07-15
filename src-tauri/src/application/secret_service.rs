use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use crate::application::ports::CredentialStore;
use crate::db::connection::Db;
use crate::db::repository;
use crate::error::{AppError, AppResult};

#[cfg(not(test))]
use crate::infrastructure::credential_store::SystemCredentialStore;

const APPLICATION_SCOPE: &str = "application";
pub const CLIENT_TOKEN_SETTING: &str = "client_token";
pub const SMTP_PASSWORD_SETTING: &str = "smtp_password";
pub const SMTP_PASSWORD_CONFIGURED_SETTING: &str = "smtp_password_configured";

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ApplicationSecret {
    ClientToken,
    SmtpPassword,
}

#[derive(Default)]
struct SecretCache {
    client_token: Option<Option<String>>,
    smtp_password: Option<Option<String>>,
}

static SECRET_CACHE: LazyLock<Mutex<HashMap<String, SecretCache>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

impl SecretCache {
    fn get(&self, secret: ApplicationSecret) -> Option<Option<String>> {
        match secret {
            ApplicationSecret::ClientToken => self.client_token.clone(),
            ApplicationSecret::SmtpPassword => self.smtp_password.clone(),
        }
    }

    fn set(&mut self, secret: ApplicationSecret, value: Option<String>) {
        match secret {
            ApplicationSecret::ClientToken => self.client_token = Some(value),
            ApplicationSecret::SmtpPassword => self.smtp_password = Some(value),
        }
    }
}

impl ApplicationSecret {
    fn credential_name(self) -> &'static str {
        match self {
            Self::ClientToken => "client-pairing-token",
            Self::SmtpPassword => "smtp-password",
        }
    }

    fn legacy_setting(self) -> &'static str {
        match self {
            Self::ClientToken => CLIENT_TOKEN_SETTING,
            Self::SmtpPassword => SMTP_PASSWORD_SETTING,
        }
    }
}

pub fn load(db: &Db, secret: ApplicationSecret) -> AppResult<Option<String>> {
    let scope = credential_scope(db);
    if let Some(cached) = cache_lock()?
        .get(&scope)
        .and_then(|cache| cache.get(secret))
    {
        return Ok(cached);
    }
    let store = credential_store();
    let value = match store.load_password(&scope, secret.credential_name())? {
        Some(value) => Some(value),
        None => migrate_legacy_setting(db, &store, &scope, secret)?,
    };
    cache_lock()?
        .entry(scope)
        .or_default()
        .set(secret, value.clone());
    Ok(value)
}

pub fn save(db: &Db, secret: ApplicationSecret, value: &str) -> AppResult<()> {
    let scope = credential_scope(db);
    credential_store().save_password(&scope, secret.credential_name(), value)?;
    cache_lock()?
        .entry(scope)
        .or_default()
        .set(secret, Some(value.to_string()));
    Ok(())
}

pub fn delete(db: &Db, secret: ApplicationSecret) -> AppResult<()> {
    let scope = credential_scope(db);
    credential_store().delete_password(&scope, secret.credential_name())?;
    cache_lock()?.entry(scope).or_default().set(secret, None);
    Ok(())
}

fn migrate_legacy_setting(
    db: &Db,
    store: &dyn CredentialStore,
    scope: &str,
    secret: ApplicationSecret,
) -> AppResult<Option<String>> {
    let legacy = db
        .with_conn(|conn| repository::get_setting(conn, secret.legacy_setting()))?
        .filter(|value| !value.is_empty());
    let Some(value) = legacy else {
        return Ok(None);
    };
    store.save_password(scope, secret.credential_name(), &value)?;
    db.with_conn_mut(|conn| {
        let transaction = conn.transaction()?;
        repository::delete_setting(&transaction, secret.legacy_setting())?;
        if matches!(secret, ApplicationSecret::SmtpPassword) {
            repository::set_setting(&transaction, SMTP_PASSWORD_CONFIGURED_SETTING, "true")?;
        }
        transaction.commit()?;
        Ok(())
    })?;
    Ok(Some(value))
}

fn cache_lock() -> AppResult<std::sync::MutexGuard<'static, HashMap<String, SecretCache>>> {
    SECRET_CACHE
        .lock()
        .map_err(|_| AppError::Validation("安全凭据缓存异常".to_string()))
}

#[cfg(not(test))]
fn credential_scope(_db: &Db) -> String {
    APPLICATION_SCOPE.to_string()
}

#[cfg(test)]
fn credential_scope(db: &Db) -> String {
    format!("{APPLICATION_SCOPE}-test-{}", db.test_identity())
}

#[cfg(not(test))]
fn credential_store() -> SystemCredentialStore {
    SystemCredentialStore
}

#[cfg(test)]
fn credential_store() -> TestCredentialStore {
    TestCredentialStore
}

#[cfg(test)]
static TEST_CREDENTIALS: LazyLock<Mutex<HashMap<(String, String), String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(test)]
struct TestCredentialStore;

#[cfg(test)]
impl CredentialStore for TestCredentialStore {
    fn load_password(&self, scope: &str, username: &str) -> AppResult<Option<String>> {
        Ok(TEST_CREDENTIALS
            .lock()
            .map_err(|_| AppError::Validation("测试安全凭据缓存异常".to_string()))?
            .get(&(scope.to_string(), username.to_string()))
            .cloned())
    }

    fn save_password(&self, scope: &str, username: &str, password: &str) -> AppResult<()> {
        TEST_CREDENTIALS
            .lock()
            .map_err(|_| AppError::Validation("测试安全凭据缓存异常".to_string()))?
            .insert(
                (scope.to_string(), username.to_string()),
                password.to_string(),
            );
        Ok(())
    }

    fn delete_password(&self, scope: &str, username: &str) -> AppResult<()> {
        TEST_CREDENTIALS
            .lock()
            .map_err(|_| AppError::Validation("测试安全凭据缓存异常".to_string()))?
            .remove(&(scope.to_string(), username.to_string()));
        Ok(())
    }
}
