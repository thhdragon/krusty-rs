// src/web/rate_limiter.rs
//! In-memory, thread-safe rate limiter for authentication endpoints.
//! Limits login attempts per IP within a configurable time window.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct RateLimiter {
    // Map of IP -> (attempt count, first attempt timestamp)
    inner: Arc<Mutex<HashMap<IpAddr, (u32, Instant)>>>,
    pub max_attempts: u32,
    pub window: Duration,
}

impl RateLimiter {
    pub fn new(max_attempts: u32, window: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_attempts,
            window,
        }
    }

    /// Returns true if the IP is allowed to attempt login, false if rate limited.
    pub async fn check_and_increment(&self, ip: IpAddr) -> bool {
        let mut map = self.inner.lock().await;
        let now = Instant::now();
        let entry = map.entry(ip).or_insert((0, now));
        // If window expired, reset
        if now.duration_since(entry.1) > self.window {
            *entry = (1, now);
            return true;
        }
        if entry.0 < self.max_attempts {
            entry.0 += 1;
            true
        } else {
            false
        }
    }

    /// Optionally: clear old entries to prevent unbounded growth
    pub async fn cleanup(&self) {
        let mut map = self.inner.lock().await;
        let now = Instant::now();
        map.retain(|_, &mut (_, ts)| now.duration_since(ts) < self.window * 2);
    }
}
