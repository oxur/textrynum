//! Error types for the Zapier webhook adapter.

use thiserror::Error;

/// Errors specific to the Zapier webhook adapter.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ZapierAdapterError {
    /// The webhook HTTP server failed to bind to the configured address.
    #[error("server bind failed on {bind_addr}: {message}")]
    BindFailed {
        /// The address that failed to bind.
        bind_addr: String,
        /// Error detail.
        message: String,
    },

    /// The internal channel was closed unexpectedly.
    #[error("channel closed unexpectedly")]
    ChannelClosed,

    /// The webhook payload could not be parsed.
    #[error("invalid payload from {source_hint}: {message}")]
    InvalidPayload {
        /// The source hint that was used for parsing.
        source_hint: String,
        /// Error detail.
        message: String,
    },

    /// Webhook authentication failed.
    #[error("authentication failed")]
    AuthFailed,

    /// An I/O error occurred.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Result type alias for Zapier adapter operations.
pub type Result<T> = std::result::Result<T, ZapierAdapterError>;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_bind_failed() {
        let err = ZapierAdapterError::BindFailed {
            bind_addr: "127.0.0.1:9090".to_string(),
            message: "address in use".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("127.0.0.1:9090"));
        assert!(msg.contains("address in use"));
    }

    #[test]
    fn test_error_display_invalid_payload() {
        let err = ZapierAdapterError::InvalidPayload {
            source_hint: "granola".to_string(),
            message: "missing field 'title'".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("granola"));
        assert!(msg.contains("missing field"));
    }

    #[test]
    fn test_error_implements_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ZapierAdapterError>();
    }
}
