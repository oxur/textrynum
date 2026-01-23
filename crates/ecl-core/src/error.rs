//! Error types for ECL core library.

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
        /// Source error if available
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
            Error::Llm { .. } => true, // LLM errors are generally retryable
            Error::Io(_) => true,
            Error::Database(_) => true, // Database errors may be transient
            Error::Timeout { .. } => true,
            Error::Validation { .. } => false, // Validation errors are permanent
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
#[allow(clippy::unwrap_used)]
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
        let Error::Validation { field, message } = err else {
            unreachable!("Expected Validation error variant");
        };
        assert_eq!(field, Some("topic".to_string()));
        assert_eq!(message, "must not be empty");
    }

    #[test]
    fn test_max_revisions_exceeded() {
        let err = Error::MaxRevisionsExceeded { attempts: 5 };
        assert_eq!(err.to_string(), "Maximum revisions exceeded: 5 attempts");
    }

    #[test]
    fn test_error_implements_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Error>();
    }

    #[test]
    fn test_config_error() {
        let err = Error::config("Invalid API endpoint");
        assert_eq!(err.to_string(), "Configuration error: Invalid API endpoint");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_workflow_not_found() {
        let err = Error::WorkflowNotFound {
            id: "wf-123".to_string(),
        };
        assert_eq!(err.to_string(), "Workflow not found: wf-123");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_step_not_found() {
        let err = Error::StepNotFound {
            id: "step-456".to_string(),
        };
        assert_eq!(err.to_string(), "Step not found: step-456");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_llm_error_with_source() {
        let io_error = std::io::Error::other("network failure");
        let err = Error::llm_with_source("API call failed", Box::new(io_error));
        assert!(err.to_string().contains("API call failed"));
        assert!(err.is_retryable());
    }

    #[test]
    fn test_io_error_is_retryable() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_error.into();
        assert!(err.is_retryable());
    }

    #[test]
    fn test_serde_error_not_retryable() {
        let json = "{invalid json}";
        let serde_err = serde_json::from_str::<serde_json::Value>(json).unwrap_err();
        let err: Error = serde_err.into();
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_timeout_error_display() {
        let err = Error::Timeout { seconds: 60 };
        assert_eq!(err.to_string(), "Step timed out after 60s");
    }

    #[test]
    fn test_validation_without_field() {
        let err = Error::validation("Generic validation failure");
        let Error::Validation { field, message } = err else {
            unreachable!("Expected Validation error");
        };
        assert_eq!(field, None);
        assert_eq!(message, "Generic validation failure");
    }
}
