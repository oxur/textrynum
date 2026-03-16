//! Error types for the filesystem adapter.

use thiserror::Error;

/// Errors specific to the filesystem adapter.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum FsAdapterError {
    /// An I/O error occurred during file operations.
    #[error("filesystem I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A glob pattern was invalid.
    #[error("invalid glob pattern '{pattern}': {message}")]
    InvalidPattern {
        /// The invalid pattern.
        pattern: String,
        /// Error detail.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fs_adapter_error_display_io() {
        let err = FsAdapterError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_fs_adapter_error_display_invalid_pattern() {
        let err = FsAdapterError::InvalidPattern {
            pattern: "[bad".to_string(),
            message: "unclosed bracket".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("[bad"));
        assert!(msg.contains("unclosed bracket"));
    }

    #[test]
    fn test_fs_adapter_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err: FsAdapterError = io_err.into();
        assert!(matches!(err, FsAdapterError::Io(_)));
    }

    #[test]
    fn test_fs_adapter_error_implements_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<FsAdapterError>();
    }
}
