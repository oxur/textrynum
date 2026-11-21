# 5. Milestone 7.4: Validation Stage

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0026-phase-1-detailed-implementation-plan-make-affinity-work.md, lines 745-907. Original bytes are recoverable from import commit 2d006a7. -->

## 5. Milestone 7.4: Validation Stage

### 5.1 Scope

Configurable record validation with rule-based checking and error classification.

### 5.2 Implementation

#### File: `crates/ecl-stages/src/validate.rs` (new)

**Configuration schema:**

```rust
#[derive(Debug, Clone, Deserialize)]
struct ValidateConfig {
    rules: Vec<ValidationRule>,
}

#[derive(Debug, Clone, Deserialize)]
struct ValidationRule {
    /// Field to validate.
    field: String,
    /// Check type: "required", "regex", "date_range", "length", "numeric_range"
    check: String,
    /// Severity: "hard" (reject record) or "soft" (warn, continue).
    #[serde(default = "default_hard")]
    severity: String,
    // Check-specific params (optional fields):
    /// Regex pattern (for "regex" check).
    #[serde(default)]
    pattern: Option<String>,
    /// Min value (for "date_range", "numeric_range", "length").
    #[serde(default)]
    min: Option<String>,
    /// Max value (for "date_range", "numeric_range", "length").
    #[serde(default)]
    max: Option<String>,
}
```

**Stage implementation:**

```rust
#[derive(Debug)]
pub struct ValidateStage {
    config: ValidateConfig,
    compiled_regexes: Vec<(usize, regex::Regex)>,
}

#[async_trait]
impl Stage for ValidateStage {
    fn name(&self) -> &str { "validate" }

    async fn process(
        &self,
        mut item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let record = item.record.as_ref().ok_or_else(|| StageError::Permanent { /* ... */ })?;

        let mut errors: Vec<serde_json::Value> = Vec::new();
        let mut has_hard_failure = false;

        for rule in &self.config.rules {
            if let Some(error) = self.check_rule(rule, record) {
                if rule.severity == "hard" {
                    has_hard_failure = true;
                }
                errors.push(error);
            }
        }

        // Always attach errors to metadata (even for soft failures)
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

        // Item always passes through — downstream stages decide routing
        Ok(vec![item])
    }
}

impl ValidateStage {
    fn check_rule(
        &self,
        rule: &ValidationRule,
        record: &Record,
    ) -> Option<serde_json::Value> {
        let value = record.get(&rule.field);

        match rule.check.as_str() {
            "required" => {
                // Field must exist and not be null/empty string
                match value {
                    None | Some(Value::Null) => Some(json!({
                        "field": rule.field,
                        "check": "required",
                        "message": format!("field '{}' is required", rule.field),
                    })),
                    Some(Value::String(s)) if s.is_empty() => Some(json!({
                        "field": rule.field,
                        "check": "required",
                        "message": format!("field '{}' is empty", rule.field),
                    })),
                    _ => None,
                }
            }
            "regex" => {
                // Field must match regex pattern
                // ... find compiled regex, test against string value
            }
            "date_range" => {
                // Parse field as RFC3339, check within min..max
                // min/max support relative: "-10y", "+10y"
            }
            "length" => {
                // String length must be within min..max
            }
            "numeric_range" => {
                // Numeric value must be within min..max
            }
            _ => None, // Unknown check type → ignore (forward compatible)
        }
    }
}
```

**Tests:**

1. `test_validate_required_present` — passes
2. `test_validate_required_missing` — fails with error
3. `test_validate_required_null` — fails
4. `test_validate_required_empty_string` — fails
5. `test_validate_regex_matches` — passes
6. `test_validate_regex_no_match` — fails
7. `test_validate_date_range_in_bounds` — passes
8. `test_validate_date_range_too_old` — fails
9. `test_validate_hard_failure_sets_status`
10. `test_validate_soft_failure_sets_warned`
11. `test_validate_no_errors_sets_passed`
12. `test_validate_multiple_rules_all_checked`
13. `test_validate_no_record_returns_error`
14. `test_validate_affinity_rules` — full Affinity validation suite
15. `test_validate_from_params_invalid`

---

