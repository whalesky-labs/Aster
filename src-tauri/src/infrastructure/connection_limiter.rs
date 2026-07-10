use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct ConnectionLimiter {
    state: Mutex<LimiterState>,
    global_maximum: usize,
    source_maximum: usize,
}

#[derive(Default)]
struct LimiterState {
    active: usize,
    by_source: HashMap<String, usize>,
}

pub struct ConnectionPermit {
    limiter: Arc<ConnectionLimiter>,
    source: String,
}

impl ConnectionLimiter {
    pub fn new(global_maximum: usize, source_maximum: usize) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(LimiterState::default()),
            global_maximum,
            source_maximum,
        })
    }

    pub fn try_acquire(self: &Arc<Self>, source: &str) -> Option<ConnectionPermit> {
        let mut state = self.state.lock().ok()?;
        let source_active = state.by_source.get(source).copied().unwrap_or_default();
        if state.active >= self.global_maximum || source_active >= self.source_maximum {
            return None;
        }
        state.active += 1;
        *state.by_source.entry(source.to_string()).or_default() += 1;
        Some(ConnectionPermit {
            limiter: Arc::clone(self),
            source: source.to_string(),
        })
    }
}

impl Drop for ConnectionPermit {
    fn drop(&mut self) {
        let Ok(mut state) = self.limiter.state.lock() else {
            return;
        };
        state.active = state.active.saturating_sub(1);
        let remove_source = state
            .by_source
            .get_mut(&self.source)
            .map(|active| {
                *active = active.saturating_sub(1);
                *active == 0
            })
            .unwrap_or(false);
        if remove_source {
            state.by_source.remove(&self.source);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ConnectionLimiter;

    #[test]
    fn enforces_global_and_per_source_limits_and_releases_permits() {
        let limiter = ConnectionLimiter::new(2, 1);
        let first = limiter.try_acquire("a").expect("first permit");
        assert!(limiter.try_acquire("a").is_none());
        let second = limiter.try_acquire("b").expect("second permit");
        assert!(limiter.try_acquire("c").is_none());
        drop(first);
        assert!(limiter.try_acquire("a").is_some());
        drop(second);
    }
}
