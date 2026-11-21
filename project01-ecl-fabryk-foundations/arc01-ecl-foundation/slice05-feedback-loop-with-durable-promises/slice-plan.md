# Stage 1.5: Feedback Loop with Durable Promises

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/0001-phase-1-implementation.md, lines 2537-3003. Original bytes are recoverable from import commit 2d006a7. -->

## Stage 1.5: Feedback Loop with Durable Promises

**Goal:** Extend workflow to demonstrate bounded revision cycles using Durable Promises.

**Dependencies:** Stage 1.4 (basic Restate workflow)

### Detailed Implementation Steps

#### Step 1: Define Critique Decision Type

**`crates/ecl-core/src/types/critique.rs`**

```rust
//! Types for critique and revision decisions.

use serde::{Deserialize, Serialize};

/// Decision from a critique step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CritiqueDecision {
    /// Content passes critique, no revision needed
    Pass,

    /// Content needs revision with specific feedback
    Revise {
        /// Specific feedback on what to improve
        feedback: String,
    },
}

impl CritiqueDecision {
    /// Returns `true` if the decision is Pass.
    pub fn is_pass(&self) -> bool {
        matches!(self, CritiqueDecision::Pass)
    }

    /// Returns `true` if the decision is Revise.
    pub fn needs_revision(&self) -> bool {
        matches!(self, CritiqueDecision::Revise { .. })
    }

    /// Extracts the feedback if this is a Revise decision.
    pub fn feedback(&self) -> Option<&str> {
        match self {
            CritiqueDecision::Revise { feedback } => Some(feedback),
            CritiqueDecision::Pass => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pass_decision() {
        let decision = CritiqueDecision::Pass;
        assert!(decision.is_pass());
        assert!(!decision.needs_revision());
        assert_eq!(decision.feedback(), None);
    }

    #[test]
    fn test_revise_decision() {
        let decision = CritiqueDecision::Revise {
            feedback: "Needs more detail".to_string(),
        };
        assert!(!decision.is_pass());
        assert!(decision.needs_revision());
        assert_eq!(decision.feedback(), Some("Needs more detail"));
    }

    #[test]
    fn test_critique_decision_serialization() {
        let decision = CritiqueDecision::Revise {
            feedback: "Test feedback".to_string(),
        };
        let json = serde_json::to_string(&decision).unwrap();
        let deserialized: CritiqueDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(decision, deserialized);
    }
}
```

Update `crates/ecl-core/src/types/mod.rs`:

```rust
mod critique;
pub use critique::CritiqueDecision;
```

#### Step 2: Implement Critique-Revise Workflow

**`crates/ecl-workflows/src/critique_loop.rs`**

