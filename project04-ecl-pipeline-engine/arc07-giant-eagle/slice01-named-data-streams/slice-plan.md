# 2. Milestone 8.1: Named Data Streams

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0027-phase-2-detailed-implementation-plan-make-giant-eagle-work.md, lines 212-605. Original bytes are recoverable from import commit 3a39aea. -->

## 2. Milestone 8.1: Named Data Streams

### 2.1 Scope

Add stream tagging to `PipelineItem`, `SourceSpec`, and `StageSpec`. Update runner to route items by stream.

### 2.2 Changes to `ecl-pipeline-topo/src/traits.rs`

**Add `stream` field to `PipelineItem`:**

```rust
pub struct PipelineItem {
    // ... existing fields ...
    pub id: String,
    pub display_name: String,
    pub content: Arc<[u8]>,
    pub mime_type: String,
    pub source_name: String,
    pub source_content_hash: Blake3Hash,
    pub provenance: ItemProvenance,
    pub metadata: BTreeMap<String, serde_json::Value>,
    pub record: Option<Record>,  // from Phase 1 (7.1)

    /// Named data stream this item belongs to. Items with no stream (None)
    /// are visible to all stages (backward compatible with Phase 1 pipelines).
    /// Set by source or by runner when stage has output_stream configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
}
```

### 2.3 Changes to `ecl-pipeline-spec/src/source.rs`

**Add `stream` to all source specs (or to the SourceSpec level):**

Since `SourceSpec` is an internally-tagged enum, add stream at each variant level or — cleaner — add a wrapper:

```rust
/// In the TOML, stream is a sibling of kind:
/// [sources.transactions]
/// kind = "gcs"
/// stream = "transactions"
/// bucket = "..."
///
/// We handle this by adding `stream` to each variant's spec.

// Add to GcsSourceSpec (and optionally to others):
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcsSourceSpec {
    // ... existing fields ...
    /// Named data stream for items from this source.
    #[serde(default)]
    pub stream: Option<String>,
}
```

**Alternative approach (recommended):** Since stream applies to ALL source types uniformly, extract it as a wrapper in `PipelineSpec`:

```rust
/// In PipelineSpec, sources become:
pub sources: BTreeMap<String, SourceEntry>,

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceEntry {
    /// Named data stream for items from this source.
    #[serde(default)]
    pub stream: Option<String>,
    /// The actual source configuration.
    #[serde(flatten)]
    pub spec: SourceSpec,
}
```

**However**, `#[serde(flatten)]` with internally-tagged enums can be tricky. Safest approach: add `stream: Option<String>` directly to each source spec struct. This is more repetitive but avoids serde edge cases.

**Decision:** Add `stream` to each `*SourceSpec` struct for safety. It's 5 lines across 5 structs.

### 2.4 Changes to `ecl-pipeline-spec/src/stage.rs`

**Add stream routing to `StageSpec`:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageSpec {
    // ... existing fields ...
    pub adapter: String,
    pub source: Option<String>,
    pub resources: ResourceSpec,
    pub params: serde_json::Value,
    pub retry: Option<RetrySpec>,
    pub timeout_secs: Option<u64>,
    pub skip_on_error: bool,
    pub condition: Option<String>,

    /// Input stream(s) this stage reads from.
    /// Empty = reads from all streams (backward compatible).
    #[serde(default)]
    pub input_streams: Vec<String>,

    /// Output stream produced by this stage.
    /// None = items retain their current stream tag.
    #[serde(default)]
    pub output_stream: Option<String>,
}
```

### 2.5 Changes to Runner: Stream-Aware Item Collection

#### File: `crates/ecl-pipeline/src/runner.rs`

**Update `collect_items_for_stage`:**

```rust
fn collect_items_for_stage(&self, stage_id: &StageId) -> Vec<PipelineItem> {
    let stage_name = stage_id.as_str();

    // Get the stage's input_streams from the spec.
    let input_streams: &[String] = self.topology.spec.stages
        .get(stage_name)
        .map(|s| s.input_streams.as_slice())
        .unwrap_or(&[]);

    let mut items = Vec::new();

    // Collect from source items.
    for source_state in self.state.sources.values() {
        for item_state in source_state.items.values() {
            if !matches!(item_state.status, ItemStatus::Pending) {
                continue;
            }
            items.push(self.build_pipeline_item(item_state));
        }
    }

    // Collect from stage output items (produced by previous batches).
    // These are stored in a new field: self.stage_outputs
    items.extend(self.stage_outputs.iter()
        .filter(|item| matches_stream(item, input_streams))
        .cloned());

    items
}

