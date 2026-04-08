//! Mock provider for use in tests.
//!
//! [`MockProvider`] implements [`Provider`] using pre-loaded responses so
//! unit tests can run without real API calls.
//!
//! # Example
//!
//! ```rust
//! use cognate_core::{MockProvider, Provider, Request, Response, Choice, Message, Usage};
//!
//! #[tokio::test]
//! async fn my_test() {
//!     let provider = MockProvider::new();
//!     let req = Request::new().with_model("test");
//!     let response = provider.complete(req).await.unwrap();
//!     assert_eq!(response.content(), "Mock response");
//! }
//! ```

use async_trait::async_trait;
use crate::{Chunk, Choice, Delta, Message, Provider, Request, Response, Result};
use futures::stream::{self, BoxStream, StreamExt};
use std::sync::{Arc, Mutex};

/// A mock [`Provider`] for testing.
///
/// Responses are consumed from an internal queue in FIFO order.
/// If the queue is empty a default stub response/chunk is returned.
#[derive(Debug, Clone)]
pub struct MockProvider {
    responses: Arc<Mutex<Vec<Response>>>,
    chunks: Arc<Mutex<Vec<Vec<Chunk>>>>,
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockProvider {
    /// Create a new, empty mock provider.
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
            chunks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Enqueue a response to be returned by the next call to [`complete`](Self::complete).
    pub fn push_response(&self, response: Response) {
        self.responses.lock().unwrap().push(response);
    }

    /// Enqueue a sequence of chunks to be emitted by the next call to [`stream`](Self::stream).
    pub fn push_stream(&self, stream_chunks: Vec<Chunk>) {
        self.chunks.lock().unwrap().push(stream_chunks);
    }
}

#[async_trait]
impl Provider for MockProvider {
    async fn complete(&self, _req: Request) -> Result<Response> {
        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            Ok(Response {
                id: "mock-id".to_string(),
                model: "mock".to_string(),
                choices: vec![Choice {
                    index: 0,
                    message: Message::assistant("Mock response"),
                    finish_reason: Some("stop".to_string()),
                }],
                usage: None,
                created: None,
            })
        } else {
            Ok(responses.remove(0))
        }
    }

    async fn stream(&self, _req: Request) -> Result<BoxStream<'static, Result<Chunk>>> {
        let mut chunks = self.chunks.lock().unwrap();
        let stream_chunks = if chunks.is_empty() {
            vec![Chunk {
                id: "mock-id".to_string(),
                model: "mock".to_string(),
                delta: Delta {
                    role: None,
                    content: "Mock chunk".to_string(),
                },
                finish_reason: Some("stop".to_string()),
            }]
        } else {
            chunks.remove(0)
        };

        let s = stream::iter(stream_chunks.into_iter().map(Ok)).boxed();
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_mock_provider_default_complete() {
        let provider = MockProvider::new();
        let req = Request::new().with_model("test");
        let response = provider.complete(req).await.unwrap();
        assert_eq!(response.content(), "Mock response");
    }

    #[tokio::test]
    async fn test_mock_provider_queued_complete() {
        let provider = MockProvider::new();
        provider.push_response(Response {
            id: "r1".to_string(),
            model: "test".to_string(),
            choices: vec![Choice {
                index: 0,
                message: Message::assistant("queued"),
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
            created: None,
        });
        let req = Request::new().with_model("test");
        let response = provider.complete(req).await.unwrap();
        assert_eq!(response.content(), "queued");
    }

    #[tokio::test]
    async fn test_mock_provider_stream() {
        let provider = MockProvider::new();
        let req = Request::new().with_model("test");
        let mut stream = provider.stream(req).await.unwrap();
        let chunk = stream.next().await.unwrap().unwrap();
        assert_eq!(chunk.content(), "Mock chunk");
    }
}
