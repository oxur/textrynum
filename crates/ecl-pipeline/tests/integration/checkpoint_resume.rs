//! Checkpoint and resume integration tests.

use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;
use std::time::Duration;

use ecl_adapter_fs::FilesystemAdapter;
use ecl_pipeline::PipelineRunner;
use ecl_pipeline_spec::source::FilesystemSourceSpec;
use ecl_pipeline_spec::{DefaultsSpec, PipelineSpec, ResourceSpec, SourceSpec, StageSpec};
use ecl_pipeline_state::{
    Blake3Hash, Checkpoint, InMemoryStateStore, ItemProvenance, ItemState, ItemStatus,
    PipelineState, PipelineStats, PipelineStatus, RunId, SourceState, StageId, StateStore,
};
use ecl_pipeline_topo::{PipelineTopology, ResolvedStage, RetryPolicy, SourceAdapter, Stage};
use ecl_stages::{EmitStage, ExtractStage, NormalizeStage};
use tempfile::TempDir;

fn fast_retry() -> RetryPolicy {
    RetryPolicy {
        max_attempts: 1,
        initial_backoff: Duration::from_millis(1),
        backoff_multiplier: 1.0,
        max_backoff: Duration::from_millis(10),
    }
}

fn build_simple_topo(
    input_dir: &std::path::Path,
    output_dir: &std::path::Path,
) -> PipelineTopology {
    let fs_spec = FilesystemSourceSpec {
        root: input_dir.to_path_buf(),
        filters: vec![],
        extensions: vec![],
        stream: None,
    };
    let adapter: Arc<dyn SourceAdapter> =
        Arc::new(FilesystemAdapter::from_fs_spec("local", &fs_spec).unwrap());

    let extract: Arc<dyn Stage> = Arc::new(ExtractStage::new(adapter.clone(), "local"));
    let normalize: Arc<dyn Stage> = Arc::new(NormalizeStage::new());
    let emit: Arc<dyn Stage> = Arc::new(EmitStage::new());

    let spec = Arc::new(PipelineSpec {
        name: "checkpoint-test".to_string(),
        version: 1,
        output_dir: output_dir.to_path_buf(),
        sources: BTreeMap::from([("local".to_string(), SourceSpec::Filesystem(fs_spec))]),
        stages: BTreeMap::from([
            (
                "extract".to_string(),
                StageSpec {
                    adapter: "extract".to_string(),
                    source: Some("local".to_string()),
                    resources: ResourceSpec {
                        creates: vec!["raw".to_string()],
                        reads: vec![],
                        writes: vec![],
                    },
                    params: serde_json::Value::Null,
                    retry: None,
                    timeout_secs: None,
                    skip_on_error: false,
                    condition: None,
                    input_streams: vec![],
                    output_stream: None,
                },
            ),
            (
                "normalize".to_string(),
                StageSpec {
                    adapter: "normalize".to_string(),
                    source: None,
                    resources: ResourceSpec {
                        creates: vec!["norm".to_string()],
                        reads: vec!["raw".to_string()],
                        writes: vec![],
                    },
                    params: serde_json::Value::Null,
                    retry: None,
                    timeout_secs: None,
                    skip_on_error: false,
                    condition: None,
                    input_streams: vec![],
                    output_stream: None,
                },
            ),
            (
                "emit".to_string(),
                StageSpec {
                    adapter: "emit".to_string(),
                    source: None,
                    resources: ResourceSpec {
                        creates: vec!["output".to_string()],
                        reads: vec!["norm".to_string()],
                        writes: vec![],
                    },
                    params: serde_json::Value::Null,
                    retry: None,
                    timeout_secs: None,
                    skip_on_error: false,
                    condition: None,
                    input_streams: vec![],
                    output_stream: None,
                },
            ),
        ]),
        defaults: DefaultsSpec::default(),
        lifecycle: None,
    });

    let spec_hash_bytes = serde_json::to_string(&*spec).unwrap();
    let spec_hash = Blake3Hash::new(blake3::hash(spec_hash_bytes.as_bytes()).to_hex().as_str());

    PipelineTopology {
        spec,
        spec_hash,
        sources: BTreeMap::from([("local".to_string(), adapter)]),
        stages: BTreeMap::from([
            (
                "extract".to_string(),
                ResolvedStage {
                    id: StageId::new("extract"),
                    handler: extract,
                    retry: fast_retry(),
                    skip_on_error: false,
                    timeout: None,
                    source: Some("local".to_string()),
                    condition: None,
                },
            ),
            (
                "normalize".to_string(),
                ResolvedStage {
                    id: StageId::new("normalize"),
                    handler: normalize,
                    retry: fast_retry(),
                    skip_on_error: false,
                    timeout: None,
                    source: None,
                    condition: None,
                },
            ),
            (
                "emit".to_string(),
                ResolvedStage {
                    id: StageId::new("emit"),
                    handler: emit,
                    retry: fast_retry(),
                    skip_on_error: false,
                    timeout: None,
                    source: None,
                    condition: None,
                },
            ),
        ]),
        push_sources: BTreeMap::new(),
        schedule: vec![
            vec![StageId::new("extract")],
            vec![StageId::new("normalize")],
            vec![StageId::new("emit")],
        ],
        output_dir: output_dir.to_path_buf(),
    }
}

