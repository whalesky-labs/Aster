use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::error::{AppError, AppResult};

#[derive(Default)]
pub struct SecurityRateLimiter {
    windows: HashMap<String, AttemptWindow>,
}

struct AttemptWindow {
    started_at: Instant,
    attempts: usize,
}

impl SecurityRateLimiter {
    pub fn check(&mut self, operation: &str, source: &str) -> AppResult<()> {
        let (maximum, duration) = policy(operation);
        let key = format!("{operation}:{source}");
        let now = Instant::now();
        self.windows
            .retain(|_, window| now.duration_since(window.started_at) < duration);
        let window = self.windows.entry(key).or_insert(AttemptWindow {
            started_at: now,
            attempts: 0,
        });
        if now.duration_since(window.started_at) >= duration {
            *window = AttemptWindow {
                started_at: now,
                attempts: 0,
            };
        }
        if window.attempts >= maximum {
            return Err(AppError::RateLimited(
                "安全操作尝试过于频繁，请稍后再试".to_string(),
            ));
        }
        window.attempts += 1;
        Ok(())
    }

    pub fn clear(&mut self, operation: &str, source: &str) {
        self.windows.remove(&format!("{operation}:{source}"));
    }
}

fn policy(operation: &str) -> (usize, Duration) {
    match operation {
        "pair" => (5, Duration::from_secs(10 * 60)),
        "password-reset" => (5, Duration::from_secs(15 * 60)),
        _ => (5, Duration::from_secs(15 * 60)),
    }
}

#[cfg(test)]
mod tests {
    use super::SecurityRateLimiter;

    #[test]
    fn isolates_sources_and_rejects_attempts_over_limit() {
        let mut limiter = SecurityRateLimiter::default();
        for _ in 0..5 {
            limiter.check("login", "device-a").expect("allowed");
        }
        assert!(limiter.check("login", "device-a").is_err());
        assert!(limiter.check("login", "device-b").is_ok());
    }

    #[test]
    fn successful_operation_can_clear_its_failure_window() {
        let mut limiter = SecurityRateLimiter::default();
        for _ in 0..5 {
            limiter.check("login", "device-a").expect("allowed");
        }
        limiter.clear("login", "device-a");
        assert!(limiter.check("login", "device-a").is_ok());
    }
}
