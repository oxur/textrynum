# Stage 1.3: LLM Abstraction Layer

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/0001-phase-1-implementation.md, lines 1422-2216. Original bytes are recoverable from import commit 2d006a7. -->

## Stage 1.3: LLM Abstraction Layer

**Goal:** Wrap LLM interaction behind a trait for testability and provider flexibility.

**Dependencies:** Stage 1.2 (core types and error handling)

### Detailed Implementation Steps

#### Step 1: Define LLM Provider Trait

**Before writing code:**
1. Load `assets/ai/ai-rust/guides/06-traits.md`
2. Load `assets/ai/ai-rust/guides/07-concurrency-async.md`
3. Review async best practices (no blocking in async)

**Create directory structure:**

```bash
mkdir -p crates/ecl-core/src/llm
```

**`crates/ecl-core/src/llm/provider.rs`**

```rust
//! LLM provider abstraction.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Abstraction over LLM providers (Claude, GPT, etc.).
///
/// This trait allows swapping LLM backends without changing workflow code.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Completes a prompt and returns the full response.
    ///
    /// This is a blocking call that waits for the entire response.
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;

    /// Completes a prompt with streaming response.
    ///
    /// Returns a stream of response chunks as they arrive.
    async fn complete_streaming(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionStream>;
}

/// A request to complete a prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// System prompt (context/instructions)
    pub system_prompt: Option<String>,

    /// Conversation messages
    pub messages: Vec<Message>,

    /// Maximum tokens to generate
    pub max_tokens: u32,

    /// Temperature (0.0 = deterministic, 1.0 = creative)
    pub temperature: Option<f32>,

    /// Stop sequences
    pub stop_sequences: Vec<String>,
}

impl CompletionRequest {
    /// Creates a new completion request with default settings.
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            system_prompt: None,
            messages,
            max_tokens: 1024,
            temperature: None,
            stop_sequences: Vec::new(),
        }
    }

    /// Sets the system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Sets the maximum tokens.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Sets the temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Adds a stop sequence.
    pub fn with_stop_sequence(mut self, sequence: impl Into<String>) -> Self {
        self.stop_sequences.push(sequence.into());
        self
    }
}

/// A message in the conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender
    pub role: Role,

    /// Message content
    pub content: String,
}

impl Message {
    /// Creates a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    /// Creates an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

/// Role of a message sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// User message
    User,
    /// Assistant message
    Assistant,
}

/// Response from an LLM completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Generated content
    pub content: String,

    /// Token usage statistics
    pub tokens_used: TokenUsage,

    /// Why the model stopped generating
    pub stop_reason: StopReason,
}

/// Token usage statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input tokens consumed
    pub input: u64,

    /// Output tokens generated
    pub output: u64,
}

impl TokenUsage {
    /// Total tokens used (input + output).
    pub fn total(&self) -> u64 {
        self.input + self.output
    }
}

/// Reason why the model stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum StopReason {
    /// Reached the end of the response naturally
    EndTurn,

    /// Hit the maximum token limit
    MaxTokens,

    /// Encountered a stop sequence
    StopSequence,
}

/// Streaming response from an LLM completion.
///
/// This is a placeholder for now; full implementation in Phase 3.
pub struct CompletionStream {
    // Future: implement streaming using tokio::sync::mpsc or similar
    _private: (),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_constructors() {
        let user_msg = Message::user("Hello");
        assert_eq!(user_msg.role, Role::User);
        assert_eq!(user_msg.content, "Hello");

        let asst_msg = Message::assistant("Hi there");
        assert_eq!(asst_msg.role, Role::Assistant);
        assert_eq!(asst_msg.content, "Hi there");
    }

    #[test]
    fn test_completion_request_builder() {
        let request = CompletionRequest::new(vec![Message::user("Test")])
            .with_system_prompt("You are helpful")
            .with_max_tokens(2048)
            .with_temperature(0.7)
            .with_stop_sequence("\n\n");

        assert_eq!(
            request.system_prompt,
            Some("You are helpful".to_string())
        );
        assert_eq!(request.max_tokens, 2048);
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.stop_sequences, vec!["\n\n"]);
    }

    #[test]
    fn test_token_usage_total() {
        let usage = TokenUsage {
            input: 100,
            output: 200,
        };
        assert_eq!(usage.total(), 300);
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message::user("test content");
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, deserialized);
    }
}
```

#### Step 2: Implement Claude Provider

**`crates/ecl-core/src/llm/claude.rs`**

