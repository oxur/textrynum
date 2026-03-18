//! Validation stage: rule-based record validation with error classification.
//!
//! Applies configurable validation rules to each `Record`. Each rule specifies
//! a field, a check type, and a severity. Errors are attached as metadata
//! (`_validation_errors`, `_validation_status`) — the item always passes through.
//! Downstream stages (sinks) decide routing based on validation status.

use async_trait::async_trait;
use chrono::DateTime;
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use tracing::debug;

use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, Record, Stage, StageContext};

/// Configuration for the validation stage.
#[derive(Debug, Clone, Deserialize)]
struct ValidateConfig {
    /// Validation rules applied in order.
    rules: Vec<ValidationRule>,
}

/// A single validation rule.
#[derive(Debug, Clone, Deserialize)]
struct ValidationRule {
    /// Field to validate.
    field: String,

    /// Check type: `"required"`, `"regex"`, `"date_range"`, `"length"`, `"numeric_range"`.
    check: String,

    /// Severity: `"hard"` (reject record) or `"soft"` (warn, continue).
    #[serde(default = "default_hard")]
    severity: String,

    /// Regex pattern (for `"regex"` check).
    #[serde(default)]
    pattern: Option<String>,

    /// Minimum value (for `"date_range"`, `"numeric_range"`, `"length"` checks).
    #[serde(default)]
    min: Option<String>,

    /// Maximum value (for `"date_range"`, `"numeric_range"`, `"length"` checks).
    #[serde(default)]
    max: Option<String>,
}

fn default_hard() -> String {
    "hard".to_string()
}

/// Validation stage that checks record fields against configurable rules.
///
/// Items always pass through — validation errors are attached as metadata.
/// Regexes are pre-compiled at construction time for efficiency.
#[derive(Debug)]
pub struct ValidateStage {
    config: ValidateConfig,
    /// Pre-compiled regexes indexed by position in `config.rules`.
    compiled_regexes: Vec<(usize, Regex)>,
}

impl ValidateStage {
    /// Create a new validation stage from stage params JSON.
    ///
    /// # Errors
    ///
    /// Returns `StageError::Permanent` if the params cannot be deserialized
    /// or if a regex pattern is invalid.
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: ValidateConfig =
            serde_json::from_value(params.clone()).map_err(|e| StageError::Permanent {
                stage: "validate".to_string(),
                item_id: String::new(),
                message: format!("invalid validate params: {e}"),
            })?;

