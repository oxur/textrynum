# Stage 1.2: Core Types and Error Handling

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/0001-phase-1-implementation.md, lines 510-1421. Original bytes are recoverable from import commit 2d006a7. -->

## Stage 1.2: Core Types and Error Handling

**Goal:** Define foundational types that all other code builds upon.

**Dependencies:** Stage 1.1 (project scaffolding)

### Detailed Implementation Steps

#### Step 1: Define Error Types

**Before writing code:**
1. Load `assets/ai/ai-rust/guides/11-anti-patterns.md`
2. Load `assets/ai/ai-rust/guides/03-error-handling.md`
3. Review project error handling patterns

**`crates/ecl-core/src/error.rs`**

```rust
//! Error types for ECL core library.

use std::fmt;

/// Errors that can occur during ECL workflow execution.
///
/// All error variants are marked with `#[non_exhaustive]` to allow
/// adding new error types without breaking changes.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// LLM provider error (Claude API failures, rate limits, etc.)
    #[error("LLM error: {message}")]
    Llm {
        /// Human-readable error message
        message: String,
        /// Whether this error can be retried
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Step validation error
    #[error("Validation error: {message}")]
    Validation {
        /// Field or aspect that failed validation
        field: Option<String>,
        /// What went wrong
        message: String,
    },

    /// I/O error (file operations, network, etc.)
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Step execution timeout
    #[error("Step timed out after {seconds}s")]
    Timeout {
        /// Timeout duration in seconds
        seconds: u64,
    },

    /// Maximum revision iterations exceeded
    #[error("Maximum revisions exceeded: {attempts} attempts")]
    MaxRevisionsExceeded {
        /// Number of revision attempts made
        attempts: u32,
    },

    /// Database error
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Configuration error
    #[error("Configuration error: {message}")]
    Config {
        /// What configuration is problematic
        message: String,
    },

    /// Workflow not found
    #[error("Workflow not found: {id}")]
    WorkflowNotFound {
        /// Workflow ID that was not found
        id: String,
    },

    /// Step not found
    #[error("Step not found: {id}")]
    StepNotFound {
        /// Step ID that was not found
        id: String,
    },
}

