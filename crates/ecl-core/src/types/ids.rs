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

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
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
#[allow(clippy::unwrap_used)]
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
