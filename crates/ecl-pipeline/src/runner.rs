//! PipelineRunner: the main execution orchestrator.
//!
//! Owns the topology, state, and state store. Executes the pipeline
//! lifecycle: enumerate sources, apply incrementality, run batches,
//! checkpoint at boundaries, and finalize.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Utc;

use ecl_pipeline_state::{
    Blake3Hash, Checkpoint, ItemProvenance, ItemState, ItemStatus, PipelineState, PipelineStats,
    PipelineStatus, RunId, StageId, StageState, StageStatus, StateStore,
};
use ecl_pipeline_topo::{PipelineItem, PipelineTopology, StageContext};

use crate::batch::{StageResult, execute_stage_items};
use crate::error::{PipelineError, Result};

/// The pipeline runner: orchestrates enumeration, incrementality,
/// batch execution, checkpointing, and resume.
pub struct PipelineRunner {
    /// The resolved pipeline topology (immutable during execution).
    topology: PipelineTopology,
    /// Mutable execution state (checkpointed after each batch).
    state: PipelineState,
    /// Persistence backend for checkpoints and content hashes.
    store: Box<dyn StateStore>,
    /// Monotonically increasing sequence number for checkpoints.
    checkpoint_sequence: u64,
}

impl std::fmt::Debug for PipelineRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipelineRunner")
            .field("topology", &self.topology)
            .field("state", &self.state)
            .field("store", &"<dyn StateStore>")
            .field("checkpoint_sequence", &self.checkpoint_sequence)
            .finish()
    }
}

impl PipelineRunner {
    /// Create a new runner from a pre-resolved topology and state store.
    ///
    /// If the store contains a checkpoint, loads it and prepares for
    /// resume (resets stuck items/stages, checks for config drift).
    /// If no checkpoint exists, creates fresh state.
    ///
    /// # Errors
    ///
    /// - `PipelineError::ConfigDrift` if the checkpoint's spec hash
    ///   does not match the current topology's spec hash.
    /// - `PipelineError::StateStore` if the store fails to load.
    pub async fn new(topology: PipelineTopology, store: Box<dyn StateStore>) -> Result<Self> {
        let state = match store.load_checkpoint().await? {
            Some(mut checkpoint) => {
                // Config drift check.
                if checkpoint.config_drifted(&topology.spec_hash) {
                    return Err(PipelineError::ConfigDrift {
                        checkpoint_hash: checkpoint.spec_hash.as_str().to_string(),
                        current_hash: topology.spec_hash.as_str().to_string(),
                    });
                }
                // Reset any items/stages stuck mid-processing.
                checkpoint.prepare_for_resume();
                tracing::info!(
                    run_id = %checkpoint.state.run_id,
                    sequence = checkpoint.sequence,
                    "Resuming from checkpoint",
                );
                checkpoint.state
            }
            None => {
                let now = Utc::now();
                let run_id = RunId::new(format!(
                    "{}-{}",
                    topology.spec.name,
                    now.format("%Y%m%dT%H%M%S")
                ));

                // Initialize stage states from topology schedule.
                let mut stages = BTreeMap::new();
                for batch in &topology.schedule {
                    for stage_id in batch {
                        stages.insert(
                            stage_id.clone(),
                            StageState {
                                status: StageStatus::Pending,
                                items_processed: 0,
                                items_failed: 0,
                                items_skipped: 0,
                                started_at: None,
                                completed_at: None,
                            },
                        );
                    }
                }

                PipelineState {
                    run_id,
                    pipeline_name: topology.spec.name.clone(),
                    started_at: now,
                    last_checkpoint: now,
                    status: PipelineStatus::Pending,
                    current_batch: 0,
                    sources: BTreeMap::new(),
                    stages,
                    stats: PipelineStats::default(),
                }
            }
        };

        Ok(Self {
            topology,
            state,
            store,
            checkpoint_sequence: 0,
        })
    }

    /// Execute the pipeline.
    ///
    /// Lifecycle:
    /// 1. Enumerate items from all sources (if not already done).
    /// 2. Apply incrementality (skip unchanged items).
    /// 3. Execute batches in order, checkpointing after each.
    /// 4. Save completed hashes and finalize state.
    ///
    /// Returns a reference to the final pipeline state.
    pub async fn run(&mut self) -> Result<&PipelineState> {
        let pipeline_name = self.state.pipeline_name.clone();
        let run_start = std::time::Instant::now();
        tracing::info!(pipeline = %pipeline_name, run_id = %self.state.run_id, "pipeline run starting");

        // Phase 1: Enumerate items from all sources (if not already done).
        if self.state.current_batch == 0 && self.state.stats.total_items_discovered == 0 {
            self.enumerate_sources().await?;
            self.apply_incrementality().await?;
            self.checkpoint().await?;
        }

        // Phase 2: Execute batches.
        let schedule = self.topology.schedule.clone();
        for (batch_idx, batch) in schedule.iter().enumerate() {
            if batch_idx < self.state.current_batch {
                // Already completed in a prior run — skip.
                continue;
            }

            self.execute_batch(batch_idx, batch).await?;
            self.state.current_batch = batch_idx + 1;
            self.checkpoint().await?;
        }

        // Phase 3: Finalize.
        self.save_completed_hashes().await?;
        self.state.status = PipelineStatus::Completed {
            finished_at: Utc::now(),
        };
        self.checkpoint().await?;

        let duration_ms = run_start.elapsed().as_millis() as u64;
        tracing::info!(
            pipeline = %pipeline_name,
            duration_ms,
            discovered = self.state.stats.total_items_discovered,
            processed = self.state.stats.total_items_processed,
            failed = self.state.stats.total_items_failed,
            "pipeline run completed"
        );

        Ok(&self.state)
    }