/// Convenience `Result` type alias for ECL operations.
///
/// This is the standard Result type used throughout the ECL codebase.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Returns whether this error is retryable.
    ///
    /// Retryable errors include transient failures like rate limits,
    /// network timeouts, and temporary service unavailability.
    pub fn is_retryable(&self) -> bool {
        match self {
            Error::Llm { .. } => true,  // LLM errors are generally retryable
            Error::Io(_) => true,
            Error::Database(_) => true,  // Database errors may be transient
            Error::Timeout { .. } => true,
            Error::Validation { .. } => false,  // Validation errors are permanent
            Error::MaxRevisionsExceeded { .. } => false,
            Error::Serialization(_) => false,
            Error::Config { .. } => false,
            Error::WorkflowNotFound { .. } => false,
            Error::StepNotFound { .. } => false,
        }
    }

    /// Creates a new LLM error with a message.
    pub fn llm<S: Into<String>>(message: S) -> Self {
        Error::Llm {
            message: message.into(),
            source: None,
        }
    }

    /// Creates a new LLM error with a message and source error.
    pub fn llm_with_source<S, E>(message: S, source: E) -> Self
    where
        S: Into<String>,
        E: std::error::Error + Send + Sync + 'static,
    {
        Error::Llm {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Creates a new validation error.
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Error::Validation {
            field: None,
            message: message.into(),
        }
    }

    /// Creates a new validation error with a field name.
    pub fn validation_field<F, M>(field: F, message: M) -> Self
    where
        F: Into<String>,
        M: Into<String>,
    {
        Error::Validation {
            field: Some(field.into()),
            message: message.into(),
        }
    }

    /// Creates a new configuration error.
    pub fn config<S: Into<String>>(message: S) -> Self {
        Error::Config {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::llm("Rate limit exceeded");
        assert_eq!(err.to_string(), "LLM error: Rate limit exceeded");
    }

    #[test]
    fn test_retryable_classification() {
        assert!(Error::llm("test").is_retryable());
        assert!(Error::Timeout { seconds: 30 }.is_retryable());
        assert!(!Error::validation("test").is_retryable());
        assert!(!Error::MaxRevisionsExceeded { attempts: 3 }.is_retryable());
    }

    #[test]
    fn test_validation_error_with_field() {
        let err = Error::validation_field("topic", "must not be empty");
        match err {
            Error::Validation { field, message } => {
                assert_eq!(field, Some("topic".to_string()));
                assert_eq!(message, "must not be empty");
            }
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn test_max_revisions_exceeded() {
        let err = Error::MaxRevisionsExceeded { attempts: 5 };
        assert_eq!(
            err.to_string(),
            "Maximum revisions exceeded: 5 attempts"
        );
    }

    #[test]
    fn test_error_implements_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Error>();
    }
}
```

#### Step 2: Define Core Type Newtypes

**`crates/ecl-core/src/types/ids.rs`**

```rust
//! Unique identifier types for workflows and steps.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a workflow instance.
///
/// Internally represented as a UUID v4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowId(Uuid);

impl WorkflowId {
    /// Creates a new random workflow ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use ecl_core::WorkflowId;
    ///
    /// let id = WorkflowId::new();
    /// println!("Workflow ID: {}", id);
    /// ```
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates a workflow ID from a UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the inner UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Converts to the inner UUID.
    pub fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for WorkflowId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for WorkflowId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<WorkflowId> for Uuid {
    fn from(id: WorkflowId) -> Self {
        id.0
    }
}

impl std::str::FromStr for WorkflowId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

/// Unique identifier for a workflow step.
///
/// Step IDs are human-readable strings like "generate", "critique", "revise".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StepId(String);

impl StepId {
    /// Creates a new step ID from a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use ecl_core::StepId;
    ///
    /// let id = StepId::new("generate");
    /// assert_eq!(id.as_str(), "generate");
    /// ```
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self(id.into())
    }

    /// Returns the step ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StepId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for StepId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for StepId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for StepId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_id_new() {
        let id1 = WorkflowId::new();
        let id2 = WorkflowId::new();
        assert_ne!(id1, id2, "Each new ID should be unique");
    }

    #[test]
    fn test_workflow_id_display() {
        let uuid = Uuid::new_v4();
        let id = WorkflowId::from_uuid(uuid);
        assert_eq!(id.to_string(), uuid.to_string());
    }

    #[test]
    fn test_workflow_id_roundtrip_serialization() {
        let id = WorkflowId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: WorkflowId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_workflow_id_from_str() {
        let uuid = Uuid::new_v4();
        let id: WorkflowId = uuid.to_string().parse().unwrap();
        assert_eq!(id.as_uuid(), &uuid);
    }

    #[test]
    fn test_step_id_creation() {
        let id = StepId::new("generate");
        assert_eq!(id.as_str(), "generate");
    }

    #[test]
    fn test_step_id_from_string() {
        let id = StepId::from("critique".to_string());
        assert_eq!(id.as_str(), "critique");
    }

    #[test]
    fn test_step_id_from_str() {
        let id = StepId::from("revise");
        assert_eq!(id.as_str(), "revise");
    }

    #[test]
    fn test_step_id_display() {
        let id = StepId::new("test-step");
        assert_eq!(id.to_string(), "test-step");
    }

    #[test]
    fn test_step_id_roundtrip_serialization() {
        let id = StepId::new("my-step");
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: StepId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }
}
```

#### Step 3: Define Step Result and Workflow State

**`crates/ecl-core/src/types/step_result.rs`**

```rust
//! Step execution result types.

use serde::{Deserialize, Serialize};

/// The result of executing a workflow step.
///
/// Steps can succeed, request revision, or fail. This type captures
/// all possible outcomes along with their associated data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum StepResult<T> {
    /// Step completed successfully.
    Success(T),

    /// Step completed but requests revision of prior work.
    ///
    /// This variant is used in feedback loops where a critique step
    /// determines that prior output needs improvement.
    NeedsRevision {
        /// The output from this step (e.g., critique text)
        output: T,
        /// Feedback explaining what needs to be revised
        feedback: String,
    },

    /// Step failed with an error.
    Failed {
        /// The error that caused the failure
        error: String,
        /// Whether this failure can be retried
        retryable: bool,
    },
}

impl<T> StepResult<T> {
    /// Returns `true` if the result is `Success`.
    pub fn is_success(&self) -> bool {
        matches!(self, StepResult::Success(_))
    }

    /// Returns `true` if the result is `NeedsRevision`.
    pub fn is_needs_revision(&self) -> bool {
        matches!(self, StepResult::NeedsRevision { .. })
    }

    /// Returns `true` if the result is `Failed`.
    pub fn is_failed(&self) -> bool {
        matches!(self, StepResult::Failed { .. })
    }

    /// Returns `true` if the result is a retryable failure.
    pub fn is_retryable(&self) -> bool {
        match self {
            StepResult::Failed { retryable, .. } => *retryable,
            _ => false,
        }
    }

    /// Extracts the success value, panicking if not successful.
    ///
    /// # Panics
    ///
    /// Panics if the result is not `Success`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ecl_core::StepResult;
    ///
    /// let result = StepResult::Success(42);
    /// assert_eq!(result.unwrap(), 42);
    /// ```
    pub fn unwrap(self) -> T {
        match self {
            StepResult::Success(value) => value,
            StepResult::NeedsRevision { .. } => panic!("called `unwrap()` on NeedsRevision"),
            StepResult::Failed { error, .. } => panic!("called `unwrap()` on Failed: {}", error),
        }
    }

    /// Returns the success value, or a default if not successful.
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            StepResult::Success(value) => value,
            _ => default,
        }
    }

    /// Returns the success value, or computes it from a closure.
    pub fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        match self {
            StepResult::Success(value) => value,
            _ => f(),
        }
    }

    /// Maps a `StepResult<T>` to `StepResult<U>` by applying a function to the success value.
    pub fn map<U, F>(self, f: F) -> StepResult<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            StepResult::Success(value) => StepResult::Success(f(value)),
            StepResult::NeedsRevision { output, feedback } => StepResult::NeedsRevision {
                output: f(output),
                feedback,
            },
            StepResult::Failed { error, retryable } => StepResult::Failed { error, retryable },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_variant() {
        let result = StepResult::Success(42);
        assert!(result.is_success());
        assert!(!result.is_needs_revision());
        assert!(!result.is_failed());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_needs_revision_variant() {
        let result: StepResult<String> = StepResult::NeedsRevision {
            output: "critique".to_string(),
            feedback: "needs work".to_string(),
        };
        assert!(!result.is_success());
        assert!(result.is_needs_revision());
        assert!(!result.is_failed());
        assert!(!result.is_retryable());
    }

    #[test]
    fn test_failed_variant() {
        let result: StepResult<i32> = StepResult::Failed {
            error: "timeout".to_string(),
            retryable: true,
        };
        assert!(!result.is_success());
        assert!(!result.is_needs_revision());
        assert!(result.is_failed());
        assert!(result.is_retryable());
    }

    #[test]
    fn test_map() {
        let result = StepResult::Success(2);
        let mapped = result.map(|x| x * 2);
        assert_eq!(mapped.unwrap(), 4);
    }

    #[test]
    fn test_unwrap_or() {
        let success = StepResult::Success(10);
        assert_eq!(success.unwrap_or(0), 10);

        let failed: StepResult<i32> = StepResult::Failed {
            error: "error".to_string(),
            retryable: false,
        };
        assert_eq!(failed.unwrap_or(0), 0);
    }

    #[test]
    fn test_serialization() {
        let result = StepResult::Success("data".to_string());
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: StepResult<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(result, deserialized);
    }

    #[test]
    #[should_panic(expected = "called `unwrap()` on Failed")]
    fn test_unwrap_panics_on_failed() {
        let result: StepResult<i32> = StepResult::Failed {
            error: "test error".to_string(),
            retryable: false,
        };
        result.unwrap();
    }
}
```

**`crates/ecl-core/src/types/workflow_state.rs`**

```rust
//! Workflow state tracking types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{StepId, WorkflowId};

