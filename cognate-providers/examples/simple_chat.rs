use cognate_core::{Message, Provider, Request};
use cognate_providers::OpenAiProvider;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    let provider = OpenAiProvider::new(api_key)?;

    let request = Request::new()
        .with_model("gpt-3.5-turbo")
        .with_messages(vec![
            Message::system("You are a helpful assistant."),
            Message::user("What is the capital of France?"),
        ]);

    println!("Sending request...");
    let response = provider.complete(request).await?;

    println!("Response: {}", response.content());
    if let Some(usage) = response.usage() {
        println!(
            "Tokens used: {} ({} prompt, {} completion)",
            usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
        );
    }

    Ok(())
}
