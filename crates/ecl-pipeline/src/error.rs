//! Error types for the pipeline execution engine.

use ecl_pipeline_state::StateError;
use ecl_pipeline_topo::{ResolveError, SourceError, StageError};
use thiserror::Error;

/// Errors that can occur during pipeline execution.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PipelineError {
    /// Source enumeration failed.
    #[error("source enumeration failed for '{source_name}': {detail}")]
    SourceEnumeration {
        /// The source that failed enumeration.
        source_name: String,
        /// Error detail.
        detail: String,
    },

    /// Stage execution failed (non-retryable or exhausted retries).
    #[error("stage execution failed: {0}")]
    StageExecution(#[from] StageError),

    /// State store operation failed.
    #[error("state store error: {0}")]
    StateStore(#[from] StateError),

    /// Topology resolution failed.
    #[error("topology resolution error: {0}")]
    Resolve(#[from] ResolveError),

    /// Source adapter error.
    #[error("source error: {0}")]
    Source(#[from] SourceError),

    /// Config drift detected on resume: the TOML changed since the
    /// checkpoint was created.
    #[error(
        "config drift detected: checkpoint hash '{checkpoint_hash}' != current hash '{current_hash}'"
    )]
    ConfigDrift {
        /// The spec hash stored in the checkpoint.
        checkpoint_hash: String,
        /// The spec hash computed from the current TOML.
        current_hash: String,
    },

    /// A tokio JoinSet task panicked or was cancelled.
    #[error("task join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    /// Semaphore acquisition failed (should not happen in normal operation).
    #[error("semaphore acquire error: {0}")]
    SemaphoreError(#[from] tokio::sync::AcquireError),

    /// Push source error (webhook receiver, message queue, etc.).
    #[error("push source '{source_name}' error: {detail}")]
    PushSource {
        /// The push source that failed.
        source_name: String,
        /// Error detail.
        detail: String,
    },

    /// A batch contained a stage that failed and was not configured with
    /// skip_on_error.
    #[error("stage '{stage}' failed for item '{item_id}': {error}")]
    ItemFailed {
        /// The stage where the failure occurred.
        stage: String,
        /// The item that failed.
        item_id: String,
        /// Error detail.
        error: String,
    },
}

/// Result type for pipeline operations.
pub type Result<T> = std::result::Result<T, PipelineError>;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_source_enumeration() {
        let err = PipelineError::SourceEnumeration {
            source_name: "gdrive".to_string(),
            detail: "auth failed".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("gdrive"), "should contain source name");
        assert!(msg.contains("auth failed"), "should contain error detail");
    }

    #[test]
    fn test_error_display_config_drift() {
        let err = PipelineError::ConfigDrift {
            checkpoint_hash: "abc123".to_string(),
            current_hash: "def456".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("abc123"), "should contain checkpoint hash");
        assert!(msg.contains("def456"), "should contain current hash");
    }

    #[test]
    fn test_error_display_item_failed() {
        let err = PipelineError::ItemFailed {
            stage: "normalize".to_string(),
            item_id: "doc-1".to_string(),
            error: "parse error".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("normalize"), "should contain stage");
        assert!(msg.contains("doc-1"), "should contain item_id");
        assert!(msg.contains("parse error"), "should contain error");
    }

    #[test]
    fn test_error_implements_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PipelineError>();
    }

    #[test]
    fn test_error_from_state_error() {
        let state_err = StateError::NotFound;
        let pipeline_err: PipelineError = state_err.into();
        assert!(matches!(pipeline_err, PipelineError::StateStore(_)));
    }

    #[test]
    fn test_error_from_stage_error() {
        let stage_err = StageError::Permanent {
            stage: "test".to_string(),
            item_id: "item-1".to_string(),
            message: "boom".to_string(),
        };
        let pipeline_err: PipelineError = stage_err.into();
        assert!(matches!(pipeline_err, PipelineError::StageExecution(_)));
    }

    #[test]
    fn test_error_from_source_error() {
        let source_err = SourceError::Permanent {
            source_name: "gdrive".to_string(),
            message: "quota exceeded".to_string(),
        };
        let pipeline_err: PipelineError = source_err.into();
        assert!(matches!(pipeline_err, PipelineError::Source(_)));
    }
}
