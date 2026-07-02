use crate::app::paths::AppPaths;
use crate::db::connection::Db;
use crate::error::AppResult;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub paths: AppPaths,
    pub db: Db,
    pub session: Arc<Mutex<Option<crate::domain::users::CurrentUser>>>,
    pub host_service: Arc<Mutex<crate::services::host_service::HostServiceRuntime>>,
}

impl AppState {
    pub fn initialize() -> AppResult<Self> {
        let paths = AppPaths::resolve()?;
        let db = Db::initialize(&paths)?;
        Ok(Self {
            paths,
            db,
            session: Arc::new(Mutex::new(None)),
            host_service: Arc::new(Mutex::new(
                crate::services::host_service::HostServiceRuntime::default(),
            )),
        })
    }
}