        let compiled_regexes = config
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| rule.check == "regex")
            .map(|(i, rule)| {
                let pattern = rule.pattern.as_deref().unwrap_or("");
                Regex::new(pattern)
                    .map(|r| (i, r))
                    .map_err(|e| StageError::Permanent {
                        stage: "validate".to_string(),
                        item_id: String::new(),
                        message: format!("invalid regex in rule[{i}]: {e}"),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            config,
            compiled_regexes,
        })
    }

    /// Check a single rule against a record. Returns an error object if
    /// the check fails, or `None` if it passes.
    fn check_rule(
        &self,
        rule_index: usize,
        rule: &ValidationRule,
        record: &Record,
    ) -> Option<serde_json::Value> {
        let value = record.get(&rule.field);

        match rule.check.as_str() {
            "required" => Self::check_required(rule, value),
            "regex" => self.check_regex(rule_index, rule, value),
            "date_range" => Self::check_date_range(rule, value),
            "length" => Self::check_length(rule, value),
            "numeric_range" => Self::check_numeric_range(rule, value),
            _ => None, // Unknown check type → ignore (forward compatible)
        }
    }

    /// Check that a field is present, not null, and not empty.
    fn check_required(
        rule: &ValidationRule,
        value: Option<&Value>,
    ) -> Option<serde_json::Value> {
        match value {
            None | Some(Value::Null) => Some(serde_json::json!({
                "field": rule.field,
                "check": "required",
                "severity": rule.severity,
                "message": format!("field '{}' is required", rule.field),
            })),
            Some(Value::String(s)) if s.is_empty() => Some(serde_json::json!({
                "field": rule.field,
                "check": "required",
                "severity": rule.severity,
                "message": format!("field '{}' is empty", rule.field),
            })),
            _ => None,
        }
    }

    /// Check that a field matches a regex pattern.
    fn check_regex(
        &self,
        rule_index: usize,
        rule: &ValidationRule,
        value: Option<&Value>,
    ) -> Option<serde_json::Value> {
        let s = value.and_then(|v| v.as_str()).unwrap_or("");

        let re = self
            .compiled_regexes
            .iter()
            .find(|(i, _)| *i == rule_index)
            .map(|(_, r)| r);

        match re {
            Some(r) if r.is_match(s) => None,
            _ => Some(serde_json::json!({
                "field": rule.field,
                "check": "regex",
                "severity": rule.severity,
                "message": format!(
                    "field '{}' does not match pattern '{}'",
                    rule.field,
                    rule.pattern.as_deref().unwrap_or("")
                ),
            })),
        }
    }

    /// Check that a date field is within a range.
    fn check_date_range(
        rule: &ValidationRule,
        value: Option<&Value>,
    ) -> Option<serde_json::Value> {
        let s = match value.and_then(|v| v.as_str()) {
            Some(s) => s,
            None => {
                return Some(serde_json::json!({
                    "field": rule.field,
                    "check": "date_range",
                    "severity": rule.severity,
                    "message": format!("field '{}' is not a date string", rule.field),
                }));
            }
        };

        let dt = match s.parse::<DateTime<chrono::Utc>>() {
            Ok(dt) => dt,
            Err(_) => {
                return Some(serde_json::json!({
                    "field": rule.field,
                    "check": "date_range",
                    "severity": rule.severity,
                    "message": format!("field '{}' is not a valid RFC3339 date", rule.field),
                }));
            }
        };

        if let Some(min_str) = &rule.min {
            if let Ok(min_dt) = min_str.parse::<DateTime<chrono::Utc>>() {
                if dt < min_dt {
                    return Some(serde_json::json!({
                        "field": rule.field,
                        "check": "date_range",
                        "severity": rule.severity,
                        "message": format!(
                            "field '{}' date {} is before minimum {}",
                            rule.field, s, min_str
                        ),
                    }));
                }
            }
        }

        if let Some(max_str) = &rule.max {
            if let Ok(max_dt) = max_str.parse::<DateTime<chrono::Utc>>() {
                if dt > max_dt {
                    return Some(serde_json::json!({
                        "field": rule.field,
                        "check": "date_range",
                        "severity": rule.severity,
                        "message": format!(
                            "field '{}' date {} is after maximum {}",
                            rule.field, s, max_str
                        ),
                    }));
                }
            }
        }

        None
    }

    /// Check that a string field's length is within a range.
    fn check_length(
        rule: &ValidationRule,
        value: Option<&Value>,
    ) -> Option<serde_json::Value> {
        let s = value.and_then(|v| v.as_str()).unwrap_or("");
        let len = s.len();

        if let Some(min_str) = &rule.min {
            if let Ok(min) = min_str.parse::<usize>() {
                if len < min {
                    return Some(serde_json::json!({
                        "field": rule.field,
                        "check": "length",
                        "severity": rule.severity,
                        "message": format!(
                            "field '{}' length {} is below minimum {}",
                            rule.field, len, min
                        ),
                    }));
                }
            }
        }

        if let Some(max_str) = &rule.max {
            if let Ok(max) = max_str.parse::<usize>() {
                if len > max {
                    return Some(serde_json::json!({
                        "field": rule.field,
                        "check": "length",
                        "severity": rule.severity,
                        "message": format!(
                            "field '{}' length {} exceeds maximum {}",
                            rule.field, len, max
                        ),
                    }));
                }
            }
        }

        None
    }

    /// Check that a numeric field is within a range.
    fn check_numeric_range(
        rule: &ValidationRule,
        value: Option<&Value>,
    ) -> Option<serde_json::Value> {
        let num = match value {
            Some(Value::Number(n)) => n.as_f64(),
            Some(Value::String(s)) => s.parse::<f64>().ok(),
            _ => None,
        };

        let num = match num {
            Some(n) => n,
            None => {
                return Some(serde_json::json!({
                    "field": rule.field,
                    "check": "numeric_range",
                    "severity": rule.severity,
                    "message": format!("field '{}' is not numeric", rule.field),
                }));
            }
        };

        if let Some(min_str) = &rule.min {
            if let Ok(min) = min_str.parse::<f64>() {
                if num < min {
                    return Some(serde_json::json!({
                        "field": rule.field,
                        "check": "numeric_range",
                        "severity": rule.severity,
                        "message": format!(
                            "field '{}' value {} is below minimum {}",
                            rule.field, num, min
                        ),
                    }));
                }
            }
        }

        if let Some(max_str) = &rule.max {
            if let Ok(max) = max_str.parse::<f64>() {
                if num > max {
                    return Some(serde_json::json!({
                        "field": rule.field,
                        "check": "numeric_range",
                        "severity": rule.severity,
                        "message": format!(
                            "field '{}' value {} exceeds maximum {}",
                            rule.field, num, max
                        ),
                    }));
                }
            }
        }

        None
    }
}

