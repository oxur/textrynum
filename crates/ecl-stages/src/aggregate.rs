//! Aggregate stage: groups records by key and applies aggregate functions.
//!
//! Supports: sum, max, min, count, first, last, avg, and sub-record collection.
//! This is a batch stage that requires all items at once.

use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use tracing::debug;

use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, Stage, StageContext};

type Record = serde_json::Map<String, serde_json::Value>;

/// Configuration for the aggregate stage, parsed from stage params.
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

/// A single aggregate operation.
#[derive(Debug, Clone, Deserialize)]
pub struct AggregateOp {
    /// Source field to aggregate.
    pub field: String,
    /// Aggregate function: "sum", "max", "min", "count", "first", "last", "avg".
    pub function: String,
    /// Output field name for the aggregate result.
    pub output: String,
}

/// Collect sub-records into a JSON array.
#[derive(Debug, Clone, Deserialize)]
pub struct CollectArrayOp {
    /// Output field name (will be a JSON array).
    pub output: String,
    /// Fields to include in each array element.
    pub fields: Vec<String>,
}

/// Aggregate stage: groups records by key and computes aggregates.
///
/// This is a batch stage (`requires_batch() -> true`).
#[derive(Debug)]
pub struct AggregateStage {
    config: AggregateConfig,
}

impl AggregateStage {
    /// Create an AggregateStage from stage params JSON.
    ///
    /// # Errors
    ///
    /// Returns `StageError::Permanent` if params cannot be deserialized.
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: AggregateConfig =
            serde_json::from_value(params.clone()).map_err(|e| StageError::Permanent {
                stage: "aggregate".to_string(),
                item_id: String::new(),
                message: format!("invalid aggregate config: {e}"),
            })?;
        Ok(Self { config })
    }

    fn compute_group_key(&self, record: &Record) -> String {
        self.config
            .group_by
            .iter()
            .map(|f| record.get(f).and_then(|v| v.as_str()).unwrap_or(""))
            .collect::<Vec<_>>()
            .join("|")
    }

    fn compute_aggregate(&self, function: &str, field: &str, records: &[&Record]) -> Value {
        let values: Vec<f64> = records
            .iter()
            .filter_map(|r| r.get(field))
            .filter_map(|v| match v {
                Value::Number(n) => n.as_f64(),
                Value::String(s) => s.parse::<f64>().ok(),
                _ => None,
            })
            .collect();

        match function {
            "sum" => Value::from(values.iter().sum::<f64>()),
            "max" => values
                .iter()
                .cloned()
                .reduce(f64::max)
                .map(Value::from)
                .unwrap_or(Value::Null),
            "min" => values
                .iter()
                .cloned()
                .reduce(f64::min)
                .map(Value::from)
                .unwrap_or(Value::Null),
            "avg" => {
                if values.is_empty() {
                    Value::Null
                } else {
                    Value::from(values.iter().sum::<f64>() / values.len() as f64)
                }
            }
            "count" => Value::from(values.len() as i64),
            "first" => records
                .first()
                .and_then(|r| r.get(field).cloned())
                .unwrap_or(Value::Null),
            "last" => records
                .last()
                .and_then(|r| r.get(field).cloned())
                .unwrap_or(Value::Null),
            _ => Value::Null,
        }
    }
}

#[async_trait]
impl Stage for AggregateStage {
    fn name(&self) -> &str {
        "aggregate"
    }

    fn requires_batch(&self) -> bool {
        true
    }

    async fn process(
        &self,
        _item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        Err(StageError::Permanent {
            stage: "aggregate".to_string(),
            item_id: String::new(),
            message: "aggregate stage requires batch mode; use process_batch()".to_string(),
        })
    }

    async fn process_batch(
        &self,
        items: Vec<PipelineItem>,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        debug!(items = items.len(), "aggregate stage starting");

        // Group items by composite key.
        let mut groups: BTreeMap<String, Vec<(PipelineItem, Record)>> = BTreeMap::new();

        for item in items {
            let record = item.record.clone().ok_or_else(|| StageError::Permanent {
                stage: "aggregate".to_string(),
                item_id: item.id.clone(),
                message: "item has no record".to_string(),
            })?;
            let key = self.compute_group_key(&record);
            groups.entry(key).or_default().push((item, record));
        }

        // For each group, compute aggregates.
        let mut results = Vec::new();
        for (key, group) in &groups {
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
                let array: Vec<Value> = records
                    .iter()
                    .map(|r| {
                        let mut obj = serde_json::Map::new();
                        for field in &collect.fields {
                            if let Some(val) = r.get(field) {
                                obj.insert(field.clone(), val.clone());
                            }
                        }
                        Value::Object(obj)
                    })
                    .collect();
                output_record.insert(collect.output.clone(), Value::Array(array));
            }

            results.push(PipelineItem {
                id: format!("{}:agg:{}", first_item.id, key),
                record: Some(output_record),
                ..first_item.clone()
            });
        }

