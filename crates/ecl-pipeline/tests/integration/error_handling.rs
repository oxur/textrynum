//! Error handling integration tests: retry, skip_on_error, stage failure.

use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use ecl_adapter_fs::FilesystemAdapter;
use ecl_pipeline::PipelineRunner;
use ecl_pipeline_spec::source::FilesystemSourceSpec;
use ecl_pipeline_spec::{DefaultsSpec, PipelineSpec, ResourceSpec, SourceSpec, StageSpec};
use ecl_pipeline_state::{Blake3Hash, InMemoryStateStore, PipelineStatus, StageId};
use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{
    PipelineItem, PipelineTopology, ResolvedStage, RetryPolicy, SourceAdapter, Stage, StageContext,
};
use tempfile::TempDir;

fn fast_retry() -> RetryPolicy {
    RetryPolicy {
        max_attempts: 1,
        initial_backoff: Duration::from_millis(1),
        backoff_multiplier: 1.0,
        max_backoff: Duration::from_millis(10),
    }
}

/// A stage that always fails.
#[derive(Debug)]
struct AlwaysFailingStage {
    name: String,
}

#[async_trait]
impl Stage for AlwaysFailingStage {
    fn name(&self) -> &str {
        &self.name
    }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        Err(StageError::Permanent {
            stage: self.name.clone(),
            item_id: item.id.clone(),
            message: "intentional failure".to_string(),
        })
    }
}

/// A stage that fails for specific items and passes others.
#[derive(Debug)]
struct SelectiveFailStage {
    fail_ids: Vec<String>,
}

#[async_trait]
impl Stage for SelectiveFailStage {
    fn name(&self) -> &str {
        "selective-fail"
    }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        if self.fail_ids.iter().any(|id| id == &item.id) {
            Err(StageError::Permanent {
                stage: "selective-fail".to_string(),
                item_id: item.id.clone(),
                message: "selected for failure".to_string(),
            })
        } else {
            Ok(vec![item])
        }
    }
}

/// Build a topology with a single custom stage as the only batch.
/// Items are discovered from the filesystem source and fed directly
/// to the custom stage (no prior extract stage), since the runner
/// marks items Completed after each stage.
fn build_single_stage_topo(
    input_dir: &std::path::Path,
    output_dir: &std::path::Path,
    stage_name: &str,
    handler: Arc<dyn Stage>,
    skip_on_error: bool,
    retry: RetryPolicy,
) -> PipelineTopology {
    let fs_spec = FilesystemSourceSpec {
        root: input_dir.to_path_buf(),
        filters: vec![],
        extensions: vec![],
        stream: None,
    };
    let adapter: Arc<dyn SourceAdapter> =
        Arc::new(FilesystemAdapter::from_fs_spec("local", &fs_spec).unwrap());

    let spec = Arc::new(PipelineSpec {
        name: "error-test".to_string(),
        version: 1,
        output_dir: output_dir.to_path_buf(),
        sources: BTreeMap::from([("local".to_string(), SourceSpec::Filesystem(fs_spec))]),
        stages: BTreeMap::from([(
            stage_name.to_string(),
            StageSpec {
                adapter: stage_name.to_string(),
                source: Some("local".to_string()),
                resources: ResourceSpec {
                    creates: vec!["output".to_string()],
                    reads: vec![],
                    writes: vec![],
                },
                params: serde_json::Value::Null,
                retry: None,
                timeout_secs: None,
                skip_on_error,
                condition: None,
                input_streams: vec![],
                output_stream: None,
            },
        )]),
        defaults: DefaultsSpec::default(),
        lifecycle: None,
        secrets: Default::default(),
        triggers: None,
        schedule: None,
    });

    let spec_hash_bytes = serde_json::to_string(&*spec).unwrap();
    let spec_hash = Blake3Hash::new(blake3::hash(spec_hash_bytes.as_bytes()).to_hex().as_str());

    let stage_id = StageId::new(stage_name);
    PipelineTopology {
        spec,
        spec_hash,
        sources: BTreeMap::from([("local".to_string(), adapter)]),
        stages: BTreeMap::from([(
            stage_name.to_string(),
            ResolvedStage {
                id: stage_id.clone(),
                handler,
                retry,
                skip_on_error,
                timeout: None,
                source: Some("local".to_string()),
                condition: None,
            },
        )]),
        push_sources: BTreeMap::new(),
        schedule: vec![vec![stage_id]],
        output_dir: output_dir.to_path_buf(),
    }
}

