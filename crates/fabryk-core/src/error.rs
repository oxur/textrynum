//! Error types for the Fabryk ecosystem.
//!
//! Provides a common `Error` type and `Result<T>` alias used across all Fabryk
//! crates. Uses `thiserror` for derive macros with backtrace support.
//!
//! # Error Categories
//!
//! - **I/O errors**: File operations, network, etc.
//! - **Configuration errors**: Invalid config, missing fields
//! - **Not found errors**: Missing resources (files, concepts, etc.)
//! - **Path errors**: Invalid paths, missing directories
//! - **Parse errors**: Malformed content, invalid format
//! - **Operation errors**: Generic operation failures
//!
//! # MCP Integration
//!
//! MCP-specific error mapping (converting to `ErrorData`) is provided by
//! `fabryk-mcp` via the `McpErrorExt` trait, keeping this crate free of
//! MCP dependencies.

use std::path::PathBuf;

use thiserror::Error;

/// Common error type for Fabryk operations.
///
/// All Fabryk crates use this error type or wrap it in their own domain-specific
/// error types. The variants cover common infrastructure errors; domain-specific
/// errors should use `Operation` with a descriptive message or wrap this type.
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error (file operations, network, etc.)
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// I/O error with path context.
    #[error("I/O error at {path}: {message}")]
    IoWithPath { path: PathBuf, message: String },

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// YAML serialization/deserialization error.
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// Resource not found (file, concept, source, etc.)
    #[error("{resource_type} not found: {id}")]
    NotFound { resource_type: String, id: String },

    /// File not found at specific path.
    #[error("File not found: {}", path.display())]
    FileNotFound { path: PathBuf },

    /// Invalid path.
    #[error("Invalid path {}: {reason}", path.display())]
    InvalidPath { path: PathBuf, reason: String },

    /// Parse error (malformed content, invalid format).
    #[error("Parse error: {0}")]
    Parse(String),

    /// Generic operation error (escape hatch for domain-specific errors).
    #[error("{0}")]
    Operation(String),
}

impl Error {
    // ========================================================================
    // Constructor helpers
    // ========================================================================

    /// Create an I/O error.
    ///
    /// This is useful when you have an `std::io::Error` and want to convert
    /// it explicitly (as opposed to using `?` with `From` conversion).
    pub fn io(err: std::io::Error) -> Self {
        Self::Io(err)
    }

    /// Create an I/O error with path context.
    pub fn io_with_path(err: std::io::Error, path: impl Into<PathBuf>) -> Self {
        Self::IoWithPath {
            path: path.into(),
            message: err.to_string(),
        }
    }

    /// Create a configuration error.
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a not-found error with resource type and ID.
    pub fn not_found(resource_type: impl Into<String>, id: impl Into<String>) -> Self {
        Self::NotFound {
            resource_type: resource_type.into(),
            id: id.into(),
        }
    }

    /// Create a file-not-found error.
    pub fn file_not_found(path: impl Into<PathBuf>) -> Self {
        Self::FileNotFound { path: path.into() }
    }

    /// Create a not-found error with just a message.
    ///
    /// This is a convenience method for when you don't have a specific
    /// resource type, or the message already contains the context.
    pub fn not_found_msg(msg: impl Into<String>) -> Self {
        Self::NotFound {
            resource_type: "Resource".to_string(),
            id: msg.into(),
        }
    }

    /// Create an invalid path error.
    pub fn invalid_path(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::InvalidPath {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a parse error.
    pub fn parse(msg: impl Into<String>) -> Self {
        Self::Parse(msg.into())
    }

    /// Create an operation error (generic domain-specific error).
    pub fn operation(msg: impl Into<String>) -> Self {
        Self::Operation(msg.into())
    }

    // ========================================================================
    // Inspector methods
    // ========================================================================

    /// Check if this is an I/O error.
    pub fn is_io(&self) -> bool {
        matches!(self, Self::Io(_) | Self::IoWithPath { .. })
    }

    /// Check if this is a not-found error (any variant).
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. } | Self::FileNotFound { .. })
    }

    /// Check if this is a configuration error.
    pub fn is_config(&self) -> bool {
        matches!(self, Self::Config(_))
    }

    /// Check if this is a path-related error.
    pub fn is_path_error(&self) -> bool {
        matches!(
            self,
            Self::InvalidPath { .. } | Self::FileNotFound { .. } | Self::IoWithPath { .. }
        )
    }

    /// Check if this is a parse error.
    pub fn is_parse(&self) -> bool {
        matches!(self, Self::Parse(_))
    }
}

