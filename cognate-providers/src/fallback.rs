//! Transparent provider fallback.
//!
//! [`FallbackProvider`] tries the primary provider first and, on any
//! retryable error (rate limit, 5xx, timeout), transparently retries with
//! the fallback provider.

use async_trait::async_trait;
use cognate_core::{Chunk, Provider, Request, Response, Result};
use futures::stream::BoxStream;
use std::sync::Arc;

/// A provider that falls back to a secondary provider on retryable errors.
///
/// # Example
///
/// ```rust,no_run
/// use cognate_providers::{OpenAiProvider, AnthropicProvider, FallbackProvider};
/// use cognate_core::{Provider, Request, Message};
/// use std::sync::Arc;
///
/// # async fn run() -> cognate_core::Result<()> {
/// let primary = Arc::new(OpenAiProvider::new(std::env::var("OPENAI_API_KEY").unwrap())?);
/// let secondary = Arc::new(AnthropicProvider::new(std::env::var("ANTHROPIC_API_KEY").unwrap())?);
/// let provider = FallbackProvider::new(primary, secondary);
///
/// let resp = provider
///     .complete(Request::new().with_model("gpt-4o").with_message(Message::user("Hi")))
///     .await?;
/// println!("{}", resp.content());
/// # Ok(())
/// # }
/// ```
pub struct FallbackProvider {
    primary: Arc<dyn Provider>,
    fallback: Arc<dyn Provider>,
}

impl FallbackProvider {
    /// Create a new fallback pair.
    ///
    /// On any [`retryable`](cognate_core::Error::is_retryable) error from
    /// `primary`, `fallback` is tried instead.
    pub fn new(primary: Arc<dyn Provider>, fallback: Arc<dyn Provider>) -> Self {
        Self { primary, fallback }
    }
}

#[async_trait]
impl Provider for FallbackProvider {
    async fn complete(&self, req: Request) -> Result<Response> {
        match self.primary.complete(req.clone()).await {
            Ok(resp) => Ok(resp),
            Err(e) if e.is_retryable() => {
                tracing::warn!(error = %e, "cognate: primary provider failed, trying fallback");
                self.fallback.complete(req).await
            }
            Err(e) => Err(e),
        }
    }

    async fn stream(&self, req: Request) -> Result<BoxStream<'static, Result<Chunk>>> {
        match self.primary.stream(req.clone()).await {
            Ok(s) => Ok(s),
            Err(e) if e.is_retryable() => {
                tracing::warn!(error = %e, "cognate: primary provider failed on stream, trying fallback");
                self.fallback.stream(req).await
            }
            Err(e) => Err(e),
        }
    }
}
