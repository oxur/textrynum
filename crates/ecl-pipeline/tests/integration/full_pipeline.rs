//! End-to-end pipeline tests with real filesystem adapter and stages.

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
use ecl_stages::ExtractStage;
use tempfile::TempDir;

fn fast_retry() -> RetryPolicy {
    RetryPolicy {
        max_attempts: 1,
        initial_backoff: Duration::from_millis(1),
        backoff_multiplier: 1.0,
        max_backoff: Duration::from_millis(10),
    }
}

/// A combined stage that extracts content from a source adapter AND writes
/// it to the output directory. This works around the runner's current
/// design where each stage gets fresh empty-content items (stage outputs
/// are not propagated to downstream stages).
#[derive(Debug)]
struct ExtractAndEmitStage {
    adapter: Arc<dyn SourceAdapter>,
    source_name: String,
}

#[async_trait]
impl Stage for ExtractAndEmitStage {
    fn name(&self) -> &str {
        "extract-and-emit"
    }

    async fn process(
        &self,
        item: PipelineItem,
        ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // Fetch content
        let source_item = ecl_pipeline_topo::SourceItem {
            id: item.id.clone(),
            display_name: item.display_name.clone(),
            mime_type: item.mime_type.clone(),
            path: item.id.clone(),
            modified_at: item.provenance.source_modified,
            source_hash: None,
        };

        let doc = self
            .adapter
            .fetch(&source_item)
            .await
            .map_err(|e| StageError::Permanent {
                stage: "extract-and-emit".to_string(),
                item_id: item.id.clone(),
                message: format!("fetch failed: {e}"),
            })?;

        // Write to output
        let output_path = ctx.output_dir.join(&item.id);
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| StageError::Permanent {
                    stage: "extract-and-emit".to_string(),
                    item_id: item.id.clone(),
                    message: format!("mkdir failed: {e}"),
                })?;
        }
        tokio::fs::write(&output_path, &doc.content)
            .await
            .map_err(|e| StageError::Permanent {
                stage: "extract-and-emit".to_string(),
                item_id: item.id.clone(),
                message: format!("write failed: {e}"),
            })?;

        Ok(vec![PipelineItem {
            id: doc.id,
            display_name: doc.display_name,
            content: Arc::from(doc.content),
            mime_type: doc.mime_type,
            source_name: self.source_name.clone(),
            source_content_hash: doc.content_hash,
            provenance: doc.provenance,
            metadata: BTreeMap::new(),
            record: None,
        }])
    }
}

/// Build a pipeline that extracts-and-emits in a single stage.
fn build_fs_pipeline(
    input_dir: &std::path::Path,
    output_dir: &std::path::Path,
) -> PipelineTopology {
    let fs_spec = FilesystemSourceSpec {
        root: input_dir.to_path_buf(),
        filters: vec![],
        extensions: vec![],
    };
    let adapter: Arc<dyn SourceAdapter> =
        Arc::new(FilesystemAdapter::from_fs_spec("local", &fs_spec).unwrap());

    let stage: Arc<dyn Stage> = Arc::new(ExtractAndEmitStage {
        adapter: adapter.clone(),
        source_name: "local".to_string(),
    });

    let spec = Arc::new(PipelineSpec {
        name: "integration-test".to_string(),
        version: 1,
        output_dir: output_dir.to_path_buf(),
        sources: BTreeMap::from([("local".to_string(), SourceSpec::Filesystem(fs_spec))]),
        stages: BTreeMap::from([(
            "process".to_string(),
            StageSpec {
                adapter: "extract-and-emit".to_string(),
                source: Some("local".to_string()),
                resources: ResourceSpec {
                    creates: vec!["output".to_string()],
                    reads: vec![],
                    writes: vec![],
                },
                params: serde_json::Value::Null,
                retry: None,
                timeout_secs: None,
                skip_on_error: false,
                condition: None,
            },
        )]),
        defaults: DefaultsSpec::default(),
    });

    let spec_hash_bytes = serde_json::to_string(&*spec).unwrap();
    let spec_hash = Blake3Hash::new(blake3::hash(spec_hash_bytes.as_bytes()).to_hex().as_str());

    PipelineTopology {
        spec,
        spec_hash,
        sources: BTreeMap::from([("local".to_string(), adapter)]),
        stages: BTreeMap::from([(
            "process".to_string(),
            ResolvedStage {
                id: StageId::new("process"),
                handler: stage,
                retry: fast_retry(),
                skip_on_error: false,
                timeout: None,
                source: Some("local".to_string()),
                condition: None,
            },
        )]),
        push_sources: BTreeMap::new(),
        schedule: vec![vec![StageId::new("process")]],
        output_dir: output_dir.to_path_buf(),
    }
}

