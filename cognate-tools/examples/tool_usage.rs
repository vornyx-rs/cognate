//! Basic tool usage example.
//!
//! Shows how to define a tool with `#[derive(Tool)]`, inspect its metadata,
//! and invoke it directly with JSON arguments.
//!
//! # Running
//!
//! ```bash
//! cargo run --example tool_usage -p cognate-tools
//! ```

use cognate_tools::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A simple tool that executes a SQL query (stubbed).
#[derive(Tool, JsonSchema, Deserialize, Serialize)]
#[tool(description = "Search the database")]
struct SearchDB {
    /// SQL query to execute.
    #[tool_param(description = "The SQL query string")]
    query: String,
}

impl SearchDB {
    async fn run(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // In a real implementation this would connect to a database.
        Ok(format!("Executed query: {}", self.query))
    }
}

#[tokio::main]
async fn main() {
    let tool = SearchDB {
        query: String::new(),
    };

    println!("Name:        {}", tool.name());
    println!("Description: {}", tool.description());
    println!(
        "Parameters:  {}",
        serde_json::to_string_pretty(&tool.parameters()).unwrap()
    );

    let result = tool
        .call(serde_json::json!({ "query": "SELECT * FROM users LIMIT 5" }))
        .await
        .unwrap();
    println!("\nResult: {result}");
}
