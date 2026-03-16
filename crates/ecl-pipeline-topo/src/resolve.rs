//! Topology resolution: converting a PipelineSpec into a PipelineTopology.
//!
//! This module implements the core resolution logic:
//! 1. Hash the spec for config drift detection (blake3).
//! 2. Resolve each source into a concrete adapter via the adapter registry.
//! 3. Resolve each stage into a concrete handler via the stage registry.
//! 4. Merge stage-level retry overrides with global defaults.
//! 5. Build the resource graph and validate.
//! 6. Compute the parallel execution schedule.
//! 7. Create the output directory (async).

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use ecl_pipeline_spec::{DefaultsSpec, PipelineSpec, RetrySpec, SourceSpec, StageSpec};
use ecl_pipeline_state::{Blake3Hash, StageId};

use crate::error::ResolveError;
use crate::resource_graph::ResourceGraph;
use crate::{ConditionExpr, PipelineTopology, ResolvedStage, RetryPolicy, SourceAdapter, Stage};

/// Resolve a `PipelineSpec` into a `PipelineTopology`.
///
/// This is the main entry point for topology construction:
/// 1. Hash the spec for config drift detection.
/// 2. Resolve each source into a concrete `SourceAdapter` via the adapter
///    registry lookup function.
/// 3. Resolve each stage into a concrete `Stage` handler via the stage
///    registry lookup function.
/// 4. Merge retry policies (stage override > global default).
/// 5. Build the resource graph and validate (no missing inputs, no cycles).
/// 6. Compute the parallel schedule.
/// 7. Create the output directory.
///
/// # Arguments
///
/// * `spec` -- The parsed pipeline specification.
/// * `adapter_lookup` -- A function that takes (source_name, &SourceSpec) and
///   returns a concrete adapter. Typically backed by an `AdapterRegistry`.
/// * `stage_lookup` -- A function that takes (stage_name, &StageSpec) and
///   returns a concrete stage handler. Typically backed by a `StageRegistry`.
///
/// # Errors
///
/// Returns `ResolveError` if any step fails (unknown adapter, cycle,
/// missing resource, I/O error, etc.).
pub async fn resolve<F, G>(
    spec: PipelineSpec,
    adapter_lookup: F,
    stage_lookup: G,
) -> Result<PipelineTopology, ResolveError>
where
    F: Fn(&str, &SourceSpec) -> Result<Arc<dyn SourceAdapter>, ResolveError>,
    G: Fn(&str, &StageSpec) -> Result<Arc<dyn Stage>, ResolveError>,
{
    // 1. Hash the spec for config drift detection.
    // Use serde_json for serialization since it handles all serde types
    // (toml cannot serialize unit enum variants like CheckpointStrategy).
    let spec_bytes = serde_json::to_string(&spec).map_err(|e| ResolveError::SerializeError {
        message: e.to_string(),
    })?;
    let spec_hash = Blake3Hash::new(blake3::hash(spec_bytes.as_bytes()).to_hex().to_string());
    let spec = Arc::new(spec);

    // 2. Resolve each source into a concrete adapter.
    let mut sources: BTreeMap<String, Arc<dyn SourceAdapter>> = BTreeMap::new();
    for (name, source_spec) in &spec.sources {
        let kind = source_kind(source_spec);
        let adapter = adapter_lookup(name, source_spec).map_err(|e| match e {
            ResolveError::UnknownAdapter { .. } => ResolveError::UnknownAdapter {
                stage: name.clone(),
                adapter: kind.to_string(),
            },
            other => other,
        })?;
        sources.insert(name.clone(), adapter);
    }

    // 3. Resolve each stage into a concrete handler.
    let mut stages: BTreeMap<String, ResolvedStage> = BTreeMap::new();
    for (name, stage_spec) in &spec.stages {
        let handler = stage_lookup(name, stage_spec)?;
        let resolved = resolve_stage(name, stage_spec, handler, &spec.defaults);
        stages.insert(name.clone(), resolved);
    }

    // 4. Build the resource graph and validate.
    let resource_graph = ResourceGraph::build(&spec.stages)?;
    resource_graph.validate_no_missing_inputs()?;
    resource_graph.validate_no_cycles()?;

    // 5. Compute the parallel schedule.
    let schedule = resource_graph.compute_schedule()?;

    // 6. Create output directory (async to avoid blocking the runtime).
    let output_dir = spec.output_dir.clone();
    tokio::fs::create_dir_all(&output_dir)
        .await
        .map_err(ResolveError::Io)?;

    Ok(PipelineTopology {
        spec,
        spec_hash,
        sources,
        stages,
        schedule,
        output_dir,
    })
}

