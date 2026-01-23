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
    #[allow(clippy::panic)]
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
#[allow(clippy::unwrap_used)]
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

    #[test]
    fn test_map_on_needs_revision() {
        let result: StepResult<i32> = StepResult::NeedsRevision {
            output: 5,
            feedback: "improve".to_string(),
        };
        let mapped = result.map(|x| x * 2);
        let StepResult::NeedsRevision { output, feedback } = mapped else {
            unreachable!("Expected NeedsRevision");
        };
        assert_eq!(output, 10);
        assert_eq!(feedback, "improve");
    }

    #[test]
    fn test_map_on_failed() {
        let result: StepResult<i32> = StepResult::Failed {
            error: "error".to_string(),
            retryable: true,
        };
        let mapped = result.map(|x| x * 2);
        let StepResult::Failed { error, retryable } = mapped else {
            unreachable!("Expected Failed");
        };
        assert_eq!(error, "error");
        assert!(retryable);
    }

    #[test]
    fn test_unwrap_or_on_needs_revision() {
        let result: StepResult<i32> = StepResult::NeedsRevision {
            output: 7,
            feedback: "feedback".to_string(),
        };
        assert_eq!(result.unwrap_or(0), 0);
    }

    #[test]
    fn test_clone() {
        let result = StepResult::Success(42);
        let cloned = result.clone();
        assert_eq!(result, cloned);
    }

    #[test]
    fn test_failed_not_retryable() {
        let result: StepResult<i32> = StepResult::Failed {
            error: "permanent error".to_string(),
            retryable: false,
        };
        assert!(!result.is_retryable());
    }

    #[test]
    fn test_serialization_needs_revision() {
        let result: StepResult<String> = StepResult::NeedsRevision {
            output: "draft".to_string(),
            feedback: "needs work".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: StepResult<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(result, deserialized);
    }

    #[test]
    fn test_serialization_failed() {
        let result: StepResult<String> = StepResult::Failed {
            error: "timeout".to_string(),
            retryable: true,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: StepResult<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(result, deserialized);
    }
}
