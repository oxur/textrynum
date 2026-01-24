# Stage 2.3: Toy Workflow Step Implementations

**Goal:** Implement the three steps for the Critique-Revise toy workflow.

**Dependencies:** Stages 2.1-2.2 complete (Step trait and executor)

---

## Overview

This stage implements three concrete workflow steps that demonstrate the Step trait framework:

1. **GenerateStep** - Creates initial content from a topic
2. **CritiqueStep** - Analyzes content and decides pass/revise with structured JSON output
3. **ReviseStep** - Improves content based on critique feedback

These steps compose the "Critique-Revise Loop" toy workflow, exercising:
- Sequential step execution
- Bounded feedback loops (max 3 revisions)
- LLM interaction via StepContext
- Input/output validation
- Externalized prompt templates

---

## Detailed Implementation Steps

### Before You Begin

**Load these guides first:**

1. `assets/ai/ai-rust/guides/11-anti-patterns.md` - Critical patterns to avoid
2. `assets/ai/ai-rust/guides/03-error-handling.md` - Error handling best practices
3. `assets/ai/ai-rust/guides/06-traits.md` - Trait implementation patterns
4. `assets/ai/ai-rust/guides/07-concurrency-async.md` - Async best practices

**Key Requirements:**
- No `unwrap()` in library code
- Use `&str` not `&String` for parameters
- All errors use `thiserror`
- Input/Output types must be `Serialize + Deserialize`
- All public types use `#[non_exhaustive]`
- Test coverage ≥95%

---

### Step 1: Create Crate Structure

**Create the `ecl-steps` crate:**

```bash
cargo new --lib crates/ecl-steps
```

**Update `crates/ecl-steps/Cargo.toml`:**

```toml
[package]
name = "ecl-steps"
version = "0.1.0"
edition = "2021"

[dependencies]
ecl-core = { path = "../ecl-core" }
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
tracing = "0.1"

[dev-dependencies]
tokio = { version = "1", features = ["full", "test-util"] }
```

**Add to workspace `Cargo.toml`:**

```toml
[workspace]
members = [
    "crates/ecl-core",
    "crates/ecl-steps",
    # ... other crates
]
```

---

### Step 2: Define Step Input/Output Types

**Create `crates/ecl-steps/src/toy/types.rs`:**

```rust
//! Type definitions for toy workflow steps.

use serde::{Deserialize, Serialize};

/// Input for the Generate step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GenerateInput {
    /// Topic to generate content about
    pub topic: String,
}

impl GenerateInput {
    /// Creates a new generate input.
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
        }
    }
}

/// Output from the Generate step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GenerateOutput {
    /// Generated draft text
    pub draft: String,
}

impl GenerateOutput {
    /// Creates a new generate output.
    pub fn new(draft: impl Into<String>) -> Self {
        Self {
            draft: draft.into(),
        }
    }
}

/// Input for the Critique step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CritiqueInput {
    /// Draft text to critique
    pub draft: String,
}

impl CritiqueInput {
    /// Creates a new critique input.
    pub fn new(draft: impl Into<String>) -> Self {
        Self {
            draft: draft.into(),
        }
    }
}

/// Decision from critique analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CritiqueDecision {
    /// Content is acceptable, no revision needed
    Pass,
    /// Content needs revision
    Revise,
}

/// Output from the Critique step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CritiqueOutput {
    /// Critique text with specific feedback
    pub critique: String,

    /// Decision: pass or revise
    pub decision: CritiqueDecision,
}

impl CritiqueOutput {
    /// Creates a new critique output.
    pub fn new(critique: impl Into<String>, decision: CritiqueDecision) -> Self {
        Self {
            critique: critique.into(),
            decision,
        }
    }
}

/// Input for the Revise step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ReviseInput {
    /// Original draft text
    pub draft: String,

    /// Feedback from critique
    pub feedback: String,

    /// Current revision attempt number (1-indexed)
    pub attempt: u32,
}

impl ReviseInput {
    /// Creates a new revise input.
    pub fn new(draft: impl Into<String>, feedback: impl Into<String>, attempt: u32) -> Self {
        Self {
            draft: draft.into(),
            feedback: feedback.into(),
            attempt,
        }
    }
}

/// Output from the Revise step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ReviseOutput {
    /// Revised draft text
    pub revised_draft: String,
}

impl ReviseOutput {
    /// Creates a new revise output.
    pub fn new(revised_draft: impl Into<String>) -> Self {
        Self {
            revised_draft: revised_draft.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_input_creation() {
        let input = GenerateInput::new("test topic");
        assert_eq!(input.topic, "test topic");
    }

    #[test]
    fn test_generate_output_creation() {
        let output = GenerateOutput::new("test draft");
        assert_eq!(output.draft, "test draft");
    }

    #[test]
    fn test_critique_input_creation() {
        let input = CritiqueInput::new("test draft");
        assert_eq!(input.draft, "test draft");
    }

    #[test]
    fn test_critique_output_creation() {
        let output = CritiqueOutput::new("good work", CritiqueDecision::Pass);
        assert_eq!(output.critique, "good work");
        assert_eq!(output.decision, CritiqueDecision::Pass);
    }

    #[test]
    fn test_critique_decision_serialization() {
        let pass = CritiqueDecision::Pass;
        let json = serde_json::to_string(&pass).unwrap();
        assert_eq!(json, r#""pass""#);

        let revise = CritiqueDecision::Revise;
        let json = serde_json::to_string(&revise).unwrap();
        assert_eq!(json, r#""revise""#);
    }

    #[test]
    fn test_revise_input_creation() {
        let input = ReviseInput::new("draft", "feedback", 1);
        assert_eq!(input.draft, "draft");
        assert_eq!(input.feedback, "feedback");
        assert_eq!(input.attempt, 1);
    }

    #[test]
    fn test_revise_output_creation() {
        let output = ReviseOutput::new("revised");
        assert_eq!(output.revised_draft, "revised");
    }

    #[test]
    fn test_all_types_serialize() {
        // Test that all types can be serialized/deserialized
        let gen_in = GenerateInput::new("topic");
        let json = serde_json::to_string(&gen_in).unwrap();
        let _: GenerateInput = serde_json::from_str(&json).unwrap();

        let gen_out = GenerateOutput::new("draft");
        let json = serde_json::to_string(&gen_out).unwrap();
        let _: GenerateOutput = serde_json::from_str(&json).unwrap();

        let crit_in = CritiqueInput::new("draft");
        let json = serde_json::to_string(&crit_in).unwrap();
        let _: CritiqueInput = serde_json::from_str(&json).unwrap();

        let crit_out = CritiqueOutput::new("critique", CritiqueDecision::Revise);
        let json = serde_json::to_string(&crit_out).unwrap();
        let _: CritiqueOutput = serde_json::from_str(&json).unwrap();

        let rev_in = ReviseInput::new("draft", "feedback", 1);
        let json = serde_json::to_string(&rev_in).unwrap();
        let _: ReviseInput = serde_json::from_str(&json).unwrap();

        let rev_out = ReviseOutput::new("revised");
        let json = serde_json::to_string(&rev_out).unwrap();
        let _: ReviseOutput = serde_json::from_str(&json).unwrap();
    }
}
```

