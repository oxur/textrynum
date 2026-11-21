# 3. Milestone 8.2: Batch Stage Trait & Join Stage

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0027-phase-2-detailed-implementation-plan-make-giant-eagle-work.md, lines 606-896. Original bytes are recoverable from import commit 3a39aea. -->

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

