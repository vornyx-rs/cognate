//! Error types for Cognate.
#![allow(missing_docs)]

use thiserror::Error;

/// The main error type for all Cognate operations.
#[derive(Error, Debug)]
pub enum Error {
    /// An HTTP-level transport error from `reqwest`.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialisation or deserialisation failed.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// The provider API returned an error status code.
    #[error("API error (status {status}): {message}")]
    Api {
        /// HTTP status code returned by the provider.
        status: u16,
        /// Error message from the provider's response body.
        message: String,
    },

    /// The provider rate-limited this request.
    #[error("Rate limit exceeded — retry after {retry_after}s")]
    RateLimit {
        /// Number of seconds the caller should wait before retrying.
        retry_after: u64,
    },

    /// The request was invalid (missing required field, bad parameter, etc.).
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// The provider is not configured correctly (missing API key, bad URL, etc.).
    #[error("Provider configuration error: {0}")]
    Configuration(String),

    /// An error occurred while reading or parsing a streaming response.
    #[error("Stream error: {0}")]
    Stream(String),

    /// The request exceeded the configured timeout.
    #[error("Request timed out after {0}s")]
    Timeout(u64),

    /// All automatic retry attempts were exhausted.
    #[error("Max retries ({0}) exceeded")]
    RetryExhausted(u32),

    /// A vector store operation failed.
    #[error("Vector store error: {0}")]
    VectorStore(String),
}

/// A convenience type alias for `Result<T, cognate_core::Error>`.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Returns `true` if this error is a rate-limit error.
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, Error::RateLimit { .. })
    }

    /// Returns `true` if this error is safe to retry automatically.
    pub fn is_retryable(&self) -> bool {
        match self {
            Error::Http(e) => {
                e.is_timeout()
                    || e.is_connect()
                    || e.status()
                        .map(|s| s.is_server_error() || s.as_u16() == 429)
                        .unwrap_or(false)
            }
            Error::RateLimit { .. } | Error::Timeout(_) => true,
            Error::Api { status, .. } => {
                matches!(status, 429 | 500 | 502 | 503 | 504)
            }
            Error::RetryExhausted(_) => false,
            _ => false,
        }
    }

    /// Return the recommended retry delay in seconds, if known.
    pub fn retry_after(&self) -> Option<u64> {
        match self {
            Error::RateLimit { retry_after } => Some(*retry_after),
            _ => None,
        }
    }
}
