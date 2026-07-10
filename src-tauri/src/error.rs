use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("无法解析应用数据目录")]
    ProjectDirectoryUnavailable,
    #[error("IO 错误：{0}")]
    Io(#[from] std::io::Error),
    #[error("数据库错误：{0}")]
    Database(#[from] rusqlite::Error),
    #[error("不支持的运行模式：{0}")]
    InvalidRuntimeMode(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    PayloadTooLarge(String),
    #[error("{0}")]
    RequestHeaderTooLarge(String),
    #[error("{0}")]
    RateLimited(String),
    #[error("{0}")]
    Timeout(String),
    #[error("业务校验失败：{0}")]
    Validation(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
