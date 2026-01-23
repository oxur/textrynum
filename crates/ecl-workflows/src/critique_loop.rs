//! Critique-Revise workflow with bounded feedback loop.

use ecl_core::llm::{CompletionRequest, LlmProvider, Message};
use ecl_core::{CritiqueDecision, Error, Result, WorkflowId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Maximum number of revision attempts before giving up.
const MAX_REVISIONS: u32 = 3;

/// Input for critique-revise workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueLoopInput {
    /// Unique workflow ID
    pub workflow_id: WorkflowId,

    /// Topic to generate content about
    pub topic: String,

    /// Optional custom max revisions (defaults to MAX_REVISIONS)
    pub max_revisions: Option<u32>,
}

impl CritiqueLoopInput {
    /// Creates a new critique loop input.
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            workflow_id: WorkflowId::new(),
            topic: topic.into(),
            max_revisions: None,
        }
    }

    /// Sets a custom max revisions limit.
    pub fn with_max_revisions(mut self, max: u32) -> Self {
        self.max_revisions = Some(max);
        self
    }
}

/// Output from critique-revise workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueLoopOutput {
    /// Workflow ID
    pub workflow_id: WorkflowId,

    /// Final approved text
    pub final_text: String,

    /// Number of revision iterations performed
    pub revision_count: u32,

    /// All critiques generated during the workflow
    pub critiques: Vec<String>,
}

/// Workflow with critique and revision loop.
#[derive(Clone)]
pub struct CritiqueLoopWorkflow {
    llm: Arc<dyn LlmProvider>,
}

impl CritiqueLoopWorkflow {
    /// Creates a new critique loop workflow.
    pub fn new(llm: Arc<dyn LlmProvider>) -> Self {
        Self { llm }
    }

    /// Runs the critique-revise workflow with bounded iteration.
    pub async fn run(&self, input: CritiqueLoopInput) -> Result<CritiqueLoopOutput> {
        let max_revisions = input.max_revisions.unwrap_or(MAX_REVISIONS);

        tracing::info!(
            workflow_id = %input.workflow_id,
            topic = %input.topic,
            max_revisions = max_revisions,
            "Starting critique-revise workflow"
        );

        // Step 1: Generate initial draft
        let mut current_draft = self.generate_step(&input.topic).await?;

        let mut revision_count = 0u32;
        let mut critiques = Vec::new();

        // Revision loop with bounded iterations
        loop {
            // Step 2: Critique current draft
            let (critique_text, decision) =
                self.critique_step(&current_draft, revision_count).await?;

            critiques.push(critique_text.clone());

            match decision {
                CritiqueDecision::Pass => {
                    tracing::info!(
                        workflow_id = %input.workflow_id,
                        revision_count,
                        "Critique passed, workflow complete"
                    );
                    break;
                }
                CritiqueDecision::Revise { feedback } => {
                    if revision_count >= max_revisions {
                        tracing::warn!(
                            workflow_id = %input.workflow_id,
                            attempts = max_revisions,
                            "Maximum revisions exceeded"
                        );
                        return Err(Error::MaxRevisionsExceeded {
                            attempts: max_revisions,
                        });
                    }

                    tracing::info!(
                        workflow_id = %input.workflow_id,
                        revision_count,
                        feedback = %feedback,
                        "Revision requested"
                    );

                    // Step 3: Revise based on feedback
                    current_draft = self
                        .revise_step(&current_draft, &feedback, revision_count)
                        .await?;

                    revision_count += 1;
                }
                // Handle future variants (non_exhaustive)
                #[allow(unreachable_patterns)]
                _ => {
                    return Err(Error::validation("Unknown critique decision variant"));
                }
            }
        }

        tracing::info!(
            workflow_id = %input.workflow_id,
            revision_count,
            "Critique-revise workflow completed"
        );

        Ok(CritiqueLoopOutput {
            workflow_id: input.workflow_id,
            final_text: current_draft,
            revision_count,
            critiques,
        })
    }

    /// Generate initial content.
    async fn generate_step(&self, topic: &str) -> Result<String> {
        tracing::info!(topic = %topic, "Generating initial content");

        let request = CompletionRequest::new(vec![Message::user(format!(
            "Write a paragraph about: {}",
            topic
        ))])
        .with_system_prompt("You are a content generator. Write clear paragraphs.")
        .with_max_tokens(500);

        let response = self.llm.complete(request).await?;

        tracing::info!(tokens = response.tokens_used.total(), "Content generated");

        Ok(response.content)
    }

