# Stage 2.1: Step Trait Definition

**Version:** 1.0
**Date:** 2026-01-23
**Phase:** 2 - Step Abstraction Framework
**Stage:** 2.1
**Status:** Ready for Implementation

---

## Goal

Define the core `Step` trait that all workflow steps implement.

## Dependencies

Phase 1 complete (core types, error handling, LLM abstraction)

## Duration

**Time:** 4-5 hours

**Breakdown:**
- Step trait design: 1 hour
- StepContext implementation: 45 min
- RetryPolicy implementation: 1 hour
- StepRegistry implementation: 1 hour
- Testing: 1.5 hours

---

## Detailed Implementation Steps

### Step 1: Define Step Trait

**Before writing code:**
1. Load `assets/ai/ai-rust/guides/06-traits.md`
2. Load `assets/ai/ai-rust/guides/07-concurrency-async.md`
3. Review trait design patterns and async-trait usage

**`crates/ecl-core/src/step/trait.rs`**

```rust
//! Core Step trait for workflow components.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{Result, StepId, StepResult};
use super::{StepContext, RetryPolicy, ValidationError};

/// A workflow step that can be executed.
///
/// Steps are the building blocks of workflows. Each step:
/// - Accepts typed input
/// - Produces typed output
/// - Can validate inputs and outputs
/// - Can specify retry behavior
/// - Can request revision (feedback loops)
///
/// # Examples
///
/// ```rust,no_run
/// use ecl_core::step::{Step, StepContext};
/// use async_trait::async_trait;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Serialize, Deserialize)]
/// struct MyInput {
///     value: String,
/// }
///
/// #[derive(Debug, Serialize, Deserialize)]
/// struct MyOutput {
///     result: String,
/// }
///
/// struct MyStep;
///
/// #[async_trait]
/// impl Step for MyStep {
///     type Input = MyInput;
///     type Output = MyOutput;
///
///     fn id(&self) -> &StepId {
///         &StepId::new("my-step")
///     }
///
///     fn name(&self) -> &str {
///         "My Step"
///     }
///
///     async fn execute(
///         &self,
///         ctx: &StepContext,
///         input: Self::Input,
///     ) -> StepResult<Self::Output> {
///         // Implementation here
///         todo!()
///     }
/// }
/// ```
#[async_trait]
pub trait Step: Send + Sync + fmt::Debug {
    /// Input type for this step.
    ///
    /// Must be serializable for Restate persistence.
    type Input: Serialize + for<'de> Deserialize<'de> + Send + fmt::Debug;

    /// Output type for this step.
    ///
    /// Must be serializable for Restate persistence.
    type Output: Serialize + for<'de> Deserialize<'de> + Send + fmt::Debug;

    /// Returns the unique identifier for this step.
    fn id(&self) -> &StepId;

    /// Returns the human-readable name of this step.
    fn name(&self) -> &str;

    /// Executes the step with the given input.
    ///
    /// This is the core logic of the step. It receives a context
    /// (providing access to LLM, storage, etc.) and input data,
    /// and produces either success, a request for revision, or failure.
    ///
    /// # Errors
    ///
    /// Returns `StepResult::Failed` if the step cannot complete.
    /// The error should indicate whether it's retryable.
    async fn execute(
        &self,
        ctx: &StepContext,
        input: Self::Input,
    ) -> StepResult<Self::Output>;

    /// Validates input before execution.
    ///
    /// Default implementation performs no validation (accepts all inputs).
    /// Override this to add custom validation logic.
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if input is invalid.
    async fn validate_input(&self, _input: &Self::Input) -> Result<(), ValidationError> {
        Ok(())
    }

    /// Validates output after execution.
    ///
    /// Default implementation performs no validation (accepts all outputs).
    /// Override this to add custom validation logic.
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if output is invalid.
    async fn validate_output(&self, _output: &Self::Output) -> Result<(), ValidationError> {
        Ok(())
    }

