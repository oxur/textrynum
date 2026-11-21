# 3. Milestone 10.2: UPC Check Digit Stage

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0029-phase-4-detailed-implementation-plan-full-parity.md, lines 332-485. Original bytes are recoverable from import commit 3a39aea. -->

## 3. Milestone 10.2: UPC Check Digit Stage

### 3.1 Scope

Specialized stage implementing the GS1 check digit algorithm for retail UPC/EAN codes. Acme Inc. and Giant Eagle both need this.

### 3.2 Why a Dedicated Stage

While this could theoretically be an expression, the GS1 algorithm involves iterating over digits with alternating weights — not naturally expressible in a simple expression language. A dedicated stage is cleaner.

### 3.3 Implementation

#### File: `crates/ecl-stages/src/upc_check_digit.rs` (new)

**Configuration:**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct UpcCheckDigitConfig {
    /// Field containing the UPC/EAN string.
    pub field: String,
    /// Output field for the corrected UPC. Default: overwrites input field.
    #[serde(default)]
    pub output: Option<String>,
    /// Action: "add" (add check digit if missing), "validate" (check and flag errors).
    /// Default: "add"
    #[serde(default = "default_add")]
    pub action: String,
    /// Expected length after check digit added. Default: 13 (EAN-13).
    #[serde(default = "default_13")]
    pub target_length: usize,
}
```

**GS1 check digit algorithm:**

```rust
/// Compute GS1 check digit for a numeric string.
/// Used for UPC-A (12 digits), EAN-13 (13 digits), GTIN-14 (14 digits).
///
/// Algorithm:
/// 1. Number digits from right to left (rightmost = position 1)
/// 2. Sum of digits at odd positions × 3
/// 3. Sum of digits at even positions × 1
/// 4. Check digit = (10 - (total_sum % 10)) % 10
fn gs1_check_digit(digits: &str) -> Option<char> {
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let sum: u32 = digits.chars().rev().enumerate().map(|(i, c)| {
        let digit = c.to_digit(10).unwrap_or(0);
        if i % 2 == 0 {
            digit * 3  // Odd positions (1-indexed from right)
        } else {
            digit      // Even positions
        }
    }).sum();

    let check = (10 - (sum % 10)) % 10;
    char::from_digit(check, 10)
}

#[derive(Debug)]
pub struct UpcCheckDigitStage {
    config: UpcCheckDigitConfig,
}

#[async_trait]
impl Stage for UpcCheckDigitStage {
    fn name(&self) -> &str { "upc_check_digit" }

    async fn process(
        &self,
        mut item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let record = item.record.as_mut()
            .ok_or_else(|| StageError::Permanent { /* ... */ })?;

        let upc_value = record.get(&self.config.field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        if upc_value.is_empty() {
            return Ok(vec![item]);
        }

        let output_field = self.config.output.as_deref()
            .unwrap_or(&self.config.field);

        match self.config.action.as_str() {
            "add" => {
                let target = self.config.target_length;
                if upc_value.len() == target - 1 {
                    // Needs check digit.
                    if let Some(check) = gs1_check_digit(&upc_value) {
                        let corrected = format!("{upc_value}{check}");
                        record.insert(output_field.to_string(), json!(corrected));
                    }
                } else if upc_value.len() < target - 1 {
                    // Pad with leading zeros then add check digit.
                    let padded = format!("{:0>width$}", upc_value, width = target - 1);
                    if let Some(check) = gs1_check_digit(&padded) {
                        let corrected = format!("{padded}{check}");
                        record.insert(output_field.to_string(), json!(corrected));
                    }
                }
                // If already target_length, leave as-is.
            }
            "validate" => {
                if upc_value.len() >= 2 {
                    let payload = &upc_value[..upc_value.len() - 1];
                    let existing_check = upc_value.chars().last().unwrap_or('0');
                    let computed_check = gs1_check_digit(payload);
                    if computed_check != Some(existing_check) {
                        item.metadata.insert(
                            "_upc_check_digit_error".to_string(),
                            json!({
                                "field": self.config.field,
                                "value": upc_value,
                                "expected": computed_check.map(|c| c.to_string()),
                                "actual": existing_check.to_string(),
                            }),
                        );
                    }
                }
            }
            _ => {}
        }

        Ok(vec![item])
    }
}
```

### 3.4 Tests

1. `test_gs1_check_digit_upc_a` — 12-digit UPC → check digit
2. `test_gs1_check_digit_ean_13` — 13-digit EAN validation
3. `test_gs1_check_digit_known_values` — verified against real UPCs
4. `test_upc_stage_add_check_digit_12_to_13`
5. `test_upc_stage_add_check_digit_short_padded`
6. `test_upc_stage_already_13_no_change`
7. `test_upc_stage_validate_correct` — no error metadata
8. `test_upc_stage_validate_incorrect` — error metadata added
9. `test_upc_stage_empty_field_passthrough`
10. `test_upc_stage_non_numeric_passthrough`
11. `test_upc_stage_acme_inc_mixed_upc_ean` — real Acme Inc. scenario

---

