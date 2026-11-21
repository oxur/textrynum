# Stage 2.2: Step Executor

**Version:** 1.0
**Date:** 2026-01-23

## Goal

Create executor that runs steps with retry, validation, and observability.

## Dependencies

- Stage 2.1 (Step trait definition)

## Duration

4-5 hours

## Detailed Implementation Steps

### Step 1: Define StepExecution Result Type

**`crates/ecl-core/src/step/execution.rs`**

```rust
//! Step execution results and metadata.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::StepMetadata;

/// Result of executing a workflow step.
///
/// Contains the output along with execution metadata and traces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecution<T> {
    /// Step output
    pub output: T,

    /// Execution metadata (timing, tokens, etc.)
    pub metadata: StepMetadata,

    /// Trace events captured during execution
    pub traces: Vec<TraceEvent>,
}

impl<T> StepExecution<T> {
    /// Creates a new step execution result.
    pub fn new(output: T, metadata: StepMetadata) -> Self {
        Self {
            output,
            metadata,
            traces: Vec::new(),
        }
    }

    /// Adds a trace event.
    pub fn add_trace(&mut self, event: TraceEvent) {
        self.traces.push(event);
    }

    /// Maps the output using a function.
    pub fn map<U, F>(self, f: F) -> StepExecution<U>
    where
        F: FnOnce(T) -> U,
    {
        StepExecution {
            output: f(self.output),
            metadata: self.metadata,
            traces: self.traces,
        }
    }
}

/// A trace event captured during step execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    /// When this event occurred
    pub timestamp: DateTime<Utc>,

    /// Event type/level
    pub level: TraceLevel,

    /// Event message
    pub message: String,

    /// Additional structured data
    pub fields: serde_json::Value,
}

impl TraceEvent {
    /// Creates a new trace event.
    pub fn new(level: TraceLevel, message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            message: message.into(),
            fields: serde_json::Value::Null,
        }
    }

    /// Adds structured fields to the event.
    pub fn with_fields(mut self, fields: serde_json::Value) -> Self {
        self.fields = fields;
        self
    }
}

/// Trace event level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceLevel {
    /// Debug-level trace
    Debug,
    /// Info-level trace
    Info,
    /// Warning-level trace
    Warn,
    /// Error-level trace
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StepId;

    #[test]
    fn test_step_execution_creation() {
        let metadata = StepMetadata::new(StepId::new("test"));
        let execution = StepExecution::new("output", metadata.clone());

        assert_eq!(execution.output, "output");
        assert_eq!(execution.metadata.step_id, metadata.step_id);
        assert!(execution.traces.is_empty());
    }

    #[test]
    fn test_step_execution_map() {
        let metadata = StepMetadata::new(StepId::new("test"));
        let execution = StepExecution::new(5, metadata);
        let mapped = execution.map(|x| x * 2);

        assert_eq!(mapped.output, 10);
    }

    #[test]
    fn test_trace_event_creation() {
        let event = TraceEvent::new(TraceLevel::Info, "test message");
        assert_eq!(event.level, TraceLevel::Info);
        assert_eq!(event.message, "test message");
    }

    #[test]
    fn test_trace_event_with_fields() {
        let event = TraceEvent::new(TraceLevel::Debug, "test")
            .with_fields(serde_json::json!({"key": "value"}));
        assert!(event.fields.is_object());
    }
}
```

### Step 2: Implement StepExecutor

**`crates/ecl-core/src/step/executor.rs`**

