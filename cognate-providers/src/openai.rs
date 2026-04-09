//! OpenAI provider implementation.
//!
//! Supports chat completions (non-streaming and streaming), tool calling, and
//! embeddings via the OpenAI HTTP API.

use async_trait::async_trait;
use cognate_core::{
    Choice, Chunk, Delta, EmbeddingProvider, Message, Provider, ProviderConfig, Request, Response,
    Role, ToolCall, ToolCallFunction,
};
use futures::stream::{BoxStream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};

use crate::retry::{with_retry, RetryConfig};
use crate::sse::SseStream;

const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";

// ─── Provider struct ───────────────────────────────────────────────────────

/// Provider client for the OpenAI API.
///
/// Supports chat completions, tool calling, and text embeddings.
///
/// # Example
///
/// ```rust,no_run
/// use cognate_providers::OpenAiProvider;
/// use cognate_core::{Provider, Request, Message};
///
/// # async fn run() -> cognate_core::Result<()> {
/// let provider = OpenAiProvider::new(std::env::var("OPENAI_API_KEY").unwrap())?;
/// let resp = provider
///     .complete(
///         Request::new()
///             .with_model("gpt-4o-mini")
///             .with_message(Message::user("Hello!")),
///     )
///     .await?;
/// println!("{}", resp.content());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct OpenAiProvider {
    client: reqwest::Client,
    config: ProviderConfig,
    base_url: String,
    rate_limiter: Option<cognate_core::TokenBucket>,
    retry_config: RetryConfig,
}

impl OpenAiProvider {
    /// Create a provider from an API key using default settings.
    pub fn new(api_key: impl Into<String>) -> cognate_core::Result<Self> {
        Self::with_config(ProviderConfig::new(api_key))
    }

    /// Create a provider with full [`ProviderConfig`] control.
    pub fn with_config(config: ProviderConfig) -> cognate_core::Result<Self> {
        let base_url = if config.base_url.is_empty() {
            OPENAI_BASE_URL.to_string()
        } else {
            config.base_url.clone()
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| cognate_core::Error::Configuration(e.to_string()))?;

        Ok(Self {
            client,
            config,
            base_url,
            rate_limiter: None,
            retry_config: RetryConfig::default(),
        })
    }

    /// Attach a token-bucket rate limiter.
    pub fn with_rate_limiter(mut self, rate_limiter: cognate_core::TokenBucket) -> Self {
        self.rate_limiter = Some(rate_limiter);
        self
    }

    /// Override the default retry configuration.
    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.config.api_key)
    }

    async fn handle_error(response: reqwest::Response) -> cognate_core::Error {
        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        if status == 429 {
            cognate_core::Error::RateLimit { retry_after: 60 }
        } else {
            cognate_core::Error::Api {
                status,
                message: body,
            }
        }
    }
}

// ─── Provider impl ─────────────────────────────────────────────────────────

#[async_trait]
impl Provider for OpenAiProvider {
    async fn complete(&self, req: Request) -> cognate_core::Result<Response> {
        with_retry(&self.retry_config, || async {
            if let Some(ref limiter) = self.rate_limiter {
                limiter.acquire(1.0).await;
            }

            let url = format!("{}/chat/completions", self.base_url);
            let openai_req = OpenAiRequest::from_request(&req);

            let http_resp = self
                .client
                .post(&url)
                .header("Authorization", self.auth_header())
                .header("Content-Type", "application/json")
                .json(&openai_req)
                .send()
                .await?;

            if !http_resp.status().is_success() {
                return Err(Self::handle_error(http_resp).await);
            }

            let resp: OpenAiResponse = http_resp.json().await?;
            Ok(resp.into_response())
        })
        .await
    }

    async fn stream(
        &self,
        req: Request,
    ) -> cognate_core::Result<BoxStream<'static, cognate_core::Result<Chunk>>> {
        with_retry(&self.retry_config, || async {
            if let Some(ref limiter) = self.rate_limiter {
                limiter.acquire(1.0).await;
            }

            let url = format!("{}/chat/completions", self.base_url);
            let mut openai_req = OpenAiRequest::from_request(&req);
            openai_req.stream = Some(true);

            let http_resp = self
                .client
                .post(&url)
                .header("Authorization", self.auth_header())
                .header("Content-Type", "application/json")
                .header("Accept", "text/event-stream")
                .json(&openai_req)
                .send()
                .await?;

            if !http_resp.status().is_success() {
                return Err(Self::handle_error(http_resp).await);
            }

            let chunk_stream = SseStream::new(http_resp.bytes_stream())
                .try_filter_map(|event| async move {
                    if event.data == "[DONE]" {
                        return Ok(None);
                    }
                    match serde_json::from_str::<OpenAiStreamChunk>(&event.data) {
                        Ok(chunk) => Ok(Some(chunk.into_chunk())),
                        Err(e) => Err(cognate_core::Error::Json(e)),
                    }
                })
                .boxed();

            Ok(chunk_stream)
        })
        .await
    }
}

// ─── EmbeddingProvider impl ────────────────────────────────────────────────

