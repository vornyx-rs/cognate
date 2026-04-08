//! Cognate Tools — type-safe tool calling and automatic execution.
//!
//! # Overview
//!
//! 1. Define a struct representing your tool's parameters and derive [`Tool`].
//! 2. Implement an `async fn run(&self)` method on the struct.
//! 3. Register it with a [`ToolExecutor`] and let it drive the
//!    tool-call loop automatically.
//!
//! # Example
//!
//! ```rust,no_run
//! use cognate_tools::{Tool, ToolExecutor};
//! use cognate_core::{Request, Message};
//! use serde::{Deserialize, Serialize};
//! use schemars::JsonSchema;
//!
//! #[derive(Tool, Serialize, Deserialize, JsonSchema)]
//! #[tool(description = "Search the web for a query")]
//! struct WebSearch {
//!     /// The search query.
//!     query: String,
//! }
//!
//! impl WebSearch {
//!     async fn run(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
//!         Ok(format!("Results for: {}", self.query))
//!     }
//! }
//! ```
#![warn(missing_docs)]

pub mod executor;

pub use cognate_tools_derive::Tool;
pub use executor::ToolExecutor;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ─── Tool trait ────────────────────────────────────────────────────────────

/// The core trait for all callable tools.
///
/// Rather than implementing this manually, derive it with `#[derive(Tool)]`
/// and provide an `async fn run(&self) -> Result<T, E>` method on the struct.
///
/// The generated implementation:
/// * Uses [`schemars`] to produce a JSON Schema for the `parameters` field.
/// * Deserialises the LLM-supplied JSON arguments into `Self`.
/// * Calls `self.run()` and serialises the result back to JSON.
#[async_trait]
pub trait Tool: Send + Sync {
    /// The tool name as it will appear in the provider's function-calling API.
    fn name(&self) -> &str;

    /// A description of what the tool does, shown to the model.
    fn description(&self) -> &str;

    /// JSON Schema describing the tool's input parameters.
    fn parameters(&self) -> serde_json::Value;

    /// Execute the tool with the given JSON-encoded parameters.
    ///
    /// Parameters are the raw JSON object the model produced for this call.
    async fn call(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;
}

// ─── ToolDefinition ────────────────────────────────────────────────────────

/// Serialisable metadata for a tool, used when registering tools with a provider.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolDefinition {
    /// The tool's name.
    pub name: String,
    /// Human-readable description shown to the model.
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    pub parameters: serde_json::Value,
}

impl ToolDefinition {
    /// Build a [`ToolDefinition`] from any [`Tool`] implementation.
    pub fn from_tool<T: Tool + ?Sized>(tool: &T) -> Self {
        Self {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            parameters: tool.parameters(),
        }
    }
}
