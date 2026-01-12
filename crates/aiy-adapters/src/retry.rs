//! Retry logic with exponential backoff for transient errors.
//!
//! This module provides shared retry functionality for adapter HTTP transports.

use std::future::Future;

use crate::traits::RetryConfig;

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
    use std::time::Duration;

    #[test]
    fn test_default_config() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 30000);
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_calculate_delay_exponential() {
        let config = RetryConfig::default();
        assert_eq!(config.calculate_delay(0), Duration::from_millis(1000)); // 1s
        assert_eq!(config.calculate_delay(1), Duration::from_millis(2000)); // 2s
        assert_eq!(config.calculate_delay(2), Duration::from_millis(4000)); // 4s
    }

    #[test]
    fn test_calculate_delay_respects_max_cap() {
        let config = RetryConfig {
            max_retries: 10,
            initial_delay_ms: 1000,
            max_delay_ms: 5000, // 5 second cap
            backoff_multiplier: 2.0,
        };
        // Attempt 3: 1000 * 2^3 = 8000ms, but capped at 5000ms
        assert_eq!(config.calculate_delay(3), Duration::from_millis(5000));
        // Attempt 4: 1000 * 2^4 = 16000ms, still capped at 5000ms
        assert_eq!(config.calculate_delay(4), Duration::from_millis(5000));
    }
}
