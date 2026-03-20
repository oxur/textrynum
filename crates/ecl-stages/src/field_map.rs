//! Field mapping stage: rename, drop, set, copy, parse dates, pad, regex extract, nest.
//!
//! Operates on the `Record` attached to each `PipelineItem`. Each operation
//! type is applied in a fixed order to ensure deterministic results:
//! rename → copy → set → date parse → pad → regex extract → nest → drop.

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::NaiveDate;
use regex::Regex;
use serde::Deserialize;
use tracing::debug;

use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, Record, Stage, StageContext};

/// Configuration for the field mapping stage, deserialized from stage params.
#[derive(Debug, Clone, Deserialize)]
struct FieldMapConfig {
    /// Rename fields: `{ from: "old_name", to: "new_name" }`.
    #[serde(default)]
    rename: Vec<RenameOp>,

    /// Drop fields by name.
    #[serde(default)]
    drop: Vec<String>,

    /// Set literal values: `{ field: "name", value: <json_value> }`.
    #[serde(default)]
    set: Vec<SetOp>,

    /// Copy a field: `{ from: "source", to: "dest" }`.
    #[serde(default)]
    copy: Vec<CopyOp>,

    /// Date parsing: `{ field: "date_col", format: "%m/%d/%Y", output: "ts", timezone: "UTC" }`.
    #[serde(default)]
    parse_dates: Vec<DateParseOp>,

    /// String padding: `{ field: "auth_code", width: 6, pad_char: "0", side: "left" }`.
    #[serde(default)]
    pad: Vec<PadOp>,

    /// Regex extract: `{ field: "src", pattern: "...", output: "dest", group: 1 }`.
    #[serde(default)]
    regex_extract: Vec<RegexExtractOp>,

    /// Nested object construction: `{ output: "payment", fields: { "out": "src" } }`.
    #[serde(default)]
    nest: Vec<NestOp>,
}

/// Rename a field from one name to another.
#[derive(Debug, Clone, Deserialize)]
struct RenameOp {
    /// Source field name.
    from: String,
    /// Destination field name.
    to: String,
}

/// Set a literal value on a field.
#[derive(Debug, Clone, Deserialize)]
struct SetOp {
    /// Field name to set.
    field: String,
    /// Literal JSON value.
    value: serde_json::Value,
}

/// Copy a field value to a new field.
#[derive(Debug, Clone, Deserialize)]
struct CopyOp {
    /// Source field name.
    from: String,
    /// Destination field name.
    to: String,
}

/// Parse a date string into RFC 3339 format.
#[derive(Debug, Clone, Deserialize)]
struct DateParseOp {
    /// Source field containing the date string.
    field: String,
    /// strftime format string (e.g., `"%m/%d/%Y"`).
    format: String,
    /// Output field name for the parsed date.
    output: String,
    /// Timezone (default: `"UTC"`). Reserved for future IANA timezone support.
    #[serde(default = "default_utc")]
    #[allow(dead_code)]
    timezone: String,
}

/// Pad a string field to a minimum width.
#[derive(Debug, Clone, Deserialize)]
struct PadOp {
    /// Field name to pad.
    field: String,
    /// Minimum width.
    width: usize,
    /// Character to pad with (default: `"0"`).
    #[serde(default = "default_pad_char")]
    pad_char: String,
    /// Side to pad on: `"left"` or `"right"` (default: `"left"`).
    #[serde(default = "default_pad_side")]
    side: String,
}

/// Extract a capture group from a regex match.
#[derive(Debug, Clone, Deserialize)]
struct RegexExtractOp {
    /// Source field to match against.
    field: String,
    /// Regex pattern (with capture groups).
    pattern: String,
    /// Output field name for the extracted value.
    output: String,
    /// Capture group index (default: 1).
    #[serde(default = "default_group")]
    group: usize,
}

/// Group fields into a nested JSON object.
#[derive(Debug, Clone, Deserialize)]
struct NestOp {
    /// Output field name for the nested object.
    output: String,
    /// Map of output_field_name → source_field_name.
    fields: BTreeMap<String, String>,
}

fn default_utc() -> String {
    "UTC".to_string()
}

fn default_pad_char() -> String {
    "0".to_string()
}

fn default_pad_side() -> String {
    "left".to_string()
}

fn default_group() -> usize {
    1
}

/// Field mapping stage that transforms record fields.
///
/// Applies rename, copy, set, date parse, pad, regex extract, nest, and drop
/// operations in a fixed order. Regexes are pre-compiled at construction time
/// for efficiency.
#[derive(Debug)]
pub struct FieldMapStage {
    /// Deserialized configuration.
    config: FieldMapConfig,
    /// Pre-compiled regexes indexed by position in `config.regex_extract`.
    compiled_regexes: Vec<(usize, Regex)>,
}

