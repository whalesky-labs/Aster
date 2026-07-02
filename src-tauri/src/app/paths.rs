use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub database_path: PathBuf,
    pub backup_dir: PathBuf,
    pub export_dir: PathBuf,
    pub import_report_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> AppResult<Self> {
        let project_dirs = ProjectDirs::from("com", "Aster", "Aster")
            .ok_or(AppError::ProjectDirectoryUnavailable)?;
        let data_dir = project_dirs.data_dir().to_path_buf();
        let database_path = data_dir.join("aster.sqlite");
        let backup_dir = data_dir.join("backups");
        let export_dir = data_dir.join("exports");
        let import_report_dir = data_dir.join("import-reports");

        fs::create_dir_all(&backup_dir)?;
        fs::create_dir_all(&export_dir)?;
        fs::create_dir_all(&import_report_dir)?;

        Ok(Self {
            data_dir,
            database_path,
            backup_dir,
            export_dir,
            import_report_dir,
        })
    }
}
