//! Error types for the Google Cloud Storage adapter.

use thiserror::Error;

/// Errors that can occur in the GCS adapter.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GcsAdapterError {
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

    /// A glob pattern is invalid.
    #[error("invalid glob pattern '{pattern}': {message}")]
    InvalidPattern {
        /// The invalid pattern.
        pattern: String,
        /// Error detail.
        message: String,
    },

    /// The GCS JSON API returned an error response.
    #[error("GCS API error ({status}): {message}")]
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

/// Result type alias for GCS adapter operations.
pub type Result<T> = std::result::Result<T, GcsAdapterError>;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_display() {
        let err = GcsAdapterError::Auth {
            message: "token expired".to_string(),
        };
        assert!(err.to_string().contains("token expired"));
    }

    #[test]
    fn test_api_error_display() {
        let err = GcsAdapterError::ApiError {
            status: 403,
            message: "forbidden".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("403"));
        assert!(msg.contains("forbidden"));
    }

    #[test]
    fn test_invalid_pattern_display() {
        let err = GcsAdapterError::InvalidPattern {
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
        assert_send_sync::<GcsAdapterError>();
    }
}