fn create_test_files(dir: &std::path::Path, count: usize) -> Vec<String> {
    let mut names = Vec::new();
    for i in 0..count {
        let name = format!("file-{i:02}.txt");
        fs::write(dir.join(&name), format!("Content of file {i}")).unwrap();
        names.push(name);
    }
    names
}

// ── Happy-path tests ───────────────────────────────────────────────

#[tokio::test]
async fn test_full_pipeline_10_files() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let names = create_test_files(input.path(), 10);

    let topo = build_fs_pipeline(input.path(), output.path());
    let store = Box::new(InMemoryStateStore::new());
    let mut runner = PipelineRunner::new(topo, store).await.unwrap();

    let state = runner.run().await.unwrap();
    assert!(
        matches!(state.status, PipelineStatus::Completed { .. }),
        "Pipeline should complete, was {:?}",
        state.status
    );

    for name in &names {
        let output_path = output.path().join(name);
        assert!(output_path.exists(), "Output file {name} should exist");
        let input_content = fs::read_to_string(input.path().join(name)).unwrap();
        let output_content = fs::read_to_string(&output_path).unwrap();
        assert_eq!(input_content, output_content, "Content mismatch for {name}");
    }
}

#[tokio::test]
async fn test_full_pipeline_empty_directory() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();

    let topo = build_fs_pipeline(input.path(), output.path());
    let store = Box::new(InMemoryStateStore::new());
    let mut runner = PipelineRunner::new(topo, store).await.unwrap();

    let state = runner.run().await.unwrap();
    assert!(matches!(state.status, PipelineStatus::Completed { .. }));
    assert_eq!(state.stats.total_items_discovered, 0);
}

#[tokio::test]
async fn test_full_pipeline_nested_directories() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();

    fs::create_dir_all(input.path().join("sub/deep")).unwrap();
    fs::write(input.path().join("root.txt"), "root").unwrap();
    fs::write(input.path().join("sub/middle.txt"), "middle").unwrap();
    fs::write(input.path().join("sub/deep/leaf.txt"), "leaf").unwrap();

    let topo = build_fs_pipeline(input.path(), output.path());
    let store = Box::new(InMemoryStateStore::new());
    let mut runner = PipelineRunner::new(topo, store).await.unwrap();

    let state = runner.run().await.unwrap();
    assert!(matches!(state.status, PipelineStatus::Completed { .. }));

    assert!(output.path().join("root.txt").exists());
    assert!(output.path().join("sub/middle.txt").exists());
    assert!(output.path().join("sub/deep/leaf.txt").exists());
    assert_eq!(
        fs::read_to_string(output.path().join("sub/deep/leaf.txt")).unwrap(),
        "leaf"
    );
}

#[tokio::test]
async fn test_full_pipeline_preserves_binary_content() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();

    let binary = vec![0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD];
    fs::write(input.path().join("data.bin"), &binary).unwrap();

    let topo = build_fs_pipeline(input.path(), output.path());
    let store = Box::new(InMemoryStateStore::new());
    let mut runner = PipelineRunner::new(topo, store).await.unwrap();

    runner.run().await.unwrap();
    let written = fs::read(output.path().join("data.bin")).unwrap();
    assert_eq!(written, binary);
}

#[tokio::test]
async fn test_full_pipeline_state_shows_correct_counts() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    create_test_files(input.path(), 5);

    let topo = build_fs_pipeline(input.path(), output.path());
    let store = Box::new(InMemoryStateStore::new());
    let mut runner = PipelineRunner::new(topo, store).await.unwrap();

    let state = runner.run().await.unwrap();
    assert_eq!(state.stats.total_items_discovered, 5);
}

