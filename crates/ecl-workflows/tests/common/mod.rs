//! Common test utilities and harness for ECL workflows integration tests.

use ecl_core::llm::MockLlmProvider;
use std::sync::Arc;

/// Test harness for integration tests.
///
/// Provides utilities for setting up test environments with mock LLM providers
/// and test data helpers.
pub struct TestHarness {
    /// Mock LLM provider for testing
    pub llm: Arc<MockLlmProvider>,
}

impl TestHarness {
    /// Creates a new test harness with a single-response mock LLM.
    pub fn new() -> Self {
        let llm = Arc::new(MockLlmProvider::with_response("Test response"));
        Self { llm }
    }

    /// Creates a test harness with a custom mock LLM provider.
    pub fn with_llm(llm: Arc<MockLlmProvider>) -> Self {
        Self { llm }
    }

    /// Creates a test harness with multiple mock responses.
    pub fn with_responses(responses: Vec<String>) -> Self {
        let llm = Arc::new(MockLlmProvider::new(responses));
        Self { llm }
    }
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create a mock LLM that always passes critique on first attempt.
pub fn mock_pass_immediately() -> Arc<MockLlmProvider> {
    Arc::new(MockLlmProvider::new(vec![
        "Generated content.".to_string(),
        r#"{"decision": "pass", "critique": "Looks good!"}"#.to_string(),
    ]))
}

/// Helper to create a mock LLM that revises once then passes.
pub fn mock_revise_once() -> Arc<MockLlmProvider> {
    Arc::new(MockLlmProvider::new(vec![
        "Initial draft.".to_string(),
        r#"{"decision": "revise", "critique": "Needs work", "feedback": "Add more detail"}"#
            .to_string(),
        "Improved draft with more detail.".to_string(),
        r#"{"decision": "pass", "critique": "Much better!"}"#.to_string(),
    ]))
}

/// Helper to create a mock LLM that always requests revision (for testing max revisions).
pub fn mock_always_revise() -> Arc<MockLlmProvider> {
    Arc::new(MockLlmProvider::new(vec![
        "Draft.".to_string(),
        r#"{"decision": "revise", "critique": "Try again", "feedback": "More work needed"}"#
            .to_string(),
        "Revised 1.".to_string(),
        r#"{"decision": "revise", "critique": "Still not good", "feedback": "Keep trying"}"#
            .to_string(),
        "Revised 2.".to_string(),
        r#"{"decision": "revise", "critique": "Nope", "feedback": "Again"}"#.to_string(),
        "Revised 3.".to_string(),
        r#"{"decision": "revise", "critique": "Still no", "feedback": "More"}"#.to_string(),
    ]))
}
