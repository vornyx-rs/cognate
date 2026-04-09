//! Tower-inspired middleware system for Cognate providers.
//!
//! Wrap any [`Provider`] with additional behaviour — logging, metrics,
//! retries, or timeouts — without modifying the provider itself.
//!
//! # Example
//!
//! ```rust,no_run
//! use cognate_core::{Provider, ProviderExt, middleware::{TracingMiddleware, MiddlewareLayer}};
//! use std::sync::Arc;
//!
//! fn wrap<P: Provider + 'static>(provider: P) -> Arc<dyn Provider> {
//!     provider.with(MiddlewareLayer::new(TracingMiddleware::default()))
//! }
//! ```

use crate::{Chunk, Provider, Request, Response, Result};
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::sync::Arc;

// ─── Layer / Middleware traits ─────────────────────────────────────────────

/// A factory that wraps a [`Provider`] to produce a new [`Provider`].
pub trait Layer: Send + Sync {
    /// Wrap `provider` with this layer and return the decorated provider.
    fn layer(&self, provider: Arc<dyn Provider>) -> Arc<dyn Provider>;
}

/// A middleware that intercepts completion and streaming requests.
///
/// Default implementations delegate directly to `next`, so you only need
/// to override the methods you care about.
#[async_trait]
pub trait Middleware: Send + Sync {
    /// Intercept a non-streaming completion request.
    async fn complete(&self, req: Request, next: &dyn Provider) -> Result<Response> {
        next.complete(req).await
    }

    /// Intercept a streaming request.
    async fn stream(
        &self,
        req: Request,
        next: &dyn Provider,
    ) -> Result<BoxStream<'static, Result<Chunk>>> {
        next.stream(req).await
    }
}

// ─── MiddlewareProvider ────────────────────────────────────────────────────

/// A [`Provider`] that applies a [`Middleware`] before delegating to an inner
/// provider.
pub struct MiddlewareProvider<M> {
    middleware: M,
    inner: Arc<dyn Provider>,
}

#[async_trait]
impl<M: Middleware> Provider for MiddlewareProvider<M> {
    async fn complete(&self, req: Request) -> Result<Response> {
        self.middleware.complete(req, self.inner.as_ref()).await
    }

    async fn stream(&self, req: Request) -> Result<BoxStream<'static, Result<Chunk>>> {
        self.middleware.stream(req, self.inner.as_ref()).await
    }
}

// ─── MiddlewareLayer ───────────────────────────────────────────────────────

/// A [`Layer`] that applies a concrete [`Middleware`] implementation.
pub struct MiddlewareLayer<M> {
    middleware: M,
}

impl<M> MiddlewareLayer<M> {
    /// Create a new layer wrapping `middleware`.
    pub fn new(middleware: M) -> Self {
        Self { middleware }
    }
}

impl<M: Middleware + Clone + 'static> Layer for MiddlewareLayer<M> {
    fn layer(&self, provider: Arc<dyn Provider>) -> Arc<dyn Provider> {
        Arc::new(MiddlewareProvider {
            middleware: self.middleware.clone(),
            inner: provider,
        })
    }
}

// ─── ProviderExt ───────────────────────────────────────────────────────────

/// Extension trait that adds a fluent `.with(layer)` combinator to any [`Provider`].
pub trait ProviderExt: Provider {
    /// Wrap `self` with the given [`Layer`], returning a new [`Arc<dyn Provider>`].
    fn with<L: Layer>(self, layer: L) -> Arc<dyn Provider>
    where
        Self: Sized + 'static,
    {
        layer.layer(Arc::new(self))
    }
}

/// Blanket implementation so every [`Provider`] automatically gets `.with()`.
impl<P: Provider + Sized + 'static> ProviderExt for P {}

// ─── Built-in middleware ───────────────────────────────────────────────────

/// Middleware that logs every request and response using [`tracing`].
///
/// Add to any provider with:
/// ```rust,no_run
/// use cognate_core::{ProviderExt, middleware::{TracingMiddleware, MiddlewareLayer}};
/// # use cognate_core::MockProvider;
/// let provider = MockProvider::new()
///     .with(MiddlewareLayer::new(TracingMiddleware::default()));
/// ```
#[derive(Clone, Default)]
pub struct TracingMiddleware;

#[async_trait]
impl Middleware for TracingMiddleware {
    async fn complete(&self, req: Request, next: &dyn Provider) -> Result<Response> {
        let model = req.model.clone();
        tracing::info!(model = %model, "cognate: sending completion request");
        let response = next.complete(req).await?;
        if let Some(usage) = response.usage() {
            tracing::info!(
                model = %model,
                prompt_tokens = usage.prompt_tokens,
                completion_tokens = usage.completion_tokens,
                "cognate: request completed"
            );
        }
        Ok(response)
    }

    async fn stream(
        &self,
        req: Request,
        next: &dyn Provider,
    ) -> Result<BoxStream<'static, Result<Chunk>>> {
        tracing::info!(model = %req.model, "cognate: starting streaming request");
        next.stream(req).await
    }
}
