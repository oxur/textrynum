//! Error types for the Slack stub adapter.

use thiserror::Error;

/// Errors specific to the Slack adapter.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SlackAdapterError {
    /// An I/O error occurred during fixture file operations.
    #[error("slack adapter I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A fixture file could not be parsed.
    #[error("fixture parse error for '{item_id}': {message}")]
    FixtureParse {
        /// The item that failed to parse.
        item_id: String,
        /// Error detail.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slack_adapter_error_display_io() {
        let err = SlackAdapterError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_slack_adapter_error_display_fixture_parse() {
        let err = SlackAdapterError::FixtureParse {
            item_id: "C001:msg-001".to_string(),
            message: "invalid JSON".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("C001:msg-001"));
        assert!(msg.contains("invalid JSON"));
    }

    #[test]
    fn test_slack_adapter_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err: SlackAdapterError = io_err.into();
        assert!(matches!(err, SlackAdapterError::Io(_)));
    }

    #[test]
    fn test_slack_adapter_error_implements_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SlackAdapterError>();
    }
}