```rust
//! Critique-Revise workflow with bounded feedback loop.

use serde::{Deserialize, Serialize};
use restate_sdk::prelude::*;
use std::sync::Arc;

use ecl_core::{
    WorkflowId, Result, Error,
    llm::{LlmProvider, Message, CompletionRequest},
    types::CritiqueDecision,
};

/// Maximum number of revision attempts before giving up.
const MAX_REVISIONS: u32 = 3;

/// Input for critique-revise workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueLoopInput {
    pub workflow_id: WorkflowId,
    pub topic: String,
}

/// Output from critique-revise workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueLoopOutput {
    pub workflow_id: WorkflowId,
    pub final_text: String,
    pub revision_count: u32,
    pub critiques: Vec<String>,
}

/// Workflow with critique and revision loop.
pub struct CritiqueLoopWorkflow {
    llm: Arc<dyn LlmProvider>,
}

impl CritiqueLoopWorkflow {
    pub fn new(llm: Arc<dyn LlmProvider>) -> Self {
        Self { llm }
    }

    /// Runs the critique-revise workflow with bounded iteration.
    pub async fn run(
        &self,
        ctx: Context<'_>,
        input: CritiqueLoopInput,
    ) -> Result<CritiqueLoopOutput> {
        // Step 1: Generate initial draft
        let mut current_draft = ctx
            .run("generate", || async {
                self.generate_step(&input.topic).await
            })
            .await?;

        let mut revision_count = 0u32;
        let mut critiques = Vec::new();

        // Revision loop with bounded iterations
        loop {
            // Step 2: Critique current draft
            let (critique_text, decision) = ctx
                .run(&format!("critique_{}", revision_count), || async {
                    self.critique_step(&current_draft, revision_count).await
                })
                .await?;

            critiques.push(critique_text.clone());

            match decision {
                CritiqueDecision::Pass => {
                    tracing::info!(
                        revision_count,
                        "Critique passed, workflow complete"
                    );
                    break;
                }
                CritiqueDecision::Revise { feedback } => {
                    if revision_count >= MAX_REVISIONS {
                        return Err(Error::MaxRevisionsExceeded {
                            attempts: MAX_REVISIONS,
                        });
                    }

                    tracing::info!(
                        revision_count,
                        feedback = %feedback,
                        "Revision requested"
                    );

                    // Step 3: Revise based on feedback
                    current_draft = ctx
                        .run(&format!("revise_{}", revision_count), || async {
                            self.revise_step(&current_draft, &feedback, revision_count).await
                        })
                        .await?;

                    revision_count += 1;
                }
            }
        }

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

        let request = CompletionRequest::new(vec![
            Message::user(format!("Write a paragraph about: {}", topic))
        ])
        .with_system_prompt("You are a content generator. Write clear paragraphs.")
        .with_max_tokens(500);

        let response = self.llm.complete(request).await?;
        Ok(response.content)
    }

    /// Critique content and decide if revision is needed.
    async fn critique_step(
        &self,
        content: &str,
        attempt: u32,
    ) -> Result<(String, CritiqueDecision)> {
        tracing::info!(attempt, "Critiquing content");

        let request = CompletionRequest::new(vec![
            Message::user(format!(
                "Critique this text and decide if it needs revision.\n\
                Respond with JSON: {{\"decision\": \"pass\" or \"revise\", \"critique\": \"your critique\", \"feedback\": \"what to improve\"}}\n\n\
                Text:\n{}",
                content
            ))
        ])
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

        Ok((critique, decision))
    }

    /// Revise content based on feedback.
    async fn revise_step(
        &self,
        original: &str,
        feedback: &str,
        attempt: u32,
    ) -> Result<String> {
        tracing::info!(attempt, "Revising content");

        let request = CompletionRequest::new(vec![
            Message::user(format!(
                "Revise this text based on the feedback:\n\n\
                Original:\n{}\n\n\
                Feedback:\n{}",
                original, feedback
            ))
        ])
        .with_system_prompt("You are an editor. Improve the text based on feedback.")
        .with_max_tokens(600);

        let response = self.llm.complete(request).await?;
        Ok(response.content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_critique_loop_input() {
        let input = CritiqueLoopInput {
            workflow_id: WorkflowId::new(),
            topic: "Test".to_string(),
        };
        assert_eq!(input.topic, "Test");
    }

    #[test]
    fn test_max_revisions_constant() {
        assert_eq!(MAX_REVISIONS, 3);
    }
}
```

#### Step 3: Add Workflow State Persistence

**`crates/ecl-workflows/src/critique_loop.rs`** (add to workflow impl)

```rust
/// State stored in Restate K/V for workflow persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkflowState {
    current_draft: String,
    revision_count: u32,
    critiques: Vec<String>,
}

impl CritiqueLoopWorkflow {
    /// Store workflow state in Restate K/V.
    async fn save_state(&self, ctx: &Context<'_>, state: &WorkflowState) -> Result<()> {
        ctx.set("workflow_state", state).await;
        Ok(())
    }

    /// Load workflow state from Restate K/V.
    async fn load_state(&self, ctx: &Context<'_>) -> Result<Option<WorkflowState>> {
        Ok(ctx.get::<WorkflowState>("workflow_state").await?)
    }
}
```

