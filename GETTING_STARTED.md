# Getting Started with Cognate

This guide walks you through building your first LLM application with Cognate in 10 minutes.

## Prerequisites

- Rust 1.70 or later
- An OpenAI API key (get one at https://platform.openai.com/api-keys)
- Basic Rust knowledge

## Step 1: Create a New Project

```bash
cargo new my-llm-app
cd my-llm-app
```

## Step 2: Add Dependencies

Edit `Cargo.toml`:

```toml
[dependencies]
cognate-core = "0.1"
cognate-providers = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Step 3: Write Your First App

Replace the contents of `src/main.rs`:

```rust
use cognate_core::{Provider, Request, Message};
use cognate_providers::OpenAiProvider;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create provider with API key from environment
    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("Set OPENAI_API_KEY environment variable");
    
    let provider = OpenAiProvider::new(api_key)?;
    
    // Create a request
    let request = Request::new()
        .with_model("gpt-4o-mini")
        .with_messages(vec![
            Message::system("You are a helpful assistant"),
            Message::user("What is Rust used for?"),
        ]);
    
    // Send request
    let response = provider.complete(request).await?;
    println!("{}", response.content());
    
    Ok(())
}
```

## Step 4: Run Your App

```bash
export OPENAI_API_KEY="sk-..."
cargo run
```

Output:
```
Rust is a modern programming language...
```

Congratulations! You have built your first Cognate application.

## Next Steps

### Add Streaming

Responses can be streamed token-by-token:

```rust
use futures::StreamExt;

let mut stream = provider.stream(request).await?;

while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    print!("{}", chunk.text);
}
println!();
```

### Use Type-Safe Tools

Define tools that the LLM can call:

```rust
use cognate_tools::Tool;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Tool, Serialize, Deserialize, JsonSchema)]
#[tool(description = "Get the current time")]
struct CurrentTime;

impl CurrentTime {
    async fn run(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        Ok(format!("Current time: {}", chrono::Local::now()))
    }
}

// Use in request
let request = Request::new()
    .with_model("gpt-4o")
    .with_messages(vec![
        Message::user("What time is it?"),
    ])
    .with_tool(CurrentTime);
```

### Build a RAG Pipeline

Retrieve documents and augment requests:

```rust
use cognate_rag::memory::InMemoryVectorStore;

let vector_store = InMemoryVectorStore::new();

// Add documents
vector_store.add("id1", "Rust is a systems programming language").await?;
vector_store.add("id2", "Cognate is an LLM framework").await?;

// Search
let results = vector_store.search("What is Rust?", 2).await?;

// Augment request
let mut messages = vec![Message::system("You are helpful")];
for doc in results {
    messages.push(Message::system(format!("Context: {}", doc.content)));
}
messages.push(Message::user("What is Rust?"));

let request = Request::new()
    .with_model("gpt-4o-mini")
    .with_messages(messages);
```

### Deploy on Axum

Build a web server:

```rust
use axum::{Router, routing::post, Json};
use cognate_axum::*;
use cognate_core::{Provider, Request};
use cognate_providers::OpenAiProvider;

async fn chat(
    Json(request): Json<Request>,
) -> Result<String, String> {
    let provider = OpenAiProvider::new(
        std::env::var("OPENAI_API_KEY").unwrap()
    ).map_err(|e| e.to_string())?;
    
    let response = provider.complete(request)
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(response.content())
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/chat", post(chat));
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    
    axum::serve(listener, app).await.unwrap();
}
```

Test:
```bash
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4o-mini","messages":[{"role":"user","content":"Hello"}]}'
```

### Switch Providers

Cognate supports multiple providers:

```rust
// OpenAI
let provider = OpenAiProvider::new(api_key)?;

// Anthropic
use cognate_providers::AnthropicProvider;
let provider = AnthropicProvider::new(api_key)?;

// With fallback (try Anthropic, fall back to OpenAI)
use cognate_providers::FallbackProvider;
let provider = FallbackProvider::new(
    Box::new(AnthropicProvider::new(claude_key)?),
    Box::new(OpenAiProvider::new(openai_key)?),
);
```

## Common Patterns

### Error Handling

```rust
use cognate_core::{Error, Result};

match provider.complete(request).await {
    Ok(response) => println!("{}", response.content()),
    
    Err(Error::RateLimited { retry_after }) => {
        println!("Rate limited. Retry after: {:?}", retry_after);
    }
    
    Err(Error::Configuration(msg)) => {
        eprintln!("Configuration error: {}", msg);
    }
    
    Err(e) => {
        eprintln!("Request failed: {}", e);
    }
}
```

### Retries

```rust
use cognate_providers::RetryConfig;
use std::time::Duration;

let provider = OpenAiProvider::new(api_key)?
    .with_retry(RetryConfig {
        max_retries: 3,
        initial_backoff: Duration::from_millis(100),
        max_backoff: Duration::from_secs(10),
    });
```

### Timeouts

```rust
use std::time::Duration;

let provider = OpenAiProvider::new(api_key)?
    .with_timeout(Duration::from_secs(30));
```

## Troubleshooting

### "OPENAI_API_KEY not found"

Set your API key:
```bash
export OPENAI_API_KEY="sk-..."
```

### "Connection refused"

Check your internet connection and API key validity.

### Slow responses

- Use `gpt-4o-mini` for faster responses
- Check network latency
- Review rate limiting in your account

### High memory usage

- Use streaming for large responses
- Close connections properly
- Check for connection leaks

## Examples

Full examples are available in the repository:

- [Simple Chat](../cognate-providers/examples/simple_chat.rs)
- [Streaming Chat](../cognate-providers/examples/streaming_chat.rs)
- [Tool Usage](../cognate-tools/examples/tool_usage.rs)
- [RAG Pipeline](../cognate-rag/examples/rag_pipeline.rs)
- [Web Server](../cognate-axum/examples/chatgpt_clone.rs)

Run examples:
```bash
cargo run --example simple_chat -p cognate-providers
cargo run --example streaming_chat -p cognate-providers
cargo run --example tool_usage -p cognate-tools
cargo run --example rag_pipeline -p cognate-rag
cargo run --example agent -p cognate-tools
cargo run --example chatgpt_clone -p cognate-axum
```

## What's Next?

- Read [ARCHITECTURE.md](ARCHITECTURE.md) to understand the design
- Check [BENCHMARKS.md](BENCHMARKS.md) for performance details
- Review [CONTRIBUTING.md](CONTRIBUTING.md) to contribute
- Join discussions on GitHub

## Support

- Documentation: https://docs.rs/cognate-core
- Issues: https://github.com/YOUR_ORG/cognate/issues
- Discussions: https://github.com/YOUR_ORG/cognate/discussions
