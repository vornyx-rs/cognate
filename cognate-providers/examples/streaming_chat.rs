use cognate_core::{Provider, Request, Message};
use cognate_providers::OpenAiProvider;
use futures::StreamExt;
use std::env;
use std::io::{stdout, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY environment variable not set");
    
    let provider = OpenAiProvider::new(api_key)?;
    
    let request = Request::new()
        .with_model("gpt-3.5-turbo")
        .with_messages(vec![
            Message::user("Tell me a short joke about Rust programming."),
        ]);
    
    println!("Streaming response:");
    let mut stream = provider.stream(request).await?;
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        print!("{}", chunk.content());
        stdout().flush()?;
    }
    println!("\nStream finished.");
    
    Ok(())
}