impl FieldMapStage {
    /// Create a new field map stage from stage params JSON.
    ///
    /// # Errors
    ///
    /// Returns `StageError::Permanent` if the params cannot be deserialized
    /// or if a regex pattern is invalid.
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: FieldMapConfig =
            serde_json::from_value(params.clone()).map_err(|e| StageError::Permanent {
                stage: "field_map".to_string(),
                item_id: String::new(),
                message: format!("invalid field_map params: {e}"),
            })?;

        let compiled_regexes = config
            .regex_extract
            .iter()
            .enumerate()
            .map(|(i, op)| {
                Regex::new(&op.pattern)
                    .map(|r| (i, r))
                    .map_err(|e| StageError::Permanent {
                        stage: "field_map".to_string(),
                        item_id: String::new(),
                        message: format!("invalid regex in regex_extract[{i}]: {e}"),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            config,
            compiled_regexes,
        })
    }

    /// Apply all rename operations to the record.
    fn apply_renames(record: &mut Record, ops: &[RenameOp]) {
        for op in ops {
            if let Some(val) = record.remove(&op.from) {
                record.insert(op.to.clone(), val);
            }
        }
    }

    /// Apply all copy operations to the record.
    fn apply_copies(record: &mut Record, ops: &[CopyOp]) {
        for op in ops {
            if let Some(val) = record.get(&op.from).cloned() {
                record.insert(op.to.clone(), val);
            }
        }
    }

    /// Apply all set operations to the record.
    fn apply_sets(record: &mut Record, ops: &[SetOp]) {
        for op in ops {
            record.insert(op.field.clone(), op.value.clone());
        }
    }

    /// Apply all date parse operations to the record.
    fn apply_date_parses(record: &mut Record, ops: &[DateParseOp]) {
        for op in ops {
            let parsed = record
                .get(&op.field)
                .and_then(|v| v.as_str())
                .and_then(|s| NaiveDate::parse_from_str(s, &op.format).ok())
                .map(|d| {
                    // Convert to RFC 3339 date string (midnight UTC)
                    let dt = d.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc();
                    serde_json::Value::String(dt.to_rfc3339())
                });

            match parsed {
                Some(val) => {
                    record.insert(op.output.clone(), val);
                }
                None => {
                    record.insert(op.output.clone(), serde_json::Value::Null);
                }
            }
        }
    }

    /// Apply all pad operations to the record.
    fn apply_pads(record: &mut Record, ops: &[PadOp]) {
        for op in ops {
            let padded = record.get(&op.field).and_then(|v| v.as_str()).map(|s| {
                let pad_char = op.pad_char.chars().next().unwrap_or('0');
                if s.len() >= op.width {
                    s.to_string()
                } else {
                    let padding_needed = op.width - s.len();
                    let pad_str: String = std::iter::repeat_n(pad_char, padding_needed).collect();
                    if op.side == "right" {
                        format!("{s}{pad_str}")
                    } else {
                        format!("{pad_str}{s}")
                    }
                }
            });

            if let Some(val) = padded {
                record.insert(op.field.clone(), serde_json::Value::String(val));
            }
        }
    }

    /// Apply all regex extract operations to the record.
    fn apply_regex_extracts(
        record: &mut Record,
        ops: &[RegexExtractOp],
        compiled: &[(usize, Regex)],
    ) {
        for (idx, re) in compiled {
            let op = &ops[*idx];
            let extracted = record
                .get(&op.field)
                .and_then(|v| v.as_str())
                .and_then(|s| re.captures(s))
                .and_then(|caps| caps.get(op.group))
                .map(|m| serde_json::Value::String(m.as_str().to_string()));

            match extracted {
                Some(val) => {
                    record.insert(op.output.clone(), val);
                }
                None => {
                    record.insert(op.output.clone(), serde_json::Value::Null);
                }
            }
        }
    }

    /// Apply all nest operations to the record.
    fn apply_nests(record: &mut Record, ops: &[NestOp]) {
        for op in ops {
            let mut nested = serde_json::Map::new();
            for (out_field, src_field) in &op.fields {
                if let Some(val) = record.get(src_field).cloned() {
                    nested.insert(out_field.clone(), val);
                }
            }
            record.insert(op.output.clone(), serde_json::Value::Object(nested));
        }
    }

    /// Apply all drop operations to the record.
    fn apply_drops(record: &mut Record, fields: &[String]) {
        for field in fields {
            record.remove(field);
        }
    }
}

#[async_trait]
impl Stage for FieldMapStage {
    fn name(&self) -> &str {
        "field_map"
    }

