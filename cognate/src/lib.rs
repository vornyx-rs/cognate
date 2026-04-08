#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

//! # Cognate
//!
//! A modular, extensible LLM framework for Rust with multi-provider support, type-safe tools, and RAG capabilities.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use cognate::prelude::*;
//!
//! #[tokio::main]
//! async fn main() {
//!     let client = cognate::providers::OpenAiProvider::new("sk-...".to_string());
//!     // Use the client...
//! }
//! ```
//!
//! ## Features
//!
//! - `providers` - OpenAI and Anthropic provider support (default)
//! - `tools` - Type-safe tool calling with derive macros (default)
//! - `prompts` - Compile-time validated prompt templates (default)
//! - `rag` - Retrieval-Augmented Generation support
//! - `axum` - Axum web framework integration
//! - `full` - All features

pub use cognate_core::{
    error, middleware, ratelimit, types, Client, Error, Layer, Message, MessageRole, Provider,
    Request, Response, StreamProvider,
};

pub use cognate_providers::{
    anthropic, openai, retry, sse, AnthropicConfig, AnthropicProvider, FallbackProvider,
    OpenAiConfig, OpenAiProvider, RetryConfig,
};

pub use cognate_tools::{Tool, ToolExecutor, ToolResult};
pub use cognate_tools_derive::Tool;

pub use cognate_prompts::{Prompt, PromptRenderer};
pub use cognate_prompts_derive::Prompt;

#[cfg(feature = "rag")]
pub use cognate_rag::{Document, InMemoryVectorStore, VectorStore};

#[cfg(feature = "axum")]
pub use cognate_axum::{extract_response, StreamExt};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        Client, Error, Message, MessageRole, Provider, Request, Response, StreamProvider,
    };
    pub use crate::{AnthropicProvider, FallbackProvider, OpenAiProvider, RetryConfig};
    pub use crate::{Tool, ToolExecutor, ToolResult};
    pub use crate::{Prompt, PromptRenderer};

    #[cfg(feature = "rag")]
    pub use crate::{Document, InMemoryVectorStore, VectorStore};

    #[cfg(feature = "axum")]
    pub use crate::{extract_response, StreamExt};
}
