use cognate_core::{Provider, Request, Message};
use cognate_providers::OpenAiProvider;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let provider = OpenAiProvider::new(api_key)?;
    
    let mut messages = vec![
        Message::system("You are a helpful CLI assistant."),
    ];

    println!("Welcome to Cognate CLI! (Type 'exit' to quit)");

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "exit" || input == "quit" {
            break;
        }

        messages.push(Message::user(input));
        
        let req = Request::new()
            .with_model("gpt-3.5-turbo")
            .with_messages(messages.clone());

        println!("Thinking...");
        let resp = provider.complete(req).await?;
        let content = resp.content();
        
        println!("Assistant: {}", content);
        messages.push(Message::assistant(content));
    }

    Ok(())
}