/// Extract the `kind` string from a `SourceSpec` variant.
///
/// This maps each variant to the string used as the registry key:
/// - `SourceSpec::GoogleDrive(..)` -> `"google_drive"`
/// - `SourceSpec::Slack(..)` -> `"slack"`
/// - `SourceSpec::Filesystem(..)` -> `"filesystem"`
fn source_kind(spec: &SourceSpec) -> &'static str {
    match spec {
        SourceSpec::GoogleDrive(_) => "google_drive",
        SourceSpec::Slack(_) => "slack",
        SourceSpec::Filesystem(_) => "filesystem",
    }
}

/// Resolve a single stage: create the `ResolvedStage` by merging the
/// stage-level configuration with global defaults.
fn resolve_stage(
    name: &str,
    stage_spec: &StageSpec,
    handler: Arc<dyn Stage>,
    defaults: &DefaultsSpec,
) -> ResolvedStage {
    // Merge retry: stage override > global default.
    let retry = resolve_retry_policy(stage_spec.retry.as_ref(), &defaults.retry);

    // Resolve timeout from seconds to Duration.
    let timeout = stage_spec.timeout_secs.map(Duration::from_secs);

    // Resolve condition expression.
    let condition = stage_spec.condition.as_ref().map(ConditionExpr::new);

    ResolvedStage {
        id: StageId::new(name),
        handler,
        retry,
        skip_on_error: stage_spec.skip_on_error,
        timeout,
        source: stage_spec.source.clone(),
        condition,
    }
}

