# 0029 — Phase 2 Detailed Implementation Plan: "Make Giant Eagle Work"

**Date:** 2026-03-17
**Status:** Draft
**Depends on:** 0028 (Phase 1 Plan)
**Goal:** Run the Giant Eagle (Partner 296) pipeline end-to-end in ECL — multiple CSV file types parsed independently, joined by key, aggregated, assembled into receipts, validated, and output to Kafka.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Milestone 8.1: Named Data Streams](#2-milestone-81-named-data-streams)
3. [Milestone 8.2: Batch Stage Trait & Join Stage](#3-milestone-82-batch-stage-trait--join-stage)
4. [Milestone 8.3: Aggregation Stage](#4-milestone-83-aggregation-stage)
5. [Milestone 8.4: Lookup Table Stage](#5-milestone-84-lookup-table-stage)
6. [Milestone 8.5: Date/Time & Timezone Stages](#6-milestone-85-datetime--timezone-stages)
7. [Milestone 8.6: ZIP Decompression Stage](#7-milestone-86-zip-decompression-stage)
8. [Milestone 8.7: Receipt Assembly Stage](#8-milestone-87-receipt-assembly-stage)
9. [Milestone 8.8: Giant Eagle End-to-End Integration Test](#9-milestone-88-giant-eagle-end-to-end-integration-test)
10. [Cross-Cutting Concerns](#10-cross-cutting-concerns)
11. [Verification Checklist](#11-verification-checklist)

---

## 1. Architecture Overview

### 1.1 Target Pipeline Shape (Giant Eagle)

```
GCS (stores.csv)       ─→ decompress ─→ parse ─→ map ─→ tz_convert ─────────────────────────┐
GCS (transactions.csv) ─→ decompress ─→ parse ─→ map ─→ date_parse ─────────────────────────┤
GCS (items.csv)        ─→ decompress ─→ parse ─→ map ───────┐                               │
GCS (products.csv)     ─→ decompress ─→ parse ─→ map ───────┤                               │
                                                              ├─→ join(items+products by UPC) │
                                                              │                               │
GCS (tenders.csv)      ─→ decompress ─→ parse ─→ map ─→ lookup(payment types) ──────────────┤
                                                              │                               │
                                                              ├─→ aggregate(tenders by txn)   │
                                                              │                               │
                                                              └───────────────────────────────┤
                                                                                              │
                                    assemble_receipt ←────────────────────────────────────────┘
                                         │
                                         ├─→ validate ─→ kafka_sink (receipts)
                                         └─→ taxonomy_extract ─→ kafka_sink (taxonomy)
```

### 1.2 TOML Spec Preview (Giant Eagle, abbreviated)

```toml
name = "giant-eagle-296-transformation"
version = 1
output_dir = "./output/giant-eagle-296"

# ── Sources (one per file type) ─────────────────────────
[sources.stores]
kind = "gcs"
bucket = "byn-prod-merchant-296-giant-eagle-inc"
prefix = "staging/"
pattern = "*store*.csv"
stream = "stores"

[sources.transactions]
kind = "gcs"
bucket = "byn-prod-merchant-296-giant-eagle-inc"
prefix = "staging/"
pattern = "*transaction*.csv"
stream = "transactions"

[sources.items]
kind = "gcs"
bucket = "byn-prod-merchant-296-giant-eagle-inc"
prefix = "staging/"
pattern = "*item*.csv"
stream = "items"

[sources.products]
kind = "gcs"
bucket = "byn-prod-merchant-296-giant-eagle-inc"
prefix = "staging/"
pattern = "*product*.csv"
stream = "products"

[sources.tenders]
kind = "gcs"
bucket = "byn-prod-merchant-296-giant-eagle-inc"
prefix = "staging/"
pattern = "*tender*.csv"
stream = "tenders"

# ── Per-stream parse stages ─────────────────────────────
[stages.parse-stores]
adapter = "csv_parse"
input_streams = ["stores"]
output_stream = "parsed-stores"
resources = { reads = ["stores-files"], creates = ["parsed-stores"] }
[stages.parse-stores.params]
has_headers = false
columns = [
  { name = "merchant_name", type = "string" },
  { name = "store_id", type = "string" },
  # ... 12 columns
]

[stages.parse-transactions]
adapter = "csv_parse"
input_streams = ["transactions"]
output_stream = "parsed-transactions"
resources = { reads = ["txn-files"], creates = ["parsed-transactions"] }
[stages.parse-transactions.params]
has_headers = true
columns = [
  { name = "TRANSACTION_ID", type = "string" },
  # ... 12 columns
]

# ... similar for items, products, tenders

# ── Lookup stage (payment types) ────────────────────────
[stages.map-payment-types]
adapter = "lookup"
input_streams = ["mapped-tenders"]
output_stream = "typed-tenders"
resources = { reads = ["mapped-tenders"], creates = ["typed-tenders"] }
[stages.map-payment-types.params]
lookups = [
  { field = "payment_type", output = "canonical_payment_type", default = "OTHER",
    table = { "Credit" = "CREDIT", "Debit" = "DEBIT", "Cash" = "CASH", "EBT" = "EBT" } },
  { field = "payment_scheme", output = "canonical_scheme",
    table = { "Visa" = "VI", "V" = "VI", "Mastercard" = "MC", "M" = "MC",
              "Discover" = "DS", "D" = "DS", "Amex" = "AX" } },
]

# ── Join stage (items + products by UPC) ────────────────
[stages.join-items-products]
adapter = "join"
input_streams = ["mapped-items", "mapped-products"]
output_stream = "enriched-items"
resources = { reads = ["mapped-items", "mapped-products"], creates = ["enriched-items"] }
[stages.join-items-products.params]
join_type = "left"
left_stream = "mapped-items"
right_stream = "mapped-products"
left_key = "upc"
right_key = "upc"

# ── Aggregation stage (tenders by transaction) ──────────
[stages.aggregate-tenders]
adapter = "aggregate"
input_streams = ["typed-tenders"]
output_stream = "aggregated-tenders"
resources = { reads = ["typed-tenders"], creates = ["aggregated-tenders"] }
[stages.aggregate-tenders.params]
group_by = ["transaction_id"]
aggregates = [
  { field = "payment_amt", function = "sum", output = "total_charged" },
  { field = "tax_amount", function = "max", output = "tax_amount" },
  { field = "shipping_amount", function = "max", output = "shipping_amount" },
]
collect_arrays = [
  { output = "payments", fields = ["payment_type", "scheme", "card_last_four", "bin", "auth_code", "arn", "payment_amt"] },
]

# ── Assembly stage ──────────────────────────────────────
[stages.assemble]
adapter = "assemble"
input_streams = ["parsed-stores", "parsed-transactions", "enriched-items", "aggregated-tenders"]
output_stream = "receipts"
resources = { reads = ["parsed-stores", "parsed-transactions", "enriched-items", "aggregated-tenders"],
              creates = ["receipts"] }
[stages.assemble.params]
primary_stream = "parsed-transactions"
primary_key = "transaction_id"
joins = [
  { stream = "parsed-stores", key = "store_id", foreign_key = "store_id", nest_as = "store" },
  { stream = "enriched-items", key = "transaction_id", foreign_key = "transaction_id", nest_as = "items", collect = true },
  { stream = "aggregated-tenders", key = "transaction_id", foreign_key = "transaction_id", nest_as = "payment" },
]

# ── Validate + Output ──────────────────────────────────
[stages.validate]
adapter = "validate"
input_streams = ["receipts"]
output_stream = "validated"
resources = { reads = ["receipts"], creates = ["validated"] }

[stages.kafka-receipts]
adapter = "kafka_sink"
input_streams = ["validated"]
resources = { reads = ["validated"] }
[stages.kafka-receipts.params]
topic = "by_production_296_giant-eagle-inc_receipt-accept_canonical_batch_avro_0"
filter = "valid_only"
# ... kafka config
```

### 1.3 Key Design Decisions

**D1: Stream as a tag on PipelineItem, not a separate collection.** Items carry a `stream: Option<String>` field. Sources tag items at creation. Stages declare `input_streams` and `output_stream` to control routing. The runner filters items by stream when collecting for a stage.

**D2: Batch stages via `process_batch` on Stage trait.** Add a default method `process_batch(items: Vec<PipelineItem>, ctx: &StageContext) -> Result<Vec<PipelineItem>>` that calls `process` per item. Join and Aggregate stages override this to process all items at once. A `requires_batch() -> bool` flag tells the runner which path to take.

**D3: Stage output items get the stage's `output_stream` tag.** When a stage produces output items, the runner tags them with the stage's configured `output_stream`. This keeps stream routing out of individual stage implementations.

**D4: Items accumulate across batches.** After a batch executes, its output items are added to the item pool. The next batch's stages see both the original source items and any newly produced items, filtered by their `input_streams`. Items that have been consumed (Completed status) are still visible to later stages if they share the same stream — but the `collect_items_for_stage` method checks both stream AND Pending status.

**D5: Fan-out stages (CSV parse) produce items in the output_stream.** A csv_parse stage reading from stream "transactions" produces items in stream "parsed-transactions". This makes stream routing explicit in the TOML spec.

---

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

## 3. Milestone 8.2: Batch Stage Trait & Join Stage

### 3.1 Scope

Extend the `Stage` trait with batch processing support. Implement a `JoinStage` that merges records from two streams by key.

### 3.2 Stage Trait Extension

#### File: `crates/ecl-pipeline-topo/src/traits.rs`

Add default methods to the `Stage` trait:

```rust
#[async_trait]
pub trait Stage: Send + Sync + Debug {
    fn name(&self) -> &str;

    /// Process a single item. Default implementation for per-item stages.
    async fn process(
        &self,
        item: PipelineItem,
        ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError>;

    /// Whether this stage requires all items at once (join, aggregate).
    /// Default: false (per-item processing).
    fn requires_batch(&self) -> bool {
        false
    }

    /// Process all items as a batch. Default calls process() per item.
    /// Override for join/aggregate stages that need cross-item visibility.
    async fn process_batch(
        &self,
        items: Vec<PipelineItem>,
        ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let mut results = Vec::new();
        for item in items {
            results.extend(self.process(item, ctx).await?);
        }
        Ok(results)
    }
}
```

**Backward compatibility:** All existing stages inherit the default `requires_batch() -> false` and default `process_batch` that delegates to `process`. Zero changes to ExtractStage, NormalizeStage, FilterStage, EmitStage, CsvParseStage, FieldMapStage, ValidateStage.

### 3.3 Batch Execution Function

#### File: `crates/ecl-pipeline/src/batch.rs`

Add a new function for batch-mode execution:

```rust
/// Execute a batch stage: all items processed together.
///
/// Unlike execute_stage_items which processes items concurrently,
/// this passes ALL items to the stage's process_batch method at once.
/// Used for join and aggregation stages that need cross-item visibility.
pub async fn execute_stage_batch(
    stage: ResolvedStage,
    items: Vec<PipelineItem>,
    ctx: StageContext,
) -> std::result::Result<StageResult, PipelineError> {
    let stage_name = stage.id.as_str().to_string();
    let item_count = items.len();
    tracing::info!(stage = %stage_name, items = item_count, "starting batch stage");

    let start = std::time::Instant::now();
    let item_ids: Vec<String> = items.iter().map(|i| i.id.clone()).collect();

    let mut stage_result = StageResult::new(stage.id.clone());

    match stage.handler.process_batch(items, &ctx).await {
        Ok(outputs) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::info!(
                stage = %stage_name,
                input_items = item_count,
                output_items = outputs.len(),
                duration_ms,
                "batch stage completed"
            );
            // Record all input items as successful with the batch outputs.
            // The first item gets all outputs; others get empty.
            // (Alternatively, distribute or use a batch-level success model.)
            if let Some(first_id) = item_ids.first() {
                stage_result.record_success(first_id.clone(), outputs, duration_ms);
            }
            for id in item_ids.iter().skip(1) {
                stage_result.record_success(id.clone(), vec![], duration_ms);
            }
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::error!(stage = %stage_name, error = %e, "batch stage failed");
            for id in &item_ids {
                if stage.skip_on_error {
                    stage_result.record_skipped(id.clone(), e.clone());
                } else {
                    stage_result.record_failure(id.clone(), e.clone(), 1);
                }
            }
        }
    }

    Ok(stage_result)
}
```

**Note:** `StageError` needs `Clone`. Check if it already derives Clone — if not, add it. (Looking at the error types: they contain only `String` and `u64` fields, so `Clone` is trivially derivable.)

### 3.4 JoinStage Implementation

#### File: `crates/ecl-stages/src/join.rs` (new)

**Configuration:**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct JoinConfig {
    /// Join type: "inner", "left", "full"
    #[serde(default = "default_left")]
    pub join_type: String,
    /// Name of the left (primary) stream.
    pub left_stream: String,
    /// Name of the right (secondary) stream.
    pub right_stream: String,
    /// Key field in left stream records.
    pub left_key: String,
    /// Key field in right stream records.
    pub right_key: String,
    /// Prefix for right-side fields to avoid name collisions.
    /// Default: "{right_stream}_" (e.g., "products_brand")
    #[serde(default)]
    pub right_prefix: Option<String>,
}
```

**Implementation:**

```rust
#[derive(Debug)]
pub struct JoinStage {
    config: JoinConfig,
}

impl JoinStage {
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: JoinConfig = serde_json::from_value(params.clone())
            .map_err(|e| StageError::Permanent { /* ... */ })?;
        Ok(Self { config })
    }
}

#[async_trait]
impl Stage for JoinStage {
    fn name(&self) -> &str { "join" }

    fn requires_batch(&self) -> bool { true }

    async fn process(
        &self,
        _item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // Should not be called — requires_batch is true.
        Err(StageError::Permanent {
            stage: "join".to_string(),
            item_id: String::new(),
            message: "join stage requires batch mode".to_string(),
        })
    }

    async fn process_batch(
        &self,
        items: Vec<PipelineItem>,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // 1. Partition items by stream.
        let (left_items, right_items): (Vec<_>, Vec<_>) = items.into_iter()
            .partition(|item| {
                item.stream.as_deref() == Some(&self.config.left_stream)
            });

        // 2. Build right-side lookup: key → Vec<Record>
        let mut right_index: HashMap<String, Vec<&Record>> = HashMap::new();
        for item in &right_items {
            if let Some(record) = &item.record {
                if let Some(Value::String(key)) = record.get(&self.config.right_key) {
                    right_index.entry(key.clone()).or_default().push(record);
                }
            }
        }

        // 3. For each left item, lookup and merge.
        let mut results = Vec::new();
        let right_prefix = self.config.right_prefix.as_deref()
            .unwrap_or(&format!("{}_", self.config.right_stream));

        for left_item in left_items {
            let left_record = left_item.record.as_ref()
                .ok_or_else(|| StageError::Permanent { /* ... */ })?;
            let left_key = left_record.get(&self.config.left_key)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            match right_index.get(left_key) {
                Some(right_records) => {
                    // Matched: merge first right record's fields into left.
                    // (For one-to-many, could produce multiple items.)
                    let mut merged = left_record.clone();
                    if let Some(right_record) = right_records.first() {
                        for (key, value) in *right_record {
                            if key != &self.config.right_key {
                                merged.insert(
                                    format!("{right_prefix}{key}"),
                                    value.clone(),
                                );
                            }
                        }
                    }
                    results.push(PipelineItem {
                        record: Some(merged),
                        ..left_item
                    });
                }
                None => {
                    match self.config.join_type.as_str() {
                        "inner" => {
                            // Drop unmatched left items.
                        }
                        "left" | "full" => {
                            // Keep left item with null right fields.
                            results.push(left_item);
                        }
                        _ => {
                            results.push(left_item);
                        }
                    }
                }
            }
        }

        // 4. For full join, add unmatched right items.
        if self.config.join_type == "full" {
            let matched_keys: HashSet<&str> = results.iter()
                .filter_map(|i| i.record.as_ref()?.get(&self.config.left_key)?.as_str())
                .collect();
            for right_item in right_items {
                if let Some(record) = &right_item.record {
                    let key = record.get(&self.config.right_key)
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !matched_keys.contains(key) {
                        results.push(right_item);
                    }
                }
            }
        }

        Ok(results)
    }
}
```

**Memory considerations:**
- Right-side index is `HashMap<String, Vec<&Record>>` — borrows from items, no cloning
- For Giant Eagle, the right side (products) is typically smaller than left (items)
- For very large datasets (>1M rows), consider a streaming merge or spill-to-disk strategy in a future milestone

### 3.5 Tests

1. `test_join_inner_basic` — 3 left + 2 right, 2 matches → 2 output
2. `test_join_left_basic` — 3 left + 2 right, 2 matches → 3 output (1 unmatched)
3. `test_join_full_basic` — 3 left + 2 right → 4 output (incl unmatched right)
4. `test_join_right_prefix_applied`
5. `test_join_no_matches_left` — all left items pass through with null rights
6. `test_join_no_matches_inner` — empty result
7. `test_join_duplicate_keys_right` — first match used
8. `test_join_missing_key_field` — items without key field skipped
9. `test_join_no_record_returns_error`
10. `test_join_requires_batch_true`
11. `test_join_process_returns_error` — direct process() call fails
12. `test_join_empty_inputs`
13. `test_join_ge_items_products` — realistic Giant Eagle UPC join

---

## 4. Milestone 8.3: Aggregation Stage

### 4.1 Scope

Group records by key and apply aggregate functions (sum, max, min, count, first, last, collect).

### 4.2 Implementation

#### File: `crates/ecl-stages/src/aggregate.rs` (new)

**Configuration:**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct AggregateConfig {
    /// Fields to group by.
    pub group_by: Vec<String>,
    /// Aggregate function definitions.
    #[serde(default)]
    pub aggregates: Vec<AggregateOp>,
    /// Collect sub-records into arrays.
    #[serde(default)]
    pub collect_arrays: Vec<CollectArrayOp>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AggregateOp {
    /// Source field to aggregate.
    pub field: String,
    /// Aggregate function: "sum", "max", "min", "count", "first", "last", "avg"
    pub function: String,
    /// Output field name for the aggregate result.
    pub output: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CollectArrayOp {
    /// Output field name (will be a JSON array).
    pub output: String,
    /// Fields to include in each array element.
    pub fields: Vec<String>,
}
```

**Stage implementation:**

```rust
#[derive(Debug)]
pub struct AggregateStage {
    config: AggregateConfig,
}

#[async_trait]
impl Stage for AggregateStage {
    fn name(&self) -> &str { "aggregate" }
    fn requires_batch(&self) -> bool { true }

    async fn process(
        &self,
        _item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        Err(StageError::Permanent {
            stage: "aggregate".to_string(),
            item_id: String::new(),
            message: "aggregate requires batch mode".to_string(),
        })
    }

    async fn process_batch(
        &self,
        items: Vec<PipelineItem>,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // 1. Group items by composite key (group_by fields).
        let mut groups: BTreeMap<String, Vec<(PipelineItem, Record)>> = BTreeMap::new();

        for item in items {
            let record = item.record.clone()
                .ok_or_else(|| StageError::Permanent { /* ... */ })?;
            let key = self.compute_group_key(&record);
            groups.entry(key).or_default().push((item, record));
        }

        // 2. For each group, compute aggregates.
        let mut results = Vec::new();
        for (_key, group) in groups {
            let (first_item, _) = &group[0];
            let records: Vec<&Record> = group.iter().map(|(_, r)| r).collect();

            let mut output_record = Record::new();

            // Copy group_by fields from first record.
            for field in &self.config.group_by {
                if let Some(val) = records[0].get(field) {
                    output_record.insert(field.clone(), val.clone());
                }
            }

            // Compute aggregates.
            for agg in &self.config.aggregates {
                let result = self.compute_aggregate(&agg.function, &agg.field, &records);
                output_record.insert(agg.output.clone(), result);
            }

            // Collect arrays.
            for collect in &self.config.collect_arrays {
                let array: Vec<Value> = records.iter().map(|r| {
                    let mut obj = serde_json::Map::new();
                    for field in &collect.fields {
                        if let Some(val) = r.get(field) {
                            obj.insert(field.clone(), val.clone());
                        }
                    }
                    Value::Object(obj)
                }).collect();
                output_record.insert(collect.output.clone(), Value::Array(array));
            }

            results.push(PipelineItem {
                id: format!("{}:agg:{}", first_item.id, _key),
                record: Some(output_record),
                ..first_item.clone()
            });
        }

        Ok(results)
    }
}

impl AggregateStage {
    fn compute_group_key(&self, record: &Record) -> String {
        self.config.group_by.iter()
            .map(|f| record.get(f)
                .and_then(|v| v.as_str())
                .unwrap_or(""))
            .collect::<Vec<_>>()
            .join("|")
    }

    fn compute_aggregate(
        &self,
        function: &str,
        field: &str,
        records: &[&Record],
    ) -> Value {
        let values: Vec<f64> = records.iter()
            .filter_map(|r| r.get(field))
            .filter_map(|v| match v {
                Value::Number(n) => n.as_f64(),
                Value::String(s) => s.parse::<f64>().ok(),
                _ => None,
            })
            .collect();

        match function {
            "sum" => Value::from(values.iter().sum::<f64>()),
            "max" => values.iter().cloned().reduce(f64::max)
                .map(Value::from).unwrap_or(Value::Null),
            "min" => values.iter().cloned().reduce(f64::min)
                .map(Value::from).unwrap_or(Value::Null),
            "avg" => if values.is_empty() { Value::Null }
                else { Value::from(values.iter().sum::<f64>() / values.len() as f64) },
            "count" => Value::from(values.len() as i64),
            "first" => records.first()
                .and_then(|r| r.get(field).cloned())
                .unwrap_or(Value::Null),
            "last" => records.last()
                .and_then(|r| r.get(field).cloned())
                .unwrap_or(Value::Null),
            _ => Value::Null,
        }
    }
}
```

### 4.3 Tests

1. `test_aggregate_sum_basic` — 3 rows, group by A, sum B
2. `test_aggregate_max_min` — verify max/min functions
3. `test_aggregate_count` — count per group
4. `test_aggregate_first_last` — first/last in group
5. `test_aggregate_avg` — average calculation
6. `test_aggregate_collect_array` — sub-records collected into array
7. `test_aggregate_composite_key` — group_by two fields
8. `test_aggregate_string_to_numeric_coercion` — "25.99" → f64
9. `test_aggregate_empty_input`
10. `test_aggregate_single_group` — all same key
11. `test_aggregate_ge_tenders` — Giant Eagle tender aggregation

---

## 5. Milestone 8.4: Lookup Table Stage

### 5.1 Scope

Static value mapping for payment types, schemes, and other enumerated fields.

### 5.2 Implementation

#### File: `crates/ecl-stages/src/lookup.rs` (new)

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct LookupConfig {
    pub lookups: Vec<LookupOp>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LookupOp {
    /// Source field to look up.
    pub field: String,
    /// Output field for the mapped value.
    pub output: String,
    /// Lookup table: { input_value: output_value }
    pub table: BTreeMap<String, String>,
    /// Default value if input not found in table.
    #[serde(default)]
    pub default: Option<String>,
    /// Case-insensitive matching. Default: false.
    #[serde(default)]
    pub case_insensitive: bool,
}

#[derive(Debug)]
pub struct LookupStage {
    config: LookupConfig,
    /// Pre-built lookup tables (optionally lowercased keys for case-insensitive).
    tables: Vec<HashMap<String, String>>,
}

impl LookupStage {
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: LookupConfig = serde_json::from_value(params.clone())
            .map_err(|e| StageError::Permanent { /* ... */ })?;

        let tables = config.lookups.iter().map(|op| {
            if op.case_insensitive {
                op.table.iter()
                    .map(|(k, v)| (k.to_lowercase(), v.clone()))
                    .collect()
            } else {
                op.table.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            }
        }).collect();

        Ok(Self { config, tables })
    }
}

#[async_trait]
impl Stage for LookupStage {
    fn name(&self) -> &str { "lookup" }

    async fn process(
        &self,
        mut item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let record = item.record.as_mut()
            .ok_or_else(|| StageError::Permanent { /* ... */ })?;

        for (i, op) in self.config.lookups.iter().enumerate() {
            let input_value = record.get(&op.field)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let lookup_key = if op.case_insensitive {
                input_value.to_lowercase()
            } else {
                input_value.to_string()
            };

            let output_value = self.tables[i].get(&lookup_key)
                .cloned()
                .or_else(|| op.default.clone());

            if let Some(val) = output_value {
                record.insert(op.output.clone(), Value::String(val));
            } else {
                record.insert(op.output.clone(), Value::Null);
            }
        }

        Ok(vec![item])
    }
}
```

### 5.3 Tests

1. `test_lookup_basic_match`
2. `test_lookup_no_match_default`
3. `test_lookup_no_match_no_default_null`
4. `test_lookup_case_insensitive`
5. `test_lookup_multiple_lookups_in_one_stage`
6. `test_lookup_ge_payment_types` — full Giant Eagle payment mapping
7. `test_lookup_ge_payment_schemes` — Visa→VI, MC→MC, etc.
8. `test_lookup_missing_field_uses_empty_string`

---

## 6. Milestone 8.5: Date/Time & Timezone Stages

### 6.1 Scope

Two stages: `DateParseStage` (string → RFC3339) and `TimezoneStage` (local → UTC by ZIP code).

### 6.2 DateParseStage

#### File: `crates/ecl-stages/src/date_parse.rs` (new)

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct DateParseConfig {
    pub conversions: Vec<DateConversion>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DateConversion {
    /// Source field containing the date string.
    pub field: String,
    /// Output field for the parsed RFC3339 datetime.
    pub output: String,
    /// strftime format string (e.g., "%m/%d/%Y", "%Y-%m-%dT%H:%M:%S").
    pub format: String,
    /// Assumed timezone if the input has no timezone.
    /// "UTC", "US/Eastern", or IANA timezone name.
    #[serde(default = "default_utc")]
    pub assume_timezone: String,
}
```

**Implementation:** Parse date string with `chrono::NaiveDateTime::parse_from_str` (or `NaiveDate` for date-only formats), attach timezone, convert to UTC, format as RFC3339.

### 6.3 TimezoneStage

#### File: `crates/ecl-stages/src/timezone.rs` (new)

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct TimezoneConfig {
    /// Field containing the datetime string (already parsed).
    pub datetime_field: String,
    /// Field containing the ZIP code for timezone lookup.
    pub zipcode_field: String,
    /// Output field for UTC datetime.
    pub output: String,
    /// Fallback timezone if ZIP lookup fails.
    #[serde(default = "default_us_eastern")]
    pub fallback_timezone: String,
    /// Special overrides: { "5995": "UTC" } for online stores.
    #[serde(default)]
    pub overrides: BTreeMap<String, String>,
    /// Override key field (e.g., "store_id" to match against overrides).
    #[serde(default)]
    pub override_key_field: Option<String>,
}
```

**Implementation:** Use an embedded US ZIP → timezone lookup table (compile-time or loaded from a CSV resource). The `uszipcode` equivalent in Rust is `zipcode` crate or an embedded table.

**Simpler approach for Phase 2:** Ship a static `HashMap<String, String>` of US ZIP prefixes → timezone strings (the first 3 digits of a ZIP determine the timezone with high accuracy). ~1000 entries covers all US ZIPs.

**Dependencies:**
```toml
chrono-tz = "0.10"  # IANA timezone support
```

### 6.4 Tests

**DateParse:**
1. `test_date_parse_mm_dd_yyyy`
2. `test_date_parse_iso8601`
3. `test_date_parse_iso8601_millis` — digital file timestamps
4. `test_date_parse_invalid_returns_null`
5. `test_date_parse_assume_timezone`

**Timezone:**
1. `test_timezone_us_eastern_to_utc`
2. `test_timezone_us_pacific_to_utc`
3. `test_timezone_override_store_5995` — store ID → UTC
4. `test_timezone_fallback_on_unknown_zip`
5. `test_timezone_zip_prefix_lookup`

---

## 7. Milestone 8.6: ZIP Decompression Stage

### 7.1 Scope

Decompress ZIP archives, producing one PipelineItem per extracted file (fan-out).

### 7.2 Implementation

#### File: `crates/ecl-stages/src/decompress.rs` (new)

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct DecompressConfig {
    /// Supported formats: "zip", "gzip". Default: "zip"
    #[serde(default = "default_zip")]
    pub format: String,
    /// File extension filter for extracted files (empty = all).
    #[serde(default)]
    pub extensions: Vec<String>,
}

#[derive(Debug)]
pub struct DecompressStage {
    config: DecompressConfig,
}

#[async_trait]
impl Stage for DecompressStage {
    fn name(&self) -> &str { "decompress" }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        match self.config.format.as_str() {
            "zip" => self.decompress_zip(item),
            "gzip" => self.decompress_gzip(item),
            _ => Err(StageError::Permanent { /* unsupported format */ }),
        }
    }
}

impl DecompressStage {
    fn decompress_zip(&self, item: PipelineItem) -> Result<Vec<PipelineItem>, StageError> {
        use std::io::Cursor;

        let cursor = Cursor::new(item.content.as_ref());
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| StageError::Permanent { /* ... */ })?;

        let mut results = Vec::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| StageError::Permanent { /* ... */ })?;

            if file.is_dir() { continue; }

            let name = file.name().to_string();

            // Extension filter.
            if !self.config.extensions.is_empty() {
                let ext = std::path::Path::new(&name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                if !self.config.extensions.iter().any(|e| e == ext) {
                    continue;
                }
            }

            let mut content = Vec::new();
            std::io::Read::read_to_end(&mut file, &mut content)
                .map_err(|e| StageError::Permanent { /* ... */ })?;

            results.push(PipelineItem {
                id: format!("{}:{}", item.id, name),
                display_name: name.clone(),
                content: Arc::from(content.as_slice()),
                mime_type: mime_from_extension(&name),
                stream: item.stream.clone(),
                record: None,
                ..item.clone()
            });
        }

        Ok(results)
    }
}
```

**Dependencies:**
```toml
zip = "2"
flate2 = "1"  # for gzip
```

**Note:** `zip::ZipArchive::new` is synchronous but operates on in-memory `Cursor<&[u8]>`, so it's CPU-bound not I/O-bound. For large archives, consider `spawn_blocking`. For typical Giant Eagle files (few MB), inline is fine.

### 7.3 Tests

1. `test_decompress_zip_basic` — 3 files in archive → 3 items
2. `test_decompress_zip_extension_filter` — only .csv extracted
3. `test_decompress_zip_skips_directories`
4. `test_decompress_zip_preserves_stream_tag`
5. `test_decompress_zip_fan_out_ids` — `{parent_id}:{filename}`
6. `test_decompress_zip_empty_archive` — returns empty vec
7. `test_decompress_zip_invalid_archive` — returns Permanent error
8. `test_decompress_gzip_basic`

---

## 8. Milestone 8.7: Receipt Assembly Stage

### 8.1 Scope

A batch stage that merges multiple streams into nested receipt structures matching the Banyan canonical output model.

### 8.2 Implementation

#### File: `crates/ecl-stages/src/assemble.rs` (new)

**Configuration:**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct AssembleConfig {
    /// The primary stream (e.g., "transactions"). One output per primary record.
    pub primary_stream: String,
    /// The key field in the primary stream.
    pub primary_key: String,
    /// How to join other streams into the primary.
    pub joins: Vec<AssembleJoin>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssembleJoin {
    /// Which stream to join from.
    pub stream: String,
    /// Key field in the primary stream for matching.
    pub key: String,
    /// Key field in this stream for matching (foreign key).
    pub foreign_key: String,
    /// Field name in the output where this data is nested.
    pub nest_as: String,
    /// If true, collect all matching records into an array.
    /// If false, take the first matching record as a nested object.
    #[serde(default)]
    pub collect: bool,
}
```

**Stage implementation:**

```rust
#[derive(Debug)]
pub struct AssembleStage {
    config: AssembleConfig,
}

#[async_trait]
impl Stage for AssembleStage {
    fn name(&self) -> &str { "assemble" }
    fn requires_batch(&self) -> bool { true }

    async fn process(
        &self,
        _item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        Err(StageError::Permanent {
            stage: "assemble".to_string(),
            item_id: String::new(),
            message: "assemble requires batch mode".to_string(),
        })
    }

    async fn process_batch(
        &self,
        items: Vec<PipelineItem>,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // 1. Partition items by stream.
        let mut stream_items: BTreeMap<String, Vec<PipelineItem>> = BTreeMap::new();
        for item in items {
            let stream = item.stream.as_deref().unwrap_or("").to_string();
            stream_items.entry(stream).or_default().push(item);
        }

        // 2. Get primary stream items.
        let primary_items = stream_items.remove(&self.config.primary_stream)
            .unwrap_or_default();

        // 3. Build indexes for each join stream: foreign_key → Vec<Record>
        let mut join_indexes: BTreeMap<String, HashMap<String, Vec<Value>>> = BTreeMap::new();
        for join_def in &self.config.joins {
            let stream_recs = stream_items.get(&join_def.stream).unwrap_or(&vec![]);
            let mut index: HashMap<String, Vec<Value>> = HashMap::new();
            for item in stream_recs {
                if let Some(record) = &item.record {
                    let key = record.get(&join_def.foreign_key)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    index.entry(key).or_default().push(Value::Object(record.clone()));
                }
            }
            join_indexes.insert(join_def.stream.clone(), index);
        }

        // 4. Assemble receipts.
        let mut results = Vec::new();
        for primary_item in primary_items {
            let primary_record = primary_item.record.as_ref()
                .ok_or_else(|| StageError::Permanent { /* ... */ })?;
            let primary_key_value = primary_record.get(&self.config.primary_key)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mut assembled = primary_record.clone();

            for join_def in &self.config.joins {
                let lookup_key = primary_record.get(&join_def.key)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if let Some(index) = join_indexes.get(&join_def.stream) {
                    let matches = index.get(&lookup_key);
                    if join_def.collect {
                        // Array of matching records.
                        assembled.insert(
                            join_def.nest_as.clone(),
                            Value::Array(matches.cloned().unwrap_or_default()),
                        );
                    } else {
                        // Single nested object (first match).
                        assembled.insert(
                            join_def.nest_as.clone(),
                            matches.and_then(|m| m.first().cloned())
                                .unwrap_or(Value::Null),
                        );
                    }
                }
            }

            results.push(PipelineItem {
                id: format!("receipt:{primary_key_value}"),
                display_name: format!("Receipt {primary_key_value}"),
                record: Some(assembled),
                ..primary_item
            });
        }

        Ok(results)
    }
}
```

### 8.3 Tests

1. `test_assemble_basic_one_join` — transaction + store
2. `test_assemble_collect_array` — transaction + multiple items
3. `test_assemble_multiple_joins` — transaction + store + items + payment
4. `test_assemble_no_match_null` — missing store → null
5. `test_assemble_no_match_collect_empty_array` — no items → []
6. `test_assemble_ge_full_receipt` — full Giant Eagle receipt structure
7. `test_assemble_requires_batch`
8. `test_assemble_empty_primary` — no transactions → empty result

---

## 9. Milestone 8.8: Giant Eagle End-to-End Integration Test

### 9.1 Test Data

Create sample CSVs at `crates/ecl-pipeline/tests/fixtures/giant_eagle/`:

- `stores.csv` — 3 stores (no headers)
- `transactions.csv` — 5 transactions across 3 stores
- `items.csv` — 10 items across the 5 transactions
- `products.csv` — 8 products (2 items won't match = left join)
- `tenders.csv` — 7 tenders (some transactions have split payment)

Create a `giant_eagle.zip` containing all 5 CSVs for decompression testing.

### 9.2 Test Pipeline Spec

Create `crates/ecl-pipeline/tests/fixtures/giant_eagle_pipeline.toml` using filesystem sources (substituting for GCS) with all Phase 2 stages.

### 9.3 Test Scenarios

#### File: `crates/ecl-pipeline/tests/integration/giant_eagle_e2e.rs` (new)

1. **`test_ge_e2e_full_pipeline`**
   - Parse 5 CSV files into 5 streams
   - Map fields per stream
   - Lookup payment types and schemes
   - Join items with products by UPC
   - Aggregate tenders by transaction_id
   - Assemble receipts with nested store, items, payment
   - Validate
   - Emit to output directory
   - Verify: 5 receipt files, each with correct nested structure

2. **`test_ge_e2e_left_join_unmatched_products`**
   - Items with unknown UPCs still appear in receipts (null product fields)

3. **`test_ge_e2e_split_payment_aggregation`**
   - Transaction with 3 tenders → single payment object with payments array

4. **`test_ge_e2e_stream_isolation`**
   - Verify that store records never appear in item stages and vice versa

5. **`test_ge_e2e_decompression_to_receipt`**
   - Start from ZIP file → decompress → full pipeline
   - Verify same output as test 1

---

## 10. Cross-Cutting Concerns

### 10.1 StageError: Add Clone Derive

`StageError` needs `Clone` for the batch execution model (errors shared across item IDs):

```rust
#[derive(Debug, Clone, thiserror::Error)]  // Add Clone
pub enum StageError { /* ... */ }
```

All fields are `String` and `u64`, so this is trivially derivable.

### 10.2 New Dependencies

**Root `Cargo.toml` workspace deps:**
```toml
zip = "2"
flate2 = "1"
chrono-tz = "0.10"
```

**`ecl-stages/Cargo.toml` additions:**
```toml
zip = { workspace = true }
flate2 = { workspace = true }
chrono-tz = { workspace = true }
```

### 10.3 Registry Updates

#### File: `crates/ecl-cli/src/pipeline/registry.rs`

Add to `stage_lookup_fn`:
```rust
"join" => {
    let stage = ecl_stages::JoinStage::from_params(&spec.params)?;
    Ok(Arc::new(stage) as Arc<dyn Stage>)
}
"aggregate" => {
    let stage = ecl_stages::AggregateStage::from_params(&spec.params)?;
    Ok(Arc::new(stage) as Arc<dyn Stage>)
}
"lookup" => {
    let stage = ecl_stages::LookupStage::from_params(&spec.params)?;
    Ok(Arc::new(stage) as Arc<dyn Stage>)
}
"date_parse" => {
    let stage = ecl_stages::DateParseStage::from_params(&spec.params)?;
    Ok(Arc::new(stage) as Arc<dyn Stage>)
}
"timezone" => {
    let stage = ecl_stages::TimezoneStage::from_params(&spec.params)?;
    Ok(Arc::new(stage) as Arc<dyn Stage>)
}
"decompress" => {
    let stage = ecl_stages::DecompressStage::from_params(&spec.params)?;
    Ok(Arc::new(stage) as Arc<dyn Stage>)
}
"assemble" => {
    let stage = ecl_stages::AssembleStage::from_params(&spec.params)?;
    Ok(Arc::new(stage) as Arc<dyn Stage>)
}
```

### 10.4 Module Wiring in ecl-stages

```rust
// In crates/ecl-stages/src/lib.rs:
pub mod join;
pub mod aggregate;
pub mod lookup;
pub mod date_parse;
pub mod timezone;
pub mod decompress;
pub mod assemble;

pub use join::JoinStage;
pub use aggregate::AggregateStage;
pub use lookup::LookupStage;
pub use date_parse::DateParseStage;
pub use timezone::TimezoneStage;
pub use decompress::DecompressStage;
pub use assemble::AssembleStage;
```

### 10.5 Backward Compatibility

All changes are additive:
- `PipelineItem` gains `stream: Option<String>` with `#[serde(default)]`
- `StageSpec` gains `input_streams: Vec<String>` and `output_stream: Option<String>` with `#[serde(default)]`
- `Stage` trait gains `requires_batch()` and `process_batch()` with defaults
- Source specs gain `stream: Option<String>` with `#[serde(default)]`
- `PipelineRunner` gains `active_items: Vec<PipelineItem>` (internal)

**Phase 1 pipelines (no streams) continue to work unchanged.** Empty `input_streams` = all items visible. No `output_stream` = items retain their tags.

**Existing tests must pass.** The `collect_items_for_stage` change is backward compatible because empty `input_streams` accepts all items.

---

## 11. Verification Checklist

### Per-Milestone

- [ ] `make test` passes (all existing + new tests)
- [ ] `make lint` passes
- [ ] `make format` passes
- [ ] No compiler warnings
- [ ] Coverage ≥ 95% on new code
- [ ] Checked against AP-01 through AP-20
- [ ] Doc comments on all public items

### Phase 2 Complete

- [ ] Named streams route items correctly
- [ ] Join stage merges two streams by key (inner, left, full)
- [ ] Aggregate stage groups and computes (sum, max, min, count, collect)
- [ ] Lookup stage maps values from static tables
- [ ] Date parse converts string → RFC3339
- [ ] Timezone converts local → UTC by ZIP code
- [ ] Decompress extracts ZIP archives with fan-out
- [ ] Assemble merges multiple streams into nested receipts
- [ ] Giant Eagle E2E integration test passes
- [ ] Phase 1 Affinity E2E test still passes (no regressions)
- [ ] Existing pre-Phase-1 tests still pass

---

## Appendix A: Milestone Dependency Graph

```
8.1 Named Streams ───────────────────────────────────┐
    │                                                 │
    ├── 8.2 Batch Trait + Join ──────────────┐       │
    │                                         │       │
    ├── 8.3 Aggregation ─────────────────────┤       │
    │                                         │       │
    ├── 8.4 Lookup ──────────────────────────┤       │
    │                                         │       │
    ├── 8.5 Date/Time + Timezone ────────────┤       │
    │                                         │       │
    ├── 8.6 ZIP Decompression ───────────────┤       │
    │                                         │       │
    └─────────────────────────────────────────┘       │
                              │                        │
                       8.7 Assembly ──────────────────┤
                              │                        │
                       8.8 Giant Eagle E2E ───────────┘
```

**Parallelizable work:**
- 8.2, 8.3, 8.4, 8.5, 8.6 can ALL proceed in parallel after 8.1
- 8.7 depends on 8.2 + 8.3 (uses batch mode + multi-stream assembly)
- 8.8 depends on all prior milestones

## Appendix B: Files Changed/Created Summary

### New Files
| File | Milestone |
|------|-----------|
| `crates/ecl-stages/src/join.rs` | 8.2 |
| `crates/ecl-stages/src/aggregate.rs` | 8.3 |
| `crates/ecl-stages/src/lookup.rs` | 8.4 |
| `crates/ecl-stages/src/date_parse.rs` | 8.5 |
| `crates/ecl-stages/src/timezone.rs` | 8.5 |
| `crates/ecl-stages/src/decompress.rs` | 8.6 |
| `crates/ecl-stages/src/assemble.rs` | 8.7 |
| `tests/fixtures/giant_eagle/*.csv` | 8.8 |
| `tests/fixtures/giant_eagle_pipeline.toml` | 8.8 |
| `tests/integration/giant_eagle_e2e.rs` | 8.8 |

### Modified Files
| File | Milestone | Change |
|------|-----------|--------|
| `crates/ecl-pipeline-topo/src/traits.rs` | 8.1, 8.2 | Add `stream` to PipelineItem, add `requires_batch`/`process_batch` to Stage |
| `crates/ecl-pipeline-topo/src/error.rs` | 8.2 | Add Clone to StageError |
| `crates/ecl-pipeline-spec/src/source.rs` | 8.1 | Add `stream` to all source specs |
| `crates/ecl-pipeline-spec/src/stage.rs` | 8.1 | Add `input_streams`, `output_stream` |
| `crates/ecl-pipeline/src/runner.rs` | 8.1 | Add `active_items`, stream-aware collection, output propagation |
| `crates/ecl-pipeline/src/batch.rs` | 8.2 | Add `execute_stage_batch` function |
| `crates/ecl-stages/src/lib.rs` | 8.2-8.7 | Module declarations + re-exports |
| `crates/ecl-stages/Cargo.toml` | 8.5, 8.6 | Add zip, flate2, chrono-tz deps |
| `crates/ecl-cli/src/pipeline/registry.rs` | 8.2-8.7 | Register 7 new stages |
| `Cargo.toml` (workspace) | 8.5, 8.6 | Add workspace deps |

## Appendix C: Runner State Model Evolution

### Before Phase 2 (Current)

```
Sources enumerate → ItemState (Pending) → Stage processes → ItemState (Completed)
                                           ↓
                                    No output propagation
```

### After Phase 2

```
Sources enumerate → active_items [stream tagged]
                         │
                    Batch 0: stages consume/produce active_items
                         │
                    Batch 1: stages see previous batch outputs
                         │
                    Batch N: terminal stages (sinks) consume, produce nothing
                         │
                    Final: active_items should be empty (all consumed)
```

This is the key architectural shift. Items are a flowing pool, not a fixed set.