/// The current state of a workflow instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WorkflowState {
    /// Workflow has been created but not yet started.
    Pending,

    /// Workflow is actively executing steps.
    Running,

    /// Workflow is waiting for a revision to complete.
    WaitingForRevision,

    /// Workflow has completed successfully.
    Completed,

    /// Workflow has failed and cannot proceed.
    Failed,
}

impl WorkflowState {
    /// Returns `true` if the workflow is in a terminal state (Completed or Failed).
    pub fn is_terminal(&self) -> bool {
        matches!(self, WorkflowState::Completed | WorkflowState::Failed)
    }

    /// Returns `true` if the workflow is active (Running or WaitingForRevision).
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            WorkflowState::Running | WorkflowState::WaitingForRevision
        )
    }
}

impl std::fmt::Display for WorkflowState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowState::Pending => write!(f, "pending"),
            WorkflowState::Running => write!(f, "running"),
            WorkflowState::WaitingForRevision => write!(f, "waiting_for_revision"),
            WorkflowState::Completed => write!(f, "completed"),
            WorkflowState::Failed => write!(f, "failed"),
        }
    }
}

/// Metadata about a step execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepMetadata {
    /// Unique identifier for this step
    pub step_id: StepId,

    /// When this step started executing
    pub started_at: DateTime<Utc>,

    /// When this step completed (if finished)
    pub completed_at: Option<DateTime<Utc>>,

    /// Attempt number (1-indexed)
    pub attempt: u32,

    /// Number of LLM tokens used (if applicable)
    pub llm_tokens_used: Option<u64>,
}