    /// Returns the maximum number of revision iterations for this step.
    ///
    /// Used when this step is part of a feedback loop.
    /// Default: 3 revisions.
    fn max_revisions(&self) -> u32 {
        3
    }

    /// Returns the retry policy for this step.
    ///
    /// Determines how the executor should retry on transient failures.
    /// Default: exponential backoff with 3 attempts.
    fn retry_policy(&self) -> RetryPolicy {
        RetryPolicy::default()
    }
}

/// Validation error with field-level details.
#[derive(Debug, Clone, thiserror::Error)]
pub struct ValidationError {
    /// Field that failed validation (if applicable)
    pub field: Option<String>,

    /// Validation error message
    pub message: String,

    /// Additional context
    pub details: Option<serde_json::Value>,
}

impl ValidationError {
    /// Creates a new validation error.
    pub fn new<S: Into<String>>(message: S) -> Self {
        Self {
            field: None,
            message: message.into(),
            details: None,
        }
    }

    /// Creates a validation error for a specific field.
    pub fn for_field<F, M>(field: F, message: M) -> Self
    where
        F: Into<String>,
        M: Into<String>,
    {
        Self {
            field: Some(field.into()),
            message: message.into(),
            details: None,
        }
    }

    /// Adds additional context to the error.
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(field) = &self.field {
            write!(f, "Validation error in field '{}': {}", field, self.message)
        } else {
            write!(f, "Validation error: {}", self.message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::new("test error");
        assert_eq!(err.to_string(), "Validation error: test error");
    }

    #[test]
    fn test_validation_error_with_field() {
        let err = ValidationError::for_field("email", "invalid format");
        assert!(err.to_string().contains("field 'email'"));
        assert!(err.to_string().contains("invalid format"));
    }

    #[test]
    fn test_validation_error_with_details() {
        let err = ValidationError::new("test")
            .with_details(serde_json::json!({"code": 123}));
        assert!(err.details.is_some());
    }
}
```

### Step 2: Define StepContext

**`crates/ecl-core/src/step/context.rs`**

```rust
//! Step execution context providing access to runtime dependencies.

use std::sync::Arc;
use tracing::Span;

use crate::{
    llm::LlmProvider,
    types::{StepId, WorkflowId},
};

/// Context provided to steps during execution.
///
/// Gives steps access to:
/// - LLM provider for making AI calls
/// - Workflow metadata (ID, current step, attempt number)
/// - Tracing span for structured logging
/// - Future: artifact storage
#[derive(Clone)]
pub struct StepContext {
    /// LLM provider for this execution
    llm: Arc<dyn LlmProvider>,

    /// Workflow ID
    workflow_id: WorkflowId,

    /// Current step ID
    step_id: StepId,

    /// Attempt number (1-indexed)
    attempt: u32,

    /// Tracing span for this step execution
    span: Span,
}

impl StepContext {
    /// Creates a new step context.
    pub fn new(
        llm: Arc<dyn LlmProvider>,
        workflow_id: WorkflowId,
        step_id: StepId,
        attempt: u32,
    ) -> Self {
        let span = tracing::info_span!(
            "step_execution",
            workflow_id = %workflow_id,
            step_id = %step_id,
            attempt = attempt,
        );

        Self {
            llm,
            workflow_id,
            step_id,
            attempt,
            span,
        }
    }

    /// Returns the LLM provider.
    pub fn llm(&self) -> &Arc<dyn LlmProvider> {
        &self.llm
    }

    /// Returns the workflow ID.
    pub fn workflow_id(&self) -> &WorkflowId {
        &self.workflow_id
    }

    /// Returns the current step ID.
    pub fn step_id(&self) -> &StepId {
        &self.step_id
    }

    /// Returns the attempt number (1-indexed).
    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    /// Returns the tracing span for this execution.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Enters the tracing span for this step.
    pub fn enter(&self) -> tracing::span::Entered<'_> {
        self.span.enter()
    }
}

