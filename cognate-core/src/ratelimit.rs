//! Token bucket rate limiting implementation
//!
//! This module provides a simple, async-safe token bucket for rate limiting
//! LLM provider requests.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// A token bucket rate limiter
#[derive(Debug, Clone)]
pub struct TokenBucket {
    state: Arc<Mutex<BucketState>>,
}

#[derive(Debug)]
struct BucketState {
    last_refill: Instant,
    tokens: f64,
    capacity: f64,
    fill_rate: f64,
}

impl TokenBucket {
    /// Create a new token bucket
    ///
    /// # Arguments
    /// * `capacity` - Maximum number of tokens the bucket can hold
    /// * `fill_rate` - Number of tokens added to the bucket per second
    pub fn new(capacity: f64, fill_rate: f64) -> Self {
        Self {
            state: Arc::new(Mutex::new(BucketState {
                last_refill: Instant::now(),
                tokens: capacity,
                capacity,
                fill_rate,
            })),
        }
    }

    /// Try to acquire tokens from the bucket
    ///
    /// Returns true if tokens were acquired, false otherwise.
    pub async fn try_acquire(&self, amount: f64) -> bool {
        let mut state = self.state.lock().await;
        state.refill();

        if state.tokens >= amount {
            state.tokens -= amount;
            true
        } else {
            false
        }
    }

    /// Wait until tokens are available and acquire them
    pub async fn acquire(&self, amount: f64) -> Duration {
        loop {
            let mut state = self.state.lock().await;
            state.refill();

            if state.tokens >= amount {
                state.tokens -= amount;
                return Duration::from_secs(0);
            }

            let tokens_needed = amount - state.tokens;
            let wait_time = Duration::from_secs_f64(tokens_needed / state.fill_rate);

            // Drop lock before sleeping
            drop(state);
            tokio::time::sleep(wait_time).await;
        }
    }
}

impl BucketState {
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        self.tokens = (self.tokens + elapsed * self.fill_rate).min(self.capacity);
        self.last_refill = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_token_bucket() {
        let bucket = TokenBucket::new(10.0, 1.0);

        // Should be able to acquire 5 tokens immediately
        assert!(bucket.try_acquire(5.0).await);

        // Should be able to acquire another 5 tokens immediately
        assert!(bucket.try_acquire(5.0).await);

        // Should fail to acquire more tokens
        assert!(!bucket.try_acquire(1.0).await);

        // Wait for 1.1s to get 1 token
        tokio::time::sleep(Duration::from_millis(1100)).await;
        assert!(bucket.try_acquire(1.0).await);
    }
}
