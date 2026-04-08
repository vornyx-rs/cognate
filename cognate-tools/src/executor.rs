//! Automatic tool-call dispatch loop.
//!
//! [`ToolExecutor`] wraps a [`Provider`] and a registry of [`Tool`]s.  When
//! the provider requests a tool call, the executor dispatches it, feeds the
//! result back, and loops until the model produces a final text response or
//! the iteration limit is reached.

use crate::Tool;
use cognate_core::{Error, Message, Provider, Request, Response, ToolCall};
use std::collections::HashMap;
use std::sync::Arc;

/// Executes a request with automatic tool-call dispatch.
///
/// # Example
///
/// ```rust,no_run
/// use cognate_tools::{ToolExecutor, Tool};
/// use cognate_core::{Request, Message};
/// use serde::{Deserialize, Serialize};
/// use schemars::JsonSchema;
///
/// #[derive(Tool, Serialize, Deserialize, JsonSchema)]
/// #[tool(description = "Add two numbers together")]
/// struct Add {
///     a: f64,
///     b: f64,
/// }
///
/// impl Add {
///     async fn run(&self) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
///         Ok(self.a + self.b)
///     }
/// }
///
/// # async fn example(provider: impl cognate_core::Provider + 'static) {
/// let mut executor = ToolExecutor::new(provider);
/// executor.add_tool(Add { a: 0.0, b: 0.0 });
/// let req = Request::new()
///     .with_model("gpt-4o-mini")
///     .with_message(Message::user("What is 3 + 4?"));
/// let response = executor.execute(req).await.unwrap();
/// println!("{}", response.content());
/// # }
/// ```
pub struct ToolExecutor {
    provider: Box<dyn Provider>,
    tools: HashMap<String, Arc<dyn Tool>>,
    /// Maximum number of tool-call round-trips before returning an error.
    pub max_iterations: u32,
}

impl ToolExecutor {
    /// Create a new executor with the given provider and no registered tools.
    pub fn new(provider: impl Provider + 'static) -> Self {
        Self {
            provider: Box::new(provider),
            tools: HashMap::new(),
            max_iterations: 10,
        }
    }

    /// Register a tool.  The tool's [`name`](Tool::name) is used as the key.
    pub fn add_tool(&mut self, tool: impl Tool + 'static) {
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
    }

    /// Build the `tools` array in OpenAI function-calling format.
    fn tool_definitions(&self) -> serde_json::Value {
        let defs: Vec<serde_json::Value> = self
            .tools
            .values()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name(),
                        "description": t.description(),
                        "parameters": t.parameters(),
                    }
                })
            })
            .collect();
        serde_json::Value::Array(defs)
    }

    /// Run the request, automatically dispatching any tool calls until the
    /// model produces a final response.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidRequest`] if the iteration limit is exceeded,
    /// if the model requests an unknown tool, or if tool arguments cannot be
    /// parsed.  Provider errors are propagated as-is.
    pub async fn execute(&self, mut req: Request) -> cognate_core::Result<Response> {
        // Inject tool definitions once; they stay in `extra` for every round.
        if !self.tools.is_empty() {
            req.extra
                .insert("tools".to_string(), self.tool_definitions());
        }

        for iteration in 0..self.max_iterations {
            let response = self.provider.complete(req.clone()).await?;

            let choice = response
                .choices
                .first()
                .ok_or_else(|| Error::InvalidRequest("Provider returned no choices".to_string()))?;

            let tool_calls = match &choice.message.tool_calls {
                Some(calls) if !calls.is_empty() => calls.clone(),
                _ => return Ok(response),
            };

            // Append the assistant message (carrying the tool_calls) to history
            // so the next round has full context.
            req.messages.push(choice.message.clone());

            // Dispatch every tool call in this round.
            for call in &tool_calls {
                let result = self.dispatch(call).await?;
                req.messages
                    .push(Message::tool_result(result, call.id.clone()));
            }

            let _ = iteration; // suppress unused warning on last iteration path
        }

        Err(Error::InvalidRequest(format!(
            "Tool call loop exceeded {} iterations",
            self.max_iterations
        )))
    }

    /// Dispatch a single [`ToolCall`] and return a JSON-string result.
    async fn dispatch(&self, call: &ToolCall) -> cognate_core::Result<String> {
        let tool = self.tools.get(&call.function.name).ok_or_else(|| {
            Error::InvalidRequest(format!("Unknown tool: '{}'", call.function.name))
        })?;

        let args: serde_json::Value = serde_json::from_str(&call.function.arguments)
            .map_err(|e| {
                Error::InvalidRequest(format!(
                    "Invalid arguments for tool '{}': {}",
                    call.function.name, e
                ))
            })?;

        let result = tool
            .call(args)
            .await
            .map_err(|e| Error::InvalidRequest(e.to_string()))?;

        serde_json::to_string(&result).map_err(Error::Json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;
    use async_trait::async_trait;
    use cognate_core::{Choice, MockProvider, Response, Role, ToolCallFunction};

    /// A minimal tool that echoes its input.
    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "Echo the input"
        }
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" }
                },
                "required": ["message"]
            })
        }
        async fn call(
            &self,
            params: serde_json::Value,
        ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
            Ok(params["message"].clone())
        }
    }

    fn make_tool_call_response(call_id: &str, tool_name: &str, args: &str) -> Response {
        Response {
            id: "r1".to_string(),
            model: "test".to_string(),
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: Role::Assistant,
                    content: String::new(),
                    name: None,
                    tool_calls: Some(vec![ToolCall {
                        id: call_id.to_string(),
                        call_type: "function".to_string(),
                        function: ToolCallFunction {
                            name: tool_name.to_string(),
                            arguments: args.to_string(),
                        },
                    }]),
                    tool_call_id: None,
                },
                finish_reason: Some("tool_calls".to_string()),
            }],
            usage: None,
            created: None,
        }
    }

    #[tokio::test]
    async fn test_tool_executor_dispatches_and_loops() {
        let provider = MockProvider::new();

        // First response requests a tool call.
        provider.push_response(make_tool_call_response(
            "call-1",
            "echo",
            r#"{"message": "hello"}"#,
        ));

        // Second response is the final text answer.
        provider.push_response(Response {
            id: "r2".to_string(),
            model: "test".to_string(),
            choices: vec![Choice {
                index: 0,
                message: Message::assistant("The echo said: hello"),
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
            created: None,
        });

        let mut executor = ToolExecutor::new(provider);
        executor.add_tool(EchoTool);

        let req = Request::new()
            .with_model("test")
            .with_message(Message::user("Echo hello"));

        let response = executor.execute(req).await.unwrap();
        assert_eq!(response.content(), "The echo said: hello");
    }

    #[tokio::test]
    async fn test_tool_executor_unknown_tool_returns_error() {
        let provider = MockProvider::new();
        provider.push_response(make_tool_call_response(
            "call-2",
            "nonexistent",
            r#"{}"#,
        ));

        let executor = ToolExecutor::new(provider);
        let req = Request::new().with_model("test");
        let result = executor.execute(req).await;
        assert!(matches!(result, Err(Error::InvalidRequest(_))));
    }
}