```rust
//! Claude API provider implementation.

use async_trait::async_trait;

use super::provider::{
    CompletionRequest, CompletionResponse, CompletionStream, LlmProvider, StopReason, TokenUsage,
};
use crate::{Error, Result};

/// LLM provider using Anthropic's Claude API.
pub struct ClaudeProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
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
        }
    }
}

#[async_trait]
impl LlmProvider for ClaudeProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        // Build Claude API request
        let mut body = serde_json::json!({
            "model": self.model,
            "max_tokens": request.max_tokens,
            "messages": request.messages,
        });

        if let Some(system) = request.system_prompt {
            body["system"] = serde_json::json!(system);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        if !request.stop_sequences.is_empty() {
            body["stop_sequences"] = serde_json::json!(request.stop_sequences);
        }

        // Make API request
        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
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

        // Extract content
        let content = response_body["content"][0]["text"]
            .as_str()
            .ok_or_else(|| Error::llm("Missing content in Claude response"))?
            .to_string();

        // Extract token usage
        let usage = response_body["usage"].as_object()
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

    async fn complete_streaming(
        &self,
        _request: CompletionRequest,
    ) -> Result<CompletionStream> {
        // Streaming implementation deferred to Phase 3
        Err(Error::llm("Streaming not yet implemented"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::Message;

    #[test]
    fn test_claude_provider_construction() {
        let provider = ClaudeProvider::new("test-key", "claude-3-opus");
        assert_eq!(provider.api_key, "test-key");
        assert_eq!(provider.model, "claude-3-opus");
    }

    // Integration test (requires API key, run manually)
    #[tokio::test]
    #[ignore]
    async fn test_claude_provider_integration() {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .expect("ANTHROPIC_API_KEY must be set for integration tests");

        let provider = ClaudeProvider::new(api_key, "claude-sonnet-4-20250514");

        let request = CompletionRequest::new(vec![Message::user("Say hello")])
            .with_max_tokens(100);

        let response = provider.complete(request).await.unwrap();

        assert!(!response.content.is_empty());
        assert!(response.tokens_used.output > 0);
    }
}
```

#### Step 3: Implement Mock Provider

**`crates/ecl-core/src/llm/mock.rs`**

```rust
//! Mock LLM provider for testing.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::provider::{
    CompletionRequest, CompletionResponse, CompletionStream, LlmProvider, StopReason, TokenUsage,
};
use crate::Result;

/// Mock LLM provider that returns canned responses.
///
/// Useful for testing without making actual API calls.
#[derive(Clone)]
pub struct MockLlmProvider {
    responses: Arc<Mutex<MockResponses>>,
}

struct MockResponses {
    canned: Vec<String>,
    index: usize,
}

impl MockLlmProvider {
    /// Creates a new mock provider with canned responses.
    ///
    /// Responses are returned in order. After all responses are used,
    /// the provider cycles back to the first response.
    ///
    /// # Examples
    ///
    /// ```
    /// use ecl_core::llm::MockLlmProvider;
    ///
    /// let provider = MockLlmProvider::new(vec![
    ///     "First response".to_string(),
    ///     "Second response".to_string(),
    /// ]);
    /// ```
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(MockResponses {
                canned: responses,
                index: 0,
            })),
        }
    }

    /// Creates a mock provider with a single response.
    pub fn with_response(response: impl Into<String>) -> Self {
        Self::new(vec![response.into()])
    }
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
        let mut responses = self.responses.lock().await;

        // Get current response
        let content = responses.canned[responses.index].clone();

        // Advance to next response (cycling)
        responses.index = (responses.index + 1) % responses.canned.len();

        Ok(CompletionResponse {
            content,
            tokens_used: TokenUsage {
                input: 10,  // Mock values
                output: 20,
            },
            stop_reason: StopReason::EndTurn,
        })
    }

    async fn complete_streaming(
        &self,
        _request: CompletionRequest,
    ) -> Result<CompletionStream> {
        // Streaming not implemented for mock
        Err(crate::Error::llm("Streaming not supported in mock provider"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::Message;

    #[tokio::test]
    async fn test_mock_provider_single_response() {
        let provider = MockLlmProvider::with_response("Test response");

        let request = CompletionRequest::new(vec![Message::user("Hello")]);

        let response = provider.complete(request).await.unwrap();
        assert_eq!(response.content, "Test response");
    }

    #[tokio::test]
    async fn test_mock_provider_multiple_responses() {
        let provider = MockLlmProvider::new(vec![
            "First".to_string(),
            "Second".to_string(),
            "Third".to_string(),
        ]);

        let request = CompletionRequest::new(vec![Message::user("Test")]);

        assert_eq!(
            provider.complete(request.clone()).await.unwrap().content,
            "First"
        );
        assert_eq!(
            provider.complete(request.clone()).await.unwrap().content,
            "Second"
        );
        assert_eq!(
            provider.complete(request.clone()).await.unwrap().content,
            "Third"
        );
        // Cycles back
        assert_eq!(
            provider.complete(request.clone()).await.unwrap().content,
            "First"
        );
    }

    #[tokio::test]
    async fn test_mock_provider_clone() {
        let provider = MockLlmProvider::with_response("Shared");
        let provider2 = provider.clone();

        let request = CompletionRequest::new(vec![Message::user("Test")]);

        // Both providers share the same state
        provider.complete(request.clone()).await.unwrap();
        // Would cycle if shared, but we have only one response so it's the same
        let response = provider2.complete(request).await.unwrap();
        assert_eq!(response.content, "Shared");
    }
}
```

#### Step 4: Implement Retry Logic

**`crates/ecl-core/src/llm/retry.rs`**

```rust
//! Retry wrapper for LLM providers.

use async_trait::async_trait;
use backon::{ExponentialBuilder, Retryable};
use std::sync::Arc;
use std::time::Duration;

use super::provider::{CompletionRequest, CompletionResponse, CompletionStream, LlmProvider};
use crate::{Error, Result};

/// Wraps an LLM provider with retry logic.
pub struct RetryWrapper {
    inner: Arc<dyn LlmProvider>,
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
}

impl RetryWrapper {
    /// Creates a new retry wrapper with default settings.
    ///
    /// Default settings:
    /// - Max attempts: 3
    /// - Initial delay: 1 second
    /// - Max delay: 10 seconds
    /// - Multiplier: 2.0 (exponential backoff)
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            inner: provider,
            max_attempts: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(10),
        }
    }

    /// Sets the maximum number of attempts.
    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    /// Sets the initial delay between retries.
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Sets the maximum delay between retries.
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Determines if an error should be retried.
    fn should_retry(error: &Error) -> bool {
        error.is_retryable()
    }
}