#### Step 4: Integration with Main

Update `crates/ecl-workflows/src/main.rs`:

```rust
mod critique_loop;
use critique_loop::CritiqueLoopWorkflow;

// In main():
let critique_workflow = CritiqueLoopWorkflow::new(llm.clone());

let endpoint = Endpoint::builder()
    .bind(service)
    .bind(critique_workflow)
    .build();
```

### Testing Strategy

**Test coverage goal:** ≥95%

**Tests needed:**

1. **Revision loop logic**:
   - Passes on first critique (no revisions)
   - Revises once and passes
   - Revises multiple times and passes
   - Hits MAX_REVISIONS and fails appropriately

2. **State persistence**:
   - State saves correctly
   - State loads after restart
   - Revision count persists

3. **Decision parsing**:
   - Valid pass decision
   - Valid revise decision with feedback
   - Invalid JSON rejected
   - Missing fields rejected

**Integration tests:**

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use ecl_core::llm::MockLlmProvider;

    #[tokio::test]
    async fn test_workflow_passes_immediately() {
        let mock = Arc::new(MockLlmProvider::new(vec![
            "Initial draft".to_string(),
            r#"{"decision": "pass", "critique": "Looks good!"}"#.to_string(),
        ]));

        let workflow = CritiqueLoopWorkflow::new(mock);
        // Test execution...
    }

    #[tokio::test]
    async fn test_workflow_revision_loop() {
        let mock = Arc::new(MockLlmProvider::new(vec![
            "Draft 1".to_string(),
            r#"{"decision": "revise", "critique": "Needs work", "feedback": "Add detail"}"#.to_string(),
            "Draft 2".to_string(),
            r#"{"decision": "pass", "critique": "Better!"}"#.to_string(),
        ]));

        let workflow = CritiqueLoopWorkflow::new(mock);
        // Test execution...
    }

    #[tokio::test]
    async fn test_max_revisions_exceeded() {
        let mock = Arc::new(MockLlmProvider::new(vec![
            "Draft".to_string(),
            r#"{"decision": "revise", "critique": "Bad", "feedback": "Fix it"}"#.to_string(),
            "Draft".to_string(),
            r#"{"decision": "revise", "critique": "Still bad", "feedback": "Fix more"}"#.to_string(),
            "Draft".to_string(),
            r#"{"decision": "revise", "critique": "Nope", "feedback": "Again"}"#.to_string(),
            "Draft".to_string(),
            r#"{"decision": "revise", "critique": "Nope", "feedback": "Last try"}"#.to_string(),
        ]));

        let workflow = CritiqueLoopWorkflow::new(mock);
        // Should error with MaxRevisionsExceeded
    }
}
```

### Potential Blockers

1. **Restate state persistence API**
   - **Mitigation**: Review Restate K/V documentation
   - **Fallback**: Store in workflow-local memory for Phase 1

2. **JSON parsing from LLM**
   - **Mitigation**: Use structured prompts
   - **Mitigation**: Validate and provide helpful errors
   - **Fallback**: Use simpler text-based decisions

3. **Loop termination edge cases**
   - **Mitigation**: Comprehensive testing
   - **Mitigation**: Always check revision_count bounds

### Acceptance Criteria

- [ ] Workflow correctly loops on Revise decisions
- [ ] Loop terminates at MAX_REVISIONS with appropriate error
- [ ] Loop terminates on Pass decision
- [ ] Workflow survives restart mid-revision-loop
- [ ] State (revision_count, current_draft) persists across restarts
- [ ] All decision types handled correctly
- [ ] JSON parsing errors are graceful
- [ ] Test coverage ≥95%
- [ ] Tracing logs show loop iterations

### Estimated Effort

**Time:** 4-5 hours

**Breakdown:**
- Critique decision types: 1 hour
- Workflow implementation: 2 hours
- State persistence: 1 hour
- Testing: 1.5 hours

---