impl std::fmt::Debug for StepContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StepContext")
            .field("workflow_id", &self.workflow_id)
            .field("step_id", &self.step_id)
            .field("attempt", &self.attempt)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockLlmProvider;

    #[test]
    fn test_step_context_creation() {
        let llm = Arc::new(MockLlmProvider::with_response("test"));
        let workflow_id = WorkflowId::new();
        let step_id = StepId::new("test-step");

        let ctx = StepContext::new(llm, workflow_id, step_id.clone(), 1);

        assert_eq!(ctx.workflow_id(), &workflow_id);
        assert_eq!(ctx.step_id(), &step_id);
        assert_eq!(ctx.attempt(), 1);
    }
}
```

### Step 3: Define RetryPolicy

**`crates/ecl-core/src/step/policy.rs`**

```rust
//! Retry policies for step execution.

use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Retry policy for step execution.
///
/// Defines how many times to retry a failed step and with what delays.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Initial delay before first retry
    pub initial_delay: Duration,

    /// Maximum delay between retries
    pub max_delay: Duration,

    /// Multiplier for exponential backoff
    pub multiplier: f64,
}

impl RetryPolicy {
    /// Creates a new retry policy.
    pub fn new(
        max_attempts: u32,
        initial_delay: Duration,
        max_delay: Duration,
        multiplier: f64,
    ) -> Self {
        Self {
            max_attempts,
            initial_delay,
            max_delay,
            multiplier,
        }
    }

    /// Creates a policy with no retries.
    pub fn none() -> Self {
        Self {
            max_attempts: 1,
            initial_delay: Duration::from_secs(0),
            max_delay: Duration::from_secs(0),
            multiplier: 1.0,
        }
    }

    /// Creates a policy with linear backoff.
    pub fn linear(max_attempts: u32, delay: Duration) -> Self {
        Self {
            max_attempts,
            initial_delay: delay,
            max_delay: delay,
            multiplier: 1.0,
        }
    }

    /// Creates a policy with exponential backoff.
    pub fn exponential(max_attempts: u32, initial_delay: Duration, max_delay: Duration) -> Self {
        Self {
            max_attempts,
            initial_delay,
            max_delay,
            multiplier: 2.0,
        }
    }

    /// Calculates the delay for a given attempt number.
    ///
    /// # Arguments
    ///
    /// * `attempt` - The attempt number (0-indexed)
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_secs(0);
        }

        let delay_secs = self.initial_delay.as_secs_f64()
            * self.multiplier.powi((attempt - 1) as i32);

        let capped = delay_secs.min(self.max_delay.as_secs_f64());
        Duration::from_secs_f64(capped)
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::exponential(
            3,
            Duration::from_secs(1),
            Duration::from_secs(10),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.initial_delay, Duration::from_secs(1));
        assert_eq!(policy.max_delay, Duration::from_secs(10));
        assert_eq!(policy.multiplier, 2.0);
    }

    #[test]
    fn test_retry_policy_none() {
        let policy = RetryPolicy::none();
        assert_eq!(policy.max_attempts, 1);
    }

    #[test]
    fn test_retry_policy_linear() {
        let policy = RetryPolicy::linear(3, Duration::from_secs(2));
        assert_eq!(policy.multiplier, 1.0);
        assert_eq!(policy.initial_delay, Duration::from_secs(2));
    }

    #[test]
    fn test_exponential_backoff_delays() {
        let policy = RetryPolicy::exponential(
            5,
            Duration::from_secs(1),
            Duration::from_secs(10),
        );

        assert_eq!(policy.delay_for_attempt(0), Duration::from_secs(0)); // No delay before first attempt
        assert_eq!(policy.delay_for_attempt(1), Duration::from_secs(1)); // 1s
        assert_eq!(policy.delay_for_attempt(2), Duration::from_secs(2)); // 2s
        assert_eq!(policy.delay_for_attempt(3), Duration::from_secs(4)); // 4s
        assert_eq!(policy.delay_for_attempt(4), Duration::from_secs(8)); // 8s
        assert_eq!(policy.delay_for_attempt(5), Duration::from_secs(10)); // Capped at max_delay
    }

    #[test]
    fn test_retry_policy_serialization() {
        let policy = RetryPolicy::default();
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: RetryPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy, deserialized);
    }
}
```

### Step 4: Define StepRegistry

**`crates/ecl-core/src/step/registry.rs`**

```rust
//! Registry for dynamically looking up steps by ID.

