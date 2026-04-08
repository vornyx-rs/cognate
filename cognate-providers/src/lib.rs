//! Cognate Providers — concrete LLM provider implementations.
//!
//! # Providers
//!
//! | Provider | Struct | Chat | Stream | Tools | Embeddings |
//! |----------|--------|------|--------|-------|------------|
//! | OpenAI   | [`OpenAiProvider`] | ✓ | ✓ | ✓ | ✓ |
//! | Anthropic | [`AnthropicProvider`] | ✓ | ✓ | ✓ | — |
//!
//! # Resilience
//!
//! * All providers have built-in exponential-backoff retry via [`RetryConfig`].
//! * Token-bucket rate limiting is available on [`OpenAiProvider`] and
//!   [`AnthropicProvider`].
//! * [`FallbackProvider`] transparently retries with a secondary provider on
//!   any retryable error.
#![warn(missing_docs)]

use std::time::Duration;

pub mod anthropic;
pub mod costs;
pub mod fallback;
pub mod openai;
pub mod retry;
pub mod sse;

pub use anthropic::AnthropicProvider;
pub use costs::ModelCost;
pub use fallback::FallbackProvider;
pub use openai::OpenAiProvider;
pub use retry::{with_retry, RetryConfig};
pub use sse::SseStream;

/// Default request timeout applied when no explicit timeout is configured.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// Build a `reqwest` client with standard Cognate defaults.
pub fn create_http_client(timeout: Duration) -> cognate_core::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| cognate_core::Error::Configuration(e.to_string()))
}
