//! Lookup stage: static value mapping for enumerated fields.
//!
//! Maps field values through lookup tables (e.g., payment type codes to names,
//! card scheme codes to standard abbreviations). Supports case-insensitive
//! matching and default fallback values.

use std::collections::HashMap;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use tracing::debug;

use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, Stage, StageContext};

type Record = serde_json::Map<String, serde_json::Value>;

/// Configuration for the lookup stage, parsed from stage params.
#[derive(Debug, Clone, Deserialize)]
pub struct LookupConfig {
    /// List of lookup operations to apply.
    pub lookups: Vec<LookupOp>,
}

/// A single lookup operation mapping values from one field to another.
#[derive(Debug, Clone, Deserialize)]
pub struct LookupOp {
    /// Source field to look up.
    pub field: String,
    /// Output field for the mapped value.
    pub output: String,
    /// Lookup table: { input_value: output_value }.
    pub table: std::collections::BTreeMap<String, String>,
    /// Default value if input not found in table.
    #[serde(default)]
    pub default: Option<String>,
    /// Case-insensitive matching. Default: false.
    #[serde(default)]
    pub case_insensitive: bool,
}

/// Lookup stage that maps field values through static tables.
///
/// Pre-builds `HashMap`s at construction time for O(1) lookups.
/// For case-insensitive operations, keys are lowercased at build time.
#[derive(Debug)]
pub struct LookupStage {
    config: LookupConfig,
    /// Pre-built lookup tables (optionally lowercased keys for case-insensitive).
    tables: Vec<HashMap<String, String>>,
}

impl LookupStage {
    /// Create a lookup stage from JSON params.
    ///
    /// # Errors
    ///
    /// Returns `StageError::Permanent` if params cannot be deserialized.
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: LookupConfig =
            serde_json::from_value(params.clone()).map_err(|e| StageError::Permanent {
                stage: "lookup".into(),
                item_id: String::new(),
                message: format!("invalid lookup config: {e}"),
            })?;

        let tables = config
            .lookups
            .iter()
            .map(|op| {
                if op.case_insensitive {
                    op.table
                        .iter()
                        .map(|(k, v)| (k.to_lowercase(), v.clone()))
                        .collect()
                } else {
                    op.table
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect()
                }
            })
            .collect();

        Ok(Self { config, tables })
    }

    /// Apply all lookup operations to a record.
    fn apply_lookups(&self, record: &mut Record) {
        for (i, op) in self.config.lookups.iter().enumerate() {
            let input_value = record.get(&op.field).and_then(|v| v.as_str()).unwrap_or("");

            let lookup_key = if op.case_insensitive {
                input_value.to_lowercase()
            } else {
                input_value.to_string()
            };

            let output_value = self.tables[i]
                .get(&lookup_key)
                .cloned()
                .or_else(|| op.default.clone());

            if let Some(val) = output_value {
                record.insert(op.output.clone(), Value::String(val));
            } else {
                record.insert(op.output.clone(), Value::Null);
            }
        }
    }
}

#[async_trait]
impl Stage for LookupStage {
    fn name(&self) -> &str {
        "lookup"
    }

    async fn process(
        &self,
        mut item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let record = item.record.as_mut().ok_or_else(|| StageError::Permanent {
            stage: "lookup".into(),
            item_id: item.id.clone(),
            message: "item has no record".into(),
        })?;

        debug!(
            item_id = %item.id,
            lookups = self.config.lookups.len(),
            "applying lookup tables"
        );

        self.apply_lookups(record);

        Ok(vec![item])
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
    use std::sync::Arc;

    fn make_item(id: &str, record: serde_json::Map<String, Value>) -> PipelineItem {
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
    async fn test_lookup_basic_match() {
        let params = json!({
            "lookups": [{
                "field": "code",
                "output": "name",
                "table": { "A": "Alpha", "B": "Beta", "C": "Charlie" }
            }]
        });
        let stage = LookupStage::from_params(&params).unwrap();
        let mut record = serde_json::Map::new();
        record.insert("code".into(), json!("B"));
        let item = make_item("i1", record);

        let result = stage.process(item, &ctx()).await.unwrap();
        assert_eq!(result.len(), 1);
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec.get("name").unwrap(), "Beta");
    }

    #[tokio::test]
    async fn test_lookup_no_match_default() {
        let params = json!({
            "lookups": [{
                "field": "code",
                "output": "name",
                "table": { "A": "Alpha" },
                "default": "Unknown"
            }]
        });
        let stage = LookupStage::from_params(&params).unwrap();
        let mut record = serde_json::Map::new();
        record.insert("code".into(), json!("Z"));
        let item = make_item("i1", record);

        let result = stage.process(item, &ctx()).await.unwrap();
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec.get("name").unwrap(), "Unknown");
    }

