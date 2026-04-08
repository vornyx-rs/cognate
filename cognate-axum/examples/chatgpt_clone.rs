//! ChatGPT-style SSE streaming server.
//!
//! Exposes a single `POST /chat` endpoint that accepts a [`Request`] body and
//! streams the response as Server-Sent Events.
//!
//! # Running
//!
//! ```bash
//! OPENAI_API_KEY=sk-… cargo run --example chatgpt_clone -p cognate-axum
//! # Then:
//! curl -X POST http://localhost:3000/chat \
//!   -H 'Content-Type: application/json' \
//!   -d '{"model":"gpt-4o-mini","messages":[{"role":"user","content":"Hello!"}]}'
//! ```

use axum::{routing::post, Json, Router};
use cognate_axum::{into_sse, CognateProvider};
use cognate_core::{Provider, Request};
use cognate_providers::OpenAiProvider;
use std::sync::Arc;

/// Shared application state.
#[derive(Clone)]
struct AppState {
    provider: Arc<dyn Provider>,
}

// Tell axum how to pull `Arc<dyn Provider>` out of `AppState`.
impl axum::extract::FromRef<AppState> for Arc<dyn Provider> {
    fn from_ref(state: &AppState) -> Self {
        state.provider.clone()
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let provider: Arc<dyn Provider> =
        Arc::new(OpenAiProvider::new(api_key).expect("failed to create OpenAI provider"));

    let app = Router::new()
        .route("/chat", post(chat_handler))
        .with_state(AppState { provider });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind");

    println!("ChatGPT clone listening on http://localhost:3000");
    axum::serve(listener, app).await.expect("server error");
}

async fn chat_handler(
    CognateProvider(provider): CognateProvider,
    Json(req): Json<Request>,
) -> impl axum::response::IntoResponse {
    match provider.stream(req).await {
        Ok(stream) => into_sse(stream),
        Err(e) => {
            // Return an SSE stream that immediately emits a single error event.
            let error_stream =
                futures::stream::once(async move { Ok::<_, cognate_core::Error>(cognate_core::Chunk {
                    id: "err".to_string(),
                    model: String::new(),
                    delta: cognate_core::Delta { role: None, content: e.to_string() },
                    finish_reason: Some("error".to_string()),
                }) });
            into_sse(Box::pin(error_stream))
        }
    }
}
