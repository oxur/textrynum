# 8. Milestone 8.7: Receipt Assembly Stage

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0027-phase-2-detailed-implementation-plan-make-giant-eagle-work.md, lines 1402-1562. Original bytes are recoverable from import commit 2d006a7. -->

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