    /// Critique content and decide if revision is needed.
    async fn critique_step(
        &self,
        content: &str,
        attempt: u32,
    ) -> Result<(String, CritiqueDecision)> {
        tracing::info!(attempt, "Critiquing content");

        let request = CompletionRequest::new(vec![Message::user(format!(
            "Critique this text and decide if it needs revision.\n\
            Respond with JSON: {{\"decision\": \"pass\" or \"revise\", \"critique\": \"your critique\", \"feedback\": \"what to improve\"}}\n\n\
            Text:\n{}",
            content
        ))])
        .with_system_prompt("You are a writing critic. Be helpful but thorough.")
        .with_max_tokens(400);

        let response = self.llm.complete(request).await?;

        // Parse JSON response
        let parsed: serde_json::Value = serde_json::from_str(&response.content)
            .map_err(|e| Error::validation(format!("Failed to parse critique JSON: {}", e)))?;

        let critique = parsed["critique"]
            .as_str()
            .ok_or_else(|| Error::validation("Missing critique field"))?
            .to_string();

        let decision = match parsed["decision"].as_str() {
            Some("pass") => CritiqueDecision::Pass,
            Some("revise") => {
                let feedback = parsed["feedback"]
                    .as_str()
                    .ok_or_else(|| Error::validation("Missing feedback for revise decision"))?
                    .to_string();
                CritiqueDecision::Revise { feedback }
            }
            _ => return Err(Error::validation("Invalid decision value")),
        };

        tracing::info!(
            attempt,
            decision = ?decision,
            "Critique step completed"
        );

        Ok((critique, decision))
    }

    /// Revise content based on feedback.
    async fn revise_step(&self, original: &str, feedback: &str, attempt: u32) -> Result<String> {
        tracing::info!(attempt, "Revising content");

        let request = CompletionRequest::new(vec![Message::user(format!(
            "Revise this text based on the feedback:\n\n\
            Original:\n{}\n\n\
            Feedback:\n{}",
            original, feedback
        ))])
        .with_system_prompt("You are a content editor. Improve the text based on feedback.")
        .with_max_tokens(600);

        let response = self.llm.complete(request).await?;

        tracing::info!(
            attempt,
            tokens = response.tokens_used.total(),
            "Revision completed"
        );

        Ok(response.content)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ecl_core::llm::MockLlmProvider;

    #[tokio::test]
    async fn test_critique_loop_input_creation() {
        let input = CritiqueLoopInput::new("Test topic");
        assert_eq!(input.topic, "Test topic");
        assert_eq!(input.max_revisions, None);
    }

    #[tokio::test]
    async fn test_critique_loop_with_max_revisions() {
        let input = CritiqueLoopInput::new("Test").with_max_revisions(5);
        assert_eq!(input.max_revisions, Some(5));
    }

    #[tokio::test]
    async fn test_critique_loop_pass_immediately() {
        // Mock responses: generate, then critique that passes
        let mock_llm = Arc::new(MockLlmProvider::new(vec![
            "Generated content.".to_string(),
            r#"{"decision": "pass", "critique": "Looks good!"}"#.to_string(),
        ]));

        let workflow = CritiqueLoopWorkflow::new(mock_llm);
        let input = CritiqueLoopInput::new("Test topic");

        let output = workflow.run(input.clone()).await.unwrap();

        assert_eq!(output.workflow_id, input.workflow_id);
        assert_eq!(output.final_text, "Generated content.");
        assert_eq!(output.revision_count, 0);
        assert_eq!(output.critiques.len(), 1);
    }

    #[tokio::test]
    async fn test_critique_loop_with_one_revision() {
        // Mock responses: generate, critique (revise), revise, critique (pass)
        let mock_llm = Arc::new(MockLlmProvider::new(vec![
            "Initial draft.".to_string(),
            r#"{"decision": "revise", "critique": "Needs work", "feedback": "Add more detail"}"#
                .to_string(),
            "Improved draft with more detail.".to_string(),
            r#"{"decision": "pass", "critique": "Much better!"}"#.to_string(),
        ]));

        let workflow = CritiqueLoopWorkflow::new(mock_llm);
        let input = CritiqueLoopInput::new("Test topic");

        let output = workflow.run(input).await.unwrap();

        assert_eq!(output.final_text, "Improved draft with more detail.");
        assert_eq!(output.revision_count, 1);
        assert_eq!(output.critiques.len(), 2);
    }

    #[tokio::test]
    async fn test_critique_loop_max_revisions_exceeded() {
        // Mock responses that always request revision
        let mock_llm = Arc::new(MockLlmProvider::new(vec![
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
        ]));

        let workflow = CritiqueLoopWorkflow::new(mock_llm);
        let input = CritiqueLoopInput::new("Test topic");

        let result = workflow.run(input).await;

        assert!(result.is_err());
        let Error::MaxRevisionsExceeded { attempts } = result.unwrap_err() else {
            unreachable!("Expected MaxRevisionsExceeded error");
        };
        assert_eq!(attempts, MAX_REVISIONS);
    }

    #[tokio::test]
    async fn test_critique_loop_custom_max_revisions() {
        let mock_llm = Arc::new(MockLlmProvider::new(vec![
            "Draft.".to_string(),
            r#"{"decision": "revise", "critique": "Revise", "feedback": "Improve"}"#.to_string(),
            "Revised 1.".to_string(),
            r#"{"decision": "revise", "critique": "Again", "feedback": "More"}"#.to_string(),
        ]));

        let workflow = CritiqueLoopWorkflow::new(mock_llm);
        let input = CritiqueLoopInput::new("Test").with_max_revisions(1);

        let result = workflow.run(input).await;

        assert!(result.is_err());
        let Error::MaxRevisionsExceeded { attempts } = result.unwrap_err() else {
            unreachable!("Expected MaxRevisionsExceeded error with 1 attempt");
        };
        assert_eq!(attempts, 1);
    }
}
