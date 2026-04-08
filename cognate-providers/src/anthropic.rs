//! Anthropic provider implementation.
//!
//! Supports the Anthropic Messages API including tool use and streaming.

use async_trait::async_trait;
use cognate_core::{
    Chunk, Choice, Delta, Message, Provider, ProviderConfig, Request, Response, Role, ToolCall,
    ToolCallFunction,
};
use futures::stream::{BoxStream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};

use crate::retry::{with_retry, RetryConfig};
use crate::sse::SseStream;

const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com/v1";
/// Minimum supported Anthropic API version.
const ANTHROPIC_VERSION: &str = "2023-06-01";

// ─── Provider struct ───────────────────────────────────────────────────────

/// Provider client for the Anthropic API (Claude models).
///
/// # Example
///
/// ```rust,no_run
/// use cognate_providers::AnthropicProvider;
/// use cognate_core::{Provider, Request, Message};
///
/// # async fn run() -> cognate_core::Result<()> {
/// let provider = AnthropicProvider::new(std::env::var("ANTHROPIC_API_KEY").unwrap())?;
/// let resp = provider
///     .complete(
///         Request::new()
///             .with_model("claude-3-5-sonnet-20241022")
///             .with_message(Message::user("Hello!")),
///     )
///     .await?;
/// println!("{}", resp.content());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    client: reqwest::Client,
    config: ProviderConfig,
    base_url: String,
    rate_limiter: Option<cognate_core::TokenBucket>,
    retry_config: RetryConfig,
}

impl AnthropicProvider {
    /// Create a provider from an API key using default settings.
    pub fn new(api_key: impl Into<String>) -> cognate_core::Result<Self> {
        Self::with_config(ProviderConfig::new(api_key))
    }

    /// Create a provider with full [`ProviderConfig`] control.
    pub fn with_config(config: ProviderConfig) -> cognate_core::Result<Self> {
        let base_url = if config.base_url.is_empty() {
            ANTHROPIC_BASE_URL.to_string()
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

    async fn handle_error(response: reqwest::Response) -> cognate_core::Error {
        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        if status == 429 {
            cognate_core::Error::RateLimit { retry_after: 60 }
        } else {
            cognate_core::Error::Api { status, message: body }
        }
    }

    fn auth_headers(&self) -> [(&'static str, String); 2] {
        [
            ("x-api-key", self.config.api_key.clone()),
            ("anthropic-version", ANTHROPIC_VERSION.to_string()),
        ]
    }
}

// ─── Provider impl ─────────────────────────────────────────────────────────

#[async_trait]
impl Provider for AnthropicProvider {
    async fn complete(&self, req: Request) -> cognate_core::Result<Response> {
        with_retry(&self.retry_config, || async {
            if let Some(ref limiter) = self.rate_limiter {
                limiter.acquire(1.0).await;
            }

            let url = format!("{}/messages", self.base_url);
            let body = AnthropicRequest::from_request(&req);

            let mut builder = self
                .client
                .post(&url)
                .header("Content-Type", "application/json");
            for (k, v) in self.auth_headers() {
                builder = builder.header(k, v);
            }

            let http_resp = builder.json(&body).send().await?;

            if !http_resp.status().is_success() {
                return Err(Self::handle_error(http_resp).await);
            }

            let resp: AnthropicResponse = http_resp.json().await?;
            Ok(resp.into_response())
        })
        .await
    }

    async fn stream(&self, req: Request) -> cognate_core::Result<BoxStream<'static, cognate_core::Result<Chunk>>> {
        with_retry(&self.retry_config, || async {
            if let Some(ref limiter) = self.rate_limiter {
                limiter.acquire(1.0).await;
            }

            let url = format!("{}/messages", self.base_url);
            let mut body = AnthropicRequest::from_request(&req);
            body.stream = Some(true);

            let mut builder = self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("Accept", "text/event-stream");
            for (k, v) in self.auth_headers() {
                builder = builder.header(k, v);
            }

            let http_resp = builder.json(&body).send().await?;
            if !http_resp.status().is_success() {
                return Err(Self::handle_error(http_resp).await);
            }

            // Anthropic streaming events:
            //   event: content_block_delta  — incremental text
            //   event: message_stop         — stream finished
            let chunk_stream = SseStream::new(http_resp.bytes_stream())
                .try_filter_map(|event| async move {
                    match event.event.as_str() {
                        "content_block_delta" => {
                            match serde_json::from_str::<AnthropicStreamDelta>(&event.data) {
                                Ok(d) => Ok(Some(Chunk {
                                    id: "anthropic".to_string(),
                                    model: String::new(),
                                    delta: Delta {
                                        role: None,
                                        content: d.delta.text.unwrap_or_default(),
                                    },
                                    finish_reason: None,
                                })),
                                Err(e) => Err(cognate_core::Error::Json(e)),
                            }
                        }
                        "message_stop" => Ok(Some(Chunk {
                            id: "anthropic".to_string(),
                            model: String::new(),
                            delta: Delta::default(),
                            finish_reason: Some("stop".to_string()),
                        })),
                        _ => Ok(None),
                    }
                })
                .boxed();

            Ok(chunk_stream)
        })
        .await
    }
}

// ─── Wire types (request) ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    /// Tool definitions in Anthropic format (different from OpenAI).
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
    /// `"auto"`, `"any"`, or `{"type":"tool","name":"…"}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<serde_json::Value>,
}

impl AnthropicRequest {
    fn from_request(req: &Request) -> Self {
        // Anthropic requires the system prompt to be separate.
        let mut system: Option<String> = None;
        let messages: Vec<AnthropicMessage> = req
            .messages
            .iter()
            .filter_map(|m| {
                if m.role == Role::System {
                    system = Some(m.content.clone());
                    None
                } else {
                    Some(AnthropicMessage::from_msg(m))
                }
            })
            .collect();

        // Convert OpenAI-format tool definitions to Anthropic format.
        let tools = req.extra.get("tools").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let func = t.get("function")?;
                    Some(AnthropicTool {
                        name: func.get("name")?.as_str()?.to_string(),
                        description: func
                            .get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string(),
                        input_schema: func
                            .get("parameters")
                            .cloned()
                            .unwrap_or_else(|| serde_json::json!({"type":"object","properties":{}})),
                    })
                })
                .collect::<Vec<_>>()
        });

        Self {
            model: req.model.clone(),
            messages,
            max_tokens: req.max_tokens.unwrap_or(4096),
            system,
            temperature: req.temperature,
            stream: req.stream,
            tools,
            tool_choice: req.extra.get("tool_choice").cloned(),
        }
    }
}

