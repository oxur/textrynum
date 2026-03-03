//! In-memory mock Redis for testing.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::RedisOps;
use crate::error::RedisError;

/// In-memory mock Redis backed by a `HashMap<String, String>`.
///
/// Useful for unit tests that need Redis operations without a real server.
#[derive(Debug)]
pub struct MockRedis {
    store: Mutex<HashMap<String, String>>,
}

impl MockRedis {
    /// Create an empty mock Redis store.
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockRedis {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RedisOps for MockRedis {
    async fn set_str(&self, key: &str, value: &str) -> Result<(), RedisError> {
        self.store
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    async fn get_str(&self, key: &str) -> Result<Option<String>, RedisError> {
        let store = self.store.lock().unwrap();
        Ok(store.get(key).cloned())
    }

    async fn incr_by(&self, key: &str, amount: u64) -> Result<u64, RedisError> {
        let mut store = self.store.lock().unwrap();
        let current: u64 = store.get(key).and_then(|s| s.parse().ok()).unwrap_or(0);
        let new_val = current + amount;
        store.insert(key.to_string(), new_val.to_string());
        Ok(new_val)
    }

    async fn get_u64(&self, key: &str) -> Result<u64, RedisError> {
        let store = self.store.lock().unwrap();
        match store.get(key) {
            Some(s) => s
                .parse::<u64>()
                .map_err(|e| RedisError::Serialization(e.to_string())),
            None => Ok(0),
        }
    }

    async fn scan_keys(&self, pattern: &str) -> Result<Vec<String>, RedisError> {
        let store = self.store.lock().unwrap();
        // Simple glob match: convert trailing * to prefix match.
        let prefix = pattern.trim_end_matches('*');
        let keys: Vec<String> = store
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        Ok(keys)
    }

    async fn get_multi_str(&self, keys: &[String]) -> Result<Vec<String>, RedisError> {
        let store = self.store.lock().unwrap();
        let mut results = Vec::new();
        for key in keys {
            if let Some(val) = store.get(key) {
                results.push(val.clone());
            }
        }
        Ok(results)
    }

    async fn health_check(&self) -> Result<(), RedisError> {
        Ok(())
    }
}
