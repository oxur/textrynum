//! Error types for GCP credential operations.

use thiserror::Error;

/// Errors that can occur during GCP credential resolution.
#[derive(Error, Debug)]
pub enum GcpError {
    /// No ADC file found on disk or via environment variable.
    #[error("no Application Default Credentials file found")]
    AdcNotFound,

    /// Credential file exists but cannot be read or parsed.
    #[error("credential file unreadable at {path}: {reason}")]
    CredentialRead { path: String, reason: String },

    /// Credential type is not suitable for the requested operation.
    #[error("credential type mismatch: expected {expected}, found {found}")]
    CredentialTypeMismatch { expected: String, found: String },
}

/// Result type alias for GCP operations.
pub type Result<T> = std::result::Result<T, GcpError>;
