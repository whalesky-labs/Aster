use crate::error::{AppError, AppResult};

pub fn validate_line_count(count: usize, label: &str) -> AppResult<()> {
    if count > 2_000 {
        return Err(AppError::PayloadTooLarge(format!(
            "{label}一次最多提交 2000 行"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_line_count;

    #[test]
    fn accepts_limit_and_rejects_one_over() {
        assert!(validate_line_count(2_000, "单据").is_ok());
        assert!(validate_line_count(2_001, "单据").is_err());
    }
}
