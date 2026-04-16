//! Error types for the Google Drive adapter.

use thiserror::Error;

/// Errors that can occur in the Google Drive adapter.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DriveAdapterError {
    /// Authentication failed.
    #[error("authentication failed: {message}")]
    Auth {
        /// Error detail.
        message: String,
    },

    /// Credentials file is invalid or unreadable.
    #[error("invalid credentials: {message}")]
    InvalidCredentials {
        /// Error detail.
        message: String,
    },

    /// A glob filter pattern is invalid.
    #[error("invalid glob pattern '{pattern}': {message}")]
    InvalidPattern {
        /// The invalid pattern.
        pattern: String,
        /// Error detail.
        message: String,
    },

    /// The Drive API returned an error response.
    #[error("Drive API error ({status}): {message}")]
    ApiError {
        /// HTTP status code.
        status: u16,
        /// Error detail.
        message: String,
    },

    /// HTTP request failed.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON parsing failed.
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    /// I/O error (reading credential files, etc.).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Shared GCP authentication error.
    #[error(transparent)]
    GcpAuth(#[from] ecl_gcp_auth::GcpAuthError),
}

/// Result type alias for Drive adapter operations.
pub type Result<T> = std::result::Result<T, DriveAdapterError>;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_display() {
        let err = DriveAdapterError::Auth {
            message: "token expired".to_string(),
        };
        assert!(err.to_string().contains("token expired"));
    }

    #[test]
    fn test_api_error_display() {
        let err = DriveAdapterError::ApiError {
            status: 403,
            message: "forbidden".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("403"));
        assert!(msg.contains("forbidden"));
    }

    #[test]
    fn test_invalid_pattern_display() {
        let err = DriveAdapterError::InvalidPattern {
            pattern: "[bad".to_string(),
            message: "unclosed bracket".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("[bad"));
        assert!(msg.contains("unclosed bracket"));
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DriveAdapterError>();
    }
}
