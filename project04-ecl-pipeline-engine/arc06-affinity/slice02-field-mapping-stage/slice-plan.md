# 3. Milestone 7.2: Field Mapping Stage

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0026-phase-1-detailed-implementation-plan-make-affinity-work.md, lines 340-534. Original bytes are recoverable from import commit 3a39aea. -->

## 3. Milestone 7.2: Field Mapping Stage

### 3.1 Scope

Configurable stage that renames, drops, sets, and copies fields on Records.

### 3.2 Implementation

#### File: `crates/ecl-stages/src/field_map.rs` (new)

**Configuration schema:**

```rust
#[derive(Debug, Clone, Deserialize)]
struct FieldMapConfig {
    /// Rename fields: { from: "old_name", to: "new_name" }
    #[serde(default)]
    rename: Vec<RenameOp>,

    /// Drop fields by name.
    #[serde(default)]
    drop: Vec<String>,

    /// Set literal values: { field: "name", value: <json_value> }
    #[serde(default)]
    set: Vec<SetOp>,

    /// Copy a field: { from: "source", to: "dest" }
    #[serde(default)]
    copy: Vec<CopyOp>,

    /// Date parsing: { field: "Transaction_Date", format: "%m/%d/%Y", output: "purchase_ts", timezone: "UTC" }
    #[serde(default)]
    parse_dates: Vec<DateParseOp>,

    /// String padding: { field: "auth_code", width: 6, pad_char: "0", side: "left" }
    #[serde(default)]
    pad: Vec<PadOp>,

    /// Regex extract: { field: "Merchant_Location_Name", pattern: "ACME INC.\\s*#\\s*(\\d+)", output: "merchant_store_id", group: 1 }
    #[serde(default)]
    regex_extract: Vec<RegexExtractOp>,

    /// Nested object construction: { output: "payment", fields: { "total_amount": "total_amount", "card_last_four": "card_last_four" } }
    #[serde(default)]
    nest: Vec<NestOp>,
}

#[derive(Debug, Clone, Deserialize)]
struct RenameOp { from: String, to: String }

#[derive(Debug, Clone, Deserialize)]
struct SetOp { field: String, value: serde_json::Value }

#[derive(Debug, Clone, Deserialize)]
struct CopyOp { from: String, to: String }

#[derive(Debug, Clone, Deserialize)]
struct DateParseOp {
    field: String,
    format: String,        // strftime format
    output: String,        // output field name
    #[serde(default = "default_utc")]
    timezone: String,      // "UTC" or IANA timezone
}

#[derive(Debug, Clone, Deserialize)]
struct PadOp {
    field: String,
    width: usize,
    #[serde(default = "default_pad_char")]
    pad_char: String,
    #[serde(default = "default_pad_side")]
    side: String,          // "left" or "right"
}

#[derive(Debug, Clone, Deserialize)]
struct RegexExtractOp {
    field: String,
    pattern: String,
    output: String,
    #[serde(default = "default_group")]
    group: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct NestOp {
    output: String,
    /// Map of output_field_name → source_field_name
    fields: BTreeMap<String, String>,
}
```

**Stage implementation:**

```rust
#[derive(Debug)]
pub struct FieldMapStage {
    config: FieldMapConfig,
    compiled_regexes: Vec<(usize, regex::Regex)>,  // index into regex_extract + compiled
}

impl FieldMapStage {
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: FieldMapConfig = serde_json::from_value(params.clone())
            .map_err(|e| StageError::Permanent { /* ... */ })?;

        // Pre-compile regexes at construction time
        let compiled_regexes = config.regex_extract.iter().enumerate()
            .map(|(i, op)| {
                regex::Regex::new(&op.pattern)
                    .map(|r| (i, r))
                    .map_err(|e| StageError::Permanent { /* ... */ })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { config, compiled_regexes })
    }
}

#[async_trait]
impl Stage for FieldMapStage {
    fn name(&self) -> &str { "field_map" }

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

        // Apply operations in order:
        // 1. Renames (remove old key, insert new key)
        // 2. Copies (read source, insert dest)
        // 3. Sets (insert literal value)
        // 4. Date parsing (parse string → RFC3339)
        // 5. Padding (pad string fields)
        // 6. Regex extract (extract capture group)
        // 7. Nesting (group fields into sub-objects)
        // 8. Drops (remove fields last, after they may have been used)

        Ok(vec![item])
    }
}
```

**Operations detail:**

1. **Rename:** `record.remove("old")` → `record.insert("new", value)`
2. **Copy:** `record.get("src").cloned()` → `record.insert("dst", value)`
3. **Set:** `record.insert("field", literal_value)`
4. **Date parse:** Read string, parse with `chrono::NaiveDate::parse_from_str`, convert to RFC3339 string with timezone
5. **Pad:** Read string, apply `format!("{:0>width$}", val)` for left-pad
6. **Regex extract:** Match against field value string, extract capture group, insert result
7. **Nest:** Create sub-object from listed fields, insert as nested Value::Object
8. **Drop:** `record.remove("field")` for each

**Dependencies to add to `ecl-stages/Cargo.toml`:**

```toml
regex = "1"
chrono = { workspace = true }
```

**Tests:**

1. `test_field_map_rename_basic`
2. `test_field_map_rename_missing_field_no_error` — skip silently
3. `test_field_map_drop_fields`
4. `test_field_map_set_literal_string`
5. `test_field_map_set_literal_number`
6. `test_field_map_copy_field`
7. `test_field_map_date_parse_mm_dd_yyyy` — Affinity date format
8. `test_field_map_date_parse_invalid_returns_null`
9. `test_field_map_pad_left_six_digits` — auth code padding
10. `test_field_map_pad_already_at_width` — no-op
11. `test_field_map_regex_extract_store_id` — `ACME INC. #1234` → `1234`
12. `test_field_map_regex_no_match_sets_null`
13. `test_field_map_nest_creates_sub_object`
14. `test_field_map_no_record_returns_error`
15. `test_field_map_combined_affinity_pipeline` — full Affinity mapping
16. `test_field_map_from_params_invalid`

**Rust patterns:**

- PF-01: Compile regexes once in constructor, reuse per item
- AP-08: Accept `&serde_json::Value` not `&serde_json::Map` for params
- EH-02: Use `?` throughout, never unwrap

---