---

### Step 3: Create Prompt Templates

Create external prompt files for maintainability and easy editing.

**Create `crates/ecl-steps/src/prompts/generate.txt`:**

```text
You are a creative content generator. Your task is to create clear, informative content on the given topic.

Requirements:
- Write in a clear, engaging style
- Include relevant details and examples
- Keep the content focused on the topic
- Aim for 200-500 words
- Use proper grammar and formatting

Topic: {topic}

Generate content about this topic. Be informative and engaging.
```

**Create `crates/ecl-steps/src/prompts/critique.txt`:**

```text
You are a critical content reviewer. Your task is to analyze the provided draft and decide whether it needs revision.

Evaluation criteria:
- Clarity and coherence
- Accuracy and completeness
- Grammar and style
- Relevance to topic
- Overall quality

Draft to review:
{draft}

Provide your analysis in JSON format:
{{
  "critique": "Your detailed critique here, explaining strengths and weaknesses",
  "decision": "pass" or "revise"
}}

Decision rules:
- Use "pass" if the content meets all criteria and is ready for publication
- Use "revise" if significant improvements are needed

Respond ONLY with valid JSON, no other text.
```

**Create `crates/ecl-steps/src/prompts/revise.txt`:**

```text
You are a content editor. Your task is to revise the draft based on the provided feedback.

Original draft:
{draft}

Feedback from review:
{feedback}

Revision attempt: {attempt} of 3

Instructions:
- Address all points raised in the feedback
- Improve clarity, accuracy, and style
- Keep the core message and structure where appropriate
- Ensure the revision is substantively different from the original
- Maintain proper grammar and formatting

Provide the revised content. Do not include meta-commentary, just the improved content.
```

---

### Step 4: Implement GenerateStep

**Create `crates/ecl-steps/src/toy/generate.rs`:**