/// Check if an item matches the stage's input streams.
/// Empty input_streams = match all (backward compatible).
fn matches_stream(item: &PipelineItem, input_streams: &[String]) -> bool {
    if input_streams.is_empty() {
        return true;  // No stream filter = accept all
    }
    match &item.stream {
        Some(stream) => input_streams.contains(stream),
        None => true,  // Untagged items are visible to all
    }
}
```

**New field on PipelineRunner:**

```rust
pub struct PipelineRunner {
    // ... existing fields ...
    /// Items produced by completed stages, available for downstream stages.
    /// Key insight: stage outputs are NOT tracked in PipelineState (they're transient).
    /// They exist only during a single run. On resume, stages re-execute from checkpoint.
    stage_outputs: Vec<PipelineItem>,
}
```

**Update `merge_stage_result` to capture output items:**

```rust
fn merge_stage_result(&mut self, result: StageResult) -> Result<()> {
    let stage_id = &result.stage_id;
    let stage_name = stage_id.as_str();

    // Determine output_stream tag for this stage's outputs.
    let output_stream = self.topology.spec.stages
        .get(stage_name)
        .and_then(|s| s.output_stream.clone());

    // ... existing success/skip/failure recording ...

    // NEW: Capture output items from successes for downstream stages.
    for success in &result.successes {
        for mut output_item in success.outputs.clone() {
            // Tag with output_stream if configured.
            if let Some(ref stream) = output_stream {
                output_item.stream = Some(stream.clone());
            }
            self.stage_outputs.push(output_item);
        }
    }

    // ... rest of existing logic ...
}
```

**Critical change to item status model:** Currently, `merge_stage_result` marks items as `Completed` after each stage. But in a multi-stage pipeline, an item needs to flow through multiple stages. The current model works because items are re-collected as `Pending` for each batch.

Wait — looking at the current code more carefully: `collect_items_for_stage` only returns items with `ItemStatus::Pending`. After a stage marks them `Completed`, they won't be collected for the next batch. This works for the current single-stage-per-item model, but breaks for multi-stage.

**The fix:** In Phase 1, this was already an issue (csv_parse → field_map → validate → kafka_sink is 4 stages). The current runner marks items Completed after the *first* stage, so they'd be invisible to stage 2.

**Looking more carefully at the runner:** The schedule has batches, and within a batch, stages run in parallel. The FULL pipeline flows through batches sequentially. So the model is:

- Batch 0: [extract] — produces items, marks originals Completed
- Batch 1: [normalize] — needs to see extract's output items
- etc.

The current code has a problem: stage outputs (fan-out) aren't captured. Let me look at how the existing integration tests handle multi-stage...

Actually, re-reading `execute_batch` and `merge_stage_result` — the current model treats each source item as a single unit flowing through ALL stages. Items aren't re-collected between batches; they're marked Completed/Failed after processing. The `collect_items_for_stage` just returns Pending items.

For fan-out (csv_parse producing N rows), the output items from stage 1 need to become the input for stage 2. The current runner doesn't handle this.

**This is a fundamental change needed for Phase 2 AND Phase 1.** Let me design it properly.

### 2.6 Item Flow Model: Output Propagation

**New model:** Stages produce output items. These outputs become the input to downstream stages. The runner maintains a pool of "active items" that grows as stages produce fan-out and shrinks as terminal stages consume them.

```rust
pub struct PipelineRunner {
    // ... existing fields ...
    /// Pool of active items available for stage execution.
    /// Populated initially by source enumeration.
    /// Grows as stages produce fan-out (csv_parse: 1 file → N rows).
    /// Items are tagged with streams for routing.
    active_items: Vec<PipelineItem>,
}
```

**Execution flow per batch:**

```
1. For each stage in batch:
   a. Collect items matching stage's input_streams from active_items
   b. Execute stage (per-item or batch mode)
   c. Remove consumed items from active_items
   d. Add output items (tagged with output_stream) to active_items
