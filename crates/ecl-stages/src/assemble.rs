//! Assemble stage: merges multiple streams into nested receipt structures.
//!
//! A batch stage that joins a primary stream with secondary streams, nesting
//! matched records as objects or arrays. Used for building the Banyan canonical
//! output model (e.g., receipts with nested stores, items, payments).

use std::collections::{BTreeMap, HashMap};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use tracing::debug;

use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, Stage, StageContext};

/// Configuration for the assemble stage, parsed from stage params.
#[derive(Debug, Clone, Deserialize)]
pub struct AssembleConfig {
    /// The primary stream (e.g., "transactions"). One output per primary record.
    pub primary_stream: String,
    /// The key field in the primary stream.
    pub primary_key: String,
    /// How to join other streams into the primary.
    #[serde(default)]
    pub joins: Vec<AssembleJoin>,
}

/// A join definition for the assemble stage.
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

/// Assemble stage that merges multiple streams into nested structures.
#[derive(Debug)]
pub struct AssembleStage {
    config: AssembleConfig,
}

impl AssembleStage {
    /// Create an assemble stage from JSON params.
    ///
    /// # Errors
    ///
    /// Returns `StageError::Permanent` if params cannot be deserialized.
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: AssembleConfig =
            serde_json::from_value(params.clone()).map_err(|e| StageError::Permanent {
                stage: "assemble".into(),
                item_id: String::new(),
                message: format!("invalid assemble config: {e}"),
            })?;

        Ok(Self { config })
    }
}

