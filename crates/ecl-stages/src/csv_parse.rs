//! CSV parsing stage: fan-out from file → individual records.
//!
//! Reads raw CSV content from a `PipelineItem`, parses each row into a
//! structured `Record`, and emits one `PipelineItem` per row. Supports
//! configurable column definitions with type conversion, custom delimiters,
//! and header handling.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;

use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, Record, Stage, StageContext};

/// Parsed from stage params TOML/JSON.
#[derive(Debug, Clone, Deserialize)]
struct CsvParseConfig {
    /// Column definitions in order.
    columns: Vec<ColumnDef>,
    /// Field delimiter character. Default: ','
    #[serde(default = "default_delimiter")]
    delimiter: char,
    /// Quote character. Default: '"'
    #[serde(default = "default_quote")]
    quote_char: char,
    /// Whether the CSV has a header row. Default: true
    #[serde(default = "default_has_headers")]
    has_headers: bool,
    /// How to handle parse errors for individual rows.
    /// "skip" = skip bad rows (log warning), "fail" = fail the item.
    /// Default: "skip"
    #[serde(default = "default_on_error")]
    on_row_error: String,
}

/// A single column definition.
#[derive(Debug, Clone, Deserialize)]
struct ColumnDef {
    /// Column name (used as Record field key).
    name: String,
    /// Column type for conversion: "string", "integer", "float", "boolean".
    /// Default: "string"
    #[serde(default = "default_column_type")]
    r#type: String,
}

fn default_delimiter() -> char {
    ','
}
fn default_quote() -> char {
    '"'
}
fn default_has_headers() -> bool {
    true
}
fn default_on_error() -> String {
    "skip".to_string()
}
fn default_column_type() -> String {
    "string".to_string()
}

/// Convert a raw string value to the appropriate JSON type.
fn convert_value(raw: &str, col_type: &str) -> serde_json::Value {
    match col_type {
        "integer" => raw
            .parse::<i64>()
            .map(serde_json::Value::from)
            .unwrap_or_else(|_| serde_json::Value::String(raw.to_string())),
        "float" => raw
            .parse::<f64>()
            .map(serde_json::Value::from)
            .unwrap_or_else(|_| serde_json::Value::String(raw.to_string())),
        "boolean" => match raw.to_lowercase().as_str() {
            "true" | "1" | "yes" | "y" => serde_json::Value::Bool(true),
            "false" | "0" | "no" | "n" => serde_json::Value::Bool(false),
            _ => serde_json::Value::String(raw.to_string()),
        },
        _ => serde_json::Value::String(raw.to_string()), // "string" or unknown
    }
}

/// CSV parsing stage: one file in, N record items out.
#[derive(Debug)]
pub struct CsvParseStage {
    config: CsvParseConfig,
}

impl CsvParseStage {
    /// Create from stage params JSON.
    ///
    /// # Errors
    ///
    /// Returns `StageError::Permanent` if the config is invalid.
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: CsvParseConfig =
            serde_json::from_value(params.clone()).map_err(|e| StageError::Permanent {
                stage: "csv_parse".to_string(),
                item_id: String::new(),
                message: format!("invalid csv_parse config: {e}"),
            })?;
        Ok(Self { config })
    }
}

#[async_trait]
impl Stage for CsvParseStage {
    fn name(&self) -> &str {
        "csv_parse"
    }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(self.config.delimiter as u8)
            .quote(self.config.quote_char as u8)
            .has_headers(self.config.has_headers)
            .from_reader(item.content.as_ref());

        let mut output = Vec::new();
        let columns = &self.config.columns;