```rust
//! Step executor with retry, validation, and observability.

use std::sync::Arc;
use backon::{ExponentialBuilder, Retryable};

use crate::{
    llm::LlmProvider,
    types::{StepMetadata, StepResult, WorkflowId},
    Error, Result,
};
use super::{Step, StepContext, StepExecution, StepRegistry, TraceEvent, TraceLevel};

/// Executor for running workflow steps.
///
/// Handles:
/// - Input validation
/// - Retry with backoff
/// - Output validation
/// - Trace collection
/// - Metadata recording
pub struct StepExecutor {
    llm: Arc<dyn LlmProvider>,
    registry: Arc<StepRegistry>,
}

impl StepExecutor {
    /// Creates a new step executor.
    pub fn new(llm: Arc<dyn LlmProvider>, registry: Arc<StepRegistry>) -> Self {
        Self { llm, registry }
    }

    /// Executes a step with full validation and retry logic.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Input validation fails
    /// - Step execution fails after all retries
    /// - Output validation fails
    pub async fn execute_step<S>(
        &self,
        step: &S,
        input: S::Input,
        workflow_id: &WorkflowId,
    ) -> Result<StepExecution<S::Output>>
    where
        S: Step,
    {
        let mut metadata = StepMetadata::new(step.id().clone());
        let mut traces = Vec::new();

        // Create step context
        let ctx = StepContext::new(
            self.llm.clone(),
            workflow_id.clone(),
            step.id().clone(),
            metadata.attempt,
        );

        // Enter tracing span
        let _guard = ctx.enter();

        // Step 1: Validate input
        tracing::debug!("Validating step input");
        traces.push(TraceEvent::new(TraceLevel::Debug, "Validating input"));

        step.validate_input(&input)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Input validation failed");
                Error::validation(e.to_string())
            })?;

        traces.push(TraceEvent::new(TraceLevel::Info, "Input validation passed"));

        // Step 2: Execute with retry
        tracing::info!("Executing step");
        traces.push(TraceEvent::new(TraceLevel::Info, "Starting step execution"));

        let policy = step.retry_policy();
        let backoff = ExponentialBuilder::default()
            .with_min_delay(policy.initial_delay)
            .with_max_delay(policy.max_delay)
            .with_max_times(policy.max_attempts as usize);

        let result = (|| async {
            let step_result = step.execute(&ctx, input.clone()).await;

            match &step_result {
                StepResult::Success(_) => {
                    tracing::info!("Step executed successfully");
                    Ok(step_result)
                }
                StepResult::NeedsRevision { .. } => {
                    tracing::info!("Step completed with revision request");
                    Ok(step_result)
                }
                StepResult::Failed { error, retryable } => {
                    tracing::warn!(
                        error = %error,
                        retryable = %retryable,
                        "Step execution failed"
                    );

                    if *retryable {
                        Err(Error::llm(error))
                    } else {
                        // Non-retryable error, return it wrapped
                        Ok(step_result)
                    }
                }
            }
        })
        .retry(backoff)
        .when(|e: &Error| e.is_retryable())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Step execution failed after retries");
            e
        })?;

        // Extract output from result
        let output = match result {
            StepResult::Success(output) | StepResult::NeedsRevision { output, .. } => output,
            StepResult::Failed { error, .. } => {
                return Err(Error::llm(error));
            }
        };

        traces.push(TraceEvent::new(TraceLevel::Info, "Step execution completed"));

        // Step 3: Validate output
        tracing::debug!("Validating step output");
        traces.push(TraceEvent::new(TraceLevel::Debug, "Validating output"));

        step.validate_output(&output)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Output validation failed");
                Error::validation(e.to_string())
            })?;

        traces.push(TraceEvent::new(TraceLevel::Info, "Output validation passed"));

        // Mark metadata as completed
        metadata.mark_completed();

        tracing::info!(
            duration_ms = metadata.duration().unwrap().num_milliseconds(),
            "Step execution successful"
        );

        Ok(StepExecution {
            output,
            metadata,
            traces,
        })
    }

    /// Returns a reference to the step registry.
    pub fn registry(&self) -> &Arc<StepRegistry> {
        &self.registry
    }

    /// Returns a reference to the LLM provider.
    pub fn llm(&self) -> &Arc<dyn LlmProvider> {
        &self.llm
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockLlmProvider;
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestInput {
        value: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestOutput {
        result: String,
    }

    #[derive(Debug)]
    struct TestStep;

    #[async_trait]
    impl Step for TestStep {
        type Input = TestInput;
        type Output = TestOutput;

        fn id(&self) -> &crate::StepId {
            &crate::StepId::new("test-step")
        }

        fn name(&self) -> &str {
            "Test Step"
        }

        async fn execute(
            &self,
            _ctx: &StepContext,
            input: Self::Input,
        ) -> StepResult<Self::Output> {
            StepResult::Success(TestOutput {
                result: input.value.to_uppercase(),
            })
        }
    }

    #[tokio::test]
    async fn test_step_executor_success() {
        let llm = Arc::new(MockLlmProvider::with_response("test"));
        let registry = Arc::new(StepRegistry::new());
        let executor = StepExecutor::new(llm, registry);

        let step = TestStep;
        let input = TestInput {
            value: "hello".to_string(),
        };
        let workflow_id = WorkflowId::new();

        let execution = executor.execute_step(&step, input, &workflow_id).await.unwrap();

        assert_eq!(execution.output.result, "HELLO");
        assert!(execution.metadata.is_completed());
        assert!(!execution.traces.is_empty());
    }
}
```

### Step 3: Update Module Organization

Update `crates/ecl-core/src/step/mod.rs`:

```rust
mod execution;
mod executor;

pub use execution::{StepExecution, TraceEvent, TraceLevel};
pub use executor::StepExecutor;
```

## Testing Strategy

**Test coverage goal:** ≥95%

**Tests needed:**

1. **StepExecution tests**:
   - Creation and field access
   - Map operation
   - Trace event addition
   - Serialization

2. **TraceEvent tests**:
   - Event creation
   - Field addition
   - Level variants

3. **StepExecutor tests**:
   - Successful execution
   - Input validation failure
   - Output validation failure
   - Retry on transient failure
   - Non-retryable failure
   - Trace collection
   - Metadata recording

**Integration tests:**

```rust
#[tokio::test]
async fn test_executor_retries_on_failure() {
    // Mock LLM that fails twice then succeeds
    // Verify retry attempts and final success
}

#[tokio::test]
async fn test_executor_respects_max_retries() {
    // Mock that always fails
    // Verify it stops after max_attempts
}
```

## Potential Blockers

1. **Retry logic complexity**
   - **Mitigation**: Use `backon` crate as specified
   - **Mitigation**: Test thoroughly with different policies

2. **Trace collection overhead**
   - **Mitigation**: Keep traces minimal and structured
   - **Mitigation**: Consider async trace writing

3. **Input cloning for retries**
   - **Mitigation**: Require Input: Clone bound
   - **Note**: This is acceptable for most use cases

## Acceptance Criteria

- [ ] Steps execute with proper retry behavior
- [ ] Input validation runs before execution
- [ ] Output validation runs after execution
- [ ] Retry policy is respected
- [ ] Traces capture execution flow
- [ ] Metadata records timing and attempts
- [ ] Test coverage ≥95%
- [ ] All public APIs documented
- [ ] No compiler warnings

## Estimated Effort

**Time:** 4-5 hours

**Breakdown:**
- StepExecution types: 1 hour
- StepExecutor implementation: 2 hours
- Retry integration: 1 hour
- Testing: 1.5 hours

---

**Next:** [Stage 2.3: Toy Workflow Step Implementations](./0005-phase-2-03-toy-workflow-steps.md)
