use crate::error::{AppError, AppResult};

pub fn normalize(value: &mut String, label: &str) -> AppResult<()> {
    *value = normalized(value, label)?;
    Ok(())
}

pub fn validate(value: &str, label: &str) -> AppResult<()> {
    normalized(value, label).map(|_| ())
}

fn normalized(value: &str, label: &str) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(format!("{label}不能为空")));
    }
    let normalized = trimmed.replace('T', " ");
    let datetime = if normalized.len() == 10 {
        chrono::NaiveDate::parse_from_str(&normalized, "%Y-%m-%d")
            .map_err(|_| AppError::Validation(format!("{label}格式必须是 YYYY-MM-DD HH:mm")))?
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| AppError::Validation(format!("{label}格式无效")))?
    } else if normalized.len() == 16 {
        chrono::NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%d %H:%M")
            .map_err(|_| AppError::Validation(format!("{label}格式必须是 YYYY-MM-DD HH:mm")))?
    } else {
        chrono::NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%d %H:%M:%S")
            .map_err(|_| AppError::Validation(format!("{label}格式必须是 YYYY-MM-DD HH:mm:ss")))?
    };
    if datetime > chrono::Local::now().naive_local() {
        return Err(AppError::Validation(format!("{label}不能晚于当前时间")));
    }
    Ok(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
}