    #[tokio::test]
    async fn test_lookup_no_match_no_default_null() {
        let params = json!({
            "lookups": [{
                "field": "code",
                "output": "name",
                "table": { "A": "Alpha" }
            }]
        });
        let stage = LookupStage::from_params(&params).unwrap();
        let mut record = serde_json::Map::new();
        record.insert("code".into(), json!("Z"));
        let item = make_item("i1", record);

        let result = stage.process(item, &ctx()).await.unwrap();
        let rec = result[0].record.as_ref().unwrap();
        assert!(rec.get("name").unwrap().is_null());
    }

    #[tokio::test]
    async fn test_lookup_case_insensitive() {
        let params = json!({
            "lookups": [{
                "field": "code",
                "output": "name",
                "table": { "visa": "Visa", "mastercard": "MasterCard" },
                "case_insensitive": true
            }]
        });
        let stage = LookupStage::from_params(&params).unwrap();
        let mut record = serde_json::Map::new();
        record.insert("code".into(), json!("VISA"));
        let item = make_item("i1", record);

        let result = stage.process(item, &ctx()).await.unwrap();
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec.get("name").unwrap(), "Visa");
    }

    #[tokio::test]
    async fn test_lookup_multiple_lookups_in_one_stage() {
        let params = json!({
            "lookups": [
                {
                    "field": "payment_code",
                    "output": "payment_type",
                    "table": { "01": "Credit", "02": "Debit", "03": "Cash" }
                },
                {
                    "field": "scheme_code",
                    "output": "scheme_name",
                    "table": { "VI": "Visa", "MC": "MasterCard", "AX": "AmEx" }
                }
            ]
        });
        let stage = LookupStage::from_params(&params).unwrap();
        let mut record = serde_json::Map::new();
        record.insert("payment_code".into(), json!("02"));
        record.insert("scheme_code".into(), json!("VI"));
        let item = make_item("i1", record);

        let result = stage.process(item, &ctx()).await.unwrap();
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec.get("payment_type").unwrap(), "Debit");
        assert_eq!(rec.get("scheme_name").unwrap(), "Visa");
    }

    #[tokio::test]
    async fn test_lookup_ge_payment_types() {
        // Giant Eagle payment type mapping
        let params = json!({
            "lookups": [{
                "field": "tender_type",
                "output": "payment_method",
                "table": {
                    "01": "Cash",
                    "02": "Check",
                    "03": "Credit Card",
                    "04": "Debit Card",
                    "05": "EBT",
                    "06": "Gift Card",
                    "07": "Store Credit",
                    "08": "Mobile Payment"
                },
                "default": "Other"
            }]
        });
        let stage = LookupStage::from_params(&params).unwrap();

        // Credit card
        let mut rec = serde_json::Map::new();
        rec.insert("tender_type".into(), json!("03"));
        let result = stage.process(make_item("i1", rec), &ctx()).await.unwrap();
        assert_eq!(
            result[0]
                .record
                .as_ref()
                .unwrap()
                .get("payment_method")
                .unwrap(),
            "Credit Card"
        );

        // Unknown type uses default
        let mut rec = serde_json::Map::new();
        rec.insert("tender_type".into(), json!("99"));
        let result = stage.process(make_item("i2", rec), &ctx()).await.unwrap();
        assert_eq!(
            result[0]
                .record
                .as_ref()
                .unwrap()
                .get("payment_method")
                .unwrap(),
            "Other"
        );
    }

    #[tokio::test]
    async fn test_lookup_ge_payment_schemes() {
        // Giant Eagle card scheme mapping
        let params = json!({
            "lookups": [{
                "field": "card_scheme",
                "output": "scheme_abbrev",
                "table": {
                    "Visa": "VI",
                    "MasterCard": "MC",
                    "American Express": "AX",
                    "Discover": "DI"
                },
                "case_insensitive": true
            }]
        });
        let stage = LookupStage::from_params(&params).unwrap();

        let mut rec = serde_json::Map::new();
        rec.insert("card_scheme".into(), json!("VISA"));
        let result = stage.process(make_item("i1", rec), &ctx()).await.unwrap();
        assert_eq!(
            result[0]
                .record
                .as_ref()
                .unwrap()
                .get("scheme_abbrev")
                .unwrap(),
            "VI"
        );

        let mut rec = serde_json::Map::new();
        rec.insert("card_scheme".into(), json!("mastercard"));
        let result = stage.process(make_item("i2", rec), &ctx()).await.unwrap();
        assert_eq!(
            result[0]
                .record
                .as_ref()
                .unwrap()
                .get("scheme_abbrev")
                .unwrap(),
            "MC"
        );
    }

    #[tokio::test]
    async fn test_lookup_missing_field_uses_empty_string() {
        let params = json!({
            "lookups": [{
                "field": "missing_field",
                "output": "result",
                "table": { "": "empty_match", "x": "x_match" }
            }]
        });
        let stage = LookupStage::from_params(&params).unwrap();
        let record = serde_json::Map::new(); // no fields at all
        let item = make_item("i1", record);

        let result = stage.process(item, &ctx()).await.unwrap();
        let rec = result[0].record.as_ref().unwrap();
        // Missing field treated as empty string, which matches "" key
        assert_eq!(rec.get("result").unwrap(), "empty_match");
    }
}