/// A message in Anthropic wire format.
///
/// `content` can be a plain string or a JSON array of content blocks
/// (required for tool results).
#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: serde_json::Value,
}

impl AnthropicMessage {
    fn from_msg(msg: &Message) -> Self {
        let role = match msg.role {
            Role::Assistant => "assistant",
            _ => "user",
        };

        // Tool-result messages must use the structured content-block format.
        if msg.role == Role::Tool {
            let tool_use_id = msg
                .tool_call_id
                .clone()
                .unwrap_or_else(|| "unknown".to_string());
            return Self {
                role: "user".to_string(),
                content: serde_json::json!([{
                    "type": "tool_result",
                    "tool_use_id": tool_use_id,
                    "content": msg.content,
                }]),
            };
        }

        // Assistant messages that carry tool_calls need content blocks.
        if let Some(calls) = &msg.tool_calls {
            let mut blocks: Vec<serde_json::Value> = Vec::new();
            if !msg.content.is_empty() {
                blocks.push(serde_json::json!({"type":"text","text":msg.content}));
            }
            for tc in calls {
                let input: serde_json::Value =
                    serde_json::from_str(&tc.function.arguments).unwrap_or_default();
                blocks.push(serde_json::json!({
                    "type": "tool_use",
                    "id": tc.id,
                    "name": tc.function.name,
                    "input": input,
                }));
            }
            return Self {
                role: role.to_string(),
                content: serde_json::Value::Array(blocks),
            };
        }

        Self {
            role: role.to_string(),
            content: serde_json::Value::String(msg.content.clone()),
        }
    }
}

#[derive(Debug, Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

// ─── Wire types (response) ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    model: String,
    content: Vec<AnthropicContentBlock>,
    usage: AnthropicUsage,
    stop_reason: Option<String>,
}

/// A content block in an Anthropic response.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicContentBlock {
    /// Plain text response.
    Text {
        /// The text content.
        text: String,
    },
    /// A tool invocation.
    ToolUse {
        /// Unique identifier for this tool call.
        id: String,
        /// Name of the tool to invoke.
        name: String,
        /// JSON object of arguments.
        input: serde_json::Value,
    },
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

impl AnthropicResponse {
    fn into_response(self) -> Response {
        let mut text_parts: Vec<String> = Vec::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();

        for block in self.content {
            match block {
                AnthropicContentBlock::Text { text } => text_parts.push(text),
                AnthropicContentBlock::ToolUse { id, name, input } => {
                    let arguments = serde_json::to_string(&input).unwrap_or_default();
                    tool_calls.push(ToolCall {
                        id,
                        call_type: "function".to_string(),
                        function: ToolCallFunction { name, arguments },
                    });
                }
            }
        }

        let tool_calls_opt = if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        };

        let finish_reason = self.stop_reason.map(|r| match r.as_str() {
            "end_turn" => "stop".to_string(),
            "tool_use" => "tool_calls".to_string(),
            other => other.to_string(),
        });

        Response {
            id: self.id,
            model: self.model,
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: Role::Assistant,
                    content: text_parts.join(""),
                    name: None,
                    tool_calls: tool_calls_opt,
                    tool_call_id: None,
                },
                finish_reason,
            }],
            usage: Some(cognate_core::Usage {
                prompt_tokens: self.usage.input_tokens,
                completion_tokens: self.usage.output_tokens,
                total_tokens: self.usage.input_tokens + self.usage.output_tokens,
            }),
            created: None,
        }
    }
}

// ─── Wire types (streaming) ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct AnthropicStreamDelta {
    delta: AnthropicDeltaContent,
}

#[derive(Debug, Deserialize)]
struct AnthropicDeltaContent {
    /// Present on `content_block_delta` events of type `text_delta`.
    text: Option<String>,
}
