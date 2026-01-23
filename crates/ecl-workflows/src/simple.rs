//! Simple 2-step workflow for Phase 1 validation.

use ecl_core::llm::{CompletionRequest, LlmProvider, Message};
use ecl_core::{Result as EclResult, WorkflowId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Input for the simple workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleWorkflowInput {
    /// Unique ID for this workflow instance
    pub workflow_id: WorkflowId,

    /// Topic to generate content about
    pub topic: String,
}

impl SimpleWorkflowInput {
    /// Creates a new workflow input.
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            workflow_id: WorkflowId::new(),
            topic: topic.into(),
        }
    }
}

/// Output from the simple workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleWorkflowOutput {
    /// Workflow ID that produced this output
    pub workflow_id: WorkflowId,

    /// Generated text from step 1
    pub generated_text: String,

    /// Critique of generated text from step 2
    pub critique: String,
}

/// Simple workflow service demonstrating core workflow logic.
///
/// Note: Full Restate integration deferred pending SDK 0.7 API verification.
/// This implementation demonstrates the workflow logic without Restate durability.
#[derive(Clone)]
pub struct SimpleWorkflowService {
    llm: Arc<dyn LlmProvider>,
}

impl SimpleWorkflowService {
    /// Creates a new simple workflow service.
    pub fn new(llm: Arc<dyn LlmProvider>) -> Self {
        Self { llm }
    }

    /// Runs the simple 2-step workflow.
    pub async fn run_simple(&self, input: SimpleWorkflowInput) -> EclResult<SimpleWorkflowOutput> {
        // Step 1: Generate content
        tracing::info!(
            workflow_id = %input.workflow_id,
            topic = %input.topic,
            "Starting workflow"
        );

        let generated_text = self.generate_step(&input.topic).await?;

        // Step 2: Critique the generated content
        let critique = self.critique_step(&generated_text).await?;

        tracing::info!(
            workflow_id = %input.workflow_id,
            "Workflow completed"
        );

        Ok(SimpleWorkflowOutput {
            workflow_id: input.workflow_id,
            generated_text,
            critique,
        })
    }

    /// Step 1: Generate content on a topic.
    async fn generate_step(&self, topic: &str) -> EclResult<String> {
        tracing::info!(topic = %topic, "Generating content");

        let request = CompletionRequest::new(vec![Message::user(format!(
            "Write a short paragraph about: {}",
            topic
        ))])
        .with_system_prompt("You are a helpful content generator. Write clear, concise paragraphs.")
        .with_max_tokens(500);

        let response = self.llm.complete(request).await?;

        tracing::info!(tokens = response.tokens_used.total(), "Content generated");

        Ok(response.content)
    }

    /// Step 2: Critique generated content.
    async fn critique_step(&self, content: &str) -> EclResult<String> {
        tracing::info!("Critiquing generated content");

        let request = CompletionRequest::new(vec![Message::user(format!(
            "Please provide constructive criticism of the following text:\n\n{}",
            content
        ))])
        .with_system_prompt(
            "You are a helpful writing critic. Provide specific, actionable feedback.",
        )
        .with_max_tokens(300);

        let response = self.llm.complete(request).await?;

        tracing::info!(tokens = response.tokens_used.total(), "Critique completed");

        Ok(response.content)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ecl_core::llm::MockLlmProvider;

    #[tokio::test]
    async fn test_simple_workflow_input_creation() {
        let input = SimpleWorkflowInput::new("Test topic");
        assert_eq!(input.topic, "Test topic");
    }

    #[tokio::test]
    async fn test_simple_workflow_execution() {
        let mock_llm = Arc::new(MockLlmProvider::new(vec![
            "Generated content about the topic.".to_string(),
            "This is constructive feedback.".to_string(),
        ]));

        let service = SimpleWorkflowService::new(mock_llm);
        let input = SimpleWorkflowInput::new("Rust programming");

        let output = service.run_simple(input.clone()).await.unwrap();

        assert_eq!(output.workflow_id, input.workflow_id);
        assert_eq!(output.generated_text, "Generated content about the topic.");
        assert_eq!(output.critique, "This is constructive feedback.");
    }
}
