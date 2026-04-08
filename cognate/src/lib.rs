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
    error, middleware, ratelimit, types, Error, Layer, Message, Provider, Request, Response,
};

pub use cognate_providers::{
    anthropic, openai, retry, sse, AnthropicProvider, FallbackProvider, OpenAiProvider,
    RetryConfig,
};

pub use cognate_tools::{Tool, ToolExecutor};
pub use cognate_tools_derive::Tool as DeriveToolMacro;

pub use cognate_prompts::Prompt;
pub use cognate_prompts_derive::Prompt as DerivePromptMacro;

#[cfg(feature = "rag")]
pub use cognate_rag::{Document, InMemoryVectorStore, VectorStore};

#[cfg(feature = "axum")]
pub use cognate_axum;

// Re-export derive macros for convenience
pub use cognate_tools_derive;
pub use cognate_prompts_derive;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{Error, Layer, Message, Provider, Request, Response};
    pub use crate::{AnthropicProvider, FallbackProvider, OpenAiProvider, RetryConfig};
    pub use crate::{Tool, ToolExecutor};

    #[cfg(feature = "rag")]
    pub use crate::{Document, InMemoryVectorStore, VectorStore};
}

pub mod providers {
    //! Provider implementations
    pub use cognate_providers::*;
}

pub mod tools {
    //! Tool calling and execution
    pub use cognate_tools::*;
}

pub mod prompts {
    //! Prompt templating
    pub use cognate_prompts::*;
}

