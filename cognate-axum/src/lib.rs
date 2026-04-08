//! Axum integration for Cognate.
//!
//! # Features
//!
//! * [`CognateProvider`] — an Axum extractor that pulls an `Arc<dyn Provider>`
//!   out of shared application state via [`axum::extract::FromRef`].
//! * [`UsageLayer`] / [`UsageHandle`] — Tower middleware that accumulates token
//!   usage across all requests.
//! * [`into_sse`] — convert a `BoxStream<Result<Chunk>>` directly into an Axum
//!   [`Sse`] response.
//!
//! # Wiring up the provider
//!
//! ```rust,no_run
//! use axum::{Router, routing::post};
//! use cognate_axum::CognateProvider;
//! use cognate_core::{Provider, Request};
//! use std::sync::Arc;
//!
//! #[derive(Clone)]
//! struct AppState {
//!     provider: Arc<dyn Provider>,
//! }
//!
//! impl axum::extract::FromRef<AppState> for Arc<dyn Provider> {
//!     fn from_ref(state: &AppState) -> Self {
//!         state.provider.clone()
//!     }
//! }
//!
//! async fn chat(
//!     CognateProvider(provider): CognateProvider,
//!     axum::Json(req): axum::Json<Request>,
//! ) -> String {
//!     provider.complete(req).await.unwrap().content().to_string()
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let provider: Arc<dyn Provider> = unimplemented!();
//!     let _app: Router = Router::new()
//!         .route("/chat", post(chat))
//!         .with_state(AppState { provider });
//! }
//! ```
#![warn(missing_docs)]

use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
    response::sse::{Event, Sse},
};
use cognate_core::{Chunk, Provider};
use futures::stream::{BoxStream, Stream, StreamExt};
use std::{
    convert::Infallible,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

// ─── CognateProvider extractor ─────────────────────────────────────────────

/// Axum extractor that resolves an `Arc<dyn Provider>` from shared application
/// state.
///
/// The application state `S` must implement
/// `axum::extract::FromRef<S, Target = Arc<dyn Provider>>`.
///
/// See the [crate documentation](crate) for a full wiring example.
pub struct CognateProvider(pub Arc<dyn Provider>);

impl<S> FromRequestParts<S> for CognateProvider
where
    Arc<dyn Provider>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Infallible> {
        Ok(CognateProvider(Arc::<dyn Provider>::from_ref(state)))
    }
}

// ─── SSE helper ────────────────────────────────────────────────────────────

/// Convert a streaming provider response into an Axum [`Sse`] response.
///
/// Text deltas are emitted as `data:` events.  Provider errors are emitted as
/// `event: error` events so the client can detect them.
///
/// # Example
///
/// ```rust,no_run
/// use axum::{extract::State, response::Sse};
/// use cognate_axum::{CognateProvider, into_sse};
/// use cognate_core::{Request, Message};
///
/// async fn stream_handler(
///     CognateProvider(provider): CognateProvider,
///     axum::Json(req): axum::Json<Request>,
/// ) -> impl axum::response::IntoResponse {
///     let stream = provider.stream(req).await.unwrap();
///     into_sse(stream)
/// }
/// ```
pub fn into_sse(
    stream: BoxStream<'static, cognate_core::Result<Chunk>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mapped = stream.map(|result| match result {
        Ok(chunk) => {
            let mut event = Event::default().data(chunk.content());
            if chunk.is_finished() {
                event = event.event("done");
            }
            Ok(event)
        }
        Err(e) => Ok(Event::default().event("error").data(e.to_string())),
    });
    Sse::new(mapped)
}

// ─── UsageHandle ───────────────────────────────────────────────────────────

/// A handle for reading token usage accumulated by [`UsageLayer`].
///
/// Clone freely — all clones share the same underlying counters.
#[derive(Debug, Clone, Default)]
pub struct UsageHandle {
    /// Total prompt tokens seen across all requests.
    pub prompt_tokens: Arc<AtomicU64>,
    /// Total completion tokens seen across all requests.
    pub completion_tokens: Arc<AtomicU64>,
}