#[async_trait]
impl EmbeddingProvider for OpenAiProvider {
    async fn embed(&self, inputs: Vec<String>) -> cognate_core::Result<Vec<Vec<f32>>> {
        let url = format!("{}/embeddings", self.base_url);
        let body = serde_json::json!({
            "model": "text-embedding-3-small",
            "input": inputs,
        });

        let http_resp = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !http_resp.status().is_success() {
            return Err(Self::handle_error(http_resp).await);
        }

        let resp: OpenAiEmbeddingResponse = http_resp.json().await?;
        let mut data = resp.data;
        // Sort by index to guarantee order matches `inputs`.
        data.sort_by_key(|e| e.index);
        Ok(data.into_iter().map(|e| e.embedding).collect())
    }
}

// ─── Wire types (request) ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<cognate_core::ResponseFormat>,
    /// Tool definitions in OpenAI function-calling format.
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    /// `"auto"`, `"none"`, or `{"type":"function","function":{"name":"…"}}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<serde_json::Value>,
}

impl OpenAiRequest {
    fn from_request(req: &Request) -> Self {
        Self {
            model: req.model.clone(),
            messages: req.messages.iter().map(OpenAiMessage::from_msg).collect(),
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            top_p: req.top_p,
            frequency_penalty: req.frequency_penalty,
            presence_penalty: req.presence_penalty,
            stop: req.stop.clone(),
            stream: req.stream,
            response_format: req.response_format.clone(),
            tools: req
                .extra
                .get("tools")
                .and_then(|v| v.as_array())
                .map(|a| a.to_vec()),
            tool_choice: req.extra.get("tool_choice").cloned(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    /// Content may be empty for assistant messages that only carry tool_calls.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

impl OpenAiMessage {
    fn from_msg(msg: &Message) -> Self {
        let role = role_to_str(&msg.role).to_string();
        Self {
            role,
            content: msg.content.clone(),
            name: msg.name.clone(),
            tool_calls: msg.tool_calls.as_ref().map(|calls| {
                calls
                    .iter()
                    .map(|tc| OpenAiToolCall {
                        id: tc.id.clone(),
                        call_type: tc.call_type.clone(),
                        function: OpenAiToolCallFunction {
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        },
                    })
                    .collect()
            }),
            tool_call_id: msg.tool_call_id.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OpenAiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAiToolCallFunction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OpenAiToolCallFunction {
    name: String,
    arguments: String,
}

// ─── Wire types (response) ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    id: String,
    model: String,
    created: u64,
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    index: u32,
    message: OpenAiMessageResponse,
    finish_reason: Option<String>,
}

/// Separate response type for deserialization (content can be null in tool-call responses).
#[derive(Debug, Deserialize)]
struct OpenAiMessageResponse {
    role: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAiToolCall>>,
    #[serde(default)]
    tool_call_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

impl OpenAiResponse {
    fn into_response(self) -> Response {
        Response {
            id: self.id,
            model: self.model,
            created: Some(self.created),
            choices: self
                .choices
                .into_iter()
                .map(|c| {
                    let tool_calls = c.message.tool_calls.map(|calls| {
                        calls
                            .into_iter()
                            .map(|tc| ToolCall {
                                id: tc.id,
                                call_type: tc.call_type,
                                function: ToolCallFunction {
                                    name: tc.function.name,
                                    arguments: tc.function.arguments,
                                },
                            })
                            .collect()
                    });
                    Choice {
                        index: c.index,
                        message: Message {
                            role: str_to_role(&c.message.role),
                            content: c.message.content.unwrap_or_default(),
                            name: c.message.name,
                            tool_calls,
                            tool_call_id: c.message.tool_call_id,
                        },
                        finish_reason: c.finish_reason,
                    }
                })
                .collect(),
            usage: self.usage.map(|u| cognate_core::Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        }
    }
}

// ─── Wire types (streaming) ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct OpenAiStreamChunk {
    id: String,
    model: String,
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(Debug, Deserialize, Clone)]
struct OpenAiStreamChoice {
    delta: OpenAiStreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct OpenAiStreamDelta {
    role: Option<String>,
    #[serde(default)]
    content: Option<String>,
}

impl OpenAiStreamChunk {
    fn into_chunk(self) -> Chunk {
        let choice = self
            .choices
            .into_iter()
            .next()
            .unwrap_or(OpenAiStreamChoice {
                delta: OpenAiStreamDelta::default(),
                finish_reason: None,
            });
        Chunk {
            id: self.id,
            model: self.model,
            delta: Delta {
                role: choice.delta.role.as_deref().map(str_to_role),
                content: choice.delta.content.unwrap_or_default(),
            },
            finish_reason: choice.finish_reason,
        }
    }
}

// ─── Wire types (embeddings) ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct OpenAiEmbeddingResponse {
    data: Vec<OpenAiEmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct OpenAiEmbeddingData {
    index: usize,
    embedding: Vec<f32>,
}

// ─── Helpers ───────────────────────────────────────────────────────────────

fn role_to_str(role: &Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Function => "function",
        Role::Tool => "tool",
    }
}

fn str_to_role(s: &str) -> Role {
    match s {
        "system" => Role::System,
        "user" => Role::User,
        "assistant" => Role::Assistant,
        "function" => Role::Function,
        "tool" => Role::Tool,
        _ => Role::Assistant,
    }
}