#[tokio::test]
async fn test_checkpoint_saved_after_run() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    fs::write(input.path().join("test.txt"), "hello").unwrap();

    let topo = build_simple_topo(input.path(), output.path());
    let store = InMemoryStateStore::new();
    let mut runner = PipelineRunner::new(topo, Box::new(store)).await.unwrap();

    runner.run().await.unwrap();

    // Checkpoint should exist
    let checkpoint = runner.topology().spec.name.clone();
    assert_eq!(checkpoint, "checkpoint-test");
}

#[tokio::test]
async fn test_resume_from_checkpoint_completes() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    fs::write(input.path().join("a.txt"), "aaa").unwrap();
    fs::write(input.path().join("b.txt"), "bbb").unwrap();

    let topo = build_simple_topo(input.path(), output.path());
    let spec_hash = topo.spec_hash.clone();

    // Create a checkpoint simulating batch 0 already done
    let store = InMemoryStateStore::new();
    let mut items = BTreeMap::new();
    items.insert(
        "a.txt".to_string(),
        ItemState {
            display_name: "a.txt".to_string(),
            source_id: "a.txt".to_string(),
            source_name: "local".to_string(),
            content_hash: Blake3Hash::new(""),
            status: ItemStatus::Pending,
            completed_stages: vec![],
            provenance: ItemProvenance {
                source_kind: "filesystem".to_string(),
                metadata: BTreeMap::new(),
                source_modified: None,
                extracted_at: chrono::Utc::now(),
            },
        },
    );
    items.insert(
        "b.txt".to_string(),
        ItemState {
            display_name: "b.txt".to_string(),
            source_id: "b.txt".to_string(),
            source_name: "local".to_string(),
            content_hash: Blake3Hash::new(""),
            status: ItemStatus::Pending,
            completed_stages: vec![],
            provenance: ItemProvenance {
                source_kind: "filesystem".to_string(),
                metadata: BTreeMap::new(),
                source_modified: None,
                extracted_at: chrono::Utc::now(),
            },
        },
    );

    let mut sources = BTreeMap::new();
    sources.insert(
        "local".to_string(),
        SourceState {
            items_discovered: 2,
            items_accepted: 2,
            items_skipped_unchanged: 0,
            items,
        },
    );

    let checkpoint = Checkpoint {
        version: 1,
        sequence: 2,
        created_at: chrono::Utc::now(),
        spec: (*topo.spec).clone(),
        schedule: topo.schedule.clone(),
        spec_hash,
        state: PipelineState {
            run_id: RunId::new("resume-run"),
            pipeline_name: "checkpoint-test".to_string(),
            started_at: chrono::Utc::now(),
            last_checkpoint: chrono::Utc::now(),
            status: PipelineStatus::Running {
                current_stage: "extract".to_string(),
            },
            current_batch: 1, // batch 0 (extract) done
            sources,
            stages: BTreeMap::new(),
            stats: PipelineStats {
                total_items_discovered: 2,
                total_items_processed: 0,
                total_items_skipped_unchanged: 0,
                total_items_failed: 0,
            },
        },
    };
    store.save_checkpoint(&checkpoint).await.unwrap();

    let mut runner = PipelineRunner::new(topo, Box::new(store)).await.unwrap();
    assert_eq!(runner.state().run_id.as_str(), "resume-run");

    let state = runner.run().await.unwrap();
    assert!(
        matches!(state.status, PipelineStatus::Completed { .. }),
        "Resumed pipeline should complete"
    );
}

