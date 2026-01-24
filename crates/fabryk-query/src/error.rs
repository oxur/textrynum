//! Error types for fabryk-query

use thiserror::Error;

/// Result type alias for fabryk-query operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in fabryk-query
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Error from fabryk-core
    #[error("Core error: {0}")]
    Core(#[from] fabryk_core::Error),

    /// Error from fabryk-storage
    #[error("Storage error: {0}")]
    Storage(#[from] fabryk_storage::Error),

    /// Placeholder error variant
    #[error("Not yet implemented: {0}")]
    NotImplemented(&'static str),
}
