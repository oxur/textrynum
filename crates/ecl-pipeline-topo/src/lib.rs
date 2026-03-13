//! Pipeline topology, resource graph, and core traits for the ECL pipeline runner.
//!
//! This crate defines:
//! - The resolved pipeline topology (`PipelineTopology`, `ResolvedStage`)
//! - Resource graph computation and parallel schedule derivation
//! - Core traits (`SourceAdapter`, `Stage`) and their supporting types
//!   (`PipelineItem`, `SourceItem`, `ExtractedDocument`, `StageContext`)
//!
//! The topology is computed from a `PipelineSpec` (from `ecl-pipeline-spec`)
//! at init time and is immutable during execution.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod error;
pub mod resolve;
pub mod resource_graph;
pub mod schedule;
pub mod traits;

pub use error::{ResolveError, ResolveResult, SourceError, StageError};
pub use traits::{ExtractedDocument, PipelineItem, SourceAdapter, SourceItem, Stage, StageContext};

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use ecl_pipeline_spec::PipelineSpec;
use ecl_pipeline_state::{Blake3Hash, StageId};

/// The resolved pipeline, ready to execute.
/// Computed from `PipelineSpec` at init time. Immutable during execution.
#[derive(Debug, Clone)]
pub struct PipelineTopology {
    /// The original spec, preserved for checkpoint embedding.
    pub spec: Arc<PipelineSpec>,

    /// Blake3 hash of the serialized spec, for detecting config drift
    /// between a checkpoint and the current TOML file.
    pub spec_hash: Blake3Hash,

    /// Resolved source adapters, keyed by source name from the spec.
    pub sources: BTreeMap<String, Arc<dyn SourceAdapter>>,

    /// Resolved stage implementations, keyed by stage name from the spec.
    pub stages: BTreeMap<String, ResolvedStage>,

    /// The computed execution schedule: batches of parallel stages.
    /// Each inner `Vec` contains stages that can run concurrently.
    pub schedule: Vec<Vec<StageId>>,

    /// Resolved output directory (created if needed at init).
    pub output_dir: PathBuf,
}

/// A resolved stage: the concrete implementation with merged configuration.
#[derive(Debug, Clone)]
pub struct ResolvedStage {
    /// Name from the spec (stable across runs for checkpointing).
    pub id: StageId,

    /// The concrete stage implementation.
    pub handler: Arc<dyn Stage>,

    /// Resolved retry policy (stage override merged with global default).
    pub retry: RetryPolicy,

    /// Skip-on-error behavior for item-level failures.
    pub skip_on_error: bool,

    /// Timeout for stage execution.
    pub timeout: Option<Duration>,

    /// Which source this stage operates on (for extract stages).
    pub source: Option<String>,

    /// Condition predicate (`None` = always run).
    pub condition: Option<ConditionExpr>,
}

/// Retry policy with resolved, concrete values.
/// Unlike `RetrySpec` (from ecl-pipeline-spec) which stores milliseconds as
/// `u64`, this stores `Duration` values — the resolved form ready for use
/// by the runner.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetryPolicy {
    /// Total attempts (1 = no retry).
    pub max_attempts: u32,
    /// Initial backoff duration before the first retry.
    pub initial_backoff: Duration,
    /// Multiplier applied to backoff after each attempt.
    pub backoff_multiplier: f64,
    /// Maximum backoff duration (caps exponential growth).
    pub max_backoff: Duration,
}

impl RetryPolicy {
    /// Create a `RetryPolicy` from a `RetrySpec` (millisecond-based config form).
    pub fn from_spec(spec: &ecl_pipeline_spec::RetrySpec) -> Self {
        Self {
            max_attempts: spec.max_attempts,
            initial_backoff: Duration::from_millis(spec.initial_backoff_ms),
            backoff_multiplier: spec.backoff_multiplier,
            max_backoff: Duration::from_millis(spec.max_backoff_ms),
        }
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(1000),
            backoff_multiplier: 2.0,
            max_backoff: Duration::from_millis(30_000),
        }
    }
}

