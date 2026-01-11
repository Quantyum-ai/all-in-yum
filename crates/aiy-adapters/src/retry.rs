//! Retry logic with exponential backoff for transient errors.
//!
//! This module provides shared retry functionality for adapter HTTP transports.

use std::future::Future;
use std::time::Duration;

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (default: 3)
    pub max_retries: u32,
    /// Initial delay in milliseconds before first retry (default: 1000ms)
    pub initial_delay_ms: u64,
    /// Multiplier for exponential backoff (default: 2.0)
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Create config with custom max retries
    pub fn with_max_retries(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    /// Calculate delay for given attempt number (0-indexed)
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay_ms = self.initial_delay_ms as f64
            * self.backoff_multiplier.powi(attempt as i32);
        Duration::from_millis(delay_ms as u64)
    }
}

/// Execute operation with retry logic
pub async fn with_retry<F, Fut, T, E>(
    mut operation: F,
    should_retry: fn(&E) -> bool,
    config: &RetryConfig,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut last_error = None;

    for attempt in 0..=config.max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(err) => {
                // Check if we should retry
                if attempt < config.max_retries && should_retry(&err) {
                    let delay = config.calculate_delay(attempt);
                    tokio::time::sleep(delay).await;
                    last_error = Some(err);
                    continue;
                } else {
                    return Err(err);
                }
            }
        }
    }

    // Should never reach here but handle it
    Err(last_error.expect("No error after retry loop"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_calculate_delay_exponential() {
        let config = RetryConfig::default();
        assert_eq!(config.calculate_delay(0), Duration::from_millis(1000)); // 1s
        assert_eq!(config.calculate_delay(1), Duration::from_millis(2000)); // 2s
        assert_eq!(config.calculate_delay(2), Duration::from_millis(4000)); // 4s
    }
}
