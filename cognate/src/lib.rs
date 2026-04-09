#![warn(missing_docs)]

//! # Cognate
//!
//! A modular, extensible LLM framework for Rust with multi-provider support, type-safe tools, and RAG capabilities.
//!
//! ## Quick Start
//!
//! For detailed documentation, examples, and guides, please visit:
//! - **Main Crate**: https://crates.io/crates/cognate-llm
//! - **Documentation**: https://docs.rs/cognate-llm/
//! - **Repository**: https://github.com/vornyx-rs/cognate
//! - **Examples**: See https://github.com/vornyx-rs/cognate/tree/main/cognate-providers/examples
//! - **Getting Started**: https://github.com/vornyx-rs/cognate/blob/main/GETTING_STARTED.md

pub use cognate_core::{
    error, middleware, ratelimit, types, Error, Layer, Message, Provider, Request, Response,
};

pub use cognate_providers::{
    anthropic, openai, retry, sse, AnthropicProvider, FallbackProvider, OpenAiProvider, RetryConfig,
};

pub use cognate_tools::{Tool, ToolExecutor};
pub use cognate_tools_derive::Tool as DeriveToolMacro;

pub use cognate_prompts::Prompt;
pub use cognate_prompts_derive::Prompt as DerivePromptMacro;

#[cfg(feature = "rag")]
pub use cognate_rag::{Document, MemoryVectorStore, VectorStore};

#[cfg(feature = "axum")]
pub use cognate_axum;

// Re-export derive macros for convenience
pub use cognate_prompts_derive;
pub use cognate_tools_derive;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{AnthropicProvider, FallbackProvider, OpenAiProvider, RetryConfig};
    pub use crate::{Error, Layer, Message, Provider, Request, Response};
    pub use crate::{Tool, ToolExecutor};

    #[cfg(feature = "rag")]
    pub use crate::{Document, MemoryVectorStore, VectorStore};
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