        for (row_idx, result) in reader.records().enumerate() {
            let line_number = if self.config.has_headers {
                row_idx + 2 // 1-indexed, skip header
            } else {
                row_idx + 1
            };

            let csv_record = match result {
                Ok(r) => r,
                Err(e) => {
                    if self.config.on_row_error == "fail" {
                        return Err(StageError::Permanent {
                            stage: "csv_parse".to_string(),
                            item_id: item.id.clone(),
                            message: format!("row {line_number} parse error: {e}"),
                        });
                    }
                    tracing::warn!(
                        item_id = %item.id,
                        line = line_number,
                        "skipping bad CSV row: {e}"
                    );
                    continue;
                }
            };

            // Build the Record from column definitions.
            let mut record = Record::new();
            for (col_idx, col_def) in columns.iter().enumerate() {
                let raw = csv_record.get(col_idx).unwrap_or("");
                record.insert(col_def.name.clone(), convert_value(raw, &col_def.r#type));
            }

            // Build metadata inherited from parent + row-specific fields.
            let mut metadata = item.metadata.clone();
            metadata.insert(
                "_source_file".to_string(),
                serde_json::Value::String(item.display_name.clone()),
            );
            metadata.insert(
                "_line_number".to_string(),
                serde_json::Value::Number(serde_json::Number::from(line_number)),
            );

            // Build the raw CSV row bytes for debugging.
            let row_bytes: Vec<u8> = csv_record
                .iter()
                .collect::<Vec<_>>()
                .join(&(self.config.delimiter.to_string()))
                .into_bytes();

            output.push(PipelineItem {
                id: format!("{}:row:{line_number}", item.id),
                display_name: format!("{}:{line_number}", item.display_name),
                content: Arc::from(row_bytes),
                mime_type: "application/x-csv-row".to_string(),
                source_name: item.source_name.clone(),
                source_content_hash: item.source_content_hash.clone(),
                provenance: item.provenance.clone(),
                metadata,
                record: Some(record),
            });
        }

        Ok(output)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
    use serde_json::json;

    fn make_test_item(csv_content: &str) -> PipelineItem {
        PipelineItem {
            id: "file-1".to_string(),
            display_name: "test.csv".to_string(),
            content: Arc::from(csv_content.as_bytes()),
            mime_type: "text/csv".to_string(),
            source_name: "test".to_string(),
            source_content_hash: Blake3Hash::new("abc"),
            provenance: ItemProvenance {
                source_kind: "filesystem".to_string(),
                metadata: BTreeMap::new(),
                source_modified: None,
                extracted_at: chrono::Utc::now(),
            },
            metadata: BTreeMap::new(),
            record: None,
        }
    }

    fn string_columns(names: &[&str]) -> serde_json::Value {
        let cols: Vec<serde_json::Value> = names
            .iter()
            .map(|n| json!({"name": n, "type": "string"}))
            .collect();
        json!({
            "columns": cols,
            "has_headers": true
        })
    }

    #[tokio::test]
    async fn test_csv_parse_basic_three_columns() {
        let csv = "a,b,c\n1,2,3\n4,5,6\n7,8,9\n";
        let item = make_test_item(csv);
        let stage = CsvParseStage::from_params(&string_columns(&["a", "b", "c"])).unwrap();
        let ctx = StageContext {
            spec: Arc::new(
                ecl_pipeline_spec::PipelineSpec::from_toml(
                    "name = \"t\"\nversion = 1\noutput_dir = \"./o\"\n[sources.s]\nkind = \"filesystem\"\nroot = \"/tmp\"\n[stages.x]\nadapter = \"x\"\nresources = { creates = [\"y\"] }",
                ).unwrap(),
            ),
            output_dir: std::path::PathBuf::from("/tmp"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        };

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].record.as_ref().unwrap()["a"], json!("1"));
        assert_eq!(result[1].record.as_ref().unwrap()["b"], json!("5"));
        assert_eq!(result[2].record.as_ref().unwrap()["c"], json!("9"));
    }

    #[tokio::test]
    async fn test_csv_parse_type_conversion() {
        let csv = "name,age,score,active\nAlice,30,95.5,true\n";
        let item = make_test_item(csv);
        let params = json!({
            "columns": [
                {"name": "name", "type": "string"},
                {"name": "age", "type": "integer"},
                {"name": "score", "type": "float"},
                {"name": "active", "type": "boolean"},
            ],
            "has_headers": true
        });
        let stage = CsvParseStage::from_params(&params).unwrap();
        let ctx = StageContext {
            spec: Arc::new(
                ecl_pipeline_spec::PipelineSpec::from_toml(
                    "name = \"t\"\nversion = 1\noutput_dir = \"./o\"\n[sources.s]\nkind = \"filesystem\"\nroot = \"/tmp\"\n[stages.x]\nadapter = \"x\"\nresources = { creates = [\"y\"] }",
                ).unwrap(),
            ),
            output_dir: std::path::PathBuf::from("/tmp"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        };

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result.len(), 1);
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec["name"], json!("Alice"));
        assert_eq!(rec["age"], json!(30));
        assert_eq!(rec["score"], json!(95.5));
        assert_eq!(rec["active"], json!(true));
    }