```rust
//! Generate step implementation.

use async_trait::async_trait;
use ecl_core::{
    llm::LlmRequest,
    step::{Step, StepContext, ValidationError},
    StepId, StepResult,
};
use tracing::instrument;

use super::types::{GenerateInput, GenerateOutput};

/// Step that generates initial content from a topic.
///
/// Takes a topic string and uses an LLM to generate draft content.
/// Validates that output is non-empty and under 2000 characters.
#[derive(Debug)]
pub struct GenerateStep {
    id: StepId,
    prompt_template: String,
}

impl GenerateStep {
    /// Creates a new generate step.
    pub fn new() -> Self {
        Self {
            id: StepId::new("generate"),
            prompt_template: include_str!("../prompts/generate.txt").to_string(),
        }
    }

    /// Creates a generate step with a custom prompt template.
    pub fn with_template(template: impl Into<String>) -> Self {
        Self {
            id: StepId::new("generate"),
            prompt_template: template.into(),
        }
    }

    /// Formats the prompt with the given topic.
    fn format_prompt(&self, topic: &str) -> String {
        self.prompt_template.replace("{topic}", topic)
    }
}

impl Default for GenerateStep {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Step for GenerateStep {
    type Input = GenerateInput;
    type Output = GenerateOutput;

    fn id(&self) -> &StepId {
        &self.id
    }

    fn name(&self) -> &str {
        "Generate"
    }

    #[instrument(skip(self, ctx), fields(topic = %input.topic))]
    async fn execute(
        &self,
        ctx: &StepContext,
        input: Self::Input,
    ) -> StepResult<Self::Output> {
        tracing::debug!("Generating content for topic: {}", input.topic);

        // Format the prompt
        let prompt = self.format_prompt(&input.topic);

        // Make LLM request
        let request = LlmRequest::new(prompt);
        let response = match ctx.llm().complete(request).await {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!(error = %e, "LLM request failed");
                return StepResult::Failed {
                    error: format!("LLM request failed: {}", e),
                    retryable: e.is_retryable(),
                };
            }
        };

        let draft = response.content().trim().to_string();

        tracing::info!(
            draft_length = draft.len(),
            "Generated draft content"
        );

        StepResult::Success(GenerateOutput { draft })
    }

    async fn validate_input(&self, input: &Self::Input) -> Result<(), ValidationError> {
        // Topic must not be empty
        if input.topic.trim().is_empty() {
            return Err(ValidationError::for_field(
                "topic",
                "Topic cannot be empty"
            ));
        }

        // Topic should be reasonable length
        if input.topic.len() > 500 {
            return Err(ValidationError::for_field(
                "topic",
                "Topic is too long (max 500 characters)"
            ));
        }

        Ok(())
    }

    async fn validate_output(&self, output: &Self::Output) -> Result<(), ValidationError> {
        // Draft must not be empty
        if output.draft.trim().is_empty() {
            return Err(ValidationError::for_field(
                "draft",
                "Generated draft is empty"
            ));
        }

        // Draft must be under 2000 characters
        if output.draft.len() > 2000 {
            return Err(ValidationError::for_field(
                "draft",
                format!("Draft too long: {} chars (max 2000)", output.draft.len())
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecl_core::{llm::MockLlmProvider, WorkflowId};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_generate_step_success() {
        let llm = Arc::new(MockLlmProvider::with_response(
            "This is a generated draft about the topic."
        ));
        let step = GenerateStep::new();
        let ctx = StepContext::new(
            llm,
            WorkflowId::new(),
            step.id().clone(),
            1,
        );

        let input = GenerateInput::new("test topic");
        let result = step.execute(&ctx, input).await;

        match result {
            StepResult::Success(output) => {
                assert!(!output.draft.is_empty());
            }
            _ => panic!("Expected success"),
        }
    }

    #[tokio::test]
    async fn test_generate_step_validates_empty_topic() {
        let step = GenerateStep::new();
        let input = GenerateInput::new("");

        let result = step.validate_input(&input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_step_validates_long_topic() {
        let step = GenerateStep::new();
        let input = GenerateInput::new("a".repeat(501));

        let result = step.validate_input(&input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_step_validates_empty_output() {
        let step = GenerateStep::new();
        let output = GenerateOutput::new("");

        let result = step.validate_output(&output).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_step_validates_long_output() {
        let step = GenerateStep::new();
        let output = GenerateOutput::new("a".repeat(2001));

        let result = step.validate_output(&output).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_step_format_prompt() {
        let step = GenerateStep::new();
        let prompt = step.format_prompt("Rust programming");

        assert!(prompt.contains("Rust programming"));
        assert!(!prompt.contains("{topic}"));
    }

    #[test]
    fn test_generate_step_with_custom_template() {
        let custom_template = "Custom prompt: {topic}";
        let step = GenerateStep::with_template(custom_template);
        let prompt = step.format_prompt("test");

        assert_eq!(prompt, "Custom prompt: test");
    }
}
```

---

### Step 5: Implement CritiqueStep

**Create `crates/ecl-steps/src/toy/critique.rs`:**

