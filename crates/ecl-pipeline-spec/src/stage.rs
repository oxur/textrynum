//! Stage specification types.

use crate::defaults::RetrySpec;
use serde::{Deserialize, Serialize};

/// A stage is "what work to perform."
/// Resource declarations determine execution order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageSpec {
    /// Which registered stage implementation to use.
    pub adapter: String,

    /// Which source this stage operates on (for extract stages).
    pub source: Option<String>,

    /// Resource access declarations (from dagga's model).
    /// The topology layer uses these to compute the parallel schedule.
    #[serde(default)]
    pub resources: ResourceSpec,

    /// Stage-specific parameters passed to the adapter.
    #[serde(default)]
    pub params: serde_json::Value,

    /// Override the default retry policy for this stage.
    pub retry: Option<RetrySpec>,

    /// Override the default timeout for this stage.
    pub timeout_secs: Option<u64>,

    /// If true, item-level failures skip the item rather than failing
    /// the pipeline.
    #[serde(default)]
    pub skip_on_error: bool,

    /// Optional predicate expression. When false, the stage is skipped.
    pub condition: Option<String>,
}

/// Resource access declarations in TOML-friendly form.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceSpec {
    /// Resources this stage reads (shared access).
    #[serde(default)]
    pub reads: Vec<String>,
    /// Resources this stage creates (produces for the first time).
    #[serde(default)]
    pub creates: Vec<String>,
    /// Resources this stage writes (exclusive access).
    #[serde(default)]
    pub writes: Vec<String>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_spec_serde_roundtrip() {
        let stage = StageSpec {
            adapter: "extract".to_string(),
            source: Some("my-source".to_string()),
            resources: ResourceSpec {
                reads: vec!["api-client".to_string()],
                creates: vec!["raw-docs".to_string()],
                writes: vec![],
            },
            params: serde_json::json!({"format": "markdown"}),
            retry: Some(RetrySpec {
                max_attempts: 5,
                initial_backoff_ms: 500,
                backoff_multiplier: 1.5,
                max_backoff_ms: 10_000,
            }),
            timeout_secs: Some(300),
            skip_on_error: true,
            condition: Some("source.items_discovered > 0".to_string()),
        };
        let json = serde_json::to_string(&stage).unwrap();
        let deserialized: StageSpec = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_resource_spec_default_is_empty() {
        let resources = ResourceSpec::default();
        assert!(resources.reads.is_empty());
        assert!(resources.creates.is_empty());
        assert!(resources.writes.is_empty());
    }

    #[test]
    fn test_stage_spec_params_json_value() {
        let stage = StageSpec {
            adapter: "emit".to_string(),
            source: None,
            resources: ResourceSpec::default(),
            params: serde_json::json!({
                "subdir": "normalized",
                "compress": true,
                "max_size": 1024
            }),
            retry: None,
            timeout_secs: None,
            skip_on_error: false,
            condition: None,
        };
        let json = serde_json::to_string(&stage).unwrap();
        let deserialized: StageSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.params["subdir"], "normalized");
        assert_eq!(deserialized.params["compress"], true);
        assert_eq!(deserialized.params["max_size"], 1024);
    }
}