#[tokio::test]
async fn test_full_pipeline_with_extension_filter() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();

    fs::write(input.path().join("readme.md"), "# Hello").unwrap();
    fs::write(input.path().join("notes.txt"), "notes").unwrap();
    fs::write(input.path().join("data.json"), "{}").unwrap();

    // Only .md extensions
    let fs_spec = FilesystemSourceSpec {
        root: input.path().to_path_buf(),
        filters: vec![],
        extensions: vec!["md".to_string()],
    };
    let adapter: Arc<dyn SourceAdapter> =
        Arc::new(FilesystemAdapter::from_fs_spec("local", &fs_spec).unwrap());

    let stage: Arc<dyn Stage> = Arc::new(ExtractAndEmitStage {
        adapter: adapter.clone(),
        source_name: "local".to_string(),
    });

    let spec = Arc::new(PipelineSpec {
        name: "filter-test".to_string(),
        version: 1,
        output_dir: output.path().to_path_buf(),
        sources: BTreeMap::from([("local".to_string(), SourceSpec::Filesystem(fs_spec))]),
        stages: BTreeMap::from([(
            "process".to_string(),
            StageSpec {
                adapter: "extract-and-emit".to_string(),
                source: Some("local".to_string()),
                resources: ResourceSpec {
                    creates: vec!["output".to_string()],
                    reads: vec![],
                    writes: vec![],
                },
                params: serde_json::Value::Null,
                retry: None,
                timeout_secs: None,
                skip_on_error: false,
                condition: None,
            },
        )]),
        defaults: DefaultsSpec::default(),
    });

    let spec_hash = Blake3Hash::new("test-hash");
    let topo = PipelineTopology {
        spec,
        spec_hash,
        sources: BTreeMap::from([("local".to_string(), adapter)]),
        stages: BTreeMap::from([(
            "process".to_string(),
            ResolvedStage {
                id: StageId::new("process"),
                handler: stage,
                retry: fast_retry(),
                skip_on_error: false,
                timeout: None,
                source: Some("local".to_string()),
                condition: None,
            },
        )]),
        push_sources: BTreeMap::new(),
        schedule: vec![vec![StageId::new("process")]],
        output_dir: output.path().to_path_buf(),
    };

    let store = Box::new(InMemoryStateStore::new());
    let mut runner = PipelineRunner::new(topo, store).await.unwrap();
    let state = runner.run().await.unwrap();

    assert!(matches!(state.status, PipelineStatus::Completed { .. }));
    assert_eq!(state.stats.total_items_discovered, 1);
    assert!(output.path().join("readme.md").exists());
    assert!(!output.path().join("notes.txt").exists());
    assert!(!output.path().join("data.json").exists());
}

// ── Multi-stage pipeline tests (using runner's current design) ─────

#[tokio::test]
async fn test_multi_stage_extract_then_passthrough() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    fs::write(input.path().join("a.txt"), "hello").unwrap();

    let fs_spec = FilesystemSourceSpec {
        root: input.path().to_path_buf(),
        filters: vec![],
        extensions: vec![],
    };
    let adapter: Arc<dyn SourceAdapter> =
        Arc::new(FilesystemAdapter::from_fs_spec("local", &fs_spec).unwrap());

    // Stage 1: extract (fetches content, but runner discards output items)
    let extract: Arc<dyn Stage> = Arc::new(ExtractStage::new(adapter.clone(), "local"));

    let spec = Arc::new(PipelineSpec {
        name: "multi-stage-test".to_string(),
        version: 1,
        output_dir: output.path().to_path_buf(),
        sources: BTreeMap::from([("local".to_string(), SourceSpec::Filesystem(fs_spec))]),
        stages: BTreeMap::from([(
            "extract".to_string(),
            StageSpec {
                adapter: "extract".to_string(),
                source: Some("local".to_string()),
                resources: ResourceSpec {
                    creates: vec!["docs".to_string()],
                    reads: vec![],
                    writes: vec![],
                },
                params: serde_json::Value::Null,
                retry: None,
                timeout_secs: None,
                skip_on_error: false,
                condition: None,
            },
        )]),
        defaults: DefaultsSpec::default(),
    });

    let spec_hash = Blake3Hash::new("test-hash");
    let topo = PipelineTopology {
        spec,
        spec_hash,
        sources: BTreeMap::from([("local".to_string(), adapter)]),
        stages: BTreeMap::from([(
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
        )]),
        push_sources: BTreeMap::new(),
        schedule: vec![vec![StageId::new("extract")]],
        output_dir: output.path().to_path_buf(),
    };

    let store = Box::new(InMemoryStateStore::new());
    let mut runner = PipelineRunner::new(topo, store).await.unwrap();
    let state = runner.run().await.unwrap();

    assert!(matches!(state.status, PipelineStatus::Completed { .. }));
    assert_eq!(state.stats.total_items_discovered, 1);
}