2. Checkpoint
```

**Backward compatibility:** For Phase 1 pipelines (no streams), all items are visible to all stages, and output items replace input items. This is equivalent to the current behavior.

### 2.7 Updated Runner Methods

```rust
impl PipelineRunner {
    async fn execute_batch(&mut self, batch_idx: usize, stages: &[StageId]) -> Result<()> {
        // ... existing batch logging ...

        let active_stages: Vec<&StageId> = stages.iter()
            .filter(|id| self.should_execute_stage(id))
            .collect();

        // For each stage in the batch (they can still run concurrently
        // if they operate on different streams):
        let mut join_set = tokio::task::JoinSet::new();

        for stage_id in &active_stages {
            let stage_name = stage_id.as_str().to_string();
            let stage = self.topology.stages.get(&stage_name)
                .ok_or_else(|| /* error */)?
                .clone();

            let input_streams = self.topology.spec.stages
                .get(&stage_name)
                .map(|s| s.input_streams.clone())
                .unwrap_or_default();

            // Collect items matching this stage's input streams.
            let items: Vec<PipelineItem> = self.active_items.iter()
                .filter(|item| matches_stream(item, &input_streams))
                .cloned()
                .collect();

            let ctx = self.build_stage_context(&stage_name);
            let concurrency = self.topology.spec.defaults.concurrency;
            let requires_batch = stage.handler.requires_batch();

            if requires_batch {
                // Batch mode: pass all items at once.
                join_set.spawn(async move {
                    execute_stage_batch(stage, items, ctx).await
                });
            } else {
                // Per-item mode: existing concurrent execution.
                join_set.spawn(async move {
                    execute_stage_items(stage, items, ctx, concurrency).await
                });
            }
        }

        // Collect results.
        while let Some(result) = join_set.join_next().await {
            let stage_result = result??;
            self.apply_stage_outputs(stage_result)?;
        }

        Ok(())
    }

    /// Apply stage outputs: remove consumed items, add produced items.
    fn apply_stage_outputs(&mut self, result: StageResult) -> Result<()> {
        let stage_name = result.stage_id.as_str();
        let output_stream = self.topology.spec.stages
            .get(stage_name)
            .and_then(|s| s.output_stream.clone());

        // Remove consumed items (those that were inputs to this stage).
        let consumed_ids: std::collections::HashSet<String> = result.successes.iter()
            .map(|s| s.item_id.clone())
            .chain(result.skipped.iter().map(|s| s.item_id.clone()))
            .chain(result.failures.iter().map(|s| s.item_id.clone()))
            .collect();
        self.active_items.retain(|item| !consumed_ids.contains(&item.id));

        // Add output items, tagged with output_stream.
        for success in result.successes {
            for mut output in success.outputs {
                if let Some(ref stream) = output_stream {
                    output.stream = Some(stream.clone());
                }
                self.active_items.push(output);
            }
        }

        // Update state statistics...
        self.merge_stage_result_stats(&result)?;

        Ok(())
    }
}
```

### 2.8 Source Enumeration: Stream Tagging

**Update `enumerate_sources`:** When creating `PipelineItem`s from source items, tag them with the source's stream:

```rust
async fn enumerate_sources(&mut self) -> Result<()> {
    for (name, adapter) in &self.topology.sources {
        let items = adapter.enumerate().await?;

        // Get stream tag from source spec.
        let stream = self.topology.spec.sources.get(name)
            .and_then(|s| source_stream(s));

        for item in items {
            let mut pipeline_item = PipelineItem {
                id: item.id.clone(),
                // ... existing fields ...
                stream: stream.clone(),
                record: None,
            };
            self.active_items.push(pipeline_item);
            // ... existing ItemState tracking ...
        }
    }
    Ok(())
}

fn source_stream(spec: &SourceSpec) -> Option<String> {
    match spec {
        SourceSpec::Gcs(s) => s.stream.clone(),
        SourceSpec::Filesystem(s) => s.stream.clone(),
        // ... etc
    }
}
```

### 2.9 Tests

1. `test_stream_tag_on_pipeline_item_serde` — roundtrip with stream
2. `test_stream_tag_none_backward_compatible` — old items work
3. `test_matches_stream_empty_input_accepts_all`
4. `test_matches_stream_filters_by_name`
5. `test_matches_stream_untagged_visible_to_all`
6. `test_collect_items_filters_by_stream`
7. `test_stage_output_gets_output_stream_tag`
8. `test_active_items_consumed_after_stage`
9. `test_multi_stream_two_sources_independent_stages`
10. `test_fan_out_items_available_to_next_batch`
11. `test_backward_compat_no_streams_all_stages_see_all_items`

**Rust patterns:**

- AP-14: Don't collect into Vec unnecessarily — use iterators where possible
- CA-06: Consider cancellation safety for JoinSet batch execution
- TR-10: Minimal bounds — `matches_stream` takes `&[String]` not `&Vec<String>`

---