    async fn process(
        &self,
        mut item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let record = item.record.as_mut().ok_or_else(|| StageError::Permanent {
            stage: "field_map".to_string(),
            item_id: item.id.clone(),
            message: "field_map requires a record (did you run csv_parse first?)".to_string(),
        })?;

        debug!(item_id = %item.id, "applying field mappings");

        // Apply operations in fixed order
        Self::apply_renames(record, &self.config.rename);
        Self::apply_copies(record, &self.config.copy);
        Self::apply_sets(record, &self.config.set);
        Self::apply_date_parses(record, &self.config.parse_dates);
        Self::apply_pads(record, &self.config.pad);
        Self::apply_regex_extracts(record, &self.config.regex_extract, &self.compiled_regexes);
        Self::apply_nests(record, &self.config.nest);
        Self::apply_drops(record, &self.config.drop);

        Ok(vec![item])
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
    use serde_json::json;
    use std::sync::Arc;

    fn make_record(fields: &[(&str, serde_json::Value)]) -> Record {
        let mut record = Record::new();
        for (k, v) in fields {
            record.insert((*k).to_string(), v.clone());
        }
        record
    }

    fn make_item_with_record(record: Record) -> PipelineItem {
        PipelineItem {
            id: "row-1".to_string(),
            display_name: "test.csv:1".to_string(),
            content: Arc::from(b"" as &[u8]),
            mime_type: "application/x-csv-row".to_string(),
            source_name: "test".to_string(),
            source_content_hash: Blake3Hash::new("abc"),
            provenance: ItemProvenance {
                source_kind: "filesystem".to_string(),
                metadata: BTreeMap::new(),
                source_modified: None,
                extracted_at: chrono::Utc::now(),
            },
            metadata: BTreeMap::new(),
            record: Some(record),
            stream: None,
        }
    }

    fn make_context() -> StageContext {
        use ecl_pipeline_spec::PipelineSpec;
        use std::path::PathBuf;
        StageContext {
            spec: Arc::new(
                PipelineSpec::from_toml(
                    r#"
name = "test"
version = 1
output_dir = "./out"

[sources.local]
kind = "filesystem"
root = "/tmp"

[stages.field_map]
adapter = "field_map"
resources = { creates = ["mapped"] }
"#,
                )
                .unwrap(),
            ),
            output_dir: PathBuf::from("./out"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        }
    }

    #[test]
    fn test_field_map_rename_basic() {
        let params = json!({ "rename": [{ "from": "old_name", "to": "new_name" }] });
        let stage = FieldMapStage::from_params(&params).unwrap();
        let record = make_record(&[("old_name", json!("Alice"))]);
        let item = make_item_with_record(record);
        let ctx = make_context();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(stage.process(item, &ctx)).unwrap();
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec.get("new_name").unwrap(), "Alice");
        assert!(rec.get("old_name").is_none());
    }

    #[test]
    fn test_field_map_rename_missing_field_no_error() {
        let params = json!({ "rename": [{ "from": "nonexistent", "to": "dest" }] });
        let stage = FieldMapStage::from_params(&params).unwrap();
        let record = make_record(&[("name", json!("Alice"))]);
        let item = make_item_with_record(record);
        let ctx = make_context();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(stage.process(item, &ctx)).unwrap();
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec.get("name").unwrap(), "Alice");
        assert!(rec.get("dest").is_none());
    }

    #[test]
    fn test_field_map_drop_fields() {
        let params = json!({ "drop": ["secret", "internal_id"] });
        let stage = FieldMapStage::from_params(&params).unwrap();
        let record = make_record(&[
            ("name", json!("Alice")),
            ("secret", json!("xyz")),
            ("internal_id", json!(42)),
        ]);
        let item = make_item_with_record(record);
        let ctx = make_context();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(stage.process(item, &ctx)).unwrap();
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec.len(), 1);
        assert_eq!(rec.get("name").unwrap(), "Alice");
    }

