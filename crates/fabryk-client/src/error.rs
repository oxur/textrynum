//! Error types for fabryk-client

use thiserror::Error;

/// Result type alias for fabryk-client operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in fabryk-client
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Error from fabryk-core
    #[error("Core error: {0}")]
    Core(#[from] fabryk_core::Error),

    /// HTTP client error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Placeholder error variant
    #[error("Not yet implemented: {0}")]
    NotImplemented(&'static str),
}
