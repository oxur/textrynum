//! Error types for the specification layer.

use thiserror::Error;

/// Errors that can occur when parsing or validating a pipeline specification.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SpecError {
    /// TOML parsing failed.
    #[error("failed to parse pipeline spec: {message}")]
    ParseError {
        /// The parse error message.
        message: String,
    },

    /// A stage references a source that doesn't exist.
    #[error("stage '{stage}' references unknown source '{source_name}'")]
    UnknownSource {
        /// The stage that references the unknown source.
        stage: String,
        /// The source name that was referenced.
        source_name: String,
    },

    /// Duplicate stage name detected.
    #[error("duplicate stage name: '{name}'")]
    DuplicateStage {
        /// The duplicate stage name.
        name: String,
    },

    /// Pipeline has no stages defined.
    #[error("pipeline has no stages defined")]
    EmptyPipeline,

    /// Pipeline has no sources defined.
    #[error("pipeline has no sources defined")]
    EmptySources,

    /// Validation error with a custom message.
    #[error("validation error: {message}")]
    ValidationError {
        /// Description of the validation failure.
        message: String,
    },
}

/// Result type for specification operations.
pub type Result<T> = std::result::Result<T, SpecError>;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_parse_error() {
        let err = SpecError::ParseError {
            message: "unexpected token".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "failed to parse pipeline spec: unexpected token"
        );
    }

    #[test]
    fn test_error_display_unknown_source() {
        let err = SpecError::UnknownSource {
            stage: "fetch-docs".to_string(),
            source_name: "missing-drive".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("fetch-docs"));
        assert!(msg.contains("missing-drive"));
        assert_eq!(
            msg,
            "stage 'fetch-docs' references unknown source 'missing-drive'"
        );
    }

    #[test]
    fn test_error_implements_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SpecError>();
    }

    #[test]
    fn test_error_display_duplicate_stage() {
        let err = SpecError::DuplicateStage {
            name: "fetch".to_string(),
        };
        assert_eq!(err.to_string(), "duplicate stage name: 'fetch'");
    }

    #[test]
    fn test_error_display_empty_pipeline() {
        let err = SpecError::EmptyPipeline;
        assert_eq!(err.to_string(), "pipeline has no stages defined");
    }

    #[test]
    fn test_error_display_empty_sources() {
        let err = SpecError::EmptySources;
        assert_eq!(err.to_string(), "pipeline has no sources defined");
    }

    #[test]
    fn test_error_display_validation_error() {
        let err = SpecError::ValidationError {
            message: "concurrency must be > 0".to_string(),
        };
        assert_eq!(err.to_string(), "validation error: concurrency must be > 0");
    }
}