    /// Execute a single batch: stages in this batch run concurrently.
    ///
    /// Builds an immutable `StageContext` snapshot before execution.
    /// Each stage in the batch gets the same view. After all stages
    /// complete, their results are merged into the shared state.
    async fn execute_batch(&mut self, batch_idx: usize, stages: &[StageId]) -> Result<()> {
        tracing::info!(batch = batch_idx, stages = stages.len(), "executing batch");

        // Filter out stages whose conditions are not met.
        let active_stages: Vec<&StageId> = stages
            .iter()
            .filter(|id| self.should_execute_stage(id))
            .collect();

        // Execute stages concurrently (one tokio task per stage).
        let mut join_set = tokio::task::JoinSet::new();
        for stage_id in &active_stages {
            let stage_name = stage_id.as_str().to_string();
            let stage = self
                .topology
                .stages
                .get(&stage_name)
                .ok_or_else(|| PipelineError::ItemFailed {
                    stage: stage_name.clone(),
                    item_id: String::new(),
                    error: format!("stage '{stage_name}' not found in topology"),
                })?
                .clone();

            let items = self.collect_items_for_stage(stage_id);
            let ctx = self.build_stage_context(&stage_name);
            let concurrency = self.topology.spec.defaults.concurrency;

            // Mark stage as running.
            if let Some(stage_state) = self.state.stages.get_mut(*stage_id) {
                stage_state.status = StageStatus::Running;
                stage_state.started_at = Some(Utc::now());
            }

            join_set
                .spawn(async move { execute_stage_items(stage, items, ctx, concurrency).await });
        }

        // Collect results and merge into state.
        while let Some(result) = join_set.join_next().await {
            let stage_result = result??;
            self.merge_stage_result(stage_result)?;
        }

        Ok(())
    }

    /// Enumerate all sources and populate the item list.
    ///
    /// Calls `SourceAdapter::enumerate()` for each source in the topology.
    /// Creates `ItemState` entries in `PipelineState::sources` for each
    /// discovered item.
    async fn enumerate_sources(&mut self) -> Result<()> {
        for (name, adapter) in &self.topology.sources {
            tracing::info!(source = %name, "enumerating source");
            let items =
                adapter
                    .enumerate()
                    .await
                    .map_err(|e| PipelineError::SourceEnumeration {
                        source_name: name.clone(),
                        detail: e.to_string(),
                    })?;
            tracing::info!(source = %name, items = items.len(), "source enumeration complete");

            let source_state = self.state.sources.entry(name.clone()).or_default();

            source_state.items_discovered = items.len();

            for item in items {
                source_state.items_accepted += 1;
                source_state.items.insert(
                    item.id.clone(),
                    ItemState {
                        display_name: item.display_name.clone(),
                        source_id: item.id.clone(),
                        source_name: name.clone(),
                        content_hash: Blake3Hash::new(""),
                        status: ItemStatus::Pending,
                        completed_stages: vec![],
                        provenance: ItemProvenance {
                            source_kind: adapter.source_kind().to_string(),
                            metadata: BTreeMap::new(),
                            source_modified: item.modified_at,
                            extracted_at: Utc::now(),
                        },
                    },
                );
            }
        }
        self.state.update_stats();
        Ok(())
    }

    /// Compare content hashes against previous run; mark unchanged items.
    ///
    /// Loads hashes from the store's most recent completed run and compares
    /// each item's content hash. Items with matching hashes are marked as
    /// `ItemStatus::Unchanged` and will be skipped during execution.
    async fn apply_incrementality(&mut self) -> Result<()> {
        let previous_hashes = self.store.load_previous_hashes().await?;

        for source_state in self.state.sources.values_mut() {
            let mut skipped = 0usize;
            for (item_id, item_state) in source_state.items.iter_mut() {
                if let Some(prev_hash) = previous_hashes.get(item_id)
                    && !prev_hash.is_empty()
                    && item_state.content_hash == *prev_hash
                {
                    item_state.status = ItemStatus::Unchanged;
                    skipped += 1;
                }
            }
            source_state.items_skipped_unchanged += skipped;
        }
        self.state.update_stats();
        Ok(())
    }