/// Merge a stage-level `RetrySpec` override with the global default
/// `RetrySpec`, producing a resolved `RetryPolicy` with `Duration` values.
///
/// If `stage_retry` is `Some`, its values are used. If `None`, the global
/// defaults are used. This is a wholesale override (the entire `RetrySpec`
/// from the stage replaces the global default), not a field-by-field merge,
/// because `RetrySpec` fields all have their own serde defaults -- a stage
/// that specifies `retry = { max_attempts = 5 }` in TOML will get
/// `max_attempts = 5` with default values for the other fields, which is
/// the correct behavior.
pub fn resolve_retry_policy(
    stage_retry: Option<&RetrySpec>,
    global_retry: &RetrySpec,
) -> RetryPolicy {
    let spec = stage_retry.unwrap_or(global_retry);
    RetryPolicy::from_spec(spec)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use ecl_pipeline_spec::source::FilesystemSourceSpec;
    use std::path::PathBuf;

    use crate::error::SourceError;
    use crate::{ExtractedDocument, PipelineItem, SourceItem, StageContext, StageError};

    // ── Mock types ───────────────────────────────────────────────────

    #[derive(Debug)]
    struct MockSourceAdapter {
        kind: String,
    }

    impl MockSourceAdapter {
        fn new(kind: &str) -> Self {
            Self {
                kind: kind.to_string(),
            }
        }
    }

    #[async_trait]
    impl SourceAdapter for MockSourceAdapter {
        fn source_kind(&self) -> &str {
            &self.kind
        }
        async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
            Ok(vec![])
        }
        async fn fetch(&self, _item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
            Err(SourceError::NotFound {
                source_name: self.kind.clone(),
                item_id: "none".to_string(),
            })
        }
    }

    #[derive(Debug)]
    struct MockStage {
        name: String,
    }

    impl MockStage {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    #[async_trait]
    impl Stage for MockStage {
        fn name(&self) -> &str {
            &self.name
        }
        async fn process(
            &self,
            item: PipelineItem,
            _ctx: &StageContext,
        ) -> Result<Vec<PipelineItem>, StageError> {
            Ok(vec![item])
        }
    }

    // ── Mock lookups ─────────────────────────────────────────────────

    fn mock_adapter_lookup(
        _name: &str,
        spec: &SourceSpec,
    ) -> Result<Arc<dyn SourceAdapter>, ResolveError> {
        let kind = match spec {
            SourceSpec::GoogleDrive(_) => "google_drive",
            SourceSpec::Slack(_) => "slack",
            SourceSpec::Filesystem(_) => "filesystem",
        };
        Ok(Arc::new(MockSourceAdapter::new(kind)))
    }

    fn mock_stage_lookup(_name: &str, spec: &StageSpec) -> Result<Arc<dyn Stage>, ResolveError> {
        Ok(Arc::new(MockStage::new(&spec.adapter)))
    }

    fn failing_adapter_lookup(
        name: &str,
        _spec: &SourceSpec,
    ) -> Result<Arc<dyn SourceAdapter>, ResolveError> {
        Err(ResolveError::UnknownAdapter {
            stage: name.to_string(),
            adapter: "unknown".to_string(),
        })
    }

    fn failing_stage_lookup(name: &str, spec: &StageSpec) -> Result<Arc<dyn Stage>, ResolveError> {
        Err(ResolveError::UnknownAdapter {
            stage: name.to_string(),
            adapter: spec.adapter.clone(),
        })
    }

    // ── Helper to build a minimal spec ───────────────────────────────

    fn minimal_spec(output_dir: &str) -> PipelineSpec {
        PipelineSpec::from_toml(&format!(
            r#"
name = "test-pipeline"
version = 1
output_dir = "{output_dir}"

[sources.local]
kind = "filesystem"
root = "/tmp/test-data"

[stages.extract]
adapter = "extract"
source = "local"
resources = {{ creates = ["raw-docs"] }}

[stages.emit]
adapter = "emit"
resources = {{ reads = ["raw-docs"] }}
"#
        ))
        .unwrap()
    }

    fn full_example_spec(output_dir: &str) -> PipelineSpec {
        PipelineSpec::from_toml(&format!(
            r#"
name = "q1-knowledge-sync"
version = 1
output_dir = "{output_dir}"

[defaults]
concurrency = 4
checkpoint = {{ every = "Batch" }}

[defaults.retry]
max_attempts = 3
initial_backoff_ms = 1000
backoff_multiplier = 2.0
max_backoff_ms = 30000

[sources.engineering-drive]
kind = "google_drive"
credentials = {{ type = "env", env = "GOOGLE_CREDENTIALS" }}
root_folders = ["1abc123def456"]
file_types = [
    {{ extension = "docx" }},
    {{ extension = "pdf" }},
    {{ mime = "application/vnd.google-apps.document" }},
]
modified_after = "last_run"

  [[sources.engineering-drive.filters]]
  pattern = "**/Archive/**"
  action = "Exclude"

  [[sources.engineering-drive.filters]]
  pattern = "**"
  action = "Include"

[sources.team-slack]
kind = "slack"
credentials = {{ type = "env", env = "SLACK_BOT_TOKEN" }}
channels = ["C01234ABCDE", "C05678FGHIJ"]
thread_depth = 3
modified_after = "2026-01-01T00:00:00Z"

[stages.fetch-gdrive]
adapter = "extract"
source = "engineering-drive"
resources = {{ reads = ["gdrive-api"], creates = ["raw-gdrive-docs"] }}
retry = {{ max_attempts = 3, initial_backoff_ms = 1000, backoff_multiplier = 2.0, max_backoff_ms = 30000 }}
timeout_secs = 300

[stages.fetch-slack]
adapter = "extract"
source = "team-slack"
resources = {{ reads = ["slack-api"], creates = ["raw-slack-messages"] }}
retry = {{ max_attempts = 3, initial_backoff_ms = 500, backoff_multiplier = 2.0, max_backoff_ms = 10000 }}

[stages.normalize-gdrive]
adapter = "normalize"
source = "engineering-drive"
resources = {{ reads = ["raw-gdrive-docs"], creates = ["normalized-docs"] }}

[stages.normalize-slack]
adapter = "slack-normalize"
source = "team-slack"
resources = {{ reads = ["raw-slack-messages"], creates = ["normalized-messages"] }}

[stages.emit]
adapter = "emit"
resources = {{ reads = ["normalized-docs", "normalized-messages"] }}

[stages.emit.params]
subdir = "normalized"
"#
        ))
        .unwrap()
    }

    // ── resolve_retry_policy tests ───────────────────────────────────

    #[test]
    fn test_resolve_retry_policy_uses_global_when_no_override() {
        let global = RetrySpec {
            max_attempts: 3,
            initial_backoff_ms: 1000,
            backoff_multiplier: 2.0,
            max_backoff_ms: 30_000,
        };
        let policy = resolve_retry_policy(None, &global);
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.initial_backoff, Duration::from_millis(1000));
    }

    #[test]
    fn test_resolve_retry_policy_uses_stage_override() {
        let global = RetrySpec {
            max_attempts: 3,
            initial_backoff_ms: 1000,
            backoff_multiplier: 2.0,
            max_backoff_ms: 30_000,
        };
        let stage = RetrySpec {
            max_attempts: 5,
            initial_backoff_ms: 500,
            backoff_multiplier: 1.5,
            max_backoff_ms: 10_000,
        };
        let policy = resolve_retry_policy(Some(&stage), &global);
        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.initial_backoff, Duration::from_millis(500));
    }

    #[test]
    fn test_resolve_retry_policy_converts_ms_to_duration() {
        let spec = RetrySpec {
            max_attempts: 1,
            initial_backoff_ms: 1500,
            backoff_multiplier: 1.0,
            max_backoff_ms: 60_000,
        };
        let policy = resolve_retry_policy(Some(&spec), &RetrySpec::default());
        assert_eq!(policy.initial_backoff, Duration::from_millis(1500));
        assert_eq!(policy.max_backoff, Duration::from_millis(60_000));
    }

    #[test]
    fn test_resolve_retry_policy_custom_stage_values() {
        let global = RetrySpec::default();
        let stage = RetrySpec {
            max_attempts: 5,
            initial_backoff_ms: 500,
            backoff_multiplier: 1.5,
            max_backoff_ms: 10_000,
        };
        let policy = resolve_retry_policy(Some(&stage), &global);
        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.initial_backoff, Duration::from_millis(500));
        assert!((policy.backoff_multiplier - 1.5).abs() < f64::EPSILON);
        assert_eq!(policy.max_backoff, Duration::from_millis(10_000));
    }

    // ── source_kind tests ────────────────────────────────────────────

    #[test]
    fn test_source_kind_filesystem() {
        let spec = SourceSpec::Filesystem(FilesystemSourceSpec {
            root: PathBuf::from("/tmp"),
            filters: vec![],
            extensions: vec![],
        });
        assert_eq!(source_kind(&spec), "filesystem");
    }

    #[test]
    fn test_source_kind_google_drive() {
        let spec = PipelineSpec::from_toml(
            r#"
name = "test"
version = 1
output_dir = "./out"

[sources.gdrive]
kind = "google_drive"
credentials = { type = "env", env = "CREDS" }
root_folders = ["abc"]

[stages.s]
adapter = "x"
"#,
        )
        .unwrap();
        let gdrive_spec = &spec.sources["gdrive"];
        assert_eq!(source_kind(gdrive_spec), "google_drive");
    }

    #[test]
    fn test_source_kind_slack() {
        let spec = PipelineSpec::from_toml(
            r#"
name = "test"
version = 1
output_dir = "./out"

[sources.slack]
kind = "slack"
credentials = { type = "env", env = "TOKEN" }
channels = ["C123"]

[stages.s]
adapter = "x"
"#,
        )
        .unwrap();
        let slack_spec = &spec.sources["slack"];
        assert_eq!(source_kind(slack_spec), "slack");
    }

    // ── Full resolve() tests ─────────────────────────────────────────

    #[tokio::test]
    async fn test_resolve_full_example_spec_returns_topology() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output");
        let spec = full_example_spec(output.to_str().unwrap());

        let topo = resolve(spec, mock_adapter_lookup, mock_stage_lookup)
            .await
            .unwrap();

        assert_eq!(topo.sources.len(), 2);
        assert_eq!(topo.stages.len(), 5);
        assert!(
            topo.schedule.len() >= 2,
            "should have at least 2 batches, got {}",
            topo.schedule.len()
        );
        assert!(!topo.spec_hash.is_empty());
    }

    #[tokio::test]
    async fn test_resolve_unknown_source_kind_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output");
        let spec = minimal_spec(output.to_str().unwrap());

        let result = resolve(spec, failing_adapter_lookup, mock_stage_lookup).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ResolveError::UnknownAdapter { .. }
        ));
    }

    #[tokio::test]
    async fn test_resolve_unknown_stage_adapter_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output");
        let spec = minimal_spec(output.to_str().unwrap());

        let result = resolve(spec, mock_adapter_lookup, failing_stage_lookup).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ResolveError::UnknownAdapter { .. }
        ));
    }

    #[tokio::test]
    async fn test_resolve_creates_output_directory() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("nested/output/dir");
        assert!(!output.exists());

        let spec = minimal_spec(output.to_str().unwrap());
        let _topo = resolve(spec, mock_adapter_lookup, mock_stage_lookup)
            .await
            .unwrap();

        assert!(output.exists(), "output directory should be created");
    }

    #[tokio::test]
    async fn test_resolve_spec_hash_is_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output");
        let spec1 = minimal_spec(output.to_str().unwrap());
        let spec2 = minimal_spec(output.to_str().unwrap());

        let topo1 = resolve(spec1, mock_adapter_lookup, mock_stage_lookup)
            .await
            .unwrap();
        let topo2 = resolve(spec2, mock_adapter_lookup, mock_stage_lookup)
            .await
            .unwrap();

        assert_eq!(topo1.spec_hash, topo2.spec_hash);
    }

    #[tokio::test]
    async fn test_resolve_spec_hash_changes_with_spec() {
        let dir = tempfile::tempdir().unwrap();
        let spec1 = minimal_spec(dir.path().join("out1").to_str().unwrap());

        // Different spec (different name).
        let spec2 = PipelineSpec::from_toml(&format!(
            r#"
name = "different-pipeline"
version = 2
output_dir = "{}"

[sources.local]
kind = "filesystem"
root = "/tmp/other"

[stages.extract]
adapter = "extract"
source = "local"
resources = {{ creates = ["raw-docs"] }}

[stages.emit]
adapter = "emit"
resources = {{ reads = ["raw-docs"] }}
"#,
            dir.path().join("out2").display()
        ))
        .unwrap();

        let topo1 = resolve(spec1, mock_adapter_lookup, mock_stage_lookup)
            .await
            .unwrap();
        let topo2 = resolve(spec2, mock_adapter_lookup, mock_stage_lookup)
            .await
            .unwrap();

        assert_ne!(topo1.spec_hash, topo2.spec_hash);
    }

    #[tokio::test]
    async fn test_resolve_stages_have_correct_retry_policy() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output");
        let spec = PipelineSpec::from_toml(&format!(
            r#"
name = "retry-test"
version = 1
output_dir = "{}"

[defaults.retry]
max_attempts = 3
initial_backoff_ms = 1000
backoff_multiplier = 2.0
max_backoff_ms = 30000

[sources.local]
kind = "filesystem"
root = "/tmp/test-data"

[stages.fast-retry]
adapter = "extract"
source = "local"
resources = {{ creates = ["raw-docs"] }}
retry = {{ max_attempts = 5, initial_backoff_ms = 500, backoff_multiplier = 1.5, max_backoff_ms = 10000 }}

[stages.default-retry]
adapter = "emit"
resources = {{ reads = ["raw-docs"] }}
"#,
            output.display()
        ))
        .unwrap();

        let topo = resolve(spec, mock_adapter_lookup, mock_stage_lookup)
            .await
            .unwrap();

        let fast = &topo.stages["fast-retry"];
        assert_eq!(fast.retry.max_attempts, 5);
        assert_eq!(fast.retry.initial_backoff, Duration::from_millis(500));
        assert!((fast.retry.backoff_multiplier - 1.5).abs() < f64::EPSILON);
        assert_eq!(fast.retry.max_backoff, Duration::from_millis(10_000));
    }

    #[tokio::test]
    async fn test_resolve_stages_use_global_retry_when_no_override() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output");
        let spec = PipelineSpec::from_toml(&format!(
            r#"
name = "retry-test"
version = 1
output_dir = "{}"

[defaults.retry]
max_attempts = 3
initial_backoff_ms = 1000
backoff_multiplier = 2.0
max_backoff_ms = 30000

[sources.local]
kind = "filesystem"
root = "/tmp/test-data"

[stages.fast-retry]
adapter = "extract"
source = "local"
resources = {{ creates = ["raw-docs"] }}
retry = {{ max_attempts = 5, initial_backoff_ms = 500, backoff_multiplier = 1.5, max_backoff_ms = 10000 }}

[stages.default-retry]
adapter = "emit"
resources = {{ reads = ["raw-docs"] }}
"#,
            output.display()
        ))
        .unwrap();

        let topo = resolve(spec, mock_adapter_lookup, mock_stage_lookup)
            .await
            .unwrap();

        let default = &topo.stages["default-retry"];
        assert_eq!(default.retry.max_attempts, 3);
        assert_eq!(default.retry.initial_backoff, Duration::from_millis(1000));
    }

    #[tokio::test]
    async fn test_resolve_stages_have_correct_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output");
        let spec = PipelineSpec::from_toml(&format!(
            r#"
name = "timeout-test"
version = 1
output_dir = "{}"

[sources.local]
kind = "filesystem"
root = "/tmp"

[stages.with-timeout]
adapter = "extract"
source = "local"
resources = {{ creates = ["docs"] }}
timeout_secs = 300

[stages.without-timeout]
adapter = "emit"
resources = {{ reads = ["docs"] }}
"#,
            output.display()
        ))
        .unwrap();

        let topo = resolve(spec, mock_adapter_lookup, mock_stage_lookup)
            .await
            .unwrap();

        assert_eq!(
            topo.stages["with-timeout"].timeout,
            Some(Duration::from_secs(300))
        );
        assert_eq!(topo.stages["without-timeout"].timeout, None);
    }

    #[tokio::test]
    async fn test_resolve_stages_have_correct_skip_on_error() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output");
        let spec = PipelineSpec::from_toml(&format!(
            r#"
name = "skip-test"
version = 1
output_dir = "{}"

[sources.local]
kind = "filesystem"
root = "/tmp"

[stages.skippable]
adapter = "extract"
source = "local"
resources = {{ creates = ["docs"] }}
skip_on_error = true

[stages.not-skippable]
adapter = "emit"
resources = {{ reads = ["docs"] }}
"#,
            output.display()
        ))
        .unwrap();

        let topo = resolve(spec, mock_adapter_lookup, mock_stage_lookup)
            .await
            .unwrap();

        assert!(topo.stages["skippable"].skip_on_error);
        assert!(!topo.stages["not-skippable"].skip_on_error);
    }

    #[tokio::test]
    async fn test_resolve_stages_have_correct_condition() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output");
        let spec = PipelineSpec::from_toml(&format!(
            r#"
name = "condition-test"
version = 1
output_dir = "{}"

[sources.local]
kind = "filesystem"
root = "/tmp"

[stages.conditional]
adapter = "extract"
source = "local"
resources = {{ creates = ["docs"] }}
condition = "x > 1"

[stages.unconditional]
adapter = "emit"
resources = {{ reads = ["docs"] }}
"#,
            output.display()
        ))
        .unwrap();

        let topo = resolve(spec, mock_adapter_lookup, mock_stage_lookup)
            .await
            .unwrap();

        assert_eq!(
            topo.stages["conditional"].condition,
            Some(ConditionExpr::new("x > 1"))
        );
        assert_eq!(topo.stages["unconditional"].condition, None);
    }

    #[tokio::test]
    async fn test_resolve_schedule_matches_expected() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output");
        let spec = full_example_spec(output.to_str().unwrap());

        let topo = resolve(spec, mock_adapter_lookup, mock_stage_lookup)
            .await
            .unwrap();

        // The 5-stage example has resource dependencies that produce
        // at least 3 batches: fetch-* -> normalize-* -> emit.
        assert!(
            topo.schedule.len() >= 3,
            "expected at least 3 batches, got {}",
            topo.schedule.len()
        );

        // All 5 stages should appear in the schedule exactly once.
        let total_scheduled: usize = topo.schedule.iter().map(|b| b.len()).sum();
        assert_eq!(total_scheduled, 5, "all 5 stages should be scheduled");
    }
}