    #[test]
    fn test_field_map_set_literal_string() {
        let params = json!({ "set": [{ "field": "source", "value": "affinity" }] });
        let stage = FieldMapStage::from_params(&params).unwrap();
        let record = make_record(&[("name", json!("Alice"))]);
        let item = make_item_with_record(record);
        let ctx = make_context();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(stage.process(item, &ctx)).unwrap();
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec.get("source").unwrap(), "affinity");
    }

    #[test]
    fn test_field_map_set_literal_number() {
        let params = json!({ "set": [{ "field": "version", "value": 2 }] });
        let stage = FieldMapStage::from_params(&params).unwrap();
        let record = make_record(&[]);
        let item = make_item_with_record(record);
        let ctx = make_context();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(stage.process(item, &ctx)).unwrap();
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec.get("version").unwrap(), 2);
    }

    #[test]
    fn test_field_map_copy_field() {
        let params = json!({ "copy": [{ "from": "email", "to": "contact_email" }] });
        let stage = FieldMapStage::from_params(&params).unwrap();
        let record = make_record(&[("email", json!("alice@example.com"))]);
        let item = make_item_with_record(record);
        let ctx = make_context();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(stage.process(item, &ctx)).unwrap();
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec.get("email").unwrap(), "alice@example.com");
        assert_eq!(rec.get("contact_email").unwrap(), "alice@example.com");
    }

    #[test]
    fn test_field_map_date_parse_mm_dd_yyyy() {
        let params = json!({
            "parse_dates": [{
                "field": "Transaction_Date",
                "format": "%m/%d/%Y",
                "output": "purchase_ts"
            }]
        });
        let stage = FieldMapStage::from_params(&params).unwrap();
        let record = make_record(&[("Transaction_Date", json!("03/15/2026"))]);
        let item = make_item_with_record(record);
        let ctx = make_context();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(stage.process(item, &ctx)).unwrap();
        let rec = result[0].record.as_ref().unwrap();
        let ts = rec.get("purchase_ts").unwrap().as_str().unwrap();
        assert!(ts.starts_with("2026-03-15"));
    }

    #[test]
    fn test_field_map_date_parse_invalid_returns_null() {
        let params = json!({
            "parse_dates": [{
                "field": "date",
                "format": "%m/%d/%Y",
                "output": "parsed"
            }]
        });
        let stage = FieldMapStage::from_params(&params).unwrap();
        let record = make_record(&[("date", json!("not-a-date"))]);
        let item = make_item_with_record(record);
        let ctx = make_context();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(stage.process(item, &ctx)).unwrap();
        let rec = result[0].record.as_ref().unwrap();
        assert!(rec.get("parsed").unwrap().is_null());
    }

    #[test]
    fn test_field_map_pad_left_six_digits() {
        let params = json!({
            "pad": [{ "field": "auth_code", "width": 6, "pad_char": "0", "side": "left" }]
        });
        let stage = FieldMapStage::from_params(&params).unwrap();
        let record = make_record(&[("auth_code", json!("42"))]);
        let item = make_item_with_record(record);
        let ctx = make_context();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(stage.process(item, &ctx)).unwrap();
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec.get("auth_code").unwrap(), "000042");
    }

    #[test]
    fn test_field_map_pad_already_at_width() {
        let params = json!({
            "pad": [{ "field": "code", "width": 4 }]
        });
        let stage = FieldMapStage::from_params(&params).unwrap();
        let record = make_record(&[("code", json!("1234"))]);
        let item = make_item_with_record(record);
        let ctx = make_context();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(stage.process(item, &ctx)).unwrap();
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec.get("code").unwrap(), "1234");
    }

    #[test]
    fn test_field_map_regex_extract_store_id() {
        let params = json!({
            "regex_extract": [{
                "field": "Merchant_Location_Name",
                "pattern": r"WALGREENS\s*#\s*(\d+)",
                "output": "merchant_store_id",
                "group": 1
            }]
        });
        let stage = FieldMapStage::from_params(&params).unwrap();
        let record = make_record(&[(
            "Merchant_Location_Name",
            json!("WALGREENS #1234 CHICAGO IL"),
        )]);
        let item = make_item_with_record(record);
        let ctx = make_context();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(stage.process(item, &ctx)).unwrap();
        let rec = result[0].record.as_ref().unwrap();
        assert_eq!(rec.get("merchant_store_id").unwrap(), "1234");
    }

    #[test]
    fn test_field_map_regex_no_match_sets_null() {
        let params = json!({
            "regex_extract": [{
                "field": "name",
                "pattern": r"(\d+)",
                "output": "number"
            }]
        });
        let stage = FieldMapStage::from_params(&params).unwrap();
        let record = make_record(&[("name", json!("no digits here"))]);
        let item = make_item_with_record(record);
        let ctx = make_context();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(stage.process(item, &ctx)).unwrap();
        let rec = result[0].record.as_ref().unwrap();
        assert!(rec.get("number").unwrap().is_null());
    }

    #[test]
    fn test_field_map_nest_creates_sub_object() {
        let params = json!({
            "nest": [{
                "output": "payment",
                "fields": {
                    "total_amount": "total_amount",
                    "card_last_four": "card_last_four"
                }
            }]
        });
        let stage = FieldMapStage::from_params(&params).unwrap();
        let record = make_record(&[
            ("total_amount", json!(99.99)),
            ("card_last_four", json!("1234")),
            ("merchant", json!("ACME")),
        ]);
        let item = make_item_with_record(record);
        let ctx = make_context();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(stage.process(item, &ctx)).unwrap();
        let rec = result[0].record.as_ref().unwrap();
        let payment = rec.get("payment").unwrap().as_object().unwrap();
        assert_eq!(payment.get("total_amount").unwrap(), &json!(99.99));
        assert_eq!(payment.get("card_last_four").unwrap(), "1234");
        // Original fields still present
        assert_eq!(rec.get("merchant").unwrap(), "ACME");
    }

    #[test]
    fn test_field_map_no_record_returns_error() {
        let params = json!({ "rename": [{ "from": "a", "to": "b" }] });
        let stage = FieldMapStage::from_params(&params).unwrap();
        let item = PipelineItem {
            id: "no-record".to_string(),
            display_name: "test".to_string(),
            content: Arc::from(b"data" as &[u8]),
            mime_type: "text/plain".to_string(),
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
            stream: None,
        };
        let ctx = make_context();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(stage.process(item, &ctx));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StageError::Permanent { .. }));
    }

    #[test]
    fn test_field_map_combined_affinity_pipeline() {
        let params = json!({
            "rename": [
                { "from": "Transaction_Date", "to": "transaction_date_raw" },
                { "from": "Merchant_Location_Name", "to": "merchant_name" }
            ],
            "copy": [
                { "from": "transaction_date_raw", "to": "date_for_parse" }
            ],
            "set": [
                { "field": "partner_id", "value": 290 },
                { "field": "source_system", "value": "affinity" }
            ],
            "parse_dates": [{
                "field": "date_for_parse",
                "format": "%m/%d/%Y",
                "output": "purchase_ts"
            }],
            "pad": [{
                "field": "Auth_Code",
                "width": 6,
                "pad_char": "0",
                "side": "left"
            }],
            "regex_extract": [{
                "field": "merchant_name",
                "pattern": r"WALGREENS\s*#\s*(\d+)",
                "output": "store_id",
                "group": 1
            }],
            "nest": [{
                "output": "payment",
                "fields": {
                    "amount": "Total_Amount",
                    "auth_code": "Auth_Code"
                }
            }],
            "drop": ["date_for_parse"]
        });
        let stage = FieldMapStage::from_params(&params).unwrap();
        let record = make_record(&[
            ("Transaction_Date", json!("03/15/2026")),
            (
                "Merchant_Location_Name",
                json!("WALGREENS #5678 PITTSBURGH PA"),
            ),
            ("Total_Amount", json!(42.50)),
            ("Auth_Code", json!("123")),
            ("Card_Number", json!("****1234")),
        ]);
        let item = make_item_with_record(record);
        let ctx = make_context();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(stage.process(item, &ctx)).unwrap();
        let rec = result[0].record.as_ref().unwrap();

        // Renames applied
        assert!(rec.get("Transaction_Date").is_none());
        assert_eq!(rec.get("transaction_date_raw").unwrap(), "03/15/2026");

        // Set literals
        assert_eq!(rec.get("partner_id").unwrap(), 290);
        assert_eq!(rec.get("source_system").unwrap(), "affinity");

        // Date parsed
        let ts = rec.get("purchase_ts").unwrap().as_str().unwrap();
        assert!(ts.starts_with("2026-03-15"));

        // Padded
        assert_eq!(rec.get("Auth_Code").unwrap(), "000123");

        // Regex extracted
        assert_eq!(rec.get("store_id").unwrap(), "5678");

        // Nested
        let payment = rec.get("payment").unwrap().as_object().unwrap();
        assert_eq!(payment.get("amount").unwrap(), &json!(42.50));
        assert_eq!(payment.get("auth_code").unwrap(), "000123");

        // Dropped
        assert!(rec.get("date_for_parse").is_none());
    }

    #[test]
    fn test_field_map_from_params_invalid() {
        let params = json!({ "rename": "not_an_array" });
        let result = FieldMapStage::from_params(&params);
        assert!(result.is_err());
    }
}
