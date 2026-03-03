//! Redis error types.

/// Errors that can occur during Redis operations.
#[derive(Debug, thiserror::Error)]
pub enum RedisError {
    /// Connection to Redis failed.
    #[error("Redis connection error: {0}")]
    Connection(String),

    /// A Redis command failed.
    #[error("Redis command error: {0}")]
    Command(String),

    /// Serialization or deserialization of a Redis value failed.
    #[error("Redis serialization error: {0}")]
    Serialization(String),
}