```rust
//! Critique step implementation.

use async_trait::async_trait;
use ecl_core::{
    llm::LlmRequest,
    step::{Step, StepContext, ValidationError},
    StepId, StepResult,
};
use tracing::instrument;

use super::types::{CritiqueDecision, CritiqueInput, CritiqueOutput};

/// Step that critiques draft content and decides pass/revise.
///
/// Analyzes draft text and returns structured feedback with a decision.
/// Output is JSON-formatted with critique text and a decision field.
#[derive(Debug)]
pub struct CritiqueStep {
    id: StepId,
    prompt_template: String,
}

impl CritiqueStep {
    /// Creates a new critique step.
    pub fn new() -> Self {
        Self {
            id: StepId::new("critique"),
            prompt_template: include_str!("../prompts/critique.txt").to_string(),
        }
    }

    /// Creates a critique step with a custom prompt template.
    pub fn with_template(template: impl Into<String>) -> Self {
        Self {
            id: StepId::new("critique"),
            prompt_template: template.into(),
        }
    }

    /// Formats the prompt with the given draft.
    fn format_prompt(&self, draft: &str) -> String {
        self.prompt_template.replace("{draft}", draft)
    }

    /// Parses LLM response as JSON critique output.
    fn parse_response(&self, response: &str) -> Result<CritiqueOutput, String> {
        let trimmed = response.trim();

        // Try to parse as JSON
        let parsed: serde_json::Value = serde_json::from_str(trimmed)
            .map_err(|e| format!("Failed to parse JSON response: {}", e))?;

        // Extract critique field
        let critique = parsed
            .get("critique")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'critique' field")?
            .to_string();

        // Extract decision field
        let decision_str = parsed
            .get("decision")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'decision' field")?;

        let decision = match decision_str {
            "pass" => CritiqueDecision::Pass,
            "revise" => CritiqueDecision::Revise,
            other => return Err(format!("Invalid decision value: {}", other)),
        };

        Ok(CritiqueOutput { critique, decision })
    }
}

impl Default for CritiqueStep {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Step for CritiqueStep {
    type Input = CritiqueInput;
    type Output = CritiqueOutput;

    fn id(&self) -> &StepId {
        &self.id
    }

    fn name(&self) -> &str {
        "Critique"
    }

    #[instrument(skip(self, ctx, input), fields(draft_length = input.draft.len()))]
    async fn execute(
        &self,
        ctx: &StepContext,
        input: Self::Input,
    ) -> StepResult<Self::Output> {
        tracing::debug!("Critiquing draft (length: {})", input.draft.len());

        // Format the prompt
        let prompt = self.format_prompt(&input.draft);

        // Make LLM request
        let request = LlmRequest::new(prompt);
        let response = match ctx.llm().complete(request).await {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!(error = %e, "LLM request failed");
                return StepResult::Failed {
                    error: format!("LLM request failed: {}", e),
                    retryable: e.is_retryable(),
                };
            }
        };

        // Parse the JSON response
        let output = match self.parse_response(response.content()) {
            Ok(output) => output,
            Err(e) => {
                tracing::error!(error = %e, "Failed to parse critique response");
                return StepResult::Failed {
                    error: format!("Failed to parse response: {}", e),
                    retryable: true, // LLM might do better on retry
                };
            }
        };

        tracing::info!(
            decision = ?output.decision,
            critique_length = output.critique.len(),
            "Critique completed"
        );

        // If decision is "revise", signal revision needed
        if output.decision == CritiqueDecision::Revise {
            StepResult::NeedsRevision {
                output: output.clone(),
                feedback: output.critique.clone(),
            }
        } else {
            StepResult::Success(output)
        }
    }

    async fn validate_input(&self, input: &Self::Input) -> Result<(), ValidationError> {
        // Draft must not be empty
        if input.draft.trim().is_empty() {
            return Err(ValidationError::for_field(
                "draft",
                "Draft cannot be empty"
            ));
        }

        Ok(())
    }

    async fn validate_output(&self, output: &Self::Output) -> Result<(), ValidationError> {
        // Critique must not be empty
        if output.critique.trim().is_empty() {
            return Err(ValidationError::for_field(
                "critique",
                "Critique cannot be empty"
            ));
        }

        // Decision must be present (always true for enum, but good for documentation)
        // This validates that the decision was properly parsed
        match output.decision {
            CritiqueDecision::Pass | CritiqueDecision::Revise => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecl_core::{llm::MockLlmProvider, WorkflowId};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_critique_step_pass_decision() {
        let json_response = r#"{"critique": "Good work!", "decision": "pass"}"#;
        let llm = Arc::new(MockLlmProvider::with_response(json_response));
        let step = CritiqueStep::new();
        let ctx = StepContext::new(
            llm,
            WorkflowId::new(),
            step.id().clone(),
            1,
        );

        let input = CritiqueInput::new("Test draft content");
        let result = step.execute(&ctx, input).await;

        match result {
            StepResult::Success(output) => {
                assert_eq!(output.decision, CritiqueDecision::Pass);
                assert_eq!(output.critique, "Good work!");
            }
            _ => panic!("Expected success"),
        }
    }

    #[tokio::test]
    async fn test_critique_step_revise_decision() {
        let json_response = r#"{"critique": "Needs improvement", "decision": "revise"}"#;
        let llm = Arc::new(MockLlmProvider::with_response(json_response));
        let step = CritiqueStep::new();
        let ctx = StepContext::new(
            llm,
            WorkflowId::new(),
            step.id().clone(),
            1,
        );

        let input = CritiqueInput::new("Test draft content");
        let result = step.execute(&ctx, input).await;

        match result {
            StepResult::NeedsRevision { output, feedback } => {
                assert_eq!(output.decision, CritiqueDecision::Revise);
                assert_eq!(output.critique, "Needs improvement");
                assert_eq!(feedback, "Needs improvement");
            }
            _ => panic!("Expected needs revision"),
        }
    }

    #[tokio::test]
    async fn test_critique_step_parse_valid_json() {
        let step = CritiqueStep::new();
        let json = r#"{"critique": "Test critique", "decision": "pass"}"#;

        let output = step.parse_response(json).unwrap();
        assert_eq!(output.critique, "Test critique");
        assert_eq!(output.decision, CritiqueDecision::Pass);
    }

    #[tokio::test]
    async fn test_critique_step_parse_invalid_json() {
        let step = CritiqueStep::new();
        let invalid = "not json";

        let result = step.parse_response(invalid);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_critique_step_parse_missing_fields() {
        let step = CritiqueStep::new();
        let json = r#"{"critique": "Test"}"#; // Missing decision

        let result = step.parse_response(json);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_critique_step_parse_invalid_decision() {
        let step = CritiqueStep::new();
        let json = r#"{"critique": "Test", "decision": "invalid"}"#;

        let result = step.parse_response(json);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_critique_step_validates_empty_input() {
        let step = CritiqueStep::new();
        let input = CritiqueInput::new("");

        let result = step.validate_input(&input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_critique_step_validates_empty_critique() {
        let step = CritiqueStep::new();
        let output = CritiqueOutput::new("", CritiqueDecision::Pass);

        let result = step.validate_output(&output).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_critique_step_format_prompt() {
        let step = CritiqueStep::new();
        let prompt = step.format_prompt("Draft content here");

        assert!(prompt.contains("Draft content here"));
        assert!(!prompt.contains("{draft}"));
    }
}
```

---

### Step 6: Implement ReviseStep

**Create `crates/ecl-steps/src/toy/revise.rs`:**

