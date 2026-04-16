//! Claude API provider implementation.

use async_trait::async_trait;

use super::provider::{
    CompletionRequest, CompletionResponse, CompletionStream, LlmProvider, StopReason, TokenUsage,
};
use crate::{Error, Result};

/// Default Claude API base URL.
const DEFAULT_API_BASE: &str = "https://api.anthropic.com";

/// LLM provider using Anthropic's Claude API.
pub struct ClaudeProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
    api_base: String,
}

impl ClaudeProvider {
    /// Creates a new Claude provider.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Anthropic API key
    /// * `model` - Model ID (e.g., "claude-sonnet-4-20250514")
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            client: reqwest::Client::new(),
            api_base: DEFAULT_API_BASE.to_string(),
        }
    }

    /// Override the API base URL (for testing).
    #[cfg(test)]
    fn with_api_base(mut self, base: impl Into<String>) -> Self {
        self.api_base = base.into();
        self
    }

    /// Build the request body for a completion request.
    fn build_request_body(&self, request: &CompletionRequest) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": self.model,
            "max_tokens": request.max_tokens,
            "messages": request.messages,
        });

        if let Some(ref system) = request.system_prompt {
            body["system"] = serde_json::json!(system);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        if !request.stop_sequences.is_empty() {
            body["stop_sequences"] = serde_json::json!(&request.stop_sequences);
        }

        body
    }

    /// Parse a Claude API response body into a `CompletionResponse`.
    fn parse_response(response_body: &serde_json::Value) -> Result<CompletionResponse> {
        // Extract content
        let content = response_body["content"][0]["text"]
            .as_str()
            .ok_or_else(|| Error::llm("Missing content in Claude response"))?
            .to_string();

        // Extract token usage
        let usage = response_body["usage"]
            .as_object()
            .ok_or_else(|| Error::llm("Missing usage data in Claude response"))?;

        let input_tokens = usage["input_tokens"]
            .as_u64()
            .ok_or_else(|| Error::llm("Invalid input_tokens"))?;
        let output_tokens = usage["output_tokens"]
            .as_u64()
            .ok_or_else(|| Error::llm("Invalid output_tokens"))?;

        // Extract stop reason
        let stop_reason_str = response_body["stop_reason"]
            .as_str()
            .ok_or_else(|| Error::llm("Missing stop_reason"))?;

        let stop_reason = match stop_reason_str {
            "end_turn" => StopReason::EndTurn,
            "max_tokens" => StopReason::MaxTokens,
            "stop_sequence" => StopReason::StopSequence,
            other => return Err(Error::llm(format!("Unknown stop reason: {}", other))),
        };

        Ok(CompletionResponse {
            content,
            tokens_used: TokenUsage {
                input: input_tokens,
                output: output_tokens,
            },
            stop_reason,
        })
    }
}

