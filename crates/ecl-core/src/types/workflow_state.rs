//! Workflow state tracking types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::StepId;

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
#[allow(clippy::unwrap_used)]
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