```rust
//! Revise step implementation.

use async_trait::async_trait;
use ecl_core::{
    llm::LlmRequest,
    step::{Step, StepContext, ValidationError},
    StepId, StepResult,
};
use tracing::instrument;

use super::types::{ReviseInput, ReviseOutput};

/// Step that revises content based on critique feedback.
///
/// Takes the original draft, critique feedback, and attempt number,
/// then generates an improved version addressing the feedback.
#[derive(Debug)]
pub struct ReviseStep {
    id: StepId,
    prompt_template: String,
}

impl ReviseStep {
    /// Creates a new revise step.
    pub fn new() -> Self {
        Self {
            id: StepId::new("revise"),
            prompt_template: include_str!("../prompts/revise.txt").to_string(),
        }
    }

    /// Creates a revise step with a custom prompt template.
    pub fn with_template(template: impl Into<String>) -> Self {
        Self {
            id: StepId::new("revise"),
            prompt_template: template.into(),
        }
    }

    /// Formats the prompt with the given draft, feedback, and attempt.
    fn format_prompt(&self, draft: &str, feedback: &str, attempt: u32) -> String {
        self.prompt_template
            .replace("{draft}", draft)
            .replace("{feedback}", feedback)
            .replace("{attempt}", &attempt.to_string())
    }
}

impl Default for ReviseStep {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Step for ReviseStep {
    type Input = ReviseInput;
    type Output = ReviseOutput;

    fn id(&self) -> &StepId {
        &self.id
    }

    fn name(&self) -> &str {
        "Revise"
    }

    fn max_revisions(&self) -> u32 {
        3
    }

    #[instrument(
        skip(self, ctx, input),
        fields(
            draft_length = input.draft.len(),
            feedback_length = input.feedback.len(),
            attempt = input.attempt
        )
    )]
    async fn execute(
        &self,
        ctx: &StepContext,
        input: Self::Input,
    ) -> StepResult<Self::Output> {
        tracing::debug!(
            "Revising draft (attempt {}/{})",
            input.attempt,
            self.max_revisions()
        );

        // Format the prompt
        let prompt = self.format_prompt(&input.draft, &input.feedback, input.attempt);

        // Make LLM request
        let request = LlmRequest::new(prompt);
        let response = match ctx.llm().complete(request).await {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!(error = %e, "LLM request failed");
                return StepResult::Failed {
                    error: format!("LLM request failed: {}", e),
                    retryable: e.is_retryable(),
                };
            }
        };

        let revised_draft = response.content().trim().to_string();

        tracing::info!(
            revised_length = revised_draft.len(),
            original_length = input.draft.len(),
            "Draft revised"
        );

        StepResult::Success(ReviseOutput { revised_draft })
    }

    async fn validate_input(&self, input: &Self::Input) -> Result<(), ValidationError> {
        // Draft must not be empty
        if input.draft.trim().is_empty() {
            return Err(ValidationError::for_field(
                "draft",
                "Draft cannot be empty"
            ));
        }

        // Feedback must not be empty
        if input.feedback.trim().is_empty() {
            return Err(ValidationError::for_field(
                "feedback",
                "Feedback cannot be empty"
            ));
        }

        // Attempt must be positive
        if input.attempt == 0 {
            return Err(ValidationError::for_field(
                "attempt",
                "Attempt number must be positive (1-indexed)"
            ));
        }

        // Attempt should not exceed max revisions
        if input.attempt > self.max_revisions() {
            return Err(ValidationError::for_field(
                "attempt",
                format!(
                    "Attempt {} exceeds max revisions {}",
                    input.attempt,
                    self.max_revisions()
                )
            ));
        }

        Ok(())
    }

    async fn validate_output(&self, output: &Self::Output) -> Result<(), ValidationError> {
        // Revised draft must not be empty
        if output.revised_draft.trim().is_empty() {
            return Err(ValidationError::for_field(
                "revised_draft",
                "Revised draft is empty"
            ));
        }

        // Revised draft must be under 2000 characters (same limit as generate)
        if output.revised_draft.len() > 2000 {
            return Err(ValidationError::for_field(
                "revised_draft",
                format!(
                    "Revised draft too long: {} chars (max 2000)",
                    output.revised_draft.len()
                )
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecl_core::{llm::MockLlmProvider, WorkflowId};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_revise_step_success() {
        let llm = Arc::new(MockLlmProvider::with_response(
            "This is a revised and improved draft."
        ));
        let step = ReviseStep::new();
        let ctx = StepContext::new(
            llm,
            WorkflowId::new(),
            step.id().clone(),
            1,
        );

        let input = ReviseInput::new(
            "Original draft",
            "Please improve clarity",
            1,
        );
        let result = step.execute(&ctx, input).await;

        match result {
            StepResult::Success(output) => {
                assert!(!output.revised_draft.is_empty());
            }
            _ => panic!("Expected success"),
        }
    }

    #[tokio::test]
    async fn test_revise_step_validates_empty_draft() {
        let step = ReviseStep::new();
        let input = ReviseInput::new("", "feedback", 1);

        let result = step.validate_input(&input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_revise_step_validates_empty_feedback() {
        let step = ReviseStep::new();
        let input = ReviseInput::new("draft", "", 1);

        let result = step.validate_input(&input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_revise_step_validates_zero_attempt() {
        let step = ReviseStep::new();
        let input = ReviseInput::new("draft", "feedback", 0);

        let result = step.validate_input(&input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_revise_step_validates_max_attempts() {
        let step = ReviseStep::new();
        let input = ReviseInput::new("draft", "feedback", 4); // Max is 3

        let result = step.validate_input(&input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_revise_step_validates_empty_output() {
        let step = ReviseStep::new();
        let output = ReviseOutput::new("");

        let result = step.validate_output(&output).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_revise_step_validates_long_output() {
        let step = ReviseStep::new();
        let output = ReviseOutput::new("a".repeat(2001));

        let result = step.validate_output(&output).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_revise_step_format_prompt() {
        let step = ReviseStep::new();
        let prompt = step.format_prompt(
            "Original draft",
            "Improve this",
            2,
        );

        assert!(prompt.contains("Original draft"));
        assert!(prompt.contains("Improve this"));
        assert!(prompt.contains("2"));
        assert!(!prompt.contains("{draft}"));
        assert!(!prompt.contains("{feedback}"));
        assert!(!prompt.contains("{attempt}"));
    }

    #[test]
    fn test_revise_step_max_revisions() {
        let step = ReviseStep::new();
        assert_eq!(step.max_revisions(), 3);
    }

    #[test]
    fn test_revise_step_with_custom_template() {
        let custom_template = "Revise {draft} with {feedback} (attempt {attempt})";
        let step = ReviseStep::with_template(custom_template);
        let prompt = step.format_prompt("test draft", "test feedback", 1);

        assert_eq!(prompt, "Revise test draft with test feedback (attempt 1)");
    }
}
```

