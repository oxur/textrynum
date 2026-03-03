//! Production Redis client wrapping `ConnectionManager`.

use async_trait::async_trait;

use crate::RedisOps;
use crate::error::RedisError;

/// Production Redis client with automatic reconnection.
#[derive(Clone, Debug)]
pub struct RedisClient {
    conn: redis::aio::ConnectionManager,
}

impl RedisClient {
    /// Connect to Redis at the given URL.
    pub async fn new(url: &str) -> Result<Self, RedisError> {
        let client = redis::Client::open(url).map_err(|e| RedisError::Connection(e.to_string()))?;
        let conn = redis::aio::ConnectionManager::new(client)
            .await
            .map_err(|e| RedisError::Connection(e.to_string()))?;
        Ok(Self { conn })
    }
}

#[async_trait]
impl RedisOps for RedisClient {
    async fn set_str(&self, key: &str, value: &str) -> Result<(), RedisError> {
        redis::cmd("SET")
            .arg(key)
            .arg(value)
            .exec_async(&mut self.conn.clone())
            .await
            .map_err(|e| RedisError::Command(e.to_string()))
    }

    async fn get_str(&self, key: &str) -> Result<Option<String>, RedisError> {
        redis::cmd("GET")
            .arg(key)
            .query_async(&mut self.conn.clone())
            .await
            .map_err(|e| RedisError::Command(e.to_string()))
    }

    async fn incr_by(&self, key: &str, amount: u64) -> Result<u64, RedisError> {
        redis::cmd("INCRBY")
            .arg(key)
            .arg(amount)
            .query_async(&mut self.conn.clone())
            .await
            .map_err(|e| RedisError::Command(e.to_string()))
    }

    async fn get_u64(&self, key: &str) -> Result<u64, RedisError> {
        let val: Option<String> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut self.conn.clone())
            .await
            .map_err(|e| RedisError::Command(e.to_string()))?;
        match val {
            Some(s) => s
                .parse::<u64>()
                .map_err(|e| RedisError::Serialization(e.to_string())),
            None => Ok(0),
        }
    }

    async fn scan_keys(&self, pattern: &str) -> Result<Vec<String>, RedisError> {
        let mut conn = self.conn.clone();
        let mut keys = Vec::new();
        let mut iter: redis::AsyncIter<String> = redis::cmd("SCAN")
            .cursor_arg(0)
            .arg("MATCH")
            .arg(pattern)
            .arg("COUNT")
            .arg(100)
            .clone()
            .iter_async(&mut conn)
            .await
            .map_err(|e| RedisError::Command(e.to_string()))?;
        while let Some(key) = iter.next_item().await {
            keys.push(key.map_err(|e| RedisError::Command(e.to_string()))?);
        }
        Ok(keys)
    }

    async fn get_multi_str(&self, keys: &[String]) -> Result<Vec<String>, RedisError> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        let values: Vec<Option<String>> = redis::cmd("MGET")
            .arg(keys)
            .query_async(&mut self.conn.clone())
            .await
            .map_err(|e| RedisError::Command(e.to_string()))?;
        Ok(values.into_iter().flatten().collect())
    }

    async fn health_check(&self) -> Result<(), RedisError> {
        let pong: String = redis::cmd("PING")
            .query_async(&mut self.conn.clone())
            .await
            .map_err(|e| RedisError::Command(e.to_string()))?;
        if pong == "PONG" {
            Ok(())
        } else {
            Err(RedisError::Command(format!(
                "unexpected PING response: {pong}"
            )))
        }
    }
}
