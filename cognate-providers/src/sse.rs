//! Server-Sent Events (SSE) streaming parser
//!
//! This module provides utilities for parsing SSE streams from LLM providers.

use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

/// A single parsed Server-Sent Event.
#[derive(Debug, Clone)]
pub struct SseEvent {
    /// The `event:` field, or an empty string if absent.
    pub event: String,
    /// The `data:` payload (multi-line data is joined with `\n`).
    pub data: String,
    /// The optional `id:` field.
    pub id: Option<String>,
}

impl SseEvent {
    /// Parse an SSE event from a string
    pub fn parse(input: &str) -> Option<Self> {
        let mut event = String::new();
        let mut data = String::new();
        let mut id = None;

        for line in input.lines() {
            if let Some(val) = line.strip_prefix("event: ") {
                event = val.to_string();
            } else if let Some(val) = line.strip_prefix("data: ") {
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(val);
            } else if let Some(val) = line.strip_prefix("id: ") {
                id = Some(val.to_string());
            }
        }

        if data.is_empty() {
            None
        } else {
            Some(Self { event, data, id })
        }
    }

    /// Check if this is a data event
    pub fn is_data_event(&self) -> bool {
        self.event.is_empty() || self.event == "message"
    }
}

/// A stream of SSE events
pub struct SseStream<S> {
    inner: S,
    buffer: String,
}

impl<S, E> SseStream<S>
where
    S: Stream<Item = Result<bytes::Bytes, E>> + Unpin,
    E: std::error::Error + Send + Sync + 'static,
{
    /// Create a new SSE stream from a byte stream
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            buffer: String::new(),
        }
    }
}

impl<S, E> Stream for SseStream<S>
where
    S: Stream<Item = Result<bytes::Bytes, E>> + Unpin,
    E: std::error::Error + Send + Sync + 'static,
{
    type Item = Result<SseEvent, cognate_core::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.inner.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    let chunk = String::from_utf8_lossy(&bytes);
                    self.buffer.push_str(&chunk);

                    // Check for complete events (double newline)
                    if let Some(pos) = self.buffer.find("\n\n") {
                        let event_str = self.buffer[..pos].to_string();
                        self.buffer = self.buffer[pos + 2..].to_string();

                        if let Some(event) = SseEvent::parse(&event_str) {
                            return Poll::Ready(Some(Ok(event)));
                        }
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(cognate_core::Error::Stream(e.to_string()))));
                }
                Poll::Ready(None) => {
                    // Process any remaining data in buffer
                    if !self.buffer.is_empty() {
                        let event_str = self.buffer.clone();
                        self.buffer.clear();

                        if let Some(event) = SseEvent::parse(&event_str) {
                            return Poll::Ready(Some(Ok(event)));
                        }
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_event() {
        let input = "event: message\ndata: hello world\n";
        let event = SseEvent::parse(input).unwrap();
        assert_eq!(event.event, "message");
        assert_eq!(event.data, "hello world");
    }

    #[test]
    fn test_parse_sse_with_id() {
        let input = "id: 123\nevent: update\ndata: some data\n";
        let event = SseEvent::parse(input).unwrap();
        assert_eq!(event.id, Some("123".to_string()));
        assert_eq!(event.event, "update");
        assert_eq!(event.data, "some data");
    }

    #[test]
    fn test_parse_empty_data() {
        let input = "event: ping\n";
        assert!(SseEvent::parse(input).is_none());
    }
}