use std::collections::HashMap;
use std::sync::Arc;

use crate::{Error, Result, StepId};
use super::Step;

/// Registry of workflow steps.
///
/// Allows looking up step implementations by their ID at runtime.
/// This enables dynamic workflow construction where steps are
/// referenced by string IDs in workflow definitions.
pub struct StepRegistry {
    steps: HashMap<StepId, Arc<dyn StepTrait>>,
}

/// Type-erased step trait for registry storage.
///
/// Since `Step` has associated types, we can't make it object-safe directly.
/// This trait provides a type-erased interface for registry storage.
trait StepTrait: Send + Sync + std::fmt::Debug {
    fn id(&self) -> &StepId;
    fn name(&self) -> &str;
}

impl StepRegistry {
    /// Creates a new empty step registry.
    pub fn new() -> Self {
        Self {
            steps: HashMap::new(),
        }
    }

    /// Registers a step in the registry.
    ///
    /// # Errors
    ///
    /// Returns an error if a step with the same ID is already registered.
    pub fn register<S>(&mut self, step: S) -> Result<()>
    where
        S: Step + 'static,
    {
        let id = step.id().clone();

        if self.steps.contains_key(&id) {
            return Err(Error::config(format!(
                "Step with ID '{}' is already registered",
                id
            )));
        }

        self.steps.insert(id, Arc::new(StepWrapper::new(step)));
        Ok(())
    }

    /// Gets a step by ID.
    ///
    /// # Errors
    ///
    /// Returns `Error::StepNotFound` if no step with the given ID exists.
    pub fn get(&self, id: &StepId) -> Result<&Arc<dyn StepTrait>> {
        self.steps
            .get(id)
            .ok_or_else(|| Error::StepNotFound { id: id.to_string() })
    }

    /// Returns all registered step IDs.
    pub fn list_ids(&self) -> Vec<StepId> {
        self.steps.keys().cloned().collect()
    }

    /// Returns the number of registered steps.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Returns `true` if the registry has no steps.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

impl Default for StepRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper to make Step trait object-safe.
struct StepWrapper<S> {
    step: S,
}

impl<S> StepWrapper<S> {
    fn new(step: S) -> Self {
        Self { step }
    }
}

impl<S> StepTrait for StepWrapper<S>
where
    S: Step,
{
    fn id(&self) -> &StepId {
        self.step.id()
    }

    fn name(&self) -> &str {
        self.step.name()
    }
}

impl<S: Step> std::fmt::Debug for StepWrapper<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StepWrapper")
            .field("step", &self.step)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};

    #[derive(Debug)]
    struct TestStep {
        id: StepId,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct TestInput;

    #[derive(Debug, Serialize, Deserialize)]
    struct TestOutput;

    #[async_trait]
    impl Step for TestStep {
        type Input = TestInput;
        type Output = TestOutput;

        fn id(&self) -> &StepId {
            &self.id
        }

        fn name(&self) -> &str {
            "Test Step"
        }

        async fn execute(
            &self,
            _ctx: &super::StepContext,
            _input: Self::Input,
        ) -> crate::StepResult<Self::Output> {
            crate::StepResult::Success(TestOutput)
        }
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = StepRegistry::new();
        let step_id = StepId::new("test-step");
        let step = TestStep { id: step_id.clone() };

        registry.register(step).unwrap();

        let retrieved = registry.get(&step_id).unwrap();
        assert_eq!(retrieved.id(), &step_id);
    }

    #[test]
    fn test_registry_duplicate_registration_fails() {
        let mut registry = StepRegistry::new();
        let step_id = StepId::new("test-step");

        registry.register(TestStep { id: step_id.clone() }).unwrap();

        let result = registry.register(TestStep { id: step_id });
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_get_nonexistent_step() {
        let registry = StepRegistry::new();
        let result = registry.get(&StepId::new("nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_list_ids() {
        let mut registry = StepRegistry::new();

        registry.register(TestStep { id: StepId::new("step1") }).unwrap();
        registry.register(TestStep { id: StepId::new("step2") }).unwrap();

        let ids = registry.list_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&StepId::new("step1")));
        assert!(ids.contains(&StepId::new("step2")));
    }

    #[test]
    fn test_registry_len_and_is_empty() {
        let mut registry = StepRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);

        registry.register(TestStep { id: StepId::new("test") }).unwrap();
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
    }
}
```

### Step 5: Module Organization

**`crates/ecl-core/src/step/mod.rs`**

```rust
//! Step abstraction framework.
//!
//! This module provides the core traits and types for defining workflow steps.
//! Steps are the building blocks of workflows, encapsulating units of work
//! that can be composed, validated, retried, and monitored.

