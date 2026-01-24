//! Error types for fabryk-api

use thiserror::Error;

/// Result type alias for fabryk-api operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in fabryk-api
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Error from fabryk-core
    #[error("Core error: {0}")]
    Core(#[from] fabryk_core::Error),

    /// Error from fabryk-acl
    #[error("ACL error: {0}")]
    Acl(#[from] fabryk_acl::Error),

    /// Error from fabryk-storage
    #[error("Storage error: {0}")]
    Storage(#[from] fabryk_storage::Error),

    /// Error from fabryk-query
    #[error("Query error: {0}")]
    Query(#[from] fabryk_query::Error),

    /// Placeholder error variant
    #[error("Not yet implemented: {0}")]
    NotImplemented(&'static str),
}
