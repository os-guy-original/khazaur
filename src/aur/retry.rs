use std::time::Duration;
use reqwest::{Response, StatusCode};
use anyhow::Result;
use tracing::{warn, debug};

/// Retry configuration for HTTP requests
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial backoff delay in milliseconds
    pub initial_backoff_ms: u64,
    /// Maximum backoff delay in milliseconds
    pub max_backoff_ms: u64,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 500,
            max_backoff_ms: 10000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Check if an HTTP status code is retryable
pub fn is_retryable_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::REQUEST_TIMEOUT           // 408
        | StatusCode::TOO_MANY_REQUESTS       // 429
        | StatusCode::INTERNAL_SERVER_ERROR   // 500
        | StatusCode::BAD_GATEWAY             // 502
        | StatusCode::SERVICE_UNAVAILABLE     // 503
        | StatusCode::GATEWAY_TIMEOUT         // 504
    )
}

/// Retry a request with exponential backoff
pub async fn retry_request<F, Fut>(
    operation: F,
    config: &RetryConfig,
) -> Result<Response>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<Response, reqwest::Error>>,
{
    let mut attempt = 0;
    let mut backoff_ms = config.initial_backoff_ms;

    loop {
        attempt += 1;
        debug!("Attempt {}/{}", attempt, config.max_retries + 1);

        match operation().await {
            Ok(response) => {
                let status = response.status();
                
                if status.is_success() {
                    debug!("Request successful on attempt {}", attempt);
                    return Ok(response);
                }

                if is_retryable_status(status) && attempt <= config.max_retries {
                    warn!(
                        "Received retryable status {} on attempt {}, retrying in {}ms...",
                        status, attempt, backoff_ms
                    );
                    
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    
                    // Exponential backoff
                    backoff_ms = ((backoff_ms as f64 * config.backoff_multiplier) as u64)
                        .min(config.max_backoff_ms);
                    
                    continue;
                } else {
                    // Non-retryable status or max retries exceeded
                    return Ok(response);
                }
            }
            Err(e) => {
                // Network errors are also retryable
                if attempt <= config.max_retries {
                    warn!(
                        "Network error on attempt {}: {}. Retrying in {}ms...",
                        attempt, e, backoff_ms
                    );
                    
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    
                    backoff_ms = ((backoff_ms as f64 * config.backoff_multiplier) as u64)
                        .min(config.max_backoff_ms);
                    
                    continue;
                } else {
                    return Err(e.into());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retryable_statuses() {
        assert!(is_retryable_status(StatusCode::BAD_GATEWAY));
        assert!(is_retryable_status(StatusCode::SERVICE_UNAVAILABLE));
        assert!(is_retryable_status(StatusCode::GATEWAY_TIMEOUT));
        assert!(!is_retryable_status(StatusCode::NOT_FOUND));
        assert!(!is_retryable_status(StatusCode::FORBIDDEN));
    }
}
