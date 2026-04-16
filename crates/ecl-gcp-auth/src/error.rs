//! Error types for Google Cloud authentication.

use thiserror::Error;

/// Errors that can occur during Google Cloud authentication.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GcpAuthError {
    /// Authentication failed (JWT signing, token exchange, etc.).
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

    /// HTTP request failed.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON parsing failed.
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    /// I/O error (reading credential files, etc.).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_display() {
        let err = GcpAuthError::Auth {
            message: "token expired".to_string(),
        };
        assert!(err.to_string().contains("token expired"));
    }

    #[test]
    fn test_invalid_credentials_display() {
        let err = GcpAuthError::InvalidCredentials {
            message: "bad key".to_string(),
        };
        assert!(err.to_string().contains("bad key"));
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<GcpAuthError>();
    }
}
