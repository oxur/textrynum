//! Redis client abstraction for Fabryk.
//!
//! Provides:
//! - [`RedisOps`] — Async trait abstracting Redis operations for testability
//! - [`RedisClient`] — Production client wrapping `redis::aio::ConnectionManager`
//! - [`MockRedis`] — In-memory mock for tests
//! - JSON convenience functions ([`set_json`], [`get_json`], [`get_multi_json`])
//!
//! The trait uses concrete string-based methods (dyn-compatible). Free
//! functions provide typed serialization/deserialization on top.

mod client;
mod error;
mod mock;

pub use client::RedisClient;
pub use error::RedisError;
pub use mock::MockRedis;

use async_trait::async_trait;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Trait abstracting Redis operations for testability.
///
/// Production code uses [`RedisClient`], tests use [`MockRedis`].
/// All methods use concrete types so the trait is dyn-compatible (`dyn RedisOps`).
#[async_trait]
pub trait RedisOps: Send + Sync + std::fmt::Debug {
    /// Store a string value at the given key.
    async fn set_str(&self, key: &str, value: &str) -> Result<(), RedisError>;

    /// Retrieve a string value from the given key. Returns `None` if key doesn't exist.
    async fn get_str(&self, key: &str) -> Result<Option<String>, RedisError>;

    /// Increment a key's integer value by the given amount. Returns the new value.
    async fn incr_by(&self, key: &str, amount: u64) -> Result<u64, RedisError>;

    /// Get a key's integer value. Returns 0 if key doesn't exist.
    async fn get_u64(&self, key: &str) -> Result<u64, RedisError>;

    /// Scan for keys matching a glob pattern.
    async fn scan_keys(&self, pattern: &str) -> Result<Vec<String>, RedisError>;

    /// Get multiple string values by key. Missing keys are skipped.
    async fn get_multi_str(&self, keys: &[String]) -> Result<Vec<String>, RedisError>;

    /// Check Redis connectivity.
    async fn health_check(&self) -> Result<(), RedisError>;
}

// ── JSON convenience functions ──────────────────────────────────────

/// Store a JSON-serializable value at the given key.
pub async fn set_json<T: Serialize>(
    redis: &dyn RedisOps,
    key: &str,
    value: &T,
) -> Result<(), RedisError> {
    let json =
        serde_json::to_string(value).map_err(|e| RedisError::Serialization(e.to_string()))?;
    redis.set_str(key, &json).await
}

/// Retrieve and deserialize a JSON value from the given key.
/// Returns `None` if the key doesn't exist.
pub async fn get_json<T: DeserializeOwned>(
    redis: &dyn RedisOps,
    key: &str,
) -> Result<Option<T>, RedisError> {
    match redis.get_str(key).await? {
        Some(s) => {
            let parsed =
                serde_json::from_str(&s).map_err(|e| RedisError::Serialization(e.to_string()))?;
            Ok(Some(parsed))
        }
        None => Ok(None),
    }
}

/// Get multiple JSON values by key. Missing keys and parse failures are skipped.
pub async fn get_multi_json<T: DeserializeOwned>(
    redis: &dyn RedisOps,
    keys: &[String],
) -> Result<Vec<T>, RedisError> {
    let strings = redis.get_multi_str(keys).await?;
    let mut results = Vec::new();
    for s in strings {
        if let Ok(parsed) = serde_json::from_str(&s) {
            results.push(parsed);
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_redis_set_get_json() {
        let redis = MockRedis::new();
        set_json(&redis, "key1", &"hello").await.unwrap();
        let val: Option<String> = get_json(&redis, "key1").await.unwrap();
        assert_eq!(val, Some("hello".to_string()));
    }

    #[tokio::test]
    async fn test_mock_redis_get_missing_key() {
        let redis = MockRedis::new();
        let val: Option<String> = get_json(&redis, "missing").await.unwrap();
        assert!(val.is_none());
    }

    #[tokio::test]
    async fn test_mock_redis_incr_by() {
        let redis = MockRedis::new();
        let v1 = redis.incr_by("counter", 10).await.unwrap();
        assert_eq!(v1, 10);
        let v2 = redis.incr_by("counter", 5).await.unwrap();
        assert_eq!(v2, 15);
    }

    #[tokio::test]
    async fn test_mock_redis_get_u64_default() {
        let redis = MockRedis::new();
        assert_eq!(redis.get_u64("missing").await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_mock_redis_scan_keys() {
        let redis = MockRedis::new();
        set_json(&redis, "prefix:alice:1", &"v1").await.unwrap();
        set_json(&redis, "prefix:alice:2", &"v2").await.unwrap();
        set_json(&redis, "prefix:bob:1", &"v3").await.unwrap();
        let keys = redis.scan_keys("prefix:alice:*").await.unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_redis_get_multi_json() {
        let redis = MockRedis::new();
        set_json(&redis, "k1", &"a").await.unwrap();
        set_json(&redis, "k2", &"b").await.unwrap();
        let vals: Vec<String> = get_multi_json(
            &redis,
            &["k1".to_string(), "k2".to_string(), "k3".to_string()],
        )
        .await
        .unwrap();
        assert_eq!(vals.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_redis_health_check() {
        let redis = MockRedis::new();
        assert!(redis.health_check().await.is_ok());
    }
}
