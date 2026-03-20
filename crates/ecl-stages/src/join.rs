//! Join stage: merges records from two streams by key.
//!
//! Supports inner, left, and full join types. This is a batch stage
//! that requires all items at once to build the join index.

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, Stage, StageContext};

type Record = serde_json::Map<String, serde_json::Value>;

/// Configuration for the join stage, parsed from stage params.
#[derive(Debug, Clone, Deserialize)]
pub struct JoinConfig {
    /// Join type: "inner", "left", "full".
    #[serde(default = "default_join_type")]
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
    /// Default: `"{right_stream}_"`.
    #[serde(default)]
    pub right_prefix: Option<String>,
}

fn default_join_type() -> String {
    "left".to_string()
}

/// Join stage: merges records from two named streams by key.
///
/// This is a batch stage (`requires_batch() -> true`) because it needs
/// to see all items from both streams to build the right-side lookup index.
#[derive(Debug)]
pub struct JoinStage {
    config: JoinConfig,
}

impl JoinStage {
    /// Create a JoinStage from stage params JSON.
    ///
    /// # Errors
    ///
    /// Returns `StageError::Permanent` if the params cannot be deserialized
    /// into a `JoinConfig`.
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: JoinConfig =
            serde_json::from_value(params.clone()).map_err(|e| StageError::Permanent {
                stage: "join".to_string(),
                item_id: String::new(),
                message: format!("invalid join config: {e}"),
            })?;
        Ok(Self { config })
    }
}