    #[tokio::test]
    async fn test_csv_parse_type_conversion_fallback() {
        let csv = "val\nnot_a_number\n";
        let item = make_test_item(csv);
        let params = json!({
            "columns": [{"name": "val", "type": "integer"}],
            "has_headers": true
        });
        let stage = CsvParseStage::from_params(&params).unwrap();
        let ctx = StageContext {
            spec: Arc::new(
                ecl_pipeline_spec::PipelineSpec::from_toml(
                    "name = \"t\"\nversion = 1\noutput_dir = \"./o\"\n[sources.s]\nkind = \"filesystem\"\nroot = \"/tmp\"\n[stages.x]\nadapter = \"x\"\nresources = { creates = [\"y\"] }",
                ).unwrap(),
            ),
            output_dir: std::path::PathBuf::from("/tmp"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        };

        let result = stage.process(item, &ctx).await.unwrap();
        // Invalid integer falls back to string
        assert_eq!(
            result[0].record.as_ref().unwrap()["val"],
            json!("not_a_number")
        );
    }

    #[tokio::test]
    async fn test_csv_parse_fan_out_ids() {
        let csv = "x\n1\n2\n";
        let item = make_test_item(csv);
        let params = json!({"columns": [{"name": "x"}], "has_headers": true});
        let stage = CsvParseStage::from_params(&params).unwrap();
        let ctx = StageContext {
            spec: Arc::new(
                ecl_pipeline_spec::PipelineSpec::from_toml(
                    "name = \"t\"\nversion = 1\noutput_dir = \"./o\"\n[sources.s]\nkind = \"filesystem\"\nroot = \"/tmp\"\n[stages.x]\nadapter = \"x\"\nresources = { creates = [\"y\"] }",
                ).unwrap(),
            ),
            output_dir: std::path::PathBuf::from("/tmp"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        };

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result[0].id, "file-1:row:2");
        assert_eq!(result[1].id, "file-1:row:3");
        assert_eq!(result[0].display_name, "test.csv:2");
        assert_eq!(result[1].display_name, "test.csv:3");
    }

    #[tokio::test]
    async fn test_csv_parse_with_headers() {
        let csv = "header_a,header_b\nval1,val2\n";
        let item = make_test_item(csv);
        let params = json!({
            "columns": [{"name": "header_a"}, {"name": "header_b"}],
            "has_headers": true
        });
        let stage = CsvParseStage::from_params(&params).unwrap();
        let ctx = StageContext {
            spec: Arc::new(
                ecl_pipeline_spec::PipelineSpec::from_toml(
                    "name = \"t\"\nversion = 1\noutput_dir = \"./o\"\n[sources.s]\nkind = \"filesystem\"\nroot = \"/tmp\"\n[stages.x]\nadapter = \"x\"\nresources = { creates = [\"y\"] }",
                ).unwrap(),
            ),
            output_dir: std::path::PathBuf::from("/tmp"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        };

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].record.as_ref().unwrap()["header_a"],
            json!("val1")
        );
    }

    #[tokio::test]
    async fn test_csv_parse_without_headers() {
        let csv = "val1,val2\nval3,val4\n";
        let item = make_test_item(csv);
        let params = json!({
            "columns": [{"name": "col_a"}, {"name": "col_b"}],
            "has_headers": false
        });
        let stage = CsvParseStage::from_params(&params).unwrap();
        let ctx = StageContext {
            spec: Arc::new(
                ecl_pipeline_spec::PipelineSpec::from_toml(
                    "name = \"t\"\nversion = 1\noutput_dir = \"./o\"\n[sources.s]\nkind = \"filesystem\"\nroot = \"/tmp\"\n[stages.x]\nadapter = \"x\"\nresources = { creates = [\"y\"] }",
                ).unwrap(),
            ),
            output_dir: std::path::PathBuf::from("/tmp"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        };

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "file-1:row:1");
        assert_eq!(
            result[0].record.as_ref().unwrap()["col_a"],
            json!("val1")
        );
    }