#[async_trait]
impl Stage for ValidateStage {
    fn name(&self) -> &str {
        "validate"
    }

    async fn process(
        &self,
        mut item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let record = item.record.as_ref().ok_or_else(|| StageError::Permanent {
            stage: "validate".to_string(),
            item_id: item.id.clone(),
            message: "validate requires a record (did you run csv_parse first?)".to_string(),
        })?;

        debug!(item_id = %item.id, rules = self.config.rules.len(), "validating record");

        let mut errors: Vec<serde_json::Value> = Vec::new();
        let mut has_hard_failure = false;

        for (i, rule) in self.config.rules.iter().enumerate() {
            if let Some(error) = self.check_rule(i, rule, record) {
                if rule.severity == "hard" {
                    has_hard_failure = true;
                }
                errors.push(error);
            }
        }

        if !errors.is_empty() {
            item.metadata.insert(
                "_validation_errors".to_string(),
                serde_json::Value::Array(errors),
            );
            item.metadata.insert(
                "_validation_status".to_string(),
                if has_hard_failure {
                    serde_json::json!("failed")
                } else {
                    serde_json::json!("warned")
                },
            );
        } else {
            item.metadata.insert(
                "_validation_status".to_string(),
                serde_json::json!("passed"),
            );
        }

        Ok(vec![item])
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
    use serde_json::json;

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

[stages.validate]
adapter = "validate"
resources = { creates = ["validated"] }
"#,
                )
                .unwrap(),
            ),
            output_dir: PathBuf::from("./out"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        }
    }

    fn run_validate(params: serde_json::Value, record: Record) -> Vec<PipelineItem> {
        let stage = ValidateStage::from_params(&params).unwrap();
        let item = make_item_with_record(record);
        let ctx = make_context();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(stage.process(item, &ctx)).unwrap()
    }

    #[test]
    fn test_validate_required_present() {
        let params = json!({
            "rules": [{ "field": "name", "check": "required" }]
        });
        let record = make_record(&[("name", json!("Alice"))]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "passed");
    }

    #[test]
    fn test_validate_required_missing() {
        let params = json!({
            "rules": [{ "field": "name", "check": "required" }]
        });
        let record = make_record(&[("other", json!("x"))]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "failed");
        let errors = result[0].metadata["_validation_errors"].as_array().unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0]["check"], "required");
    }

    #[test]
    fn test_validate_required_null() {
        let params = json!({
            "rules": [{ "field": "name", "check": "required" }]
        });
        let record = make_record(&[("name", Value::Null)]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "failed");
    }

    #[test]
    fn test_validate_required_empty_string() {
        let params = json!({
            "rules": [{ "field": "name", "check": "required" }]
        });
        let record = make_record(&[("name", json!(""))]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "failed");
        let errors = result[0].metadata["_validation_errors"].as_array().unwrap();
        assert!(errors[0]["message"].as_str().unwrap().contains("empty"));
    }

    #[test]
    fn test_validate_regex_matches() {
        let params = json!({
            "rules": [{ "field": "email", "check": "regex", "pattern": r"^.+@.+\..+$" }]
        });
        let record = make_record(&[("email", json!("alice@example.com"))]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "passed");
    }

    #[test]
    fn test_validate_regex_no_match() {
        let params = json!({
            "rules": [{ "field": "email", "check": "regex", "pattern": r"^.+@.+\..+$" }]
        });
        let record = make_record(&[("email", json!("not-an-email"))]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "failed");
        let errors = result[0].metadata["_validation_errors"].as_array().unwrap();
        assert_eq!(errors[0]["check"], "regex");
    }

    #[test]
    fn test_validate_date_range_in_bounds() {
        let params = json!({
            "rules": [{
                "field": "ts",
                "check": "date_range",
                "min": "2020-01-01T00:00:00Z",
                "max": "2030-01-01T00:00:00Z"
            }]
        });
        let record = make_record(&[("ts", json!("2026-03-15T00:00:00Z"))]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "passed");
    }

    #[test]
    fn test_validate_date_range_too_old() {
        let params = json!({
            "rules": [{
                "field": "ts",
                "check": "date_range",
                "min": "2020-01-01T00:00:00Z"
            }]
        });
        let record = make_record(&[("ts", json!("2019-06-15T00:00:00Z"))]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "failed");
        let errors = result[0].metadata["_validation_errors"].as_array().unwrap();
        assert!(errors[0]["message"].as_str().unwrap().contains("before minimum"));
    }

    #[test]
    fn test_validate_hard_failure_sets_status() {
        let params = json!({
            "rules": [
                { "field": "amount", "check": "required", "severity": "hard" }
            ]
        });
        let record = make_record(&[]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "failed");
    }

    #[test]
    fn test_validate_soft_failure_sets_warned() {
        let params = json!({
            "rules": [
                { "field": "optional_note", "check": "required", "severity": "soft" }
            ]
        });
        let record = make_record(&[]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "warned");
    }

    #[test]
    fn test_validate_no_errors_sets_passed() {
        let params = json!({
            "rules": [
                { "field": "name", "check": "required" }
            ]
        });
        let record = make_record(&[("name", json!("Alice"))]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "passed");
        assert!(result[0].metadata.get("_validation_errors").is_none());
    }

    #[test]
    fn test_validate_multiple_rules_all_checked() {
        let params = json!({
            "rules": [
                { "field": "name", "check": "required" },
                { "field": "email", "check": "regex", "pattern": r"^.+@.+$" },
                { "field": "age", "check": "numeric_range", "min": "0", "max": "200" }
            ]
        });
        let record = make_record(&[
            ("name", json!("Alice")),
            ("email", json!("alice@example.com")),
            ("age", json!(30)),
        ]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "passed");
    }

    #[test]
    fn test_validate_no_record_returns_error() {
        let params = json!({
            "rules": [{ "field": "name", "check": "required" }]
        });
        let stage = ValidateStage::from_params(&params).unwrap();
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
    fn test_validate_affinity_rules() {
        let params = json!({
            "rules": [
                { "field": "transaction_date_raw", "check": "required" },
                { "field": "purchase_ts", "check": "date_range",
                  "min": "2020-01-01T00:00:00Z", "max": "2030-01-01T00:00:00Z" },
                { "field": "merchant_name", "check": "required" },
                { "field": "Auth_Code", "check": "length", "min": "6", "max": "6" },
                { "field": "Total_Amount", "check": "numeric_range", "min": "0" },
                { "field": "Card_Number", "check": "regex", "pattern": r"^\*{4}\d{4}$" },
                { "field": "partner_id", "check": "required" }
            ]
        });
        let record = make_record(&[
            ("transaction_date_raw", json!("03/15/2026")),
            ("purchase_ts", json!("2026-03-15T00:00:00+00:00")),
            ("merchant_name", json!("WALGREENS #5678 PITTSBURGH PA")),
            ("Auth_Code", json!("000123")),
            ("Total_Amount", json!(42.50)),
            ("Card_Number", json!("****1234")),
            ("partner_id", json!(290)),
        ]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "passed");
    }

    #[test]
    fn test_validate_from_params_invalid() {
        let params = json!({ "rules": "not_an_array" });
        let result = ValidateStage::from_params(&params);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_length_too_short() {
        let params = json!({
            "rules": [{ "field": "code", "check": "length", "min": "6" }]
        });
        let record = make_record(&[("code", json!("abc"))]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "failed");
        let errors = result[0].metadata["_validation_errors"].as_array().unwrap();
        assert!(errors[0]["message"].as_str().unwrap().contains("below minimum"));
    }

    #[test]
    fn test_validate_length_too_long() {
        let params = json!({
            "rules": [{ "field": "code", "check": "length", "max": "3" }]
        });
        let record = make_record(&[("code", json!("abcdef"))]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "failed");
    }

    #[test]
    fn test_validate_numeric_range_below() {
        let params = json!({
            "rules": [{ "field": "amount", "check": "numeric_range", "min": "0" }]
        });
        let record = make_record(&[("amount", json!(-5.0))]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "failed");
    }

    #[test]
    fn test_validate_numeric_range_above() {
        let params = json!({
            "rules": [{ "field": "amount", "check": "numeric_range", "max": "10000" }]
        });
        let record = make_record(&[("amount", json!(99999))]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "failed");
    }

    #[test]
    fn test_validate_numeric_range_string_value() {
        let params = json!({
            "rules": [{ "field": "amount", "check": "numeric_range", "min": "0", "max": "100" }]
        });
        let record = make_record(&[("amount", json!("42.5"))]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "passed");
    }

    #[test]
    fn test_validate_numeric_range_not_numeric() {
        let params = json!({
            "rules": [{ "field": "amount", "check": "numeric_range", "min": "0" }]
        });
        let record = make_record(&[("amount", json!("abc"))]);
        let result = run_validate(params, record);
        assert_eq!(result[0].metadata["_validation_status"], "failed");
        let errors = result[0].metadata["_validation_errors"].as_array().unwrap();
        assert!(errors[0]["message"].as_str().unwrap().contains("not numeric"));
    }
}
