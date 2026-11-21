# 4. Milestone 8.3: Aggregation Stage

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0027-phase-2-detailed-implementation-plan-make-giant-eagle-work.md, lines 897-1088. Original bytes are recoverable from import commit 3a39aea. -->

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