    #[tokio::test]
    async fn test_csv_parse_custom_delimiter() {
        let csv = "a\tb\n1\t2\n";
        let item = make_test_item(csv);
        let params = json!({
            "columns": [{"name": "a"}, {"name": "b"}],
            "has_headers": true,
            "delimiter": "\t"
        });
        let stage = CsvParseStage::from_params(&params).unwrap();
        let ctx = StageContext {
            spec: Arc::new(
                ecl_pipeline_spec::PipelineSpec::from_toml(
                    "name = \"t\"\nversion = 1\noutput_dir = \"./o\"\n[sources.s]\nkind = \"filesystem\"\nroot = \"/tmp\"\n[stages.x]\nadapter = \"x\"\nresources = { creates = [\"y\"] }",
                ).unwrap(),
            ),
            output_dir: std::path::PathBuf::from("/tmp"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        };

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result[0].record.as_ref().unwrap()["a"], json!("1"));
        assert_eq!(result[0].record.as_ref().unwrap()["b"], json!("2"));
    }

    #[tokio::test]
    async fn test_csv_parse_quoted_fields() {
        let csv = "name,value\n\"Smith, John\",\"100\"\n";
        let item = make_test_item(csv);
        let params = json!({
            "columns": [{"name": "name"}, {"name": "value"}],
            "has_headers": true
        });
        let stage = CsvParseStage::from_params(&params).unwrap();
        let ctx = StageContext {
            spec: Arc::new(
                ecl_pipeline_spec::PipelineSpec::from_toml(
                    "name = \"t\"\nversion = 1\noutput_dir = \"./o\"\n[sources.s]\nkind = \"filesystem\"\nroot = \"/tmp\"\n[stages.x]\nadapter = \"x\"\nresources = { creates = [\"y\"] }",
                ).unwrap(),
            ),
            output_dir: std::path::PathBuf::from("/tmp"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        };

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(
            result[0].record.as_ref().unwrap()["name"],
            json!("Smith, John")
        );
    }

    #[tokio::test]
    async fn test_csv_parse_empty_file() {
        let csv = "a,b,c\n";
        let item = make_test_item(csv);
        let stage = CsvParseStage::from_params(&string_columns(&["a", "b", "c"])).unwrap();
        let ctx = StageContext {
            spec: Arc::new(
                ecl_pipeline_spec::PipelineSpec::from_toml(
                    "name = \"t\"\nversion = 1\noutput_dir = \"./o\"\n[sources.s]\nkind = \"filesystem\"\nroot = \"/tmp\"\n[stages.x]\nadapter = \"x\"\nresources = { creates = [\"y\"] }",
                ).unwrap(),
            ),
            output_dir: std::path::PathBuf::from("/tmp"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        };

        let result = stage.process(item, &ctx).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_csv_parse_row_error_skip() {
        // The csv crate handles most malformed rows gracefully, but we can test
        // with fewer fields than expected — they just get empty string.
        // For a true parse error, we'd need binary garbage.
        let csv = "a,b\n1,2\n3,4\n";
        let item = make_test_item(csv);
        let params = json!({
            "columns": [{"name": "a"}, {"name": "b"}],
            "has_headers": true,
            "on_row_error": "skip"
        });
        let stage = CsvParseStage::from_params(&params).unwrap();
        let ctx = StageContext {
            spec: Arc::new(
                ecl_pipeline_spec::PipelineSpec::from_toml(
                    "name = \"t\"\nversion = 1\noutput_dir = \"./o\"\n[sources.s]\nkind = \"filesystem\"\nroot = \"/tmp\"\n[stages.x]\nadapter = \"x\"\nresources = { creates = [\"y\"] }",
                ).unwrap(),
            ),
            output_dir: std::path::PathBuf::from("/tmp"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        };

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_csv_parse_metadata_inheritance() {
        let csv = "x\n1\n";
        let mut item = make_test_item(csv);
        item.metadata.insert(
            "parent_key".to_string(),
            serde_json::Value::String("parent_val".to_string()),
        );

        let params = json!({"columns": [{"name": "x"}], "has_headers": true});
        let stage = CsvParseStage::from_params(&params).unwrap();
        let ctx = StageContext {
            spec: Arc::new(
                ecl_pipeline_spec::PipelineSpec::from_toml(
                    "name = \"t\"\nversion = 1\noutput_dir = \"./o\"\n[sources.s]\nkind = \"filesystem\"\nroot = \"/tmp\"\n[stages.x]\nadapter = \"x\"\nresources = { creates = [\"y\"] }",
                ).unwrap(),
            ),
            output_dir: std::path::PathBuf::from("/tmp"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        };

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result[0].metadata["parent_key"], json!("parent_val"));
        assert_eq!(result[0].metadata["_source_file"], json!("test.csv"));
        assert_eq!(result[0].metadata["_line_number"], json!(2));
    }

    #[tokio::test]
    async fn test_csv_parse_affinity_schema() {
        let csv = "Channel_Aggregator_ID,Program_ID,Account_ID,Card_BIN,Card_Last_Four,Account_Postal_Code,Merchant_Location_MID,Merchant_Location_Name,Merchant_Location_Street,Merchant_Location_City,Merchant_Location_State,Merchant_Location_Postal_Code,Merchant_Location_Category_Code,Transaction_ID,Transaction_Date,Transaction_Settlement_Date,Transaction_Code,Transaction_Amount,Transaction_Auth_Code\nAGG001,PRG001,ACCT-001,411111,1234,90210,MID-001,WALGREENS #1234,123 Main St,Los Angeles,CA,90001,5912,TXN-001,03/15/2026,03/16/2026,PUR,25.99,123\n";
        let item = make_test_item(csv);
        let params = json!({
            "columns": [
                {"name": "Channel_Aggregator_ID"},
                {"name": "Program_ID"},
                {"name": "Account_ID"},
                {"name": "Card_BIN"},
                {"name": "Card_Last_Four"},
                {"name": "Account_Postal_Code"},
                {"name": "Merchant_Location_MID"},
                {"name": "Merchant_Location_Name"},
                {"name": "Merchant_Location_Street"},
                {"name": "Merchant_Location_City"},
                {"name": "Merchant_Location_State"},
                {"name": "Merchant_Location_Postal_Code"},
                {"name": "Merchant_Location_Category_Code"},
                {"name": "Transaction_ID"},
                {"name": "Transaction_Date"},
                {"name": "Transaction_Settlement_Date"},
                {"name": "Transaction_Code"},
                {"name": "Transaction_Amount", "type": "float"},
                {"name": "Transaction_Auth_Code"},
            ],
            "has_headers": true
        });
        let stage = CsvParseStage::from_params(&params).unwrap();
        let ctx = StageContext {
            spec: Arc::new(
                ecl_pipeline_spec::PipelineSpec::from_toml(
                    "name = \"t\"\nversion = 1\noutput_dir = \"./o\"\n[sources.s]\nkind = \"filesystem\"\nroot = \"/tmp\"\n[stages.x]\nadapter = \"x\"\nresources = { creates = [\"y\"] }",
                ).unwrap(),
            ),
            output_dir: std::path::PathBuf::from("/tmp"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        };

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result.len(), 1);
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec["Account_ID"], json!("ACCT-001"));
        assert_eq!(rec["Transaction_Amount"], json!(25.99));
        assert_eq!(rec["Card_Last_Four"], json!("1234"));
        assert_eq!(rec["Merchant_Location_Name"], json!("WALGREENS #1234"));
    }

    #[tokio::test]
    async fn test_csv_parse_from_params_invalid() {
        let params = json!({"not_valid": true});
        let result = CsvParseStage::from_params(&params);
        assert!(result.is_err());
    }
}