        debug!(
            groups = groups.len(),
            output_items = results.len(),
            "aggregate stage complete"
        );
        Ok(results)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ecl_pipeline_spec::PipelineSpec;
    use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
    use serde_json::json;
    use std::sync::Arc;

    fn make_item(id: &str, record: Record) -> PipelineItem {
        PipelineItem {
            id: id.to_string(),
            display_name: id.to_string(),
            content: Arc::from(b"" as &[u8]),
            mime_type: "application/x-csv-row".to_string(),
            source_name: "test".to_string(),
            source_content_hash: Blake3Hash::new("test"),
            provenance: ItemProvenance {
                source_kind: "test".to_string(),
                metadata: BTreeMap::new(),
                source_modified: None,
                extracted_at: chrono::Utc::now(),
            },
            metadata: BTreeMap::new(),
            record: Some(record),
            stream: None,
        }
    }

    fn make_record(fields: &[(&str, serde_json::Value)]) -> Record {
        let mut record = Record::new();
        for (k, v) in fields {
            record.insert((*k).to_string(), v.clone());
        }
        record
    }

    fn make_context() -> StageContext {
        StageContext {
            spec: Arc::new(PipelineSpec {
                name: "test".to_string(),
                version: 1,
                output_dir: std::path::PathBuf::from("./out"),
                sources: BTreeMap::new(),
                stages: BTreeMap::new(),
                defaults: ecl_pipeline_spec::DefaultsSpec::default(),
                lifecycle: None,
                secrets: Default::default(),
                triggers: None,
                schedule: None,
            }),
            output_dir: std::path::PathBuf::from("./out"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        }
    }

    #[test]
    fn test_aggregate_requires_batch() {
        let params = json!({ "group_by": ["k"], "aggregates": [] });
        let stage = AggregateStage::from_params(&params).unwrap();
        assert!(stage.requires_batch());
    }

    #[tokio::test]
    async fn test_aggregate_sum_basic() {
        let params = json!({
            "group_by": ["store"],
            "aggregates": [{ "field": "amount", "function": "sum", "output": "total" }]
        });
        let stage = AggregateStage::from_params(&params).unwrap();
        let ctx = make_context();

        let items = vec![
            make_item(
                "r1",
                make_record(&[("store", json!("A")), ("amount", json!(10.0))]),
            ),
            make_item(
                "r2",
                make_record(&[("store", json!("A")), ("amount", json!(20.0))]),
            ),
            make_item(
                "r3",
                make_record(&[("store", json!("B")), ("amount", json!(30.0))]),
            ),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        assert_eq!(result.len(), 2);

        let a = result
            .iter()
            .find(|i| i.record.as_ref().unwrap()["store"] == "A")
            .unwrap();
        assert_eq!(a.record.as_ref().unwrap()["total"], json!(30.0));

        let b = result
            .iter()
            .find(|i| i.record.as_ref().unwrap()["store"] == "B")
            .unwrap();
        assert_eq!(b.record.as_ref().unwrap()["total"], json!(30.0));
    }

    #[tokio::test]
    async fn test_aggregate_max_min() {
        let params = json!({
            "group_by": ["g"],
            "aggregates": [
                { "field": "val", "function": "max", "output": "max_val" },
                { "field": "val", "function": "min", "output": "min_val" },
            ]
        });
        let stage = AggregateStage::from_params(&params).unwrap();
        let ctx = make_context();

        let items = vec![
            make_item("r1", make_record(&[("g", json!("X")), ("val", json!(5.0))])),
            make_item(
                "r2",
                make_record(&[("g", json!("X")), ("val", json!(15.0))]),
            ),
            make_item(
                "r3",
                make_record(&[("g", json!("X")), ("val", json!(10.0))]),
            ),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        assert_eq!(result.len(), 1);
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec["max_val"], json!(15.0));
        assert_eq!(rec["min_val"], json!(5.0));
    }

    #[tokio::test]
    async fn test_aggregate_count() {
        let params = json!({
            "group_by": ["cat"],
            "aggregates": [{ "field": "id", "function": "count", "output": "cnt" }]
        });
        let stage = AggregateStage::from_params(&params).unwrap();
        let ctx = make_context();

        let items = vec![
            make_item("r1", make_record(&[("cat", json!("A")), ("id", json!(1))])),
            make_item("r2", make_record(&[("cat", json!("A")), ("id", json!(2))])),
            make_item("r3", make_record(&[("cat", json!("A")), ("id", json!(3))])),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        assert_eq!(result[0].record.as_ref().unwrap()["cnt"], json!(3));
    }

    #[tokio::test]
    async fn test_aggregate_first_last() {
        let params = json!({
            "group_by": ["g"],
            "aggregates": [
                { "field": "name", "function": "first", "output": "first_name" },
                { "field": "name", "function": "last", "output": "last_name" },
            ]
        });
        let stage = AggregateStage::from_params(&params).unwrap();
        let ctx = make_context();

        let items = vec![
            make_item(
                "r1",
                make_record(&[("g", json!("X")), ("name", json!("Alice"))]),
            ),
            make_item(
                "r2",
                make_record(&[("g", json!("X")), ("name", json!("Bob"))]),
            ),
            make_item(
                "r3",
                make_record(&[("g", json!("X")), ("name", json!("Charlie"))]),
            ),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec["first_name"], json!("Alice"));
        assert_eq!(rec["last_name"], json!("Charlie"));
    }

    #[tokio::test]
    async fn test_aggregate_avg() {
        let params = json!({
            "group_by": ["g"],
            "aggregates": [{ "field": "val", "function": "avg", "output": "avg_val" }]
        });
        let stage = AggregateStage::from_params(&params).unwrap();
        let ctx = make_context();

        let items = vec![
            make_item(
                "r1",
                make_record(&[("g", json!("X")), ("val", json!(10.0))]),
            ),
            make_item(
                "r2",
                make_record(&[("g", json!("X")), ("val", json!(20.0))]),
            ),
            make_item(
                "r3",
                make_record(&[("g", json!("X")), ("val", json!(30.0))]),
            ),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        assert_eq!(result[0].record.as_ref().unwrap()["avg_val"], json!(20.0));
    }

    #[tokio::test]
    async fn test_aggregate_collect_array() {
        let params = json!({
            "group_by": ["store"],
            "collect_arrays": [{ "output": "items", "fields": ["sku", "qty"] }]
        });
        let stage = AggregateStage::from_params(&params).unwrap();
        let ctx = make_context();

        let items = vec![
            make_item(
                "r1",
                make_record(&[
                    ("store", json!("S1")),
                    ("sku", json!("A")),
                    ("qty", json!(2)),
                ]),
            ),
            make_item(
                "r2",
                make_record(&[
                    ("store", json!("S1")),
                    ("sku", json!("B")),
                    ("qty", json!(5)),
                ]),
            ),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        assert_eq!(result.len(), 1);
        let arr = result[0].record.as_ref().unwrap()["items"]
            .as_array()
            .unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["sku"], json!("A"));
        assert_eq!(arr[1]["sku"], json!("B"));
    }

    #[tokio::test]
    async fn test_aggregate_composite_key() {
        let params = json!({
            "group_by": ["store", "dept"],
            "aggregates": [{ "field": "amount", "function": "sum", "output": "total" }]
        });
        let stage = AggregateStage::from_params(&params).unwrap();
        let ctx = make_context();

        let items = vec![
            make_item(
                "r1",
                make_record(&[
                    ("store", json!("S1")),
                    ("dept", json!("D1")),
                    ("amount", json!(10.0)),
                ]),
            ),
            make_item(
                "r2",
                make_record(&[
                    ("store", json!("S1")),
                    ("dept", json!("D1")),
                    ("amount", json!(20.0)),
                ]),
            ),
            make_item(
                "r3",
                make_record(&[
                    ("store", json!("S1")),
                    ("dept", json!("D2")),
                    ("amount", json!(30.0)),
                ]),
            ),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        assert_eq!(result.len(), 2); // Two groups: S1|D1 and S1|D2

        let d1 = result
            .iter()
            .find(|i| i.record.as_ref().unwrap()["dept"] == "D1")
            .unwrap();
        assert_eq!(d1.record.as_ref().unwrap()["total"], json!(30.0));

        let d2 = result
            .iter()
            .find(|i| i.record.as_ref().unwrap()["dept"] == "D2")
            .unwrap();
        assert_eq!(d2.record.as_ref().unwrap()["total"], json!(30.0));
    }

    #[tokio::test]
    async fn test_aggregate_string_to_numeric_coercion() {
        let params = json!({
            "group_by": ["g"],
            "aggregates": [{ "field": "price", "function": "sum", "output": "total" }]
        });
        let stage = AggregateStage::from_params(&params).unwrap();
        let ctx = make_context();

        let items = vec![
            make_item(
                "r1",
                make_record(&[("g", json!("X")), ("price", json!("25.99"))]),
            ),
            make_item(
                "r2",
                make_record(&[("g", json!("X")), ("price", json!("14.01"))]),
            ),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        assert_eq!(result[0].record.as_ref().unwrap()["total"], json!(40.0));
    }

    #[tokio::test]
    async fn test_aggregate_empty_input() {
        let params = json!({
            "group_by": ["g"],
            "aggregates": [{ "field": "val", "function": "sum", "output": "total" }]
        });
        let stage = AggregateStage::from_params(&params).unwrap();
        let ctx = make_context();

        let result = stage.process_batch(vec![], &ctx).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_aggregate_single_group() {
        let params = json!({
            "group_by": ["g"],
            "aggregates": [
                { "field": "val", "function": "sum", "output": "total" },
                { "field": "val", "function": "count", "output": "cnt" },
            ]
        });
        let stage = AggregateStage::from_params(&params).unwrap();
        let ctx = make_context();

        let items = vec![
            make_item(
                "r1",
                make_record(&[("g", json!("same")), ("val", json!(10.0))]),
            ),
            make_item(
                "r2",
                make_record(&[("g", json!("same")), ("val", json!(20.0))]),
            ),
            make_item(
                "r3",
                make_record(&[("g", json!("same")), ("val", json!(30.0))]),
            ),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        assert_eq!(result.len(), 1);
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec["total"], json!(60.0));
        assert_eq!(rec["cnt"], json!(3));
    }

    #[tokio::test]
    async fn test_aggregate_ge_tenders() {
        // Giant Eagle: aggregate tender amounts per transaction.
        let params = json!({
            "group_by": ["txn_id"],
            "aggregates": [
                { "field": "amount", "function": "sum", "output": "total_amount" },
                { "field": "amount", "function": "count", "output": "tender_count" },
            ],
            "collect_arrays": [{
                "output": "tenders",
                "fields": ["tender_type", "amount"]
            }]
        });
        let stage = AggregateStage::from_params(&params).unwrap();
        let ctx = make_context();

        let items = vec![
            make_item(
                "t1-cash",
                make_record(&[
                    ("txn_id", json!("T1")),
                    ("tender_type", json!("cash")),
                    ("amount", json!(20.0)),
                ]),
            ),
            make_item(
                "t1-card",
                make_record(&[
                    ("txn_id", json!("T1")),
                    ("tender_type", json!("card")),
                    ("amount", json!(30.0)),
                ]),
            ),
            make_item(
                "t2-cash",
                make_record(&[
                    ("txn_id", json!("T2")),
                    ("tender_type", json!("cash")),
                    ("amount", json!(15.0)),
                ]),
            ),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        assert_eq!(result.len(), 2);

        let t1 = result
            .iter()
            .find(|i| i.record.as_ref().unwrap()["txn_id"] == "T1")
            .unwrap();
        let t1_rec = t1.record.as_ref().unwrap();
        assert_eq!(t1_rec["total_amount"], json!(50.0));
        assert_eq!(t1_rec["tender_count"], json!(2));
        let tenders = t1_rec["tenders"].as_array().unwrap();
        assert_eq!(tenders.len(), 2);

        let t2 = result
            .iter()
            .find(|i| i.record.as_ref().unwrap()["txn_id"] == "T2")
            .unwrap();
        assert_eq!(t2.record.as_ref().unwrap()["total_amount"], json!(15.0));
    }
}