#[async_trait]
impl Stage for AssembleStage {
    fn name(&self) -> &str {
        "assemble"
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
            stage: "assemble".into(),
            item_id: String::new(),
            message: "assemble requires batch mode".into(),
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
        let primary_items = stream_items
            .remove(&self.config.primary_stream)
            .unwrap_or_default();

        debug!(
            primary_stream = %self.config.primary_stream,
            primary_count = primary_items.len(),
            joins = self.config.joins.len(),
            "assembling records"
        );

        // 3. Build indexes for each join stream: foreign_key -> Vec<Record as Value>.
        let empty_vec = vec![];
        let mut join_indexes: BTreeMap<String, HashMap<String, Vec<Value>>> = BTreeMap::new();
        for join_def in &self.config.joins {
            let stream_recs = stream_items.get(&join_def.stream).unwrap_or(&empty_vec);
            let mut index: HashMap<String, Vec<Value>> = HashMap::new();
            for item in stream_recs {
                if let Some(record) = &item.record {
                    let key = record
                        .get(&join_def.foreign_key)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    index
                        .entry(key)
                        .or_default()
                        .push(Value::Object(record.clone()));
                }
            }
            join_indexes.insert(join_def.stream.clone(), index);
        }

        // 4. Assemble results.
        let mut results = Vec::new();
        for primary_item in primary_items {
            let primary_record =
                primary_item
                    .record
                    .as_ref()
                    .ok_or_else(|| StageError::Permanent {
                        stage: "assemble".into(),
                        item_id: primary_item.id.clone(),
                        message: "primary item has no record".into(),
                    })?;

            let primary_key_value = primary_record
                .get(&self.config.primary_key)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mut assembled = primary_record.clone();

            for join_def in &self.config.joins {
                let lookup_key = primary_record
                    .get(&join_def.key)
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
                            matches
                                .and_then(|m| m.first().cloned())
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ecl_pipeline_spec::PipelineSpec;
    use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
    use serde_json::json;
    use std::sync::Arc;

    type Record = serde_json::Map<String, serde_json::Value>;

    fn make_item(id: &str, stream: &str, record: Record) -> PipelineItem {
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
            stream: Some(stream.to_string()),
        }
    }

    fn make_record(fields: &[(&str, Value)]) -> Record {
        let mut record = Record::new();
        for (k, v) in fields {
            record.insert((*k).to_string(), v.clone());
        }
        record
    }

    fn ctx() -> StageContext {
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

    #[tokio::test]
    async fn test_assemble_requires_batch() {
        let params = json!({
            "primary_stream": "txns",
            "primary_key": "id"
        });
        let stage = AssembleStage::from_params(&params).unwrap();
        assert!(stage.requires_batch());
    }

    #[tokio::test]
    async fn test_assemble_basic_one_join() {
        let params = json!({
            "primary_stream": "transactions",
            "primary_key": "txn_id",
            "joins": [{
                "stream": "stores",
                "key": "store_id",
                "foreign_key": "store_id",
                "nest_as": "store",
                "collect": false
            }]
        });
        let stage = AssembleStage::from_params(&params).unwrap();

        let txn = make_item(
            "txn1",
            "transactions",
            make_record(&[
                ("txn_id", json!("T001")),
                ("store_id", json!("S01")),
                ("total", json!("42.50")),
            ]),
        );
        let store = make_item(
            "store1",
            "stores",
            make_record(&[
                ("store_id", json!("S01")),
                ("name", json!("Main St")),
                ("city", json!("Pittsburgh")),
            ]),
        );

        let result = stage.process_batch(vec![txn, store], &ctx()).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "receipt:T001");
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec.get("total").unwrap(), "42.50");
        let store_obj = rec.get("store").unwrap().as_object().unwrap();
        assert_eq!(store_obj.get("name").unwrap(), "Main St");
    }

    #[tokio::test]
    async fn test_assemble_collect_array() {
        let params = json!({
            "primary_stream": "transactions",
            "primary_key": "txn_id",
            "joins": [{
                "stream": "items",
                "key": "txn_id",
                "foreign_key": "txn_id",
                "nest_as": "line_items",
                "collect": true
            }]
        });
        let stage = AssembleStage::from_params(&params).unwrap();

        let txn = make_item(
            "txn1",
            "transactions",
            make_record(&[("txn_id", json!("T001"))]),
        );
        let item1 = make_item(
            "item1",
            "items",
            make_record(&[("txn_id", json!("T001")), ("product", json!("Milk"))]),
        );
        let item2 = make_item(
            "item2",
            "items",
            make_record(&[("txn_id", json!("T001")), ("product", json!("Bread"))]),
        );

        let result = stage
            .process_batch(vec![txn, item1, item2], &ctx())
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        let rec = result[0].record.as_ref().unwrap();
        let items = rec.get("line_items").unwrap().as_array().unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_assemble_multiple_joins() {
        let params = json!({
            "primary_stream": "transactions",
            "primary_key": "txn_id",
            "joins": [
                {
                    "stream": "stores",
                    "key": "store_id",
                    "foreign_key": "store_id",
                    "nest_as": "store",
                    "collect": false
                },
                {
                    "stream": "items",
                    "key": "txn_id",
                    "foreign_key": "txn_id",
                    "nest_as": "line_items",
                    "collect": true
                },
                {
                    "stream": "payments",
                    "key": "txn_id",
                    "foreign_key": "txn_id",
                    "nest_as": "payments",
                    "collect": true
                }
            ]
        });
        let stage = AssembleStage::from_params(&params).unwrap();

        let items = vec![
            make_item(
                "txn1",
                "transactions",
                make_record(&[("txn_id", json!("T001")), ("store_id", json!("S01"))]),
            ),
            make_item(
                "store1",
                "stores",
                make_record(&[
                    ("store_id", json!("S01")),
                    ("name", json!("Giant Eagle #1")),
                ]),
            ),
            make_item(
                "item1",
                "items",
                make_record(&[("txn_id", json!("T001")), ("product", json!("Milk"))]),
            ),
            make_item(
                "pay1",
                "payments",
                make_record(&[
                    ("txn_id", json!("T001")),
                    ("method", json!("Credit")),
                    ("amount", json!("42.50")),
                ]),
            ),
        ];

        let result = stage.process_batch(items, &ctx()).await.unwrap();
        assert_eq!(result.len(), 1);
        let rec = result[0].record.as_ref().unwrap();
        assert!(rec.get("store").unwrap().is_object());
        assert_eq!(rec.get("line_items").unwrap().as_array().unwrap().len(), 1);
        assert_eq!(rec.get("payments").unwrap().as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_assemble_no_match_null() {
        let params = json!({
            "primary_stream": "transactions",
            "primary_key": "txn_id",
            "joins": [{
                "stream": "stores",
                "key": "store_id",
                "foreign_key": "store_id",
                "nest_as": "store",
                "collect": false
            }]
        });
        let stage = AssembleStage::from_params(&params).unwrap();

        let txn = make_item(
            "txn1",
            "transactions",
            make_record(&[
                ("txn_id", json!("T001")),
                ("store_id", json!("S99")), // No matching store
            ]),
        );

        let result = stage.process_batch(vec![txn], &ctx()).await.unwrap();
        assert_eq!(result.len(), 1);
        let rec = result[0].record.as_ref().unwrap();
        assert!(rec.get("store").unwrap().is_null());
    }

    #[tokio::test]
    async fn test_assemble_no_match_collect_empty_array() {
        let params = json!({
            "primary_stream": "transactions",
            "primary_key": "txn_id",
            "joins": [{
                "stream": "items",
                "key": "txn_id",
                "foreign_key": "txn_id",
                "nest_as": "line_items",
                "collect": true
            }]
        });
        let stage = AssembleStage::from_params(&params).unwrap();

        let txn = make_item(
            "txn1",
            "transactions",
            make_record(&[("txn_id", json!("T001"))]),
        );

        // No items in the batch
        let result = stage.process_batch(vec![txn], &ctx()).await.unwrap();
        let rec = result[0].record.as_ref().unwrap();
        let items = rec.get("line_items").unwrap().as_array().unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_assemble_ge_full_receipt() {
        // Full Giant Eagle receipt structure
        let params = json!({
            "primary_stream": "transactions",
            "primary_key": "txn_id",
            "joins": [
                {
                    "stream": "stores",
                    "key": "store_id",
                    "foreign_key": "store_id",
                    "nest_as": "store",
                    "collect": false
                },
                {
                    "stream": "items",
                    "key": "txn_id",
                    "foreign_key": "txn_id",
                    "nest_as": "line_items",
                    "collect": true
                },
                {
                    "stream": "tenders",
                    "key": "txn_id",
                    "foreign_key": "txn_id",
                    "nest_as": "payments",
                    "collect": true
                }
            ]
        });
        let stage = AssembleStage::from_params(&params).unwrap();

        let items = vec![
            // Transaction
            make_item(
                "txn1",
                "transactions",
                make_record(&[
                    ("txn_id", json!("T001")),
                    ("store_id", json!("0101")),
                    ("date", json!("2024-03-15")),
                    ("total", json!("87.32")),
                ]),
            ),
            // Store
            make_item(
                "store1",
                "stores",
                make_record(&[
                    ("store_id", json!("0101")),
                    ("name", json!("Giant Eagle #0101")),
                    ("address", json!("100 Market Square")),
                    ("zip", json!("15213")),
                ]),
            ),
            // Items
            make_item(
                "item1",
                "items",
                make_record(&[
                    ("txn_id", json!("T001")),
                    ("upc", json!("041196010459")),
                    ("description", json!("Organic Milk 1gal")),
                    ("quantity", json!("1")),
                    ("price", json!("5.99")),
                ]),
            ),
            make_item(
                "item2",
                "items",
                make_record(&[
                    ("txn_id", json!("T001")),
                    ("upc", json!("071000006005")),
                    ("description", json!("Wheat Bread")),
                    ("quantity", json!("2")),
                    ("price", json!("3.49")),
                ]),
            ),
            // Tenders (split payment)
            make_item(
                "tender1",
                "tenders",
                make_record(&[
                    ("txn_id", json!("T001")),
                    ("payment_type", json!("Credit Card")),
                    ("amount", json!("50.00")),
                ]),
            ),
            make_item(
                "tender2",
                "tenders",
                make_record(&[
                    ("txn_id", json!("T001")),
                    ("payment_type", json!("Gift Card")),
                    ("amount", json!("37.32")),
                ]),
            ),
        ];

        let result = stage.process_batch(items, &ctx()).await.unwrap();
        assert_eq!(result.len(), 1);

        let receipt = result[0].record.as_ref().unwrap();
        assert_eq!(receipt.get("txn_id").unwrap(), "T001");
        assert_eq!(receipt.get("total").unwrap(), "87.32");

        // Nested store
        let store = receipt.get("store").unwrap().as_object().unwrap();
        assert_eq!(store.get("name").unwrap(), "Giant Eagle #0101");
        assert_eq!(store.get("zip").unwrap(), "15213");

        // Nested items
        let line_items = receipt.get("line_items").unwrap().as_array().unwrap();
        assert_eq!(line_items.len(), 2);

        // Nested payments (split)
        let payments = receipt.get("payments").unwrap().as_array().unwrap();
        assert_eq!(payments.len(), 2);
    }

    #[tokio::test]
    async fn test_assemble_empty_primary() {
        let params = json!({
            "primary_stream": "transactions",
            "primary_key": "txn_id"
        });
        let stage = AssembleStage::from_params(&params).unwrap();

        // Only items, no transactions
        let items = vec![make_item(
            "item1",
            "items",
            make_record(&[("txn_id", json!("T001"))]),
        )];

        let result = stage.process_batch(items, &ctx()).await.unwrap();
        assert!(result.is_empty());
    }
}