/// A condition expression that determines whether a stage should run.
/// Currently a simple string wrapper; the evaluator is deferred to a
/// future milestone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionExpr(String);

impl ConditionExpr {
    /// Create a new condition expression from a string.
    pub fn new(expr: impl Into<String>) -> Self {
        Self(expr.into())
    }

    /// Get the expression string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ConditionExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use ecl_pipeline_spec::RetrySpec;

    #[test]
    fn test_condition_expr_new_and_as_str() {
        let expr = ConditionExpr::new("x > 1");
        assert_eq!(expr.as_str(), "x > 1");
    }

    #[test]
    fn test_condition_expr_display() {
        let expr = ConditionExpr::new("x > 1");
        assert_eq!(format!("{expr}"), "x > 1");
    }

    #[test]
    fn test_condition_expr_serde_roundtrip() {
        let expr = ConditionExpr::new("items.count > 0");
        let json = serde_json::to_string(&expr).unwrap();
        let deserialized: ConditionExpr = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, deserialized);
    }

    #[test]
    fn test_retry_policy_default_values() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.initial_backoff, Duration::from_millis(1000));
        assert!((policy.backoff_multiplier - 2.0).abs() < f64::EPSILON);
        assert_eq!(policy.max_backoff, Duration::from_millis(30_000));
    }

    #[test]
    fn test_retry_policy_from_spec() {
        let spec = RetrySpec {
            max_attempts: 5,
            initial_backoff_ms: 500,
            backoff_multiplier: 1.5,
            max_backoff_ms: 10_000,
        };
        let policy = RetryPolicy::from_spec(&spec);
        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.initial_backoff, Duration::from_millis(500));
        assert!((policy.backoff_multiplier - 1.5).abs() < f64::EPSILON);
        assert_eq!(policy.max_backoff, Duration::from_millis(10_000));
    }

    #[test]
    fn test_retry_policy_serde_roundtrip() {
        let policy = RetryPolicy {
            max_attempts: 4,
            initial_backoff: Duration::from_millis(2000),
            backoff_multiplier: 3.0,
            max_backoff: Duration::from_millis(60_000),
        };
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: RetryPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy, deserialized);
    }

    // Mock types for structural tests

    #[derive(Debug)]
    struct MockSourceAdapter;

    #[async_trait]
    impl SourceAdapter for MockSourceAdapter {
        fn source_kind(&self) -> &str {
            "mock"
        }
        async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
            Ok(vec![])
        }
        async fn fetch(&self, _item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
            Err(SourceError::NotFound {
                source_name: "mock".to_string(),
                item_id: "none".to_string(),
            })
        }
    }

    #[derive(Debug)]
    struct MockStage;

    #[async_trait]
    impl Stage for MockStage {
        fn name(&self) -> &str {
            "mock"
        }
        async fn process(
            &self,
            item: PipelineItem,
            _ctx: &StageContext,
        ) -> Result<Vec<PipelineItem>, StageError> {
            Ok(vec![item])
        }
    }

    #[test]
    fn test_pipeline_topology_has_expected_fields() {
        let spec = Arc::new(
            PipelineSpec::from_toml(
                r#"
name = "test"
version = 1
output_dir = "./out"

[sources.local]
kind = "filesystem"
root = "/tmp"

[stages.extract]
adapter = "extract"
source = "local"
resources = { creates = ["docs"] }
"#,
            )
            .unwrap(),
        );

        let mut sources: BTreeMap<String, Arc<dyn SourceAdapter>> = BTreeMap::new();
        sources.insert("local".to_string(), Arc::new(MockSourceAdapter));

        let mut stages_map = BTreeMap::new();
        stages_map.insert(
            "extract".to_string(),
            ResolvedStage {
                id: StageId::new("extract"),
                handler: Arc::new(MockStage),
                retry: RetryPolicy::default(),
                skip_on_error: false,
                timeout: Some(Duration::from_secs(300)),
                source: Some("local".to_string()),
                condition: None,
            },
        );

        let topo = PipelineTopology {
            spec: spec.clone(),
            spec_hash: Blake3Hash::new("abc123"),
            sources,
            stages: stages_map,
            schedule: vec![vec![StageId::new("extract")]],
            output_dir: PathBuf::from("./out"),
        };

        assert_eq!(topo.spec.name, "test");
        assert_eq!(topo.schedule.len(), 1);
        assert_eq!(topo.sources.len(), 1);
        assert_eq!(topo.stages.len(), 1);
    }

    #[test]
    fn test_resolved_stage_has_expected_fields() {
        let stage = ResolvedStage {
            id: StageId::new("normalize"),
            handler: Arc::new(MockStage),
            retry: RetryPolicy::default(),
            skip_on_error: true,
            timeout: Some(Duration::from_secs(60)),
            source: Some("gdrive".to_string()),
            condition: Some(ConditionExpr::new("items.count > 0")),
        };

        assert_eq!(stage.id.as_str(), "normalize");
        assert!(stage.skip_on_error);
        assert_eq!(stage.timeout, Some(Duration::from_secs(60)));
        assert_eq!(stage.source, Some("gdrive".to_string()));
        assert_eq!(stage.condition, Some(ConditionExpr::new("items.count > 0")));
    }

    #[tokio::test]
    async fn test_mock_source_adapter_methods() {
        let adapter: Arc<dyn SourceAdapter> = Arc::new(MockSourceAdapter);
        assert_eq!(adapter.source_kind(), "mock");
        let items = adapter.enumerate().await.unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_mock_stage_process() {
        use ecl_pipeline_state::ItemProvenance;
        let stage: Arc<dyn Stage> = Arc::new(MockStage);
        assert_eq!(stage.name(), "mock");

        let item = PipelineItem {
            id: "test-item".to_string(),
            display_name: "test".to_string(),
            content: Arc::from(b"data" as &[u8]),
            mime_type: "text/plain".to_string(),
            source_name: "local".to_string(),
            source_content_hash: Blake3Hash::new("aabb"),
            provenance: ItemProvenance {
                source_kind: "filesystem".to_string(),
                metadata: BTreeMap::new(),
                source_modified: None,
                extracted_at: chrono::Utc::now(),
            },
            metadata: BTreeMap::new(),
        };

        let ctx = StageContext {
            spec: Arc::new(
                PipelineSpec::from_toml(
                    r#"
name = "test"
version = 1
output_dir = "./out"

[sources.local]
kind = "filesystem"
root = "/tmp"

[stages.extract]
adapter = "extract"
source = "local"
resources = { creates = ["docs"] }
"#,
                )
                .unwrap(),
            ),
            output_dir: PathBuf::from("./output"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        };

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "test-item");
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_retry_policy_proptest_roundtrip(
            max_attempts in 1..100u32,
            initial_ms in 1..100_000u64,
            multiplier in 1.0..10.0f64,
            max_ms in 1..1_000_000u64,
        ) {
            let policy = RetryPolicy {
                max_attempts,
                initial_backoff: Duration::from_millis(initial_ms),
                backoff_multiplier: multiplier,
                max_backoff: Duration::from_millis(max_ms),
            };
            let json = serde_json::to_string(&policy).unwrap();
            let deserialized: RetryPolicy = serde_json::from_str(&json).unwrap();
            // Compare fields individually due to f64 precision
            prop_assert_eq!(policy.max_attempts, deserialized.max_attempts);
            prop_assert_eq!(policy.initial_backoff, deserialized.initial_backoff);
            prop_assert_eq!(policy.max_backoff, deserialized.max_backoff);
            prop_assert!((policy.backoff_multiplier - deserialized.backoff_multiplier).abs() < 1e-10);
        }

        #[test]
        fn test_condition_expr_proptest_roundtrip(s in "\\PC{1,100}") {
            let expr = ConditionExpr::new(s);
            let json = serde_json::to_string(&expr).unwrap();
            let deserialized: ConditionExpr = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(expr, deserialized);
        }
    }
}
