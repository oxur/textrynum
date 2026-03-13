//! Error types for the topology layer.

use ecl_pipeline_state::StageId;
use thiserror::Error;

/// Errors that occur when resolving a pipeline topology from a specification.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ResolveError {
    /// The specification could not be serialized for hashing.
    #[error("failed to serialize spec: {message}")]
    SerializeError {
        /// The serialization error message.
        message: String,
    },

    /// A stage references a source that doesn't exist in the spec.
    #[error("stage '{stage}' references unknown source '{source_name}'")]
    UnknownSource {
        /// The stage that references the unknown source.
        stage: String,
        /// The source name that was referenced.
        source_name: String,
    },

    /// A stage references a resource that no other stage creates
    /// and is not a known external resource.
    #[error("stage '{stage}' reads resource '{resource}' which is never created")]
    MissingResource {
        /// The stage that reads the missing resource.
        stage: String,
        /// The resource name that is missing.
        resource: String,
    },

    /// The resource graph contains a cycle (impossible to schedule).
    #[error("resource dependency cycle detected involving stages: {stages:?}")]
    CycleDetected {
        /// Stage IDs involved in the cycle.
        stages: Vec<StageId>,
    },

    /// Multiple stages create the same resource.
    #[error("resource '{resource}' is created by multiple stages: {stages:?}")]
    DuplicateCreator {
        /// The resource with multiple creators.
        resource: String,
        /// The stages that create it.
        stages: Vec<StageId>,
    },

    /// An I/O error occurred (e.g., creating output directory).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// An unknown stage adapter was specified.
    #[error("unknown stage adapter '{adapter}' in stage '{stage}'")]
    UnknownAdapter {
        /// The stage with the unknown adapter.
        stage: String,
        /// The adapter name that was not recognized.
        adapter: String,
    },
}

/// Errors that occur in source adapters.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SourceError {
    /// Authentication failed.
    #[error("authentication failed for source '{source_name}': {message}")]
    AuthError {
        /// The source name.
        source_name: String,
        /// Error detail.
        message: String,
    },

    /// Rate limited by the source API.
    #[error("rate limited by '{source_name}': retry after {retry_after_secs}s")]
    RateLimited {
        /// The source name.
        source_name: String,
        /// Seconds to wait before retrying.
        retry_after_secs: u64,
    },

    /// An item was not found.
    #[error("item '{item_id}' not found in source '{source_name}'")]
    NotFound {
        /// The source name.
        source_name: String,
        /// The item ID that was not found.
        item_id: String,
    },

    /// A transient error (network, timeout) that may succeed on retry.
    #[error("transient error from source '{source_name}': {message}")]
    Transient {
        /// The source name.
        source_name: String,
        /// Error detail.
        message: String,
    },

    /// A permanent error that will not succeed on retry.
    #[error("permanent error from source '{source_name}': {message}")]
    Permanent {
        /// The source name.
        source_name: String,
        /// Error detail.
        message: String,
    },
}

/// Errors that occur during stage execution.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StageError {
    /// The stage received content it cannot process.
    #[error("stage '{stage}' cannot process item '{item_id}': {message}")]
    UnsupportedContent {
        /// The stage name.
        stage: String,
        /// The item ID.
        item_id: String,
        /// Error detail.
        message: String,
    },

    /// A transient error that may succeed on retry.
    #[error("transient error in stage '{stage}' for item '{item_id}': {message}")]
    Transient {
        /// The stage name.
        stage: String,
        /// The item ID.
        item_id: String,
        /// Error detail.
        message: String,
    },

    /// A permanent error that will not succeed on retry.
    #[error("permanent error in stage '{stage}' for item '{item_id}': {message}")]
    Permanent {
        /// The stage name.
        stage: String,
        /// The item ID.
        item_id: String,
        /// Error detail.
        message: String,
    },

    /// Stage execution timed out.
    #[error("stage '{stage}' timed out after {timeout_secs}s for item '{item_id}'")]
    Timeout {
        /// The stage name.
        stage: String,
        /// The item ID.
        item_id: String,
        /// The timeout duration in seconds.
        timeout_secs: u64,
    },
}

/// Result type for topology resolution operations.
pub type ResolveResult<T> = std::result::Result<T, ResolveError>;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_error_display_duplicate_creator() {
        let err = ResolveError::DuplicateCreator {
            resource: "raw-docs".to_string(),
            stages: vec![StageId::new("fetch"), StageId::new("extract")],
        };
        let msg = err.to_string();
        assert!(msg.contains("raw-docs"));
        assert!(msg.contains("fetch"));
        assert!(msg.contains("extract"));
    }

    #[test]
    fn test_resolve_error_display_cycle_detected() {
        let err = ResolveError::CycleDetected {
            stages: vec![StageId::new("a"), StageId::new("b")],
        };
        let msg = err.to_string();
        assert!(msg.contains("cycle"));
        assert!(msg.contains("a"));
        assert!(msg.contains("b"));
    }

    #[test]
    fn test_source_error_display_auth_error() {
        let err = SourceError::AuthError {
            source_name: "gdrive".to_string(),
            message: "token expired".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("gdrive"));
        assert!(msg.contains("token expired"));
    }

    #[test]
    fn test_stage_error_display_timeout() {
        let err = StageError::Timeout {
            stage: "normalize".to_string(),
            item_id: "doc-123".to_string(),
            timeout_secs: 60,
        };
        let msg = err.to_string();
        assert!(msg.contains("normalize"));
        assert!(msg.contains("doc-123"));
        assert!(msg.contains("60"));
    }

    #[test]
    fn test_resolve_error_implements_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ResolveError>();
    }

    #[test]
    fn test_source_error_implements_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SourceError>();
    }

    #[test]
    fn test_stage_error_implements_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<StageError>();
    }
}
