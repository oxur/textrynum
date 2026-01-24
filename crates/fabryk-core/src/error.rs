//! Error types for fabryk-core

use thiserror::Error;

/// Result type alias for fabryk-core operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in fabryk-core
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Placeholder error variant
    #[error("Not yet implemented: {0}")]
    NotImplemented(&'static str),
}