impl UsageHandle {
    /// Create a new, zeroed handle.
    pub fn new() -> Self {
        Self::default()
    }

    /// Total prompt tokens accumulated so far.
    pub fn prompt_tokens(&self) -> u64 {
        self.prompt_tokens.load(Ordering::Relaxed)
    }

    /// Total completion tokens accumulated so far.
    pub fn completion_tokens(&self) -> u64 {
        self.completion_tokens.load(Ordering::Relaxed)
    }

    /// Total tokens accumulated so far.
    pub fn total_tokens(&self) -> u64 {
        self.prompt_tokens() + self.completion_tokens()
    }
}

// ─── UsageLayer (Tower Layer) ──────────────────────────────────────────────

/// Tower [`Layer`] that wraps a [`Provider`] and records token usage.
///
/// # Example
///
/// ```rust,no_run
/// use cognate_axum::{UsageLayer, UsageHandle};
/// use cognate_core::{Provider, ProviderExt, middleware::MiddlewareLayer};
/// # use cognate_core::MockProvider;
///
/// let handle = UsageHandle::new();
/// let layer = UsageLayer::new(handle.clone());
/// let provider = MockProvider::new().with(layer);
/// // After requests, inspect: handle.total_tokens()
/// ```
#[derive(Clone)]
pub struct UsageLayer {
    handle: UsageHandle,
}

impl UsageLayer {
    /// Create a new layer that reports into `handle`.
    pub fn new(handle: UsageHandle) -> Self {
        Self { handle }
    }
}

impl cognate_core::middleware::Layer for UsageLayer {
    fn layer(&self, provider: Arc<dyn Provider>) -> Arc<dyn Provider> {
        Arc::new(UsageProvider {
            inner: provider,
            handle: self.handle.clone(),
        })
    }
}

// ─── UsageProvider ─────────────────────────────────────────────────────────

struct UsageProvider {
    inner: Arc<dyn Provider>,
    handle: UsageHandle,
}

#[async_trait::async_trait]
impl Provider for UsageProvider {
    async fn complete(&self, req: cognate_core::Request) -> cognate_core::Result<cognate_core::Response> {
        let response = self.inner.complete(req).await?;
        if let Some(usage) = &response.usage {
            self.handle
                .prompt_tokens
                .fetch_add(usage.prompt_tokens as u64, Ordering::Relaxed);
            self.handle
                .completion_tokens
                .fetch_add(usage.completion_tokens as u64, Ordering::Relaxed);
        }
        Ok(response)
    }

    async fn stream(
        &self,
        req: cognate_core::Request,
    ) -> cognate_core::Result<BoxStream<'static, cognate_core::Result<Chunk>>> {
        // Streaming doesn't return usage per-chunk; delegate as-is.
        self.inner.stream(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cognate_core::{Choice, Message, MockProvider, ProviderExt, Request, Response, Usage};

    fn make_response_with_usage(prompt: u32, completion: u32) -> Response {
        Response {
            id: "r".to_string(),
            model: "m".to_string(),
            choices: vec![Choice {
                index: 0,
                message: Message::assistant("ok"),
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(Usage {
                prompt_tokens: prompt,
                completion_tokens: completion,
                total_tokens: prompt + completion,
            }),
            created: None,
        }
    }

    #[tokio::test]
    async fn test_usage_layer_accumulates() {
        let handle = UsageHandle::new();
        let mock = MockProvider::new();
        mock.push_response(make_response_with_usage(10, 5));
        mock.push_response(make_response_with_usage(20, 8));

        let provider = mock.with(UsageLayer::new(handle.clone()));

        let req = Request::new().with_model("test");
        provider.complete(req.clone()).await.unwrap();
        provider.complete(req).await.unwrap();

        assert_eq!(handle.prompt_tokens(), 30);
        assert_eq!(handle.completion_tokens(), 13);
        assert_eq!(handle.total_tokens(), 43);
    }
}
