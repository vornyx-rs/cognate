//! Retry logic with exponential backoff
//!
//! Provides utilities for retrying failed LLM provider requests
//! with customizable backoff strategies.

use cognate_core::{Error, Result};
use futures::Future;
use std::time::Duration;

/// Configuration for exponential-backoff retry logic.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts after the initial failure.
    pub max_retries: u32,
    /// Minimum delay between retries.
    pub min_delay: Duration,
    /// Maximum delay cap — backoff will not exceed this value.
    pub max_delay: Duration,
    /// Exponential backoff multiplier applied after each failure.
    pub factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            min_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            factor: 2.0,
        }
    }
}

/// Execute a request with retries
pub async fn with_retry<F, Fut, T>(config: &RetryConfig, mut f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut last_error = None;
    let mut delay = config.min_delay;

    for i in 0..=config.max_retries {
        match f().await {
            Ok(res) => return Ok(res),
            Err(e) if e.is_retryable() && i < config.max_retries => {
                let actual_delay = e.retry_after().map(Duration::from_secs).unwrap_or(delay);

                tokio::time::sleep(actual_delay).await;

                // Update delay for next iteration (exponential backoff)
                delay = Duration::from_secs_f64(
                    (delay.as_secs_f64() * config.factor).min(config.max_delay.as_secs_f64()),
                );
                last_error = Some(e);
            }
            Err(e) => return Err(e),
        }
    }

    Err(last_error.unwrap_or_else(|| Error::RetryExhausted(config.max_retries)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_success() {
        let config = RetryConfig::default();
        let counter = Arc::new(AtomicU32::new(0));

        let result = with_retry(&config, || {
            let counter = counter.clone();
            async move {
                let val = counter.fetch_add(1, Ordering::SeqCst);
                if val < 2 {
                    Err(Error::Timeout(1))
                } else {
                    Ok("success")
                }
            }
        })
        .await;

        assert_eq!(result.unwrap(), "success");
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_failure() {
        let config = RetryConfig {
            max_retries: 2,
            ..Default::default()
        };
        let counter = Arc::new(AtomicU32::new(0));

        let result: Result<()> = with_retry(&config, || {
            let counter = counter.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Err(Error::Timeout(1))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
}