impl StepMetadata {
    /// Creates new step metadata for an initial execution.
    pub fn new(step_id: StepId) -> Self {
        Self {
            step_id,
            started_at: Utc::now(),
            completed_at: None,
            attempt: 1,
            llm_tokens_used: None,
        }
    }

    /// Marks this step as completed.
    pub fn mark_completed(&mut self) {
        self.completed_at = Some(Utc::now());
    }

    /// Returns the duration of this step execution.
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.completed_at
            .map(|end| end.signed_duration_since(self.started_at))
    }

    /// Returns `true` if the step has completed.
    pub fn is_completed(&self) -> bool {
        self.completed_at.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_state_terminal() {
        assert!(WorkflowState::Completed.is_terminal());
        assert!(WorkflowState::Failed.is_terminal());
        assert!(!WorkflowState::Running.is_terminal());
        assert!(!WorkflowState::Pending.is_terminal());
        assert!(!WorkflowState::WaitingForRevision.is_terminal());
    }

    #[test]
    fn test_workflow_state_active() {
        assert!(WorkflowState::Running.is_active());
        assert!(WorkflowState::WaitingForRevision.is_active());
        assert!(!WorkflowState::Completed.is_active());
        assert!(!WorkflowState::Failed.is_active());
        assert!(!WorkflowState::Pending.is_active());
    }

    #[test]
    fn test_workflow_state_display() {
        assert_eq!(WorkflowState::Pending.to_string(), "pending");
        assert_eq!(WorkflowState::Running.to_string(), "running");
        assert_eq!(
            WorkflowState::WaitingForRevision.to_string(),
            "waiting_for_revision"
        );
        assert_eq!(WorkflowState::Completed.to_string(), "completed");
        assert_eq!(WorkflowState::Failed.to_string(), "failed");
    }

    #[test]
    fn test_step_metadata_new() {
        let step_id = StepId::new("test");
        let metadata = StepMetadata::new(step_id.clone());

        assert_eq!(metadata.step_id, step_id);
        assert_eq!(metadata.attempt, 1);
        assert!(metadata.completed_at.is_none());
        assert!(!metadata.is_completed());
    }

    #[test]
    fn test_step_metadata_mark_completed() {
        let mut metadata = StepMetadata::new(StepId::new("test"));
        assert!(!metadata.is_completed());

        metadata.mark_completed();
        assert!(metadata.is_completed());
        assert!(metadata.completed_at.is_some());
        assert!(metadata.duration().is_some());
    }

    #[test]
    fn test_step_metadata_serialization() {
        let metadata = StepMetadata::new(StepId::new("test"));
        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: StepMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(metadata.step_id, deserialized.step_id);
        assert_eq!(metadata.attempt, deserialized.attempt);
    }
}
```

#### Step 4: Update Module Structure

**`crates/ecl-core/src/types/mod.rs`**

```rust
//! Core types for ECL workflows.