#[tokio::test]
async fn test_stage_failure_stops_pipeline() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    fs::write(input.path().join("a.txt"), "aaa").unwrap();

    let topo = build_single_stage_topo(
        input.path(),
        output.path(),
        "fail-stage",
        Arc::new(AlwaysFailingStage {
            name: "fail-stage".to_string(),
        }),
        false, // skip_on_error=false
        fast_retry(),
    );

    let store = Box::new(InMemoryStateStore::new());
    let mut runner = PipelineRunner::new(topo, store).await.unwrap();
    let result = runner.run().await;
    assert!(result.is_err(), "Pipeline should fail when stage fails");
}

#[tokio::test]
async fn test_skip_on_error_continues_pipeline() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    fs::write(input.path().join("a.txt"), "aaa").unwrap();

    let topo = build_single_stage_topo(
        input.path(),
        output.path(),
        "fail-stage",
        Arc::new(AlwaysFailingStage {
            name: "fail-stage".to_string(),
        }),
        true, // skip_on_error=true
        fast_retry(),
    );

    let store = Box::new(InMemoryStateStore::new());
    let mut runner = PipelineRunner::new(topo, store).await.unwrap();
    let state = runner.run().await.unwrap();

    assert!(
        matches!(state.status, PipelineStatus::Completed { .. }),
        "Pipeline should complete when skip_on_error=true"
    );
}

#[tokio::test]
async fn test_selective_failure_with_skip_on_error() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    fs::write(input.path().join("good.txt"), "good").unwrap();
    fs::write(input.path().join("bad.txt"), "bad").unwrap();

    let topo = build_single_stage_topo(
        input.path(),
        output.path(),
        "selective",
        Arc::new(SelectiveFailStage {
            fail_ids: vec!["bad.txt".to_string()],
        }),
        true, // skip_on_error=true
        fast_retry(),
    );

    let store = Box::new(InMemoryStateStore::new());
    let mut runner = PipelineRunner::new(topo, store).await.unwrap();
    let state = runner.run().await.unwrap();

    assert!(matches!(state.status, PipelineStatus::Completed { .. }));
    // Both items discovered, but one was skipped
    assert_eq!(state.stats.total_items_discovered, 2);
}

#[tokio::test]
async fn test_selective_failure_without_skip_aborts() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    fs::write(input.path().join("good.txt"), "good").unwrap();
    fs::write(input.path().join("bad.txt"), "bad").unwrap();

    let topo = build_single_stage_topo(
        input.path(),
        output.path(),
        "selective",
        Arc::new(SelectiveFailStage {
            fail_ids: vec!["bad.txt".to_string()],
        }),
        false, // skip_on_error=false
        fast_retry(),
    );

    let store = Box::new(InMemoryStateStore::new());
    let mut runner = PipelineRunner::new(topo, store).await.unwrap();
    let result = runner.run().await;
    assert!(result.is_err(), "Pipeline should abort on failure");
}

#[tokio::test]
async fn test_retry_with_transient_failure() {
    /// A stage that fails N times then succeeds.
    #[derive(Debug)]
    struct TransientFailStage {
        fail_count: std::sync::atomic::AtomicU32,
        fail_until: u32,
    }

    #[async_trait]
    impl Stage for TransientFailStage {
        fn name(&self) -> &str {
            "transient"
        }

        async fn process(
            &self,
            item: PipelineItem,
            _ctx: &StageContext,
        ) -> Result<Vec<PipelineItem>, StageError> {
            let count = self
                .fail_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if count < self.fail_until {
                Err(StageError::Transient {
                    stage: "transient".to_string(),
                    item_id: item.id.clone(),
                    message: format!("attempt {}", count + 1),
                })
            } else {
                Ok(vec![item])
            }
        }
    }

    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    fs::write(input.path().join("a.txt"), "aaa").unwrap();

    let topo = build_single_stage_topo(
        input.path(),
        output.path(),
        "transient",
        Arc::new(TransientFailStage {
            fail_count: std::sync::atomic::AtomicU32::new(0),
            fail_until: 2, // Fail first 2 attempts
        }),
        false,
        RetryPolicy {
            max_attempts: 5, // enough to succeed after 2 failures
            initial_backoff: Duration::from_millis(1),
            backoff_multiplier: 1.0,
            max_backoff: Duration::from_millis(10),
        },
    );

    let store = Box::new(InMemoryStateStore::new());
    let mut runner = PipelineRunner::new(topo, store).await.unwrap();
    let state = runner.run().await.unwrap();

    assert!(
        matches!(state.status, PipelineStatus::Completed { .. }),
        "Pipeline should complete after retry succeeds"
    );
}
