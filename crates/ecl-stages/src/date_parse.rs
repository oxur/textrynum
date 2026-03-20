//! Date parse stage: converts date strings to RFC3339 format.
//!
//! Parses date/datetime strings using strftime format patterns and outputs
//! standardized RFC3339 datetime strings. Supports assumed timezone for
//! inputs without timezone information.

use async_trait::async_trait;
use chrono::{NaiveDate, NaiveDateTime, TimeZone};
use chrono_tz::Tz;
use serde::Deserialize;
use serde_json::Value;
use tracing::debug;

use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, Stage, StageContext};

type Record = serde_json::Map<String, serde_json::Value>;

/// Configuration for the date parse stage, parsed from stage params.
#[derive(Debug, Clone, Deserialize)]
pub struct DateParseConfig {
    /// List of date conversions to apply.
    pub conversions: Vec<DateConversion>,
}

/// A single date conversion operation.
#[derive(Debug, Clone, Deserialize)]
pub struct DateConversion {
    /// Source field containing the date string.
    pub field: String,
    /// Output field for the parsed RFC3339 datetime.
    pub output: String,
    /// strftime format string (e.g., "%m/%d/%Y", "%Y-%m-%dT%H:%M:%S").
    pub format: String,
    /// Assumed timezone if the input has no timezone.
    /// IANA timezone name (e.g., "UTC", "US/Eastern", "America/New_York").
    #[serde(default = "default_utc")]
    pub assume_timezone: String,
}

fn default_utc() -> String {
    "UTC".to_string()
}

/// Date parse stage that converts date strings to RFC3339.
#[derive(Debug)]
pub struct DateParseStage {
    config: DateParseConfig,
    /// Pre-parsed timezones for each conversion.
    timezones: Vec<Tz>,
}

impl DateParseStage {
    /// Create a date parse stage from JSON params.
    ///
    /// # Errors
    ///
    /// Returns `StageError::Permanent` if params cannot be deserialized
    /// or if a timezone name is invalid.
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: DateParseConfig =
            serde_json::from_value(params.clone()).map_err(|e| StageError::Permanent {
                stage: "date_parse".into(),
                item_id: String::new(),
                message: format!("invalid date_parse config: {e}"),
            })?;

        let timezones: Result<Vec<Tz>, _> = config
            .conversions
            .iter()
            .map(|conv| {
                conv.assume_timezone
                    .parse::<Tz>()
                    .map_err(|e| StageError::Permanent {
                        stage: "date_parse".into(),
                        item_id: String::new(),
                        message: format!("invalid timezone '{}': {e}", conv.assume_timezone),
                    })
            })
            .collect();

        Ok(Self {
            config,
            timezones: timezones?,
        })
    }

    /// Parse a single date string and return RFC3339 or None.
    fn parse_date(&self, value: &str, conv_index: usize) -> Option<String> {
        let conv = &self.config.conversions[conv_index];
        let tz = self.timezones[conv_index];

        // Try parsing as datetime first.
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(value, &conv.format) {
            if let Some(dt) = tz.from_local_datetime(&naive_dt).earliest() {
                return Some(dt.to_rfc3339());
            }
        }

        // Try parsing as date-only (assume midnight).
        if let Ok(naive_date) = NaiveDate::parse_from_str(value, &conv.format) {
            let naive_dt = naive_date.and_hms_opt(0, 0, 0)?;
            if let Some(dt) = tz.from_local_datetime(&naive_dt).earliest() {
                return Some(dt.to_rfc3339());
            }
        }

        None
    }

    /// Apply all date conversions to a record.
    fn apply_conversions(&self, record: &mut Record) {
        for (i, conv) in self.config.conversions.iter().enumerate() {
            let input_value = record
                .get(&conv.field)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if input_value.is_empty() {
                record.insert(conv.output.clone(), Value::Null);
                continue;
            }

            match self.parse_date(input_value, i) {
                Some(rfc3339) => {
                    record.insert(conv.output.clone(), Value::String(rfc3339));
                }
                None => {
                    record.insert(conv.output.clone(), Value::Null);
                }
            }
        }
    }
}

#[async_trait]
impl Stage for DateParseStage {
    fn name(&self) -> &str {
        "date_parse"
    }