---

### Step 7: Module Organization and Registration

**Create `crates/ecl-steps/src/toy/mod.rs`:**

```rust
//! Toy workflow step implementations.
//!
//! This module contains the three steps that compose the Critique-Revise
//! toy workflow used for validating the ECL framework.

mod types;
mod generate;
mod critique;
mod revise;

pub use types::*;
pub use generate::GenerateStep;
pub use critique::CritiqueStep;
pub use revise::ReviseStep;

use ecl_core::{step::StepRegistry, Result};

/// Registers all toy workflow steps in the given registry.
///
/// This is a convenience function for registering all three steps
/// (Generate, Critique, Revise) at once.
///
/// # Errors
///
/// Returns an error if any step fails to register (e.g., duplicate IDs).
///
/// # Examples
///
/// ```rust,no_run
/// use ecl_core::step::StepRegistry;
/// use ecl_steps::toy;
///
/// let mut registry = StepRegistry::new();
/// toy::register_steps(&mut registry).expect("Failed to register steps");
/// ```
pub fn register_steps(registry: &mut StepRegistry) -> Result<()> {
    registry.register(GenerateStep::new())?;
    registry.register(CritiqueStep::new())?;
    registry.register(ReviseStep::new())?;

    tracing::info!("Registered 3 toy workflow steps");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_steps() {
        let mut registry = StepRegistry::new();
        let result = register_steps(&mut registry);

        assert!(result.is_ok());
        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn test_register_steps_has_all_step_ids() {
        let mut registry = StepRegistry::new();
        register_steps(&mut registry).unwrap();

        let ids = registry.list_ids();
        let id_strs: Vec<String> = ids.iter().map(|id| id.to_string()).collect();

        assert!(id_strs.contains(&"generate".to_string()));
        assert!(id_strs.contains(&"critique".to_string()));
        assert!(id_strs.contains(&"revise".to_string()));
    }
}
```

**Create `crates/ecl-steps/src/lib.rs`:**

```rust
//! ECL workflow step implementations.
//!
//! This crate provides concrete implementations of workflow steps
//! that use the ECL core step framework.

pub mod toy;

// Re-export for convenience
pub use toy::{
    GenerateStep, CritiqueStep, ReviseStep,
    GenerateInput, GenerateOutput,
    CritiqueInput, CritiqueOutput, CritiqueDecision,
    ReviseInput, ReviseOutput,
};
```

---

## Testing Strategy

**Test coverage goal:** ≥95%

### Unit Tests

Each step implementation includes comprehensive unit tests:

**Type Tests** (`types.rs`):
- Constructor functions
- Serialization/deserialization
- Field access
- All type combinations

**Step Implementation Tests**:

1. **GenerateStep**:
   - Successful content generation
   - Input validation (empty topic, too long)
   - Output validation (empty draft, too long)
   - Prompt formatting
   - Custom template support

2. **CritiqueStep**:
   - Pass decision returns Success
   - Revise decision returns NeedsRevision
   - JSON parsing (valid, invalid, missing fields)
   - Invalid decision values
   - Input validation (empty draft)
   - Output validation (empty critique)
   - Prompt formatting

3. **ReviseStep**:
   - Successful revision
   - Input validation (empty draft, empty feedback, zero attempt, exceeds max)
   - Output validation (empty output, too long)
   - Prompt formatting with all placeholders
   - Max revisions limit
   - Custom template support

**Module Tests** (`mod.rs`):
- Step registration
- All steps registered correctly
- Correct step IDs present

### Integration Tests

**Create `crates/ecl-steps/tests/integration.rs`:**

```rust
//! Integration tests for toy workflow steps.

use ecl_core::{
    llm::MockLlmProvider,
    step::{StepContext, StepExecutor, StepRegistry},
    WorkflowId, StepResult,
};
use ecl_steps::toy::{self, *};
use std::sync::Arc;

#[tokio::test]
async fn test_end_to_end_workflow_pass() {
    // Setup
    let llm = Arc::new(MockLlmProvider::new());
    let mut registry = StepRegistry::new();
    toy::register_steps(&mut registry).unwrap();

    // Configure mock responses
    llm.add_response("Generated draft content about Rust programming.");
    llm.add_response(r#"{"critique": "Excellent work!", "decision": "pass"}"#);

    let executor = StepExecutor::new(llm.clone(), Arc::new(registry));
    let workflow_id = WorkflowId::new();

    // Step 1: Generate
    let generate_step = GenerateStep::new();
    let gen_input = GenerateInput::new("Rust programming");
    let gen_execution = executor
        .execute_step(&generate_step, gen_input, &workflow_id)
        .await
        .unwrap();
    let draft = gen_execution.output.draft;

    // Step 2: Critique (pass)
    let critique_step = CritiqueStep::new();
    let crit_input = CritiqueInput::new(draft);
    let crit_execution = executor
        .execute_step(&critique_step, crit_input, &workflow_id)
        .await
        .unwrap();

    assert_eq!(crit_execution.output.decision, CritiqueDecision::Pass);
}

#[tokio::test]
async fn test_end_to_end_workflow_with_revision() {
    // Setup
    let llm = Arc::new(MockLlmProvider::new());
    let mut registry = StepRegistry::new();
    toy::register_steps(&mut registry).unwrap();

    // Configure mock responses
    llm.add_response("Initial draft content.");
    llm.add_response(r#"{"critique": "Needs more detail", "decision": "revise"}"#);
    llm.add_response("Revised draft with more detail.");
    llm.add_response(r#"{"critique": "Much better!", "decision": "pass"}"#);

    let executor = StepExecutor::new(llm.clone(), Arc::new(registry));
    let workflow_id = WorkflowId::new();

    // Step 1: Generate
    let generate_step = GenerateStep::new();
    let gen_execution = executor
        .execute_step(&generate_step, GenerateInput::new("Test"), &workflow_id)
        .await
        .unwrap();
    let mut draft = gen_execution.output.draft;

    // Step 2: Critique (revise)
    let critique_step = CritiqueStep::new();
    let crit_result = critique_step
        .execute(
            &StepContext::new(llm.clone(), workflow_id.clone(), critique_step.id().clone(), 1),
            CritiqueInput::new(draft.clone()),
        )
        .await;

    let feedback = match crit_result {
        StepResult::NeedsRevision { feedback, .. } => feedback,
        _ => panic!("Expected needs revision"),
    };

    // Step 3: Revise
    let revise_step = ReviseStep::new();
    let rev_execution = executor
        .execute_step(
            &revise_step,
            ReviseInput::new(draft.clone(), feedback, 1),
            &workflow_id,
        )
        .await
        .unwrap();
    draft = rev_execution.output.revised_draft;

    // Step 4: Critique again (pass)
    let crit_execution = executor
        .execute_step(&critique_step, CritiqueInput::new(draft), &workflow_id)
        .await
        .unwrap();

    assert_eq!(crit_execution.output.decision, CritiqueDecision::Pass);
}

#[tokio::test]
async fn test_max_revision_limit() {
    let revise_step = ReviseStep::new();

    // Attempt 1-3 should be valid
    for attempt in 1..=3 {
        let input = ReviseInput::new("draft", "feedback", attempt);
        assert!(revise_step.validate_input(&input).await.is_ok());
    }

    // Attempt 4 should fail validation
    let input = ReviseInput::new("draft", "feedback", 4);
    assert!(revise_step.validate_input(&input).await.is_err());
}
```

### Property-Based Tests

**Add to `Cargo.toml`:**

```toml
[dev-dependencies]
proptest = "1.0"
```

**Create `crates/ecl-steps/tests/property.rs`:**

```rust
use proptest::prelude::*;
use ecl_steps::toy::*;

proptest! {
    #[test]
    fn test_generate_input_roundtrip(topic in "\\PC{1,500}") {
        let input = GenerateInput::new(topic.clone());
        let json = serde_json::to_string(&input).unwrap();
        let decoded: GenerateInput = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.topic, topic);
    }

    #[test]
    fn test_critique_decision_roundtrip(decision in prop::bool::ANY) {
        let decision = if decision {
            CritiqueDecision::Pass
        } else {
            CritiqueDecision::Revise
        };
        let json = serde_json::to_string(&decision).unwrap();
        let decoded: CritiqueDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, decision);
    }

    #[test]
    fn test_revise_input_attempt_validation(
        draft in "\\PC{1,100}",
        feedback in "\\PC{1,100}",
        attempt in 1u32..=3
    ) {
        let step = ReviseStep::new();
        let input = ReviseInput::new(draft, feedback, attempt);
        // Valid attempts should pass
        assert!(step.validate_input(&input).await.is_ok());
    }
}
```

### Coverage Requirements

Each module must achieve:
- **Unit test coverage:** ≥95% line coverage
- **Branch coverage:** ≥90% for validation logic
- **Error paths:** All error cases tested
- **Edge cases:** Empty strings, boundaries, max values

Run coverage checks:

```bash
just coverage
```

---

## Potential Blockers

### 1. LLM Response Format Variability

**Risk:** Claude may not always return perfectly formatted JSON in CritiqueStep.

**Mitigation:**
- Clear prompt instructions requesting ONLY JSON
- Robust parsing with detailed error messages
- Retry on parse failures (retryable error)
- Consider response prefill in future iterations
- Test with various mock responses

### 2. Prompt Template Maintenance

**Risk:** Prompts in external files may become out of sync with code.

**Mitigation:**
- Use `include_str!()` for compile-time inclusion
- Document expected placeholders in code comments
- Add tests that verify prompt formatting
- Keep prompts in version control alongside code

### 3. Validation Strictness

**Risk:** Too strict validation may reject valid content; too loose may allow garbage.

**Mitigation:**
- Start with reasonable limits (2000 chars)
- Monitor validation failures in logs
- Make limits configurable in future iterations
- Document validation rationale

### 4. Step Registration Errors

**Risk:** Duplicate step IDs or registration failures.

**Mitigation:**
- Use unique, descriptive step IDs
- Test registration in unit tests
- Return clear error messages on duplicate registration
- Document step ID conventions

### 5. Async/Await Complexity

**Risk:** Incorrect async usage or blocking operations.

**Mitigation:**
- Follow `async-trait` patterns consistently
- Never use blocking I/O in async functions
- Review async guide before implementation
- Test thoroughly with tokio test runtime

---

## Acceptance Criteria

### Functionality

- [ ] GenerateStep produces coherent output via Claude
- [ ] CritiqueStep parses JSON and returns correct decision
- [ ] ReviseStep improves content based on feedback
- [ ] All steps validate inputs before execution
- [ ] All steps validate outputs after execution
- [ ] Steps handle LLM errors gracefully
- [ ] Steps can be tested with `MockLlmProvider`

### Code Quality

- [ ] All code follows Rust anti-patterns guide
- [ ] No `unwrap()` in library code
- [ ] All errors use `thiserror`
- [ ] Parameters use `&str` not `&String`
- [ ] Public types use `#[non_exhaustive]`
- [ ] All async code uses async I/O
- [ ] No clippy warnings

### Documentation

- [ ] All public items have rustdoc comments
- [ ] Examples in docs compile
- [ ] Module-level documentation complete
- [ ] Prompt templates documented

### Testing

- [ ] Test coverage ≥95%
- [ ] All unit tests pass
- [ ] Integration tests pass
- [ ] Property tests pass
- [ ] Error paths covered
- [ ] Edge cases tested

### Integration

- [ ] Prompts are externalized in separate files
- [ ] Steps can be registered in `StepRegistry`
- [ ] Steps work with `StepExecutor`
- [ ] Tracing integration works
- [ ] Metadata recording works

### Build System

- [ ] `cargo build --workspace` succeeds
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] `just test` passes
- [ ] `just coverage` shows ≥95%

---

## Estimated Effort

**Total Time:** 5-6 hours

### Breakdown

| Task | Time | Notes |
|------|------|-------|
| Crate setup and types | 45 min | Input/output types, serialization |
| Prompt templates | 30 min | Write clear, effective prompts |
| GenerateStep implementation | 1 hour | Step logic, validation, tests |
| CritiqueStep implementation | 1.5 hours | JSON parsing, decision logic, tests |
| ReviseStep implementation | 1 hour | Feedback integration, tests |
| Module organization | 30 min | Exports, registration function |
| Integration tests | 45 min | End-to-end workflow tests |
| Property tests | 30 min | Roundtrip and validation tests |

### Dependencies

Must complete before starting:
- Stage 2.1: Step trait definition
- Stage 2.2: Step executor

---

## Implementation Checklist

### Phase 1: Setup (45 min)
- [ ] Create `ecl-steps` crate
- [ ] Update workspace `Cargo.toml`
- [ ] Add dependencies
- [ ] Create directory structure
- [ ] Define all input/output types
- [ ] Write type tests

### Phase 2: Prompts (30 min)
- [ ] Create `prompts/` directory
- [ ] Write `generate.txt` prompt
- [ ] Write `critique.txt` prompt
- [ ] Write `revise.txt` prompt
- [ ] Test prompt file inclusion

### Phase 3: GenerateStep (1 hour)
- [ ] Implement step struct
- [ ] Implement `Step` trait
- [ ] Add validation logic
- [ ] Write unit tests
- [ ] Verify test coverage

### Phase 4: CritiqueStep (1.5 hours)
- [ ] Implement step struct
- [ ] Implement `Step` trait
- [ ] Add JSON parsing logic
- [ ] Add validation logic
- [ ] Write unit tests (including parse tests)
- [ ] Verify test coverage

### Phase 5: ReviseStep (1 hour)
- [ ] Implement step struct
- [ ] Implement `Step` trait
- [ ] Add validation logic
- [ ] Write unit tests
- [ ] Verify test coverage

### Phase 6: Module Organization (30 min)
- [ ] Create `toy/mod.rs`
- [ ] Export all types and steps
- [ ] Implement `register_steps()` function
- [ ] Write registration tests
- [ ] Update `lib.rs`

### Phase 7: Integration Testing (1.25 hours)
- [ ] Write end-to-end pass workflow test
- [ ] Write end-to-end revision workflow test
- [ ] Write max revision limit test
- [ ] Write property-based tests
- [ ] Run full test suite
- [ ] Verify coverage ≥95%

### Phase 8: Documentation & Polish (30 min)
- [ ] Add/review rustdoc comments
- [ ] Verify examples compile
- [ ] Run clippy and fix warnings
- [ ] Run rustfmt
- [ ] Final test run
- [ ] Document any known limitations

---

## Files Created

```
crates/ecl-steps/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── toy/
│   │   ├── mod.rs
│   │   ├── types.rs
│   │   ├── generate.rs
│   │   ├── critique.rs
│   │   └── revise.rs
│   └── prompts/
│       ├── generate.txt
│       ├── critique.txt
│       └── revise.txt
└── tests/
    ├── integration.rs
    └── property.rs
```

**Total:** ~12 new files

---

## Next Steps After Completion

Once this stage is complete:

1. **Verify all acceptance criteria** are met
2. **Run full test suite** with coverage report
3. **Document any deviations** from the plan
4. **Commit changes** with descriptive message
5. **Proceed to Stage 2.4:** Workflow Orchestrator

The next stage will use these step implementations to build the orchestrator that composes them into the full Critique-Revise workflow.

---

**Next:** [Stage 2.4: Workflow Orchestrator](./0006-phase-2-04-workflow-orchestrator.md)
