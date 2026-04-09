//! Agent example — automatic tool-calling loop.
//!
//! Demonstrates a two-tool agent that can perform arithmetic and look up
//! "weather" (mocked).  The [`ToolExecutor`] drives the loop automatically:
//! it sends the request, dispatches any tool calls, feeds results back, and
//! continues until the model produces a final text response.
//!
//! # Running
//!
//! ```bash
//! OPENAI_API_KEY=sk-… cargo run --example agent -p cognate-tools
//! ```

use cognate_core::{Message, Request};
use cognate_providers::OpenAiProvider;
use cognate_tools::{Tool, ToolExecutor};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::env;

// ─── Calculator tool ───────────────────────────────────────────────────────

/// Performs basic arithmetic on two numbers.
#[derive(Tool, Serialize, Deserialize, JsonSchema)]
#[tool(description = "Perform basic arithmetic (add, subtract, multiply, divide)")]
struct Calculator {
    /// The left operand.
    a: f64,
    /// The right operand.
    b: f64,
    /// The operation: one of \"add\", \"subtract\", \"multiply\", \"divide\".
    op: String,
}

impl Calculator {
    async fn run(&self) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        match self.op.as_str() {
            "add" => Ok(self.a + self.b),
            "subtract" => Ok(self.a - self.b),
            "multiply" => Ok(self.a * self.b),
            "divide" => {
                if self.b == 0.0 {
                    Err("division by zero".into())
                } else {
                    Ok(self.a / self.b)
                }
            }
            other => Err(format!("unknown operation: {other}").into()),
        }
    }
}

// ─── Weather tool ──────────────────────────────────────────────────────────

/// Returns the (fake) current weather for a city.
#[derive(Tool, Serialize, Deserialize, JsonSchema)]
#[tool(description = "Get the current weather for a city (returns fake data)")]
struct GetWeather {
    /// Name of the city to look up.
    city: String,
}

impl GetWeather {
    async fn run(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Fake response — replace with a real API call in production.
        Ok(format!(
            "The weather in {} is sunny with a high of 22 °C.",
            self.city
        ))
    }
}

// ─── Main ──────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let provider = OpenAiProvider::new(api_key)?;

    let mut executor = ToolExecutor::new(provider);
    executor.add_tool(Calculator {
        a: 0.0,
        b: 0.0,
        op: String::new(),
    });
    executor.add_tool(GetWeather {
        city: String::new(),
    });

    let request = Request::new().with_model("gpt-4o-mini").with_messages(vec![
        Message::system(
            "You are a helpful assistant with access to a calculator and weather service.",
        ),
        Message::user("What is 123 multiplied by 456? Also, what is the weather in Tokyo?"),
    ]);

    println!("Running agent …\n");
    let response = executor.execute(request).await?;
    println!("Final answer:\n{}", response.content());

    if let Some(usage) = response.usage() {
        println!(
            "\nTokens — prompt: {}, completion: {}, total: {}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }

    Ok(())
}