    async fn process(
        &self,
        mut item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let record = item.record.as_mut().ok_or_else(|| StageError::Permanent {
            stage: "date_parse".into(),
            item_id: item.id.clone(),
            message: "item has no record".into(),
        })?;

        debug!(
            item_id = %item.id,
            conversions = self.config.conversions.len(),
            "parsing date fields"
        );

        self.apply_conversions(record);

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
    async fn test_date_parse_mm_dd_yyyy() {
        let params = json!({
            "conversions": [{
                "field": "date_str",
                "output": "date_parsed",
                "format": "%m/%d/%Y"
            }]
        });
        let stage = DateParseStage::from_params(&params).unwrap();
        let mut record = serde_json::Map::new();
        record.insert("date_str".into(), json!("03/15/2024"));
        let item = make_item("i1", record);

        let result = stage.process(item, &ctx()).await.unwrap();
        let rec = result[0].record.as_ref().unwrap();
        let parsed = rec.get("date_parsed").unwrap().as_str().unwrap();
        assert!(parsed.starts_with("2024-03-15T00:00:00"));
    }

    #[tokio::test]
    async fn test_date_parse_iso8601() {
        let params = json!({
            "conversions": [{
                "field": "date_str",
                "output": "date_parsed",
                "format": "%Y-%m-%dT%H:%M:%S"
            }]
        });
        let stage = DateParseStage::from_params(&params).unwrap();
        let mut record = serde_json::Map::new();
        record.insert("date_str".into(), json!("2024-03-15T14:30:00"));
        let item = make_item("i1", record);

        let result = stage.process(item, &ctx()).await.unwrap();
        let rec = result[0].record.as_ref().unwrap();
        let parsed = rec.get("date_parsed").unwrap().as_str().unwrap();
        assert!(parsed.starts_with("2024-03-15T14:30:00"));
    }

    #[tokio::test]
    async fn test_date_parse_iso8601_millis() {
        let params = json!({
            "conversions": [{
                "field": "date_str",
                "output": "date_parsed",
                "format": "%Y-%m-%dT%H:%M:%S%.3f"
            }]
        });
        let stage = DateParseStage::from_params(&params).unwrap();
        let mut record = serde_json::Map::new();
        record.insert("date_str".into(), json!("2024-03-15T14:30:00.123"));
        let item = make_item("i1", record);

        let result = stage.process(item, &ctx()).await.unwrap();
        let rec = result[0].record.as_ref().unwrap();
        let parsed = rec.get("date_parsed").unwrap().as_str().unwrap();
        assert!(parsed.starts_with("2024-03-15T14:30:00"));
    }

    #[tokio::test]
    async fn test_date_parse_invalid_returns_null() {
        let params = json!({
            "conversions": [{
                "field": "date_str",
                "output": "date_parsed",
                "format": "%m/%d/%Y"
            }]
        });
        let stage = DateParseStage::from_params(&params).unwrap();
        let mut record = serde_json::Map::new();
        record.insert("date_str".into(), json!("not-a-date"));
        let item = make_item("i1", record);

        let result = stage.process(item, &ctx()).await.unwrap();
        let rec = result[0].record.as_ref().unwrap();
        assert!(rec.get("date_parsed").unwrap().is_null());
    }

    #[tokio::test]
    async fn test_date_parse_assume_timezone() {
        let params = json!({
            "conversions": [{
                "field": "date_str",
                "output": "date_parsed",
                "format": "%Y-%m-%dT%H:%M:%S",
                "assume_timezone": "US/Eastern"
            }]
        });
        let stage = DateParseStage::from_params(&params).unwrap();
        let mut record = serde_json::Map::new();
        record.insert("date_str".into(), json!("2024-03-15T14:30:00"));
        let item = make_item("i1", record);

        let result = stage.process(item, &ctx()).await.unwrap();
        let rec = result[0].record.as_ref().unwrap();
        let parsed = rec.get("date_parsed").unwrap().as_str().unwrap();
        // US/Eastern is UTC-4 in March (EDT)
        assert!(parsed.contains("14:30:00"));
        // The offset should be -04:00
        assert!(parsed.ends_with("-04:00"));
    }
}