/// Result type alias for Fabryk operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Walk the full error chain and log each cause at ERROR level.
///
/// Useful for diagnosing authentication failures, database errors, and other
/// deeply-nested error chains where the root cause is several layers deep.
///
/// # Example
///
/// ```rust,no_run
/// use fabryk_core::log_error_chain;
///
/// fn handle_error(err: &dyn std::error::Error) {
///     log::error!("Operation failed: {err}");
///     log_error_chain(err);
/// }
/// ```
pub fn log_error_chain(err: &dyn std::error::Error) {
    let mut depth = 0;
    let mut source = err.source();
    while let Some(cause) = source {
        depth += 1;
        log::error!("  cause[{depth}]: {cause}");
        log::debug!("  cause[{depth}] debug: {cause:?}");
        source = cause.source();
    }
    if depth == 0 {
        log::debug!("  (no further error sources in chain)");
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;

    // ------------------------------------------------------------------------
    // Constructor tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_error_io_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(err.is_io());
        assert!(!err.is_not_found());
        assert!(!err.is_config());
        assert!(err.to_string().contains("I/O error"));
    }

    #[test]
    fn test_error_io_constructor() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = Error::io(io_err);
        assert!(err.is_io());
        assert!(err.to_string().contains("I/O error"));
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_error_io_with_path() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let path = PathBuf::from("/test/path.txt");
        let err = Error::io_with_path(io_err, &path);
        assert!(err.is_io());
        assert!(err.is_path_error());
        let msg = err.to_string();
        assert!(msg.contains("I/O error at"));
        assert!(msg.contains("/test/path.txt"));
        assert!(msg.contains("permission denied"));
    }

    #[test]
    fn test_error_config() {
        let err = Error::config("invalid configuration");
        assert!(err.is_config());
        assert!(!err.is_io());
        assert!(!err.is_not_found());
        assert!(err.to_string().contains("Configuration error"));
        assert!(err.to_string().contains("invalid configuration"));
    }

    #[test]
    fn test_error_not_found() {
        let err = Error::not_found("Concept", "major-triad");
        assert!(err.is_not_found());
        assert!(!err.is_io());
        assert!(!err.is_config());
        let msg = err.to_string();
        assert!(msg.contains("Concept not found"));
        assert!(msg.contains("major-triad"));
    }

    #[test]
    fn test_error_file_not_found() {
        let path = PathBuf::from("/missing/file.txt");
        let err = Error::file_not_found(&path);
        assert!(err.is_not_found());
        assert!(err.is_path_error());
        assert!(!err.is_io());
        let msg = err.to_string();
        assert!(msg.contains("File not found"));
        assert!(msg.contains("/missing/file.txt"));
    }

    #[test]
    fn test_error_not_found_msg() {
        let err = Error::not_found_msg("file with id 'xyz' not in cache");
        assert!(err.is_not_found());
        assert!(!err.is_io());
        let msg = err.to_string();
        assert!(msg.contains("Resource not found"));
        assert!(msg.contains("file with id 'xyz' not in cache"));
    }

    #[test]
    fn test_error_invalid_path() {
        let path = PathBuf::from("/bad/path");
        let err = Error::invalid_path(&path, "invalid characters");
        assert!(err.is_path_error());
        assert!(!err.is_io());
        assert!(!err.is_not_found());
        let msg = err.to_string();
        assert!(msg.contains("Invalid path"));
        assert!(msg.contains("/bad/path"));
        assert!(msg.contains("invalid characters"));
    }

    #[test]
    fn test_error_parse() {
        let err = Error::parse("syntax error at line 5");
        assert!(err.is_parse());
        assert!(!err.is_io());
        assert!(!err.is_config());
        assert!(err.to_string().contains("Parse error"));
        assert!(err.to_string().contains("syntax error at line 5"));
    }

    #[test]
    fn test_error_operation() {
        let err = Error::operation("index corrupted");
        assert!(!err.is_io());
        assert!(!err.is_not_found());
        assert!(!err.is_config());
        assert!(err.to_string().contains("index corrupted"));
    }

    // ------------------------------------------------------------------------
    // From implementations
    // ------------------------------------------------------------------------

    #[test]
    fn test_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "connection lost");
        let err: Error = io_err.into();
        assert!(err.is_io());
        assert!(err.to_string().contains("connection lost"));
    }

    #[test]
    fn test_error_from_json_error() {
        let json_str = "{ invalid json }";
        let json_err = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let err: Error = json_err.into();
        assert!(matches!(err, Error::Json(_)));
        assert!(err.to_string().contains("JSON error"));
    }

    // ------------------------------------------------------------------------
    // Error trait implementation
    // ------------------------------------------------------------------------

    #[test]
    fn test_error_source_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let err: Error = io_err.into();
        assert!(err.source().is_some());
    }

    #[test]
    fn test_error_source_non_io() {
        let err = Error::config("test");
        assert!(err.source().is_none());
    }

    // ------------------------------------------------------------------------
    // Display tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_error_display_all_variants() {
        let errors = vec![
            Error::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "io")),
            Error::io_with_path(
                std::io::Error::new(std::io::ErrorKind::NotFound, "io"),
                "/path",
            ),
            Error::config("config"),
            Error::not_found("Type", "id"),
            Error::file_not_found("/path"),
            Error::invalid_path("/path", "reason"),
            Error::parse("parse"),
            Error::operation("operation"),
        ];

        for err in errors {
            let display = err.to_string();
            assert!(
                !display.is_empty(),
                "Display should produce non-empty string for {:?}",
                err
            );
        }
    }

    // ------------------------------------------------------------------------
    // Result alias
    // ------------------------------------------------------------------------

    #[test]
    fn test_result_alias() {
        let ok: Result<i32> = Ok(42);
        assert_eq!(ok.ok(), Some(42));

        let err: Result<i32> = Err(Error::config("bad"));
        assert!(err.is_err());
    }

    // ------------------------------------------------------------------------
    // log_error_chain tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_log_error_chain_no_source() {
        // Should not panic when error has no source
        let err = Error::config("standalone error");
        log_error_chain(&err);
    }

    #[test]
    fn test_log_error_chain_with_source() {
        // Should not panic when error has a source chain
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "inner cause");
        let err: Error = io_err.into();
        log_error_chain(&err);
    }

    #[test]
    fn test_log_error_chain_nested() {
        // Two-level chain: Error::Io wraps std::io::Error
        let inner = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection refused");
        let err: Error = inner.into();
        // Verify it doesn't panic with a chained error
        log_error_chain(&err);
    }
}
