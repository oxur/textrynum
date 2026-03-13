//! Global default configuration for the pipeline.

use serde::{Deserialize, Serialize};

/// Global defaults that apply across all sources/stages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsSpec {
    /// Maximum concurrent operations within a batch.
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,

    /// Default retry policy for transient failures.
    #[serde(default)]
    pub retry: RetrySpec,

    /// Default checkpoint strategy.
    #[serde(default)]
    pub checkpoint: CheckpointStrategy,
}

fn default_concurrency() -> usize {
    4
}

impl Default for DefaultsSpec {
    fn default() -> Self {
        Self {
            concurrency: default_concurrency(),
            retry: RetrySpec::default(),
            checkpoint: CheckpointStrategy::default(),
        }
    }
}

/// Retry policy configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrySpec {
    /// Total attempts (1 = no retry).
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,

    /// Initial backoff duration in milliseconds.
    #[serde(default = "default_initial_backoff")]
    pub initial_backoff_ms: u64,

    /// Multiplier applied to backoff after each attempt.
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,

    /// Maximum backoff duration in milliseconds.
    #[serde(default = "default_max_backoff")]
    pub max_backoff_ms: u64,
}

fn default_max_attempts() -> u32 {
    3
}
fn default_initial_backoff() -> u64 {
    1000
}
fn default_backoff_multiplier() -> f64 {
    2.0
}
fn default_max_backoff() -> u64 {
    30_000
}

impl Default for RetrySpec {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            initial_backoff_ms: default_initial_backoff(),
            backoff_multiplier: default_backoff_multiplier(),
            max_backoff_ms: default_max_backoff(),
        }
    }
}

/// When to write checkpoints during pipeline execution.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "every")]
pub enum CheckpointStrategy {
    /// Checkpoint after every stage batch completes (default).
    #[default]
    Batch,
    /// Checkpoint after every N items processed within a stage.
    Items {
        /// Number of items between checkpoints.
        count: usize,
    },
    /// Checkpoint on a time interval.
    Seconds {
        /// Interval duration in seconds.
        duration: u64,
    },
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_spec_default_values() {
        let retry = RetrySpec::default();
        assert_eq!(retry.max_attempts, 3);
        assert_eq!(retry.initial_backoff_ms, 1000);
        assert_eq!(retry.backoff_multiplier, 2.0);
        assert_eq!(retry.max_backoff_ms, 30_000);
    }

    #[test]
    fn test_defaults_spec_default_values() {
        let defaults = DefaultsSpec::default();
        assert_eq!(defaults.concurrency, 4);
    }

    #[test]
    fn test_checkpoint_strategy_default_is_batch() {
        let strategy = CheckpointStrategy::default();
        assert_eq!(strategy, CheckpointStrategy::Batch);
    }

    #[test]
    fn test_retry_spec_serde_roundtrip() {
        let retry = RetrySpec {
            max_attempts: 5,
            initial_backoff_ms: 500,
            backoff_multiplier: 1.5,
            max_backoff_ms: 60_000,
        };
        let json = serde_json::to_string(&retry).unwrap();
        let deserialized: RetrySpec = serde_json::from_str(&json).unwrap();
        assert_eq!(retry, deserialized);
    }

    #[test]
    fn test_defaults_spec_serde_roundtrip() {
        let defaults = DefaultsSpec {
            concurrency: 8,
            retry: RetrySpec {
                max_attempts: 5,
                initial_backoff_ms: 200,
                backoff_multiplier: 3.0,
                max_backoff_ms: 60_000,
            },
            checkpoint: CheckpointStrategy::Items { count: 50 },
        };
        let json = serde_json::to_string(&defaults).unwrap();
        let deserialized: DefaultsSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.concurrency, 8);
        assert_eq!(deserialized.retry, defaults.retry);
    }

    #[test]
    fn test_checkpoint_strategy_serde_roundtrip_all_variants() {
        let variants = vec![
            CheckpointStrategy::Batch,
            CheckpointStrategy::Items { count: 10 },
            CheckpointStrategy::Seconds { duration: 60 },
        ];
        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: CheckpointStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, deserialized);
        }
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_retry_spec_proptest_roundtrip(
            max_attempts in 1..100u32,
            initial_backoff_ms in 1..100_000u64,
            backoff_multiplier in 1.0..10.0f64,
            max_backoff_ms in 1..1_000_000u64,
        ) {
            let retry = RetrySpec {
                max_attempts,
                initial_backoff_ms,
                backoff_multiplier,
                max_backoff_ms,
            };
            let json = serde_json::to_string(&retry).unwrap();
            let deserialized: RetrySpec = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(retry.max_attempts, deserialized.max_attempts);
            prop_assert_eq!(retry.initial_backoff_ms, deserialized.initial_backoff_ms);
            prop_assert_eq!(retry.max_backoff_ms, deserialized.max_backoff_ms);
            // f64 may lose precision through JSON serialization
            prop_assert!((retry.backoff_multiplier - deserialized.backoff_multiplier).abs() < 1e-10);
        }

        #[test]
        fn test_defaults_spec_proptest_concurrency_roundtrip(
            concurrency in 1..256usize,
        ) {
            let defaults = DefaultsSpec {
                concurrency,
                retry: RetrySpec::default(),
                checkpoint: CheckpointStrategy::default(),
            };
            let json = serde_json::to_string(&defaults).unwrap();
            let deserialized: DefaultsSpec = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(concurrency, deserialized.concurrency);
        }
    }
}