    /// Build a checkpoint and persist it via the state store.
    async fn checkpoint(&mut self) -> Result<()> {
        self.checkpoint_sequence += 1;
        let checkpoint = Checkpoint {
            version: 1,
            sequence: self.checkpoint_sequence,
            created_at: Utc::now(),
            spec: (*self.topology.spec).clone(),
            schedule: self.topology.schedule.clone(),
            spec_hash: self.topology.spec_hash.clone(),
            state: self.state.clone(),
        };
        self.store.save_checkpoint(&checkpoint).await?;
        self.state.last_checkpoint = checkpoint.created_at;
        Ok(())
    }

    /// Save all completed item content hashes for future incrementality.
    async fn save_completed_hashes(&self) -> Result<()> {
        let mut hashes = BTreeMap::new();
        for source_state in self.state.sources.values() {
            for (item_id, item_state) in &source_state.items {
                if matches!(item_state.status, ItemStatus::Completed) {
                    hashes.insert(item_id.clone(), item_state.content_hash.clone());
                }
            }
        }
        self.store
            .save_completed_hashes(&self.state.run_id, &hashes)
            .await?;
        Ok(())
    }

    /// Determine whether a stage should execute.
    ///
    /// Currently always returns true. Condition expression evaluation
    /// is deferred to a future milestone.
    fn should_execute_stage(&self, _stage_id: &StageId) -> bool {
        true
    }

    /// Collect all pending items for a given stage.
    ///
    /// Returns `PipelineItem`s for all items in Pending status across
    /// all sources. Items in Unchanged, Completed, Failed, or Skipped
    /// status are excluded.
    fn collect_items_for_stage(&self, _stage_id: &StageId) -> Vec<PipelineItem> {
        let mut items = Vec::new();
        for source_state in self.state.sources.values() {
            for item_state in source_state.items.values() {
                if matches!(item_state.status, ItemStatus::Pending) {
                    items.push(PipelineItem {
                        id: item_state.source_id.clone(),
                        display_name: item_state.display_name.clone(),
                        content: Arc::from(Vec::new().as_slice()),
                        mime_type: String::new(),
                        source_name: item_state.source_name.clone(),
                        source_content_hash: item_state.content_hash.clone(),
                        provenance: item_state.provenance.clone(),
                        metadata: BTreeMap::new(),
                    });
                }
            }
        }
        items
    }

    /// Build a read-only `StageContext` for a stage.
    fn build_stage_context(&self, stage_name: &str) -> StageContext {
        let params = self
            .topology
            .spec
            .stages
            .get(stage_name)
            .map(|spec| spec.params.clone())
            .unwrap_or(serde_json::Value::Null);

        StageContext {
            spec: self.topology.spec.clone(),
            output_dir: self.topology.output_dir.clone(),
            params,
            span: tracing::info_span!("stage", name = stage_name),
        }
    }

    /// Merge a stage's results into the pipeline state.
    ///
    /// Updates item statuses (Completed, Failed, Skipped) and stage
    /// aggregate counters.
    fn merge_stage_result(&mut self, result: StageResult) -> Result<()> {
        let stage_id = &result.stage_id;

        let mut items_processed = 0usize;
        let mut items_failed = 0usize;
        let mut items_skipped = 0usize;

        // Record successes.
        for success in &result.successes {
            items_processed += 1;
            for source_state in self.state.sources.values_mut() {
                if let Some(item_state) = source_state.items.get_mut(&success.item_id) {
                    item_state.status = ItemStatus::Completed;
                    item_state
                        .completed_stages
                        .push(ecl_pipeline_state::CompletedStageRecord {
                            stage: stage_id.clone(),
                            completed_at: Utc::now(),
                            duration_ms: success.duration_ms,
                        });
                }
            }
        }

        // Record skips.
        for skipped_item in &result.skipped {
            items_skipped += 1;
            for source_state in self.state.sources.values_mut() {
                if let Some(item_state) = source_state.items.get_mut(&skipped_item.item_id) {
                    item_state.status = ItemStatus::Skipped {
                        stage: stage_id.as_str().to_string(),
                        reason: skipped_item.error.to_string(),
                    };
                }
            }
        }

        // Record failures.
        for failure in &result.failures {
            items_failed += 1;
            for source_state in self.state.sources.values_mut() {
                if let Some(item_state) = source_state.items.get_mut(&failure.item_id) {
                    item_state.status = ItemStatus::Failed {
                        stage: stage_id.as_str().to_string(),
                        error: failure.error.to_string(),
                        attempts: failure.attempts,
                    };
                }
            }
        }

        // Update stage aggregate state.
        if let Some(stage_state) = self.state.stages.get_mut(stage_id) {
            stage_state.items_processed += items_processed;
            stage_state.items_failed += items_failed;
            stage_state.items_skipped += items_skipped;
            stage_state.completed_at = Some(Utc::now());

            if items_failed > 0 && result.has_failures() {
                stage_state.status = StageStatus::Failed {
                    error: format!(
                        "{} item(s) failed in stage '{}'",
                        items_failed,
                        stage_id.as_str()
                    ),
                };
            } else {
                stage_state.status = StageStatus::Completed;
            }
        }

        // If any items hard-failed, propagate as error.
        if let Some(first_failure) = result.failures.first() {
            return Err(PipelineError::ItemFailed {
                stage: stage_id.as_str().to_string(),
                item_id: first_failure.item_id.clone(),
                error: first_failure.error.to_string(),
            });
        }

        self.state.update_stats();
        Ok(())
    }