#[async_trait]
impl Stage for JoinStage {
    fn name(&self) -> &str {
        "join"
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
            stage: "join".to_string(),
            item_id: String::new(),
            message: "join stage requires batch mode; use process_batch()".to_string(),
        })
    }

    async fn process_batch(
        &self,
        items: Vec<PipelineItem>,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let left_stream = &self.config.left_stream;
        let right_stream = &self.config.right_stream;

        debug!(
            left_stream,
            right_stream,
            join_type = %self.config.join_type,
            items = items.len(),
            "join stage starting"
        );

        // Partition items by stream.
        let mut left_items = Vec::new();
        let mut right_items = Vec::new();
        for item in items {
            match item.stream.as_deref() {
                Some(s) if s == left_stream => left_items.push(item),
                Some(s) if s == right_stream => right_items.push(item),
                _ => {
                    // Items not matching either stream are passed through unchanged.
                    left_items.push(item);
                }
            }
        }

        debug!(
            left = left_items.len(),
            right = right_items.len(),
            "partitioned items"
        );

        // Build right-side lookup: key → records.
        let mut right_index: HashMap<String, Vec<&Record>> = HashMap::new();
        for item in &right_items {
            if let Some(record) = &item.record
                && let Some(serde_json::Value::String(key)) = record.get(&self.config.right_key)
            {
                right_index.entry(key.clone()).or_default().push(record);
            }
        }

        let right_prefix = self
            .config
            .right_prefix
            .as_deref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{}_", right_stream));

        // For each left item, lookup and merge.
        let mut results = Vec::new();
        for left_item in left_items {
            let left_record = match &left_item.record {
                Some(r) => r,
                None => {
                    return Err(StageError::Permanent {
                        stage: "join".to_string(),
                        item_id: left_item.id.clone(),
                        message: "left item has no record".to_string(),
                    });
                }
            };

            let left_key = left_record
                .get(&self.config.left_key)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            match right_index.get(left_key) {
                Some(right_records) => {
                    // Matched: merge first right record's fields into left.
                    let mut merged = left_record.clone();
                    if let Some(right_record) = right_records.first() {
                        for (key, value) in *right_record {
                            if key != &self.config.right_key {
                                merged.insert(format!("{right_prefix}{key}"), value.clone());
                            }
                        }
                    }
                    results.push(PipelineItem {
                        record: Some(merged),
                        ..left_item
                    });
                }
                None => match self.config.join_type.as_str() {
                    "inner" => {
                        // Drop unmatched left items.
                    }
                    _ => {
                        // "left" or "full": keep left item as-is.
                        results.push(left_item);
                    }
                },
            }
        }

        // For full join, add unmatched right items.
        if self.config.join_type == "full" {
            let matched_keys: HashSet<String> = results
                .iter()
                .filter_map(|i| {
                    i.record
                        .as_ref()?
                        .get(&self.config.left_key)?
                        .as_str()
                        .map(|s| s.to_string())
                })
                .collect();
            for right_item in right_items {
                if let Some(record) = &right_item.record {
                    let key = record
                        .get(&self.config.right_key)
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !matched_keys.contains(key) {
                        results.push(right_item);
                    }
                }
            }
        }

        debug!(output_items = results.len(), "join stage complete");
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
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::sync::Arc;

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
                output_dir: PathBuf::from("./out"),
                sources: BTreeMap::new(),
                stages: BTreeMap::new(),
                defaults: ecl_pipeline_spec::DefaultsSpec::default(),
                lifecycle: None,
                secrets: Default::default(),
                triggers: None,
                schedule: None,
            }),
            output_dir: PathBuf::from("./out"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        }
    }

    fn make_join_stage(join_type: &str) -> JoinStage {
        let params = json!({
            "join_type": join_type,
            "left_stream": "items",
            "right_stream": "products",
            "left_key": "upc",
            "right_key": "upc",
        });
        JoinStage::from_params(&params).unwrap()
    }

    #[test]
    fn test_join_requires_batch_true() {
        let stage = make_join_stage("inner");
        assert!(stage.requires_batch());
    }

    #[tokio::test]
    async fn test_join_process_returns_error() {
        let stage = make_join_stage("inner");
        let item = make_item("x", "items", make_record(&[("upc", json!("123"))]));
        let ctx = make_context();
        let result = stage.process(item, &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_join_inner_basic() {
        let stage = make_join_stage("inner");
        let ctx = make_context();

        let items = vec![
            make_item(
                "l1",
                "items",
                make_record(&[("upc", json!("A")), ("qty", json!(10))]),
            ),
            make_item(
                "l2",
                "items",
                make_record(&[("upc", json!("B")), ("qty", json!(20))]),
            ),
            make_item(
                "l3",
                "items",
                make_record(&[("upc", json!("C")), ("qty", json!(30))]),
            ),
            make_item(
                "r1",
                "products",
                make_record(&[("upc", json!("A")), ("brand", json!("Acme"))]),
            ),
            make_item(
                "r2",
                "products",
                make_record(&[("upc", json!("B")), ("brand", json!("Beta"))]),
            ),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        assert_eq!(result.len(), 2); // C has no match → dropped
        assert!(
            result
                .iter()
                .all(|i| i.record.as_ref().unwrap().contains_key("products_brand"))
        );
    }

    #[tokio::test]
    async fn test_join_left_basic() {
        let stage = make_join_stage("left");
        let ctx = make_context();

        let items = vec![
            make_item(
                "l1",
                "items",
                make_record(&[("upc", json!("A")), ("qty", json!(10))]),
            ),
            make_item(
                "l2",
                "items",
                make_record(&[("upc", json!("B")), ("qty", json!(20))]),
            ),
            make_item(
                "l3",
                "items",
                make_record(&[("upc", json!("C")), ("qty", json!(30))]),
            ),
            make_item(
                "r1",
                "products",
                make_record(&[("upc", json!("A")), ("brand", json!("Acme"))]),
            ),
            make_item(
                "r2",
                "products",
                make_record(&[("upc", json!("B")), ("brand", json!("Beta"))]),
            ),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        assert_eq!(result.len(), 3); // C kept with no right fields
        let c_item = result.iter().find(|i| i.id == "l3").unwrap();
        assert!(
            !c_item
                .record
                .as_ref()
                .unwrap()
                .contains_key("products_brand")
        );
    }

    #[tokio::test]
    async fn test_join_full_basic() {
        let stage = make_join_stage("full");
        let ctx = make_context();

        let items = vec![
            make_item(
                "l1",
                "items",
                make_record(&[("upc", json!("A")), ("qty", json!(10))]),
            ),
            make_item(
                "l2",
                "items",
                make_record(&[("upc", json!("C")), ("qty", json!(30))]),
            ),
            make_item(
                "r1",
                "products",
                make_record(&[("upc", json!("A")), ("brand", json!("Acme"))]),
            ),
            make_item(
                "r2",
                "products",
                make_record(&[("upc", json!("D")), ("brand", json!("Delta"))]),
            ),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        // A matched, C unmatched left, D unmatched right
        assert_eq!(result.len(), 3);
        assert!(result.iter().any(|i| i.id == "l1")); // matched
        assert!(result.iter().any(|i| i.id == "l2")); // unmatched left
        assert!(result.iter().any(|i| i.id == "r2")); // unmatched right
    }

    #[tokio::test]
    async fn test_join_right_prefix_applied() {
        let params = json!({
            "join_type": "inner",
            "left_stream": "items",
            "right_stream": "products",
            "left_key": "upc",
            "right_key": "upc",
            "right_prefix": "prod_",
        });
        let stage = JoinStage::from_params(&params).unwrap();
        let ctx = make_context();

        let items = vec![
            make_item(
                "l1",
                "items",
                make_record(&[("upc", json!("A")), ("qty", json!(10))]),
            ),
            make_item(
                "r1",
                "products",
                make_record(&[("upc", json!("A")), ("brand", json!("Acme"))]),
            ),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(
            result[0]
                .record
                .as_ref()
                .unwrap()
                .contains_key("prod_brand")
        );
        assert!(
            !result[0]
                .record
                .as_ref()
                .unwrap()
                .contains_key("products_brand")
        );
    }

    #[tokio::test]
    async fn test_join_no_matches_left() {
        let stage = make_join_stage("left");
        let ctx = make_context();

        let items = vec![
            make_item("l1", "items", make_record(&[("upc", json!("X"))])),
            make_item("l2", "items", make_record(&[("upc", json!("Y"))])),
            make_item(
                "r1",
                "products",
                make_record(&[("upc", json!("A")), ("brand", json!("Acme"))]),
            ),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        assert_eq!(result.len(), 2); // All left items kept
    }

    #[tokio::test]
    async fn test_join_no_matches_inner() {
        let stage = make_join_stage("inner");
        let ctx = make_context();

        let items = vec![
            make_item("l1", "items", make_record(&[("upc", json!("X"))])),
            make_item("r1", "products", make_record(&[("upc", json!("A"))])),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        assert_eq!(result.len(), 0); // No matches
    }

    #[tokio::test]
    async fn test_join_duplicate_keys_right() {
        let stage = make_join_stage("inner");
        let ctx = make_context();

        let items = vec![
            make_item(
                "l1",
                "items",
                make_record(&[("upc", json!("A")), ("qty", json!(10))]),
            ),
            make_item(
                "r1",
                "products",
                make_record(&[("upc", json!("A")), ("brand", json!("First"))]),
            ),
            make_item(
                "r2",
                "products",
                make_record(&[("upc", json!("A")), ("brand", json!("Second"))]),
            ),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        assert_eq!(result.len(), 1);
        // First match used.
        assert_eq!(
            result[0].record.as_ref().unwrap()["products_brand"],
            json!("First")
        );
    }

    #[tokio::test]
    async fn test_join_missing_key_field() {
        let stage = make_join_stage("left");
        let ctx = make_context();

        // Right item has no "upc" field — should just not match.
        let items = vec![
            make_item("l1", "items", make_record(&[("upc", json!("A"))])),
            make_item("r1", "products", make_record(&[("name", json!("Widget"))])),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        assert_eq!(result.len(), 1); // Left kept, no match
        assert_eq!(result[0].id, "l1");
    }

    #[tokio::test]
    async fn test_join_no_record_returns_error() {
        let stage = make_join_stage("inner");
        let ctx = make_context();

        let mut item = make_item("l1", "items", make_record(&[("upc", json!("A"))]));
        item.record = None; // No record!

        let result = stage.process_batch(vec![item], &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_join_empty_inputs() {
        let stage = make_join_stage("inner");
        let ctx = make_context();

        let result = stage.process_batch(vec![], &ctx).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_join_ge_items_products() {
        // Realistic Giant Eagle scenario: items stream has UPC+qty,
        // products stream has UPC+brand+description.
        let stage = make_join_stage("left");
        let ctx = make_context();

        let items = vec![
            make_item(
                "item-1",
                "items",
                make_record(&[
                    ("upc", json!("0049000042566")),
                    ("qty", json!(24)),
                    ("store_id", json!("GE-1234")),
                ]),
            ),
            make_item(
                "item-2",
                "items",
                make_record(&[
                    ("upc", json!("0012000001086")),
                    ("qty", json!(12)),
                    ("store_id", json!("GE-1234")),
                ]),
            ),
            make_item(
                "prod-1",
                "products",
                make_record(&[
                    ("upc", json!("0049000042566")),
                    ("brand", json!("Coca-Cola")),
                    ("description", json!("Coca-Cola Classic 12oz")),
                ]),
            ),
            make_item(
                "prod-2",
                "products",
                make_record(&[
                    ("upc", json!("0012000001086")),
                    ("brand", json!("Pepsi")),
                    ("description", json!("Pepsi Cola 12oz")),
                ]),
            ),
        ];

        let result = stage.process_batch(items, &ctx).await.unwrap();
        assert_eq!(result.len(), 2);

        let item1 = result.iter().find(|i| i.id == "item-1").unwrap();
        let record1 = item1.record.as_ref().unwrap();
        assert_eq!(record1["products_brand"], json!("Coca-Cola"));
        assert_eq!(
            record1["products_description"],
            json!("Coca-Cola Classic 12oz")
        );
        assert_eq!(record1["qty"], json!(24));
        assert_eq!(record1["store_id"], json!("GE-1234"));

        let item2 = result.iter().find(|i| i.id == "item-2").unwrap();
        let record2 = item2.record.as_ref().unwrap();
        assert_eq!(record2["products_brand"], json!("Pepsi"));
    }
}