mod context;
mod policy;
mod registry;
mod r#trait;

pub use context::StepContext;
pub use policy::RetryPolicy;
pub use registry::StepRegistry;
pub use r#trait::{Step, ValidationError};

// Re-export for convenience
pub use crate::types::StepResult;
```

**Update `crates/ecl-core/src/lib.rs`:**

```rust
pub mod step;

// Add to re-exports
pub use step::{Step, StepContext, StepRegistry, RetryPolicy, ValidationError};
```

---

## Testing Strategy

**Test coverage goal:** ≥95%

### Tests Needed

1. **Step trait tests**:
   - Validation error construction and display
   - Default trait method implementations
   - Trait object creation (ensure it compiles)

2. **StepContext tests**:
   - Context creation
   - Accessor methods
   - Tracing span integration

3. **RetryPolicy tests**:
   - Default policy values
   - Exponential backoff calculation
   - Linear backoff
   - No-retry policy
   - Delay capping at max_delay
   - Serialization roundtrips

4. **StepRegistry tests**:
   - Register and retrieve steps
   - Duplicate registration failure
   - Get nonexistent step error
   - List all step IDs
   - Empty registry checks

### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_retry_policy_delay_never_exceeds_max(
        attempts in 1u32..100,
        initial_secs in 1u64..10,
        max_secs in 10u64..100,
    ) {
        let policy = RetryPolicy::exponential(
            attempts,
            Duration::from_secs(initial_secs),
            Duration::from_secs(max_secs),
        );

        for attempt in 0..attempts {
            let delay = policy.delay_for_attempt(attempt);
            assert!(delay.as_secs() <= max_secs);
        }
    }
}
```

---

## Potential Blockers

1. **Trait object safety issues**
   - **Mitigation**: Use type erasure pattern with wrapper types
   - **Mitigation**: Avoid associated types in trait objects
   - **Note**: StepRegistry uses type-erased wrapper

2. **Generic type complexity**
   - **Mitigation**: Keep trait bounds simple and well-documented
   - **Mitigation**: Provide clear examples in docs

3. **Async trait limitations**
   - **Mitigation**: Use `async-trait` crate
   - **Note**: Minor performance cost from boxing futures

---

## Acceptance Criteria

- [ ] `Step` trait compiles with proper bounds
- [ ] `StepContext` provides all required dependencies
- [ ] `RetryPolicy` calculates delays correctly
- [ ] `StepRegistry` can register and retrieve steps
- [ ] All types implement required traits (Debug, Clone where needed)
- [ ] Test coverage ≥95%
- [ ] All public APIs have rustdoc comments
- [ ] Examples in docs compile
- [ ] No compiler warnings
- [ ] Follows Rust anti-patterns guide

---

## Estimated Effort

**Time:** 4-5 hours

**Breakdown:**
- Step trait design: 1 hour
- StepContext implementation: 45 min
- RetryPolicy implementation: 1 hour
- StepRegistry implementation: 1 hour
- Testing: 1.5 hours

---

**Next:** [Stage 2.2: Step Executor](./0004-phase-2-02-step-executor.md)