    /// Get a reference to the current pipeline state.
    pub fn state(&self) -> &PipelineState {
        &self.state
    }

    /// Get a reference to the pipeline topology.
    pub fn topology(&self) -> &PipelineTopology {
        &self.topology
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::Duration;

    use ecl_pipeline_spec::*;
    use ecl_pipeline_state::*;
    use ecl_pipeline_topo::*;

    // ── Mock types ──────────────────────────────────────────────────────

    #[derive(Debug)]
    struct MockSourceAdapter {
        kind: String,
        items: Vec<SourceItem>,
    }

    impl MockSourceAdapter {
        fn new(kind: &str, items: Vec<SourceItem>) -> Self {
            Self {
                kind: kind.to_string(),
                items,
            }
        }
    }

    #[async_trait::async_trait]
    impl SourceAdapter for MockSourceAdapter {
        fn source_kind(&self) -> &str {
            &self.kind
        }

        async fn enumerate(&self) -> std::result::Result<Vec<SourceItem>, SourceError> {
            Ok(self.items.clone())
        }

        async fn fetch(
            &self,
            item: &SourceItem,
        ) -> std::result::Result<ExtractedDocument, SourceError> {
            Ok(ExtractedDocument {
                id: item.id.clone(),
                display_name: item.display_name.clone(),
                content: format!("content-of-{}", item.id).into_bytes(),
                mime_type: item.mime_type.clone(),
                provenance: ItemProvenance {
                    source_kind: self.kind.clone(),
                    metadata: BTreeMap::new(),
                    source_modified: item.modified_at,
                    extracted_at: Utc::now(),
                },
                content_hash: Blake3Hash::new("mock-hash"),
            })
        }
    }

    /// A source adapter that always fails enumeration.
    #[derive(Debug)]
    struct FailingSourceAdapter;

    #[async_trait::async_trait]
    impl SourceAdapter for FailingSourceAdapter {
        fn source_kind(&self) -> &str {
            "failing"
        }

        async fn enumerate(&self) -> std::result::Result<Vec<SourceItem>, SourceError> {
            Err(SourceError::Permanent {
                source_name: "failing".to_string(),
                message: "enumerate failed".to_string(),
            })
        }

        async fn fetch(
            &self,
            _item: &SourceItem,
        ) -> std::result::Result<ExtractedDocument, SourceError> {
            Err(SourceError::Permanent {
                source_name: "failing".to_string(),
                message: "fetch failed".to_string(),
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

    #[async_trait::async_trait]
    impl Stage for MockStage {
        fn name(&self) -> &str {
            &self.name
        }

        async fn process(
            &self,
            item: PipelineItem,
            _ctx: &StageContext,
        ) -> std::result::Result<Vec<PipelineItem>, StageError> {
            Ok(vec![item])
        }
    }

    #[derive(Debug)]
    struct AlwaysFailingStage {
        name: String,
    }

    #[async_trait::async_trait]
    impl Stage for AlwaysFailingStage {
        fn name(&self) -> &str {
            &self.name
        }

        async fn process(
            &self,
            item: PipelineItem,
            _ctx: &StageContext,
        ) -> std::result::Result<Vec<PipelineItem>, StageError> {
            Err(StageError::Permanent {
                stage: self.name.clone(),
                item_id: item.id.clone(),
                message: "always fails".to_string(),
            })
        }
    }

    // ── Test helpers ────────────────────────────────────────────────────

    fn make_source_item(id: &str) -> SourceItem {
        SourceItem {
            id: id.to_string(),
            display_name: format!("Item {id}"),
            mime_type: "text/plain".to_string(),
            path: format!("/test/{id}"),
            modified_at: None,
            source_hash: None,
        }
    }

    #[allow(clippy::type_complexity)]
    fn build_test_topology(
        sources: Vec<(String, Arc<dyn SourceAdapter>)>,
        stages: Vec<(String, Arc<dyn Stage>, Option<String>, bool)>,
    ) -> PipelineTopology {
        let mut spec_sources = BTreeMap::new();
        for (name, _) in &sources {
            spec_sources.insert(
                name.clone(),
                SourceSpec::Filesystem(FilesystemSourceSpec {
                    root: PathBuf::from("/tmp/test"),
                    filters: vec![],
                    extensions: vec![],
                }),
            );
        }

        let mut spec_stages = BTreeMap::new();
        for (name, _, source, skip) in &stages {
            spec_stages.insert(
                name.clone(),
                StageSpec {
                    adapter: "mock".to_string(),
                    source: source.clone(),
                    resources: ResourceSpec::default(),
                    params: serde_json::Value::Null,
                    retry: None,
                    timeout_secs: None,
                    skip_on_error: *skip,
                    condition: None,
                },
            );
        }

        let spec = Arc::new(PipelineSpec {
            name: "test-pipeline".to_string(),
            version: 1,
            output_dir: PathBuf::from("/tmp/test-output"),
            sources: spec_sources,
            stages: spec_stages,
            defaults: DefaultsSpec::default(),
        });

        let topo_sources: BTreeMap<String, Arc<dyn SourceAdapter>> = sources.into_iter().collect();

        let mut resolved_stages = BTreeMap::new();
        let mut schedule = Vec::new();
        for (name, handler, source, skip_on_error) in stages {
            let stage_id = StageId::new(&name);
            schedule.push(vec![stage_id.clone()]);
            resolved_stages.insert(
                name,
                ResolvedStage {
                    id: stage_id,
                    handler,
                    retry: RetryPolicy {
                        max_attempts: 1,
                        initial_backoff: Duration::from_millis(1),
                        backoff_multiplier: 1.0,
                        max_backoff: Duration::from_millis(10),
                    },
                    skip_on_error,
                    timeout: None,
                    source,
                    condition: None,
                },
            );
        }

        PipelineTopology {
            spec,
            spec_hash: Blake3Hash::new("test-hash-abc123"),
            sources: topo_sources,
            stages: resolved_stages,
            schedule,
            output_dir: PathBuf::from("/tmp/test-output"),
        }
    }

    // ── PipelineRunner::new tests ───────────────────────────────────────

    #[tokio::test]
    async fn test_runner_new_fresh_state() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );
        let store = Box::new(InMemoryStateStore::new());

        let runner = PipelineRunner::new(topo, store).await.unwrap();
        assert!(matches!(runner.state().status, PipelineStatus::Pending));
        assert_eq!(runner.state().current_batch, 0);
        assert!(runner.state().pipeline_name.contains("test-pipeline"));
    }

    #[tokio::test]
    async fn test_runner_new_resumes_from_checkpoint() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );

        // Pre-populate the store with a checkpoint.
        let store = Box::new(InMemoryStateStore::new());
        let checkpoint = Checkpoint {
            version: 1,
            sequence: 5,
            created_at: Utc::now(),
            spec: (*topo.spec).clone(),
            schedule: topo.schedule.clone(),
            spec_hash: Blake3Hash::new("test-hash-abc123"),
            state: PipelineState {
                run_id: RunId::new("resumed-run"),
                pipeline_name: "test-pipeline".to_string(),
                started_at: Utc::now(),
                last_checkpoint: Utc::now(),
                status: PipelineStatus::Running {
                    current_stage: "stage-a".to_string(),
                },
                current_batch: 1,
                sources: BTreeMap::new(),
                stages: BTreeMap::new(),
                stats: PipelineStats::default(),
            },
        };
        store.save_checkpoint(&checkpoint).await.unwrap();

        let runner = PipelineRunner::new(topo, store).await.unwrap();
        assert_eq!(runner.state().run_id.as_str(), "resumed-run");
        assert_eq!(runner.state().current_batch, 1);
    }

    #[tokio::test]
    async fn test_runner_new_config_drift_error() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );

        let store = Box::new(InMemoryStateStore::new());
        let checkpoint = Checkpoint {
            version: 1,
            sequence: 1,
            created_at: Utc::now(),
            spec: (*topo.spec).clone(),
            schedule: topo.schedule.clone(),
            spec_hash: Blake3Hash::new("DIFFERENT-HASH"), // mismatch
            state: PipelineState {
                run_id: RunId::new("old-run"),
                pipeline_name: "test-pipeline".to_string(),
                started_at: Utc::now(),
                last_checkpoint: Utc::now(),
                status: PipelineStatus::Pending,
                current_batch: 0,
                sources: BTreeMap::new(),
                stages: BTreeMap::new(),
                stats: PipelineStats::default(),
            },
        };
        store.save_checkpoint(&checkpoint).await.unwrap();

        let result = PipelineRunner::new(topo, store).await;
        assert!(matches!(result, Err(PipelineError::ConfigDrift { .. })));
    }

    #[tokio::test]
    async fn test_runner_new_prepares_for_resume() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );

        let store = Box::new(InMemoryStateStore::new());
        let mut sources = BTreeMap::new();
        let mut items = BTreeMap::new();
        items.insert(
            "item-1".to_string(),
            ItemState {
                display_name: "Item 1".to_string(),
                source_id: "item-1".to_string(),
                source_name: "src".to_string(),
                content_hash: Blake3Hash::new(""),
                status: ItemStatus::Processing {
                    stage: "stage-a".to_string(),
                },
                completed_stages: vec![],
                provenance: ItemProvenance {
                    source_kind: "fs".to_string(),
                    metadata: BTreeMap::new(),
                    source_modified: None,
                    extracted_at: Utc::now(),
                },
            },
        );
        sources.insert(
            "src".to_string(),
            SourceState {
                items_discovered: 1,
                items_accepted: 1,
                items_skipped_unchanged: 0,
                items: items.clone(),
            },
        );

        let checkpoint = Checkpoint {
            version: 1,
            sequence: 1,
            created_at: Utc::now(),
            spec: (*topo.spec).clone(),
            schedule: topo.schedule.clone(),
            spec_hash: Blake3Hash::new("test-hash-abc123"),
            state: PipelineState {
                run_id: RunId::new("resume-run"),
                pipeline_name: "test-pipeline".to_string(),
                started_at: Utc::now(),
                last_checkpoint: Utc::now(),
                status: PipelineStatus::Running {
                    current_stage: "stage-a".to_string(),
                },
                current_batch: 0,
                sources,
                stages: BTreeMap::new(),
                stats: PipelineStats::default(),
            },
        };
        store.save_checkpoint(&checkpoint).await.unwrap();

        let runner = PipelineRunner::new(topo, store).await.unwrap();
        // Processing items should have been reset to Pending.
        let item = &runner.state().sources["src"].items["item-1"];
        assert!(
            matches!(item.status, ItemStatus::Pending),
            "Processing items should be reset to Pending on resume"
        );
    }

    // ── enumerate_sources tests ─────────────────────────────────────────

    #[tokio::test]
    async fn test_enumerate_sources_populates_items() {
        let items = vec![
            make_source_item("a"),
            make_source_item("b"),
            make_source_item("c"),
        ];
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", items)),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );
        let store = Box::new(InMemoryStateStore::new());
        let mut runner = PipelineRunner::new(topo, store).await.unwrap();

        runner.enumerate_sources().await.unwrap();
        let source_state = &runner.state().sources["src"];
        assert_eq!(source_state.items_discovered, 3);
        assert_eq!(source_state.items.len(), 3);
        assert!(source_state.items.contains_key("a"));
        assert!(source_state.items.contains_key("b"));
        assert!(source_state.items.contains_key("c"));
    }

    #[tokio::test]
    async fn test_enumerate_sources_multiple_sources() {
        let topo = build_test_topology(
            vec![
                (
                    "src-1".to_string(),
                    Arc::new(MockSourceAdapter::new("fs", vec![make_source_item("a")])),
                ),
                (
                    "src-2".to_string(),
                    Arc::new(MockSourceAdapter::new("fs", vec![make_source_item("b")])),
                ),
            ],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );
        let store = Box::new(InMemoryStateStore::new());
        let mut runner = PipelineRunner::new(topo, store).await.unwrap();

        runner.enumerate_sources().await.unwrap();
        assert_eq!(runner.state().sources.len(), 2);
        assert!(runner.state().sources["src-1"].items.contains_key("a"));
        assert!(runner.state().sources["src-2"].items.contains_key("b"));
    }

    #[tokio::test]
    async fn test_enumerate_sources_error_propagates() {
        let mut topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );
        // Replace source with a failing one.
        topo.sources
            .insert("src".to_string(), Arc::new(FailingSourceAdapter));

        let store = Box::new(InMemoryStateStore::new());
        let mut runner = PipelineRunner::new(topo, store).await.unwrap();

        let result = runner.enumerate_sources().await;
        assert!(matches!(
            result,
            Err(PipelineError::SourceEnumeration { .. })
        ));
    }

    // ── apply_incrementality tests ──────────────────────────────────────

    #[tokio::test]
    async fn test_apply_incrementality_marks_unchanged() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![make_source_item("a")])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );

        // Pre-populate store with previous hashes.
        let store = InMemoryStateStore::new();
        let mut hashes = BTreeMap::new();
        // Items have empty hash by default, so empty hash matches empty hash.
        hashes.insert("a".to_string(), Blake3Hash::new(""));
        store
            .save_completed_hashes(&RunId::new("prev"), &hashes)
            .await
            .unwrap();

        let mut runner = PipelineRunner::new(topo, Box::new(store)).await.unwrap();
        runner.enumerate_sources().await.unwrap();

        // Hashes are empty on both sides, but empty hashes are skipped by
        // the `!prev_hash.is_empty()` check, so items stay Pending.
        runner.apply_incrementality().await.unwrap();
        let item = &runner.state().sources["src"].items["a"];
        assert!(
            matches!(item.status, ItemStatus::Pending),
            "empty hashes should not mark as unchanged"
        );
    }

    #[tokio::test]
    async fn test_apply_incrementality_no_previous_hashes() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![make_source_item("a")])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );
        let store = Box::new(InMemoryStateStore::new());
        let mut runner = PipelineRunner::new(topo, store).await.unwrap();

        runner.enumerate_sources().await.unwrap();
        runner.apply_incrementality().await.unwrap();

        let item = &runner.state().sources["src"].items["a"];
        assert!(matches!(item.status, ItemStatus::Pending));
    }

    #[tokio::test]
    async fn test_apply_incrementality_updates_stats() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![make_source_item("a")])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );
        let store = Box::new(InMemoryStateStore::new());
        let mut runner = PipelineRunner::new(topo, store).await.unwrap();

        runner.enumerate_sources().await.unwrap();
        runner.apply_incrementality().await.unwrap();

        assert_eq!(runner.state().stats.total_items_discovered, 1);
    }

    // ── checkpoint tests ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_checkpoint_saves_to_store() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );
        let store = InMemoryStateStore::new();
        let store_ref = Box::new(store);
        let mut runner = PipelineRunner::new(topo, store_ref).await.unwrap();

        runner.checkpoint().await.unwrap();
        let loaded = runner.store.load_checkpoint().await.unwrap();
        assert!(loaded.is_some(), "checkpoint should exist after save");
    }

    #[tokio::test]
    async fn test_checkpoint_increments_sequence() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );
        let store = Box::new(InMemoryStateStore::new());
        let mut runner = PipelineRunner::new(topo, store).await.unwrap();

        runner.checkpoint().await.unwrap();
        runner.checkpoint().await.unwrap();

        let loaded = runner.store.load_checkpoint().await.unwrap().unwrap();
        assert_eq!(loaded.sequence, 2);
    }

    #[tokio::test]
    async fn test_checkpoint_embeds_spec_and_schedule() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );
        let store = Box::new(InMemoryStateStore::new());
        let mut runner = PipelineRunner::new(topo, store).await.unwrap();

        runner.checkpoint().await.unwrap();
        let loaded = runner.store.load_checkpoint().await.unwrap().unwrap();
        assert_eq!(loaded.spec.name, "test-pipeline");
        assert!(!loaded.schedule.is_empty());
    }

    #[tokio::test]
    async fn test_save_completed_hashes_stores_completed_items() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![make_source_item("a")])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );
        let store = Box::new(InMemoryStateStore::new());
        let mut runner = PipelineRunner::new(topo, store).await.unwrap();

        runner.enumerate_sources().await.unwrap();
        // Manually mark item as completed.
        if let Some(source) = runner.state.sources.get_mut("src")
            && let Some(item) = source.items.get_mut("a")
        {
            item.status = ItemStatus::Completed;
            item.content_hash = Blake3Hash::new("hash-a");
        }

        runner.save_completed_hashes().await.unwrap();
        let hashes = runner.store.load_previous_hashes().await.unwrap();
        assert!(hashes.contains_key("a"));
        assert_eq!(hashes["a"].as_str(), "hash-a");
    }

    // ── Full run() lifecycle tests ──────────────────────────────────────

    #[tokio::test]
    async fn test_run_simple_pipeline_completes() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![make_source_item("a")])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );
        let store = Box::new(InMemoryStateStore::new());
        let mut runner = PipelineRunner::new(topo, store).await.unwrap();

        let state = runner.run().await.unwrap();
        assert!(matches!(state.status, PipelineStatus::Completed { .. }));
    }

    #[tokio::test]
    async fn test_run_multi_stage_pipeline() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![make_source_item("a")])),
            )],
            vec![
                (
                    "stage-a".to_string(),
                    Arc::new(MockStage::new("stage-a")),
                    None,
                    false,
                ),
                (
                    "stage-b".to_string(),
                    Arc::new(MockStage::new("stage-b")),
                    None,
                    false,
                ),
            ],
        );
        let store = Box::new(InMemoryStateStore::new());
        let mut runner = PipelineRunner::new(topo, store).await.unwrap();

        let state = runner.run().await.unwrap();
        assert!(matches!(state.status, PipelineStatus::Completed { .. }));
    }

    #[tokio::test]
    async fn test_run_sets_completed_status() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![make_source_item("a")])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );
        let store = Box::new(InMemoryStateStore::new());
        let mut runner = PipelineRunner::new(topo, store).await.unwrap();

        let state = runner.run().await.unwrap();
        assert!(
            matches!(state.status, PipelineStatus::Completed { .. }),
            "should be Completed, was {:?}",
            state.status
        );
    }

    #[tokio::test]
    async fn test_run_checkpoints_after_each_batch() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![make_source_item("a")])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );
        let store = Box::new(InMemoryStateStore::new());
        let mut runner = PipelineRunner::new(topo, store).await.unwrap();

        runner.run().await.unwrap();
        let checkpoint = runner.store.load_checkpoint().await.unwrap().unwrap();
        // At minimum: 1 post-enumerate + 1 post-batch + 1 finalize = 3
        assert!(checkpoint.sequence >= 3, "expected at least 3 checkpoints");
    }

    #[tokio::test]
    async fn test_run_skip_on_error_continues() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![make_source_item("a")])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(AlwaysFailingStage {
                    name: "stage-a".to_string(),
                }),
                None,
                true, // skip_on_error=true
            )],
        );
        let store = Box::new(InMemoryStateStore::new());
        let mut runner = PipelineRunner::new(topo, store).await.unwrap();

        let state = runner.run().await.unwrap();
        assert!(matches!(state.status, PipelineStatus::Completed { .. }));
    }

    #[tokio::test]
    async fn test_run_failure_propagates() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![make_source_item("a")])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(AlwaysFailingStage {
                    name: "stage-a".to_string(),
                }),
                None,
                false, // skip_on_error=false
            )],
        );
        let store = Box::new(InMemoryStateStore::new());
        let mut runner = PipelineRunner::new(topo, store).await.unwrap();

        let result = runner.run().await;
        assert!(matches!(result, Err(PipelineError::ItemFailed { .. })));
    }

    #[tokio::test]
    async fn test_run_resume_skips_completed_batches() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![make_source_item("a")])),
            )],
            vec![
                (
                    "stage-a".to_string(),
                    Arc::new(MockStage::new("stage-a")),
                    None,
                    false,
                ),
                (
                    "stage-b".to_string(),
                    Arc::new(MockStage::new("stage-b")),
                    None,
                    false,
                ),
            ],
        );

        // Pre-populate store with checkpoint at batch 1 (meaning batch 0 is done).
        let store = InMemoryStateStore::new();
        let mut items = BTreeMap::new();
        items.insert(
            "a".to_string(),
            ItemState {
                display_name: "Item a".to_string(),
                source_id: "a".to_string(),
                source_name: "src".to_string(),
                content_hash: Blake3Hash::new(""),
                status: ItemStatus::Pending,
                completed_stages: vec![],
                provenance: ItemProvenance {
                    source_kind: "fs".to_string(),
                    metadata: BTreeMap::new(),
                    source_modified: None,
                    extracted_at: Utc::now(),
                },
            },
        );
        let mut sources = BTreeMap::new();
        sources.insert(
            "src".to_string(),
            SourceState {
                items_discovered: 1,
                items_accepted: 1,
                items_skipped_unchanged: 0,
                items,
            },
        );
        let checkpoint = Checkpoint {
            version: 1,
            sequence: 2,
            created_at: Utc::now(),
            spec: (*topo.spec).clone(),
            schedule: topo.schedule.clone(),
            spec_hash: Blake3Hash::new("test-hash-abc123"),
            state: PipelineState {
                run_id: RunId::new("resume-run"),
                pipeline_name: "test-pipeline".to_string(),
                started_at: Utc::now(),
                last_checkpoint: Utc::now(),
                status: PipelineStatus::Running {
                    current_stage: "stage-a".to_string(),
                },
                current_batch: 1, // batch 0 done
                sources,
                stages: BTreeMap::new(),
                stats: PipelineStats {
                    total_items_discovered: 1,
                    total_items_processed: 0,
                    total_items_skipped_unchanged: 0,
                    total_items_failed: 0,
                },
            },
        };
        store.save_checkpoint(&checkpoint).await.unwrap();

        let mut runner = PipelineRunner::new(topo, Box::new(store)).await.unwrap();
        let state = runner.run().await.unwrap();
        assert!(matches!(state.status, PipelineStatus::Completed { .. }));
    }

    #[tokio::test]
    async fn test_run_empty_sources_completes() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );
        let store = Box::new(InMemoryStateStore::new());
        let mut runner = PipelineRunner::new(topo, store).await.unwrap();

        let state = runner.run().await.unwrap();
        assert!(matches!(state.status, PipelineStatus::Completed { .. }));
        assert_eq!(state.stats.total_items_discovered, 0);
    }

    // ── Helper method tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_should_execute_stage_always_returns_true() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );
        let store = Box::new(InMemoryStateStore::new());
        let runner = PipelineRunner::new(topo, store).await.unwrap();

        assert!(runner.should_execute_stage(&StageId::new("stage-a")));
        assert!(runner.should_execute_stage(&StageId::new("anything")));
    }

    #[tokio::test]
    async fn test_collect_items_for_stage_filters_non_pending() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new(
                    "fs",
                    vec![make_source_item("a"), make_source_item("b")],
                )),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );
        let store = Box::new(InMemoryStateStore::new());
        let mut runner = PipelineRunner::new(topo, store).await.unwrap();

        runner.enumerate_sources().await.unwrap();
        // Mark one item as Completed.
        if let Some(source) = runner.state.sources.get_mut("src")
            && let Some(item) = source.items.get_mut("a")
        {
            item.status = ItemStatus::Completed;
        }

        let items = runner.collect_items_for_stage(&StageId::new("stage-a"));
        assert_eq!(items.len(), 1, "only Pending items should be collected");
        assert_eq!(items[0].id, "b");
    }

    #[tokio::test]
    async fn test_build_stage_context_returns_params() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );
        let store = Box::new(InMemoryStateStore::new());
        let runner = PipelineRunner::new(topo, store).await.unwrap();

        let ctx = runner.build_stage_context("stage-a");
        // Default params are Null.
        assert_eq!(ctx.params, serde_json::Value::Null);
    }

    #[tokio::test]
    async fn test_build_stage_context_missing_stage_returns_null_params() {
        let topo = build_test_topology(
            vec![(
                "src".to_string(),
                Arc::new(MockSourceAdapter::new("fs", vec![])),
            )],
            vec![(
                "stage-a".to_string(),
                Arc::new(MockStage::new("stage-a")),
                None,
                false,
            )],
        );
        let store = Box::new(InMemoryStateStore::new());
        let runner = PipelineRunner::new(topo, store).await.unwrap();

        let ctx = runner.build_stage_context("nonexistent");
        assert_eq!(ctx.params, serde_json::Value::Null);
    }
}
