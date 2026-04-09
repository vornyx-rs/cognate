//! Prompt templating example.
//!
//! Demonstrates compile-time template validation — unknown variable names
//! are detected at compile time, not at runtime.
//!
//! # Running
//!
//! ```bash
//! cargo run --example prompt_usage -p cognate-prompts
//! ```

use cognate_prompts::Prompt;
use serde::Serialize;

/// A prompt that analyses a user given their name and age.
#[derive(Prompt, Serialize)]
#[template("Analyse this user: {{name}}, age {{age}}.")]
struct UserAnalysis {
    name: String,
    age: u32,
}

/// Multi-line prompt with several variables.
#[derive(Prompt, Serialize)]
#[template(
    "You are a {{role}} assistant.\nThe user asked: {{question}}\nPlease reply in {{language}}."
)]
struct AssistantPrompt {
    role: String,
    question: String,
    language: String,
}

fn main() {
    // ── UserAnalysis ────────────────────────────────────────────────────────
    let analysis = UserAnalysis {
        name: "Alice".to_string(),
        age: 30,
    };
    let rendered = analysis.render().unwrap();
    println!("UserAnalysis: {rendered}");
    assert_eq!(rendered, "Analyse this user: Alice, age 30.");

    // ── AssistantPrompt ─────────────────────────────────────────────────────
    let prompt = AssistantPrompt {
        role: "coding".to_string(),
        question: "How do I write an iterator in Rust?".to_string(),
        language: "English".to_string(),
    };
    let rendered = prompt.render().unwrap();
    println!("\nAssistantPrompt:\n{rendered}");

    println!("\nAll validations passed!");
}
