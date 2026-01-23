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
#[allow(clippy::unwrap_used)]
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

    #[test]
    fn test_pass_serialization() {
        let decision = CritiqueDecision::Pass;
        let json = serde_json::to_string(&decision).unwrap();
        let deserialized: CritiqueDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(decision, deserialized);
    }
}