mod ids;
mod step_result;
mod workflow_state;

pub use ids::{StepId, WorkflowId};
pub use step_result::StepResult;
pub use workflow_state::{StepMetadata, WorkflowState};
```

**Update `crates/ecl-core/src/lib.rs`**

```rust
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! ECL Core Library
//!
//! Core types, traits, and utilities for the ECL workflow orchestration system.

pub mod error;
pub mod types;

// Re-exports for convenience
pub use error::{Error, Result};
pub use types::{StepId, StepMetadata, StepResult, WorkflowId, WorkflowState};
```

### Testing Strategy

**Test coverage goal:** ≥95% for all types and error handling

**Tests to write:**

1. **Error type tests** (see inline tests above):
   - Error display formatting
   - Retryability classification
   - Error variant construction
   - Send + Sync bounds
   - Serialization (if needed)

2. **ID type tests**:
   - UUID generation uniqueness
   - String parsing
   - Display formatting
   - Serialization roundtrips

3. **StepResult tests**:
   - All variant constructors
   - Is methods
   - Map/unwrap operations
   - Serialization

4. **WorkflowState tests**:
   - State transitions
   - Terminal/active checks
   - Display formatting

**Test data needed:**
- Sample UUIDs
- Sample error messages
- Sample step IDs

**Additional tests:**

```rust
// Property-based tests for ID generation
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_workflow_id_roundtrip(uuid in any::<u128>()) {
            let uuid = Uuid::from_u128(uuid);
            let id = WorkflowId::from_uuid(uuid);
            assert_eq!(id.into_uuid(), uuid);
        }

        #[test]
        fn test_step_id_roundtrip(s in "\\PC+") {
            let id = StepId::new(s.clone());
            assert_eq!(id.as_str(), &s);
        }
    }
}
```

### Potential Blockers

1. **Serde serialization issues**
   - **Mitigation**: Test serialization early
   - **Mitigation**: Use standard derives

2. **Type complexity**
   - **Mitigation**: Keep types simple and focused
   - **Mitigation**: Use builder pattern if needed

### Acceptance Criteria

- [ ] All types are `Clone`, `Debug`, `Serialize`, `Deserialize` where appropriate
- [ ] Error types implement `std::error::Error` with `thiserror`
- [ ] All types have comprehensive unit tests
- [ ] Test coverage ≥95% for this module
- [ ] All types are Send + Sync (where applicable)
- [ ] Public types use `#[non_exhaustive]`
- [ ] No `unwrap()` in library code (only in tests)
- [ ] Documentation exists for all public items

### Estimated Effort

**Time:** 3-4 hours

**Breakdown:**
- Error types: 1 hour
- ID types: 45 min
- StepResult/WorkflowState: 1 hour
- Testing: 1.5 hours

---

