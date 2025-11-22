use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tokio::time::sleep;

/// Rate limiter to prevent overwhelming AUR servers
#[derive(Clone)]
pub struct RateLimiter {
    /// Semaphore to limit concurrent requests
    semaphore: Arc<Semaphore>,
    /// Track last request time
    last_request: Arc<Mutex<Instant>>,
    /// Minimum delay between requests
    min_delay: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_concurrent: usize, delay_ms: u64) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            last_request: Arc::new(Mutex::new(Instant::now())),
            min_delay: Duration::from_millis(delay_ms),
        }
    }

    /// Acquire permission to make a request
    /// This will block until:
    /// 1. A semaphore slot is available (limits concurrent requests)
    /// 2. Enough time has passed since the last request (enforces delay)
    pub async fn acquire(&self) -> RateLimitGuard {
        // Wait for semaphore slot
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();
        
        // Enforce minimum delay between requests
        let mut last = self.last_request.lock().await;
        let elapsed = last.elapsed();
        if elapsed < self.min_delay {
            sleep(self.min_delay - elapsed).await;
        }
        *last = Instant::now();
        drop(last); // Release lock
        
        RateLimitGuard { _permit: permit }
    }
}

/// Guard that holds the rate limit permit
pub struct RateLimitGuard {
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl Default for RateLimiter {
    fn default() -> Self {
        // Conservative defaults to avoid DDoS
        Self::new(10, 100) // 10 concurrent, 100ms delay
    }
}
