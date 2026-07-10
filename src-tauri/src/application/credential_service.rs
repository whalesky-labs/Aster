use crate::app::state::AppState;
use crate::application::ports::CredentialStore;
use crate::db::repository;
use crate::domain::runtime::RuntimeMode;
use crate::domain::users::{SavedCredential, SavedCredentialRequest};
use crate::error::{AppError, AppResult};

pub fn load(state: &AppState, username: String) -> AppResult<Option<SavedCredential>> {
    let store = crate::infrastructure::credential_store::SystemCredentialStore;
    load_from(state, &store, username)
}

pub fn save(state: &AppState, request: SavedCredentialRequest) -> AppResult<()> {
    let store = crate::infrastructure::credential_store::SystemCredentialStore;
    save_to(state, &store, request)
}

pub fn delete(state: &AppState, username: String) -> AppResult<()> {
    let store = crate::infrastructure::credential_store::SystemCredentialStore;
    delete_from(state, &store, username)
}

fn load_from(
    state: &AppState,
    store: &dyn CredentialStore,
    username: String,
) -> AppResult<Option<SavedCredential>> {
    let username = normalize_username(&username);
    if username.is_empty() {
        return Ok(None);
    }
    let scope = scope(state)?;
    Ok(store
        .load_password(&scope, &username)?
        .map(|password| SavedCredential { username, password }))
}

fn save_to(
    state: &AppState,
    store: &dyn CredentialStore,
    request: SavedCredentialRequest,
) -> AppResult<()> {
    let username = normalize_username(&request.username);
    if username.is_empty() || request.password.is_empty() {
        return Err(AppError::Validation("用户名和密码不能为空".to_string()));
    }
    store.save_password(&scope(state)?, &username, &request.password)
}

fn delete_from(state: &AppState, store: &dyn CredentialStore, username: String) -> AppResult<()> {
    let username = normalize_username(&username);
    if username.is_empty() {
        return Ok(());
    }
    store.delete_password(&scope(state)?, &username)
}

fn normalize_username(username: &str) -> String {
    username.trim().to_lowercase()
}

fn scope(state: &AppState) -> AppResult<String> {
    let config = crate::services::status_service::get_runtime_config(state)?;
    if config.mode == RuntimeMode::Client {
        if let Some(fingerprint) = state
            .db
            .with_conn(|conn| repository::get_setting(conn, "host_certificate_fingerprint"))?
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(format!("remote-{fingerprint}"));
        }
    }
    state.db.with_conn(|conn| {
        if let Some(scope) = repository::get_setting(conn, "credential_scope_id")? {
            return Ok(format!("local-{scope}"));
        }
        let scope = uuid::Uuid::new_v4().to_string();
        repository::set_setting(conn, "credential_scope_id", &scope)?;
        Ok(format!("local-{scope}"))
    })
}
