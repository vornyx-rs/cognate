//! RAG pipeline example.
//!
//! Demonstrates ingesting documents into a [`MemoryVectorStore`] using
//! OpenAI embeddings and then retrieving the most relevant documents for
//! a query.  The retrieved context is then injected into a chat completion
//! request.
//!
//! # Running
//!
//! ```bash
//! OPENAI_API_KEY=sk-… cargo run --example rag_pipeline -p cognate-rag
//! ```

use cognate_core::{Message, Provider, Request, Response};
use cognate_providers::OpenAiProvider;
use cognate_rag::{MemoryVectorStore, RagPipeline};
use std::env;

// ─── Sample knowledge base ─────────────────────────────────────────────────

const DOCUMENTS: &[(&str, &str)] = &[
    ("rust_safety",    "Rust is a systems programming language focused on safety, speed, and concurrency."),
    ("rust_ownership", "Rust uses an ownership system to manage memory without a garbage collector."),
    ("python_speed",   "Python is an interpreted language known for its readability and rapid development cycle."),
    ("go_concurrency", "Go has built-in concurrency primitives through goroutines and channels."),
    ("c_performance",  "C gives programmers direct control over memory and hardware, enabling maximum performance."),
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let provider = OpenAiProvider::new(api_key)?;

    // ── 1. Build the RAG pipeline ──────────────────────────────────────────
    let store = MemoryVectorStore::new();
    let pipeline = RagPipeline::new(provider.clone(), store);

    // ── 2. Ingest documents ────────────────────────────────────────────────
    println!("Embedding and ingesting {} documents …", DOCUMENTS.len());
    let texts: Vec<String> = DOCUMENTS.iter().map(|(_, text)| text.to_string()).collect();
    let metadata: Vec<serde_json::Value> = DOCUMENTS
        .iter()
        .map(|(id, _)| serde_json::json!({ "id": id }))
        .collect();

    pipeline.ingest(texts, metadata).await?;
    println!("Ingestion complete.\n");

    // ── 3. Retrieve relevant documents ────────────────────────────────────
    let query = "How does Rust handle memory management?";
    println!("Query: {query}\n");

    let results: Vec<cognate_rag::Document> = pipeline.retrieve(query, 2).await?;

    println!("Top {} relevant documents:", results.len());
    for (i, doc) in results.iter().enumerate() {
        println!("  [{i}] {}", doc.content);
    }
    println!();

    // ── 4. Build a RAG-augmented prompt ───────────────────────────────────
    let context = results
        .iter()
        .enumerate()
        .map(|(i, d)| format!("[{}] {}", i + 1, d.content))
        .collect::<Vec<_>>()
        .join("\n");

    let system_prompt = format!(
        "You are a helpful assistant. Answer questions using only the provided context.\n\nContext:\n{context}"
    );

    let request = Request::new()
        .with_model("gpt-4o-mini")
        .with_messages(vec![Message::system(system_prompt), Message::user(query)]);

    // ── 5. Generate the answer ─────────────────────────────────────────────
    println!("Generating answer …\n");
    let response: Response = provider.complete(request).await?;
    println!("Answer:\n{}", response.content());

    if let Some(usage) = response.usage() {
        println!(
            "\nTokens — prompt: {}, completion: {}, total: {}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }

    Ok(())
}