#[async_trait]
impl LlmProvider for ClaudeProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let body = self.build_request_body(&request);

        // Make API request
        let response = self
            .client
            .post(format!("{}/v1/messages", self.api_base))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::llm_with_source("Failed to call Claude API", e))?;

        // Check for errors
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::llm(format!(
                "Claude API error {}: {}",
                status, error_text
            )));
        }

        // Parse response
        let response_body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::llm_with_source("Failed to parse Claude response", e))?;

        Self::parse_response(&response_body)
    }

    async fn complete_streaming(&self, _request: CompletionRequest) -> Result<CompletionStream> {
        // Streaming implementation deferred to Phase 3
        Err(Error::llm("Streaming not yet implemented"))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::llm::Message;

    #[test]
    fn test_claude_provider_construction() {
        let provider = ClaudeProvider::new("test-key", "claude-3-opus");
        assert_eq!(provider.api_key, "test-key");
        assert_eq!(provider.model, "claude-3-opus");
        assert_eq!(provider.api_base, DEFAULT_API_BASE);
    }

    #[test]
    fn test_build_request_body_minimal() {
        let provider = ClaudeProvider::new("key", "model-1");
        let request = CompletionRequest::new(vec![Message::user("hello")]);
        let body = provider.build_request_body(&request);

        assert_eq!(body["model"], "model-1");
        assert!(body.get("system").is_none());
        assert!(body.get("temperature").is_none());
        assert!(body.get("stop_sequences").is_none());
    }

    #[test]
    fn test_build_request_body_with_all_options() {
        let provider = ClaudeProvider::new("key", "model-1");
        let request = CompletionRequest::new(vec![Message::user("hello")])
            .with_system_prompt("You are helpful.")
            .with_temperature(0.7)
            .with_max_tokens(500)
            .with_stop_sequence("STOP");
        let body = provider.build_request_body(&request);

        assert_eq!(body["system"], "You are helpful.");
        let temp = body["temperature"].as_f64().unwrap();
        assert!((temp - 0.7).abs() < 0.01);
        assert_eq!(body["max_tokens"], 500);
        assert_eq!(body["stop_sequences"][0], "STOP");
    }

    #[test]
    fn test_parse_response_success_end_turn() {
        let body = serde_json::json!({
            "content": [{"type": "text", "text": "Hello!"}],
            "usage": {"input_tokens": 10, "output_tokens": 5},
            "stop_reason": "end_turn"
        });
        let resp = ClaudeProvider::parse_response(&body).unwrap();
        assert_eq!(resp.content, "Hello!");
        assert_eq!(resp.tokens_used.input, 10);
        assert_eq!(resp.tokens_used.output, 5);
        assert!(matches!(resp.stop_reason, StopReason::EndTurn));
    }

    #[test]
    fn test_parse_response_max_tokens() {
        let body = serde_json::json!({
            "content": [{"type": "text", "text": "partial"}],
            "usage": {"input_tokens": 1, "output_tokens": 100},
            "stop_reason": "max_tokens"
        });
        let resp = ClaudeProvider::parse_response(&body).unwrap();
        assert!(matches!(resp.stop_reason, StopReason::MaxTokens));
    }

    #[test]
    fn test_parse_response_stop_sequence() {
        let body = serde_json::json!({
            "content": [{"type": "text", "text": "output"}],
            "usage": {"input_tokens": 1, "output_tokens": 1},
            "stop_reason": "stop_sequence"
        });
        let resp = ClaudeProvider::parse_response(&body).unwrap();
        assert!(matches!(resp.stop_reason, StopReason::StopSequence));
    }

    #[test]
    fn test_parse_response_unknown_stop_reason() {
        let body = serde_json::json!({
            "content": [{"type": "text", "text": "output"}],
            "usage": {"input_tokens": 1, "output_tokens": 1},
            "stop_reason": "tool_use"
        });
        let err = ClaudeProvider::parse_response(&body).unwrap_err();
        assert!(err.to_string().contains("Unknown stop reason"));
    }

    #[test]
    fn test_parse_response_missing_content() {
        let body = serde_json::json!({
            "content": [],
            "usage": {"input_tokens": 1, "output_tokens": 1},
            "stop_reason": "end_turn"
        });
        let err = ClaudeProvider::parse_response(&body).unwrap_err();
        assert!(err.to_string().contains("Missing content"));
    }

    #[test]
    fn test_parse_response_missing_usage() {
        let body = serde_json::json!({
            "content": [{"type": "text", "text": "hi"}],
            "stop_reason": "end_turn"
        });
        let err = ClaudeProvider::parse_response(&body).unwrap_err();
        assert!(err.to_string().contains("Missing usage"));
    }

    #[test]
    fn test_parse_response_missing_stop_reason() {
        let body = serde_json::json!({
            "content": [{"type": "text", "text": "hi"}],
            "usage": {"input_tokens": 1, "output_tokens": 1}
        });
        let err = ClaudeProvider::parse_response(&body).unwrap_err();
        assert!(err.to_string().contains("Missing stop_reason"));
    }

    #[tokio::test]
    async fn test_complete_streaming_not_implemented() {
        let provider = ClaudeProvider::new("key", "model");
        let request = CompletionRequest::new(vec![Message::user("hi")]);
        let err = provider.complete_streaming(request).await.err().unwrap();
        assert!(err.to_string().contains("Streaming not yet implemented"));
    }

    #[tokio::test]
    async fn test_complete_success_with_wiremock() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "content": [{"type": "text", "text": "Hello from Claude!"}],
            "usage": {"input_tokens": 12, "output_tokens": 6},
            "stop_reason": "end_turn"
        });

        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("x-api-key", "test-key"))
            .and(header("anthropic-version", "2023-06-01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let provider =
            ClaudeProvider::new("test-key", "claude-test").with_api_base(mock_server.uri());

        let request = CompletionRequest::new(vec![Message::user("Say hello")]).with_max_tokens(100);

        let response = provider.complete(request).await.unwrap();
        assert_eq!(response.content, "Hello from Claude!");
        assert_eq!(response.tokens_used.input, 12);
        assert_eq!(response.tokens_used.output, 6);
        assert!(matches!(response.stop_reason, StopReason::EndTurn));
    }

    #[tokio::test]
    async fn test_complete_api_error_response() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(429).set_body_string("Rate limit exceeded"))
            .mount(&mock_server)
            .await;

        let provider =
            ClaudeProvider::new("test-key", "claude-test").with_api_base(mock_server.uri());

        let request = CompletionRequest::new(vec![Message::user("hello")]);
        let err = provider.complete(request).await.unwrap_err();
        assert!(err.to_string().contains("429"));
        assert!(err.to_string().contains("Rate limit"));
    }

    #[tokio::test]
    async fn test_complete_with_system_prompt_and_temperature() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "content": [{"type": "text", "text": "ok"}],
            "usage": {"input_tokens": 5, "output_tokens": 1},
            "stop_reason": "end_turn"
        });

        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let provider =
            ClaudeProvider::new("test-key", "claude-test").with_api_base(mock_server.uri());

        let request = CompletionRequest::new(vec![Message::user("hello")])
            .with_system_prompt("Be brief")
            .with_temperature(0.5);

        let response = provider.complete(request).await.unwrap();
        assert_eq!(response.content, "ok");
    }

    // Integration test (requires API key, run manually)
    #[tokio::test]
    #[ignore]
    #[allow(clippy::expect_used)]
    async fn test_claude_provider_integration() {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .expect("ANTHROPIC_API_KEY must be set for integration tests");

        let provider = ClaudeProvider::new(api_key, "claude-sonnet-4-20250514");

        let request = CompletionRequest::new(vec![Message::user("Say hello")]).with_max_tokens(100);

        let response = provider.complete(request).await.unwrap();

        assert!(!response.content.is_empty());
        assert!(response.tokens_used.output > 0);
    }
}
