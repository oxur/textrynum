//! Error types for fabryk-storage

use thiserror::Error;

/// Result type alias for fabryk-storage operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in fabryk-storage
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Error from fabryk-core
    #[error("Core error: {0}")]
    Core(#[from] fabryk_core::Error),

    /// Placeholder error variant
    #[error("Not yet implemented: {0}")]
    NotImplemented(&'static str),
}
