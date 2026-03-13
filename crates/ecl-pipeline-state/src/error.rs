//! Error types for the state layer.

use thiserror::Error;

/// Errors that can occur in the state layer.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StateError {
    /// Serialization or deserialization failed.
    #[error("serialization error: {message}")]
    SerializationError {
        /// The serialization error message.
        message: String,
    },

    /// State store I/O failed.
    #[error("store I/O error: {message}")]
    StoreError {
        /// The I/O error message.
        message: String,
    },

    /// Checkpoint version is unsupported.
    #[error("unsupported checkpoint version: {version}")]
    UnsupportedVersion {
        /// The unsupported version number.
        version: u32,
    },

    /// Config drift detected on resume.
    #[error("config drift detected: spec hash changed from {expected} to {actual}")]
    ConfigDrift {
        /// The expected spec hash (from checkpoint).
        expected: String,
        /// The actual spec hash (from current TOML).
        actual: String,
    },

    /// Checkpoint not found.
    #[error("no checkpoint found")]
    NotFound,
}

/// Result type for state operations.
pub type Result<T> = std::result::Result<T, StateError>;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_serialization_error() {
        let err = StateError::SerializationError {
            message: "invalid JSON".to_string(),
        };
        assert_eq!(err.to_string(), "serialization error: invalid JSON");
    }

    #[test]
    fn test_error_display_store_error() {
        let err = StateError::StoreError {
            message: "disk full".to_string(),
        };
        assert_eq!(err.to_string(), "store I/O error: disk full");
    }

    #[test]
    fn test_error_display_config_drift() {
        let err = StateError::ConfigDrift {
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("abc123"));
        assert!(msg.contains("def456"));
    }

    #[test]
    fn test_error_implements_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<StateError>();
    }

    #[test]
    fn test_error_display_unsupported_version() {
        let err = StateError::UnsupportedVersion { version: 99 };
        assert_eq!(err.to_string(), "unsupported checkpoint version: 99");
    }

    #[test]
    fn test_error_display_not_found() {
        let err = StateError::NotFound;
        assert_eq!(err.to_string(), "no checkpoint found");
    }
}
