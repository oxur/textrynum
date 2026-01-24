//! Error types for fabryk-mcp

use thiserror::Error;

/// Result type alias for fabryk-mcp operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in fabryk-mcp
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Error from fabryk-core
    #[error("Core error: {0}")]
    Core(#[from] fabryk_core::Error),

    /// Error from fabryk-client
    #[error("Client error: {0}")]
    Client(#[from] fabryk_client::Error),

    /// Placeholder error variant
    #[error("Not yet implemented: {0}")]
    NotImplemented(&'static str),
}