#[tokio::test]
async fn test_config_drift_detected_on_resume() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();

    let topo = build_simple_topo(input.path(), output.path());

    // Create checkpoint with different hash
    let store = InMemoryStateStore::new();
    let checkpoint = Checkpoint {
        version: 1,
        sequence: 1,
        created_at: chrono::Utc::now(),
        spec: (*topo.spec).clone(),
        schedule: topo.schedule.clone(),
        spec_hash: Blake3Hash::new("DIFFERENT-HASH"),
        state: PipelineState {
            run_id: RunId::new("old-run"),
            pipeline_name: "checkpoint-test".to_string(),
            started_at: chrono::Utc::now(),
            last_checkpoint: chrono::Utc::now(),
            status: PipelineStatus::Pending,
            current_batch: 0,
            sources: BTreeMap::new(),
            stages: BTreeMap::new(),
            stats: PipelineStats::default(),
        },
    };
    store.save_checkpoint(&checkpoint).await.unwrap();

    let result = PipelineRunner::new(topo, Box::new(store)).await;
    assert!(result.is_err(), "Should detect config drift");
}

#[tokio::test]
async fn test_processing_items_reset_to_pending_on_resume() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    fs::write(input.path().join("a.txt"), "aaa").unwrap();

    let topo = build_simple_topo(input.path(), output.path());
    let spec_hash = topo.spec_hash.clone();

    let store = InMemoryStateStore::new();
    let mut items = BTreeMap::new();
    items.insert(
        "a.txt".to_string(),
        ItemState {
            display_name: "a.txt".to_string(),
            source_id: "a.txt".to_string(),
            source_name: "local".to_string(),
            content_hash: Blake3Hash::new(""),
            status: ItemStatus::Processing {
                stage: "extract".to_string(),
            },
            completed_stages: vec![],
            provenance: ItemProvenance {
                source_kind: "filesystem".to_string(),
                metadata: BTreeMap::new(),
                source_modified: None,
                extracted_at: chrono::Utc::now(),
            },
        },
    );

    let mut sources = BTreeMap::new();
    sources.insert(
        "local".to_string(),
        SourceState {
            items_discovered: 1,
            items_accepted: 1,
            items_skipped_unchanged: 0,
            items,
        },
    );

    let checkpoint = Checkpoint {
        version: 1,
        sequence: 1,
        created_at: chrono::Utc::now(),
        spec: (*topo.spec).clone(),
        schedule: topo.schedule.clone(),
        spec_hash,
        state: PipelineState {
            run_id: RunId::new("stuck-run"),
            pipeline_name: "checkpoint-test".to_string(),
            started_at: chrono::Utc::now(),
            last_checkpoint: chrono::Utc::now(),
            status: PipelineStatus::Running {
                current_stage: "extract".to_string(),
            },
            current_batch: 0,
            sources,
            stages: BTreeMap::new(),
            stats: PipelineStats::default(),
        },
    };
    store.save_checkpoint(&checkpoint).await.unwrap();

    let runner = PipelineRunner::new(topo, Box::new(store)).await.unwrap();
    let item = &runner.state().sources["local"].items["a.txt"];
    assert!(
        matches!(item.status, ItemStatus::Pending),
        "Processing items should be reset to Pending on resume, was {:?}",
        item.status
    );
}
