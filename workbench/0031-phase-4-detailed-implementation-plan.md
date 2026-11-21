# Phase 4 Detailed Implementation Plan: "Full Parity"

**Date:** 2026-03-17
**Status:** Draft
**Depends on:** 0028 (Phase 1), 0029 (Phase 2), 0030 (Phase 3)
**Goal:** Complete feature parity with all four Banyan customer pipelines — BigQuery source for Chime, expression evaluation for computed fields, UPC/auth code specialized stages, environment config overlays, stateful processing for Acme Inc. loyalty, and full E2E tests for Chime and loyalty.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Milestone 10.1: Expression Evaluation Stage](#2-milestone-101-expression-evaluation-stage)
3. [Milestone 10.2: UPC Check Digit Stage](#3-milestone-102-upc-check-digit-stage)
4. [Milestone 10.3: Auth Code Normalization Stage](#4-milestone-103-auth-code-normalization-stage)
5. [Milestone 10.4: BigQuery Source Adapter](#5-milestone-104-bigquery-source-adapter)
6. [Milestone 10.5: Environment Config Overlays](#6-milestone-105-environment-config-overlays)
7. [Milestone 10.6: Stateful Processing (Loyalty)](#7-milestone-106-stateful-processing-loyalty)
8. [Milestone 10.7: Chime End-to-End Integration Test](#8-milestone-107-chime-end-to-end-integration-test)
9. [Milestone 10.8: Acme Inc. Loyalty End-to-End Integration Test](#9-milestone-108-acme-inc-loyalty-end-to-end-integration-test)
10. [Cross-Cutting Concerns](#10-cross-cutting-concerns)
11. [Verification Checklist](#11-verification-checklist)

---

## 1. Architecture Overview

### 1.1 What Remains

After Phases 1–3, three gaps remain:

| Gap | Customer | Why it's needed |
|-----|----------|----------------|
| BigQuery as a data source | Chime | Rewards calculation queries BQ tables, not files |
| Expression evaluation | Acme Inc. | Digital price correction (`amount / 100`), conditional BOPIS logic |
| UPC check digit | Acme Inc., Giant Eagle | GS1 check digit algorithm for retail UPCs |
| Auth code normalization | Acme Inc., Affinity, Giant Eagle | Strip leading zeros, pad to N digits |
| Environment overlays | All | One base spec, three deployments (sand/test/prod) |
| Stateful processing | Acme Inc. loyalty | Database-backed diff/update for consumer tokens |

### 1.2 Chime Pipeline Shape

Chime is fundamentally different from the other three — it's SQL-driven, not file-streaming:

```
BIN files (GCS) ─→ csv_parse ─→ collect BINs ─────┐
                                                    │
BigQuery Source ─→ query(receipts × items × BINs) ──┤
                                                    │
                              rewards_calculation ←─┘
                                     │
                                     ├─→ csv_emit (GCS output)
                                     └─→ s3_sink (AWS output)
```

### 1.3 Acme Inc. Loyalty Pipeline Shape

```
GCS (loyalty files) ─→ decrypt(PGP) ─→ csv_parse ─→ field_map (hash phones)
                                                          │
                                           stateful_diff (PostgreSQL)
                                                          │
                                              ┌───────────┴───────────┐
                                              │                       │
                                       inserts/updates            deletes
                                              │                       │
                                              └───────────┬───────────┘
                                                          │
                                                   kafka_sink (Avro)
```

---

## 2. Milestone 10.1: Expression Evaluation Stage

### 2.1 Scope

A stage that evaluates expressions on record fields — arithmetic, conditionals, string operations. This replaces the need for custom per-customer Rust code for computed fields.

### 2.2 Why This Is Needed

| Use Case | Expression |
|----------|-----------|
| Acme Inc. digital price correction | `amount / 100` |
| BOPIS detection | `if bopisFlag == "Y" then "web" else order_type` |
| Chime reward calculation | `min(private_label * 0.05 + non_private * 0.03, 5.0)` |
| Cash back calculation | `if payment_type == "Debit Card/Cashback" and amount < 0 then abs(amount) else 0` |
| Net tender calculation | `total_amount - cash_back_amount` |

### 2.3 Expression Language Design

Use a simple, safe expression language. **Not** a general-purpose scripting language — no loops, no I/O, no function definitions. Just field references, literals, operators, and built-in functions.

**Syntax (inspired by jq/jsonpath with SQL-like conditionals):**

```
# Field reference
.field_name

# Literals
42, 3.14, "hello", true, null

# Arithmetic
.amount / 100
.price * .quantity
.total - .discount

# Comparison
.field == "value"
.amount > 0
.count >= 10

# Logical
.a && .b
.a || .b
!.flag

# Conditionals
if .bopisFlag == "Y" then "web" else .order_type

# Built-in functions
min(.reward, 5.0)
max(.a, .b)
abs(.amount)
round(.value, 2)
len(.name)
lower(.text)
upper(.text)
trim(.text)
concat(.first, " ", .last)
coalesce(.preferred, .fallback, "default")
```

### 2.4 Implementation

#### File: `crates/ecl-stages/src/expression.rs` (new)

**Configuration:**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ExpressionConfig {
    /// List of expressions to evaluate.
    pub expressions: Vec<ExpressionOp>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExpressionOp {
    /// Output field name for the result.
    pub output: String,
    /// Expression string to evaluate.
    pub expr: String,
}
```

**Expression evaluator:**

Rather than building a custom parser, use the `evalexpr` crate which provides a safe expression evaluator with no I/O or side effects:

```toml
# In ecl-stages/Cargo.toml:
evalexpr = "14"
```

`evalexpr` supports: arithmetic, comparisons, logical operators, string operations, and custom functions. It operates on a `HashMapContext` of variable bindings.

```rust
use evalexpr::*;

#[derive(Debug)]
pub struct ExpressionStage {
    config: ExpressionConfig,
    /// Pre-compiled expressions.
    compiled: Vec<(String, Node)>,  // (output_field, compiled_expr)
}

impl ExpressionStage {
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: ExpressionConfig = serde_json::from_value(params.clone())
            .map_err(|e| StageError::Permanent { /* ... */ })?;

        let compiled = config.expressions.iter()
            .map(|op| {
                let node = build_operator_tree(&op.expr)
                    .map_err(|e| StageError::Permanent {
                        stage: "expression".to_string(),
                        item_id: String::new(),
                        message: format!("invalid expression '{}': {e}", op.expr),
                    })?;
                Ok((op.output.clone(), node))
            })
            .collect::<Result<Vec<_>, StageError>>()?;

        Ok(Self { config, compiled })
    }
}

#[async_trait]
impl Stage for ExpressionStage {
    fn name(&self) -> &str { "expression" }

    async fn process(
        &self,
        mut item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let record = item.record.as_mut()
            .ok_or_else(|| StageError::Permanent { /* ... */ })?;

        // Build evalexpr context from record fields.
        let mut context = HashMapContext::new();
        for (key, value) in record.iter() {
            match value {
                Value::String(s) => { context.set_value(key.clone(), evalexpr::Value::String(s.clone())).ok(); }
                Value::Number(n) => {
                    if let Some(f) = n.as_f64() {
                        context.set_value(key.clone(), evalexpr::Value::Float(f)).ok();
                    } else if let Some(i) = n.as_i64() {
                        context.set_value(key.clone(), evalexpr::Value::Int(i)).ok();
                    }
                }
                Value::Bool(b) => { context.set_value(key.clone(), evalexpr::Value::Boolean(*b)).ok(); }
                Value::Null => { context.set_value(key.clone(), evalexpr::Value::Empty).ok(); }
                _ => {} // Skip arrays/objects
            }
        }

        // Register custom functions.
        context.set_function("coalesce".to_string(), Function::new(|args| {
            for arg in args.as_tuple()? {
                if arg != &evalexpr::Value::Empty {
                    return Ok(arg.clone());
                }
            }
            Ok(evalexpr::Value::Empty)
        })).ok();

        // Evaluate each expression.
        for (output, compiled_expr) in &self.compiled {
            let result = compiled_expr.eval_with_context(&context)
                .unwrap_or(evalexpr::Value::Empty);

            let json_value = evalexpr_to_json(&result);
            record.insert(output.clone(), json_value);

            // Also update context so later expressions can reference earlier results.
            context.set_value(output.clone(), result).ok();
        }

        Ok(vec![item])
    }
}

fn evalexpr_to_json(val: &evalexpr::Value) -> serde_json::Value {
    match val {
        evalexpr::Value::String(s) => serde_json::Value::String(s.clone()),
        evalexpr::Value::Float(f) => serde_json::json!(f),
        evalexpr::Value::Int(i) => serde_json::json!(i),
        evalexpr::Value::Boolean(b) => serde_json::Value::Bool(*b),
        evalexpr::Value::Empty => serde_json::Value::Null,
        evalexpr::Value::Tuple(t) => {
            serde_json::Value::Array(t.iter().map(evalexpr_to_json).collect())
        }
    }
}
```

**TOML spec example (Acme Inc. digital price correction):**

```toml
[[stages]]
name = "price_correct_digital"
handler = "expression"
input_streams = ["digital-transactions"]
output_stream = "corrected-transactions"
[stages.price_correct_digital.params]
expressions = [
  { output = "merchant_total_amount", expr = "merchant_total_amount / 100" },
  { output = "shipping_amount", expr = "shipping_amount / 100" },
  { output = "tax_amount", expr = "tax_amount / 100" },
  { output = "subtotal_amount", expr = "subtotal_amount / 100" },
  { output = "charged_amount", expr = "charged_amount / 100" },
]
```

**TOML spec example (BOPIS detection):**

```toml
[[stages]]
name = "bopis_detect"
handler = "expression"
[stages.bopis_detect.params]
expressions = [
  { output = "order_type", expr = "if(bopisFlag == \"Y\", \"web\", order_type)" },
  { output = "fulfillment_type", expr = "if(bopisFlag == \"Y\", \"in_store\", fulfillment_type)" },
]
```

**TOML spec example (Chime rewards):**

```toml
[[stages]]
name = "calc_rewards"
handler = "expression"
[stages.calc_rewards.params]
expressions = [
  { output = "private_label_reward", expr = "private_label_amt * 0.05" },
  { output = "non_private_reward", expr = "non_private_label_amt * 0.03" },
  { output = "raw_reward", expr = "private_label_reward + non_private_reward" },
  { output = "total_reward_amount", expr = "min(raw_reward, 5.0)" },
]
```

### 2.5 Tests

1. `test_expr_arithmetic_basic` — `a + b`
2. `test_expr_division` — `amount / 100`
3. `test_expr_conditional` — `if(flag == "Y", "web", "in_store")`
4. `test_expr_min_max` — `min(a, 5.0)`
5. `test_expr_abs` — `abs(negative_amount)`
6. `test_expr_string_functions` — `lower(name)`, `upper(name)`, `trim(name)`
7. `test_expr_coalesce` — first non-null value
8. `test_expr_chained_references` — later expression references earlier result
9. `test_expr_missing_field_evaluates_empty`
10. `test_expr_acme_inc_price_correction` — full digital price fix
11. `test_expr_chime_reward_calculation`
12. `test_expr_compile_error` — invalid expression → Permanent error
13. `test_expr_from_params_invalid`

---

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

## 4. Milestone 10.3: Auth Code Normalization Stage

### 4.1 Scope

Specialized stage for authorization code cleaning — strip leading zeros, left-pad to N digits. Used by Acme Inc. (Medagate codes), Affinity, and Giant Eagle.

### 4.2 Implementation

#### File: `crates/ecl-stages/src/auth_code_norm.rs` (new)

This is simple enough that it could be a `field_map` `pad` operation, but the Acme Inc. auth code has specific logic: strip leading zeros first, THEN left-pad to 6. The `pad` operation doesn't strip first.

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct AuthCodeNormConfig {
    /// Field containing the auth code.
    pub field: String,
    /// Output field. Default: overwrites input.
    #[serde(default)]
    pub output: Option<String>,
    /// Target width for padding. Default: 6.
    #[serde(default = "default_six")]
    pub width: usize,
    /// Whether to strip leading zeros before padding. Default: true.
    #[serde(default = "default_true")]
    pub strip_leading_zeros: bool,
    /// Pad character. Default: '0'.
    #[serde(default = "default_zero")]
    pub pad_char: char,
}

#[derive(Debug)]
pub struct AuthCodeNormStage {
    config: AuthCodeNormConfig,
}

#[async_trait]
impl Stage for AuthCodeNormStage {
    fn name(&self) -> &str { "auth_code_norm" }

    async fn process(
        &self,
        mut item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let record = item.record.as_mut()
            .ok_or_else(|| StageError::Permanent { /* ... */ })?;

        let raw = record.get(&self.config.field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let output_field = self.config.output.as_deref()
            .unwrap_or(&self.config.field);

        if raw.is_empty() {
            return Ok(vec![item]);
        }

        let mut normalized = raw.clone();

        // Strip leading zeros (e.g., "00123" → "123").
        if self.config.strip_leading_zeros {
            normalized = normalized.trim_start_matches('0').to_string();
            if normalized.is_empty() {
                normalized = "0".to_string();
            }
        }

        // Left-pad to target width.
        if normalized.len() < self.config.width {
            let pad = self.config.pad_char.to_string()
                .repeat(self.config.width - normalized.len());
            normalized = format!("{pad}{normalized}");
        }

        record.insert(output_field.to_string(), json!(normalized));

        Ok(vec![item])
    }
}
```

### 4.3 Tests

1. `test_auth_code_strip_and_pad` — "00123" → "000123" (strip to "123", pad to 6)
2. `test_auth_code_already_correct_length` — "123456" → "123456"
3. `test_auth_code_longer_than_width` — "1234567" → "1234567" (no truncation)
4. `test_auth_code_all_zeros` — "000000" → "000000" (strip to "0", pad to 6)
5. `test_auth_code_no_strip` — strip_leading_zeros=false, "00123" → "00123"
6. `test_auth_code_empty_passthrough`
7. `test_auth_code_acme_inc_medagate` — real Medagate codes

---

## 5. Milestone 10.4: BigQuery Source Adapter

### 5.1 Scope

New crate `ecl-adapter-bigquery` implementing `SourceAdapter` for BigQuery query results. Unlike file-based sources, this adapter runs a SQL query and produces one `PipelineItem` per result row.

### 5.2 Crate Structure

```
crates/ecl-adapter-bigquery/
├── Cargo.toml
└── src/
    └── lib.rs
```

### 5.3 Cargo.toml

```toml
[package]
name = "ecl-adapter-bigquery"
version.workspace = true
edition.workspace = true

[dependencies]
ecl-pipeline-topo = { path = "../ecl-pipeline-topo" }
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-state = { path = "../ecl-pipeline-state" }

gcp-bigquery-client = "0.25"

tokio = { workspace = true }
async-trait = { workspace = true }
blake3 = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }
chrono = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
```

### 5.4 SourceSpec Extension

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SourceSpec {
    // ... existing ...
    #[serde(rename = "bigquery")]
    BigQuery(BigQuerySourceSpec),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BigQuerySourceSpec {
    /// GCP project ID.
    pub project: String,
    /// SQL query to execute. Supports ${PARAM} interpolation.
    pub query: String,
    /// Query parameters (substituted into query).
    #[serde(default)]
    pub parameters: BTreeMap<String, serde_json::Value>,
    /// Credentials for authentication.
    #[serde(default = "default_adc")]
    pub credentials: CredentialRef,
    /// Named data stream.
    #[serde(default)]
    pub stream: Option<String>,
}
```

### 5.5 Adapter Implementation

BigQuery is different from file sources — there's no separate enumerate/fetch cycle. The query returns all data at once. We model this as:

- **`enumerate()`**: Execute the query, return one `SourceItem` per row (lightweight: just row index + hash)
- **`fetch()`**: Return the row data that was already fetched during enumerate (cached)

Alternative: enumerate returns a single "query result" SourceItem, and the CSV parse stage (or a new `bq_parse` stage) fans out into rows.

**Recommended: Single-item model.** Enumerate returns one `SourceItem` representing the query result. Fetch returns all rows as JSON lines. Then `csv_parse` or a new `json_lines_parse` stage fans out into records. This reuses existing infrastructure.

```rust
#[derive(Debug)]
pub struct BigQueryAdapter {
    project: String,
    query: String,
    parameters: BTreeMap<String, serde_json::Value>,
    stream: Option<String>,
    /// Cached query result (populated during enumerate).
    cached_result: tokio::sync::Mutex<Option<Vec<u8>>>,
}

#[async_trait]
impl SourceAdapter for BigQueryAdapter {
    fn source_kind(&self) -> &str { "bigquery" }

    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
        // 1. Interpolate parameters into query.
        // 2. Execute query via BigQuery API.
        // 3. Serialize result rows as JSON lines.
        // 4. Cache in self.cached_result.
        // 5. Return single SourceItem representing the query result.

        let client = gcp_bigquery_client::Client::from_application_default_credentials().await
            .map_err(|e| SourceError::AuthError { /* ... */ })?;

        let query = self.interpolate_query();
        let result = client.job().query(&self.project, query).await
            .map_err(|e| SourceError::Transient { /* ... */ })?;

        // Convert rows to JSON lines.
        let mut content = Vec::new();
        for row in &result.rows {
            let json_row = row_to_json(row, &result.schema);
            serde_json::to_writer(&mut content, &json_row)
                .map_err(|e| SourceError::Permanent { /* ... */ })?;
            content.push(b'\n');
        }

        let hash = blake3::hash(&content);
        *self.cached_result.lock().await = Some(content);

        Ok(vec![SourceItem {
            id: format!("bq:{}:{}", self.project, hash.to_hex()[..8].to_string()),
            display_name: format!("BigQuery query result ({} rows)", result.rows.len()),
            mime_type: "application/x-ndjson".to_string(),
            path: "query_result.jsonl".to_string(),
            modified_at: Some(Utc::now()),
            source_hash: Some(hash.to_hex().to_string()),
        }])
    }

    async fn fetch(&self, _item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
        let content = self.cached_result.lock().await.take()
            .ok_or_else(|| SourceError::Permanent {
                source_name: "bigquery".to_string(),
                message: "no cached query result (call enumerate first)".to_string(),
            })?;

        let content_hash = Blake3Hash::new(&blake3::hash(&content).to_hex().to_string());

        Ok(ExtractedDocument {
            id: _item.id.clone(),
            display_name: _item.display_name.clone(),
            content,
            mime_type: "application/x-ndjson".to_string(),
            provenance: ItemProvenance {
                source_kind: "bigquery".to_string(),
                metadata: BTreeMap::from([
                    ("project".to_string(), json!(self.project)),
                ]),
                source_modified: Some(Utc::now()),
                extracted_at: Utc::now(),
            },
            content_hash,
        })
    }
}
```

### 5.6 JSON Lines Parse Stage

Since BigQuery results come as JSON lines (not CSV), we need a complementary parse stage:

#### File: `crates/ecl-stages/src/json_lines_parse.rs` (new)

```rust
#[derive(Debug)]
pub struct JsonLinesParseStage;

#[async_trait]
impl Stage for JsonLinesParseStage {
    fn name(&self) -> &str { "json_lines_parse" }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let content = std::str::from_utf8(item.content.as_ref())
            .map_err(|e| StageError::Permanent { /* ... */ })?;

        let mut results = Vec::new();
        for (i, line) in content.lines().enumerate() {
            if line.trim().is_empty() { continue; }
            let record: Record = serde_json::from_str(line)
                .map_err(|e| StageError::Permanent {
                    stage: "json_lines_parse".to_string(),
                    item_id: item.id.clone(),
                    message: format!("invalid JSON on line {}: {e}", i + 1),
                })?;
            results.push(PipelineItem {
                id: format!("{}:row:{}", item.id, i),
                display_name: format!("{}:{}", item.display_name, i),
                content: Arc::from(line.as_bytes()),
                record: Some(record),
                ..item.clone()
            });
        }
        Ok(results)
    }
}
```

### 5.7 Tests

1. `test_bq_spec_serde_roundtrip`
2. `test_bq_query_interpolation` — parameter substitution
3. `test_json_lines_parse_basic` — 3 JSON lines → 3 items
4. `test_json_lines_parse_empty_lines_skipped`
5. `test_json_lines_parse_invalid_json_error`
6. `test_json_lines_parse_fan_out_ids`

---

## 6. Milestone 10.5: Environment Config Overlays

### 6.1 Scope

Support running one base pipeline spec across multiple environments (sandbox, testing, production) with per-environment overrides for bucket names, topics, credentials, etc.

### 6.2 Design

**Approach: TOML inheritance with override files.**

```
pipelines/
├── acme-inc-327.toml         # Base spec
├── acme-inc-327.sand.toml    # Sandbox overrides
├── acme-inc-327.test.toml    # Testing overrides
└── acme-inc-327.prod.toml    # Production overrides
```

The CLI command:
```bash
ecl pipeline run acme-inc-327.toml --env prod
```

Loads the base spec, then deep-merges the environment override file on top.

### 6.3 Override File Format

Override files contain only the fields that differ:

```toml
# acme-inc-327.prod.toml
[sources.encrypted-files.config]
bucket = "byn-prod-merchant-327-acme-inc"

[stages.kafka-receipts.params]
topic = "by_production_327_acme_inc_receipt-preprocess_canonical_batch_avro_0"
bootstrap_servers = "${KAFKA_BROKERS_PROD}"

[secrets]
provider = "gcp_secret_manager"
project = "pipeline-production-deploy-env"
```

### 6.4 Implementation

#### File: `crates/ecl-pipeline-spec/src/overlay.rs` (new)

```rust
use serde_json::Value;

/// Deep-merge two JSON values. `overlay` values take precedence.
/// Objects are recursively merged. Non-object values are replaced entirely.
pub fn deep_merge(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, overlay_val) in overlay_map {
                let base_val = base_map.entry(key.clone()).or_insert(Value::Null);
                deep_merge(base_val, overlay_val);
            }
        }
        (base, overlay) => {
            *base = overlay.clone();
        }
    }
}

/// Load a PipelineSpec with optional environment overlay.
pub fn load_with_overlay(
    base_toml: &str,
    overlay_toml: Option<&str>,
) -> Result<PipelineSpec> {
    let mut base: Value = toml::from_str(base_toml)?;

    if let Some(overlay_str) = overlay_toml {
        let overlay: Value = toml::from_str(overlay_str)?;
        deep_merge(&mut base, &overlay);
    }

    let spec: PipelineSpec = serde_json::from_value(base)?;
    spec.validate()?;
    Ok(spec)
}
```

#### CLI integration:

```rust
// In run.rs:
pub async fn execute(config_path: PathBuf, env: Option<String>) -> Result<()> {
    let base_toml = tokio::fs::read_to_string(&config_path).await?;

    let overlay_toml = if let Some(env_name) = &env {
        let overlay_path = config_path.with_extension(format!("{env_name}.toml"));
        if overlay_path.exists() {
            Some(tokio::fs::read_to_string(&overlay_path).await?)
        } else {
            None
        }
    } else {
        None
    };

    let spec = overlay::load_with_overlay(&base_toml, overlay_toml.as_deref())?;
    // ... rest of run logic ...
}
```

### 6.5 Tests

1. `test_deep_merge_simple_override` — string value replaced
2. `test_deep_merge_nested_object` — only changed fields replaced
3. `test_deep_merge_add_new_field` — overlay adds field not in base
4. `test_deep_merge_array_replaced` — arrays replaced entirely (not merged)
5. `test_load_with_overlay_prod`
6. `test_load_with_overlay_none` — no overlay = base spec
7. `test_load_with_overlay_invalid_toml`

---

## 7. Milestone 10.6: Stateful Processing (Loyalty)

### 7.1 Scope

Support Acme Inc. loyalty pipeline: database-backed state for diffing consumer tokens across runs. This is the most complex Phase 4 milestone.

### 7.2 Design

The loyalty pipeline needs to:
1. Load loyalty records from CSV
2. Compare against previously-known state (in a database)
3. Compute diffs: new, changed, deleted, unchanged
4. Emit appropriate Kafka messages (insert, update, delete)
5. Update the database state

**New concept: `StatefulStage`** — a batch stage with access to a persistent key-value store that survives across pipeline runs.

### 7.3 State Store Extension

#### File: `crates/ecl-pipeline-state/src/store.rs`

Add a key-value interface to the `StateStore` trait:

```rust
#[async_trait]
pub trait StateStore: Send + Sync {
    // ... existing checkpoint/hash methods ...

    /// Get a value from the named state table.
    async fn state_get(&self, table: &str, key: &str) -> Result<Option<Vec<u8>>, StateError>;

    /// Set a value in the named state table.
    async fn state_set(&self, table: &str, key: &str, value: &[u8]) -> Result<(), StateError>;

    /// Delete a key from the named state table.
    async fn state_delete(&self, table: &str, key: &str) -> Result<(), StateError>;

    /// Iterate all keys in a named state table.
    async fn state_keys(&self, table: &str) -> Result<Vec<String>, StateError>;

    /// Batch set multiple keys atomically.
    async fn state_batch_set(
        &self,
        table: &str,
        entries: &BTreeMap<String, Vec<u8>>,
    ) -> Result<(), StateError>;
}
```

The `RedbStateStore` implements these using additional redb tables (one per named state table).

### 7.4 Stateful Diff Stage

#### File: `crates/ecl-stages/src/stateful_diff.rs` (new)

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct StatefulDiffConfig {
    /// State table name (persisted across runs).
    pub table: String,
    /// Key field in the record (e.g., "loyalty_id_sha").
    pub key_field: String,
    /// Value field to compare (e.g., "computed_cat").
    pub value_field: String,
    /// Output field indicating the diff action: "insert", "update", "delete", "unchanged".
    #[serde(default = "default_diff_action")]
    pub action_field: String,
    /// Field to store the previous value for updates.
    #[serde(default)]
    pub previous_value_field: Option<String>,
}

#[derive(Debug)]
pub struct StatefulDiffStage {
    config: StatefulDiffConfig,
    store: Arc<dyn StateStore>,
}

#[async_trait]
impl Stage for StatefulDiffStage {
    fn name(&self) -> &str { "stateful_diff" }
    fn requires_batch(&self) -> bool { true }

    async fn process_batch(
        &self,
        items: Vec<PipelineItem>,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // 1. Load all existing keys from state table.
        let existing_keys = self.store.state_keys(&self.config.table).await
            .map_err(|e| StageError::Permanent { /* ... */ })?;
        let existing_set: HashSet<String> = existing_keys.into_iter().collect();

        // 2. Process each incoming record.
        let mut results = Vec::new();
        let mut seen_keys = HashSet::new();
        let mut updates: BTreeMap<String, Vec<u8>> = BTreeMap::new();

        for mut item in items {
            let record = item.record.as_mut()
                .ok_or_else(|| StageError::Permanent { /* ... */ })?;

            let key = record.get(&self.config.key_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let new_value = record.get(&self.config.value_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            seen_keys.insert(key.clone());

            // Look up existing value.
            let existing_value = self.store.state_get(&self.config.table, &key).await
                .map_err(|e| StageError::Permanent { /* ... */ })?
                .map(|bytes| String::from_utf8_lossy(&bytes).to_string());

            let action = match (&existing_value, new_value.is_empty()) {
                (None, false) => "insert",
                (None, true) => {
                    // No existing, no new → skip
                    continue;
                }
                (Some(_), true) => "delete",
                (Some(existing), false) if existing == &new_value => {
                    // Unchanged → skip
                    continue;
                }
                (Some(_), false) => "update",
            };

            record.insert(self.config.action_field.clone(), json!(action));
            if let Some(ref prev_field) = self.config.previous_value_field {
                record.insert(prev_field.clone(), json!(existing_value));
            }

            // Update state store.
            if action == "delete" {
                // Will delete after processing.
            } else {
                updates.insert(key.clone(), new_value.into_bytes());
            }

            results.push(item);
        }

        // 3. Find keys that exist in state but not in current batch → deletes.
        for existing_key in &existing_set {
            if !seen_keys.contains(existing_key) {
                // Key disappeared → generate delete record.
                let existing_value = self.store.state_get(&self.config.table, existing_key).await
                    .map_err(|e| StageError::Permanent { /* ... */ })?;

                let mut record = Record::new();
                record.insert(self.config.key_field.clone(), json!(existing_key));
                record.insert(self.config.action_field.clone(), json!("delete"));
                if let Some(ref prev_field) = self.config.previous_value_field {
                    record.insert(prev_field.clone(), json!(
                        existing_value.map(|b| String::from_utf8_lossy(&b).to_string())
                    ));
                }

                results.push(PipelineItem {
                    id: format!("diff:delete:{existing_key}"),
                    display_name: format!("Delete {existing_key}"),
                    record: Some(record),
                    content: Arc::from(Vec::new().as_slice()),
                    // ... other fields
                });
            }
        }

        // 4. Persist state updates atomically.
        self.store.state_batch_set(&self.config.table, &updates).await
            .map_err(|e| StageError::Permanent { /* ... */ })?;

        // 5. Delete removed keys.
        for key in &existing_set {
            if !seen_keys.contains(key) {
                self.store.state_delete(&self.config.table, key).await
                    .map_err(|e| StageError::Permanent { /* ... */ })?;
            }
        }

        Ok(results)
    }
}
```

### 7.5 Tests

1. `test_stateful_diff_first_run_all_inserts`
2. `test_stateful_diff_no_changes_empty_output`
3. `test_stateful_diff_value_changed_update`
4. `test_stateful_diff_key_disappeared_delete`
5. `test_stateful_diff_new_and_changed_mixed`
6. `test_stateful_diff_empty_value_triggers_delete`
7. `test_stateful_diff_state_persists_across_calls`
8. `test_stateful_diff_acme_inc_loyalty_scenario`

---

## 8. Milestone 10.7: Chime End-to-End Integration Test

### 8.1 Test Data

- `chime_bins.csv` — 5 BIN numbers
- Mock BigQuery result as JSON lines file (simulating query output):
  - 10 payment records matching Chime BINs at Acme Inc.
  - Include private label and non-private label items
  - Include a split-payment transaction

### 8.2 Test Pipeline Spec

`crates/ecl-pipeline/tests/fixtures/chime_pipeline.toml`:
- Filesystem source for BIN CSV (stand-in for GCS)
- Filesystem source for mock BQ result (stand-in for BigQuery)
- CSV parse for BINs
- JSON lines parse for BQ data
- Expression stage for reward calculation
- CSV emit for output (stand-in for S3)

### 8.3 Test Scenarios

1. **`test_chime_e2e_reward_calculation`**
   - Parse BINs and mock BQ data
   - Calculate rewards: 5% private label, 3% non-private, $5 cap
   - Verify output CSV has correct reward amounts

2. **`test_chime_e2e_cap_at_five_dollars`**
   - Receipt where raw reward exceeds $5
   - Verify capped at $5

3. **`test_chime_e2e_split_payment_proportional`**
   - Two payments on one receipt
   - Reward distributed proportionally

---

## 9. Milestone 10.8: Acme Inc. Loyalty End-to-End Integration Test

### 9.1 Test Data

- `wag_banyan_loyalty_file_20260315.csv` — 20 loyalty records:
  - 5 new (not in state)
  - 5 unchanged (same CAT)
  - 5 updated (different CAT)
  - 5 with NULL CAT (triggers delete if existing)

### 9.2 Test Scenarios

1. **`test_loyalty_e2e_first_run`**
   - Empty state → all non-null records are inserts
   - Verify Kafka messages (insert action)

2. **`test_loyalty_e2e_second_run_with_diffs`**
   - Pre-populate state from run 1
   - Run with mixed data
   - Verify: 5 inserts, 5 updates, 5 deletes, 5 unchanged (not emitted)

3. **`test_loyalty_e2e_state_persists`**
   - Run 1 populates state
   - Run 2 sees changes
   - Verify state store has correct values after both runs

---

## 10. Cross-Cutting Concerns

### 10.1 New Dependencies

```toml
# Root Cargo.toml workspace deps:
evalexpr = "14"
gcp-bigquery-client = "0.25"
```

### 10.2 New Crates

| Crate | Purpose |
|-------|---------|
| `ecl-adapter-bigquery` | BigQuery query source |

### 10.3 New Stages in `ecl-stages`

| Stage | File | Type |
|-------|------|------|
| `ExpressionStage` | `expression.rs` | Per-item |
| `UpcCheckDigitStage` | `upc_check_digit.rs` | Per-item |
| `AuthCodeNormStage` | `auth_code_norm.rs` | Per-item |
| `JsonLinesParseStage` | `json_lines_parse.rs` | Per-item (fan-out) |
| `StatefulDiffStage` | `stateful_diff.rs` | Batch (needs StateStore) |

### 10.4 Registry Updates

Register all new stages and the BigQuery adapter:

```rust
"expression" => ExpressionStage::from_params(&spec.params)?,
"upc_check_digit" => UpcCheckDigitStage::from_params(&spec.params)?,
"auth_code_norm" => AuthCodeNormStage::from_params(&spec.params)?,
"json_lines_parse" => JsonLinesParseStage,
"stateful_diff" => StatefulDiffStage::new(&spec.params, state_store.clone())?,
```

Note: `StatefulDiffStage` needs access to the `StateStore` — the registry must pass it through.

### 10.5 StateStore Trait Extension Impact

Adding `state_get`/`state_set`/`state_delete`/`state_keys`/`state_batch_set` to the `StateStore` trait is a breaking change for existing implementations. Both `RedbStateStore` and `InMemoryStateStore` need implementations.

For `RedbStateStore`: use dynamically-named redb tables (`state_{table_name}`).
For `InMemoryStateStore`: use `BTreeMap<String, BTreeMap<String, Vec<u8>>>`.

Provide default implementations that return `StateError::Unsupported` so the trait change is less disruptive:

```rust
async fn state_get(&self, _table: &str, _key: &str) -> Result<Option<Vec<u8>>, StateError> {
    Err(StateError::Unsupported("state_get not implemented".to_string()))
}
```

Then override in RedbStateStore and InMemoryStateStore.

---

## 11. Verification Checklist

### Phase 4 Complete

- [ ] Expression evaluation works (arithmetic, conditionals, functions)
- [ ] UPC check digit correctly computes GS1 algorithm
- [ ] Auth code normalization strips/pads correctly
- [ ] BigQuery adapter executes queries and returns results
- [ ] JSON lines parse fans out query results into records
- [ ] Environment overlays deep-merge specs correctly
- [ ] Stateful diff detects inserts, updates, deletes across runs
- [ ] Chime E2E: BINs + BQ data → reward calculation → CSV output
- [ ] Acme Inc. loyalty E2E: CSV → diff → Kafka (insert/update/delete)
- [ ] All Phase 1/2/3 tests still pass

### Full Project Complete

- [ ] **Affinity** (Phase 1): GCS CSV → parse → map → validate → Kafka
- [ ] **Giant Eagle** (Phase 2): 5 CSVs → multi-stream → join → aggregate → assemble → Kafka
- [ ] **Acme Inc. Receipt** (Phase 3): SFTP → PGP decrypt → dedup → transform → Kafka
- [ ] **Chime** (Phase 4): BINs + BigQuery → reward calculation → CSV/S3
- [ ] **Acme Inc. Loyalty** (Phase 4): CSV → stateful diff → Kafka
- [ ] **Environment overlays**: One spec, three deployments
- [ ] **Scheduling**: Cron-based pipeline execution
- [ ] **Pipeline chaining**: Ingestion triggers transformation
- [ ] **Secret management**: GCP Secret Manager integration
- [ ] **Zero customer-specific Rust code**: All pipelines are TOML configurations

---

## Appendix A: Milestone Dependency Graph

```
10.1 Expression ─────────────────────────┐
                                          │
10.2 UPC Check Digit ───────────────────┤
                                          │
10.3 Auth Code Norm ────────────────────┤  (all independent)
                                          │
10.4 BigQuery + JSON Lines Parse ───────┤
                                          │
10.5 Environment Overlays ──────────────┤
                                          │
10.6 Stateful Diff ─────────────────────┤
                                          │
                         ┌────────────────┘
                         │
              10.7 Chime E2E (needs 10.1, 10.4)
                         │
              10.8 Loyalty E2E (needs 10.6)
```

**All of 10.1–10.6 can proceed in parallel.** 10.7 depends on 10.1 + 10.4. 10.8 depends on 10.6.

## Appendix B: Complete Project Statistics

### All Phases Combined

| Category | Count |
|----------|-------|
| New crates | 10 |
| New stage types | 20 |
| New source adapters | 3 (GCS, SFTP, BigQuery) |
| New sink adapters | 3 (Kafka, GCS, S3) |
| Modified existing crates | 8 |
| Total milestones | 32 |
| Estimated test count | 200+ |

### New Crates (All Phases)

| Crate | Phase | Purpose |
|-------|-------|---------|
| `ecl-adapter-gcs` | 1 | Google Cloud Storage source |
| `ecl-sink-kafka` | 1 | Kafka producer + Avro + Schema Registry |
| `ecl-sink-gcs` | 1 | GCS file writer |
| `ecl-adapter-sftp` | 3 | SFTP source |
| `ecl-sink-s3` | 3 | AWS S3 file writer |
| `ecl-secrets` | 3 | Pluggable secret resolution |
| `ecl-adapter-bigquery` | 4 | BigQuery query source |

### All Stage Types (All Phases)

| Stage | Phase | Type | Purpose |
|-------|-------|------|---------|
| `csv_parse` | 1 | Per-item (fan-out) | CSV → Records |
| `field_map` | 1 | Per-item | Rename, drop, set, copy, date parse, pad, regex, nest |
| `validate` | 1 | Per-item | Configurable validation rules |
| `kafka_sink` | 1 | Per-item (terminal) | Kafka producer with Avro |
| `gcs_sink` | 1 | Per-item (terminal) | GCS file writer |
| `join` | 2 | Batch | Merge two streams by key |
| `aggregate` | 2 | Batch | Group-by with aggregation functions |
| `lookup` | 2 | Per-item | Static value mapping tables |
| `date_parse` | 2 | Per-item | String → RFC3339 datetime |
| `timezone` | 2 | Per-item | Local time → UTC by ZIP code |
| `decompress` | 2 | Per-item (fan-out) | ZIP/gzip extraction |
| `assemble` | 2 | Batch | Multi-stream → nested structure |
| `pgp_decrypt` | 3 | Per-item | PGP decryption |
| `deduplicate` | 3 | Batch | Remove duplicate files by hash |
| `s3_sink` | 3 | Per-item (terminal) | AWS S3 file writer |
| `expression` | 4 | Per-item | Arithmetic, conditionals, functions |
| `upc_check_digit` | 4 | Per-item | GS1 check digit algorithm |
| `auth_code_norm` | 4 | Per-item | Strip zeros, pad to width |
| `json_lines_parse` | 4 | Per-item (fan-out) | JSON lines → Records |
| `stateful_diff` | 4 | Batch | Database-backed diff across runs |

### The Final Insight

**20 reusable stage types + 7 adapter/sink crates = infrastructure to run ANY batch data pipeline as TOML configuration.**

No customer-specific Rust code. Acme Inc., Affinity, Chime, and Giant Eagle are all just `.toml` files using combinations of these generic building blocks. Adding a new customer means writing a new TOML spec, not new code.