#[async_trait]
impl LlmProvider for RetryWrapper {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let backoff = ExponentialBuilder::default()
            .with_min_delay(self.initial_delay)
            .with_max_delay(self.max_delay)
            .with_max_times(self.max_attempts as usize);

        let provider = self.inner.clone();
        let request_clone = request.clone();

        // Use backon for retry logic
        (|| async {
            provider.complete(request_clone.clone()).await
        })
        .retry(backoff)
        .when(Self::should_retry)
        .await
    }

    async fn complete_streaming(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionStream> {
        // Streaming with retry is complex, defer to Phase 3
        self.inner.complete_streaming(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockLlmProvider;

    #[tokio::test]
    async fn test_retry_wrapper_success() {
        let mock = Arc::new(MockLlmProvider::with_response("Success"));
        let retry = RetryWrapper::new(mock);

        let request = CompletionRequest::new(vec![crate::llm::Message::user("Test")]);
        let response = retry.complete(request).await.unwrap();

        assert_eq!(response.content, "Success");
    }

    #[test]
    fn test_retry_wrapper_builder() {
        let mock = Arc::new(MockLlmProvider::with_response("Test"));
        let retry = RetryWrapper::new(mock)
            .with_max_attempts(5)
            .with_initial_delay(Duration::from_millis(500))
            .with_max_delay(Duration::from_secs(30));

        assert_eq!(retry.max_attempts, 5);
        assert_eq!(retry.initial_delay, Duration::from_millis(500));
        assert_eq!(retry.max_delay, Duration::from_secs(30));
    }
}
```

#### Step 5: Module Organization

**`crates/ecl-core/src/llm/mod.rs`**

```rust
//! LLM provider abstractions and implementations.

mod claude;
mod mock;
mod provider;
mod retry;

pub use claude::ClaudeProvider;
pub use mock::MockLlmProvider;
pub use provider::{
    CompletionRequest, CompletionResponse, LlmProvider, Message, Role, StopReason, TokenUsage,
};
pub use retry::RetryWrapper;
```

**Update `crates/ecl-core/src/lib.rs`**

```rust
pub mod llm;

// Add to re-exports
pub use llm::{CompletionRequest, CompletionResponse, LlmProvider, Message};
```

### Testing Strategy

**Test coverage goal:** ≥95%

**Tests needed:**

1. **Provider trait tests**:
   - Message construction
   - Request builder pattern
   - Token usage calculation
   - Serialization roundtrips

2. **Mock provider tests**:
   - Single response
   - Multiple responses with cycling
   - Clone behavior (shared state)

3. **Claude provider tests**:
   - Construction
   - Integration test (manual, with real API key)

4. **Retry wrapper tests**:
   - Success case (no retry needed)
   - Retry on transient failure
   - Max attempts exhausted
   - Non-retryable errors fail fast

**Integration test setup:**

```bash
# In justfile
test-integration:
    ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY} cargo test --workspace -- --ignored
```

### Potential Blockers

1. **`llm` crate doesn't support Claude or has different API**
   - **Mitigation**: Verified via crates.io research
   - **Fallback**: Use `reqwest` directly (as shown in ClaudeProvider)

2. **Retry logic complexity**
   - **Mitigation**: Use `backon` crate as planned
   - **Alternative**: Hand-roll simple exponential backoff

3. **Async trait object safety**
   - **Mitigation**: Use `async-trait` crate
   - **Note**: Requires boxing futures, minor perf cost

### Acceptance Criteria

- [ ] `LlmProvider` trait is object-safe with async-trait
- [ ] `ClaudeProvider` makes successful API calls (integration test)
- [ ] `MockLlmProvider` returns canned responses in tests
- [ ] Retry logic retries on transient errors
- [ ] Retry logic fails fast on permanent errors
- [ ] All types implement required Restate serialization traits
- [ ] Test coverage ≥95%
- [ ] No blocking I/O in async functions
- [ ] All public APIs documented

### Estimated Effort

**Time:** 4-5 hours

**Breakdown:**
- Provider trait and types: 1 hour
- Claude implementation: 1.5 hours
- Mock implementation: 30 min
- Retry logic: 1 hour
- Testing: 1.5 hours

---

